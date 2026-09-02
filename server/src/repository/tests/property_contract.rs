use super::*;
use crate::repository::models::{RepositoryState, StoredState};
use tarrowyn_protocol::{
    AuthLinkRequest, FoundationPropertyAccess, FoundationPropertyAction,
    FoundationPropertyPlacementRule, FoundationPropertyRequest, FoundationPropertyStage,
    FoundationResourceKind,
};

fn request(
    id: &str,
    action: FoundationPropertyAction,
    property_id: Option<&str>,
) -> FoundationPropertyRequest {
    FoundationPropertyRequest {
        request_id: id.to_owned(),
        action,
        property_id: property_id.map(str::to_owned),
        anchor: None,
        entrance: None,
        access: None,
        resource: None,
        amount: 0,
    }
}

fn set_position(repository: &WorldRepository, key: &str, position: Position) {
    repository
        .state
        .lock()
        .unwrap()
        .identities
        .get_mut(key)
        .unwrap()
        .position = position;
}

#[test]
fn storage_version_twenty_seven_defaults_the_property_ledger() {
    let repository = repo();
    let stored = repository.state.lock().unwrap().to_stored();
    let mut json = serde_json::to_value(stored).unwrap();
    json["storage_version"] = serde_json::json!(27);
    json.as_object_mut().unwrap().remove("next_property");
    json.as_object_mut()
        .unwrap()
        .remove("foundation_properties");
    let identity = json["identities"]
        .as_object_mut()
        .unwrap()
        .values_mut()
        .next();
    if let Some(identity) = identity {
        identity
            .as_object_mut()
            .unwrap()
            .remove("foundation_property_results");
    }

    let legacy: StoredState = serde_json::from_value(json).unwrap();
    let restored = RepositoryState::from_stored_at(legacy, &ServerConfig::default(), 0);
    assert_eq!(restored.next_property, 1);
    assert!(restored.foundation_properties.is_empty());
    assert_eq!(restored.to_stored().storage_version, 28);
}

fn place_tent(
    repository: &WorldRepository,
    key: &str,
    token: &str,
    id: &str,
) -> tarrowyn_protocol::FoundationPropertyResponse {
    let mut chosen = None;
    'rows: for y in 1..11 {
        for x in 1..15 {
            set_position(repository, key, Position { x: x - 1, y });
            let mut preview = request(
                &format!("{id}-preview-{x}-{y}"),
                FoundationPropertyAction::PreviewPlacement,
                None,
            );
            preview.anchor = Some(Position { x, y });
            let result = repository.foundation_property(token, preview).unwrap().data;
            if result
                .projection
                .placement_preview
                .as_ref()
                .is_some_and(|value| value.accepted)
            {
                chosen = Some(Position { x, y });
                break 'rows;
            }
        }
    }
    let anchor = chosen.expect("baseline world has room for a personal tent");
    set_position(
        repository,
        key,
        Position {
            x: anchor.x - 1,
            y: anchor.y,
        },
    );
    let mut place = request(id, FoundationPropertyAction::PlaceTent, None);
    place.anchor = Some(anchor);
    repository.foundation_property(token, place).unwrap().data
}

#[test]
fn contract_and_spatial_guards_are_authoritative() {
    let repository = repo();
    let resident = guest(&repository, "property-spatial");
    let projection = repository
        .foundation_properties(&resident.account_token)
        .unwrap()
        .data;
    assert_eq!(projection.contract.stages.len(), 3);
    assert!(projection.own_property.is_none());
    let mut commons = request(
        "property-commons",
        FoundationPropertyAction::PreviewPlacement,
        None,
    );
    commons.anchor = Some(Position { x: 8, y: 6 });
    let commons = repository
        .foundation_property(&resident.account_token, commons)
        .unwrap()
        .data
        .projection
        .placement_preview
        .unwrap();
    assert!(!commons.accepted);
    assert!(commons
        .rejected_rules
        .contains(&FoundationPropertyPlacementRule::OutsideBeaconCommons));

    let placed = place_tent(
        &repository,
        "property-spatial",
        &resident.account_token,
        "property-place",
    );
    assert!(placed.accepted, "{placed:#?}");
    let anchor = placed.projection.own_property.unwrap().anchor;
    let blocked = repository
        .movement(
            &resident.account_token,
            MovementIntent {
                request_id: "property-blocked-move".to_owned(),
                dx: 1,
                dy: 0,
            },
        )
        .unwrap()
        .data;
    assert!(!blocked.accepted);
    assert_ne!(blocked.position, anchor);
}

