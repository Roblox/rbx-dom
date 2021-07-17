use std::io::Write;

use rbx_dom_weak::types::Tags;

use crate::{
    serializer_core::{XmlEventWriter, XmlWriteEvent},
    EncodeError,
};

pub const XML_TAG_NAME: &str = "BinaryString";

pub fn write_tags<W: Write>(
    writer: &mut XmlEventWriter<W>,
    property_name: &str,
    value: &Tags,
) -> Result<(), EncodeError> {
    writer.write(XmlWriteEvent::start_element(XML_TAG_NAME).attr("name", property_name))?;
    writer.write_string(&base64::encode(
        value.as_slice().join("\u{0000}").as_bytes(),
    ))?;
    writer.write(XmlWriteEvent::end_element())?;

    Ok(())
}
