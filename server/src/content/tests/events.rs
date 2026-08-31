use crate::content::{region_catalog, validate_events, EventsManifest};

#[test]
fn event_interventions_must_have_an_implemented_effect() {
    let mut events: EventsManifest = macroquad_toolkit::data_loader::parse_json_labeled(
        "events.json",
        macroquad_toolkit::include_json_str!("../../../../assets/data/events.json"),
    )
    .expect("checked-in events content should parse");
    events
        .events
        .first_mut()
        .expect("the events manifest should have a launch record")
        .intervention_options
        .push("invent a silent response".to_owned());

    let error = validate_events(&events, region_catalog())
        .expect_err("an event choice without a server effect must fail validation");
    assert!(error.contains("supported interventions"));
}

#[test]
fn event_affected_systems_must_not_contain_blank_entries() {
    let mut events: EventsManifest = macroquad_toolkit::data_loader::parse_json_labeled(
        "events.json",
        macroquad_toolkit::include_json_str!("../../../../assets/data/events.json"),
    )
    .expect("checked-in events content should parse");
    events
        .events
        .first_mut()
        .expect("the events manifest should have a launch record")
        .affected_systems
        .push("  ".to_owned());

    let error = validate_events(&events, region_catalog())
        .expect_err("blank affected systems must fail validation");
    assert!(error.contains("affected systems"));
}

#[test]
fn event_interventions_must_include_their_effect_location() {
    let mut events: EventsManifest = macroquad_toolkit::data_loader::parse_json_labeled(
        "events.json",
        macroquad_toolkit::include_json_str!("../../../../assets/data/events.json"),
    )
    .expect("checked-in events content should parse");
    events
        .events
        .first_mut()
        .expect("the events manifest should have a launch record")
        .affected_locations = vec!["saltmere".to_owned()];

    let error = validate_events(&events, region_catalog())
        .expect_err("an intervention must include its target location");
    assert!(error.contains("affected location"));
}
