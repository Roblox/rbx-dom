use std::{collections::HashMap, io::Read};

use log::trace;
use rbx_dom_weak::{
    types::{Ref, SharedString, Variant, VariantType},
    InstanceBuilder, WeakDom,
};
use rbx_reflection::PropertyType;

use crate::{
    compat::{TodoValueConversion, TodoValueConversionType},
    core::find_canonical_property_descriptor,
    error::{DecodeError, DecodeErrorKind},
    types::read_value_xml,
};

use crate::deserializer_core::{XmlEventReader, XmlReadEvent};

pub fn decode_internal<R: Read>(source: R, options: DecodeOptions) -> Result<WeakDom, DecodeError> {
    let mut tree = WeakDom::new(InstanceBuilder::new("DataModel"));

    let root_id = tree.root_ref();

    let mut iterator = XmlEventReader::from_source(source);
    let mut state = ParseState::new(&mut tree, options);

    deserialize_root(&mut iterator, &mut state, root_id)?;
    apply_referent_rewrites(&mut state);
    apply_shared_string_rewrites(&mut state);

    Ok(tree)
}

/// Describes the strategy that rbx_xml should use when deserializing
/// properties.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DecodePropertyBehavior {
    /// Ignores properties that aren't known by rbx_xml.
    ///
    /// The default and safest option. With this set, properties that are newer
    /// than the reflection database rbx_xml uses won't show up when
    /// deserializing files.
    IgnoreUnknown,

    /// Read properties that aren't known by rbx_xml.
    ///
    /// With this option set, properties that are newer than rbx_xml's
    /// reflection database will show up. It may be problematic to depend on
    /// these properties, since rbx_xml may start supporting them with
    /// non-reflection specific names at a future date.
    ReadUnknown,

    /// Returns an error if any properties are found that aren't known by
    /// rbx_xml.
    ErrorOnUnknown,

    /// Completely turns off rbx_xml's reflection database. Property names and
    /// types will appear exactly as they are in XML.
    ///
    /// This setting is useful for debugging the model format. It leaves the
    /// user to deal with oddities like how `Part.FormFactor` is actually
    /// serialized as `Part.formFactorRaw`.
    NoReflection,

    #[doc(hidden)]
    __Nonexhaustive,
}

/// Options available for deserializing an XML-format model or place.
#[derive(Debug, Clone)]
pub struct DecodeOptions {
    property_behavior: DecodePropertyBehavior,
}

impl DecodeOptions {
    /// Constructs a `DecodeOptions` with all values set to their defaults.
    #[inline]
    pub fn new() -> Self {
        DecodeOptions {
            property_behavior: DecodePropertyBehavior::IgnoreUnknown,
        }
    }

    /// Determines how rbx_xml will deserialize properties, especially unknown
    /// ones.
    #[inline]
    pub fn property_behavior(self, property_behavior: DecodePropertyBehavior) -> Self {
        DecodeOptions {
            property_behavior,
            ..self
        }
    }

    /// A utility function to determine whether or not we should reference the
    /// reflection database at all.
    pub(crate) fn use_reflection(&self) -> bool {
        self.property_behavior != DecodePropertyBehavior::NoReflection
    }
}

impl Default for DecodeOptions {
    fn default() -> DecodeOptions {
        DecodeOptions::new()
    }
}

/// The state needed to deserialize an XML model into an `WeakDom`.
pub struct ParseState<'a> {
    tree: &'a mut WeakDom,
    options: DecodeOptions,

    /// Metadata deserialized from 'Meta' fields in the file.
    /// Known fields are:
    /// - ExplicitAutoJoints
    metadata: HashMap<String, String>,

    /// A map referent strings to IDs. This map is filled up as instances are
    /// deserialized, and referred to when filling out Ref properties.
    ///
    /// We need to do that step in two passes because it's possible for
    /// instances to refer to instances that are later in the file.
    referents_to_ids: HashMap<String, Ref>,

    /// A list of Ref property rewrites to apply. After the first
    /// deserialization pass, we enumerate over this list and fill in the
    /// correct Ref value by using the referents map.
    referent_rewrites: Vec<ReferentRewrite>,

    /// A map from shared string hashes (currently MD5, decided by Roblox) to
    /// the actual SharedString type.
    known_shared_strings: HashMap<String, SharedString>,

    /// A list of SharedString properties to set in the tree as a secondary
    /// pass. This works just like referent rewriting since the shared string
    /// dictionary is usually at the end of the XML file.
    shared_string_rewrites: Vec<SharedStringRewrite>,
}

struct ReferentRewrite {
    id: Ref,
    property_name: String,
    referent_value: String,
}

