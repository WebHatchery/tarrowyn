use super::*;

#[test]
fn property_contract_has_monotonic_spatial_shelter_and_storage_progression() {
    let contract = FoundationPropertyContract::default();

    assert_eq!(contract.contract_id, "first-beacon-personal-property-v1");
    assert_eq!(contract.stages.len(), 3);
    assert_eq!(contract.stages[0].stage, FoundationPropertyStage::Tent);
    assert_eq!(contract.stages[1].stage, FoundationPropertyStage::Camp);
    assert_eq!(contract.stages[2].stage, FoundationPropertyStage::House);
    assert_eq!(contract.stages[0].footprint.width, 1);
    assert_eq!(contract.stages[1].footprint.width, 2);
    assert_eq!(contract.stages[2].footprint.width, 3);
    assert_eq!(
        contract
            .stages
            .iter()
            .map(|stage| stage.storage_capacity)
            .collect::<Vec<_>>(),
        vec![8, 24, 48]
    );
    assert!(contract.stages[0].material_costs.is_empty());
    assert_eq!(contract.stages[1].builder_gold_cost, 14);
    assert_eq!(contract.stages[2].builder_gold_cost, 34);
}

#[test]
fn property_contract_names_every_critical_space_and_escape_guard() {
    let placement = FoundationPropertyContract::default().placement;

    assert_eq!(placement.beacon_commons_radius, 3);
    assert_eq!(placement.entrance_clearance_tiles, 1);
    assert_eq!(placement.minimum_escape_routes, 1);
    assert_eq!(placement.maximum_properties_per_owner, 1);
    for rule in [
        FoundationPropertyPlacementRule::InsideWorld,
        FoundationPropertyPlacementRule::ClearTerrain,
        FoundationPropertyPlacementRule::NoStructureOverlap,
        FoundationPropertyPlacementRule::OutsideBeaconCommons,
        FoundationPropertyPlacementRule::OutsideProtectedRoute,
        FoundationPropertyPlacementRule::EntranceClear,
        FoundationPropertyPlacementRule::EscapePathOpen,
    ] {
        assert!(placement.rules.contains(&rule));
    }
}

#[test]
fn access_and_upkeep_never_transfer_ownership_or_delete_goods() {
    let contract = FoundationPropertyContract::default();

    assert_eq!(
        contract.access.default_access,
        FoundationPropertyAccess::OwnerOnly
    );
    assert!(contract.access.owner_may_change_access);
    assert!(contract.access.guests_may_inspect);
    assert!(contract.access.guests_may_store);
    assert!(!contract.access.guests_may_collect);
    assert!(!contract.access.guests_may_upgrade);
    assert_eq!(contract.upkeep.minimum_condition, 1);
    assert!(!contract.upkeep.ownership_changes_from_upkeep);
    assert!(!contract.upkeep.stored_goods_lost_from_upkeep);
    assert!(contract.builder.substitutes_only_missing_materials);
    assert!(contract.builder.player_building_remains_available);
}

#[test]
fn typed_property_request_and_preview_round_trip_without_registry_assumptions() {
    let request = FoundationPropertyRequest {
        request_id: "property-preview-1".to_owned(),
        action: FoundationPropertyAction::PreviewPlacement,
        property_id: None,
        anchor: Some(Position { x: 14, y: 7 }),
        entrance: Some(FoundationPropertyDirection::South),
        access: None,
        resource: None,
        amount: 0,
    };
    let encoded = serde_json::to_string(&request).unwrap();
    let decoded: FoundationPropertyRequest = serde_json::from_str(&encoded).unwrap();

    assert_eq!(decoded, request);
    assert!(!encoded.contains("claim"));
    assert!(!encoded.contains("lease"));

    let preview = FoundationPropertyPlacementPreview {
        anchor: Position { x: 8, y: 6 },
        entrance: FoundationPropertyDirection::South,
        footprint: FoundationPropertyFootprint {
            width: 1,
            height: 1,
        },
        accepted: false,
        rejected_rules: vec![FoundationPropertyPlacementRule::OutsideBeaconCommons],
        message: "The Beacon commons must remain public.".to_owned(),
    };
    let encoded = serde_json::to_string(&preview).unwrap();
    let decoded: FoundationPropertyPlacementPreview = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded, preview);
}
