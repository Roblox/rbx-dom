use std::{convert::TryFrom, fmt};

#[cfg(any(test, feature = "unstable_text_format"))]
use serde::{Deserialize, Serialize};

use rbx_dom_weak::types::VariantType;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(
    any(test, feature = "unstable_text_format"),
    derive(Serialize, Deserialize)
)]
#[repr(u8)]
pub enum Type {
    String = 0x01,
    Bool = 0x02,
    Int32 = 0x03,
    Float32 = 0x04,
    Float64 = 0x05,
    UDim = 0x06,
    UDim2 = 0x07,
    Ray = 0x08,
    Faces = 0x09,
    Axes = 0x0A,
    BrickColor = 0x0B,
    Color3 = 0x0C,
    Vector2 = 0x0D,
    Vector3 = 0x0E,
    CFrame = 0x10,
    Enum = 0x12,
    Ref = 0x13,
    Vector3int16 = 0x14,
    NumberSequence = 0x15,
    ColorSequence = 0x16,
    NumberRange = 0x17,
    Rect = 0x18,
    PhysicalProperties = 0x19,
    Color3uint8 = 0x1A,
    Int64 = 0x1B,
    SharedString = 0x1C,
    OptionalCFrame = 0x1E,
}

impl Type {
    pub fn from_rbx_type(rbx_type: VariantType) -> Option<Type> {
        Some(match rbx_type {
            // These types all serialize the same way in the binary format.
            VariantType::String => Type::String,
            VariantType::BinaryString => Type::String,
            VariantType::Content => Type::String,
            VariantType::Tags => Type::String,

            VariantType::Bool => Type::Bool,
            VariantType::Int32 => Type::Int32,
            VariantType::Float32 => Type::Float32,
            VariantType::Float64 => Type::Float64,
            VariantType::UDim => Type::UDim,
            VariantType::UDim2 => Type::UDim2,
            VariantType::Ray => Type::Ray,
            VariantType::Faces => Type::Faces,
            VariantType::Axes => Type::Axes,
            VariantType::BrickColor => Type::BrickColor,
            VariantType::Color3 => Type::Color3,
            VariantType::Vector2 => Type::Vector2,
            VariantType::Vector3 => Type::Vector3,
            VariantType::CFrame => Type::CFrame,
            VariantType::Enum => Type::Enum,
            VariantType::Ref => Type::Ref,
            VariantType::Vector3int16 => Type::Vector3int16,
            VariantType::NumberSequence => Type::NumberSequence,
            VariantType::ColorSequence => Type::ColorSequence,
            VariantType::NumberRange => Type::NumberRange,
            VariantType::Rect => Type::Rect,
            VariantType::PhysicalProperties => Type::PhysicalProperties,
            VariantType::Color3uint8 => Type::Color3uint8,
            VariantType::Int64 => Type::Int64,
            VariantType::SharedString => Type::SharedString,
            VariantType::OptionalCFrame => Type::OptionalCFrame,
            _ => return None,
        })
    }

    pub fn to_default_rbx_type(self) -> Option<VariantType> {
        Some(match self {
            // Since many buffers aren't going to be valid UTF-8, it's safer to
            // pick BinaryString for unknown property types instead of String.
            Type::String => VariantType::BinaryString,
            Type::Bool => VariantType::Bool,
            Type::Int32 => VariantType::Int32,
            Type::Float32 => VariantType::Float32,
            Type::Float64 => VariantType::Float64,
            Type::UDim => VariantType::UDim,
            Type::UDim2 => VariantType::UDim2,
            Type::Ray => VariantType::Ray,
            Type::Faces => VariantType::Faces,
            Type::Axes => VariantType::Axes,
            Type::BrickColor => VariantType::BrickColor,
            Type::Color3 => VariantType::Color3,
            Type::Vector2 => VariantType::Vector2,
            Type::Vector3 => VariantType::Vector3,
            Type::CFrame => VariantType::CFrame,
            Type::Enum => VariantType::Enum,
            Type::Ref => VariantType::Ref,
            Type::Vector3int16 => VariantType::Vector3int16,
            Type::NumberSequence => VariantType::NumberSequence,
            Type::ColorSequence => VariantType::ColorSequence,
            Type::NumberRange => VariantType::NumberRange,
            Type::Rect => VariantType::Rect,
            Type::PhysicalProperties => VariantType::PhysicalProperties,
            Type::Color3uint8 => VariantType::Color3uint8,
            Type::Int64 => VariantType::Int64,
            Type::SharedString => VariantType::SharedString,
            Type::OptionalCFrame => VariantType::OptionalCFrame,
        })
    }
}

impl TryFrom<u8> for Type {
    type Error = InvalidTypeError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        use Type::*;

        Ok(match value {
            0x01 => String,
            0x02 => Bool,
            0x03 => Int32,
            0x04 => Float32,
            0x05 => Float64,
            0x06 => UDim,
            0x07 => UDim2,
            0x08 => Ray,
            0x09 => Faces,
            0x0A => Axes,
            0x0B => BrickColor,
            0x0C => Color3,
            0x0D => Vector2,
            0x0E => Vector3,
            0x10 => CFrame,
            0x12 => Enum,
            0x13 => Ref,
            0x14 => Vector3int16,
            0x15 => NumberSequence,
            0x16 => ColorSequence,
            0x17 => NumberRange,
            0x18 => Rect,
            0x19 => PhysicalProperties,
            0x1A => Color3uint8,
            0x1B => Int64,
            0x1C => SharedString,
            0x1E => OptionalCFrame,
            _ => return Err(InvalidTypeError(value)),
        })
    }
}

#[derive(Debug)]
pub struct InvalidTypeError(u8);

impl std::error::Error for InvalidTypeError {}

impl fmt::Display for InvalidTypeError {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "Invalid binary type value {:x?}", self.0)
    }
}
