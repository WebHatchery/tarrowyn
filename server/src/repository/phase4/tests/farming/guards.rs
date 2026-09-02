use super::*;

#[test]
fn farming_rewards_saturate_player_counters_at_the_numeric_ceiling() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = guest(&repository, "farming-counter-ceiling");
    let plot_position = crate::content::farm_plot_positions()[0];
    {
        let mut state = repository.state.lock().unwrap();
        let identity = state.identities.get_mut("farming-counter-ceiling").unwrap();
        identity.position = plot_position;
        identity.inventory.wheat = u32::MAX;
        identity.gold = u32::MAX;
        identity.skill = u32::MAX;
        state
            .plots
            .iter_mut()
            .find(|plot| plot.position == plot_position)
            .unwrap()
            .crop = Some(CropState {
            kind: CropKind::Wheat,
            stage: CropState::MATURE_STAGE,
            quality: 1,
            planted_tick: 0,
            growth_ticks: 0,
            last_tended_tick: None,
        });
    }

    let response = repository
        .farming(
            &session.account_token,
            FarmingRequest {
                request_id: "farming-counter-ceiling-harvest".to_owned(),
                action: FarmingAction::Harvest,
                position: plot_position,
            },
        )
        .unwrap()
        .data;
    assert!(response.accepted);
    assert_eq!(response.player.inventory.wheat, u32::MAX);
    assert_eq!(response.player.gold, u32::MAX);
    assert_eq!(response.player.skill, u32::MAX);
}

#[test]
fn knocked_out_player_cannot_change_shared_fields_before_recovery() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = guest(&repository, "farming-knockout-boundary");
    let plot_position = crate::content::farm_plot_positions()[0];
    let seeds_before;
    let skill_before;
    {
        let mut state = repository.state.lock().expect("repository lock");
        let identity = state
            .identities
            .get_mut(&session.client_key)
            .expect("guest identity");
        identity.position = plot_position;
        identity.knocked_out = true;
        identity.injuries = 1;
        seeds_before = identity.inventory.seeds;
        skill_before = identity.skill;
    }

    let response = repository
        .farming(
            &session.account_token,
            FarmingRequest {
                request_id: "farming-knockout-plant".to_owned(),
                action: FarmingAction::Plant,
                position: plot_position,
            },
        )
        .expect("knocked-out farming response")
        .data;

    assert!(!response.accepted);
    assert!(response.player.knocked_out);
    assert!(response
        .reason
        .as_deref()
        .is_some_and(|reason| reason.contains("tap Self, Rescuer, or Healer")));
    assert_eq!(response.player.inventory.seeds, seeds_before);
    assert_eq!(response.player.skill, skill_before);
    assert!(repository
        .state(&session.account_token)
        .unwrap()
        .data
        .world
        .plots
        .iter()
        .find(|plot| plot.position == plot_position)
        .unwrap()
        .crop
        .is_none());
}

#[test]
fn crop_tending_rejects_a_same_beat_burst_without_spending_tool_condition() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = guest(&repository, "farming-same-beat");
    let plot_position = crate::content::farm_plot_positions()[0];
    {
        let mut state = repository.state.lock().expect("repository lock");
        state
            .identities
            .get_mut(&session.client_key)
            .expect("identity")
            .position = plot_position;
        state
            .plots
            .iter_mut()
            .find(|plot| plot.position == plot_position)
            .expect("plot")
            .crop = Some(CropState {
            kind: CropKind::Wheat,
            stage: 0,
            quality: 1,
            planted_tick: 0,
            growth_ticks: 0,
            last_tended_tick: None,
        });
    }

    let request = |request_id: &str| FarmingRequest {
        request_id: request_id.to_owned(),
        action: FarmingAction::Tend,
        position: plot_position,
    };
    let first = repository.farming(&session.account_token, request("farming-first-tend"));
    assert!(first.expect("first tending").data.accepted);
    let rejected = repository
        .farming(&session.account_token, request("farming-second-tend"))
        .expect("same-beat tending response")
        .data;
    assert!(!rejected.accepted);
    assert!(rejected
        .reason
        .as_deref()
        .is_some_and(|reason| reason.contains("already been tended")));
    assert_eq!(rejected.player.field_tool_condition, 2);
    assert_eq!(
        rejected
            .plot
            .expect("plot response")
            .crop
            .expect("crop response")
            .stage,
        1
    );

    repository.tick();
    let next_beat = repository
        .farming(&session.account_token, request("farming-next-tend"))
        .expect("next-beat tending response")
        .data;
    assert!(next_beat.accepted);
    assert_eq!(next_beat.player.field_tool_condition, 1);
}
