use rbx_dom_weak::{RbxTree, RbxValue, SharedString};

static TEST_FILE: &[u8] = include_bytes!("../test-files/union.rbxmx");

#[test]
fn shared_string_round_trip() {
    let _ = env_logger::try_init();

    // The property we're looking for, PhysicalConfigData, is not in the
    // reflection database right now.
    //
    // We can update this test to reference a SharedString type that is in the
    // reflection database if there ever is one.
    let decode_options = rbx_xml::DecodeOptions::new()
        .property_behavior(rbx_xml::DecodePropertyBehavior::ReadUnknown);

    let encode_options = rbx_xml::EncodeOptions::new()
        .property_behavior(rbx_xml::EncodePropertyBehavior::WriteUnknown);

    let first_tree = rbx_xml::from_reader(TEST_FILE, decode_options.clone()).unwrap();
    let first_value = get_config_data(&first_tree);

    // This buffer should have some data in it
    assert!(first_value.data().len() > 256);

    let root_id = first_tree.get_root_id();
    let root_instance = first_tree.get_instance(root_id).unwrap();
    let model_id = root_instance.get_children_ids()[0];

    let mut buffer = Vec::new();
    rbx_xml::to_writer(&mut buffer, &first_tree, &[model_id], encode_options).unwrap();

    let second_tree = rbx_xml::from_reader(buffer.as_slice(), decode_options).unwrap();
    let second_value = get_config_data(&second_tree);

    assert!(first_value.md5_hash() == second_value.md5_hash());
    assert!(first_value.data() == second_value.data());
}

/// Pulls out the PhysicalConfigData property of our target instance so we can
/// make assertions about it.
fn get_config_data(tree: &RbxTree) -> SharedString {
    let root_id = tree.get_root_id();

    let root_instance = tree.get_instance(root_id).unwrap();

    let part_id = root_instance.get_children_ids()[0];
    let part_instance = tree.get_instance(part_id).unwrap();

    let physical_config_data = part_instance
        .properties
        .get("PhysicalConfigData")
        .expect("Expected property PhysicalConfigData to be present");

    match physical_config_data {
        RbxValue::SharedString { value } => value.clone(),
        _ => panic!("PhysicalConfigData was not a SharedString value"),
    }
}
