use super::*;

#[test]
fn foundation_baseline_round_trips_with_stable_records() {
    let baseline = FoundationBaseline {
        fixture_id: "first-beacon-baseline-v1".to_owned(),
        schema_version: 1,
        settlement_id: "hearth-settlement".to_owned(),
        landmarks: vec![FoundationLandmark {
            id: "first-beacon".to_owned(),
            kind: "beacon".to_owned(),
            name: "First Beacon".to_owned(),
            position: Position { x: 8, y: 6 },
            visible: true,
            permanent: true,
            note: "The world's permanent arrival point.".to_owned(),
        }],
        interactions: vec![FoundationInteraction {
            id: "arrive-first-beacon".to_owned(),
            landmark_id: "first-beacon".to_owned(),
            action: "arrive".to_owned(),
            authority: "server".to_owned(),
            note: "Every fresh character arrives here.".to_owned(),
        }],
    };

    let encoded = serde_json::to_string(&baseline).expect("baseline should serialize");
    let decoded: FoundationBaseline =
        serde_json::from_str(&encoded).expect("baseline should deserialize");

    assert_eq!(decoded, baseline);
    assert_eq!(decoded.landmarks[0].id, "first-beacon");
    assert_eq!(decoded.interactions[0].landmark_id, "first-beacon");
}

#[test]
fn foundation_interaction_response_round_trips() {
    let response = FoundationInteractionResponse {
        request_id: "foundation-1".to_owned(),
        interaction_id: "speak-with-builder".to_owned(),
        landmark_id: "builder-mara".to_owned(),
        accepted: true,
        title: "Mara the Builder".to_owned(),
        message: "The camp needs a storehouse.".to_owned(),
    };

    let encoded = serde_json::to_string(&response).expect("response should serialize");
    let decoded: FoundationInteractionResponse =
        serde_json::from_str(&encoded).expect("response should deserialize");

    assert_eq!(decoded, response);
}

#[test]
fn foundation_resource_contract_round_trips_depletion_and_crude_access() {
    let activity = FoundationActivityState {
        resource_nodes: vec![FoundationResourceNode {
            node_id: "whisperwood-edge-node".to_owned(),
            landmark_id: "whisperwood-edge".to_owned(),
            deposits: vec![FoundationResourceDeposit {
                kind: FoundationResourceKind::Timber,
                remaining: 7,
                capacity: 12,
            }],
            recovery_interval_ticks: 6,
            last_recovered_tick: 18,
        }],
        crude_tool_access: vec![FoundationToolAccess {
            landmark_id: "first-beacon-tool-rack".to_owned(),
            tools: vec![
                FoundationCrudeToolKind::HandAxe,
                FoundationCrudeToolKind::StonePick,
            ],
            available_to_all: true,
        }],
    };
    let request = FoundationResourceRequest {
        request_id: "gather-1".to_owned(),
        node_id: "whisperwood-edge-node".to_owned(),
        action: FoundationResourceAction::Log,
    };

    let encoded = serde_json::to_string(&(activity.clone(), request.clone())).unwrap();
    let decoded: (FoundationActivityState, FoundationResourceRequest) =
        serde_json::from_str(&encoded).unwrap();

    assert_eq!(decoded, (activity, request));
    assert!(encoded.contains("\"kind\":\"timber\""));
    assert!(encoded.contains("\"action\":\"log\""));
}
