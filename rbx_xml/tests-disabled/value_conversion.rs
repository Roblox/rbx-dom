use rbx_dom_weak::RbxValue;

static TEST_FILE: &[u8] = include_bytes!("../test-files/number-value-int32.rbxmx");

#[test]
fn f32_to_f64() {
    let _ = env_logger::try_init();

    let tree = rbx_xml::from_reader_default(TEST_FILE).unwrap();
    let root_id = tree.get_root_id();

    let root_instance = tree.get_instance(root_id).unwrap();
    let value_id = root_instance.get_children_ids()[0];
    let value = tree.get_instance(value_id).unwrap();

    assert_eq!(value.name, "A NumberValue");
    assert_eq!(value.class_name, "NumberValue");
    assert_eq!(
        value.properties.get("Value"),
        Some(&RbxValue::Float64 { value: 308.0 })
    );
}
