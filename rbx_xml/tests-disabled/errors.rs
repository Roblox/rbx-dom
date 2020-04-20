use std::mem::size_of;

#[test]
fn errors_are_small() {
    assert!(size_of::<rbx_xml::DecodeError>() <= 8);
    assert!(size_of::<rbx_xml::EncodeError>() <= 8);
}

#[test]
fn first_line_bad_xml() {
    let doc = "hi";

    let err = rbx_xml::from_str_default(doc).unwrap_err();

    assert_eq!(err.line(), 1);
    assert_eq!(err.column(), 0);
}

#[test]
fn bad_version() {
    let doc = r#"<roblox version="3"></roblox>"#;

    let err = rbx_xml::from_str_default(doc).unwrap_err();

    assert_eq!(err.line(), 1);
    assert_eq!(err.column(), 0);
}

#[test]
fn second_line_gunk() {
    let doc = r#"<roblox version="4">
        <asfk />
    </roblox>"#;

    let err = rbx_xml::from_str_default(doc).unwrap_err();

    assert_eq!(err.line(), 2);
    assert_eq!(err.column(), 8);
}
