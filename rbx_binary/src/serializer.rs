use std::{
    borrow::Cow,
    collections::{BTreeMap, HashMap},
    io::{self, Write},
    u32,
};

use byteorder::{LittleEndian, WriteBytesExt};
use rbx_dom_weak::{RbxId, RbxTree, RbxValue, RbxValueType};
use rbx_reflection::RbxPropertyTypeDescriptor;
use snafu::Snafu;

use crate::{
    chunk::{ChunkBuilder, Compression},
    core::{
        find_property_descriptors, RbxWriteExt, FILE_MAGIC_HEADER, FILE_SIGNATURE, FILE_VERSION,
    },
    types::Type,
};

static FILE_FOOTER: &[u8] = b"</roblox>";

#[derive(Debug, Snafu)]
pub struct Error(InnerError);

#[derive(Debug, Snafu)]
enum InnerError {
    #[snafu(display("{}", source))]
    Io { source: io::Error },
}

impl From<io::Error> for Error {
    fn from(source: io::Error) -> Self {
        Error(InnerError::Io { source })
    }
}

pub fn encode<W: Write>(tree: &RbxTree, ids: &[RbxId], output: W) -> Result<(), Error> {
    let mut serializer = BinarySerializer::new(tree, output);

    for id in ids {
        serializer.add_instance(*id);
    }

    log::debug!("Type info discovered: {:#?}", serializer.type_infos);

    serializer.generate_referents();

    log::trace!("Referents constructed: {:#?}", serializer.id_to_referent);

    serializer.write_header()?;
    serializer.serialize_metadata()?;
    serializer.serialize_instances()?;
    serializer.serialize_properties()?;
    serializer.serialize_parents()?;
    serializer.serialize_end()?;

    Ok(())
}

/// Represents all of the state during a single serialization session. A new
/// `BinarySerializer` object should be created every time we want to serialize
/// a binary model file.
struct BinarySerializer<'a, W> {
    /// The tree containing all of the instances that we're serializing.
    tree: &'a RbxTree,

    /// Where the binary output should be written.
    output: W,

    /// All of the instances, in a deterministic order, that we're going to be
    /// serializing.
    relevant_instances: Vec<RbxId>,

    /// A map from rbx-dom's unique instance ID (RbxId) to the ID space used in
    /// the binary model format, signed integers.
    id_to_referent: HashMap<RbxId, i32>,

    /// All of the types of instance discovered by our serializer that we'll be
    /// writing into the output.
    ///
    /// These are stored sorted so that we naturally iterate over them in order
    /// and improve our chances of being deterministic.
    type_infos: BTreeMap<String, TypeInfo>,

    /// The next type ID that should be assigned if a type is discovered and
    /// added to the serializer.
    next_type_id: u32,
}

/// An instance class that our serializer knows about. We should have one struct
/// per unique ClassName.
#[derive(Debug)]
struct TypeInfo {
    /// The ID that this serializer will use to refer to this type of instance.
    type_id: u32,

    /// Whether this type is considered a service. Only one copy of a given
    /// service can exist for a given ServiceProvider. DataModel is the only
    /// ServiceProvider in most projects.
    is_service: bool,

    /// The IDs of all of the instances of this type.
    object_ids: Vec<RbxId>,

    /// All of the defined properties for this type found on any instance of
    /// this type.
    ///
    /// Stored in a sorted map to try to ensure that we write out properties in
    /// a deterministic order.
    properties: BTreeMap<String, PropInfo>,
}

#[derive(Debug)]
struct PropInfo {
    prop_type: Type,
    default_value: Cow<'static, RbxValue>,
}

impl<'a, W: Write> BinarySerializer<'a, W> {
    fn new(tree: &'a RbxTree, output: W) -> Self {
        BinarySerializer {
            tree,
            output,
            relevant_instances: Vec::new(),
            id_to_referent: HashMap::new(),
            type_infos: BTreeMap::new(),
            next_type_id: 0,
        }
    }

    /// Mark the given instance ID and all of its descendants as intended for
    /// serialization with this serializer.
    fn add_instance(&mut self, id: RbxId) {
        self.relevant_instances.push(id);
        self.collect_type_info(id);

        for descendant in self.tree.descendants(id) {
            self.relevant_instances.push(descendant.get_id());
            self.collect_type_info(descendant.get_id());
        }
    }