#[test]
fn actions_are_replay_safe_owner_isolated_and_allow_guest_deposits() {
    let repository = repo();
    let owner = guest(&repository, "property-owner");
    let visitor = guest(&repository, "property-visitor");
    let placed = place_tent(
        &repository,
        "property-owner",
        &owner.account_token,
        "property-owner-place",
    );
    let property = placed.projection.own_property.clone().unwrap();
    let mut replay_request = request(
        "property-owner-place",
        FoundationPropertyAction::PlaceTent,
        None,
    );
    replay_request.anchor = Some(property.anchor);
    assert_eq!(
        repository
            .foundation_property(&owner.account_token, replay_request)
            .unwrap()
            .data,
        placed
    );
    let beside = Position {
        x: property.anchor.x - 1,
        y: property.anchor.y,
    };
    set_position(&repository, "property-owner", beside);
    let mut access = request(
        "property-open",
        FoundationPropertyAction::SetAccess,
        Some(&property.property_id),
    );
    access.access = Some(FoundationPropertyAccess::GuestsAllowed);
    assert!(
        repository
            .foundation_property(&owner.account_token, access)
            .unwrap()
            .data
            .accepted
    );

    set_position(&repository, "property-visitor", beside);
    repository
        .state
        .lock()
        .unwrap()
        .identities
        .get_mut("property-visitor")
        .unwrap()
        .inventory
        .iron_ore = 3;
    let mut store = request(
        "property-store",
        FoundationPropertyAction::Store,
        Some(&property.property_id),
    );
    store.resource = Some(FoundationResourceKind::IronOre);
    store.amount = 2;
    assert!(
        repository
            .foundation_property(&visitor.account_token, store)
            .unwrap()
            .data
            .accepted
    );
    let mut collect = request(
        "property-collect",
        FoundationPropertyAction::Collect,
        Some(&property.property_id),
    );
    collect.resource = Some(FoundationResourceKind::IronOre);
    collect.amount = 1;
    let rejected = repository
        .foundation_property(&visitor.account_token, collect)
        .unwrap()
        .data;
    assert!(!rejected.accepted);
    assert!(rejected.reason.unwrap().contains("Only the owner"));
    assert!(rejected.projection.own_property.is_none());
}

