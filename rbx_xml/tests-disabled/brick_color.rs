use rbx_dom_weak::{BrickColor, RbxValue};

static SPAWN_LOCATION: &[u8] = include_bytes!("../test-files/spawn-location.rbxmx");
static TEAM: &[u8] = include_bytes!("../test-files/team.rbxmx");
static TEAM_LILAC: &[u8] = include_bytes!("../test-files/team-lilac.rbxmx");

#[test]
fn spawn_location() {
    let _ = env_logger::try_init();

    let tree = rbx_xml::from_reader_default(SPAWN_LOCATION).unwrap();
    let root_id = tree.get_root_id();

    let root_instance = tree.get_instance(root_id).unwrap();
    let spawn_id = root_instance.get_children_ids()[0];
    let spawn = tree.get_instance(spawn_id).unwrap();

    assert_eq!(spawn.name, "SpawnLocation");
    assert_eq!(spawn.class_name, "SpawnLocation");
    assert_eq!(
        spawn.properties.get("FormFactor"),
        Some(&RbxValue::Enum { value: 1 })
    );
    assert_eq!(spawn.properties.get("formFactorRaw"), None);
    assert_eq!(
        spawn.properties.get("TeamColor"),
        Some(&RbxValue::BrickColor {
            value: BrickColor::MediumStoneGrey,
        })
    );
}

/// Asserts that 'Team.TeamColor' deserializes correctly as a BrickColor.
#[test]
fn team() {
    let _ = env_logger::try_init();

    let tree = rbx_xml::from_reader_default(TEAM).unwrap();
    let root_id = tree.get_root_id();

    let root_instance = tree.get_instance(root_id).unwrap();
    let team_id = root_instance.get_children_ids()[0];
    let team = tree.get_instance(team_id).unwrap();

    assert_eq!(team.name, "Team");
    assert_eq!(team.class_name, "Team");
    assert_eq!(
        team.properties.get("TeamColor"),
        Some(&RbxValue::BrickColor {
            value: BrickColor::LimeGreen,
        })
    );
}

/// Roblox has name collisions for BrickColors!
///
/// This test catches one of them, Lilac and Lilac, but 'Gold', 'Deep
/// orange', and 'Rust' (haha) also collide.
///
/// rbx_dom_weak reserves the enum variant for the version that Roblox picks
/// when `BrickColor.new("Lilac")` is run. The other variant has '2'
/// appended.
#[test]
fn team_lilac() {
    let _ = env_logger::try_init();

    let tree = rbx_xml::from_reader_default(TEAM_LILAC).unwrap();
    let root_id = tree.get_root_id();

    let root_instance = tree.get_instance(root_id).unwrap();
    let team_id = root_instance.get_children_ids()[0];
    let team = tree.get_instance(team_id).unwrap();

    assert_eq!(team.name, "Team");
    assert_eq!(team.class_name, "Team");

    assert_ne!(
        team.properties.get("TeamColor"),
        Some(&RbxValue::BrickColor {
            value: BrickColor::Lilac, // This Lilac has a value of 321
        })
    );
    assert_eq!(
        team.properties.get("TeamColor"),
        Some(&RbxValue::BrickColor {
            value: BrickColor::Lilac2, // This Lilac has a value of 219
        })
    );
}

/// When BrickColor values are deserialized without reflection information, they
/// should turn into regular Int32 values.
#[test]
fn team_no_reflection() {
    let _ = env_logger::try_init();

    let options = rbx_xml::DecodeOptions::new()
        .property_behavior(rbx_xml::DecodePropertyBehavior::NoReflection);

    let tree = rbx_xml::from_reader(TEAM, options).unwrap();
    let root_id = tree.get_root_id();

    let root_instance = tree.get_instance(root_id).unwrap();
    let team_id = root_instance.get_children_ids()[0];
    let team = tree.get_instance(team_id).unwrap();

    assert_eq!(team.name, "Team");
    assert_eq!(team.class_name, "Team");
    assert_eq!(
        team.properties.get("TeamColor"),
        Some(&RbxValue::Int32 { value: 1020 })
    );
}