    /// Collect information about all the different types of instance and their
    /// properties.
    fn collect_type_info(&mut self, id: RbxId) {
        let instance = self
            .tree
            .get_instance(id)
            .expect("Instance did not exist in tree");

        let type_info = self.get_or_create_type_info(&instance.class_name);
        type_info.object_ids.push(id);

        for (prop_name, prop_value) in &instance.properties {
            let ser_name;
            let ser_rbx_type;
            let mut ser_default_value = None;

            match find_property_descriptors(&instance.class_name, prop_name) {
                Some((_canon_descriptor, ser_descriptor, ser_default)) => {
                    ser_name = ser_descriptor.name();
                    ser_rbx_type = match ser_descriptor.property_type() {
                        RbxPropertyTypeDescriptor::Data(ty) => *ty,
                        RbxPropertyTypeDescriptor::Enum(_) => RbxValueType::Enum,
                        RbxPropertyTypeDescriptor::UnimplementedType(name) => {
                            log::info!("Unimplemented data type {}", name);

                            // TODO: Configurable handling of unknown types?
                            return;
                        }
                    };
                    ser_default_value = ser_default;
                }
                None => {
                    ser_name = prop_name.as_str();
                    ser_rbx_type = prop_value.get_type();
                }
            }

            let ser_type = Type::from_rbx_type(ser_rbx_type).unwrap();

            let ser_default_value = ser_default_value.map(Cow::Borrowed).unwrap_or_else(|| {
                Cow::Owned(match ser_type {
                    Type::String => RbxValue::String {
                        value: "".to_owned(),
                    },
                    Type::Bool => RbxValue::Bool { value: false },
                    _ => unimplemented!(),
                })
            });

            if !type_info.properties.contains_key(ser_name) {
                type_info.properties.insert(
                    ser_name.to_owned(),
                    PropInfo {
                        prop_type: ser_type,
                        default_value: ser_default_value,
                    },
                );
            }
        }
    }

    /// Finds the type info from the given class name if it exists, or creates
    /// one and returns a reference to it if not.
    fn get_or_create_type_info(&mut self, class_name: &str) -> &mut TypeInfo {
        if !self.type_infos.contains_key(class_name) {
            let type_id = self.next_type_id;
            self.next_type_id += 1;

            let is_service;

            if let Some(class_descriptor) = rbx_reflection::get_class_descriptor(class_name) {
                is_service = class_descriptor.is_service();
            } else {
                log::info!("The class {} is not known to rbx_binary", class_name);
                is_service = false;
            };

            let mut properties = BTreeMap::new();

            // Every instance has a property named Name. Even though
            // rbx_dom_weak encodes the name property specially, we still insert
            // this property into the type info and handle it like a regular
            // property during encoding.
            properties.insert(
                "Name".to_owned(),
                PropInfo {
                    prop_type: Type::String,
                    default_value: Cow::Owned(RbxValue::String {
                        value: "".to_owned(),
                    }),
                },
            );

            self.type_infos.insert(
                class_name.to_owned(),
                TypeInfo {
                    type_id,
                    is_service,
                    object_ids: Vec::new(),
                    properties,
                },
            );
        }

        self.type_infos.get_mut(class_name).unwrap()
    }

    /// Populate the map from rbx-dom's instance ID space to the IDs that we'll
    /// be serializing to the model.
    fn generate_referents(&mut self) {
        self.id_to_referent.reserve(self.relevant_instances.len());

        let mut next_referent = 0;

        for id in &self.relevant_instances {
            self.id_to_referent.insert(*id, next_referent);
            next_referent += 1;
        }
    }

    fn write_header(&mut self) -> io::Result<()> {
        log::trace!("Writing header");

        self.output.write_all(FILE_MAGIC_HEADER)?;
        self.output.write_all(FILE_SIGNATURE)?;
        self.output.write_u16::<LittleEndian>(FILE_VERSION)?;

        self.output
            .write_u32::<LittleEndian>(self.type_infos.len() as u32)?;
        self.output
            .write_u32::<LittleEndian>(self.relevant_instances.len() as u32)?;
        self.output.write_u64::<LittleEndian>(0)?;

        Ok(())
    }

    /// Write out any metadata about this file, stored in a chunk named META.
    fn serialize_metadata(&mut self) -> io::Result<()> {
        log::trace!("Writing metadata (currently no-op)");
        // TODO: There is no concept of metadata in a tree yet.
        Ok(())
    }

    /// Write out the declarations of all instances, stored in a series of
    /// chunks named INST.
    fn serialize_instances(&mut self) -> io::Result<()> {
        log::trace!("Writing instance chunks");

        for (type_name, type_info) in &self.type_infos {
            log::trace!(
                "Writing chunk for {} ({} instances)",
                type_name,
                type_info.object_ids.len()
            );

            let mut chunk = ChunkBuilder::new(b"INST", Compression::Compressed);

            chunk.write_u32::<LittleEndian>(type_info.type_id)?;
            chunk.write_string(type_name)?;

            // It's possible that this integer will be expanded in the future to
            // be a general version/format field instead of just service vs
            // non-service.
            //
            // At that point, we'll start thinking about it like it's a u8
            // instead of a bool.
            chunk.write_bool(type_info.is_service)?;

            chunk.write_u32::<LittleEndian>(type_info.object_ids.len() as u32)?;

            chunk.write_referents(
                type_info
                    .object_ids
                    .iter()
                    .map(|id| self.id_to_referent[id]),
            )?;

            if type_info.is_service {
                // It's unclear what this byte is used for, but when the type is
                // a service (like Workspace, Lighting, etc), we need to write
                // the value `1` for every instance in our file of that type.
                //
                // In 99.9% of cases, there's only going to be one copy of a
                // given service, so we're not worried about doing this super
                // efficiently.
                for _ in 0..type_info.object_ids.len() {
                    chunk.write_u8(1)?;
                }
            }

            chunk.dump(&mut self.output)?;
        }

        Ok(())
    }

