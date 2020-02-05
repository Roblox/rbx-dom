// This file is also pulled in by generate_reflection in order to make sure that
// types don't get out of sync.

use std::{borrow::Cow, collections::HashMap};

use bitflags::bitflags;
use rbx_types::{Variant, VariantType};

/// Describes a class of Roblox instance. Classes relate to eachother via
/// inheritance and have properties attached to them.
///
/// All instance classes inherit directly or indirectly from `Instance`.
#[derive(Debug, PartialEq)]
pub struct RbxClassDescriptor {
    pub(crate) name: Cow<'static, str>,
    pub(crate) superclass: Option<Cow<'static, str>>,
    pub(crate) tags: RbxInstanceTags,
    pub(crate) properties: HashMap<Cow<'static, str>, RbxPropertyDescriptor>,
    pub(crate) default_properties: HashMap<Cow<'static, str>, Variant>,
}

impl RbxClassDescriptor {
    /// The name of the class as defined by Roblox.
    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The name of the class that this class inherits from, if it has one. The
    /// only instance without a superclass is `Instance`.
    #[inline]
    pub fn superclass(&self) -> Option<&str> {
        self.superclass.as_ref().map(AsRef::as_ref)
    }

    /// Locates the property descriptor on this class with the given name if it
    /// exists.
    ///
    /// Note that property descriptors inherited from a superclass will not be
    /// present here. If you want a complete view of all property descriptors
    /// for a class, you'll need to traverse up the inheritance chain and check
    /// _their_ property descriptor tables too.
    #[inline]
    pub fn get_property_descriptor<'a>(
        &'a self,
        property_name: &str,
    ) -> Option<&'a RbxPropertyDescriptor> {
        self.properties.get(property_name)
    }

    /// Returns an iterator over all property descriptors directly defined on
    /// this class.
    ///
    /// See the note on `get_property_descriptor` for caveats from inheritance.
    #[inline]
    pub fn iter_property_descriptors(
        &self,
    ) -> impl Iterator<Item = (&str, &RbxPropertyDescriptor)> {
        self.properties
            .iter()
            .map(|(key, value)| (key.as_ref(), value))
    }

    /// Returns the default value of the property with the given name, if one
    /// could be found by `rbx_reflection`.
    ///
    /// This will return default values from inherited properties, since values
    /// can be specialized.
    ///
    /// Not all properties will have default values due to the limitations of
    /// how rbx_reflection measures defaults.
    #[inline]
    pub fn get_default_value<'a>(&'a self, property_name: &str) -> Option<&'a Variant> {
        self.default_properties.get(property_name)
    }

    /// Returns an iterator over all default values on the class.
    ///
    /// See notes on `get_default_value` for inheritance interactions.
    #[inline]
    pub fn iter_default_values(&self) -> impl Iterator<Item = (&str, &Variant)> {
        self.default_properties
            .iter()
            .map(|(key, value)| (key.as_ref(), value))
    }

    /// Whether this instance is a service or not.
    ///
    /// Services can be loaded from `ServiceProvider` instances like `DataModel`
    /// using `GetService` from Lua.
    #[inline]
    pub fn is_service(&self) -> bool {
        self.tags.contains(RbxInstanceTags::SERVICE)
    }
}

/// Describes a property on a Roblox instance.
///
/// ## Canonical Properties
/// Canonical properties are considered the source of truth for a value and
/// are the preferred name when referring to a property.
///
/// Properties that are not canonical will generally link to a canonical
/// variant. Properties may have different types between canonical and
/// non-canonical forms, like `Part.BrickColor` and `Part.Color`.
///
/// There are generally two classes of non-canonical property that can overlap:
/// - Deprecated aliases, like `Camera.CoordinateFrame`
/// - Serialized forms, like `BasePart.Color3uint8` or `Fire.size_xml`
///
/// Serialized forms of canonical properties are _not_ reported as serializable.
/// Instead, their canonical forms will be. Mapping back to the serialized names
/// is up to a serialization implementation.
#[derive(Debug, PartialEq)]
pub struct RbxPropertyDescriptor {
    pub(crate) name: Cow<'static, str>,
    pub(crate) value_type: RbxPropertyTypeDescriptor,
    pub(crate) scriptability: RbxPropertyScriptability,
    pub(crate) is_canonical: bool,
    pub(crate) serializes: bool,
    pub(crate) canonical_name: Option<Cow<'static, str>>,
    pub(crate) serialized_name: Option<Cow<'static, str>>,

    pub(crate) tags: RbxPropertyTags,
}

impl RbxPropertyDescriptor {
    /// The name of the property, as defined by Roblox.
    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The type of the property.
    ///
    /// `RbxPropertyTypeDescriptor` is more detailed than `VariantType`: it
    /// contains extra information for enums and can also hold types that aren't
    /// yet implemented by rbx_dom_weak.
    #[inline]
    pub fn property_type(&self) -> &RbxPropertyTypeDescriptor {
        &self.value_type
    }

    /// Tells what kind of access there is to the property from Lua inside
    /// Roblox.
    #[inline]
    pub fn scriptability(&self) -> RbxPropertyScriptability {
        self.scriptability
    }

