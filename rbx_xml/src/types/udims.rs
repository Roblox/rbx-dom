use std::io::{Read, Write};

use rbx_dom_weak::types::{UDim, UDim2};

use crate::{
    core::XmlType,
    deserializer_core::XmlEventReader,
    error::{DecodeError, EncodeError},
    serializer_core::XmlEventWriter,
};

impl XmlType for UDim {
    const XML_TAG_NAME: &'static str = "UDim";

    fn write_xml<W: Write>(&self, writer: &mut XmlEventWriter<W>) -> Result<(), EncodeError> {
        writer.write_value_in_tag(&self.scale, "S")?;
        writer.write_value_in_tag(&self.offset, "O")?;

        Ok(())
    }

    fn read_xml<R: Read>(reader: &mut XmlEventReader<R>) -> Result<Self, DecodeError> {
        let scale: f32 = reader.read_value_in_tag("S")?;
        let offset: i32 = reader.read_value_in_tag("O")?;

        Ok(UDim { scale, offset })
    }
}

impl XmlType for UDim2 {
    const XML_TAG_NAME: &'static str = "UDim2";

    fn write_xml<W: Write>(&self, writer: &mut XmlEventWriter<W>) -> Result<(), EncodeError> {
        writer.write_value_in_tag(&self.x.scale, "XS")?;
        writer.write_value_in_tag(&self.x.offset, "XO")?;
        writer.write_value_in_tag(&self.y.scale, "YS")?;
        writer.write_value_in_tag(&self.y.offset, "YO")?;

        Ok(())
    }

    fn read_xml<R: Read>(reader: &mut XmlEventReader<R>) -> Result<Self, DecodeError> {
        let x_scale: f32 = reader.read_value_in_tag("XS")?;
        let x_offset: i32 = reader.read_value_in_tag("XO")?;
        let y_scale: f32 = reader.read_value_in_tag("YS")?;
        let y_offset: i32 = reader.read_value_in_tag("YO")?;

        Ok(UDim2 {
            x: UDim::new(x_scale, x_offset),
            y: UDim::new(y_scale, y_offset),
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use crate::test_util;

    #[test]
    fn round_trip_udim() {
        test_util::test_xml_round_trip(&UDim::new(0.5, 1));
    }

    #[test]
    fn round_trip_udim2() {
        test_util::test_xml_round_trip(&UDim2::new(UDim::new(0.5, 1), UDim::new(1.5, 2)));
    }

    #[test]
    fn de_udim() {
        test_util::test_xml_deserialize(
            r#"
                <UDim>
                    <S>0.5</S>
                    <O>1</O>
                </UDim>
            "#,
            &UDim::new(0.5, 1),
        );
    }

    #[test]
    fn de_udim2() {
        test_util::test_xml_deserialize(
            r#"
                <UDim2>
                    <XS>0.5</XS>
                    <XO>1</XO>
                    <YS>1.5</YS>
                    <YO>2</YO>
                </UDim2>
            "#,
            &UDim2::new(UDim::new(0.5, 1), UDim::new(1.5, 2)),
        );
    }
}
