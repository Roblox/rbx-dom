use rbx_dom_test::TreeViewer;

// Current blockers to enable: Faces, Axes, Ray
#[test]
#[ignore]
fn all_instances_415() {
    static CONTENT: &[u8] = include_bytes!("../../test-files/places/all-instances-415/xml.rbxlx");

    let tree = rbx_xml::from_reader_default(CONTENT).unwrap();

    let mut viewer = TreeViewer::new();
    insta::assert_yaml_snapshot!(viewer.view(&tree));
}
