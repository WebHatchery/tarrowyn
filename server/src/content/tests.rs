#[test]
fn authoritative_manifests_satisfy_the_content_contract() {
    super::validate().expect("checked-in content should satisfy the server schema");
}

#[test]
fn starting_skill_comes_from_the_shared_game_config_manifest() {
    assert_eq!(super::starting_skill(), 1);
}

#[test]
fn content_ids_must_be_unique_and_non_empty() {
    assert!(super::validate_id_list("test", vec!["one", "two"]).is_ok());
    assert!(super::validate_id_list("test", vec!["one", "one"]).is_err());
    assert!(super::validate_id_list("test", vec!["one", ""]).is_err());
}

#[test]
fn action_kinds_must_match_the_supported_protocol_actions() {
    let mut actions: Vec<super::ActionManifest> =
        macroquad_toolkit::data_loader::parse_json_labeled(
            "actions.json",
            macroquad_toolkit::include_json_str!("../../../assets/data/actions.json"),
        )
        .expect("checked-in actions content should parse");
    actions
        .first_mut()
        .expect("the actions manifest should have a launch record")
        .kind = "unmapped_action".to_owned();

    let error = super::validate_actions(&actions)
        .expect_err("an action kind without a protocol action must fail validation");
    assert!(error.contains("supported kinds"));
}

#[test]
fn event_interventions_must_have_an_implemented_effect() {
    let mut events: super::EventsManifest = macroquad_toolkit::data_loader::parse_json_labeled(
        "events.json",
        macroquad_toolkit::include_json_str!("../../../assets/data/events.json"),
    )
    .expect("checked-in events content should parse");
    events
        .events
        .first_mut()
        .expect("the events manifest should have a launch record")
        .intervention_options
        .push("invent a silent response".to_owned());

    let error = super::validate_events(&events, super::region_catalog())
        .expect_err("an event choice without a server effect must fail validation");
    assert!(error.contains("supported interventions"));
}

#[test]
fn launch_content_ids_are_required_by_the_runtime_contract() {
    let available = std::collections::HashSet::from(["hearth", "saltmere"]);
    assert!(super::validate_required_ids("location", &available, &["hearth"]).is_ok());
    let error =
        super::validate_required_ids("location", &available, &["hearth", "whisperwood-outpost"])
            .expect_err("a missing launch location must fail content validation");
    assert!(error.contains("whisperwood-outpost"));
}

#[test]
fn launch_route_topology_cannot_drift_from_runtime_contract() {
    let mut region: super::RegionManifest = macroquad_toolkit::data_loader::parse_json_labeled(
        "region.json",
        macroquad_toolkit::include_json_str!("../../../assets/data/region.json"),
    )
    .expect("checked-in region content should parse");
    region
        .routes
        .iter_mut()
        .find(|route| route.id == "north-pack-road")
        .expect("the launch road should exist")
        .origin = "saltmere".to_owned();
    let game_config: super::GameConfigManifest =
        macroquad_toolkit::data_loader::parse_json_labeled(
            "game_config.json",
            macroquad_toolkit::include_json_str!("../../../assets/data/game_config.json"),
        )
        .expect("checked-in game config should parse");
    let error = super::validate_region(&region, &game_config)
        .expect_err("a launch route with the wrong endpoint must fail validation");
    assert!(error.contains("north-pack-road"));
    assert!(error.contains("hearth"));
}

#[test]
fn region_locations_must_stay_inside_the_configured_world() {
    let mut region: super::RegionManifest = macroquad_toolkit::data_loader::parse_json_labeled(
        "region.json",
        macroquad_toolkit::include_json_str!("../../../assets/data/region.json"),
    )
    .expect("checked-in region content should parse");
    region
        .locations
        .first_mut()
        .expect("the region should have a launch location")
        .position = tarrowyn_protocol::Position { x: 99, y: 99 };
    let game_config: super::GameConfigManifest =
        macroquad_toolkit::data_loader::parse_json_labeled(
            "game_config.json",
            macroquad_toolkit::include_json_str!("../../../assets/data/game_config.json"),
        )
        .expect("checked-in game config should parse");

    let error = super::validate_region(&region, &game_config)
        .expect_err("an off-map location must fail validation");
    assert!(error.contains("inside the world"));
}

#[test]
fn infrastructure_positions_must_stay_inside_the_configured_world() {
    let mut infrastructure: super::settlements::InfrastructureManifest =
        macroquad_toolkit::data_loader::parse_json_labeled(
            "infrastructure.json",
            macroquad_toolkit::include_json_str!("../../../assets/data/infrastructure.json"),
        )
        .expect("checked-in infrastructure content should parse");
    infrastructure
        .infrastructure
        .first_mut()
        .expect("the infrastructure manifest should have a launch record")
        .position = tarrowyn_protocol::Position { x: 99, y: 99 };

    let error = super::settlements::validate_infrastructure(&infrastructure, 18, 12)
        .expect_err("an off-map infrastructure record must fail validation");
    assert!(error.contains("bounded"));
}

