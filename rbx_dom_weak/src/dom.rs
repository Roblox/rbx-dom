use std::collections::{HashMap, VecDeque};

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
    pub fn new(builder: InstanceBuilder) -> WeakDom {
        let root_ref = builder.referent;

        let mut instances = HashMap::new();
        instances.insert(
            root_ref,
            Instance {
                referent: root_ref,
                children: Vec::new(),
                parent: None,
                name: builder.name,
                class: builder.class,
                properties: builder.properties,
            },
        );

        let mut dom = WeakDom {
            root_ref,
            instances,
        };

        for child in builder.children {
            dom.insert(root_ref, child);
        }

        dom
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

    pub fn insert(&mut self, parent_ref: Ref, builder: InstanceBuilder) {
        let referent = builder.referent;

        self.instances.insert(
            referent,
            Instance {
                referent,
                children: Vec::new(),
                parent: Some(parent_ref),
                name: builder.name,
                class: builder.class,
                properties: builder.properties,
            },
        );

        let parent = self
            .instances
            .get_mut(&parent_ref)
            .unwrap_or_else(|| panic!("cannot insert into parent that does not exist"));

        parent.children.push(referent);

        for child in builder.children {
            self.insert(referent, child);
        }
    }

    pub fn destroy(&mut self, referent: Ref) {
        if let Some(parent) = self.instances[&referent].parent {
            let parent = self.instances.get_mut(&parent).unwrap();
            parent.children.retain(|&child| child != referent);
        }

        let mut to_remove = VecDeque::new();
        to_remove.push_back(referent);

        while let Some(referent) = to_remove.pop_front() {
            let instance = self.instances.remove(&referent).unwrap();
            to_remove.extend(instance.children);
        }
    }
}
