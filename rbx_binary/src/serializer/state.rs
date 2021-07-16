use std::{
    borrow::{Borrow, Cow},
    collections::{BTreeMap, BTreeSet, HashMap, VecDeque},
    convert::TryInto,
    io::Write,
    u32,
};

use rbx_dom_weak::{
    types::{
        Axes, BinaryString, BrickColor, CFrame, Color3, Color3uint8, ColorSequence,
        ColorSequenceKeypoint, Enum, Faces, Matrix3, NumberRange, NumberSequence,
        NumberSequenceKeypoint, PhysicalProperties, Ray, Rect, Ref, SharedString, UDim, UDim2,
        Variant, VariantType, Vector2, Vector3, Vector3int16,
    },
    WeakDom,
};

use rbx_reflection::{ClassDescriptor, ClassTag, DataType};

use crate::{
    cframe,
    chunk::{ChunkBuilder, ChunkCompression},
    core::{
        find_property_descriptors, RbxWriteExt, FILE_MAGIC_HEADER, FILE_SIGNATURE, FILE_VERSION,
    },
    types::Type,
};

use super::error::InnerError;

static FILE_FOOTER: &[u8] = b"</roblox>";

/// Represents all of the state during a single serialization session. A new
/// `BinarySerializer` object should be created every time we want to serialize
/// a binary model file.
pub(super) struct SerializerState<'a, W> {
    /// The dom containing all of the instances that we're serializing.
    dom: &'a WeakDom,

    /// Where the binary output should be written.
    output: W,

    /// All of the instances, in a deterministic order, that we're going to be
    /// serializing.
    relevant_instances: Vec<Ref>,

    /// A map from rbx-dom's unique instance ID (Ref) to the ID space used in
    /// the binary model format, signed integers.
    id_to_referent: HashMap<Ref, i32>,

    /// All of the types of instance discovered by our serializer that we'll be
    /// writing into the output.
    type_infos: TypeInfos,

    /// All of the SharedStrings in the DOM, in the order they'll be written
    // in.
    shared_strings: Vec<SharedString>,

    /// A map of SharedStrings to where it is in the SSTR chunk. This is used
    /// for writing PROP chunks.
    shared_string_ids: HashMap<SharedString, u32>,
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
    object_refs: Vec<Ref>,

    /// All of the defined properties for this type found on any instance of
    /// this type. Properties are keyed by their canonical name, and only one
    /// entry should be present for each logical property.
    ///
    /// Stored in a sorted map to try to ensure that we write out properties in
    /// a deterministic order.
    properties: BTreeMap<Cow<'static, str>, PropInfo>,

    /// A reference to the type's class descriptor from rbx_reflection, if this
    /// is a known class.
    class_descriptor: Option<&'static ClassDescriptor<'static>>,
}

/// A property on a specific class that our serializer knows about.
///
/// We should have one `PropInfo` per logical property per class that is used in
/// the document we are serializing. This means that even if `BasePart.Size` and
/// `BasePart.size` are present in the same document, they should share a
/// `PropInfo` as they are the same logical property.
#[derive(Debug)]
struct PropInfo {
    /// The binary format type ID that will be use to serialize this property.
    /// This type is related to the type of the serialized form of the logical
    /// property, but is not 1:1.
    ///
    /// For example, a property marked to serialize as a
    /// `VariantType::BinaryString` will serialize as `Type::String`, the same
    /// as the `Content` and `String` variants do.
    prop_type: Type,

    /// The serialized name for this property. This is the name that is actually
    /// written as part of the PROP chunk and may not line up with the canonical
    /// name for the property.
    serialized_name: Cow<'static, str>,

    /// A set containing the names of all aliases discovered while preparing to
    /// serialize this property. Ideally, this set will remain empty (and not
    /// allocate) in most cases. However, if an instance is missing a property
    /// from its canonical name, but does have another variant, we can use this
    /// set to recover and map those values.
    aliases: BTreeSet<String>,

    /// The default value for this property that should be used if any instances
    /// are missing this property.
    ///
    /// With the exception of newly-added properties, Roblox Studio will create
    /// files with instances that contain every property. When mixing old and
    /// newly-saved instances, or mixing instances generated from other tools,
    /// some properties may be missing. They will be populated from this value.
    ///
    /// Default values are first populated from the reflection database, if
    /// present, followed by an educated guess based on the type of the value.
    default_value: Cow<'static, Variant>,
}

/// Contains all of the `TypeInfo` objects known to the serializer so far. This
/// struct was broken out to help encapsulate the behavior here and to ease
/// self-borrowing issues from BinarySerializer getting too large.
#[derive(Debug)]
struct TypeInfos {
    /// A map containing one entry for each unique ClassName discovered in the
    /// DOM.
    ///
    /// These are stored sorted so that we naturally iterate over them in order
    /// and improve our chances of being deterministic.
    values: BTreeMap<String, TypeInfo>,

    /// The next type ID that should be assigned if a type is discovered and
    /// added to the serializer.
    next_type_id: u32,
}

impl TypeInfos {
    fn new() -> Self {
        Self {
            values: BTreeMap::new(),
            next_type_id: 0,
        }
    }

