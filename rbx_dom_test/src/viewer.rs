use std::collections::{BTreeMap, HashMap};

use rbx_dom_weak::{
    types::{Ref, Variant},
    WeakDom,
};

use serde::{Deserialize, Serialize};

/// Contains state for viewing/redacting WeakDom objects, making them suitable
/// for viewing in a snapshot test.
pub struct TreeViewer {
    id_map: HashMap<Ref, String>,
    next_id: usize,
}

impl TreeViewer {
    pub fn new() -> Self {
        Self {
            id_map: HashMap::new(),
            next_id: 0,
        }
    }

    pub fn view(&mut self, tree: &WeakDom) -> ViewedInstance {
        let root_id = tree.root_ref();
        self.populate_id_map(tree, root_id);
        self.view_instance(tree, root_id)
    }

    pub fn view_children(&mut self, tree: &WeakDom) -> Vec<ViewedInstance> {
        let root_instance = tree.root();
        let children = root_instance.children();

        for &id in children {
            self.populate_id_map(tree, id);
        }

        children
            .iter()
            .map(|&id| self.view_instance(tree, id))
            .collect()
    }

    fn populate_id_map(&mut self, tree: &WeakDom, id: Ref) {
        self.id_map.insert(id, format!("id-{}", self.next_id));
        self.next_id += 1;

        let instance = tree.get_by_ref(id).unwrap();
        for id in instance.children() {
            self.populate_id_map(tree, *id);
        }
    }

    fn view_instance(&self, tree: &WeakDom, id: Ref) -> ViewedInstance {
        let instance = tree.get_by_ref(id).unwrap();

        let children = instance
            .children()
            .iter()
            .copied()
            .map(|id| self.view_instance(tree, id))
            .collect();

        let properties = instance
            .properties
            .iter()
            .map(|(key, value)| {
                let key = key.clone();
                let new_value = match value {
                    Variant::Ref(ref_id) => {
                        let id_str = self
                            .id_map
                            .get(&ref_id)
                            .cloned()
                            .unwrap_or_else(|| "[unknown ID]".to_owned());
                        ViewedValue::Ref(id_str)
                    }
                    other => ViewedValue::Other(other.clone()),
                };

                (key, new_value)
            })
            .collect();

        ViewedInstance {
            id: self.id_map.get(&id).unwrap().clone(),
            name: instance.name.clone(),
            class_name: instance.class.clone(),
            properties,
            children,
        }
    }
}

/// A transformed view into an WeakDom or RbxInstance that has been redacted and
/// transformed to be more readable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewedInstance {
    id: String,
    name: String,
    class_name: String,
    properties: BTreeMap<String, ViewedValue>,
    children: Vec<ViewedInstance>,
}

/// Wrapper around Variant with refs replaced to be redacted, stable versions
/// of their original IDs.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ViewedValue {
    Ref(String),
    Other(Variant),
}

#[cfg(test)]
mod test {
    use super::*;

    use rbx_dom_weak::InstanceBuilder;

    #[test]
    fn redact_single() {
        let tree = WeakDom::new(InstanceBuilder::new("Folder").with_name("Root"));

        insta::assert_yaml_snapshot!(TreeViewer::new().view(&tree));
    }

    #[test]
    fn redact_multi() {
        let mut tree = WeakDom::new(InstanceBuilder::new("Folder").with_name("Root"));

        let root_id = tree.root_ref();

        for i in 0..4 {
            let builder = InstanceBuilder::new("Folder").with_name(format!("Child {}", i));

            tree.insert(root_id, builder);
        }

        insta::assert_yaml_snapshot!(TreeViewer::new().view(&tree));
    }

    #[test]
    fn redact_values() {
        let mut tree = WeakDom::new(InstanceBuilder::new("ObjectValue").with_name("Root"));

        let root_instance = tree.root_mut();

        root_instance
            .properties
            .insert("Value".to_owned(), Variant::Ref(root_instance.referent()));

        insta::assert_yaml_snapshot!(TreeViewer::new().view(&tree));
    }
}
