use super::*;

#[test]
fn cursor_recovery_clears_phase4_projections_and_local_work() {
    let mut client = Phase4Client::new();
    client.skills = Some(SkillsResponse {
        skills: Vec::new(),
        lessons: Vec::new(),
        cursor: 99,
    });
    client.crafting = Some(CraftingChallenge {
        order_id: "stale-order".to_owned(),
        progress: 0.5,
        direction: 1.0,
        target_start: 0.4,
        target_end: 0.6,
    });

    client.recover_cursor_boundary();

    assert!(client.skills.is_none());
    assert!(client.crafting.is_none());
}

#[test]
fn older_phase_four_projection_cannot_replace_newer_command_state() {
    let mut cursor = 0;

    assert!(super::super::polling::accept_projection_cursor(
        &mut cursor,
        Some(12)
    ));
    assert!(!super::super::polling::accept_projection_cursor(
        &mut cursor,
        Some(11)
    ));
    assert!(super::super::polling::accept_projection_cursor(
        &mut cursor,
        Some(12)
    ));
}

#[test]
fn older_same_cursor_phase_four_command_cannot_replace_newer_ledger() {
    let mut client = Phase4Client::new();
    let current = ClaimsResponse {
        claims: vec![claim_for_test(
            "current-lease",
            Some("account-1"),
            tarrowyn_protocol::ClaimLifecycleStatus::Active,
        )],
        available_plots: Vec::new(),
        lease_duration_days: 90,
        cursor: 12,
    };
    client.claims = Some(current.clone());

    let data = crate::data::GameData::load().expect("embedded game data should load");
    let mut projection = WorldProjection::new(&data.config);
    projection.server_tick = 8;
    projection.cursor = 12;
    let stale = ClaimsResponse {
        claims: vec![claim_for_test(
            "stale-lease",
            Some("account-2"),
            tarrowyn_protocol::ClaimLifecycleStatus::Requested,
        )],
        available_plots: Vec::new(),
        lease_duration_days: 90,
        cursor: 12,
    };
    let response = Phase4CommandResponse::Claim(tarrowyn_protocol::ClaimLifecycleResponse {
        request_id: "stale-command".to_owned(),
        accepted: true,
        claim: None,
        claims: stale,
        reason: None,
    });
    let mut notices = Vec::new();
    let projection_current = projection.accept_response_version(7, Some(12));

    client.apply_command(response, Some(12), projection_current, &mut notices);

    assert_eq!(client.claims, Some(current));
    assert_eq!(notices.len(), 1);
}
