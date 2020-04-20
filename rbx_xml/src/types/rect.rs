use std::io::{Read, Write};

use rbx_dom_weak::types::Rect;

use crate::{
    core::XmlType,
    deserializer_core::XmlEventReader,
    error::{DecodeError, EncodeError},
    serializer_core::XmlEventWriter,
};

impl XmlType for Rect {
    const XML_TAG_NAME: &'static str = "Rect2D";

    fn write_xml<W: Write>(&self, writer: &mut XmlEventWriter<W>) -> Result<(), EncodeError> {
        writer.write_value_in_tag(&self.min, "min")?;
        writer.write_value_in_tag(&self.max, "max")?;

        Ok(())
    }

    fn read_xml<R: Read>(reader: &mut XmlEventReader<R>) -> Result<Self, DecodeError> {
        let min = reader.read_value_in_tag("min")?;
        let max = reader.read_value_in_tag("max")?;

        Ok(Rect { min, max })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use rbx_dom_weak::types::Vector2;

    use crate::test_util;

    #[test]
    fn round_trip_rect() {
        test_util::test_xml_round_trip(&Rect {
            min: Vector2::new(-12.5, 718.5),
            max: Vector2::new(100.0, 200.0),
        });
    }

    #[test]
    fn deserialize_rect() {
        test_util::test_xml_deserialize(
            r#"
                <Rect2D name="SliceCenter">
                    <min>
                        <X>12.5</X>
                        <Y>-30.5</Y>
                    </min>
                    <max>
                        <X>23</X>
                        <Y>9</Y>
                    </max>
                </Rect2D>
            "#,
            &Rect {
                min: Vector2::new(12.5, -30.5),
                max: Vector2::new(23.0, 9.0),
            },
        );
    }

    #[test]
    fn serialize_rect() {
        test_util::test_xml_serialize(
            r#"
                <Rect2D name="foo">
                    <min>
                        <X>12.5</X>
                        <Y>-30.5</Y>
                    </min>
                    <max>
                        <X>23</X>
                        <Y>9</Y>
                    </max>
                </Rect2D>
            "#,
            &Rect {
                min: Vector2::new(12.5, -30.5),
                max: Vector2::new(23.0, 9.0),
            },
        );
    }
}
