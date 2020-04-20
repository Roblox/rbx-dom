use std::io::{Read, Write};

use rbx_dom_weak::types::{Vector2, Vector2int16, Vector3, Vector3int16};

use crate::{
    core::XmlType,
    deserializer_core::XmlEventReader,
    error::{DecodeError, EncodeError},
    serializer_core::XmlEventWriter,
};

macro_rules! impl_vector {
    ( $vector: ident, $component: ident, ( $( $axis: ident : $label: literal ),* ) ) => {
        impl XmlType for $vector {
            const XML_TAG_NAME: &'static str = stringify!($vector);

            fn write_xml<W: Write>(&self, writer: &mut XmlEventWriter<W>) -> Result<(), EncodeError> {
                $(
                    writer.write_value_in_tag(&self.$axis, $label)?;
                )*

                Ok(())
            }

            fn read_xml<R: Read>(reader: &mut XmlEventReader<R>) -> Result<Self, DecodeError> {
                $(
                    let $axis: $component = reader.read_value_in_tag($label)?;
                )*

                Ok($vector {
                    $( $axis, )*
                })
            }
        }
    };
}

impl_vector!(Vector2, f32, (x: "X", y: "Y"));
impl_vector!(Vector2int16, i16, (x: "X", y: "Y"));

impl_vector!(Vector3, f32, (x: "X", y: "Y", z: "Z"));
impl_vector!(Vector3int16, i16, (x: "X", y: "Y", z: "Z"));

#[cfg(test)]
mod test {
    use super::*;

    use crate::test_util;

    #[test]
    fn round_trip_vector2() {
        test_util::test_xml_round_trip(&Vector2::new(123.0, 456.0));
    }

    #[test]
    fn round_trip_vector2int16() {
        test_util::test_xml_round_trip(&Vector2int16::new(1234, 4567));
    }

    #[test]
    fn round_trip_vector3() {
        test_util::test_xml_round_trip(&Vector3::new(123.0, 456.0, 7890.0));
    }

    #[test]
    fn round_trip_vector3int16() {
        test_util::test_xml_round_trip(&Vector3int16::new(1234, 4567, 8913));
    }
}