struct SharedStringRewrite {
    id: Ref,
    property_name: String,
    shared_string_hash: String,
}

impl<'a> ParseState<'a> {
    fn new(tree: &mut WeakDom, options: DecodeOptions) -> ParseState {
        ParseState {
            tree,
            options,
            metadata: HashMap::new(),
            referents_to_ids: HashMap::new(),
            referent_rewrites: Vec::new(),
            known_shared_strings: HashMap::new(),
            shared_string_rewrites: Vec::new(),
        }
    }

    /// Marks that a property on this instance needs to be rewritten once we
    /// have a complete view of how referents map to Ref values.
    ///
    /// This is used to deserialize non-null Ref values correctly.
    pub fn add_referent_rewrite(&mut self, id: Ref, property_name: String, referent_value: String) {
        self.referent_rewrites.push(ReferentRewrite {
            id,
            property_name,
            referent_value,
        });
    }

    /// Marks that a property on this instance needs to be rewritten once we
    /// have a complete view of how referents map to Ref values.
    ///
    /// This is used to deserialize non-null Ref values correctly.
    pub fn add_shared_string_rewrite(
        &mut self,
        id: Ref,
        property_name: String,
        shared_string_hash: String,
    ) {
        self.shared_string_rewrites.push(SharedStringRewrite {
            id,
            property_name,
            shared_string_hash,
        });
    }
}

fn apply_referent_rewrites(state: &mut ParseState) {
    for rewrite in &state.referent_rewrites {
        let new_value = match state.referents_to_ids.get(&rewrite.referent_value) {
            Some(id) => *id,
            None => continue,
        };

        let instance = state
            .tree
            .get_by_ref_mut(rewrite.id)
            .expect("rbx_xml bug: had ID in referent rewrite list that didn't end up in the tree");

        instance
            .properties
            .insert(rewrite.property_name.clone(), Variant::Ref(new_value));
    }
}

fn apply_shared_string_rewrites(state: &mut ParseState) {
    for rewrite in &state.shared_string_rewrites {
        let new_value = match state.known_shared_strings.get(&rewrite.shared_string_hash) {
            Some(v) => v.clone(),
            None => continue,
        };

        let instance = state.tree.get_by_ref_mut(rewrite.id).expect(
            "rbx_xml bug: had ID in SharedString rewrite list that didn't end up in the tree",
        );

        instance.properties.insert(
            rewrite.property_name.clone(),
            Variant::SharedString(new_value),
        );
    }
}

fn deserialize_root<R: Read>(
    reader: &mut XmlEventReader<R>,
    state: &mut ParseState,
    parent_id: Ref,
) -> Result<(), DecodeError> {
    match reader.expect_next()? {
        XmlReadEvent::StartDocument { .. } => {}
        _ => unreachable!(),
    }

    let doc_attributes = reader.expect_start_with_name("roblox")?;

    let mut doc_version = None;

    for attribute in doc_attributes.into_iter() {
        match attribute.name.local_name.as_str() {
            "version" => doc_version = Some(attribute.value),
            _ => {}
        }
    }

    let doc_version =
        doc_version.ok_or_else(|| reader.error(DecodeErrorKind::MissingAttribute("version")))?;

    if doc_version != "4" {
        return Err(reader.error(DecodeErrorKind::WrongDocVersion(doc_version)));
    }

    loop {
        match reader.expect_peek()? {
            XmlReadEvent::StartElement { name, .. } => {
                match name.local_name.as_str() {
                    "Item" => {
                        deserialize_instance(reader, state, parent_id)?;
                    }
                    "External" => {
                        // This tag is always meaningless, there's nothing to do
                        // here except skip it.
                        reader.eat_unknown_tag()?;
                    }
                    "Meta" => {
                        deserialize_metadata(reader, state)?;
                    }
                    "SharedStrings" => {
                        deserialize_shared_string_dict(reader, state)?;
                    }
                    _ => {
                        let event = reader.expect_next().unwrap();
                        return Err(reader.error(DecodeErrorKind::UnexpectedXmlEvent(event)));
                    }
                }
            }
            XmlReadEvent::EndElement { name } => {
                if name.local_name == "roblox" {
                    reader.expect_next().unwrap();
                    break;
                } else {
                    let event = reader.expect_next().unwrap();
                    return Err(reader.error(DecodeErrorKind::UnexpectedXmlEvent(event)));
                }
            }
            XmlReadEvent::EndDocument => break,
            _ => {
                let event = reader.expect_next().unwrap();
                return Err(reader.error(DecodeErrorKind::UnexpectedXmlEvent(event)));
            }
        }
    }

    Ok(())
}

