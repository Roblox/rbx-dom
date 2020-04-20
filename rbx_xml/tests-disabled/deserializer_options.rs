use rbx_dom_weak::RbxValue;
use rbx_xml::{DecodeOptions, DecodePropertyBehavior};

static TEST_DOCUMENT: &str = r#"
    <roblox version="4">
        <Item class="StringValue" referent="A">
            <Properties>
                <string name="Name">A</string>
                <string name="Value">the value</string>
                <string name="UnknownProperty">oh nooo</string>
            </Properties>
        </Item>
    </roblox>
"#;

#[test]
fn ignore_unknown_properties() {
    let options = DecodeOptions::new().property_behavior(DecodePropertyBehavior::IgnoreUnknown);

    let tree = rbx_xml::from_str(TEST_DOCUMENT, options).expect("Couldn't decode tree");

    let root_instance = tree.get_instance(tree.get_root_id()).unwrap();

    let child_id = root_instance.get_children_ids()[0];
    let child_instance = tree.get_instance(child_id).unwrap();

    assert_eq!(child_instance.name, "A");

    assert_eq!(
        child_instance.properties.get("Value"),
        Some(&RbxValue::String {
            value: "the value".to_owned(),
        }),
    );

    assert_eq!(child_instance.properties.get("UnknownProperty"), None,);
}

#[test]
fn read_unknown_properties() {
    let options = DecodeOptions::new().property_behavior(DecodePropertyBehavior::ReadUnknown);

    let tree = rbx_xml::from_str(TEST_DOCUMENT, options).expect("Couldn't decode tree");

    let root_instance = tree.get_instance(tree.get_root_id()).unwrap();

    let child_id = root_instance.get_children_ids()[0];
    let child_instance = tree.get_instance(child_id).unwrap();

    assert_eq!(child_instance.name, "A");

    assert_eq!(
        child_instance.properties.get("Value"),
        Some(&RbxValue::String {
            value: "the value".to_owned(),
        }),
    );

    assert_eq!(
        child_instance.properties.get("UnknownProperty"),
        Some(&RbxValue::String {
            value: "oh nooo".to_owned(),
        }),
    );
}

#[test]
fn error_on_unknown_properties() {
    let options = DecodeOptions::new().property_behavior(DecodePropertyBehavior::ErrorOnUnknown);

    rbx_xml::from_str(TEST_DOCUMENT, options).expect_err("Expected tree to fail to deserialize");
}

#[test]
fn no_reflection() {
    let options = DecodeOptions::new().property_behavior(DecodePropertyBehavior::NoReflection);

    let tree = rbx_xml::from_str(TEST_DOCUMENT, options).expect("Couldn't decode tree");

    let root_instance = tree.get_instance(tree.get_root_id()).unwrap();

    let child_id = root_instance.get_children_ids()[0];
    let child_instance = tree.get_instance(child_id).unwrap();

    assert_eq!(child_instance.name, "A");

    assert_eq!(
        child_instance.properties.get("Value"),
        Some(&RbxValue::String {
            value: "the value".to_owned(),
        }),
    );

    assert_eq!(
        child_instance.properties.get("UnknownProperty"),
        Some(&RbxValue::String {
            value: "oh nooo".to_owned(),
        }),
    );
}

#[test]
fn no_reflection_renamed_value() {
    let document = r#"
        <roblox version="4">
            <Item class="Part" referent="A">
                <Properties>
                    <string name="Name">A</string>
                    <token name="formFactorRaw">1</token>
                </Properties>
            </Item>
        </roblox>
    "#;

    let options = DecodeOptions::new().property_behavior(DecodePropertyBehavior::NoReflection);

    let tree = rbx_xml::from_str(document, options).expect("Couldn't decode tree");

    let root_instance = tree.get_instance(tree.get_root_id()).unwrap();

    let child_id = root_instance.get_children_ids()[0];
    let child_instance = tree.get_instance(child_id).unwrap();

    assert_eq!(child_instance.name, "A");

    assert_eq!(
        child_instance.properties.get("formFactorRaw"),
        Some(&RbxValue::Enum { value: 1 }),
    );
}
