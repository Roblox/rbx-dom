//! This file has tests ported out of the deserializer that should be broken
//! apart and refactored eventually.

use rbx_dom_weak::RbxValue;

fn floats_approx_equal(left: f32, right: f32, epsilon: f32) -> bool {
    (left - right).abs() <= epsilon
}

#[test]
fn empty_document() {
    let _ = env_logger::try_init();
    let document = r#"<roblox version="4"></roblox>"#;

    rbx_xml::from_str_default(document).unwrap();
}

#[test]
fn wrong_version() {
    let _ = env_logger::try_init();
    let document = r#"<roblox version="5"></roblox>"#;

    assert!(rbx_xml::from_str_default(document).is_err());
}

#[test]
fn mostly_empty() {
    let _ = env_logger::try_init();
    let document = r#"
        <roblox version="4">
            <!-- hello there! -->
            <Meta name="Trash">true</Meta>
        </roblox>
    "#;

    rbx_xml::from_str_default(document).unwrap();
}

#[test]
fn top_level_garbage() {
    let _ = env_logger::try_init();
    let document = r#"
        <roblox version="4">
            <ack />
        </roblox>
    "#;

    assert!(rbx_xml::from_str_default(document).is_err());
}

#[test]
fn empty_instance() {
    let _ = env_logger::try_init();
    let document = r#"
        <roblox version="4">
            <Item class="Folder" referent="hello">
            </Item>
        </roblox>
    "#;

    let tree = rbx_xml::from_str_default(document).unwrap();

    let root = tree.get_instance(tree.get_root_id()).unwrap();
    assert_eq!(root.get_children_ids().len(), 1);
}

#[test]
fn children() {
    let _ = env_logger::try_init();
    let document = r#"
        <roblox version="4">
            <Item class="Folder" referent="hello">
                <Properties>
                    <string name="Name">Outer</string>
                </Properties>
                <Item class="Folder" referent="child">
                    <Properties>
                        <string name="Name">Inner</string>
                    </Properties>
                </Item>
            </Item>
        </roblox>
    "#;

    let tree = rbx_xml::from_str_default(document).unwrap();
    let root_id = tree.get_root_id();
    let root = tree.get_instance(root_id).unwrap();
    let first_folder = tree
        .get_instance(root.get_children_ids()[0])
        .expect("expected a child");
    let inner_folder = tree
        .get_instance(first_folder.get_children_ids()[0])
        .expect("expected a subchild");
    assert_eq!(first_folder.name, "Outer");
    assert_eq!(inner_folder.name, "Inner");
}

#[test]
fn canonicalized_names() {
    let _ = env_logger::try_init();
    let document = r#"
        <roblox version="4">
            <Item class="Part" referent="hello">
                <Properties>
                    <Vector3 name="size">
                        <X>123.0</X>
                        <Y>456.0</Y>
                        <Z>789.0</Z>
                    </Vector3>
                </Properties>
            </Item>
        </roblox>
    "#;

    let tree = rbx_xml::from_str_default(document).unwrap();
    let root_id = tree.get_root_id();

    let root_instance = tree.get_instance(root_id).unwrap();
    let descendant = tree
        .get_instance(root_instance.get_children_ids()[0])
        .unwrap();

    assert_eq!(descendant.name, "Part");
    assert_eq!(descendant.class_name, "Part");
    assert_eq!(
        descendant.properties.get("Size"),
        Some(&RbxValue::Vector3 {
            value: [123.0, 456.0, 789.0]
        })
    );
}

#[test]
fn with_bool() {
    let _ = env_logger::try_init();
    let document = r#"
        <roblox version="4">
            <Item class="BoolValue" referent="hello">
                <Properties>
                    <bool name="Value">true</bool>
                </Properties>
            </Item>
        </roblox>
    "#;

    let tree = rbx_xml::from_str_default(document).unwrap();
    let root_id = tree.get_root_id();

    let root_instance = tree.get_instance(root_id).unwrap();
    let descendant = tree
        .get_instance(root_instance.get_children_ids()[0])
        .unwrap();

    assert_eq!(descendant.name, "BoolValue");
    assert_eq!(descendant.class_name, "BoolValue");
    assert_eq!(
        descendant.properties.get("Value"),
        Some(&RbxValue::Bool { value: true })
    );
}

#[test]
fn with_vector3() {
    let _ = env_logger::try_init();
    let document = r#"
        <roblox version="4">
            <Item class="Vector3Value" referent="hello">
                <Properties>
                    <string name="Name">Test</string>
                    <Vector3 name="Value">
                        <X>0</X>
                        <Y>0.25</Y>
                        <Z>-123.23</Z>
                    </Vector3>
                </Properties>
            </Item>
        </roblox>
    "#;

    let tree = rbx_xml::from_str_default(document).unwrap();
    let root_id = tree.get_root_id();

    let root_instance = tree.get_instance(root_id).unwrap();
    let descendant = tree
        .get_instance(root_instance.get_children_ids()[0])
        .unwrap();

    assert_eq!(descendant.name, "Test");
    assert_eq!(descendant.class_name, "Vector3Value");
    assert_eq!(
        descendant.properties.get("Value"),
        Some(&RbxValue::Vector3 {
            value: [0.0, 0.25, -123.23]
        })
    );
}