fn deserialize_metadata<R: Read>(
    reader: &mut XmlEventReader<R>,
    state: &mut ParseState,
) -> Result<(), DecodeError> {
    let name = {
        let attributes = reader.expect_start_with_name("Meta")?;

        let mut name = None;

        for attribute in attributes.into_iter() {
            match attribute.name.local_name.as_str() {
                "name" => name = Some(attribute.value),
                _ => {}
            }
        }

        name.ok_or_else(|| reader.error(DecodeErrorKind::MissingAttribute("name")))?
    };

    let value = reader.read_characters()?;
    reader.expect_end_with_name("Meta")?;

    state.metadata.insert(name, value);
    Ok(())
}

fn deserialize_shared_string_dict<R: Read>(
    reader: &mut XmlEventReader<R>,
    state: &mut ParseState,
) -> Result<(), DecodeError> {
    reader.expect_start_with_name("SharedStrings")?;

    loop {
        match reader.expect_peek()? {
            XmlReadEvent::StartElement { name, .. } => {
                if name.local_name == "SharedString" {
                    deserialize_shared_string(reader, state)?;
                } else {
                    let event = reader.expect_next().unwrap();
                    return Err(reader.error(DecodeErrorKind::UnexpectedXmlEvent(event)));
                }
            }
            XmlReadEvent::EndElement { name } => {
                if name.local_name == "SharedStrings" {
                    break;
                } else {
                    let event = reader.expect_next().unwrap();
                    return Err(reader.error(DecodeErrorKind::UnexpectedXmlEvent(event)));
                }
            }
            _ => {
                let event = reader.expect_next().unwrap();
                return Err(reader.error(DecodeErrorKind::UnexpectedXmlEvent(event)));
            }
        }
    }

    reader.expect_end_with_name("SharedStrings")?;
    Ok(())
}

fn deserialize_shared_string<R: Read>(
    reader: &mut XmlEventReader<R>,
    state: &mut ParseState,
) -> Result<(), DecodeError> {
    let attributes = reader.expect_start_with_name("SharedString")?;

    let mut md5_hash = None;
    for attribute in attributes.into_iter() {
        if attribute.name.local_name == "md5" {
            md5_hash = Some(attribute.value);
            break;
        }
    }

    let md5_hash =
        md5_hash.ok_or_else(|| reader.error(DecodeErrorKind::MissingAttribute("md5")))?;

    let buffer = reader.read_base64_characters()?;

    let value = SharedString::new(buffer);

    state.known_shared_strings.insert(md5_hash, value);

    reader.expect_end_with_name("SharedString")?;
    Ok(())
}

fn deserialize_instance<R: Read>(
    reader: &mut XmlEventReader<R>,
    state: &mut ParseState,
    parent_id: Ref,
) -> Result<(), DecodeError> {
    let (class_name, referent) = {
        let attributes = reader.expect_start_with_name("Item")?;

        let mut class = None;
        let mut referent = None;

        for attribute in attributes.into_iter() {
            match attribute.name.local_name.as_str() {
                "class" => class = Some(attribute.value),
                "referent" => referent = Some(attribute.value),
                _ => {}
            }
        }

        let class =
            class.ok_or_else(|| reader.error(DecodeErrorKind::MissingAttribute("class")))?;

        (class, referent)
    };

    trace!("Class {} with referent {:?}", class_name, referent);

    let builder = InstanceBuilder::new(class_name);
    let instance_id = state.tree.insert(parent_id, builder);

    if let Some(referent) = referent {
        state.referents_to_ids.insert(referent, instance_id);
    }

    let mut properties: HashMap<String, Variant> = HashMap::new();

    loop {
        match reader.expect_peek()? {
            XmlReadEvent::StartElement { name, .. } => match name.local_name.as_str() {
                "Properties" => {
                    deserialize_properties(reader, state, instance_id, &mut properties)?;
                }
                "Item" => {
                    deserialize_instance(reader, state, instance_id)?;
                }
                _ => {
                    let event = reader.expect_next().unwrap();
                    return Err(reader.error(DecodeErrorKind::UnexpectedXmlEvent(event)));
                }
            },
            XmlReadEvent::EndElement { name } => {
                if name.local_name != "Item" {
                    let event = reader.expect_next().unwrap();
                    return Err(reader.error(DecodeErrorKind::UnexpectedXmlEvent(event)));
                }

                reader.expect_next().unwrap();

                break;
            }
            _ => {
                let event = reader.expect_next().unwrap();
                return Err(reader.error(DecodeErrorKind::UnexpectedXmlEvent(event)));
            }
        }
    }

    let instance = state.tree.get_by_ref_mut(instance_id).unwrap();

    instance.name = match properties.remove("Name") {
        Some(value) => match value {
            Variant::String(value) => value,
            _ => return Err(reader.error(DecodeErrorKind::NameMustBeString(value.ty()))),
        },

        // TODO: Use reflection to get default name instead. This should only
        // matter for ValueBase instances in files created by tools other than
        // Roblox Studio.
        None => instance.class.clone(),
    };

    instance.properties = properties;

    Ok(())
}

