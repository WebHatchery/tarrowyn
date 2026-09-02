use super::*;
use tarrowyn_protocol::{FoundationInteraction, Position};

fn baseline() -> FoundationBaseline {
    FoundationBaseline {
        fixture_id: "first-beacon-baseline-v1".to_owned(),
        schema_version: 1,
        settlement_id: "hearth-settlement".to_owned(),
        landmarks: vec![
            FoundationLandmark {
                id: "first-beacon".to_owned(),
                kind: "beacon".to_owned(),
                name: "First Beacon".to_owned(),
                position: Position { x: 8, y: 6 },
                visible: true,
                permanent: true,
                note: "Arrival".to_owned(),
            },
            FoundationLandmark {
                id: "builder-mara".to_owned(),
                kind: "npc".to_owned(),
                name: "Mara the Builder".to_owned(),
                position: Position { x: 7, y: 5 },
                visible: true,
                permanent: true,
                note: "Builder".to_owned(),
            },
        ],
        interactions: vec![
            FoundationInteraction {
                id: "arrive-first-beacon".to_owned(),
                landmark_id: "first-beacon".to_owned(),
                action: "arrive_or_travel".to_owned(),
                authority: "server".to_owned(),
                note: String::new(),
            },
            FoundationInteraction {
                id: "speak-with-builder".to_owned(),
                landmark_id: "builder-mara".to_owned(),
                action: "speak_or_request_construction".to_owned(),
                authority: "server".to_owned(),
                note: String::new(),
            },
        ],
    }
}

#[test]
fn exact_landmark_wins_over_an_adjacent_landmark() {
    let fixture = baseline();
    let context = nearby_context(&fixture, TilePos::new(7, 5)).expect("nearby context");

    assert_eq!(context.landmark.id, "builder-mara");
    assert_eq!(context.action_label, "Talk to Mara");
}

#[test]
fn context_requires_visible_adjacent_landmark() {
    let fixture = baseline();

    assert!(nearby_context(&fixture, TilePos::new(2, 2)).is_none());
}