    /// Whether the property is considered _canonical_ by rbx_reflection.
    #[inline]
    pub fn is_canonical(&self) -> bool {
        self.is_canonical
    }

    /// Whether this property will serialize. Non-canonical properties are never
    /// marked as serializable. Instead, their canonical variant (given by
    /// `canonical_name()`) will be marked as serializable.
    #[inline]
    pub fn serializes(&self) -> bool {
        self.serializes
    }

    /// If this property is not canonical, gives the name of the canonical
    /// variant.
    #[inline]
    pub fn canonical_name(&self) -> Option<&str> {
        self.canonical_name.as_ref().map(AsRef::as_ref)
    }

    /// If this property is serializable and its serialized name is different
    /// than the name given by `name()`, it will be given by `serialized_name`.
    #[inline]
    pub fn serialized_name(&self) -> Option<&str> {
        self.serialized_name.as_ref().map(AsRef::as_ref)
    }
}

/// Describes a Roblox enum.
///
/// This data is generated from the official Roblox JSON API dump and should be
/// completely accurate.
///
/// Enums have a name and zero or more variants, which are represented as `u32`
/// values.
#[derive(Debug, PartialEq)]
pub struct RbxEnumDescriptor {
    pub(crate) name: Cow<'static, str>,
    pub(crate) items: HashMap<Cow<'static, str>, u32>,
}

impl RbxEnumDescriptor {
    /// The name of the enum as reported by Roblox.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns an item from the enum by name, if it exists.
    pub fn get_item<'a>(&'a self, name: &str) -> Option<u32> {
        self.items.get(name).cloned()
    }

    /// Returns an iterator over all items in this enum and their names.
    pub fn iter_items(&self) -> impl Iterator<Item = (&str, u32)> {
        self.items.iter().map(|(key, value)| (key.as_ref(), *value))
    }
}

/// Describes the type of an instance property.
#[derive(Debug, Clone, PartialEq)]
pub enum RbxPropertyTypeDescriptor {
    /// The property is a regular value of the given type.
    Data(VariantType),

    /// The property is an enum with the given name.
    Enum(Cow<'static, str>),

    /// The property is a type that isn't representable by rbx_dom_weak with the
    /// given name. These kinds of types may be converted to another property
    /// type at any time.
    UnimplementedType(Cow<'static, str>),
}

/// Describes what kinds of access are allowed to a property from a script
/// running inside Roblox.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RbxPropertyScriptability {
    /// The property is not scriptable at all.
    None,

    /// The property can be read from or written to with regular assignments.
    ReadWrite,

    /// The property can only be read from.
    Read,

    /// The property can only be written to.
    Write,

    /// The property can only be modified indirectly.
    ///
    /// A common example is the `Tags` property, which is writable through
    /// methods on `CollectionService`.
    Custom,
}

/// The bitflags crate doesn't support iterating over the bits that are set in
/// the flag. In order to generate lists of flag names, we create a macro that
/// abstracts over the bitflags macro and additionally implements IntoIterator
/// on the type.
///
/// To avoid pulling in a dependency on either the `paste!` or `concat_idents!`
/// macros, the caller has to pass inthe name of the iterator type to define.
macro_rules! bitterflag {
    ($struct_name: ident + $iter_name: ident : $width: ident { $(const $const_name: ident = $const_value: expr;)* }) => {
        bitflags! {
            pub(crate) struct $struct_name: $width {
                $(const $const_name = $const_value;)*
            }
        }

        pub(crate) struct $iter_name {
            inner: Box<dyn Iterator<Item = $struct_name>>,
        }

        impl Iterator for $iter_name {
            type Item = $struct_name;

            fn next(&mut self) -> Option<Self::Item> {
                self.inner.next()
            }
        }

        impl IntoIterator for $struct_name {
            type Item = Self;
            type IntoIter = $iter_name;

            fn into_iter(self) -> Self::IntoIter {
                static ALL_TAGS: &[$struct_name] = &[
                    $($struct_name::$const_name,)*
                ];

                $iter_name {
                    inner: Box::new(
                        ALL_TAGS
                            .iter()
                            .cloned()
                            .filter(move |flag| self.contains(*flag)),
                    ),
                }
            }
        }
    };
}

// Tags found via:
// jq '[.Classes | .[] | .Tags // empty] | add | unique' api-dump.json
bitterflag! {
    RbxInstanceTags + RbxInstanceTagsIntoIter: u32 {
        const DEPRECATED = 0x1;
        const NOT_BROWSABLE = 0x2;
        const NOT_CREATABLE = 0x4;
        const NOT_REPLICATED = 0x8;
        const PLAYER_REPLICATED = 0x10;
        const SERVICE = 0x20;
        const SETTINGS = 0x40;
    }
}

// Tags found via:
// jq '[.Classes | .[] | .Members | .[] | select(.MemberType == "Property") | .Tags // empty] | add | unique' api-dump.json
bitterflag! {
    RbxPropertyTags + RbxPropertyTagsIntoIter: u32 {
        const DEPRECATED = 0x1;
        const HIDDEN = 0x2;
        const NOT_BROWSABLE = 0x4;
        const NOT_REPLICATED = 0x8;
        const NOT_SCRIPTABLE = 0x10;
        const READ_ONLY = 0x20;
    }
}