#[test]
fn server_crop_rotation_follows_the_validated_manifest() {
    assert_eq!(
        super::crop_kind_for_seed(0),
        tarrowyn_protocol::CropKind::Wheat
    );
    assert_eq!(
        super::crop_kind_for_seed(1),
        tarrowyn_protocol::CropKind::Turnip
    );
    assert_eq!(
        super::crop_kind_for_seed(2),
        tarrowyn_protocol::CropKind::Moonberry
    );
    assert_eq!(
        super::crop_kind_for_seed(3),
        tarrowyn_protocol::CropKind::Wheat
    );
}

#[test]
fn server_event_seed_template_follows_the_validated_manifest() {
    let event = super::regional_event_template(0);
    assert_eq!(event.id, "river-thaw");
    assert_eq!(event.kind, "seasonal_supply");
    assert_eq!(event.title, "The river thaw carries a warning");
    assert_eq!(
        event.affected_locations,
        vec![
            "hearth".to_owned(),
            "whisperwood-outpost".to_owned(),
            "saltmere".to_owned()
        ]
    );
    assert_eq!(event.effects.len(), 4);
    assert_eq!(event.intervention_options[0], "repair ferry markers");
    assert_eq!(super::regional_event_template(1).id, "river-thaw");
}

#[test]
fn settlement_profiles_follow_the_validated_manifest() {
    let hearth = super::settlement_profile("hearth-settlement");
    assert_eq!(hearth.name, "The Hearth");
    assert_eq!(hearth.population, 36);
    assert_eq!(
        hearth.condition,
        tarrowyn_protocol::SettlementCondition::Stable
    );
    assert_eq!(hearth.vacancies[0], "caravan quartermaster");
    assert_eq!(hearth.price_index_percent, 108);
    assert_eq!(
        hearth.abundant,
        vec!["wheat".to_owned(), "seeds".to_owned()]
    );
    assert_eq!(
        hearth.scarce,
        vec!["timber".to_owned(), "bandages".to_owned()]
    );
    assert_eq!(
        hearth.initial_stock,
        vec![
            super::settlements::SettlementStock {
                commodity: "timber".to_owned(),
                quantity: 4,
            },
            super::settlements::SettlementStock {
                commodity: "stone".to_owned(),
                quantity: 6,
            },
        ]
    );
}

#[test]
fn market_prices_follow_the_validated_item_manifest() {
    assert_eq!(super::item_base_price("seeds"), 2);
    assert_eq!(super::item_base_price("moonberries"), 6);
    assert_eq!(super::item_base_price("bandages"), 7);
}

#[test]
fn season_labels_follow_the_validated_region_calendar() {
    assert_eq!(super::season_for_day(1), "thaw");
    assert_eq!(super::season_for_day(15), "greenrise");
    assert_eq!(super::season_for_day(29), "harvest");
    assert_eq!(super::season_for_day(43), "deepwinter");
    assert_eq!(super::season_for_day(57), "thaw");
}

#[test]
fn region_identity_follows_the_validated_region_manifest() {
    assert_eq!(super::region_id(), "hearthlands");
}

#[test]
fn farm_plot_positions_follow_the_validated_region_manifest() {
    assert_eq!(
        super::farm_plot_positions(),
        vec![
            tarrowyn_protocol::Position { x: 2, y: 8 },
            tarrowyn_protocol::Position { x: 2, y: 9 },
            tarrowyn_protocol::Position { x: 10, y: 8 },
        ]
    );
}

#[test]
fn farm_animal_position_follows_the_validated_region_manifest() {
    assert_eq!(
        super::farm_animal_position(),
        tarrowyn_protocol::Position { x: 3, y: 8 }
    );
    assert!(!super::farm_plot_positions().contains(&super::farm_animal_position()));
}

#[test]
fn regional_bootstrap_ids_follow_the_validated_catalogs() {
    assert_eq!(
        super::region_location_ids(),
        vec![
            "hearth".to_owned(),
            "whisperwood-outpost".to_owned(),
            "saltmere".to_owned()
        ]
    );
    assert_eq!(
        super::region_route_ids(),
        vec![
            "north-pack-road".to_owned(),
            "saltmere-ferry".to_owned(),
            "watch-trail".to_owned()
        ]
    );
    assert_eq!(
        super::settlement_ids(),
        vec![
            "hearth-settlement".to_owned(),
            "whisperwood-settlement".to_owned(),
            "saltmere-settlement".to_owned()
        ]
    );
}

#[test]
fn route_profiles_follow_the_validated_region_manifest() {
    let ferry = super::region_route_profile("saltmere-ferry");
    assert_eq!(ferry.name, "Saltmere ferry");
    assert_eq!(ferry.transport, "boat");
    assert_eq!(ferry.origin, "hearth");
    assert_eq!(ferry.destination, "saltmere");
    assert_eq!(ferry.length, 7);
    assert_eq!(ferry.risk_percent, 12);
    assert_eq!(ferry.status, tarrowyn_protocol::RouteStatus::Operational);
}

