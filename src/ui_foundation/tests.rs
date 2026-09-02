use super::*;
use tarrowyn_protocol::{
    FoundationActivityState, FoundationInteraction, FoundationResourceDeposit,
    FoundationResourceKind, FoundationResourceNode, Position,
};

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
    let activity = FoundationActivityState::default();
    let context = nearby_context(&fixture, &activity, TilePos::new(7, 5)).expect("nearby context");

    assert_eq!(context.landmark.id, "builder-mara");
    assert_eq!(context.action_label, "Talk to Mara");
}

#[test]
fn context_requires_visible_adjacent_landmark() {
    let fixture = baseline();

    assert!(nearby_context(
        &fixture,
        &FoundationActivityState::default(),
        TilePos::new(2, 2)
    )
    .is_none());
}

#[test]
fn nearby_woodland_becomes_a_productive_resource_command() {
    let mut fixture = baseline();
    fixture.landmarks.push(FoundationLandmark {
        id: "whisperwood-edge".to_owned(),
        kind: "woodland".to_owned(),
        name: "Whisperwood edge".to_owned(),
        position: Position { x: 13, y: 3 },
        visible: true,
        permanent: false,
        note: "Nearby timber".to_owned(),
    });
    fixture.interactions.push(FoundationInteraction {
        id: "work-whisperwood-edge".to_owned(),
        landmark_id: "whisperwood-edge".to_owned(),
        action: "log".to_owned(),
        authority: "server".to_owned(),
        note: String::new(),
    });
    let activity = FoundationActivityState {
        resource_nodes: vec![FoundationResourceNode {
            node_id: "whisperwood-edge-node".to_owned(),
            landmark_id: "whisperwood-edge".to_owned(),
            deposits: vec![FoundationResourceDeposit {
                kind: FoundationResourceKind::Timber,
                remaining: 12,
                capacity: 12,
            }],
            recovery_interval_ticks: 6,
            last_recovered_tick: 0,
        }],
        crude_tool_access: Vec::new(),
        shared_cache: tarrowyn_protocol::FoundationSharedCache::default(),
    };

    let context = nearby_context(&fixture, &activity, TilePos::new(12, 3)).unwrap();

    assert_eq!(context.action_label, "Gather timber");
    assert_eq!(context.resource_node_id, Some("whisperwood-edge-node"));
    assert_eq!(context.resource_action, Some(FoundationResourceAction::Log));
}
