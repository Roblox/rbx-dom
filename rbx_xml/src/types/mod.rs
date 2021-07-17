//! This file packs up all of the type implementations in rbx_xml and exposes
//! them through two methods, `read_value_xml` and `write_value_xml`.
//!
//! To support a new type in rbx_xml:
//!
//! 1. Implement the type in a submodule (with tests!)
//! 2. Add a 'mod' statement immediately below this comment
//! 3. Add the type(s) to the declare_rbx_types! macro invocation

mod axes;
mod binary_string;
mod bool;
mod cframe;
mod color_sequence;
mod colors;
mod content;
mod enumeration;
mod faces;
mod number_range;
mod number_sequence;
mod numbers;
mod optional_cframe;
mod physical_properties;
mod ray;
mod rect;
mod referent;
mod shared_string;
mod strings;
mod tags;
mod udims;
mod vectors;

use std::io::{Read, Write};

use rbx_dom_weak::types::{
    Axes, BinaryString, CFrame, Color3, Color3uint8, ColorSequence, Content, Enum, Faces,
    NumberRange, NumberSequence, PhysicalProperties, Ray, Rect, Ref, Tags, UDim, UDim2, Variant,
    Vector2, Vector2int16, Vector3, Vector3int16,
};

use crate::{
    core::XmlType,
    deserializer::ParseState,
    deserializer_core::XmlEventReader,
    error::{DecodeError, EncodeError, EncodeErrorKind},
    serializer::EmitState,
    serializer_core::XmlEventWriter,
};

use self::{
    referent::{read_ref, write_ref},
    shared_string::{read_shared_string, write_shared_string},
};

/// The `declare_rbx_types` macro generates the two big match statements that
/// rbx_xml uses to read/write values inside of `read_value_xml` and
/// `write_value_xml`.
macro_rules! declare_rbx_types {
    { $($variant_name: ident : $inner_type: ty,)* } => {

        /// Reads a Roblox property value with the given type from the XML event
        /// stream.
        pub fn read_value_xml<R: Read>(
            reader: &mut XmlEventReader<R>,
            state: &mut ParseState,
            xml_type_name: &str,
            instance_id: Ref,
            property_name: &str,
        ) -> Result<Option<Variant>, DecodeError> {
            match xml_type_name {
                $(<$inner_type>::XML_TAG_NAME => Ok(Some(Variant::$variant_name(<$inner_type>::read_outer_xml(reader)?))),)*

                // Protected strings are only read, never written
                self::strings::ProtectedStringDummy::XML_TAG_NAME => {
                    let value = self::strings::ProtectedStringDummy::read_outer_xml(reader)?;
                    Ok(Some(Variant::String(value.0)))
                },

                self::referent::XML_TAG_NAME => Ok(Some(Variant::Ref(read_ref(reader, instance_id, property_name, state)?))),
                self::shared_string::XML_TAG_NAME => read_shared_string(reader, instance_id, property_name, state).map(Some),

                _ => {
                    state.unknown_type_visited(instance_id, property_name, xml_type_name);
                    reader.eat_unknown_tag()?;

                    Ok(None)
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
                Variant::BrickColor(value) =>
                    (*value as i32).write_outer_xml(xml_property_name, writer),

                Variant::Ref(value) => write_ref(writer, xml_property_name, *value, state),
                Variant::SharedString(value) => write_shared_string(writer, xml_property_name, value, state),

                unknown => {
                    Err(writer.error(EncodeErrorKind::UnsupportedPropertyType(unknown.ty())))
                },
            }
        }
    }
}

declare_rbx_types! {
    Axes: Axes,
    BinaryString: BinaryString,
    Bool: bool,
    CFrame: CFrame,
    Color3: Color3,
    Color3uint8: Color3uint8,
    ColorSequence: ColorSequence,
    Content: Content,
    Enum: Enum,
    Faces: Faces,
    Float32: f32,
    Float64: f64,
    Int32: i32,
    Int64: i64,
    NumberRange: NumberRange,
    NumberSequence: NumberSequence,
    OptionalCFrame: Option<CFrame>,
    PhysicalProperties: PhysicalProperties,
    Ray: Ray,
    Rect: Rect,
    String: String,
    UDim2: UDim2,
    UDim: UDim,
    Vector2: Vector2,
    Vector2int16: Vector2int16,
    Vector3: Vector3,
    Vector3int16: Vector3int16,
    Tags: Tags,
}