fn deserialize_properties<R: Read>(
    reader: &mut XmlEventReader<R>,
    state: &mut ParseState,
    instance_id: Ref,
    props: &mut HashMap<String, Variant>,
) -> Result<(), DecodeError> {
    reader.expect_start_with_name("Properties")?;

    let class_name = state
        .tree
        .get_by_ref(instance_id)
        .expect("Couldn't find instance to deserialize properties into")
        .class
        .clone();

    loop {
        let (xml_type_name, xml_property_name) = {
            match reader.expect_peek()? {
                XmlReadEvent::StartElement {
                    name, attributes, ..
                } => {
                    let mut xml_property_name = None;

                    for attribute in attributes {
                        if attribute.name.local_name.as_str() == "name" {
                            xml_property_name = Some(attribute.value.to_owned());
                            break;
                        }
                    }

                    let xml_property_name = match xml_property_name {
                        Some(value) => value,
                        None => return Err(reader.error(DecodeErrorKind::MissingAttribute("name"))),
                    };

                    (name.local_name.to_owned(), xml_property_name)
                }
                XmlReadEvent::EndElement { name } => {
                    if name.local_name == "Properties" {
                        reader.expect_next()?;
                        return Ok(());
                    } else {
                        let err = DecodeErrorKind::UnexpectedXmlEvent(reader.expect_next()?);
                        return Err(reader.error(err));
                    }
                }
                _ => {
                    let err = DecodeErrorKind::UnexpectedXmlEvent(reader.expect_next()?);
                    return Err(reader.error(err));
                }
            }
        };

        let maybe_descriptor = if state.options.use_reflection() {
            find_canonical_property_descriptor(&class_name, &xml_property_name)
        } else {
            None
        };

        if let Some(descriptor) = maybe_descriptor {
            let xml_value =
                read_value_xml(reader, state, &xml_type_name, instance_id, &descriptor.name)?;

            // The property descriptor might specify a different type than the
            // one we saw in the XML.
            //
            // This happens when property types are upgraded or if the
            // serialized data type is different than the canonical one.
            //
            // For example:
            // - Int/Float widening from 32-bit to 64-bit
            // - BrickColor properties turning into Color3
            let expected_type = match &descriptor.value_type {
                PropertyType::Data(value_type) => *value_type,
                PropertyType::Enum(_enum_name) => VariantType::EnumValue,

                // FIXME?
                _ => unimplemented!(),
            };

            let value = match xml_value.try_convert_ref(expected_type) {
                // In this case, the property descriptor disagreed with the type
                // in the file, but there was a conversion available.
                TodoValueConversionType::Converted(value) => value,

                // The property descriptor agreed with the type from the file,
                // or the type in the descriptor was unknown and the
                // deserializer is configured to ignore those issues
                TodoValueConversionType::Unnecessary => xml_value,

                // The property descriptor disagreed, and there was no
                // conversion available. This is always an error.
                TodoValueConversionType::Failed => {
                    return Err(
                        reader.error(DecodeErrorKind::UnsupportedPropertyConversion {
                            class_name: class_name.clone(),
                            property_name: descriptor.name.to_string(),
                            expected_type,
                            actual_type: xml_value.ty(),
                        }),
                    );
                }
            };

            props.insert(descriptor.name.to_string(), value);
        } else {
            match state.options.property_behavior {
                DecodePropertyBehavior::IgnoreUnknown => {
                    // We don't care about this property, so we can read it and
                    // throw it into the void.

                    read_value_xml(
                        reader,
                        state,
                        &xml_type_name,
                        instance_id,
                        &xml_property_name,
                    )?;
                }
                DecodePropertyBehavior::ReadUnknown | DecodePropertyBehavior::NoReflection => {
                    // We'll take this value as-is with no conversions on either
                    // the name or value.

                    let value = read_value_xml(
                        reader,
                        state,
                        &xml_type_name,
                        instance_id,
                        &xml_property_name,
                    )?;
                    props.insert(xml_property_name, value);
                }
                DecodePropertyBehavior::ErrorOnUnknown => {
                    return Err(reader.error(DecodeErrorKind::UnknownProperty {
                        class_name,
                        property_name: xml_property_name,
                    }));
                }
                DecodePropertyBehavior::__Nonexhaustive => unreachable!(),
            }
        }
    }
}
