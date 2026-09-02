use super::*;
use crate::config::ServerConfig;
use tarrowyn_protocol::{
    CropKind, CropState, FarmingAction, FarmingRequest, FoundationForgeAction,
    FoundationForgeRequest, GuestSessionRequest, Position,
};

fn guest_at_forge(
    repository: &WorldRepository,
    key: &str,
) -> tarrowyn_protocol::GuestSessionResponse {
    let session = repository
        .guest_session(GuestSessionRequest {
            client_key: Some(key.to_owned()),
            reset: false,
        })
        .unwrap()
        .data;
    repository
        .state
        .lock()
        .unwrap()
        .identities
        .get_mut(key)
        .unwrap()
        .position = Position { x: 10, y: 5 };
    session
}

fn use_forge(
    repository: &WorldRepository,
    token: &str,
    request_id: &str,
    action: FoundationForgeAction,
) -> FoundationForgeResponse {
    repository
        .foundation_forge(
            token,
            FoundationForgeRequest {
                request_id: request_id.to_owned(),
                action,
            },
        )
        .unwrap()
        .data
}

#[test]
fn rough_forge_is_proximity_checked_and_missing_costs_are_atomic() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("forge-too-far".to_owned()),
            reset: false,
        })
        .unwrap()
        .data;
    repository
        .state
        .lock()
        .unwrap()
        .identities
        .get_mut("forge-too-far")
        .unwrap()
        .inventory
        .timber = 1;

    let too_far = use_forge(
        &repository,
        &session.account_token,
        "far-charcoal",
        FoundationForgeAction::BurnCharcoal,
    );
    assert!(!too_far.accepted);
    assert_eq!(too_far.player.inventory.timber, 1);
    assert_eq!(too_far.player.inventory.charcoal, 0);

    repository
        .state
        .lock()
        .unwrap()
        .identities
        .get_mut("forge-too-far")
        .unwrap()
        .position = Position { x: 10, y: 5 };
    let missing = use_forge(
        &repository,
        &session.account_token,
        "missing-tool-costs",
        FoundationForgeAction::ForgeFieldTool,
    );
    assert!(!missing.accepted);
    assert_eq!(missing.player.inventory.timber, 1);
    assert_eq!(
        missing.player.field_tool_kind,
        FoundationFieldToolKind::Crude
    );
}

#[test]
fn gathered_inputs_become_an_iron_tool_once_and_survive_restart() {
    let path = std::env::temp_dir().join(format!(
        "tarrowyn-forge-{}-{}.json",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let config = ServerConfig {
        persistence_path: Some(path.to_string_lossy().into_owned()),
        backup_path: None,
        ..ServerConfig::default()
    };
    let repository = WorldRepository::new(config.clone());
    let session = guest_at_forge(&repository, "forge-persistence");
    {
        let mut state = repository.state.lock().unwrap();
        let inventory = &mut state
            .identities
            .get_mut("forge-persistence")
            .unwrap()
            .inventory;
        inventory.timber = 2;
        inventory.iron_ore = 2;
    }

    let inspect = use_forge(
        &repository,
        &session.account_token,
        "inspect-forge",
        FoundationForgeAction::Inspect,
    );
    assert!(inspect.accepted);
    assert_eq!(inspect.forge.recipes.len(), 3);
    assert_eq!(inspect.forge.crude_tool_action_capacity, 3);
    assert_eq!(inspect.forge.improved_tool_action_capacity, 6);

    let charcoal = use_forge(
        &repository,
        &session.account_token,
        "burn-charcoal",
        FoundationForgeAction::BurnCharcoal,
    );
    let charcoal_replay = use_forge(
        &repository,
        &session.account_token,
        "burn-charcoal",
        FoundationForgeAction::BurnCharcoal,
    );
    assert!(charcoal.accepted);
    assert_eq!(charcoal_replay, charcoal);
    assert_eq!(charcoal.player.inventory.timber, 1);
    assert_eq!(charcoal.player.inventory.charcoal, 1);

    let handle = use_forge(
        &repository,
        &session.account_token,
        "shape-handle",
        FoundationForgeAction::ShapeHandle,
    );
    assert!(handle.accepted);
    assert_eq!(handle.player.inventory.timber, 0);
    assert_eq!(handle.player.inventory.tool_handles, 1);

    let forged = use_forge(
        &repository,
        &session.account_token,
        "forge-field-tool",
        FoundationForgeAction::ForgeFieldTool,
    );
    assert!(forged.accepted);
    assert_eq!(forged.player.inventory.iron_ore, 0);
    assert_eq!(forged.player.inventory.charcoal, 0);
    assert_eq!(forged.player.inventory.tool_handles, 0);
    assert_eq!(forged.player.field_tool_kind, FoundationFieldToolKind::Iron);
    assert_eq!(forged.player.field_tool_condition, 6);
    drop(repository);

    let restarted = WorldRepository::new(config);
    let resumed = guest_at_forge(&restarted, "forge-persistence");
    let replay = use_forge(
        &restarted,
        &resumed.account_token,
        "forge-field-tool",
        FoundationForgeAction::ForgeFieldTool,
    );
    assert_eq!(replay, forged);
    let player = restarted.inventory(&resumed.account_token).unwrap().data;
    assert_eq!(player.field_tool_kind, FoundationFieldToolKind::Iron);
    assert_eq!(player.field_tool_condition, 6);
    drop(restarted);
    let _ = std::fs::remove_file(path);
}

fn accepted_tending_actions(
    repository: &WorldRepository,
    token: &str,
    identity_key: &str,
    attempts: u8,
) -> u8 {
    let mut accepted = 0;
    for index in 0..attempts {
        {
            let mut state = repository.state.lock().unwrap();
            let plot = state
                .plots
                .iter_mut()
                .find(|plot| plot.position == Position { x: 10, y: 8 })
                .unwrap();
            plot.crop = Some(CropState {
                kind: CropKind::Wheat,
                stage: 0,
                quality: 1,
                planted_tick: 0,
                growth_ticks: 0,
                last_tended_tick: None,
            });
            state.identities.get_mut(identity_key).unwrap().position = Position { x: 10, y: 8 };
        }
        let response = repository
            .farming(
                token,
                FarmingRequest {
                    request_id: format!("{identity_key}-tend-{index}"),
                    action: FarmingAction::Tend,
                    position: Position { x: 10, y: 8 },
                },
            )
            .unwrap()
            .data;
        accepted += u8::from(response.accepted);
    }
    accepted
}

#[test]
fn iron_field_tool_doubles_useful_actions_without_removing_crude_fallback() {
    let repository = WorldRepository::new(ServerConfig::default());
    let crude = guest_at_forge(&repository, "crude-comparison");
    let iron = guest_at_forge(&repository, "iron-comparison");
    {
        let mut state = repository.state.lock().unwrap();
        let identity = state.identities.get_mut("iron-comparison").unwrap();
        identity.field_tool_kind = FoundationFieldToolKind::Iron;
        identity.field_tool_condition = FoundationFieldToolKind::Iron.max_condition();
    }

    let crude_actions =
        accepted_tending_actions(&repository, &crude.account_token, "crude-comparison", 7);
    let iron_actions =
        accepted_tending_actions(&repository, &iron.account_token, "iron-comparison", 7);

    assert_eq!(crude_actions, 3);
    assert_eq!(iron_actions, 6);
    assert_eq!(iron_actions, crude_actions * 2);
}