    /// Finds the type info from the given ClassName if it exists, or creates
    /// one and returns a reference to it if not.
    fn get_or_create(&mut self, class: &str) -> &mut TypeInfo {
        if !self.values.contains_key(class) {
            let type_id = self.next_type_id;
            self.next_type_id += 1;

            let class_descriptor = rbx_reflection_database::get().classes.get(class);

            let is_service;
            if let Some(descriptor) = &class_descriptor {
                is_service = descriptor.tags.contains(&ClassTag::Service);
            } else {
                log::info!("The class {} is not known to rbx_binary", class);
                is_service = false;
            };

            let mut properties = BTreeMap::new();

            // Every instance has a property named Name. Even though
            // rbx_dom_weak encodes the name property specially, we still insert
            // this property into the type info and handle it like a regular
            // property during encoding.
            //
            // We can use a dummy default_value here because instances from
            // rbx_dom_weak always have a name set.
            properties.insert(
                Cow::Borrowed("Name"),
                PropInfo {
                    prop_type: Type::String,
                    serialized_name: Cow::Borrowed("Name"),
                    aliases: BTreeSet::new(),
                    default_value: Cow::Owned(Variant::String(String::new())),
                },
            );

            self.values.insert(
                class.to_owned(),
                TypeInfo {
                    type_id,
                    is_service,
                    object_refs: Vec::new(),
                    properties,
                    class_descriptor,
                },
            );
        }

        // This unwrap will not panic because we always insert this key into
        // type_infos in this function.
        self.values.get_mut(class).unwrap()
    }
}

impl<'a, W: Write> SerializerState<'a, W> {
    pub fn new(dom: &'a WeakDom, output: W) -> Self {
        SerializerState {
            dom,
            output,
            relevant_instances: Vec::new(),
            id_to_referent: HashMap::new(),
            type_infos: TypeInfos::new(),
            shared_strings: Vec::new(),
            shared_string_ids: HashMap::new(),
        }
    }

    /// Mark the given instance IDs and all of their descendants as intended for
    /// serialization with this serializer.
    pub fn add_instances(&mut self, referents: &[Ref]) -> Result<(), InnerError> {
        let mut to_visit = VecDeque::new();
        to_visit.extend(referents);

        while let Some(referent) = to_visit.pop_front() {
            self.relevant_instances.push(referent);
            self.collect_type_info(referent)?;

            // TODO: Turn into error
            let instance = self.dom.get_by_ref(referent).unwrap();
            to_visit.extend(instance.children());
        }

        log::debug!("Type info discovered: {:#?}", self.type_infos);

        Ok(())
    }

    /// Collect information about all the different types of instance and their
    /// properties.
    // Using the entry API here, as Clippy suggests, would require us to
    // clone canonical_name in a cold branch. We don't want to do that.
    #[allow(clippy::map_entry)]
    pub fn collect_type_info(&mut self, referent: Ref) -> Result<(), InnerError> {
        let instance = self
            .dom
            .get_by_ref(referent)
            .ok_or(InnerError::InvalidInstanceId { referent })?;

        let type_info = self.type_infos.get_or_create(&instance.class);
        type_info.object_refs.push(referent);

        for (prop_name, prop_value) in &instance.properties {
            let canonical_name;
            let serialized_name;
            let serialized_ty;

            let database = rbx_reflection_database::get();
            match find_property_descriptors(database, &instance.class, prop_name) {
                Some(descriptors) => {
                    // For any properties that do not serialize, we can skip
                    // adding them to the set of type_infos.
                    let serialized = match descriptors.serialized {
                        Some(descriptor) => descriptor,
                        None => continue,
                    };

                    canonical_name = descriptors.canonical.name.clone();
                    serialized_name = serialized.name.clone();

                    serialized_ty = match &serialized.data_type {
                        DataType::Value(ty) => *ty,
                        DataType::Enum(_) => VariantType::Enum,

                        unknown_ty => {
                            // rbx_binary is not new enough to handle this kind
                            // of property, whatever it is.
                            return Err(InnerError::UnsupportedPropType {
                                type_name: instance.class.clone(),
                                prop_name: prop_name.clone(),
                                prop_type: format!("{:?}", unknown_ty),
                            });
                        }
                    };
                }

                None => {
                    canonical_name = Cow::Owned(prop_name.clone());
                    serialized_name = Cow::Owned(prop_name.clone());
                    serialized_ty = prop_value.ty();
                }
            }

            // In order to prevent cloning canonical_name in a rare branch,
            // we conditionally clone here if we'll need canonical_name after
            // it's inserted into type_info.properties.
            let canonical_name_if_different = if prop_name != &canonical_name {
                Some(canonical_name.clone())
            } else {
                None
            };

            if !type_info.properties.contains_key(&canonical_name) {
                let default_value = type_info
                    .class_descriptor
                    .and_then(|class| {
                        class
                            .default_properties
                            .get(&canonical_name)
                            .map(Cow::Borrowed)
                    })
                    .or_else(|| Self::fallback_default_value(serialized_ty).map(Cow::Owned))
                    .ok_or_else(|| {
                        // Since we don't know how to generate the default value
                        // for this property, we consider it unsupported.
                        InnerError::UnsupportedPropType {
                            type_name: instance.class.clone(),
                            prop_name: canonical_name.to_string(),
                            prop_type: format!("{:?}", serialized_ty),
                        }
                    })?;

                let ser_type = Type::from_rbx_type(serialized_ty).ok_or_else(|| {
                    // This is a known value type, but rbx_binary doesn't have a
                    // binary type value for it. rbx_binary might be out of
                    // date?
                    InnerError::UnsupportedPropType {
                        type_name: instance.class.clone(),
                        prop_name: serialized_name.to_string(),
                        prop_type: format!("{:?}", serialized_ty),
                    }
                })?;

                type_info.properties.insert(
                    canonical_name,
                    PropInfo {
                        prop_type: ser_type,
                        serialized_name,
                        aliases: BTreeSet::new(),
                        default_value,
                    },
                );
            }

            // If the property we found on this instance is different than the
            // canonical name for this property, stash it into the set of known
            // aliases for this PropInfo.
            if let Some(canonical_name) = canonical_name_if_different {
                let prop_info = type_info.properties.get_mut(&canonical_name).unwrap();

                if !prop_info.aliases.contains(prop_name) {
                    prop_info.aliases.insert(prop_name.clone());
                }
            }

            if let Variant::SharedString(shared_string) = prop_value {
                if !self.shared_string_ids.contains_key(shared_string) {
                    let id = self.shared_strings.len() as u32;
                    self.shared_string_ids.insert(shared_string.clone(), id);
                    self.shared_strings.push(shared_string.clone())
                }
            }
        }

        Ok(())
    }

