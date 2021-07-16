use std::convert::TryInto;
use std::io::{Read, Write};

use rbx_dom_weak::types::Tags;

use crate::{
    core::XmlType,
    deserializer_core::XmlEventReader,
    error::{DecodeError, EncodeError},
    serializer_core::{XmlEventWriter, XmlWriteEvent},
};

impl XmlType for Tags {
    const XML_TAG_NAME: &'static str = "BinaryString";

    fn write_xml<W: Write>(&self, writer: &mut XmlEventWriter<W>) -> Result<(), EncodeError> {
        // FIXME: BinaryString should have an is_empty method.
        let contents: Vec<u8> = self.into();
        if !contents.is_empty() {
            writer.write(XmlWriteEvent::cdata(&base64::encode(&contents)))?;
        }

        Ok(())
    }

    fn read_xml<R: Read>(reader: &mut XmlEventReader<R>) -> Result<Tags, DecodeError> {
        let value = reader.read_base64_characters()?;
        Ok(value.try_into().unwrap_or_default())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use crate::test_util;

    #[test]
    fn round_trip_tags() {
        let value = Tags::from(vec!["ez", "pz"]);

        test_util::test_xml_round_trip(&value);
    }

    #[test]
    fn decode_tags() {
        test_util::test_xml_deserialize(
            "<BinaryString name=\"foo\">TXkAQ29vbABUYWdzAA==</BinaryString>",
            &Tags::from(vec!["My", "Cool", "Tags"]),
        )
    }

    #[test]
    fn encode_tags() {
        test_util::test_xml_serialize(
            "<BinaryString name=\"foo\"><![CDATA[TXkAQ29vbABUYWdzAA==]]></BinaryString>",
            &Tags::from(vec!["My", "Cool", "Tags"]),
        )
    }
}
