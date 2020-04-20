use std::io::{Read, Write};

use rbx_dom_weak::types::Ray;

use crate::{
    core::XmlType,
    deserializer_core::XmlEventReader,
    error::{DecodeError, EncodeError},
    serializer_core::XmlEventWriter,
};

impl XmlType for Ray {
    const XML_TAG_NAME: &'static str = "Ray";

    fn write_xml<W: Write>(&self, writer: &mut XmlEventWriter<W>) -> Result<(), EncodeError> {
        writer.write_value_in_tag(&self.origin, "origin")?;
        writer.write_value_in_tag(&self.direction, "direction")?;

        Ok(())
    }

    fn read_xml<R: Read>(reader: &mut XmlEventReader<R>) -> Result<Self, DecodeError> {
        let origin = reader.read_value_in_tag("origin")?;
        let direction = reader.read_value_in_tag("direction")?;

        Ok(Ray { origin, direction })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use rbx_dom_weak::types::Vector3;

    use crate::test_util;

    #[test]
    fn round_trip_ray() {
        test_util::test_xml_round_trip(&Ray {
            origin: Vector3::new(-12.5, 718.5, 3.0),
            direction: Vector3::new(100.0, 200.0, 9000.0),
        });
    }

    #[test]
    fn deserialize_rect() {
        test_util::test_xml_deserialize(
            r#"
                <Ray name="Value">
                    <origin>
                        <X>5.0</X>
                        <Y>10.0</Y>
                        <Z>6.5</Z>
                    </origin>
                    <direction>
                        <X>2.0</X>
                        <Y>300.0</Y>
                        <Z>900.0</Z>
                    </direction>
                </Ray>
            "#,
            &Ray {
                origin: Vector3::new(5.0, 10.0, 6.5),
                direction: Vector3::new(2.0, 300.0, 900.0),
            },
        );
    }

    #[test]
    fn serialize_ray() {
        test_util::test_xml_serialize(
            r#"
                <Ray name="foo">
                    <origin>
                        <X>5</X>
                        <Y>10</Y>
                        <Z>6.5</Z>
                    </origin>
                    <direction>
                        <X>2</X>
                        <Y>300</Y>
                        <Z>900</Z>
                    </direction>
                </Ray>
            "#,
            &Ray {
                origin: Vector3::new(5.0, 10.0, 6.5),
                direction: Vector3::new(2.0, 300.0, 900.0),
            },
        );
    }
}