    /// Populate the map from rbx-dom's instance ID space to the IDs that we'll
    /// be serializing to the model.
    pub fn generate_referents(&mut self) {
        self.id_to_referent.reserve(self.relevant_instances.len());

        for (next_referent, id) in self.relevant_instances.iter().enumerate() {
            self.id_to_referent
                .insert(*id, next_referent.try_into().unwrap());
        }

        log::trace!("Referents constructed: {:#?}", self.id_to_referent);
    }

    pub fn write_header(&mut self) -> Result<(), InnerError> {
        log::trace!("Writing header");

        self.output.write_all(FILE_MAGIC_HEADER)?;
        self.output.write_all(FILE_SIGNATURE)?;
        self.output.write_le_u16(FILE_VERSION)?;

        self.output
            .write_le_u32(self.type_infos.values.len() as u32)?;
        self.output
            .write_le_u32(self.relevant_instances.len() as u32)?;
        self.output.write_all(&[0; 8])?;

        Ok(())
    }

    /// Write out any metadata about this file, stored in a chunk named META.
    pub fn serialize_metadata(&mut self) -> Result<(), InnerError> {
        log::trace!("Writing metadata (currently no-op)");
        // TODO: There is no concept of metadata in a dom yet.
        Ok(())
    }

    /// Write out all of the SharedStrings in this file, if any exist,
    /// stored in a chunk named SSTR.
    pub fn serialize_shared_strings(&mut self) -> Result<(), InnerError> {
        log::trace!("Writing shared string chunk");

        if self.shared_strings.is_empty() {
            return Ok(());
        }

        let mut chunk = ChunkBuilder::new(b"SSTR", ChunkCompression::Compressed);

        chunk.write_le_u32(0)?; // SSTR version number
        chunk.write_le_u32(self.shared_strings.len() as u32)?;

        for shared_string in &self.shared_strings {
            // Better to write nothing than write half a hash
            chunk.write_all(&[0; 16])?;
            chunk.write_binary_string(shared_string.data())?;
        }

        chunk.dump(&mut self.output)?;

        Ok(())
    }

