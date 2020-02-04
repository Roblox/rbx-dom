use std::collections::HashMap;

use rbx_types::Ref;
use serde::{Deserialize, Serialize};

use crate::instance::{RbxInstance, RbxInstanceProperties};

/// Represents a tree containing Roblox instances.
///
/// Instances are described by [RbxInstance](struct.RbxInstance.html) objects
/// and have an ID, children, and a parent.
///
/// When constructing instances, you'll want to create
/// [RbxInstanceProperties](struct.RbxInstanceProperties.html) objects and
/// insert them into the tree.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WeakDom {
    instances: HashMap<Ref, RbxInstance>,
    root_ref: Ref,
}

impl WeakDom {
    /// Construct a new `WeakDom` with its root instance constructed using the
    /// given properties.
    pub fn new(root_properties: RbxInstanceProperties) -> WeakDom {
        let rooted_root = RbxInstance::new(root_properties);
        let root_ref = rooted_root.get_id();

        let mut instances = HashMap::new();
        instances.insert(root_ref, rooted_root);

        WeakDom {
            instances,
            root_ref,
        }
    }

    pub fn root_ref(&self) -> Ref {
        self.root_ref
    }

    /// Returns the instance with the given ID if it's contained in this tree.
    pub fn get_instance(&self, id: Ref) -> Option<&RbxInstance> {
        self.instances.get(&id)
    }

    /// Returns mutable access to the instance with the given ID if it's
    /// contained in this tree.
    pub fn get_instance_mut(&mut self, id: Ref) -> Option<&mut RbxInstance> {
        self.instances.get_mut(&id)
    }

    /// Move the instance with the given ID from this tree to a new tree,
    /// underneath the given parent instance ID.
    ///
    /// ## Panics
    /// Panics if the instance `source_id` doesn't exist in the source tree or
    /// if the instance `dest_parent_id` doesn't exist in the destination tree.
    pub fn move_instance(&mut self, source_id: Ref, dest_tree: &mut WeakDom, dest_parent_id: Ref) {
        self.orphan_instance(source_id);

        // Remove the instance we're trying to move and manually rewrite its
        // parent.
        let mut root_instance = self
            .instances
            .remove(&source_id)
            .expect("Cannot move an instance that does not exist in the tree");
        root_instance.parent = Some(dest_parent_id);

        let mut to_visit = root_instance.children.clone();

        dest_tree.insert_internal_and_unorphan(root_instance);

        // We can move children in whatever order since we aren't touching their
        // children tables
        while let Some(id) = to_visit.pop() {
            let instance = self.instances.remove(&id).unwrap();
            to_visit.extend_from_slice(&instance.children);

            dest_tree.instances.insert(instance.get_id(), instance);
        }
    }

    /// Move the instance with the ID `id` so that its new parent is
    /// `dest_parent_id`.
    ///
    /// ## Panics
    /// Panics if `id` or `dest_parent_id` do not refer to instances that exist
    /// in the tree.
    ///
    /// Panics if this operation would cause the tree to become cyclical and
    /// invalid.
    pub fn set_parent(&mut self, id: Ref, dest_parent_id: Ref) {
        for instance in self.descendants(id) {
            if instance.get_id() == dest_parent_id {
                panic!("set_parent cannot create circular references");
            }
        }

        self.orphan_instance(id);
        self.unorphan_instance(id, dest_parent_id);
    }

    /// Inserts a new instance with the given properties into the tree, putting it
    /// under the instance with the given ID.
    ///
    /// ## Panics
    /// Panics if the given ID does not refer to an instance in this tree.
    pub fn insert_instance(&mut self, properties: RbxInstanceProperties, parent_id: Ref) -> Ref {
        let mut tree_instance = RbxInstance::new(properties);
        tree_instance.parent = Some(parent_id);

        let id = tree_instance.get_id();

        self.insert_internal_and_unorphan(tree_instance);

        id
    }

