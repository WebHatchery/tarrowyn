use super::super::super::ServerConfig;
use super::super::super::WorldRepository;
use tarrowyn_protocol::{
    GuestSessionRequest, SupportRepairAction, SupportRepairRequest, TravelAction, TravelRequest,
};

#[test]
fn support_repair_clears_stuck_travel_at_recorded_origin() {
    let repository = WorldRepository::new(ServerConfig {
        support_operator_accounts: vec!["dev-account-1".to_owned()],
        ..ServerConfig::default()
    });
    let operator = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("repair-travel-operator".to_owned()),
            reset: false,
        })
        .unwrap()
        .data;
    let northbound = repository
        .travel(
            &operator.account_token,
            TravelRequest {
                request_id: "repair-travel-northbound".to_owned(),
                action: TravelAction::Start,
                route_id: Some("north-pack-road".to_owned()),
                travel_id: None,
            },
        )
        .unwrap()
        .data;
    assert!(northbound.accepted);
    for _ in 0..6 {
        repository.tick();
    }
    let outbound = repository
        .travel(
            &operator.account_token,
            TravelRequest {
                request_id: "repair-travel-outbound".to_owned(),
                action: TravelAction::Start,
                route_id: Some("watch-trail".to_owned()),
                travel_id: None,
            },
        )
        .unwrap()
        .data;
    assert!(outbound.accepted);

    let repair_request = SupportRepairRequest {
        request_id: "repair-stuck-travel".to_owned(),
        action: SupportRepairAction::ClearStuckTravel,
        account_id: Some(operator.account_id.clone()),
        target_id: None,
        note: "Return the stuck scout to the journey's recorded origin.".to_owned(),
    };
    let repaired = repository
        .support_repair(&operator.account_token, repair_request.clone())
        .unwrap()
        .data;
    assert!(repaired.accepted);
    {
        let state = repository.state.lock().unwrap();
        assert_ne!(
            state.identities["repair-travel-operator"].position,
            crate::content::region_location_profile("hearth").position
        );
        assert_eq!(
            state.identities["repair-travel-operator"].position,
            crate::content::region_location_profile("whisperwood-outpost").position
        );
        assert!(!state.phase5.travel.contains_key("repair-travel-operator"));
        assert_eq!(state.phase5.cursor, state.cursor);
    }
    assert_eq!(
        repository
            .support_repair(&operator.account_token, repair_request)
            .unwrap()
            .data,
        repaired
    );
    let already_clear = repository
        .support_repair(
            &operator.account_token,
            SupportRepairRequest {
                request_id: "repair-stuck-travel-empty".to_owned(),
                action: SupportRepairAction::ClearStuckTravel,
                account_id: Some(operator.account_id),
                target_id: None,
                note: "Confirm an already cleared journey is not silently accepted.".to_owned(),
            },
        )
        .unwrap()
        .data;
    assert!(!already_clear.accepted);
    assert!(already_clear
        .reason
        .as_deref()
        .is_some_and(|reason| reason.contains("no recorded journey")));
}