#[test]
fn location_profiles_follow_the_validated_region_manifest() {
    let hearth = super::region_location_profile("hearth");
    assert_eq!(hearth.position, tarrowyn_protocol::Position { x: 8, y: 6 });

    let watch = super::region_location_profile("whisperwood-outpost");
    assert_eq!(watch.name, "Whisperwood Watch");
    assert_eq!(watch.kind, tarrowyn_protocol::LocationKind::Outpost);
    assert_eq!(watch.position, tarrowyn_protocol::Position { x: 12, y: 4 });
    assert_eq!(watch.role, "frontier");
    assert_eq!(watch.condition, 38);
    assert_eq!(watch.services[0], "scout vacancy");
    assert_eq!(
        watch.resources,
        vec!["timber".to_owned(), "iron salvage".to_owned()]
    );
}

#[test]
fn contract_templates_follow_the_validated_manifest() {
    let contract = super::contract_template("brambleback-watch");
    assert_eq!(contract.title, "Brambleback watch");
    assert_eq!(contract.target, tarrowyn_protocol::MonsterKind::Brambleback);
    assert_eq!(contract.required_progress, 3);
    assert_eq!(contract.reward_gold, 6);
}

#[test]
fn threat_templates_follow_the_validated_manifest() {
    let threat = super::threat_template("whisperwood-edge");
    assert_eq!(threat.name, "Whisperwood Edge");
    assert_eq!(threat.monster, tarrowyn_protocol::MonsterKind::Brambleback);
    assert_eq!(threat.monster_health, 3);
    assert_eq!(threat.position, tarrowyn_protocol::Position { x: 12, y: 4 });
    assert_eq!(threat.price_modifier_percent, 20);
}

#[test]
fn threat_positions_must_stay_inside_the_configured_world() {
    let mut threats: super::frontier::ThreatsManifest =
        macroquad_toolkit::data_loader::parse_json_labeled(
            "threats.json",
            macroquad_toolkit::include_json_str!("../../../assets/data/threats.json"),
        )
        .expect("checked-in threats content should parse");
    threats
        .threats
        .first_mut()
        .expect("the threats manifest should have a launch record")
        .position = tarrowyn_protocol::Position { x: 99, y: 99 };

    let error = super::frontier::validate_threats(&threats, 18, 12)
        .expect_err("an off-map threat must fail validation");
    assert!(error.contains("bounded positions"));
}

#[test]
fn household_templates_follow_the_validated_manifest() {
    let opportunity = super::opportunity_template("household-maren");
    assert_eq!(opportunity.household_id, "household-maren");
    assert_eq!(opportunity.members.len(), 2);
    assert_eq!(
        opportunity.status,
        tarrowyn_protocol::HouseholdStatus::Travelling
    );
    assert_eq!(opportunity.opportunity_score, 48);

    let regional = super::regional_household_template("household-maren");
    assert_eq!(regional.household_id, "household-maren-region");
    assert_eq!(regional.origin_location_id, "saltmere");
    assert_eq!(regional.destination_location_id.as_deref(), Some("hearth"));
    assert_eq!(regional.status, "considering");
}

#[test]
fn infrastructure_profiles_follow_the_validated_manifest() {
    let profiles = super::infrastructure_profiles();
    assert_eq!(profiles.len(), 6);
    assert_eq!(profiles[0].id, "north-road");
    assert_eq!(
        profiles[0].kind,
        tarrowyn_protocol::InfrastructureKind::Road
    );
    assert_eq!(
        profiles[0].position,
        tarrowyn_protocol::Position { x: 11, y: 6 }
    );
    assert_eq!(profiles[0].condition, 72);
    assert_eq!(profiles[0].upkeep_per_day, 2);
    assert_eq!(profiles[0].service_quality, 58);
    assert!(profiles.iter().any(|profile| {
        profile.id == "whisperwood-watchtower"
            && profile.kind == tarrowyn_protocol::InfrastructureKind::PublicBuilding
    }));
    assert!(profiles.iter().any(|profile| {
        profile.id == "saltmere-quay"
            && profile.kind == tarrowyn_protocol::InfrastructureKind::Service
    }));
}

#[test]
fn fixed_npc_households_follow_the_validated_manifest() {
    let household = super::npc_household("bellweather");
    assert_eq!(household.household_id, "household-bellweather");
    assert_eq!(household.members.len(), 2);
    assert_eq!(household.members[0].role, "miller");
    assert_eq!(
        household.status,
        tarrowyn_protocol::HouseholdLifeStatus::Arrived
    );
    assert_eq!(household.service_quality, 72);
    assert_eq!(household.demand, 60);
}

#[test]
fn recipes_follow_the_validated_manifest() {
    let recipe = super::recipe_template("field-tool-repair");
    assert_eq!(recipe.name, "Field tool repair");
    assert_eq!(
        recipe.profession,
        tarrowyn_protocol::ProfessionKind::Carpenter
    );
    assert_eq!(recipe.materials.wood, 1);
    assert_eq!(recipe.materials.iron, 1);
    assert_eq!(recipe.tools_required, 1);
    assert_eq!(recipe.reward_gold, 5);
}
