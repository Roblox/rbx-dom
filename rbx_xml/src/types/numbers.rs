use std::io::{Read, Write};

use crate::{
    core::XmlType,
    deserializer_core::XmlEventReader,
    error::{DecodeError, EncodeError},
    serializer_core::XmlEventWriter,
};

macro_rules! float_type {
    ($rust_type: ident, $xml_name: expr) => {
        impl XmlType for $rust_type {
            const XML_TAG_NAME: &'static str = $xml_name;

            fn write_xml<W: Write>(
                &self,
                writer: &mut XmlEventWriter<W>,
            ) -> Result<(), EncodeError> {
                if *self == std::$rust_type::INFINITY {
                    writer.write_characters("INF")
                } else if *self == std::$rust_type::NEG_INFINITY {
                    writer.write_characters("-INF")
                } else if self.is_nan() {
                    writer.write_characters("NAN")
                } else {
                    writer.write_characters(self)
                }
            }

            fn read_xml<R: Read>(reader: &mut XmlEventReader<R>) -> Result<Self, DecodeError> {
                let contents = reader.read_characters()?;

                Ok(match contents.as_str() {
                    "INF" => std::$rust_type::INFINITY,
                    "-INF" => std::$rust_type::NEG_INFINITY,
                    "NAN" => std::$rust_type::NAN,
                    number => number.parse().map_err(|e| reader.error(e))?,
                })
            }
        }
    };
}

macro_rules! int_type {
    ($rust_type: ty, $xml_name: expr) => {
        impl XmlType for $rust_type {
            const XML_TAG_NAME: &'static str = $xml_name;

            fn write_xml<W: Write>(
                &self,
                writer: &mut XmlEventWriter<W>,
            ) -> Result<(), EncodeError> {
                writer.write_characters(*self)
            }

            fn read_xml<R: Read>(reader: &mut XmlEventReader<R>) -> Result<Self, DecodeError> {
                reader
                    .read_characters()?
                    .parse()
                    .map_err(|e| reader.error(e))
            }
        }
    };
}

float_type!(f32, "float");
float_type!(f64, "double");
int_type!(i32, "int");
int_type!(i64, "int64");

// Convenience implementations for other types.
// FIXME: This feels weird to bundle into the XmlType trait.
int_type!(i16, "<NOT A REAL TYPE>");

#[cfg(test)]
mod test {
    use super::*;

    use crate::{core::XmlType, deserializer_core::XmlEventReader, test_util};

    #[test]
    fn round_trip_f32() {
        test_util::test_xml_round_trip(&123456.0f32);
    }

    #[test]
    fn round_trip_f64() {
        test_util::test_xml_round_trip(&123456.0f64);
    }

    #[test]
    fn round_trip_i32() {
        test_util::test_xml_round_trip(&-4654321i32);
    }

    #[test]
    fn round_trip_i64() {
        test_util::test_xml_round_trip(&281474976710656i64);
    }

    #[test]
    fn test_inf_and_nan_deserialize() {
        test_util::test_xml_deserialize(r#"<float name="foo">INF</float>"#, &std::f32::INFINITY);

        test_util::test_xml_deserialize(
            r#"<float name="foo">-INF</float>"#,
            &std::f32::NEG_INFINITY,
        );

        // Can't just use test_util::test_xml_deserialize, because NaN != NaN!
        let mut reader = XmlEventReader::from_source(r#"<float name="foo">NAN</float>"#.as_bytes());
        reader.next().unwrap().unwrap(); // Eat StartDocument event
        let value = f32::read_outer_xml(&mut reader).unwrap();
        assert!(value.is_nan());
    }

    #[test]
    fn test_inf_and_nan_serialize() {
        test_util::test_xml_serialize(r#"<float name="foo">INF</float>"#, &std::f32::INFINITY);

        test_util::test_xml_serialize(r#"<float name="foo">-INF</float>"#, &std::f32::NEG_INFINITY);

        test_util::test_xml_serialize(r#"<float name="foo">NAN</float>"#, &std::f32::NAN);
    }
}
