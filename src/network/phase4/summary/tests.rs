use super::*;

#[test]
fn lease_remaining_uses_a_visible_day_then_hour_countdown() {
    assert_eq!(lease_remaining(172_800, 0), "2d left");
    assert_eq!(lease_remaining(3_600, 0), "1h left");
    assert_eq!(lease_remaining(0, 0), "0h left");
}

#[test]
fn local_household_status_is_visible_in_the_settlement_summary() {
    let mut client = Phase4Client::new();
    client.households = Some(tarrowyn_protocol::HouseholdsResponse {
        households: vec![tarrowyn_protocol::HouseholdRecord {
            household_id: "bellweather".to_owned(),
            household_name: "The Bellweather household".to_owned(),
            members: Vec::new(),
            home: "The Hearth settlement".to_owned(),
            needs: Vec::new(),
            work: "Milling and healing".to_owned(),
            service_quality: 72,
            demand: 60,
            housing: 70,
            safety: 62,
            food: 68,
            competition: 20,
            status: tarrowyn_protocol::HouseholdLifeStatus::ReducedService,
            clue: "The road needs care.".to_owned(),
            last_decision_tick: 4,
        }],
        cursor: 4,
    });

    assert!(render(&client).lines().next().is_some_and(|line| {
        line.starts_with("Local life reduced service") && line.contains("72%")
    }));
}

#[test]
fn lease_summary_keeps_reclamation_grace_pending_until_the_registry_opens() {
    let mut client = Phase4Client::new();
    client.own_account_id = Some("account-1".to_owned());
    client.claims = Some(tarrowyn_protocol::ClaimsResponse {
        claims: vec![tarrowyn_protocol::ClaimRecord {
            claim_id: "lease-1".to_owned(),
            plot_id: "plot-1".to_owned(),
            owner_account_id: Some("account-1".to_owned()),
            owner_name: Some("Guest".to_owned()),
            position: tarrowyn_protocol::Position { x: 3, y: 4 },
            lease_days: 90,
            started_tick: 1,
            expires_tick: 1,
            started_at_unix_seconds: 0,
            expires_at_unix_seconds: 0,
            last_active_tick: 4,
            status: tarrowyn_protocol::ClaimLifecycleStatus::Abandoned,
            approved_by: None,
            building_access: false,
            protected_goods_policy: "Stored goods remain safe.".to_owned(),
            inspection_note: "Grace is pending.".to_owned(),
        }],
        available_plots: Vec::new(),
        lease_duration_days: 90,
        cursor: 4,
    });

    assert!(render(&client).contains("lease abandoned; grace pending"));
}

#[test]
fn knowledge_summary_names_discovered_records() {
    let mut client = Phase4Client::new();
    client.knowledge = Some(tarrowyn_protocol::KnowledgeResponse {
        request_id: "knowledge-view".to_owned(),
        accepted: true,
        knowledge: tarrowyn_protocol::KnowledgeState {
            items: Vec::new(),
            known_by_player: vec!["moonberry-tending".to_owned()],
            cursor: 4,
        },
        message: "The field notes are open.".to_owned(),
        reason: None,
    });

    assert!(render(&client).contains("1 knowledge record"));
    client
        .knowledge
        .as_mut()
        .expect("knowledge projection")
        .knowledge
        .known_by_player
        .push("route-reading".to_owned());
    assert!(render(&client).contains("2 knowledge records"));
}
