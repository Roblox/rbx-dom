//! rbx_dom_weak is a common representation of the Roblox DOM for Rust. It's
//! designed to play nicely with the borrow checker and allows accessing
//! instances by ID in constant time.
//!
//! Constructing a new tree of instances is accomplished by first creating an
//! [`RbxInstanceProperties`] object that describes the root instance of the
//! tree, and then wrapping it with an [`RbxTree`]:
//!
//! ```
//! use std::collections::HashMap;
//! use rbx_dom_weak::{RbxInstanceProperties, RbxTree};
//!
//! let props = RbxInstanceProperties {
//!     name: "My Cool Game".to_owned(),
//!     class_name: "DataModel".to_owned(),
//!     properties: HashMap::new(),
//! };
//!
//! let mut tree = RbxTree::new(props);
//! println!("ID of instance we just inserted is {:?}", tree.get_root_id());
//! ```
//!
//! Note that the [maplit] crate is incredibly useful for defining properties
//! inline.
//!
//! Once we have a tree, we can use [`RbxTree::insert_instance`] and
//! [`RbxTree::get_instance`] to add instances to the tree and retrieve them.
//!
//! ```
//! # use std::collections::HashMap;
//! # use rbx_dom_weak::{RbxInstanceProperties, RbxTree};
//! use rbx_dom_weak::types::Variant;
//! use maplit::hashmap;
//! #
//! # let props = RbxInstanceProperties {
//! #     name: "My Cool Game".to_owned(),
//! #     class_name: "DataModel".to_owned(),
//! #     properties: HashMap::new(),
//! # };
//! #
//! # let mut tree = RbxTree::new(props);
//! #
//! let http_service = RbxInstanceProperties {
//!     name: "HttpService".to_owned(),
//!     class_name: "HttpService".to_owned(),
//!     properties: hashmap! {
//          // Properties are represented via the Variant enum
//!         "HttpEnabled".to_owned() => true.into(),
//!     },
//! };
//!
//! let datamodel_id = tree.get_root_id();
//! let http_service_id = tree.insert_instance(http_service, datamodel_id);
//!
//! println!("HttpService has ID {:?}", http_service_id);
//! ```
//!
//! To change properties on an instance that's already present in the tree, use
//! [`RbxTree::get_instance_mut`]. Note that it isn't possible to add or remove
//! children through this method, use [`RbxTree::insert_instance`] instead.
//!
//! [`RbxTree`]: struct.RbxTree.html
//! [`RbxTree::insert_instance`]: struct.RbxTree.html#method.insert_instance
//! [`RbxTree::get_instance`]: struct.RbxTree.html#method.get_instance
//! [`RbxTree::get_instance_mut`]: struct.RbxTree.html#method.get_instance_mut
//! [`RbxInstanceProperties`]: struct.RbxInstanceProperties.html
//! [maplit]: https://crates.io/crates/maplit

mod instance;
mod tree;

pub use rbx_types as types;

pub use crate::{
    instance::{RbxInstance, RbxInstanceProperties},
    tree::{Descendants, RbxTree},
};
