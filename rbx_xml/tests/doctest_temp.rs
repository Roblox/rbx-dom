#[test]
fn doctest_temp() {
    use rbx_dom_weak::RbxValue;

    let model_file = r#"
    <roblox version="4">
        <Item class="NumberValue" referent="RBX3B3D9D3DB43D4E6793B190B081E0A886">
            <Properties>
                <string name="Name">My NumberValue</string>
                <double name="Value">12345</double>
            </Properties>
        </Item>
    </roblox>
    "#;

    let tree = rbx_xml::from_str_default(model_file)
        .expect("Couldn't decode model file");

    let data_model = tree.get_instance(tree.get_root_id()).unwrap();
    let number_value_id = data_model.get_children_ids()[0];

    let number_value = tree.get_instance(number_value_id).unwrap();

    assert_eq!(
        number_value.properties.get("Value"),
        Some(&RbxValue::Float64 { value: 12345.0 }),
    );
}