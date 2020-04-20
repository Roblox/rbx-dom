//! Configurable Roblox XML place/model format (rbxmx and rbxlx) serializer and
//! deserializer.
//!
//! rbx_xml uses the [rbx_dom_weak][rbx_dom_weak] crate as its DOM.
//!
//! This crate implements most of the format and is driven by an up-to-date
//! reflection database.
//!
//! ## Deserialization
//! To decode a place or model, use a method like
//! [`from_reader_default`][from_reader_default] if you're reading from a file,
//! or [`from_str_default`][from_str_default] if you already have a string.
//! These methods also have variants like [`from_str`][from_str] that let you
//! pass in custom options.
//!
//! ```ignore
//! # // FIXME: This test overflows its stack only as a doctest on Windows. :/
//! # // see: https://github.com/rust-lang/rust/issues/60753
//! #
//! # std::thread::spawn(|| {
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
//! let model = rbx_xml::from_str_default(model_file)
//!     .expect("Couldn't decode model file");
//!
//! let data_model = model.get_instance(model.get_root_id()).unwrap();
//! let number_value_id = data_model.get_children_ids()[0];
//!
//! let number_value = model.get_instance(number_value_id).unwrap();
//!
//! assert_eq!(
//!     number_value.properties.get("Value"),
//!     Some(&RbxValue::Float64 { value: 12345.0 }),
//! );
//! #
//! # });
//! ```
//!
//! If you're decoding from a file, you'll want to do your own I/O buffering,
//! like with [`BufReader`][BufReader]:
//!
//! ```ignore
//! # fn main() -> Result<(), Box<std::error::Error>> {
//! use std::{
//!     io::BufReader,
//!     fs::File,
//! };
//!
//! let file = BufReader::new(File::open("place.rbxlx")?);
//! let place = rbx_xml::from_reader_default(file)?;
//! # Ok(())
//! # }
//! ```
//!
//! Note that the `WeakDom` instance returned by the rbx_xml decode methods will
//! have a root instance with the class name `DataModel`. This is great for
//! deserializing a place, but kind of strange for deserializing a model.
//!
//! Because models can have multiple instances at the top level, rbx_xml can't
//! just return an `WeakDom` with your single instance at the top. Instead, the
//! crate instead always creates a top-level `DataModel` instance which is
//! pretty close to free.
//!
//! ## Serialization
//! To serialize an existing `WeakDom` instance, use methods like
//! [`to_writer_default`][to_writer_default] or [`to_writer`][to_writer].
//!
//! For example, to re-save the place file we loaded above:
//!
//! ```ignore
//! # fn main() -> Result<(), Box<std::error::Error>> {
//! use std::{
//!     io::BufWriter,
//!     fs::File,
//! };
//! # use rbx_dom_weak::{WeakDom, RbxInstanceProperties};
//!
//! # let place = WeakDom::new(RbxInstanceProperties {
//! #   class_name: "DataModel".to_owned(),
//! #   name: "DataModel".to_owned(),
//! #   properties: Default::default(),
//! # });
//! // A Roblox place file contains all of its top-level instances.
//! let data_model = place.get_instance(place.get_root_id()).unwrap();
//! let top_level_ids = data_model.get_children_ids();
//!
//! // Just like when reading a place file, we should buffer our I/O.
//! let file = BufWriter::new(File::create("place-2.rbxlx")?);
//!
//! rbx_xml::to_writer_default(file, &place, top_level_ids)?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Configuration
//! rbx_xml exposes no useful configuration yet, but there are methods that
//! accept [`DecodeOptions`][DecodeOptions] and
//! [`EncodeOptions`][EncodeOptions] that will be useful when it does.
//!
//! [DecodeOptions]: struct.DecodeOptions.html
//! [EncodeOptions]: struct.EncodeOptions.html
//! [from_str]: fn.from_str.html
//! [from_reader_default]: fn.from_reader_default.html
//! [from_str_default]: fn.from_str_default.html
//! [to_writer]: fn.to_writer.html
//! [to_writer_default]: fn.to_writer_default.html
//! [rbx_dom_weak]: https://crates.io/crates/rbx_dom_weak
//! [BufReader]: https://doc.rust-lang.org/std/io/struct.BufReader.html

#![deny(missing_docs)]

mod compat;

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

use rbx_dom_weak::{types::Ref, WeakDom};

use crate::{deserializer::decode_internal, serializer::encode_internal};

pub use crate::{
    deserializer::{DecodeOptions, DecodePropertyBehavior},
    error::{DecodeError, EncodeError},
    serializer::{EncodeOptions, EncodePropertyBehavior},
};

/// Decodes an XML-format model or place from something that implements the
/// `std::io::Read` trait.
pub fn from_reader<R: Read>(reader: R, options: DecodeOptions) -> Result<WeakDom, DecodeError> {
    decode_internal(reader, options)
}

/// Decodes an XML-format model or place from something that implements the
/// `std::io::Read` trait using the default decoder options.
pub fn from_reader_default<R: Read>(reader: R) -> Result<WeakDom, DecodeError> {
    decode_internal(reader, DecodeOptions::default())
}

/// Decodes an XML-format model or place from a string.
pub fn from_str<S: AsRef<str>>(reader: S, options: DecodeOptions) -> Result<WeakDom, DecodeError> {
    decode_internal(reader.as_ref().as_bytes(), options)
}

/// Decodes an XML-format model or place from a string using the default decoder
/// options.
pub fn from_str_default<S: AsRef<str>>(reader: S) -> Result<WeakDom, DecodeError> {
    decode_internal(reader.as_ref().as_bytes(), DecodeOptions::default())
}

/// Serializes a subset of the given tree to an XML format model or place,
/// writing to something that implements the `std::io::Write` trait.
pub fn to_writer<W: Write>(
    writer: W,
    tree: &WeakDom,
    ids: &[Ref],
    options: EncodeOptions,
) -> Result<(), EncodeError> {
    encode_internal(writer, tree, ids, options)
}

/// Serializes a subset of the given tree to an XML format model or place,
/// writing to something that implements the `std::io::Write` trait using the
/// default encoder options.
pub fn to_writer_default<W: Write>(
    writer: W,
    tree: &WeakDom,
    ids: &[Ref],
) -> Result<(), EncodeError> {
    encode_internal(writer, tree, ids, EncodeOptions::default())
}
