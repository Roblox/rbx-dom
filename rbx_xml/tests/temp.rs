//! Temporary tests while re-bootstrapping rbx_xml

use rbx_dom_weak::types::Variant;

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

    let root = tree.root();
    let child = tree.get_by_ref(root.children()[0]).unwrap();

    assert_eq!(child.name, "BoolValue");
    assert_eq!(child.class, "BoolValue");
    assert_eq!(child.properties.get("Value"), Some(&Variant::Bool(true)));
}
