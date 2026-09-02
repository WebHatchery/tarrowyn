use super::*;
use tarrowyn_protocol::{
    FoundationCacheAction, FoundationFieldToolKind, FoundationForgeAction,
    FoundationResourceAction, FoundationResourceAmount, FoundationResourceKind,
    FoundationStorehouseAction, FoundationStorehouseContributionInput,
};

#[test]
fn nearby_resource_queue_uses_one_non_blocking_authoritative_command() {
    let mut client = OnlineClient::new("http://127.0.0.1:8787", &config());
    client.state = ConnectionState::Online;
    client.state_reload_pending = false;

    assert!(
        client.queue_foundation_resource("whisperwood-edge-node", FoundationResourceAction::Log)
    );
    let pending = client.pending_foundation_resource.as_ref().unwrap();
    assert_eq!(pending.request.node_id, "whisperwood-edge-node");
    assert_eq!(pending.request.action, FoundationResourceAction::Log);
    assert!(!client
        .queue_foundation_resource("shallow-stone-seam-node", FoundationResourceAction::Mine));
}

#[test]
fn forge_queue_keeps_one_typed_request_for_safe_retries() {
    let mut client = OnlineClient::new("http://127.0.0.1:8787", &config());
    client.state = ConnectionState::Online;
    client.state_reload_pending = false;

    assert!(client.queue_foundation_forge(FoundationForgeAction::BurnCharcoal));
    let pending = client.pending_foundation_forge.as_ref().unwrap();
    assert_eq!(pending.request.action, FoundationForgeAction::BurnCharcoal);
    assert!(pending.request.request_id.starts_with("foundation-forge-"));
    assert!(!client.queue_foundation_forge(FoundationForgeAction::Inspect));
}

#[test]
fn forge_feedback_projects_materials_and_measured_tool_capacity() {
    let player = tarrowyn_protocol::PlayerProjection {
        account_id: "forge-account".to_owned(),
        character_id: "forge-character".to_owned(),
        display_name: "Smith".to_owned(),
        position: tarrowyn_protocol::Position { x: 8, y: 6 },
        gold: 0,
        field_tool_condition: 6,
        field_tool_kind: FoundationFieldToolKind::Iron,
        field_weather: Default::default(),
        field_pest_pressure: 0,
        animal_condition: 10,
        animal_max_condition: 10,
        skill: 1,
        reputation: 0,
        adventurer_rank: Default::default(),
        adventurer_credentials: Vec::new(),
        inventory: tarrowyn_protocol::Inventory {
            timber: 1,
            charcoal: 1,
            tool_handles: 1,
            ..Default::default()
        },
        weapon: tarrowyn_protocol::WeaponKind::IronSword,
        knocked_out: false,
        injuries: 0,
        recovery_cost: 0,
    };

    let notice = super::super::foundation::foundation_forge_success_notice(
        FoundationForgeAction::ForgeFieldTool,
        &player,
    );
    assert!(notice.contains("Forged an iron field tool"));
    assert!(notice.contains("1 timber, 0 iron ore, 1 charcoal, 1 handles"));
    assert!(notice.contains("iron field tool 6/6"));
}

#[test]
fn shared_cache_queue_keeps_one_typed_request_for_safe_retries() {
    let mut client = OnlineClient::new("http://127.0.0.1:8787", &config());
    client.state = ConnectionState::Online;
    client.state_reload_pending = false;

    assert!(client.queue_foundation_cache(
        FoundationCacheAction::Deposit,
        Some(FoundationResourceKind::Stone)
    ));
    let pending = client.pending_foundation_cache.as_ref().unwrap();
    assert_eq!(pending.request.action, FoundationCacheAction::Deposit);
    assert_eq!(
        pending.request.resource,
        Some(FoundationResourceKind::Stone)
    );
    assert_eq!(pending.request.amount, 1);
    assert!(pending.request.request_id.starts_with("foundation-cache-"));
    assert!(!client.queue_foundation_cache(FoundationCacheAction::Inspect, None));
}

#[test]
fn gathering_notice_names_every_authoritative_yield() {
    let notice = super::super::foundation::foundation_resource_success_notice(&[
        FoundationResourceAmount {
            kind: FoundationResourceKind::Stone,
            amount: 2,
        },
        FoundationResourceAmount {
            kind: FoundationResourceKind::IronOre,
            amount: 1,
        },
    ]);

    assert_eq!(
        notice,
        "Gathered 2 stone and 1 iron ore with the shared crude tools."
    );
}

#[test]
fn cache_feedback_names_the_authoritative_transfer() {
    assert_eq!(
        super::super::foundation::foundation_cache_success_notice(
            FoundationCacheAction::Withdraw,
            Some(FoundationResourceKind::IronOre)
        ),
        "Collected 1 iron ore from the shared cache."
    );
}

#[test]
fn storehouse_queue_keeps_one_typed_landmark_contribution_for_retry() {
    let mut client = OnlineClient::new("http://127.0.0.1:8787", &config());
    client.state = ConnectionState::Online;
    client.state_reload_pending = false;

    assert!(client.queue_foundation_storehouse(
        "storehouse-site",
        Some(FoundationStorehouseContributionInput::Material {
            kind: FoundationResourceKind::Timber,
            amount: 1,
        })
    ));
    let pending = client.pending_foundation_storehouse.as_ref().unwrap();
    assert_eq!(
        pending.request.action,
        FoundationStorehouseAction::Contribute
    );
    assert_eq!(pending.request.landmark_id, "storehouse-site");
    assert!(pending
        .request
        .request_id
        .starts_with("foundation-storehouse-"));
    assert!(!client.queue_foundation_storehouse("builder-mara", None));
}

#[test]
fn storehouse_feedback_names_stage_remaining_need_and_completion() {
    let project = tarrowyn_protocol::FoundationStorehouseState::default();
    let progress = super::super::foundation::foundation_storehouse_success_notice(
        &project,
        FoundationStorehouseAction::Inspect,
    );
    assert!(progress.contains("Marked storehouse site"));
    assert!(progress.contains("8 timber and 6 stone remain"));

    let mut complete = project;
    complete.current_stage = tarrowyn_protocol::FoundationStorehouseStage::Operational;
    complete.completion = Some(tarrowyn_protocol::FoundationStorehouseCompletion {
        completed_tick: 4,
        contributor_account_ids: vec!["account-1".to_owned()],
        operational_infrastructure_id: "first-beacon-storehouse".to_owned(),
    });
    assert!(
        super::super::foundation::foundation_storehouse_success_notice(
            &complete,
            FoundationStorehouseAction::Contribute,
        )
        .contains("permanently recorded")
    );
}
