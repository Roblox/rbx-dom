use std::io::{Read, Write};

use rbx_dom_weak::types::BinaryString;

use crate::{
    core::XmlType,
    deserializer_core::XmlEventReader,
    error::{DecodeError, EncodeError},
    serializer_core::{XmlEventWriter, XmlWriteEvent},
};

impl XmlType for BinaryString {
    const XML_TAG_NAME: &'static str = "BinaryString";

    fn write_xml<W: Write>(&self, writer: &mut XmlEventWriter<W>) -> Result<(), EncodeError> {
        // FIXME: BinaryString should have an is_empty method.
        let contents: &[u8] = self.as_ref();
        if !contents.is_empty() {
            writer.write(XmlWriteEvent::cdata(&base64::encode(self)))?;
        }

        Ok(())
    }

    fn read_xml<R: Read>(reader: &mut XmlEventReader<R>) -> Result<BinaryString, DecodeError> {
        let value = reader.read_base64_characters()?;
        Ok(value.into())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use crate::test_util;

    #[test]
    fn round_trip_binary_string() {
        let value = BinaryString::from(
            b"\x00\x01hello,\n\x7Fworld, from a fairly sizable binary string literal.\n".to_vec(),
        );

        test_util::test_xml_round_trip(&value);
    }

    #[test]
    fn round_trip_empty() {
        test_util::test_xml_round_trip(&BinaryString::new());
    }

    #[test]
    fn decode_simple() {
        test_util::test_xml_deserialize(
            "<BinaryString name=\"foo\">SGVsbG8sIHdvcmxkIQ==</BinaryString>",
            &BinaryString::from(b"Hello, world!".to_vec()),
        );
    }

    #[test]
    fn decode_lf() {
        test_util::test_xml_deserialize(
            "<BinaryString name=\"foo\">SGVsbG8s\nIHdv\n\ncmxkIQ==</BinaryString>",
            &BinaryString::from(b"Hello, world!".to_vec()),
        );
    }

    #[test]
    fn decode_crlf() {
        test_util::test_xml_deserialize(
            "<BinaryString name=\"foo\">SGVsbG8s\r\nIHdv\r\n\r\ncmxk\nIQ==</BinaryString>",
            &BinaryString::from(b"Hello, world!".to_vec()),
        );
    }

    #[test]
    fn decode_spaces() {
        test_util::test_xml_deserialize(
            "<BinaryString name=\"foo\">SGVsbG8s IHdv  cmxkIQ= =</BinaryString>",
            &BinaryString::from(b"Hello, world!".to_vec()),
        );
    }

    #[test]
    fn cdata_serialize() {
        test_util::test_xml_serialize(
            "<BinaryString name=\"foo\"><![CDATA[SGVsbG8sIHdvcmxkIQ==]]></BinaryString>",
            &BinaryString::from(b"Hello, world!".to_vec()),
        );
    }

    #[test]
    fn no_cdata_empty() {
        test_util::test_xml_serialize(
            "<BinaryString name=\"foo\"></BinaryString>",
            &BinaryString::new(),
        );
    }
}