    /// Write out batch declarations of property values for the instances
    /// previously defined in the INST chunks. Property data is contained in
    /// chunks named PROP.
    fn serialize_properties(&mut self) -> io::Result<()> {
        log::trace!("Writing properties");

        for (type_name, type_info) in &self.type_infos {
            for (prop_name, prop_info) in &type_info.properties {
                log::trace!(
                    "Writing property {}.{} (type {:?})",
                    type_name,
                    prop_name,
                    prop_info.prop_type
                );

                let mut chunk = ChunkBuilder::new(b"PROP", Compression::Compressed);

                chunk.write_u32::<LittleEndian>(type_info.type_id)?;
                chunk.write_string(&prop_name)?;
                chunk.write_u8(prop_info.prop_type as u8)?;

                let tree = &self.tree;
                let values = type_info.object_ids.iter().map(|id| {
                    let instance = tree.get_instance(*id).unwrap();

                    // We store the Name property in a different field for
                    // convenience, but when serializing to the binary model
                    // format we need to handle it just like other properties.
                    if prop_name == "Name" {
                        Cow::Owned(RbxValue::String {
                            value: instance.name.clone(),
                        })
                    } else {
                        // TODO: Fall back to default value for this property
                        // descriptor.
                        Cow::Borrowed(instance.properties.get(prop_name).unwrap())
                    }
                });

                match prop_info.prop_type {
                    Type::String => {
                        for rbx_value in values {
                            match rbx_value.as_ref() {
                                RbxValue::String { value } => {
                                    chunk.write_string(&value)?;
                                }
                                RbxValue::Content { value } => {
                                    chunk.write_string(&value)?;
                                }
                                RbxValue::BinaryString { value } => {
                                    chunk.write_binary_string(&value)?;
                                }
                                _ => panic!("type mismatch"),
                            }
                        }
                    }
                    Type::Bool => {
                        for rbx_value in values {
                            match rbx_value.as_ref() {
                                RbxValue::Bool { value } => {
                                    chunk.write_bool(*value)?;
                                }
                                _ => panic!("type mismatch"),
                            }
                        }
                    }
                    _ => {
                        // This should be unreachable because we assert that we
                        // have a known binary format type ID above. We might
                        // hit this panic if we forget to add a case for any
                        // newly supported types here.

                        unreachable!();
                    }
                }

                chunk.dump(&mut self.output)?;
            }
        }

        Ok(())
    }

    /// Write out the hierarchical relations between instances, stored in a
    /// chunk named PRNT.
    fn serialize_parents(&mut self) -> io::Result<()> {
        log::trace!("Writing parent relationships");

        let mut chunk = ChunkBuilder::new(b"PRNT", Compression::Compressed);

        chunk.write_u8(0)?; // PRNT version 0
        chunk.write_u32::<LittleEndian>(self.relevant_instances.len() as u32)?;

        let object_referents = self
            .relevant_instances
            .iter()
            .map(|id| self.id_to_referent[id]);

        let parent_referents = self.relevant_instances.iter().map(|id| {
            let instance = self.tree.get_instance(*id).unwrap();

            // If there's no parent set OR our parent is not one of the
            // instances we're serializing, we use -1 to represent a null
            // parent.
            instance
                .get_parent_id()
                .and_then(|parent_id| self.id_to_referent.get(&parent_id).cloned())
                .unwrap_or(-1)
        });

        chunk.write_referents(object_referents)?;
        chunk.write_referents(parent_referents)?;

        chunk.dump(&mut self.output)?;

        Ok(())
    }

    /// Write the fixed, uncompressed end chunk used to verify that the file
    /// hasn't been truncated mistakenly. This chunk is named END\0, with a zero
    /// byte at the end.
    fn serialize_end(&mut self) -> io::Result<()> {
        log::trace!("Writing file end");

        let mut end = ChunkBuilder::new(b"END\0", Compression::Uncompressed);
        end.write_all(FILE_FOOTER)?;
        end.dump(&mut self.output)?;

        Ok(())
    }
}
