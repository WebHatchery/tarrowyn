use super::super::super::WorldRepository;
use super::guest;
use crate::ServerConfig;
use std::collections::HashMap;
use tarrowyn_protocol::{RegionalEventAction, RegionalEventRequest};

#[test]
fn alternate_event_choice_changes_supply_and_resolution_text() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = guest(&repository, "phase5-event-choice");
    let seeded = repository
        .event_action(
            &session.account_token,
            RegionalEventRequest {
                request_id: "choice-seed".to_owned(),
                action: RegionalEventAction::Seed,
                event_id: None,
                intervention: None,
            },
        )
        .unwrap()
        .data;
    let event_id = seeded.event.unwrap().event_id;
    for _ in 0..3 {
        repository.tick();
    }

    let intervention = repository
        .event_action(
            &session.account_token,
            RegionalEventRequest {
                request_id: "choice-intervene".to_owned(),
                action: RegionalEventAction::Intervene,
                event_id: Some(event_id.clone()),
                intervention: Some("open the frontier storehouse".to_owned()),
            },
        )
        .unwrap()
        .data;
    assert!(intervention.accepted);
    let state = repository.state.lock().expect("repository lock");
    assert_eq!(
        state.phase5.stock.get("whisperwood-outpost:seeds"),
        Some(&4)
    );
    drop(state);

    let resolved = repository
        .event_action(
            &session.account_token,
            RegionalEventRequest {
                request_id: "choice-resolve".to_owned(),
                action: RegionalEventAction::Resolve,
                event_id: Some(event_id),
                intervention: None,
            },
        )
        .unwrap()
        .data;
    assert!(resolved
        .event
        .and_then(|event| event.outcome)
        .is_some_and(|outcome| outcome.contains("frontier storehouse")));
}

#[test]
fn regional_event_effects_follow_their_affected_locations() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = guest(&repository, "phase5-local-event-scope");
    let seeded = repository
        .event_action(
            &session.account_token,
            RegionalEventRequest {
                request_id: "local-scope-seed".to_owned(),
                action: RegionalEventAction::Seed,
                event_id: None,
                intervention: None,
            },
        )
        .expect("event seed")
        .data;
    let event_id = seeded.event.expect("seeded event").event_id;
    {
        let mut state = repository.state.lock().expect("repository state");
        state.phase5.events[0].affected_location_ids = vec!["hearth".to_owned()];
        state.tick = 2;
        super::super::logic::advance_events(&mut state);
        let food_by_location: HashMap<_, _> = state
            .phase5
            .settlements
            .iter()
            .map(|settlement| (settlement.location_id.as_str(), settlement.food))
            .collect();
        assert_eq!(food_by_location.get("hearth"), Some(&68));
        assert_eq!(food_by_location.get("saltmere"), Some(&61));
        assert_eq!(
            state
                .phase4
                .households
                .first()
                .expect("Bellweather household")
                .service_quality,
            68
        );

        let (accepted, _, reason) = super::super::logic::intervene_event(
            &mut state,
            Some(&event_id),
            Some("escort the grain caravan"),
        );
        assert!(accepted, "localized intervention failed: {reason:?}");
        let food_by_location: HashMap<_, _> = state
            .phase5
            .settlements
            .iter()
            .map(|settlement| (settlement.location_id.as_str(), settlement.food))
            .collect();
        assert_eq!(food_by_location.get("hearth"), Some(&74));
        assert_eq!(food_by_location.get("saltmere"), Some(&61));

        let (accepted, _, reason) = super::super::logic::resolve_event(&mut state, Some(&event_id));
        assert!(accepted, "localized resolution failed: {reason:?}");
        let safety_by_location: HashMap<_, _> = state
            .phase5
            .settlements
            .iter()
            .map(|settlement| (settlement.location_id.as_str(), settlement.safety))
            .collect();
        assert_eq!(safety_by_location.get("hearth"), Some(&74));
        assert_eq!(safety_by_location.get("saltmere"), Some(&76));
    }
}
