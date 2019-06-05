use std::{
    io::Read,
    collections::HashMap,
};

use log::trace;
use rbx_reflection::RbxPropertyTypeDescriptor;
use rbx_dom_weak::{RbxTree, RbxId, RbxInstanceProperties, RbxValue, RbxValueType, RbxValueConversion};

use crate::{
    core::find_canonical_property_descriptor,
    types::{read_value_xml, read_ref},
    error::{DecodeError as NewDecodeError, DecodeErrorKind},
};

use crate::deserializer_core::{XmlEventReader, XmlReadEvent};

pub fn decode_internal<R: Read>(source: R, options: DecodeOptions) -> Result<RbxTree, NewDecodeError> {
    let mut tree = RbxTree::new(RbxInstanceProperties {
        class_name: "DataModel".to_owned(),
        name: "DataModel".to_owned(),
        properties: HashMap::new(),
    });

    let root_id = tree.get_root_id();

    let mut iterator = XmlEventReader::from_source(source);
    let mut state = ParseState::new(&mut tree, options);

    deserialize_root(&mut iterator, &mut state, root_id)?;
    apply_id_rewrites(&mut state);

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

    pub(crate) fn use_reflection(&self) -> bool {
        self.property_behavior != DecodePropertyBehavior::NoReflection
    }
}

impl Default for DecodeOptions {
    fn default() -> DecodeOptions {
        DecodeOptions::new()
    }
}

struct IdPropertyRewrite {
    pub id: RbxId,
    pub property_name: String,
    pub referent_value: String,
}

struct SharedStringPropertyRewrite {
    pub id: RbxId,
    pub property_name: String,
    pub shared_string_hash: String,
}

/// The state needed to deserialize an XML model into an `RbxTree`.
pub struct ParseState<'a> {
    options: DecodeOptions,
    metadata: HashMap<String, String>,

    referents: HashMap<String, RbxId>,
    rewrite_ids: Vec<IdPropertyRewrite>,

    shared_strings: HashMap<String, Vec<u8>>,
    shared_string_rewrites: Vec<SharedStringPropertyRewrite>,

    tree: &'a mut RbxTree,
}

impl<'a> ParseState<'a> {
    fn new(tree: &mut RbxTree, options: DecodeOptions) -> ParseState {
        ParseState {
            options,
            metadata: HashMap::new(),
            referents: HashMap::new(),
            rewrite_ids: Vec::new(),
            shared_strings: HashMap::new(),
            shared_string_rewrites: Vec::new(),
            tree,
        }
    }

    /// Marks that a property on this instance needs to be rewritten once we
    /// have a complete view of how referents map to RbxId values.
    ///
    /// This is used to deserialize non-null Ref values correctly.
    pub fn add_id_rewrite(&mut self, id: RbxId, property_name: String, referent_value: String) {
        self.rewrite_ids.push(IdPropertyRewrite {
            id,
            property_name,
            referent_value,
        });
    }
}

fn apply_id_rewrites(state: &mut ParseState) {
    for rewrite in &state.rewrite_ids {
        let new_value = match state.referents.get(&rewrite.referent_value) {
            Some(id) => *id,
            None => continue
        };

        let instance = state.tree.get_instance_mut(rewrite.id)
            .expect("rbx_xml bug: had ID in referent map that didn't end up in the tree");

        instance.properties.insert(rewrite.property_name.clone(), RbxValue::Ref {
            value: Some(new_value),
        });
    }
}

