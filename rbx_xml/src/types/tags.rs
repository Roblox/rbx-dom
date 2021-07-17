use std::io::{Read, Write};

use rbx_dom_weak::types::Tags;

use crate::{
    core::XmlType,
    deserializer_core::XmlEventReader,
    error::{DecodeError, EncodeError},
    serializer_core::{XmlEventWriter, XmlWriteEvent},
};

impl XmlType for Tags {
    const XML_TAG_NAME: &'static str = "Tags";

    fn write_xml<W: Write>(&self, writer: &mut XmlEventWriter<W>) -> Result<(), EncodeError> {
        // FIXME: BinaryString should have an is_empty method.
        let contents: Vec<u8> = self.as_slice().join("\u{0000}").into_bytes();
        if !contents.is_empty() {
            writer.write(XmlWriteEvent::cdata(&base64::encode(&contents)))?;
        }

        Ok(())
    }

    fn read_xml<R: Read>(reader: &mut XmlEventReader<R>) -> Result<Tags, DecodeError> {
        let value = reader.read_base64_characters()?;
        Ok(Tags::decode(value).unwrap_or_default())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use crate::test_util;

    #[test]
    fn round_trip_tags() {
        let value = Tags::from(
            vec!["My", "Cool", "Tags"]
                .into_iter()
                .map(String::from)
                .collect::<Vec<String>>(),
        );

        test_util::test_xml_round_trip(&value);
    }

    #[test]
    fn decode_tags() {
        test_util::test_xml_deserialize(
            "<BinaryString name=\"foo\">TXkAQ29vbABUYWdz</BinaryString>",
            &Tags::from(
                vec!["My", "Cool", "Tags"]
                    .into_iter()
                    .map(String::from)
                    .collect::<Vec<String>>(),
            ),
        )
    }

    #[test]
    fn encode_tags() {
        test_util::test_xml_serialize(
            "<BinaryString name=\"foo\"><![CDATA[TXkAQ29vbABUYWdz]]></BinaryString>",
            &Tags::from(
                vec!["My", "Cool", "Tags"]
                    .into_iter()
                    .map(String::from)
                    .collect::<Vec<String>>(),
            ),
        )
    }
}
