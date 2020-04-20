use std::io::{Read, Write};

use rbx_dom_weak::types::{CFrame, Matrix3, Vector3};

use crate::{
    core::XmlType,
    deserializer_core::XmlEventReader,
    error::{DecodeError, EncodeError},
    serializer_core::XmlEventWriter,
};

static TAG_NAMES: [&str; 12] = [
    "X", "Y", "Z", "R00", "R01", "R02", "R10", "R11", "R12", "R20", "R21", "R22",
];

impl XmlType for CFrame {
    const XML_TAG_NAME: &'static str = "CoordinateFrame";

    fn write_xml<W: Write>(&self, writer: &mut XmlEventWriter<W>) -> Result<(), EncodeError> {
        // FIXME: Should this be built into rbx_types?
        let as_slice = &[
            self.position.x,
            self.position.y,
            self.position.z,
            self.orientation.x.x,
            self.orientation.x.y,
            self.orientation.x.z,
            self.orientation.y.x,
            self.orientation.y.y,
            self.orientation.y.z,
            self.orientation.z.x,
            self.orientation.z.y,
            self.orientation.z.z,
        ];

        writer.write_tag_array(as_slice, &TAG_NAMES)?;

        Ok(())
    }

    fn read_xml<R: Read>(reader: &mut XmlEventReader<R>) -> Result<Self, DecodeError> {
        let mut value = CFrame::new(Vector3::new(0.0, 0.0, 0.0), Matrix3::identity());

        for index in 0..12 {
            let tag_name = TAG_NAMES[index];
            let component: f32 = reader.read_value_in_tag(tag_name)?;

            match tag_name {
                "X" => value.position.x = component,
                "Y" => value.position.y = component,
                "Z" => value.position.z = component,

                "R00" => value.orientation.x.x = component,
                "R01" => value.orientation.x.y = component,
                "R02" => value.orientation.x.z = component,

                "R10" => value.orientation.y.x = component,
                "R11" => value.orientation.y.y = component,
                "R12" => value.orientation.y.z = component,

                "R20" => value.orientation.z.x = component,
                "R21" => value.orientation.z.y = component,
                "R22" => value.orientation.z.z = component,

                _ => unreachable!(),
            }
        }

        Ok(value)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use crate::test_util;

    #[test]
    fn round_trip() {
        let test_input = CFrame::new(
            Vector3::new(123.0, 456.0, 789.0),
            Matrix3 {
                x: Vector3::new(987.0, 654.0, 432.0),
                y: Vector3::new(210.0, 0.0, -12345.0),
                z: Vector3::new(765.0, 234.0, 123123.0),
            },
        );

        test_util::test_xml_round_trip(&test_input);
    }
}