    /// Given an ID, remove the instance from the tree with that ID, along with
    /// all of its descendants.
    pub fn remove_instance(&mut self, root_ref: Ref) -> Option<WeakDom> {
        if self.root_ref == root_ref {
            panic!("Cannot remove root ID from tree!");
        }

        self.orphan_instance(root_ref);

        let mut ids_to_visit = vec![root_ref];
        let mut new_tree_instances = HashMap::new();

        while let Some(id) = ids_to_visit.pop() {
            match self.instances.get(&id) {
                Some(instance) => ids_to_visit.extend_from_slice(&instance.children),
                None => continue,
            }

            let instance = self.instances.remove(&id).unwrap();
            new_tree_instances.insert(id, instance);
        }

        Some(WeakDom {
            instances: new_tree_instances,
            root_ref,
        })
    }

    /// Returns an iterator over all of the descendants of the given instance by
    /// ID.
    ///
    /// ## Panics
    /// Panics if the given ID is not present in the tree.
    pub fn descendants(&self, id: Ref) -> Descendants<'_> {
        let instance = self
            .get_instance(id)
            .expect("Cannot enumerate descendants of an instance not in the tree");

        Descendants {
            tree: self,
            ids_to_visit: instance.get_children_ids().to_vec(),
        }
    }

    /// Unlinks the parent->child link for the given ID, effectively making it
    /// an orphan in the tree.
    ///
    /// The instance will still refer to its parent by ID, so any method calling
    /// orphan_instance will need to make additional changes to preserve
    /// WeakDom's invariants.
    ///
    /// # Panics
    /// Panics if the given instance does not exist, does not have a parent, or
    /// if any WeakDom variants were violated.
    fn orphan_instance(&mut self, orphan_id: Ref) {
        let parent_id = self
            .instances
            .get(&orphan_id)
            .expect("Cannot orphan an instance that does not exist in the tree")
            .get_parent_id()
            .expect("Cannot orphan an instance without a parent, like the root instance");

        let parent = self
            .get_instance_mut(parent_id)
            .expect("Instance referred to an ID that does not exist");

        parent.children.retain(|&id| id != orphan_id);
    }

    /// Inserts a fully-constructed instance into this tree's instance table and
    /// links it to the parent given by its parent ID field.
    ///
    /// # Panics
    /// Panics if the instance has a None parent or if the parent it refers to
    /// does not exist in this tree.
    fn insert_internal_and_unorphan(&mut self, instance: RbxInstance) {
        let id = instance.get_id();
        let parent_id = instance
            .parent
            .expect("Cannot use insert_internal_and_unorphan on instances with no parent");

        self.instances.insert(instance.get_id(), instance);
        self.unorphan_instance(id, parent_id);
    }

    fn unorphan_instance(&mut self, id: Ref, parent_id: Ref) {
        {
            let instance = self
                .instances
                .get_mut(&id)
                .expect("Cannot unorphan and instance not in this tree");

            instance.parent = Some(parent_id);
        }

        let parent = self
            .instances
            .get_mut(&parent_id)
            .expect("Cannot unorphan into an instance not in this tree");

        parent.children.push(id);
    }
}

/// An iterator over all descendants of an instance in an [`WeakDom`]. Returned
/// by [`WeakDom::descendants`].
///
/// [`WeakDom`]: struct.WeakDom.html
/// [`WeakDom::descendants`]: struct.WeakDom.html#method.descendants
pub struct Descendants<'a> {
    tree: &'a WeakDom,
    ids_to_visit: Vec<Ref>,
}

