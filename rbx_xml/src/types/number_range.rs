use std::io::{Read, Write};

use rbx_dom_weak::types::NumberRange;

use crate::{
    core::XmlType,
    deserializer_core::XmlEventReader,
    error::{DecodeError, DecodeErrorKind, EncodeError},
    serializer_core::{XmlEventWriter, XmlWriteEvent},
};

impl XmlType for NumberRange {
    const XML_TAG_NAME: &'static str = "NumberRange";

    fn write_xml<W: Write>(&self, writer: &mut XmlEventWriter<W>) -> Result<(), EncodeError> {
        writer.write_characters(self.min)?;
        writer.write(XmlWriteEvent::characters(" "))?;
        writer.write_characters(self.max)?;
        writer.write(XmlWriteEvent::characters(" "))?;

        Ok(())
    }

    fn read_xml<R: Read>(reader: &mut XmlEventReader<R>) -> Result<Self, DecodeError> {
        let contents = reader.read_characters()?;
        let mut pieces = contents
            .split(" ")
            .filter(|slice| !slice.is_empty())
            .map(|piece| piece.parse::<f32>().map_err(|e| reader.error(e)));

        let min = pieces
            .next()
            .ok_or_else(|| reader.error(DecodeErrorKind::InvalidContent("missing min value")))??;

        let max = pieces
            .next()
            .ok_or_else(|| reader.error(DecodeErrorKind::InvalidContent("missing max value")))??;

        match pieces.next() {
            None => {}
            Some(_) => return Err(reader.error(DecodeErrorKind::InvalidContent("too many values"))),
        }

        Ok(NumberRange { min, max })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use crate::test_util;

    #[test]
    fn round_trip_number_range() {
        test_util::test_xml_round_trip(&NumberRange {
            min: -30.0,
            max: 103.5,
        });
    }

    #[test]
    fn deserialize_number_range() {
        test_util::test_xml_deserialize(
            r#"
                <NumberRange name="foo">80.5 -30 </NumberRange>
            "#,
            &NumberRange {
                min: 80.5,
                max: -30.0,
            },
        );
    }

    #[test]
    fn serialize_number_range() {
        test_util::test_xml_serialize(
            r#"
                <NumberRange name="foo">80.5 -30 </NumberRange>
            "#,
            &NumberRange {
                min: 80.5,
                max: -30.0,
            },
        );
    }
}
