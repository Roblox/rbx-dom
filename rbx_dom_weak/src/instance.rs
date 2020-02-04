use std::collections::HashMap;

use rbx_types::{Ref, Variant};

#[derive(Debug)]
pub struct InstanceBuilder {
    pub(crate) referent: Ref,
    pub(crate) name: String,
    pub(crate) class: String,
    pub(crate) properties: HashMap<String, Variant>,
    pub(crate) children: Vec<InstanceBuilder>,
}

impl InstanceBuilder {
    pub fn new<S: Into<String>>(class: S) -> Self {
        let class = class.into();
        let name = class.clone();

        InstanceBuilder {
            referent: Ref::new(),
            name,
            class,
            properties: HashMap::new(),
            children: Vec::new(),
        }
    }

    pub fn with_name<S: Into<String>>(self, name: S) -> Self {
        Self {
            name: name.into(),
            ..self
        }
    }

    pub fn with_property<K: Into<String>, V: Into<Variant>>(mut self, key: K, value: V) -> Self {
        self.properties.insert(key.into(), value.into());
        self
    }

    pub fn with_properties<K, V, I>(mut self, props: I) -> Self
    where
        K: Into<String>,
        V: Into<Variant>,
        I: IntoIterator<Item = (K, V)>,
    {
        for (key, value) in props {
            self.properties.insert(key.into(), value.into());
        }

        self
    }

    pub fn with_child(mut self, child: InstanceBuilder) -> Self {
        self.children.push(child);
        self
    }

    pub fn with_children<I: IntoIterator<Item = InstanceBuilder>>(mut self, children: I) -> Self {
        self.children.extend(children.into_iter());
        self
    }
}

#[derive(Debug)]
pub struct Instance {
    pub(crate) referent: Ref,
    pub(crate) children: Vec<Ref>,
    pub(crate) parent: Option<Ref>,

    pub name: String,
    pub class: String,
    pub properties: HashMap<String, Variant>,
}

impl Instance {
    pub fn referent(&self) -> Ref {
        self.referent
    }

    pub fn children(&self) -> &[Ref] {
        &self.children
    }

    pub fn parent(&self) -> Option<Ref> {
        self.parent
    }
}
