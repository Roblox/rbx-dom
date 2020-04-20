use std::collections::HashMap;

use rbx_dom_weak::{RbxInstanceProperties, RbxTree, RbxValue};
use rbx_xml::{EncodeOptions, EncodePropertyBehavior};

fn make_test_tree() -> RbxTree {
    let mut properties = HashMap::new();

    properties.insert("FormFactor".to_owned(), RbxValue::Enum { value: 1 });

    properties.insert(
        "UnknownProperty".to_owned(),
        RbxValue::String {
            value: "ahhhh".to_owned(),
        },
    );

    RbxTree::new(RbxInstanceProperties {
        name: "Foo".to_owned(),
        class_name: "Part".to_owned(),
        properties,
    })
}

#[test]
fn ignore_unknown_properties() {
    let options = EncodeOptions::new().property_behavior(EncodePropertyBehavior::IgnoreUnknown);

    let tree = make_test_tree();

    let mut output = Vec::new();
    rbx_xml::to_writer(&mut output, &tree, &[tree.get_root_id()], options)
        .expect("Couldn't encode tree");

    let output = String::from_utf8(output).expect("Couldn't convert output to UTF-8");

    assert!(output.contains("formFactorRaw"));
    assert!(!output.contains("FormFactor"));
    assert!(!output.contains("UnknownProperty"));
    assert!(!output.contains("ahhhh"));
}

#[test]
fn write_unknown_properties() {
    let options = EncodeOptions::new().property_behavior(EncodePropertyBehavior::WriteUnknown);

    let tree = make_test_tree();

    let mut output = Vec::new();
    rbx_xml::to_writer(&mut output, &tree, &[tree.get_root_id()], options)
        .expect("Couldn't encode tree");

    let output = String::from_utf8(output).expect("Couldn't convert output to UTF-8");

    assert!(output.contains("formFactorRaw"));
    assert!(!output.contains("FormFactor"));
    assert!(output.contains("UnknownProperty"));
    assert!(output.contains("ahhhh"));
}

#[test]
fn error_on_unknown_properties() {
    let options = EncodeOptions::new().property_behavior(EncodePropertyBehavior::ErrorOnUnknown);

    let tree = make_test_tree();

    let mut output = Vec::new();
    rbx_xml::to_writer(&mut output, &tree, &[tree.get_root_id()], options)
        .expect_err("Successfully encoded malformed tree");
}

#[test]
fn no_reflection() {
    let options = EncodeOptions::new().property_behavior(EncodePropertyBehavior::NoReflection);

    let tree = make_test_tree();

    let mut output = Vec::new();
    rbx_xml::to_writer(&mut output, &tree, &[tree.get_root_id()], options)
        .expect("Couldn't encode tree");

    let output = String::from_utf8(output).expect("Couldn't convert output to UTF-8");

    assert!(!output.contains("formFactorRaw"));
    assert!(output.contains("FormFactor"));
    assert!(output.contains("UnknownProperty"));
    assert!(output.contains("ahhhh"));
}
