use std::{fmt::Write as FmtWrite, io::Write};

use xml::writer::{EmitterConfig, EventWriter};

pub use xml::writer::XmlEvent as XmlWriteEvent;

use crate::{
    core::XmlType,
    error::{EncodeError as NewEncodeError, EncodeErrorKind},
};

/// A wrapper around an xml-rs `EventWriter` as well as other state kept around
/// for performantly emitting XML.
pub struct XmlEventWriter<W> {
    inner: EventWriter<W>,
    character_buffer: String,
}

impl<W: Write> XmlEventWriter<W> {
    /// Constructs an `XmlEventWriter` from an output that implements `Write`.
    pub fn from_output(output: W) -> XmlEventWriter<W> {
        let inner = EmitterConfig::new()
            .perform_indent(true)
            .write_document_declaration(false)
            .normalize_empty_elements(false)
            .create_writer(output);

        XmlEventWriter {
            inner,
            character_buffer: String::new(),
        }
    }

    pub(crate) fn error<T: Into<EncodeErrorKind>>(&self, kind: T) -> NewEncodeError {
        NewEncodeError::new_from_writer(kind.into(), &self.inner)
    }

    pub fn end_element(&mut self) -> Result<(), NewEncodeError> {
        self.inner
            .write(XmlWriteEvent::end_element())
            .map_err(|e| self.error(e))
    }

    /// Writes a single XML event to the output stream.
    pub fn write<'a, E>(&mut self, event: E) -> Result<(), NewEncodeError>
    where
        E: Into<XmlWriteEvent<'a>>,
    {
        self.inner.write(event).map_err(|e| self.error(e))
    }

    /// Writes a string slice to the output stream as characters or CDATA.
    pub fn write_string(&mut self, value: &str) -> Result<(), NewEncodeError> {
        write_characters_or_cdata(&mut self.inner, value)
    }

    /// Writes a value that implements `Display` as characters or CDATA. Resuses
    /// an internal buffer to avoid unnecessary allocations.
    pub fn write_characters<T: std::fmt::Display>(
        &mut self,
        value: T,
    ) -> Result<(), NewEncodeError> {
        write!(self.character_buffer, "{}", value).unwrap();
        write_characters_or_cdata(&mut self.inner, &self.character_buffer)?;
        self.character_buffer.clear();

        Ok(())
    }

    pub fn write_value<T: XmlType>(&mut self, value: &T) -> Result<(), NewEncodeError> {
        value.write_xml(self)
    }

    pub fn write_value_in_tag<T: XmlType>(
        &mut self,
        value: &T,
        tag: &str,
    ) -> Result<(), NewEncodeError> {
        self.write(XmlWriteEvent::start_element(tag))?;
        self.write_value(value)?;
        self.write(XmlWriteEvent::end_element())
    }

    /// The same as `write_characters`, but wraps the characters in a tag with
    /// the given name and no attributes.
    pub fn write_tag_characters<T: std::fmt::Display>(
        &mut self,
        tag: &str,
        value: T,
    ) -> Result<(), NewEncodeError> {
        self.write(XmlWriteEvent::start_element(tag))?;
        self.write_characters(value)?;
        self.write(XmlWriteEvent::end_element())
    }

    /// Writes a list of values that implement `Display`, with each wrapped in
    /// an associated tag. This method uses the same optimization as
    /// `write_characters` to avoid extra allocations.
    pub fn write_tag_array<T: std::fmt::Display>(
        &mut self,
        values: &[T],
        tags: &[&str],
    ) -> Result<(), NewEncodeError> {
        assert_eq!(values.len(), tags.len());

        for (index, component) in values.iter().enumerate() {
            self.write_tag_characters(tags[index], component)?;
        }

        Ok(())
    }
}

/// Given a value, writes a `Characters` event or a `CData` event depending on
/// whether the input string contains whitespace that needs to be explicitly
/// preserved.
///
/// This method is extracted so that it can be used inside both `write_string`
/// and `write_characters` without borrowing issues.
fn write_characters_or_cdata<W: Write>(
    writer: &mut EventWriter<W>,
    value: &str,
) -> Result<(), NewEncodeError> {
    let first_char = value.chars().next();
    let last_char = value.chars().next_back();

    // If the string has leading or trailing whitespace, we switch to
    // writing it as part of a CDATA block instead of a regular characters
    // block.
    let has_outer_whitespace = match (first_char, last_char) {
        (Some(first), Some(last)) => first.is_whitespace() || last.is_whitespace(),
        (Some(char), None) | (None, Some(char)) => char.is_whitespace(),
        (None, None) => false,
    };

    if has_outer_whitespace {
        writer
            .write(XmlWriteEvent::cdata(value))
            .map_err(|e| NewEncodeError::new_from_writer(e.into(), &writer))?;
    } else {
        writer
            .write(XmlWriteEvent::characters(value))
            .map_err(|e| NewEncodeError::new_from_writer(e.into(), &writer))?;
    }

    Ok(())
}
