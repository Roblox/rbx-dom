use std::{borrow::Cow, collections::HashMap, io::Write};

use rbx_dom_weak::{
    types::{Ref, SharedString, Variant, VariantType},
    WeakDom,
};
use rbx_reflection::PropertyType;

use crate::{
    compat::{TodoValueConversion, TodoValueConversionType},
    core::find_serialized_property_descriptor,
    error::{EncodeError as NewEncodeError, EncodeErrorKind},
    types::write_value_xml,
};

use crate::serializer_core::{XmlEventWriter, XmlWriteEvent};

pub fn encode_internal<W: Write>(
    output: W,
    tree: &WeakDom,
    ids: &[Ref],
    options: EncodeOptions,
) -> Result<(), NewEncodeError> {
    let mut writer = XmlEventWriter::from_output(output);
    let mut state = EmitState::new(options);

    writer.write(XmlWriteEvent::start_element("roblox").attr("version", "4"))?;

    let mut property_buffer = Vec::new();
    for id in ids {
        serialize_instance(&mut writer, &mut state, tree, *id, &mut property_buffer)?;
    }

    serialize_shared_strings(&mut writer, &mut state)?;

    writer.write(XmlWriteEvent::end_element())?;

    Ok(())
}

/// Describes the strategy that rbx_xml should use when serializing properties.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EncodePropertyBehavior {
    /// Ignores properties that aren't known by rbx_xml.
    ///
    /// This is the default.
    IgnoreUnknown,

    /// Write unrecognized properties.
    ///
    /// With this option set, properties that are newer than rbx_xml's
    /// reflection database will show up. It may be problematic to depend on
    /// these properties, since rbx_xml may start supporting them with
    /// non-reflection specific names at a future date.
    WriteUnknown,

    /// Returns an error if any properties are found that aren't known by
    /// rbx_xml.
    ErrorOnUnknown,

    /// Completely turns off rbx_xml's reflection database. Property names and
    /// types will appear exactly as they are in the tree.
    ///
    /// This setting is useful for debugging the model format. It leaves the
    /// user to deal with oddities like how `Part.FormFactor` is actually
    /// serialized as `Part.formFactorRaw`.
    NoReflection,

    #[doc(hidden)]
    __Nonexhaustive,
}

/// Options available for serializing an XML-format model or place.
#[derive(Debug, Clone)]
pub struct EncodeOptions {
    property_behavior: EncodePropertyBehavior,
}

impl EncodeOptions {
    /// Constructs a `EncodeOptions` with all values set to their defaults.
    #[inline]
    pub fn new() -> Self {
        EncodeOptions {
            property_behavior: EncodePropertyBehavior::IgnoreUnknown,
        }
    }

    /// Determines how rbx_xml will serialize properties, especially unknown
    /// ones.
    #[inline]
    pub fn property_behavior(self, property_behavior: EncodePropertyBehavior) -> Self {
        EncodeOptions {
            property_behavior,
            ..self
        }
    }

    pub(crate) fn use_reflection(&self) -> bool {
        self.property_behavior != EncodePropertyBehavior::NoReflection
    }
}

impl Default for EncodeOptions {
    fn default() -> EncodeOptions {
        EncodeOptions::new()
    }
}

pub struct EmitState {
    options: EncodeOptions,

    /// A map of IDs written so far to the generated referent that they use.
    /// This map is used to correctly emit Ref properties.
    referent_map: HashMap<Ref, u32>,

    /// The referent value that will be used for emitting the next instance.
    next_referent: u32,

    /// A map of all shared strings referenced so far while generating XML. This
    /// map will be written as the file's SharedString dictionary.
    shared_strings_to_emit: HashMap<[u8; 16], SharedString>,
}

impl EmitState {
    pub fn new(options: EncodeOptions) -> EmitState {
        EmitState {
            options,
            referent_map: HashMap::new(),
            next_referent: 0,
            shared_strings_to_emit: HashMap::new(),
        }
    }

    pub fn map_id(&mut self, id: Ref) -> u32 {
        match self.referent_map.get(&id) {
            Some(&value) => value,
            None => {
                let referent = self.next_referent;
                self.referent_map.insert(id, referent);
                self.next_referent += 1;
                referent
            }
        }
    }

