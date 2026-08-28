use crate::config::ServerConfig;
use crate::repository::WorldRepository;
use std::time::Duration;
use tarrowyn_protocol::{
    ClaimLifecycleAction, ClaimLifecycleRequest, CommodityKind, GovernanceAction,
    GovernanceRequest, GuestSessionRequest, MarketOrderAction, MarketOrderRequest,
    RegionalEventAction, RegionalEventRequest, SettlementCondition,
};

fn guest(repository: &WorldRepository, key: &str) -> tarrowyn_protocol::GuestSessionResponse {
    repository
        .guest_session(GuestSessionRequest {
            client_key: Some(key.to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data
}

#[test]
fn long_session_crosses_calendar_and_keeps_world_accessible() {
    let repository = WorldRepository::new(ServerConfig {
        day_length_seconds: 1.0,
        world_seconds_per_tick: 1.0,
        tick_interval: Duration::from_millis(1),
        household_decision_interval_ticks: 1,
        backup_path: None,
        ..ServerConfig::default()
    });
    let resident = guest(&repository, "long-session-resident");

    let office = repository
        .governance(
            &resident.account_token,
            GovernanceRequest {
                request_id: "long-session-office".to_owned(),
                action: GovernanceAction::ClaimOffice,
                office_id: Some("steward".to_owned()),
                proposal_id: None,
                public_action: None,
                target: None,
                cost: None,
                tax_rate_percent: None,
            },
        )
        .unwrap()
        .data;
    assert!(office.accepted);
    let tax = repository
        .governance(
            &resident.account_token,
            GovernanceRequest {
                request_id: "long-session-tax".to_owned(),
                action: GovernanceAction::SetTaxRate,
                office_id: None,
                proposal_id: None,
                public_action: None,
                target: None,
                cost: None,
                tax_rate_percent: Some(10),
            },
        )
        .unwrap()
        .data;
    assert!(tax.accepted);

    let requested = repository
        .claim_lifecycle(
            &resident.account_token,
            ClaimLifecycleRequest {
                request_id: "long-session-lease-request".to_owned(),
                action: ClaimLifecycleAction::Request,
                claim_id: None,
                target_account_id: None,
            },
        )
        .unwrap()
        .data;
    let claim_id = requested.claim.unwrap().claim_id;
    let approved = repository
        .claim_lifecycle(
            &resident.account_token,
            ClaimLifecycleRequest {
                request_id: "long-session-lease-approve".to_owned(),
                action: ClaimLifecycleAction::Approve,
                claim_id: Some(claim_id.clone()),
                target_account_id: None,
            },
        )
        .unwrap()
        .data;
    assert!(approved.accepted);
    assert_eq!(approved.claim.unwrap().lease_days, 90);

    let order = repository
        .market_order(
            &resident.account_token,
            MarketOrderRequest {
                request_id: "long-session-market".to_owned(),
                action: MarketOrderAction::Create,
                order_id: None,
                destination_location_id: Some("saltmere".to_owned()),
                commodity: Some(CommodityKind::Seeds),
                quantity: Some(1),
            },
        )
        .unwrap()
        .data;
    assert!(order.accepted);

    for (expected, ticks) in [
        ("thaw", 14),
        ("greenrise", 14),
        ("harvest", 14),
        ("deepwinter", 14),
    ] {
        let region = repository.region(&resident.account_token).unwrap().data;
        assert_eq!(region.season, expected);
        for _ in 0..ticks {
            repository.tick();
        }
    }
    assert_eq!(
        repository
            .region(&resident.account_token)
            .unwrap()
            .data
            .season,
        "thaw"
    );

    let order = repository.market(&resident.account_token).unwrap().data;
    assert!(order
        .orders
        .iter()
        .any(|order| order.status == tarrowyn_protocol::MarketOrderStatus::Failed));
    let claims = repository.claims(&resident.account_token).unwrap().data;
    let lease = claims
        .claims
        .iter()
        .find(|claim| claim.claim_id == claim_id)
        .expect("long-session lease should remain visible");
    assert_eq!(
        lease.status,
        tarrowyn_protocol::ClaimLifecycleStatus::Active
    );
    assert_eq!(claims.lease_duration_days, 90);

    let households = repository
        .households_region(&resident.account_token)
        .unwrap()
        .data;
    assert_eq!(households.households[0].status, "arrived");
    assert!(households.households[0].history.len() >= 3);

    let event = repository
        .event_action(
            &resident.account_token,
            RegionalEventRequest {
                request_id: "long-session-event-seed".to_owned(),
                action: RegionalEventAction::Seed,
                event_id: None,
                intervention: None,
            },
        )
        .unwrap()
        .data;
    let event_id = event.event.unwrap().event_id;
    assert!(
        repository
            .event_action(
                &resident.account_token,
                RegionalEventRequest {
                    request_id: "long-session-event-intervene".to_owned(),
                    action: RegionalEventAction::Intervene,
                    event_id: Some(event_id.clone()),
                    intervention: Some("repair ferry markers".to_owned()),
                },
            )
            .unwrap()
            .data
            .accepted
    );
    assert!(
        repository
            .event_action(
                &resident.account_token,
                RegionalEventRequest {
                    request_id: "long-session-event-resolve".to_owned(),
                    action: RegionalEventAction::Resolve,
                    event_id: Some(event_id.clone()),
                    intervention: None,
                },
            )
            .unwrap()
            .data
            .accepted
    );
    for _ in 0..7 {
        repository.tick();
    }
    let events = repository
        .events_region(&resident.account_token, 0)
        .unwrap()
        .data;
    assert!(events.events.iter().any(|event| event.event_id == event_id
        && event.stage == tarrowyn_protocol::RegionalEventStage::Aftermath));
    assert!(repository
        .chronicle(&resident.account_token, 0)
        .unwrap()
        .data
        .entries
        .iter()
        .any(|entry| entry.kind.contains("regional event")));

    let governance = repository
        .governance(
            &resident.account_token,
            GovernanceRequest {
                request_id: "long-session-inspect".to_owned(),
                action: GovernanceAction::Inspect,
                office_id: None,
                proposal_id: None,
                public_action: None,
                target: None,
                cost: None,
                tax_rate_percent: None,
            },
        )
        .unwrap()
        .data;
    assert!(!governance.governance.tax_ledger.is_empty());
    assert!(
        governance
            .governance
            .tax_ledger
            .iter()
            .map(|collection| collection.amount)
            .sum::<u32>()
            > 0
    );
    assert!(governance.governance.public_treasury < 48);

    let newcomer = guest(&repository, "long-session-newcomer");
    let newcomer_projection = repository.inventory(&newcomer.account_token).unwrap().data;
    assert!(newcomer_projection.inventory.seeds > 0);
    let newcomer_region = repository.region(&newcomer.account_token).unwrap().data;
    assert!(!newcomer_region.locations.is_empty());
    let settlements = repository
        .settlements(&newcomer.account_token)
        .unwrap()
        .data;
    assert!(settlements.settlements.iter().any(|settlement| {
        !settlement.vacancies.is_empty() && settlement.condition != SettlementCondition::Quiet
    }));

    let state = repository.state.lock().unwrap();
    assert!(state.phase3.chronicle.len() <= 64);
    assert!(state.phase4.governance.tax_ledger.len() <= 64);
}
