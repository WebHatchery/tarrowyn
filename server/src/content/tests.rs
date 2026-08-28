#[test]
fn authoritative_manifests_satisfy_the_content_contract() {
    super::validate().expect("checked-in content should satisfy the server schema");
}

#[test]
fn content_ids_must_be_unique_and_non_empty() {
    assert!(super::validate_id_list("test", vec!["one", "two"]).is_ok());
    assert!(super::validate_id_list("test", vec!["one", "one"]).is_err());
    assert!(super::validate_id_list("test", vec!["one", ""]).is_err());
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
    assert_eq!(profiles.len(), 4);
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
}
