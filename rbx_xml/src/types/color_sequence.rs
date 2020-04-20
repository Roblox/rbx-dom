use std::io::{Read, Write};

use rbx_dom_weak::types::{Color3, ColorSequence, ColorSequenceKeypoint};

use crate::{
    core::XmlType,
    deserializer_core::XmlEventReader,
    error::{DecodeError, DecodeErrorKind, EncodeError},
    serializer_core::{XmlEventWriter, XmlWriteEvent},
};

impl XmlType for ColorSequence {
    const XML_TAG_NAME: &'static str = "ColorSequence";

    fn write_xml<W: Write>(&self, writer: &mut XmlEventWriter<W>) -> Result<(), EncodeError> {
        for keypoint in &self.keypoints {
            writer.write_characters(keypoint.time)?;
            writer.write(XmlWriteEvent::characters(" "))?;
            writer.write_characters(keypoint.color.r)?;
            writer.write(XmlWriteEvent::characters(" "))?;
            writer.write_characters(keypoint.color.g)?;
            writer.write(XmlWriteEvent::characters(" "))?;
            writer.write_characters(keypoint.color.b)?;
            writer.write(XmlWriteEvent::characters(" "))?;

            // Envelope is always 0 for ColorSequenceKeypoint. This value isn't
            // exposed to developers but serializes in the XML format.
            writer.write_characters(0)?;
            writer.write(XmlWriteEvent::characters(" "))?;
        }

        Ok(())
    }

    fn read_xml<R: Read>(reader: &mut XmlEventReader<R>) -> Result<Self, DecodeError> {
        let contents = reader.read_characters()?;
        let mut pieces = contents
            .split(" ")
            .filter(|slice| !slice.is_empty())
            .map(|piece| piece.parse::<f32>().map_err(|e| reader.error(e)));
        let mut keypoints = Vec::new();

        let wrong_length = || {
            reader.error(DecodeErrorKind::InvalidContent(
                "incorrect number of values",
            ))
        };

        loop {
            let time = match pieces.next() {
                Some(value) => value?,
                None => break,
            };

            let r = pieces.next().ok_or_else(wrong_length)??;
            let g = pieces.next().ok_or_else(wrong_length)??;
            let b = pieces.next().ok_or_else(wrong_length)??;

            // This value is always zero, isn't developer-exposed, and doesn't
            // have a corresponding field in rbx_dom_weak's type.
            let _envelope = pieces.next().ok_or_else(wrong_length)??;

            keypoints.push(ColorSequenceKeypoint {
                time,
                color: Color3::new(r, g, b),
            });
        }

        if keypoints.len() < 2 {
            return Err(reader.error(DecodeErrorKind::InvalidContent(
                "expected two or more keypoints",
            )));
        }

        Ok(ColorSequence { keypoints })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use crate::test_util;

    #[test]
    fn round_trip_color_sequence() {
        test_util::test_xml_round_trip(&ColorSequence {
            keypoints: vec![
                ColorSequenceKeypoint {
                    time: 0.0,
                    color: Color3::new(0.0, 0.5, 1.0),
                },
                ColorSequenceKeypoint {
                    time: 1.0,
                    color: Color3::new(1.0, 0.5, 0.0),
                },
            ],
        });
    }

    #[test]
    fn deserialize_color_sequence() {
        test_util::test_xml_deserialize(
            r#"
                <ColorSequence name="foo">0 0 0.5 1 0 1 1 0.5 0 0 </ColorSequence>
            "#,
            &ColorSequence {
                keypoints: vec![
                    ColorSequenceKeypoint {
                        time: 0.0,
                        color: Color3::new(0.0, 0.5, 1.0),
                    },
                    ColorSequenceKeypoint {
                        time: 1.0,
                        color: Color3::new(1.0, 0.5, 0.0),
                    },
                ],
            },
        );
    }

    #[test]
    fn serialize_color_sequence() {
        test_util::test_xml_serialize(
            r#"
                <ColorSequence name="foo">0 0 0.5 1 0 1 1 0.5 0 0 </ColorSequence>
            "#,
            &ColorSequence {
                keypoints: vec![
                    ColorSequenceKeypoint {
                        time: 0.0,
                        color: Color3::new(0.0, 0.5, 1.0),
                    },
                    ColorSequenceKeypoint {
                        time: 1.0,
                        color: Color3::new(1.0, 0.5, 0.0),
                    },
                ],
            },
        );
    }
}
