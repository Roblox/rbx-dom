//! This file packs up all of the type implementations in rbx_xml and exposes
//! them through two methods, `read_value_xml` and `write_value_xml`.
//!
//! To support a new type in rbx_xml:
//!
//! 1. Implement the type in a submodule (with tests!)
//! 2. Add a 'mod' statement immediately below this comment
//! 3. Add the type(s) to the declare_rbx_types! macro invocation

mod binary_string;
mod bool;
mod cframe;
mod color_sequence;
mod colors;
mod content;
mod enumeration;
mod number_range;
mod number_sequence;
mod numbers;
mod physical_properties;
mod ray;
mod rect;
mod referent;
// mod shared_string;
mod strings;
mod udims;
mod vectors;

use std::io::{Read, Write};

use rbx_dom_weak::types::{
    BinaryString, CFrame, Color3, Color3uint8, ColorSequence, Content, EnumValue, NumberRange,
    NumberSequence, PhysicalProperties, Ray, Rect, Ref, Variant, Vector2, Vector2int16, Vector3,
    Vector3int16,
};

use crate::{
    core::XmlType,
    deserializer::ParseState,
    deserializer_core::XmlEventReader,
    error::{DecodeError, DecodeErrorKind, EncodeError, EncodeErrorKind},
    serializer::EmitState,
    serializer_core::XmlEventWriter,
};

use self::referent::{read_ref, write_ref};

/// The `declare_rbx_types` macro generates the two big match statements that
/// rbx_xml uses to read/write values inside of `read_value_xml` and
/// `write_value_xml`.
macro_rules! declare_rbx_types {
    { $($variant_name: ident : $inner_type: ident,)* } => {

        /// Reads a Roblox property value with the given type from the XML event
        /// stream.
        pub fn read_value_xml<R: Read>(
            reader: &mut XmlEventReader<R>,
            state: &mut ParseState,
            xml_type_name: &str,
            instance_id: Ref,
            property_name: &str,
        ) -> Result<Variant, DecodeError> {
            match xml_type_name {
                $(<$inner_type>::XML_TAG_NAME => Ok(Variant::$variant_name(<$inner_type>::read_outer_xml(reader)?)),)*

                // Protected strings are only read, never written
                // self::strings::ProtectedStringType::XML_TAG_NAME => self::strings::ProtectedStringType::read_xml(reader),

                self::referent::XML_TAG_NAME => Ok(Variant::Ref(read_ref(reader, instance_id, property_name, state)?)),
                // self::shared_string::XML_TAG_NAME => read_shared_string(reader, instance_id, property_name, state),

                _ => {
                    Err(reader.error(DecodeErrorKind::UnknownPropertyType(xml_type_name.to_owned())))
                },
            }
        }

        /// Writes a Roblox property value with the given XML name to the XML
        /// stream.
        pub fn write_value_xml<W: Write>(
            writer: &mut XmlEventWriter<W>,
            state: &mut EmitState,
            xml_property_name: &str,
            value: &Variant,
        ) -> Result<(), EncodeError> {
            match value {
                $(Variant::$variant_name(value) => value.write_outer_xml(xml_property_name, writer),)*

                // BrickColor values just encode as 32-bit ints, and have no
                // unique appearance for reading.
                // Variant::BrickColor(value) =>
                //     self::numbers::Int32Type::write_xml(writer, xml_property_name, &(*value as i32)),

                Variant::Ref(value) => write_ref(writer, xml_property_name, *value, state),
                // Variant::SharedString(value) => write_shared_string(writer, xml_property_name, value, state),

                unknown => {
                    Err(writer.error(EncodeErrorKind::UnsupportedPropertyType(unknown.ty())))
                },
            }
        }
    }
}

declare_rbx_types! {
    BinaryString: BinaryString,
    Bool: bool,
    CFrame: CFrame,
    String: String,
    ColorSequence: ColorSequence,
    Color3: Color3,
    Color3uint8: Color3uint8,
    Content: Content,
    EnumValue: EnumValue,
    NumberRange: NumberRange,
    NumberSequence: NumberSequence,
    Float32: f32,
    Float64: f64,
    Int32: i32,
    Int64: i64,
    PhysicalProperties: PhysicalProperties,
    Ray: Ray,
    Rect: Rect,
    // self::udims::UDim2Type => UDim2,
    // self::udims::UDimType => UDim,
    Vector2: Vector2,
    Vector2int16: Vector2int16,
    Vector3: Vector3,
    Vector3int16: Vector3int16,
}
