//! Describes conversions that are allowed when deserializing properties from
//! the XML format.

use std::borrow::{Borrow, Cow};
use std::convert::TryInto;

use rbx_dom_weak::types::{BrickColor, Color3uint8, Tags, Variant, VariantType};

pub trait ConvertVariant: Clone + Sized {
    fn try_convert(self, target_type: VariantType) -> Result<Self, String> {
        Self::try_convert_cow(Cow::Owned(self), target_type).map(|value| value.into_owned())
    }

    fn try_convert_ref(&self, target_type: VariantType) -> Result<Cow<'_, Self>, String> {
        Self::try_convert_cow(Cow::Borrowed(self), target_type)
    }

    fn try_convert_cow(
        value: Cow<'_, Self>,
        target_type: VariantType,
    ) -> Result<Cow<'_, Self>, String>;
}

impl ConvertVariant for Variant {
    fn try_convert_cow(
        value: Cow<'_, Self>,
        target_type: VariantType,
    ) -> Result<Cow<'_, Self>, String> {
        match (value.borrow(), target_type) {
            (Variant::Int32(value), VariantType::BrickColor) => {
                let narrowed: u16 = (*value).try_into().map_err(|_| {
                    format!("Value {} is not in the range of a valid BrickColor", value)
                })?;

                BrickColor::from_number(narrowed)
                    .ok_or_else(|| format!("{} is not a valid BrickColor number", value))
                    .map(Into::into)
                    .map(Cow::Owned)
            }
            (Variant::Color3(value), VariantType::Color3uint8) => {
                Ok(Cow::Owned(Color3uint8::from(*value).into()))
            }
            (Variant::BinaryString(value), VariantType::Tags) => Ok(Cow::Owned(
                Tags::decode(value.to_owned().into_vec())
                    .map_err(|_| format!("Tags contain invalid UTF-8"))?
                    .into(),
            )),
            (_, _) => Ok(value),
        }
    }
}