    /// Write out the declarations of all instances, stored in a series of
    /// chunks named INST.
    pub fn serialize_instances(&mut self) -> Result<(), InnerError> {
        log::trace!("Writing instance chunks");

        for (type_name, type_info) in &self.type_infos.values {
            log::trace!(
                "Writing chunk for {} ({} instances)",
                type_name,
                type_info.object_refs.len()
            );

            let mut chunk = ChunkBuilder::new(b"INST", ChunkCompression::Compressed);

            chunk.write_le_u32(type_info.type_id)?;
            chunk.write_string(type_name)?;

            // It's possible that this integer will be expanded in the future to
            // be a general version/format field instead of just service vs
            // non-service.
            //
            // At that point, we'll start thinking about it like it's a u8
            // instead of a bool.
            chunk.write_bool(type_info.is_service)?;

            chunk.write_le_u32(type_info.object_refs.len() as u32)?;

            chunk.write_referent_array(
                type_info
                    .object_refs
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
                for _ in 0..type_info.object_refs.len() {
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
    pub fn serialize_properties(&mut self) -> Result<(), InnerError> {
        log::trace!("Writing properties");

        for (type_name, type_info) in &self.type_infos.values {
            for (prop_name, prop_info) in &type_info.properties {
                log::trace!(
                    "Writing property {}.{} (type {:?})",
                    type_name,
                    prop_name,
                    prop_info.prop_type
                );

                let mut chunk = ChunkBuilder::new(b"PROP", ChunkCompression::Compressed);

                chunk.write_le_u32(type_info.type_id)?;
                chunk.write_string(&prop_info.serialized_name)?;
                chunk.write_u8(prop_info.prop_type as u8)?;

                let dom = &self.dom;
                let values = type_info
                    .object_refs
                    .iter()
                    .map(|id| {
                        // This unwrap will not panic because we uphold the
                        // invariant that any ID in object_refs must be part of
                        // this dom.
                        let instance = dom.get_by_ref(*id).unwrap();

                        // We store the Name property in a different field for
                        // convenience, but when serializing to the binary model
                        // format we need to handle it just like other properties.
                        if prop_name == "Name" {
                            return Cow::Owned(Variant::String(instance.name.clone()));
                        }

                        // Most properties will be stored on instances using the
                        // property's canonical name, so we'll try that first.
                        if let Some(property) = instance.properties.get(prop_name.as_ref()) {
                            return Cow::Borrowed(property);
                        }

                        // If there were any known aliases for this property
                        // used as part of this file, we can check those next.
                        for alias in &prop_info.aliases {
                            if let Some(property) = instance.properties.get(alias) {
                                return Cow::Borrowed(property);
                            }
                        }

                        // Finally, we can fall back to the default value we
                        // computed for this PropInfo. This is sourced from the
                        // reflection database if available, or falls back to a
                        // reasonable default.
                        Cow::Borrowed(prop_info.default_value.borrow())
                    })
                    .enumerate();

                // Helper to generate a type mismatch error with context from
                // this chunk.
                let type_mismatch =
                    |i: usize, bad_value: &Variant, valid_type_names: &'static str| {
                        Err(InnerError::PropTypeMismatch {
                            type_name: type_name.clone(),
                            prop_name: prop_name.to_string(),
                            valid_type_names,
                            actual_type_name: format!("{:?}", bad_value.ty()),
                            instance_full_name: self.full_name_for(type_info.object_refs[i]),
                        })
                    };

                match prop_info.prop_type {
                    Type::String => {
                        for (i, rbx_value) in values {
                            match rbx_value.as_ref() {
                                Variant::String(value) => {
                                    chunk.write_string(&value)?;
                                }
                                Variant::Content(value) => {
                                    chunk.write_string(value.as_ref())?;
                                }
                                Variant::BinaryString(value) => {
                                    chunk.write_binary_string(value.as_ref())?;
                                }
                                Variant::Tags(value) => {
                                    let buf: Vec<u8> = value.into();
                                    chunk.write_binary_string(buf.as_ref())?;
                                }
                                _ => {
                                    return type_mismatch(
                                        i,
                                        &rbx_value,
                                        "String, Content, Tags, or BinaryString",
                                    );
                                }
                            }
                        }
                    }
                    Type::Bool => {
                        for (i, rbx_value) in values {
                            if let Variant::Bool(value) = rbx_value.as_ref() {
                                chunk.write_bool(*value)?;
                            } else {
                                return type_mismatch(i, &rbx_value, "Bool");
                            }
                        }
                    }
                    Type::Int32 => {
                        let mut buf = Vec::with_capacity(values.len());

                        for (i, rbx_value) in values {
                            if let Variant::Int32(value) = rbx_value.as_ref() {
                                buf.push(*value);
                            } else {
                                return type_mismatch(i, &rbx_value, "Int32");
                            }
                        }

                        chunk.write_interleaved_i32_array(buf.into_iter())?;
                    }
                    Type::Float32 => {
                        let mut buf = Vec::with_capacity(values.len());

                        for (i, rbx_value) in values {
                            if let Variant::Float32(value) = rbx_value.as_ref() {
                                buf.push(*value);
                            } else {
                                return type_mismatch(i, &rbx_value, "Float32");
                            }
                        }

                        chunk.write_interleaved_f32_array(buf.into_iter())?;
                    }
                    Type::Float64 => {
                        for (i, rbx_value) in values {
                            match rbx_value.as_ref() {
                                Variant::Float64(value) => {
                                    chunk.write_le_f64(*value)?;
                                }
                                Variant::Float32(value) => {
                                    chunk.write_le_f64(*value as f64)?;
                                }
                                _ => return type_mismatch(i, &rbx_value, "Float64"),
                            }
                        }
                    }
                    Type::UDim => {
                        let mut scale = Vec::with_capacity(values.len());
                        let mut offset = Vec::with_capacity(values.len());

                        for (i, rbx_value) in values {
                            if let Variant::UDim(value) = rbx_value.as_ref() {
                                scale.push(value.scale);
                                offset.push(value.offset);
                            } else {
                                return type_mismatch(i, &rbx_value, "UDim");
                            }
                        }

                        chunk.write_interleaved_f32_array(scale.into_iter())?;
                        chunk.write_interleaved_i32_array(offset.into_iter())?;
                    }
                    Type::UDim2 => {
                        let mut scale_x = Vec::with_capacity(values.len());
                        let mut scale_y = Vec::with_capacity(values.len());
                        let mut offset_x = Vec::with_capacity(values.len());
                        let mut offset_y = Vec::with_capacity(values.len());

                        for (i, rbx_value) in values {
                            if let Variant::UDim2(value) = rbx_value.as_ref() {
                                scale_x.push(value.x.scale);
                                scale_y.push(value.y.scale);
                                offset_x.push(value.x.offset);
                                offset_y.push(value.y.offset);
                            } else {
                                return type_mismatch(i, &rbx_value, "UDim2");
                            }
                        }

                        chunk.write_interleaved_f32_array(scale_x.into_iter())?;
                        chunk.write_interleaved_f32_array(scale_y.into_iter())?;
                        chunk.write_interleaved_i32_array(offset_x.into_iter())?;
                        chunk.write_interleaved_i32_array(offset_y.into_iter())?;
                    }
                    Type::Ray => {
                        for (i, rbx_value) in values {
                            if let Variant::Ray(value) = rbx_value.as_ref() {
                                chunk.write_le_f32(value.origin.x)?;
                                chunk.write_le_f32(value.origin.y)?;
                                chunk.write_le_f32(value.origin.z)?;
                                chunk.write_le_f32(value.direction.x)?;
                                chunk.write_le_f32(value.direction.y)?;
                                chunk.write_le_f32(value.direction.x)?;
                            } else {
                                return type_mismatch(i, &rbx_value, "Ray");
                            }
                        }
                    }
                    Type::Faces => {
                        for (i, rbx_value) in values {
                            if let Variant::Faces(value) = rbx_value.as_ref() {
                                chunk.write_u8(value.bits())?;
                            } else {
                                return type_mismatch(i, &rbx_value, "Faces");
                            }
                        }
                    }
                    Type::Axes => {
                        for (i, rbx_value) in values {
                            if let Variant::Axes(value) = rbx_value.as_ref() {
                                chunk.write_u8(value.bits())?;
                            } else {
                                return type_mismatch(i, &rbx_value, "Axes");
                            }
                        }
                    }
                    Type::BrickColor => {
                        let mut numbers = Vec::with_capacity(values.len());

                        for (i, rbx_value) in values {
                            if let Variant::BrickColor(value) = rbx_value.as_ref() {
                                numbers.push(*value as u32);
                            } else if let Variant::Int32(value) = rbx_value.as_ref() {
                                numbers.push(*value as u32);
                            } else {
                                return type_mismatch(i, &rbx_value, "BrickColor");
                            }
                        }

                        chunk.write_interleaved_u32_array(&numbers)?;
                    }
                    Type::Color3 => {
                        let mut r = Vec::with_capacity(values.len());
                        let mut g = Vec::with_capacity(values.len());
                        let mut b = Vec::with_capacity(values.len());

                        for (i, rbx_value) in values {
                            if let Variant::Color3(value) = rbx_value.as_ref() {
                                r.push(value.r);
                                g.push(value.g);
                                b.push(value.b);
                            } else {
                                return type_mismatch(i, &rbx_value, "Color3");
                            }
                        }

                        chunk.write_interleaved_f32_array(r.into_iter())?;
                        chunk.write_interleaved_f32_array(g.into_iter())?;
                        chunk.write_interleaved_f32_array(b.into_iter())?;
                    }
                    Type::Vector2 => {
                        let mut x = Vec::with_capacity(values.len());
                        let mut y = Vec::with_capacity(values.len());

                        for (i, rbx_value) in values {
                            if let Variant::Vector2(value) = rbx_value.as_ref() {
                                x.push(value.x);
                                y.push(value.y)
                            } else {
                                return type_mismatch(i, &rbx_value, "Vector2");
                            }
                        }

                        chunk.write_interleaved_f32_array(x.into_iter())?;
                        chunk.write_interleaved_f32_array(y.into_iter())?;
                    }
                    Type::Vector3 => {
                        let mut x = Vec::with_capacity(values.len());
                        let mut y = Vec::with_capacity(values.len());
                        let mut z = Vec::with_capacity(values.len());

                        for (i, rbx_value) in values {
                            if let Variant::Vector3(value) = rbx_value.as_ref() {
                                x.push(value.x);
                                y.push(value.y);
                                z.push(value.z)
                            } else {
                                return type_mismatch(i, &rbx_value, "Vector3");
                            }
                        }

                        chunk.write_interleaved_f32_array(x.into_iter())?;
                        chunk.write_interleaved_f32_array(y.into_iter())?;
                        chunk.write_interleaved_f32_array(z.into_iter())?;
                    }
                    Type::CFrame => {
                        let mut rotations = Vec::with_capacity(values.len());
                        let mut x = Vec::with_capacity(values.len());
                        let mut y = Vec::with_capacity(values.len());
                        let mut z = Vec::with_capacity(values.len());

                        for (i, rbx_value) in values {
                            if let Variant::CFrame(value) = rbx_value.as_ref() {
                                rotations.push(value.orientation);
                                x.push(value.position.x);
                                y.push(value.position.y);
                                z.push(value.position.z);
                            } else {
                                return type_mismatch(i, &rbx_value, "CFrame");
                            }
                        }

                        for matrix in rotations {
                            if let Some(id) = cframe::to_basic_rotation_id(matrix) {
                                chunk.write_u8(id)?;
                            } else {
                                chunk.write_u8(0x00)?;

                                chunk.write_le_f32(matrix.x.x)?;
                                chunk.write_le_f32(matrix.x.y)?;
                                chunk.write_le_f32(matrix.x.z)?;

                                chunk.write_le_f32(matrix.y.x)?;
                                chunk.write_le_f32(matrix.y.y)?;
                                chunk.write_le_f32(matrix.y.z)?;

                                chunk.write_le_f32(matrix.z.x)?;
                                chunk.write_le_f32(matrix.z.y)?;
                                chunk.write_le_f32(matrix.z.z)?;
                            }
                        }

                        chunk.write_interleaved_f32_array(x.into_iter())?;
                        chunk.write_interleaved_f32_array(y.into_iter())?;
                        chunk.write_interleaved_f32_array(z.into_iter())?;
                    }
                    Type::Enum => {
                        let mut buf = Vec::with_capacity(values.len());

                        for (i, rbx_value) in values {
                            if let Variant::Enum(value) = rbx_value.as_ref() {
                                buf.push(value.to_u32());
                            } else {
                                return type_mismatch(i, &rbx_value, "Enum");
                            }
                        }

                        chunk.write_interleaved_u32_array(&buf)?;
                    }
                    Type::Ref => {
                        let mut buf = Vec::with_capacity(values.len());

                        for (i, rbx_value) in values {
                            if let Variant::Ref(value) = rbx_value.as_ref() {
                                if let Some(id) = self.id_to_referent.get(value) {
                                    buf.push(*id);
                                } else {
                                    buf.push(-1);
                                }
                            } else {
                                return type_mismatch(i, &rbx_value, "Ref");
                            }
                        }

                        chunk.write_referent_array(buf.into_iter())?;
                    }
                    Type::Vector3int16 => {
                        for (i, rbx_value) in values {
                            if let Variant::Vector3int16(value) = rbx_value.as_ref() {
                                chunk.write_le_i16(value.x)?;
                                chunk.write_le_i16(value.y)?;
                                chunk.write_le_i16(value.z)?;
                            } else {
                                return type_mismatch(i, &rbx_value, "Vector3int16");
                            }
                        }
                    }
                    Type::NumberSequence => {
                        for (i, rbx_value) in values {
                            if let Variant::NumberSequence(value) = rbx_value.as_ref() {
                                chunk.write_le_u32(value.keypoints.len() as u32)?;

                                for keypoint in &value.keypoints {
                                    chunk.write_le_f32(keypoint.time)?;
                                    chunk.write_le_f32(keypoint.value)?;
                                    chunk.write_le_f32(keypoint.envelope)?;
                                }
                            } else {
                                return type_mismatch(i, &rbx_value, "NumberSequence");
                            }
                        }
                    }
                    Type::ColorSequence => {
                        for (i, rbx_value) in values {
                            if let Variant::ColorSequence(value) = rbx_value.as_ref() {
                                chunk.write_le_u32(value.keypoints.len() as u32)?;

                                for keypoint in &value.keypoints {
                                    chunk.write_le_f32(keypoint.time)?;
                                    chunk.write_le_f32(keypoint.color.r)?;
                                    chunk.write_le_f32(keypoint.color.g)?;
                                    chunk.write_le_f32(keypoint.color.b)?;

                                    // write out a dummy value for envelope, which is serialized but doesn't do anything
                                    chunk.write_le_f32(0.0)?;
                                }
                            } else {
                                return type_mismatch(i, &rbx_value, "ColorSequence");
                            }
                        }
                    }
                    Type::NumberRange => {
                        for (i, rbx_value) in values {
                            if let Variant::NumberRange(value) = rbx_value.as_ref() {
                                chunk.write_le_f32(value.min)?;
                                chunk.write_le_f32(value.max)?;
                            } else {
                                return type_mismatch(i, &rbx_value, "NumberRange");
                            }
                        }
                    }
                    Type::Rect => {
                        let mut x_min = Vec::with_capacity(values.len());
                        let mut y_min = Vec::with_capacity(values.len());
                        let mut x_max = Vec::with_capacity(values.len());
                        let mut y_max = Vec::with_capacity(values.len());

                        for (i, rbx_value) in values {
                            if let Variant::Rect(value) = rbx_value.as_ref() {
                                x_min.push(value.min.x);
                                y_min.push(value.min.y);
                                x_max.push(value.max.x);
                                y_max.push(value.max.y);
                            } else {
                                return type_mismatch(i, &rbx_value, "Rect");
                            }
                        }

                        chunk.write_interleaved_f32_array(x_min.into_iter())?;
                        chunk.write_interleaved_f32_array(y_min.into_iter())?;
                        chunk.write_interleaved_f32_array(x_max.into_iter())?;
                        chunk.write_interleaved_f32_array(y_max.into_iter())?;
                    }
                    Type::PhysicalProperties => {
                        for (i, rbx_value) in values {
                            if let Variant::PhysicalProperties(value) = rbx_value.as_ref() {
                                if let PhysicalProperties::Custom(props) = value {
                                    chunk.write_u8(1)?;
                                    chunk.write_le_f32(props.density)?;
                                    chunk.write_le_f32(props.friction)?;
                                    chunk.write_le_f32(props.elasticity)?;
                                    chunk.write_le_f32(props.friction_weight)?;
                                    chunk.write_le_f32(props.elasticity_weight)?;
                                } else {
                                    chunk.write_u8(0)?;
                                }
                            } else {
                                return type_mismatch(i, &rbx_value, "PhysicalProperties");
                            }
                        }
                    }
                    Type::Color3uint8 => {
                        let mut r = Vec::with_capacity(values.len());
                        let mut g = Vec::with_capacity(values.len());
                        let mut b = Vec::with_capacity(values.len());

                        for (i, rbx_value) in values {
                            match rbx_value.as_ref() {
                                Variant::Color3uint8(value) => {
                                    r.push(value.r);
                                    g.push(value.g);
                                    b.push(value.b);
                                }
                                Variant::Color3(value) => {
                                    let color: Color3uint8 = (*value).into();

                                    r.push(color.r);
                                    g.push(color.g);
                                    b.push(color.b);
                                }
                                _ => return type_mismatch(i, &rbx_value, "Color3uint8 or Color3"),
                            }
                        }

                        chunk.write_all(r.as_slice())?;
                        chunk.write_all(g.as_slice())?;
                        chunk.write_all(b.as_slice())?;
                    }
                    Type::Int64 => {
                        let mut buf = Vec::with_capacity(values.len());

                        for (i, rbx_value) in values {
                            match rbx_value.as_ref() {
                                Variant::Int64(value) => {
                                    buf.push(*value);
                                }
                                Variant::Int32(value) => {
                                    buf.push(*value as i64);
                                }
                                _ => return type_mismatch(i, &rbx_value, "Int64"),
                            }
                        }

                        chunk.write_interleaved_i64_array(buf.into_iter())?;
                    }
                    Type::SharedString => {
                        let mut entries = Vec::with_capacity(values.len());

                        for (i, rbx_value) in values {
                            if let Variant::SharedString(value) = rbx_value.as_ref() {
                                let id = &self.shared_string_ids[value];
                                entries.push(*id);
                            } else {
                                return type_mismatch(i, &rbx_value, "SharedString");
                            }
                        }

                        chunk.write_interleaved_u32_array(&entries)?;
                    }
                    Type::OptionalCFrame => {
                        let mut rotations = Vec::with_capacity(values.len());
                        let mut bools = Vec::with_capacity(values.len());
                        let mut x = Vec::with_capacity(values.len());
                        let mut y = Vec::with_capacity(values.len());
                        let mut z = Vec::with_capacity(values.len());

                        chunk.write_u8(Type::CFrame as u8)?;

                        for (i, rbx_value) in values {
                            if let Variant::OptionalCFrame(value) = rbx_value.as_ref() {
                                if let Some(value) = value {
                                    rotations.push(value.orientation);
                                    x.push(value.position.x);
                                    y.push(value.position.y);
                                    z.push(value.position.z);
                                    bools.push(0x01);
                                } else {
                                    rotations.push(Matrix3::identity());
                                    x.push(0.0);
                                    y.push(0.0);
                                    z.push(0.0);
                                    bools.push(0x00);
                                }
                            } else {
                                return type_mismatch(i, &rbx_value, "OptionalCFrame");
                            }
                        }

                        for matrix in rotations {
                            if let Some(id) = cframe::to_basic_rotation_id(matrix) {
                                chunk.write_u8(id)?;
                            } else {
                                chunk.write_u8(0x00)?;

                                chunk.write_le_f32(matrix.x.x)?;
                                chunk.write_le_f32(matrix.x.y)?;
                                chunk.write_le_f32(matrix.x.z)?;

                                chunk.write_le_f32(matrix.y.x)?;
                                chunk.write_le_f32(matrix.y.y)?;
                                chunk.write_le_f32(matrix.y.z)?;

                                chunk.write_le_f32(matrix.z.x)?;
                                chunk.write_le_f32(matrix.z.y)?;
                                chunk.write_le_f32(matrix.z.z)?;
                            }
                        }

                        chunk.write_interleaved_f32_array(x.into_iter())?;
                        chunk.write_interleaved_f32_array(y.into_iter())?;
                        chunk.write_interleaved_f32_array(z.into_iter())?;

                        chunk.write_u8(Type::Bool as u8)?;
                        chunk.write_all(bools.as_slice())?;
                    }
                }

                chunk.dump(&mut self.output)?;
            }
        }

        Ok(())
    }

    /// Write out the hierarchical relations between instances, stored in a
    /// chunk named PRNT.
    pub fn serialize_parents(&mut self) -> Result<(), InnerError> {
        log::trace!("Writing parent relationships");

        let mut chunk = ChunkBuilder::new(b"PRNT", ChunkCompression::Compressed);

        chunk.write_u8(0)?; // PRNT version 0
        chunk.write_le_u32(self.relevant_instances.len() as u32)?;

        let object_referents = self
            .relevant_instances
            .iter()
            .map(|id| self.id_to_referent[id]);

        let parent_referents = self.relevant_instances.iter().map(|id| {
            let instance = self.dom.get_by_ref(*id).unwrap();

            // If there's no parent set OR our parent is not one of the
            // instances we're serializing, we use -1 to represent a null
            // parent.
            if instance.parent().is_some() {
                self.id_to_referent
                    .get(&instance.parent())
                    .cloned()
                    .unwrap_or(-1)
            } else {
                -1
            }
        });

        chunk.write_referent_array(object_referents)?;
        chunk.write_referent_array(parent_referents)?;

        chunk.dump(&mut self.output)?;

        Ok(())
    }

    /// Write the fixed, uncompressed end chunk used to verify that the file
    /// hasn't been truncated mistakenly. This chunk is named END\0, with a zero
    /// byte at the end.
    pub fn serialize_end(&mut self) -> Result<(), InnerError> {
        log::trace!("Writing file end");

        let mut end = ChunkBuilder::new(b"END\0", ChunkCompression::Uncompressed);
        end.write_all(FILE_FOOTER)?;
        end.dump(&mut self.output)?;

        Ok(())
    }

    /// Equivalent to Instance:GetFullName() from Roblox.
    fn full_name_for(&self, subject_ref: Ref) -> String {
        let mut components = Vec::new();
        let mut current_id = subject_ref;

        while current_id.is_some() {
            let instance = self.dom.get_by_ref(current_id).unwrap();
            components.push(instance.name.as_str());
            current_id = instance.parent();
        }

        let mut name = String::new();
        for component in components.iter().rev() {
            name.push_str(component);
            name.push('.');
        }
        name.pop();

        name
    }

    fn fallback_default_value(rbx_type: VariantType) -> Option<Variant> {
        Some(match rbx_type {
            VariantType::String => Variant::String(String::new()),
            VariantType::BinaryString => Variant::BinaryString(BinaryString::new()),
            VariantType::Bool => Variant::Bool(false),
            VariantType::Int32 => Variant::Int32(0),
            VariantType::Float32 => Variant::Float32(0.0),
            VariantType::Float64 => Variant::Float64(0.0),
            VariantType::UDim => Variant::UDim(UDim::new(0.0, 0)),
            VariantType::UDim2 => Variant::UDim2(UDim2::new(UDim::new(0.0, 0), UDim::new(0.0, 0))),
            VariantType::Ray => Variant::Ray(Ray::new(
                Vector3::new(0.0, 0.0, 0.0),
                Vector3::new(0.0, 0.0, 0.0),
            )),
            VariantType::Faces => Variant::Faces(Faces::from_bits(0)?),
            VariantType::Axes => Variant::Axes(Axes::from_bits(0)?),
            VariantType::BrickColor => Variant::BrickColor(BrickColor::MediumStoneGrey),
            VariantType::CFrame => Variant::CFrame(CFrame::new(
                Vector3::new(0.0, 0.0, 0.0),
                Matrix3::identity(),
            )),
            VariantType::Enum => Variant::Enum(Enum::from_u32(u32::MAX)),
            VariantType::Color3 => Variant::Color3(Color3::new(0.0, 0.0, 0.0)),
            VariantType::Vector2 => Variant::Vector2(Vector2::new(0.0, 0.0)),
            VariantType::Vector3 => Variant::Vector3(Vector3::new(0.0, 0.0, 0.0)),
            VariantType::Ref => Variant::Ref(Ref::none()),
            VariantType::Vector3int16 => Variant::Vector3int16(Vector3int16::new(0, 0, 0)),
            VariantType::NumberSequence => Variant::NumberSequence(NumberSequence {
                keypoints: [
                    NumberSequenceKeypoint::new(0.0, 0.0, 0.0),
                    NumberSequenceKeypoint::new(0.0, 0.0, 0.0),
                ]
                .to_vec(),
            }),
            VariantType::ColorSequence => Variant::ColorSequence(ColorSequence {
                keypoints: [
                    ColorSequenceKeypoint::new(0.0, Color3::new(0.0, 0.0, 0.0)),
                    ColorSequenceKeypoint::new(0.0, Color3::new(0.0, 0.0, 0.0)),
                ]
                .to_vec(),
            }),
            VariantType::NumberRange => Variant::NumberRange(NumberRange::new(0.0, 0.0)),
            VariantType::Rect => {
                Variant::Rect(Rect::new(Vector2::new(0.0, 0.0), Vector2::new(0.0, 0.0)))
            }
            VariantType::PhysicalProperties => {
                Variant::PhysicalProperties(PhysicalProperties::Default)
            }
            VariantType::Color3uint8 => Variant::Color3uint8(Color3uint8::new(0, 0, 0)),
            VariantType::Int64 => Variant::Int64(0),
            VariantType::SharedString => Variant::SharedString(SharedString::new(Vec::new())),
            VariantType::OptionalCFrame => Variant::OptionalCFrame(None),
            _ => return None,
        })
    }
}
