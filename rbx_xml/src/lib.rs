//! Configurable Roblox XML place/model format (rbxmx and rbxlx) serializer and
//! deserializer.
//!
//! rbx_xml uses the [rbx_dom_weak][rbx_dom_weak] crate as its DOM.
//!
//! This crate implements most of the format and is driven by an up-to-date
//! reflection database.
//!
//! To decode a place, use a method like `from_reader` if you're reading from a
//! file, or `from_str` if you already have a string. These methods also have
//! variants that pass in default configuration.
//!
//! ```
//! use rbx_dom_weak::RbxValue;
//!
//! let model_file = r#"
//! <roblox version="4">
//!     <Item class="NumberValue" referent="RBX3B3D9D3DB43D4E6793B190B081E0A886">
//!         <Properties>
//!             <string name="Name">My NumberValue</string>
//!             <double name="Value">12345</double>
//!         </Properties>
//!     </Item>
//! </roblox>
//! "#;
//!
//! let tree = rbx_xml::from_str_default(model_file)
//!     .expect("Couldn't decode model file");
//!
//! let data_model = tree.get_instance(tree.get_root_id()).unwrap();
//! let number_value_id = data_model.get_children_ids()[0];
//!
//! let number_value = tree.get_instance(number_value_id).unwrap();
//!
//! assert_eq!(
//!     number_value.properties.get("Value"),
//!     Some(&RbxValue::Float64 { value: 12345.0 }),
//! );
//! ```
//!
//! [rbx_dom_weak]: https://crates.io/crates/rbx_dom_weak

#![deny(missing_docs)]

mod core;
mod deserializer;
mod deserializer_core;
mod error;
mod serializer;
mod serializer_core;
mod types;

#[cfg(test)]
mod test_util;

use std::io::{Read, Write};

use rbx_dom_weak::{RbxTree, RbxId};

use crate::{
    deserializer::decode_internal,
    serializer::encode_internal,
};

pub use crate::{
    error::{EncodeError, DecodeError},
    deserializer::DecodeOptions,
    serializer::EncodeOptions,
};

/// Decodes an XML-format model or place from something that implements the
/// `std::io::Read` trait.
pub fn from_reader<R: Read>(reader: R, options: DecodeOptions) -> Result<RbxTree, DecodeError> {
    decode_internal(reader, options)
}

/// Decodes an XML-format model or place from something that implements the
/// `std::io::Read` trait using the default decoder options.
pub fn from_reader_default<R: Read>(reader: R) -> Result<RbxTree, DecodeError> {
    decode_internal(reader, DecodeOptions::default())
}

/// Decodes an XML-format model or place from a string.
pub fn from_str<S: AsRef<str>>(reader: S, options: DecodeOptions) -> Result<RbxTree, DecodeError> {
    decode_internal(reader.as_ref().as_bytes(), options)
}

/// Decodes an XML-format model or place from a string using the default decoder
/// options.
pub fn from_str_default<S: AsRef<str>>(reader: S) -> Result<RbxTree, DecodeError> {
    decode_internal(reader.as_ref().as_bytes(), DecodeOptions::default())
}

/// Serializes a subset of the given tree to an XML format model or place,
/// writing to something that implements the `std::io::Write` trait.
pub fn to_writer<W: Write>(writer: W, tree: &RbxTree, ids: &[RbxId], options: EncodeOptions) -> Result<(), EncodeError> {
    encode_internal(writer, tree, ids, options)
}

/// Serializes a subset of the given tree to an XML format model or place,
/// writing to something that implements the `std::io::Write` trait using the
/// default encoder options.
pub fn to_writer_default<W: Write>(writer: W, tree: &RbxTree, ids: &[RbxId]) -> Result<(), EncodeError> {
    encode_internal(writer, tree, ids, EncodeOptions::default())
}