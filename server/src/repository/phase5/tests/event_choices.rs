use super::super::super::WorldRepository;
use super::guest;
use crate::ServerConfig;
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