    pub fn add_shared_string(&mut self, value: SharedString) {
        // FIXME
        // self.shared_strings_to_emit.insert(value.md5_hash(), value);
    }
}

/// Serialize a single instance.
///
/// `property_buffer` is a Vec that can be reused between calls to
/// serialize_instance to make sorting properties more efficient.
fn serialize_instance<'a, W: Write>(
    writer: &mut XmlEventWriter<W>,
    state: &mut EmitState,
    tree: &'a WeakDom,
    id: Ref,
    property_buffer: &mut Vec<(&'a String, &'a Variant)>,
) -> Result<(), NewEncodeError> {
    let instance = tree.get_by_ref(id).unwrap();
    let mapped_id = state.map_id(id);

    writer.write(
        XmlWriteEvent::start_element("Item")
            .attr("class", &instance.class)
            .attr("referent", &mapped_id.to_string()),
    )?;

    writer.write(XmlWriteEvent::start_element("Properties"))?;

    write_value_xml(
        writer,
        state,
        "Name",
        &Variant::String(instance.name.clone()),
    )?;

    // Move references to our properties into property_buffer so we can sort
    // them and iterate them in order.
    property_buffer.extend(&instance.properties);
    property_buffer.sort_unstable_by_key(|(key, _)| *key);

    for (property_name, value) in property_buffer.drain(..) {
        let maybe_serialized_descriptor = if state.options.use_reflection() {
            find_serialized_property_descriptor(&instance.class, property_name)
        } else {
            None
        };

        if let Some(serialized_descriptor) = maybe_serialized_descriptor {
            let value_type = match &serialized_descriptor.value_type {
                PropertyType::Data(value_type) => *value_type,
                PropertyType::Enum(_enum_name) => VariantType::EnumValue,

                // FIXME?
                _ => unimplemented!(),
            };

            let converted_value = match value.try_convert_ref(value_type) {
                TodoValueConversionType::Converted(converted) => Cow::Owned(converted),
                TodoValueConversionType::Unnecessary => Cow::Borrowed(value),
                TodoValueConversionType::Failed => {
                    return Err(
                        writer.error(EncodeErrorKind::UnsupportedPropertyConversion {
                            class_name: instance.class.clone(),
                            property_name: property_name.to_string(),
                            expected_type: value_type,
                            actual_type: value.ty(),
                        }),
                    )
                }
            };

            write_value_xml(writer, state, &serialized_descriptor.name, &converted_value)?;
        } else {
            match state.options.property_behavior {
                EncodePropertyBehavior::IgnoreUnknown => {}
                EncodePropertyBehavior::WriteUnknown | EncodePropertyBehavior::NoReflection => {
                    // We'll take this value as-is with no conversions on
                    // either the name or value.

                    write_value_xml(writer, state, property_name, value)?;
                }
                EncodePropertyBehavior::ErrorOnUnknown => {
                    return Err(writer.error(EncodeErrorKind::UnknownProperty {
                        class_name: instance.class.clone(),
                        property_name: property_name.clone(),
                    }));
                }
                EncodePropertyBehavior::__Nonexhaustive => unreachable!(),
            }
        }
    }

    writer.write(XmlWriteEvent::end_element())?;

    for child_id in instance.children() {
        serialize_instance(writer, state, tree, *child_id, property_buffer)?;
    }

    writer.write(XmlWriteEvent::end_element())?;

    Ok(())
}

fn serialize_shared_strings<W: Write>(
    writer: &mut XmlEventWriter<W>,
    state: &mut EmitState,
) -> Result<(), NewEncodeError> {
    if state.shared_strings_to_emit.is_empty() {
        return Ok(());
    }

    writer.write(XmlWriteEvent::start_element("SharedStrings"))?;

    for value in state.shared_strings_to_emit.values() {
        writer.write(
            XmlWriteEvent::start_element("SharedString")
                // FIXME
                // .attr("md5", &base64::encode(&value.md5_hash())),
        )?;

        writer.write_string(&base64::encode(value.data()))?;
        writer.end_element()?;
    }

    writer.end_element()?;
    Ok(())
}