fn deserialize_root<R: Read>(
    reader: &mut XmlEventReader<R>,
    state: &mut ParseState,
    parent_id: RbxId
) -> Result<(), NewDecodeError> {
    match reader.expect_next()? {
        XmlReadEvent::StartDocument { .. } => {},
        _ => unreachable!(),
    }

    {
        let attributes = reader.expect_start_with_name("roblox")?;

        let mut found_version = false;
        for attribute in attributes.into_iter() {
            if attribute.name.local_name == "version" {
                found_version = true;

                if attribute.value != "4" {
                    return Err(reader.error(DecodeErrorKind::WrongDocVersion(attribute.value)));
                }
            }
        }

        if !found_version {
            return Err(reader.error(DecodeErrorKind::MissingAttribute("version")));
        }
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

fn deserialize_metadata<R: Read>(reader: &mut XmlEventReader<R>, state: &mut ParseState) -> Result<(), NewDecodeError> {
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
) -> Result<(), NewDecodeError> {
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
) -> Result<(), NewDecodeError> {
    let attributes = reader.expect_start_with_name("SharedString")?;

    let mut md5_hash = None;
    for attribute in attributes.into_iter() {
        if attribute.name.local_name == "md5" {
            md5_hash = Some(attribute.value);
            break;
        }
    }

    let md5_hash = md5_hash
        .ok_or_else(|| reader.error(DecodeErrorKind::MissingAttribute("md5")))?;

    let encoded_value = reader.read_characters()?
        .replace("\n", "");

    let value = base64::decode(&encoded_value)
        .map_err(|e| reader.error(e))?;

    state.shared_strings.insert(md5_hash, value);

    reader.expect_end_with_name("SharedString")?;
    Ok(())
}

fn deserialize_instance<R: Read>(
    reader: &mut XmlEventReader<R>,
    state: &mut ParseState,
    parent_id: RbxId,
) -> Result<(), NewDecodeError> {
    let (class_name, referent) = {
        let attributes = reader.expect_start_with_name("Item")?;

        let mut class = None;
        let mut referent = None;

        for attribute in attributes.into_iter() {
            match attribute.name.local_name.as_str() {
                "class" => class = Some(attribute.value),
                "referent" => referent = Some(attribute.value),
                _ => {},
            }
        }

        let class = class
            .ok_or_else(|| reader.error(DecodeErrorKind::MissingAttribute("class")))?;

        (class, referent)
    };

    trace!("Class {} with referent {:?}", class_name, referent);

    let instance_props = RbxInstanceProperties {
        class_name,
        name: String::new(),
        properties: HashMap::new(),
    };

    let instance_id = state.tree.insert_instance(instance_props, parent_id);

    if let Some(referent) = referent {
        state.referents.insert(referent, instance_id);
    }

    // we have to collect properties in order to create the instance
    // name will be captured in this map and extracted later; XML doesn't store it separately
    let mut properties: HashMap<String, RbxValue> = HashMap::new();

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
            }
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

    let instance = state.tree.get_instance_mut(instance_id).unwrap();

    instance.name = match properties.remove("Name") {
        Some(value) => match value {
            RbxValue::String { value } => value,
            _ => return Err(reader.error(DecodeErrorKind::NameMustBeString(value.get_type()))),
        },
        None => instance.class_name.clone(),
    };

    instance.properties = properties;

    Ok(())
}

fn deserialize_properties<R: Read>(
    reader: &mut XmlEventReader<R>,
    state: &mut ParseState,
    instance_id: RbxId,
    props: &mut HashMap<String, RbxValue>,
) -> Result<(), NewDecodeError> {
    reader.expect_start_with_name("Properties")?;

    let class_name = state.tree.get_instance(instance_id)
        .expect("Couldn't find instance to deserialize properties into")
        .class_name.clone();

    loop {
        let (xml_type_name, xml_property_name) = {
            match reader.expect_peek()? {
                XmlReadEvent::StartElement { name, attributes, .. } => {
                    let mut xml_property_name = None;

                    for attribute in attributes {
                        if attribute.name.local_name.as_str() == "name" {
                            xml_property_name = Some(attribute.value.to_owned());
                            break;
                        }
                    }

                    let xml_property_name = match xml_property_name {
                        Some(value) => value,
                        None => return Err(reader.error(DecodeErrorKind::MissingAttribute("name")))
                    };

                    (name.local_name.to_owned(), xml_property_name)
                },
                XmlReadEvent::EndElement { name } => {
                    if name.local_name == "Properties" {
                        reader.expect_next()?;
                        return Ok(())
                    } else {
                        let err = DecodeErrorKind::UnexpectedXmlEvent(reader.expect_next()?);
                        return Err(reader.error(err));
                    }
                },
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
            let xml_value = if xml_type_name.as_str() == "Ref" {
                // Refs need additional state that we don't want to pass to
                // other property types unnecessarily, so we special-case it
                // here.

                read_ref(reader, instance_id, descriptor.name(), state)?
            } else {
                read_value_xml(reader, &xml_type_name)?
            };

            // The property descriptor might specify a different type than the
            // one we saw in the XML.
            //
            // This happens when property types are upgraded or if the
            // serialized data type is different than the canonical one.
            //
            // For example:
            // - Int/Float widening from 32-bit to 64-bit
            // - BrickColor properties turning into Color3
            let expected_type = match descriptor.property_type() {
                RbxPropertyTypeDescriptor::Data(value_type) => *value_type,
                RbxPropertyTypeDescriptor::Enum(_enum_name) => RbxValueType::Enum,
                RbxPropertyTypeDescriptor::UnimplementedType(_) => {
                    // Properties with types that aren't implemented yet are
                    // effectively unknown properties, so we handle them
                    // similarly.
                    match state.options.property_behavior {
                        DecodePropertyBehavior::IgnoreUnknown => {
                            // We've already read the value, we can just skip
                            // over it.
                            continue;
                        }
                        DecodePropertyBehavior::ReadUnknown => {
                            // This conversion will be returned into a no-op by
                            // try_convert_ref
                            xml_value.get_type()
                        }
                        DecodePropertyBehavior::ErrorOnUnknown => {
                            return Err(reader.error(DecodeErrorKind::UnknownProperty {
                                class_name,
                                property_name: xml_property_name,
                            }));
                        }
                        DecodePropertyBehavior::NoReflection | DecodePropertyBehavior::__Nonexhaustive => {
                            unreachable!();
                        }
                    }
                },
            };

            let value = match xml_value.try_convert_ref(expected_type) {
                // In this case, the property descriptor disagreed with the type
                // in the file, but there was a conversion available.
                RbxValueConversion::Converted(value) => value,

                // The property descriptor agreed with the type from the file,
                // or the type in the descriptor was unknown and the
                // deserializer is configured to ignore those issues
                RbxValueConversion::Unnecessary => xml_value,

                // The property descriptor disagreed, and there was no
                // conversion available. This is always an error.
                RbxValueConversion::Failed => {
                    return Err(reader.error(DecodeErrorKind::UnsupportedPropertyConversion {
                        class_name: class_name.clone(),
                        property_name: descriptor.name().to_string(),
                        expected_type,
                        actual_type: xml_value.get_type(),
                    }));
                }
            };

            props.insert(descriptor.name().to_string(), value);
        } else {
            match state.options.property_behavior {
                DecodePropertyBehavior::IgnoreUnknown => {
                    // We don't care about this property, so we can read it and
                    // throw it into the void.

                    read_value_xml(reader, &xml_type_name)?;
                }
                DecodePropertyBehavior::ReadUnknown | DecodePropertyBehavior::NoReflection => {
                    // We'll take this value as-is with no conversions on either
                    // the name or value.

                    let value = read_value_xml(reader, &xml_type_name)?;
                    props.insert(xml_property_name, value);
                }
                DecodePropertyBehavior::ErrorOnUnknown => {
                    return Err(reader.error(DecodeErrorKind::UnknownProperty {
                        class_name,
                        property_name: xml_property_name,
                    }));
                }
                DecodePropertyBehavior::__Nonexhaustive => unreachable!()
            }
        }
    }
}