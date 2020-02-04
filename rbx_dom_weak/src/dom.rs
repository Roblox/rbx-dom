use std::collections::HashMap;

use rbx_types::Ref;

use crate::instance::{Instance, InstanceBuilder};

/// Represents a tree containing Roblox instances.
///
/// Instances are described by [RbxInstance](struct.RbxInstance.html) objects
/// and have an ID, children, and a parent.
///
/// When constructing instances, you'll want to create
/// [RbxInstanceProperties](struct.RbxInstanceProperties.html) objects and
/// insert them into the tree.
#[derive(Debug)]
pub struct WeakDom {
    instances: HashMap<Ref, Instance>,
    root_ref: Ref,
}

impl WeakDom {
    pub fn new(root: InstanceBuilder) -> WeakDom {
        let root = Instance {
            referent: root.referent,
            children: Vec::new(),
            parent: None,
            name: root.name,
            class: root.class,
            properties: root.properties,
        };

        let root_ref = root.referent;
        let mut instances = HashMap::new();
        instances.insert(root_ref, root);

        WeakDom {
            root_ref,
            instances,
        }
    }

    pub fn root_ref(&self) -> Ref {
        self.root_ref
    }

    pub fn root(&self) -> &Instance {
        self.instances.get(&self.root_ref).unwrap()
    }

    pub fn root_mut(&mut self) -> &mut Instance {
        self.instances.get_mut(&self.root_ref).unwrap()
    }

    pub fn get_by_ref(&self, referent: Ref) -> Option<&Instance> {
        self.instances.get(&referent)
    }

    pub fn get_by_ref_mut(&mut self, referent: Ref) -> Option<&mut Instance> {
        self.instances.get_mut(&referent)
    }

    pub fn insert(&mut self, parent_ref: Ref, instance: InstanceBuilder) {
        unimplemented!()
    }

    pub fn remove(&mut self, referent: Ref) {
        unimplemented!()
    }

    pub fn take(&mut self, referent: Ref) -> InstanceBuilder {
        unimplemented!()
    }
}