impl<'a> Iterator for Descendants<'a> {
    type Item = &'a RbxInstance;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(id) = self.ids_to_visit.pop() {
            if let Some(instance) = self.tree.get_instance(id) {
                for child_id in &instance.children {
                    self.ids_to_visit.push(*child_id);
                }

                return Some(instance);
            }
        }

        None
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use std::collections::HashSet;

    #[test]
    fn descendants() {
        let mut tree = WeakDom::new(RbxInstanceProperties {
            name: "Place 1".to_owned(),
            class_name: "DataModel".to_owned(),
            properties: HashMap::new(),
        });

        let root_ref = tree.get_root_ref();

        let a_id = tree.insert_instance(
            RbxInstanceProperties {
                name: "A".to_owned(),
                class_name: "Folder".to_owned(),
                properties: HashMap::new(),
            },
            root_ref,
        );

        let b_id = tree.insert_instance(
            RbxInstanceProperties {
                name: "B".to_owned(),
                class_name: "Folder".to_owned(),
                properties: HashMap::new(),
            },
            root_ref,
        );

        let c_id = tree.insert_instance(
            RbxInstanceProperties {
                name: "C".to_owned(),
                class_name: "Folder".to_owned(),
                properties: HashMap::new(),
            },
            b_id,
        );

        let mut seen_ids = HashSet::new();

        for instance in tree.descendants(root_ref) {
            assert!(seen_ids.insert(instance.get_id()));
        }

        assert_eq!(seen_ids.len(), 3);
        assert!(seen_ids.contains(&a_id));
        assert!(seen_ids.contains(&b_id));
        assert!(seen_ids.contains(&c_id));
    }

    #[test]
    fn move_instances() {
        let mut source_tree = WeakDom::new(RbxInstanceProperties {
            name: "Place 1".to_owned(),
            class_name: "DataModel".to_owned(),
            properties: HashMap::new(),
        });

        let source_root_ref = source_tree.get_root_ref();

        let a_id = source_tree.insert_instance(
            RbxInstanceProperties {
                name: "A".to_owned(),
                class_name: "Folder".to_owned(),
                properties: HashMap::new(),
            },
            source_root_ref,
        );

        let b_id = source_tree.insert_instance(
            RbxInstanceProperties {
                name: "B".to_owned(),
                class_name: "Folder".to_owned(),
                properties: HashMap::new(),
            },
            a_id,
        );

        let c_id = source_tree.insert_instance(
            RbxInstanceProperties {
                name: "C".to_owned(),
                class_name: "Folder".to_owned(),
                properties: HashMap::new(),
            },
            a_id,
        );

        let mut dest_tree = WeakDom::new(RbxInstanceProperties {
            name: "Place 2".to_owned(),
            class_name: "DataModel".to_owned(),
            properties: HashMap::new(),
        });

        let dest_root_ref = dest_tree.get_root_ref();

        source_tree.move_instance(a_id, &mut dest_tree, dest_root_ref);

        assert!(source_tree.get_instance(a_id).is_none());
        assert!(source_tree.get_instance(b_id).is_none());
        assert!(source_tree.get_instance(c_id).is_none());
        assert_eq!(
            source_tree
                .get_instance(source_root_ref)
                .unwrap()
                .get_children_ids()
                .len(),
            0
        );

        assert!(dest_tree.get_instance(a_id).is_some());
        assert!(dest_tree.get_instance(b_id).is_some());
        assert!(dest_tree.get_instance(c_id).is_some());
        assert_eq!(
            dest_tree
                .get_instance(dest_root_ref)
                .unwrap()
                .get_children_ids()
                .len(),
            1
        );
        assert_eq!(
            dest_tree.get_instance(a_id).unwrap().get_children_ids(),
            &[b_id, c_id]
        );
    }

    #[test]
    fn set_parent() {
        let mut tree = WeakDom::new(RbxInstanceProperties {
            name: "Place 1".to_owned(),
            class_name: "DataModel".to_owned(),
            properties: HashMap::new(),
        });

        let root_ref = tree.get_root_ref();

        let a_id = tree.insert_instance(
            RbxInstanceProperties {
                name: "A".to_owned(),
                class_name: "A".to_owned(),
                properties: HashMap::new(),
            },
            root_ref,
        );

        let b_id = tree.insert_instance(
            RbxInstanceProperties {
                name: "B".to_owned(),
                class_name: "B".to_owned(),
                properties: HashMap::new(),
            },
            root_ref,
        );

        tree.set_parent(a_id, b_id);

        let a = tree.get_instance(a_id).unwrap();
        assert_eq!(a.get_parent_id(), Some(b_id));
    }
}
