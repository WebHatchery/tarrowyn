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
