use std::io::{Read, Write};

use rbx_dom_weak::types::{NumberSequence, NumberSequenceKeypoint};

use crate::{
    core::XmlType,
    deserializer_core::XmlEventReader,
    error::{DecodeError, DecodeErrorKind, EncodeError},
    serializer_core::{XmlEventWriter, XmlWriteEvent},
};

impl XmlType for NumberSequence {
    const XML_TAG_NAME: &'static str = "NumberSequence";

    fn write_xml<W: Write>(&self, writer: &mut XmlEventWriter<W>) -> Result<(), EncodeError> {
        for keypoint in &self.keypoints {
            writer.write_characters(keypoint.time)?;
            writer.write(XmlWriteEvent::characters(" "))?;
            writer.write_characters(keypoint.value)?;
            writer.write(XmlWriteEvent::characters(" "))?;
            writer.write_characters(keypoint.envelope)?;
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

            let value = pieces.next().ok_or_else(wrong_length)??;
            let envelope = pieces.next().ok_or_else(wrong_length)??;

            keypoints.push(NumberSequenceKeypoint {
                time,
                value,
                envelope,
            });
        }

        if keypoints.len() < 2 {
            return Err(reader.error(DecodeErrorKind::InvalidContent(
                "expected two or more keypoints",
            )));
        }

        Ok(NumberSequence { keypoints })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use crate::test_util;

    #[test]
    fn round_trip_number_sequence() {
        test_util::test_xml_round_trip(&NumberSequence {
            keypoints: vec![
                NumberSequenceKeypoint {
                    time: 0.0,
                    value: 10.0,
                    envelope: 11.0,
                },
                NumberSequenceKeypoint {
                    time: 1.0,
                    value: 12.0,
                    envelope: 13.0,
                },
            ],
        });
    }

    #[test]
    fn deserialize_number_sequence() {
        test_util::test_xml_deserialize(
            r#"
                <NumberSequence name="foo">0 10.5 11.5 1 12 13 </NumberSequence>
            "#,
            &NumberSequence {
                keypoints: vec![
                    NumberSequenceKeypoint {
                        time: 0.0,
                        value: 10.5,
                        envelope: 11.5,
                    },
                    NumberSequenceKeypoint {
                        time: 1.0,
                        value: 12.0,
                        envelope: 13.0,
                    },
                ],
            },
        );
    }

    #[test]
    fn serialize_number_sequence() {
        test_util::test_xml_serialize(
            r#"
                <NumberSequence name="foo">0 10.5 11.5 1 12 13 </NumberSequence>
            "#,
            &NumberSequence {
                keypoints: vec![
                    NumberSequenceKeypoint {
                        time: 0.0,
                        value: 10.5,
                        envelope: 11.5,
                    },
                    NumberSequenceKeypoint {
                        time: 1.0,
                        value: 12.0,
                        envelope: 13.0,
                    },
                ],
            },
        );
    }
}