#[test]
fn with_color3() {
    let _ = env_logger::try_init();
    let document = r#"
        <roblox version="4">
            <Item class="Color3Value" referent="hello">
                <Properties>
                    <string name="Name">Test</string>
                    <Color3 name="Value">
                        <R>0</R>
                        <G>0.25</G>
                        <B>0.75</B>
                    </Color3>
                </Properties>
            </Item>
            <Item class="Color3Value" referent="hello">
                <Properties>
                    <string name="Name">Test2</string>
                    <Color3 name="Value">4294934592</Color3>
                </Properties>
            </Item>
        </roblox>
    "#;

    let tree = rbx_xml::from_str_default(document).unwrap();
    let root_id = tree.get_root_id();

    for descendant in tree.descendants(root_id) {
        if descendant.name == "Test" {
            assert_eq!(
                descendant.properties.get("Value"),
                Some(&RbxValue::Color3 {
                    value: [0.0, 0.25, 0.75]
                })
            );
        } else if descendant.name == "Test2" {
            if let Some(&RbxValue::Color3 { value }) = descendant.properties.get("Value") {
                assert!(floats_approx_equal(value[0], 1.0, 0.001));
                assert!(floats_approx_equal(value[1], 0.501961, 0.001));
                assert!(floats_approx_equal(value[2], 0.250980, 0.001));
            } else {
                panic!("value was not a Color3 or did not deserialize properly");
            }
        }
    }
}

#[test]
fn with_color3uint8() {
    let _ = env_logger::try_init();
    let document = r#"
        <roblox version="4">
            <Item class="Color3Value" referent="hello">
                <Properties>
                    <string name="Name">Test</string>
                    <Color3uint8 name="Value">4294934592</Color3uint8>
                </Properties>
            </Item>
        </roblox>
    "#;

    let tree = rbx_xml::from_str_default(document).unwrap();
    let root_id = tree.get_root_id();

    let root_instance = tree.get_instance(root_id).unwrap();
    let descendant = tree
        .get_instance(root_instance.get_children_ids()[0])
        .unwrap();

    assert_eq!(descendant.name, "Test");
    assert_eq!(descendant.class_name, "Color3Value");

    // With reflection-based serialization and property value conversion, the
    // Color3uint8 value will be converted to Color3 on deserialization!

    let value = descendant
        .properties
        .get("Value")
        .expect("Missing 'Value' property");

    match value {
        RbxValue::Color3 { value } => {
            let epsilon = 1.0 / 255.0;

            floats_approx_equal(value[0], 1.0, epsilon);
            floats_approx_equal(value[1], 0.5, epsilon);
            floats_approx_equal(value[2], 0.25, epsilon);
        }
        _ => panic!("Expected Color3, got {:?}", value),
    }
}

#[test]
fn with_cframe() {
    let _ = env_logger::try_init();
    let document = r#"
        <roblox version="4">
            <Item class="CFrameValue" referent="hello">
                <Properties>
                    <string name="Name">Test</string>
                    <CoordinateFrame name="Value">
                        <X>0</X>
                        <Y>0.5</Y>
                        <Z>0</Z>
                        <R00>1</R00>
                        <R01>0</R01>
                        <R02>0</R02>
                        <R10>0</R10>
                        <R11>1</R11>
                        <R12>0</R12>
                        <R20>0</R20>
                        <R21>0</R21>
                        <R22>1</R22>
                    </CoordinateFrame>
                </Properties>
            </Item>
        </roblox>
    "#;

    let tree = rbx_xml::from_str_default(document).unwrap();
    let root_id = tree.get_root_id();

    let root_instance = tree.get_instance(root_id).unwrap();
    let descendant = tree
        .get_instance(root_instance.get_children_ids()[0])
        .unwrap();

    assert_eq!(descendant.name, "Test");
    assert_eq!(descendant.class_name, "CFrameValue");
    assert_eq!(
        descendant.properties.get("Value"),
        Some(&RbxValue::CFrame {
            value: [0.0, 0.5, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0,],
        })
    );
}

#[test]
fn with_ref_some() {
    let _ = env_logger::try_init();
    let document = r#"
        <roblox version="4">
            <Item class="Folder" referent="RBX1B9CDD1FD0884F76BFE6091C1731E1FB">
            </Item>

            <Item class="ObjectValue" referent="hello">
                <Properties>
                    <string name="Name">Test</string>
                    <Ref name="Value">RBX1B9CDD1FD0884F76BFE6091C1731E1FB</Ref>
                </Properties>
            </Item>
        </roblox>
    "#;

    let tree = rbx_xml::from_str_default(document).unwrap();
    let root_id = tree.get_root_id();

    let root_instance = tree.get_instance(root_id).unwrap();
    let target_instance_id = root_instance.get_children_ids()[0];
    let source_instance_id = root_instance.get_children_ids()[1];

    let source_instance = tree.get_instance(source_instance_id).unwrap();
    assert_eq!(source_instance.name, "Test");
    assert_eq!(source_instance.class_name, "ObjectValue");

    let value = source_instance.properties.get("Value").unwrap();
    if let RbxValue::Ref { value } = value {
        assert_eq!(value.unwrap(), target_instance_id);
    } else {
        panic!("RBXValue was not Ref, but instead {:?}", value);
    }
}

#[test]
fn with_ref_none() {
    let _ = env_logger::try_init();
    let document = r#"
        <roblox version="4">
            <Item class="ObjectValue" referent="hello">
                <Properties>
                    <string name="Name">Test</string>
                    <Ref name="Value">null</Ref>
                </Properties>
            </Item>
        </roblox>
    "#;

    let tree = rbx_xml::from_str_default(document).unwrap();
    let root_id = tree.get_root_id();

    let root_instance = tree.get_instance(root_id).unwrap();
    let descendant = tree
        .get_instance(root_instance.get_children_ids()[0])
        .unwrap();

    assert_eq!(descendant.name, "Test");
    assert_eq!(descendant.class_name, "ObjectValue");

    let value = descendant
        .properties
        .get("Value")
        .expect("no value property");
    assert_eq!(value, &RbxValue::Ref { value: None });
}
