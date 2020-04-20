use std::io::Cursor;

use rbx_dom_weak::{RbxTree, RbxValue};

static TEST_FILE: &[u8] = include_bytes!("../test-files/part-referent.rbxmx");

fn assert_referents_sound(tree: &RbxTree) {
    let root_id = tree.get_root_id();

    let root_instance = tree.get_instance(root_id).unwrap();

    let model_id = root_instance.get_children_ids()[0];
    let model_instance = tree.get_instance(model_id).unwrap();

    let part_id = model_instance.get_children_ids()[0];

    let primary_part = model_instance.properties.get("PrimaryPart");
    let expected_value = RbxValue::Ref {
        value: Some(part_id),
    };

    assert_eq!(primary_part, Some(&expected_value));
}

#[test]
fn referents_work() {
    let _ = env_logger::try_init();

    let first_tree = rbx_xml::from_reader_default(TEST_FILE).unwrap();
    let root_id = first_tree.get_root_id();

    assert_referents_sound(&first_tree);

    let root_instance = first_tree.get_instance(root_id).unwrap();
    let model_id = root_instance.get_children_ids()[0];

    let mut buffer = Vec::new();
    rbx_xml::to_writer_default(Cursor::new(&mut buffer), &first_tree, &[model_id]).unwrap();

    let second_tree = rbx_xml::from_reader_default(buffer.as_slice()).unwrap();

    assert_referents_sound(&second_tree);
}