#[test]
fn progression_upkeep_and_integrity_preserve_ownership_and_goods() {
    let repository = repo();
    let owner = guest(&repository, "property-progress");
    let placed = place_tent(
        &repository,
        "property-progress",
        &owner.account_token,
        "property-progress-place",
    );
    let property = placed.projection.own_property.unwrap();
    {
        let mut state = repository.state.lock().unwrap();
        let identity = state.identities.get_mut("property-progress").unwrap();
        identity.inventory.timber = 12;
        identity.inventory.stone = 8;
    }
    let camp = repository
        .foundation_property(
            &owner.account_token,
            request(
                "property-camp",
                FoundationPropertyAction::UpgradeWithMaterials,
                Some(&property.property_id),
            ),
        )
        .unwrap()
        .data;
    assert!(camp.accepted);
    assert_eq!(
        camp.projection.own_property.unwrap().stage,
        FoundationPropertyStage::Camp
    );

    let mara = crate::content::foundation_baseline()
        .landmarks
        .into_iter()
        .find(|landmark| landmark.id == "builder-mara")
        .unwrap()
        .position;
    set_position(&repository, "property-progress", mara);
    {
        let mut state = repository.state.lock().unwrap();
        let identity = state.identities.get_mut("property-progress").unwrap();
        identity.gold = 27;
        identity.inventory.timber = 2;
        identity.inventory.stone = 1;
    }
    let house = repository
        .foundation_property(
            &owner.account_token,
            request(
                "property-house",
                FoundationPropertyAction::HireBuilder,
                Some(&property.property_id),
            ),
        )
        .unwrap()
        .data;
    assert!(house.accepted);
    assert_eq!(house.player.gold, 0);
    assert_eq!(house.player.inventory.timber, 0);
    assert_eq!(house.player.inventory.stone, 0);
    assert_eq!(
        house.projection.own_property.unwrap().stage,
        FoundationPropertyStage::House
    );
    {
        let mut state = repository.state.lock().unwrap();
        let property = state.foundation_properties.first_mut().unwrap();
        property.storage.iron_ore = 2;
        property.last_maintained_unix_millis = property
            .last_maintained_unix_millis
            .saturating_sub(31 * 24 * 60 * 60 * 1_000);
    }
    let weathered = repository
        .foundation_properties(&owner.account_token)
        .unwrap()
        .data
        .own_property
        .unwrap();
    assert_eq!(weathered.condition, 90);
    assert_eq!(weathered.storage.iron_ore, 2);
    assert_eq!(weathered.owner_account_id, owner.account_id);
    assert!(repository.ops_health().data.integrity_ok);
    repository.state.lock().unwrap().foundation_properties[0].condition = 0;
    assert!(!repository.ops_health().data.integrity_ok);
}

#[test]
fn property_survives_restart_and_account_link() {
    let path = std::env::temp_dir().join(format!("tarrowyn-property-{}.json", std::process::id()));
    let config = ServerConfig {
        persistence_path: Some(path.to_string_lossy().into_owned()),
        backup_path: None,
        ..ServerConfig::default()
    };
    let repository = WorldRepository::new(config.clone());
    let owner = guest(&repository, "property-persist");
    let property_id = place_tent(
        &repository,
        "property-persist",
        &owner.account_token,
        "property-persist-place",
    )
    .projection
    .own_property
    .unwrap()
    .property_id;
    drop(repository);
    let restarted = WorldRepository::new(config);
    let resumed = guest(&restarted, "property-persist");
    assert_eq!(
        restarted
            .foundation_properties(&resumed.account_token)
            .unwrap()
            .data
            .own_property
            .unwrap()
            .property_id,
        property_id
    );
    let linked = restarted
        .auth_link(
            &resumed.account_token,
            AuthLinkRequest {
                request_id: "property-link".to_owned(),
                provider: "webhatchery-identity-oidc".to_owned(),
                subject: "property-owner-subject".to_owned(),
                display_name: Some("Property Owner".to_owned()),
            },
        )
        .unwrap()
        .data;
    let linked_property = restarted
        .foundation_properties(&linked.session.account_token)
        .unwrap()
        .data
        .own_property
        .unwrap();
    assert_eq!(linked_property.property_id, property_id);
    assert_eq!(linked_property.owner_account_id, linked.account_id);
    let _ = std::fs::remove_file(path);
}

#[test]
fn resetting_an_unlinked_guest_removes_their_property() {
    let repository = repo();
    let owner = guest(&repository, "property-reset");
    place_tent(
        &repository,
        "property-reset",
        &owner.account_token,
        "property-reset-place",
    );
    repository
        .guest_session(GuestSessionRequest {
            client_key: Some("property-reset".to_owned()),
            reset: true,
        })
        .unwrap();
    assert!(repository
        .state
        .lock()
        .unwrap()
        .foundation_properties
        .is_empty());
}
