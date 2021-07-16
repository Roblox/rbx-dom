use crate::{
    Axes, BinaryString, BrickColor, CFrame, Color3, Color3uint8, ColorSequence, Content, Enum,
    Faces, NumberRange, NumberSequence, PhysicalProperties, Ray, Rect, Ref, Region3, Region3int16,
    SharedString, Tags, UDim, UDim2, Vector2, Vector2int16, Vector3, Vector3int16,
};

/// Reduces boilerplate from listing different values of Variant by wrapping
/// them into a macro.
macro_rules! make_variant {
    (
        $(
            $( #[$attr:meta] )*
            $variant_name:ident ($inner_type:ty),
        )*
    ) => {
        /// Represents any Roblox type. Useful for operating generically on
        /// Roblox instances.
        ///
        /// ## Stability
        ///
        /// New variants may be added to `Variant` in minor releases. As
        /// such, it is marked `#[non_exhaustive]`.
        #[derive(Debug, Clone, PartialEq)]
        #[non_exhaustive]
        #[cfg_attr(
            feature = "serde",
            derive(serde::Serialize, serde::Deserialize),
        )]
        pub enum Variant {
            $(
                $(
                    #[$attr]
                )*
                $variant_name($inner_type),
            )*
        }

        impl Variant {
            pub fn ty(&self) -> VariantType {
                match self {
                    $(
                        Variant::$variant_name(_) => VariantType::$variant_name,
                    )*
                }
            }
        }

        $(
            impl From<$inner_type> for Variant {
                fn from(value: $inner_type) -> Self {
                    Self::$variant_name(value)
                }
            }
        )*

        /// Represents any type that can be held in a `Variant`.
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        #[non_exhaustive]
        #[cfg_attr(
            feature = "serde",
            derive(serde::Serialize, serde::Deserialize),
        )]
        pub enum VariantType {
            $(
                $variant_name,
            )*
        }

        #[cfg(test)]
        mod generated_test {
            use super::*;

            /// This test makes sure that every type represented in `Variant`
            /// can be converted via `Into` into Variant.
            ///
            /// If we forget to impl From when new types are added to Variant,
            /// this test will start failing.
            #[allow(dead_code)]
            fn conversions_are_exhaustive() {
                fn trait_test<T: Into<Variant>>() {}

                $( trait_test::<$inner_type>(); )*
                trait_test::<SharedString>();
            }
        }
    };
}

// IMPORTANT! The order of this enum is very important in order to preserve the
// discriminant values that Rust assigns for both Variant and VariantType. Any
// newly-added variants MUST be added to the end!
make_variant! {
    Axes(Axes),
    BinaryString(BinaryString),
    Bool(bool),
    BrickColor(BrickColor),
    CFrame(CFrame),
    Color3(Color3),
    Color3uint8(Color3uint8),
    ColorSequence(ColorSequence),
    Content(Content),
    Enum(Enum),
    Faces(Faces),
    Float32(f32),
    Float64(f64),
    Int32(i32),
    Int64(i64),
    NumberRange(NumberRange),
    NumberSequence(NumberSequence),
    PhysicalProperties(PhysicalProperties),
    Ray(Ray),
    Rect(Rect),
    Ref(Ref),
    Region3(Region3),
    Region3int16(Region3int16),
    #[cfg_attr(
        feature = "serde",
        serde(with = "crate::shared_string::variant_serialization"),
    )]
    SharedString(SharedString),
    String(String),
    UDim(UDim),
    UDim2(UDim2),
    Vector2(Vector2),
    Vector2int16(Vector2int16),
    Vector3(Vector3),
    Vector3int16(Vector3int16),
    OptionalCFrame(Option<CFrame>),
    Tags(Tags),
}

impl From<&'_ str> for Variant {
    fn from(value: &str) -> Self {
        Self::String(value.to_owned())
    }
}

#[cfg(all(test, feature = "serde"))]
mod serde_test {
    use super::*;

    #[test]
    fn human() {
        let vec2 = Variant::Vector2(Vector2::new(5.0, 7.0));

        let ser = serde_json::to_string(&vec2).unwrap();
        assert_eq!(ser, r#"{"Vector2":[5.0,7.0]}"#);

        let de: Variant = serde_json::from_str(&ser).unwrap();
        assert_eq!(de, vec2);
    }

    #[test]
    fn non_human() {
        let vec2 = Variant::Vector2(Vector2::new(5.0, 7.0));

        let ser = bincode::serialize(&vec2).unwrap();

        let de: Variant = bincode::deserialize(&ser).unwrap();
        assert_eq!(de, vec2);
    }
}
