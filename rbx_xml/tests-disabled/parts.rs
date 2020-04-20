use std::str;

use rbx_dom_weak::RbxValue;

static TEST_FILE: &[u8] = include_bytes!("../test-files/part.rbxmx");

fn float_approx_eq(a: f32, b: f32) {
    assert!((a - b).abs() < 0.001);
}

fn color3_approx_eq(a: [f32; 3], b: [f32; 3]) {
    float_approx_eq(a[0], b[0]);
    float_approx_eq(a[1], b[1]);
    float_approx_eq(a[2], b[2]);
}

#[test]
fn part_color() {
    let _ = env_logger::try_init();

    let tree = rbx_xml::from_reader_default(TEST_FILE).unwrap();
    let root_id = tree.get_root_id();

    let root_instance = tree.get_instance(root_id).unwrap();
    let part_id = root_instance.get_children_ids()[0];
    let part = tree.get_instance(part_id).unwrap();

    let color = part
        .properties
        .get("Color")
        .expect("Missing 'Color' property");

    match color {
        RbxValue::Color3 { value } => {
            color3_approx_eq(*value, [0.639216, 0.635294, 0.647059]);
        }
        _ => panic!(
            "Color property wrong type, expected Color3, got value {:?}",
            color
        ),
    }

    let mut buffer = Vec::new();
    rbx_xml::to_writer_default(&mut buffer, &tree, &[part_id]).unwrap();

    // The serialized form should contain a Color3uint8 property named
    // Color3uint8. This is a kludge to check that it does!
    let as_str = str::from_utf8(&buffer).unwrap();
    assert!(as_str.contains(r#"<Color3uint8 name="Color3uint8""#));

    let new_tree = rbx_xml::from_reader_default(buffer.as_slice()).unwrap();
    let new_root_id = new_tree.get_root_id();

    let new_root_instance = new_tree.get_instance(new_root_id).unwrap();
    let new_part_id = new_root_instance.get_children_ids()[0];
    let new_part = new_tree.get_instance(new_part_id).unwrap();

    assert_eq!(part.properties, new_part.properties);
}
