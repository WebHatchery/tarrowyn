use macroquad_toolkit::data_loader::parse_json_labeled;
use serde::Deserialize;
use std::collections::HashSet;
use std::sync::OnceLock;
use tarrowyn_protocol::{InfrastructureKind, SettlementCondition};

#[derive(Debug, Deserialize)]
struct SettlementsManifest {
    settlements: Vec<SettlementManifest>,
}

static SETTLEMENT_CATALOG: OnceLock<Vec<SettlementManifest>> = OnceLock::new();

#[derive(Debug, Deserialize)]
struct SettlementManifest {
    id: String,
    location: String,
    name: String,
    population: u32,
    food: u8,
    safety: u8,
    infrastructure: u8,
    industry: u8,
    governance: u8,
    player_activity: u8,
    condition: String,
    milestones: Vec<String>,
    vacancies: Vec<String>,
    demand: Vec<String>,
    abundant: Vec<String>,
    scarce: Vec<String>,
    price_index_percent: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SettlementProfile {
    pub(crate) name: String,
    pub(crate) location: String,
    pub(crate) population: u32,
    pub(crate) food: u8,
    pub(crate) safety: u8,
    pub(crate) infrastructure: u8,
    pub(crate) industry: u8,
    pub(crate) governance: u8,
    pub(crate) player_activity: u8,
    pub(crate) condition: SettlementCondition,
    pub(crate) milestones: Vec<String>,
    pub(crate) vacancies: Vec<String>,
    pub(crate) demand: Vec<String>,
    pub(crate) abundant: Vec<String>,
    pub(crate) scarce: Vec<String>,
    pub(crate) price_index_percent: u16,
}

#[derive(Debug, Deserialize)]
struct InfrastructureManifest {
    infrastructure: Vec<InfrastructureRecordManifest>,
}

static INFRASTRUCTURE_CATALOG: OnceLock<Vec<InfrastructureRecordManifest>> = OnceLock::new();

#[derive(Debug, Deserialize)]
struct InfrastructureRecordManifest {
    id: String,
    name: String,
    kind: String,
    position: super::Position,
    condition: u8,
    upkeep_per_day: u32,
    service_quality: u8,
    note: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InfrastructureProfile {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) kind: InfrastructureKind,
    pub(crate) position: super::Position,
    pub(crate) condition: u8,
    pub(crate) upkeep_per_day: u32,
    pub(crate) service_quality: u8,
    pub(crate) note: String,
}

pub(super) fn validate(region: &super::RegionManifest) -> Result<(), String> {
    let settlements: SettlementsManifest = parse_json_labeled(
        "settlements.json",
        macroquad_toolkit::include_json_str!("../../../assets/data/settlements.json"),
    )
    .map_err(|error| format!("settlements JSON is invalid: {error}"))?;
    let infrastructure: InfrastructureManifest = parse_json_labeled(
        "infrastructure.json",
        macroquad_toolkit::include_json_str!("../../../assets/data/infrastructure.json"),
    )
    .map_err(|error| format!("infrastructure JSON is invalid: {error}"))?;
    validate_settlements(&settlements, region)?;
    validate_infrastructure(&infrastructure)?;
    Ok(())
}

pub(crate) fn settlement_profile(settlement_id: &str) -> SettlementProfile {
    let settlements = SETTLEMENT_CATALOG.get_or_init(|| {
        let settlements: SettlementsManifest = parse_json_labeled(
            "settlements.json",
            macroquad_toolkit::include_json_str!("../../../assets/data/settlements.json"),
        )
        .expect("settlements content JSON must be valid");
        validate_settlements(&settlements, super::region_catalog())
            .expect("settlements content must satisfy its schema");
        settlements.settlements
    });
    let settlement = settlements
        .iter()
        .find(|settlement| settlement.id == settlement_id)
        .expect("validated settlement catalog must contain the requested settlement");
    SettlementProfile {
        name: settlement.name.clone(),
        location: settlement.location.clone(),
        population: settlement.population,
        food: settlement.food,
        safety: settlement.safety,
        infrastructure: settlement.infrastructure,
        industry: settlement.industry,
        governance: settlement.governance,
        player_activity: settlement.player_activity,
        condition: settlement_condition(&settlement.condition),
        milestones: settlement.milestones.clone(),
        vacancies: settlement.vacancies.clone(),
        demand: settlement.demand.clone(),
        abundant: settlement.abundant.clone(),
        scarce: settlement.scarce.clone(),
        price_index_percent: settlement.price_index_percent,
    }
}

pub(crate) fn infrastructure_profiles() -> Vec<InfrastructureProfile> {
    let infrastructure = INFRASTRUCTURE_CATALOG.get_or_init(|| {
        let infrastructure: InfrastructureManifest = parse_json_labeled(
            "infrastructure.json",
            macroquad_toolkit::include_json_str!("../../../assets/data/infrastructure.json"),
        )
        .expect("infrastructure content JSON must be valid");
        validate_infrastructure(&infrastructure)
            .expect("infrastructure content must satisfy its schema");
        infrastructure.infrastructure
    });
    infrastructure
        .iter()
        .map(|record| InfrastructureProfile {
            id: record.id.clone(),
            name: record.name.clone(),
            kind: infrastructure_kind(&record.kind),
            position: record.position,
            condition: record.condition,
            upkeep_per_day: record.upkeep_per_day,
            service_quality: record.service_quality,
            note: record.note.clone(),
        })
        .collect()
}

fn settlement_condition(condition: &str) -> SettlementCondition {
    match condition {
        "flourishing" => SettlementCondition::Flourishing,
        "stable" => SettlementCondition::Stable,
        "strained" => SettlementCondition::Strained,
        "quiet" => SettlementCondition::Quiet,
        "recovering" => SettlementCondition::Recovering,
        _ => panic!("validated settlement catalog contains an unsupported condition"),
    }
}

fn infrastructure_kind(kind: &str) -> InfrastructureKind {
    match kind {
        "road" => InfrastructureKind::Road,
        "bridge" => InfrastructureKind::Bridge,
        "plot" => InfrastructureKind::Plot,
        "public_building" => InfrastructureKind::PublicBuilding,
        "service" => InfrastructureKind::Service,
        _ => panic!("validated infrastructure catalog contains an unsupported kind"),
    }
}

fn validate_infrastructure(infrastructure: &InfrastructureManifest) -> Result<(), String> {
    super::validate_id_list(
        "infrastructure",
        infrastructure
            .infrastructure
            .iter()
            .map(|record| record.id.as_str())
            .collect(),
    )?;
    if infrastructure.infrastructure.is_empty()
        || infrastructure.infrastructure.iter().any(|record| {
            record.name.trim().is_empty()
                || !matches!(
                    record.kind.as_str(),
                    "road" | "bridge" | "plot" | "public_building" | "service"
                )
                || record.condition > 100
                || record.upkeep_per_day == 0
                || record.service_quality > 100
                || record.note.trim().is_empty()
        })
    {
        return Err(
            "infrastructure needs unique IDs, known kinds, bounded condition, upkeep, quality, and notes"
                .to_owned(),
        );
    }
    Ok(())
}

fn validate_settlements(
    settlements: &SettlementsManifest,
    region: &super::RegionManifest,
) -> Result<(), String> {
    super::validate_id_list(
        "settlement",
        settlements
            .settlements
            .iter()
            .map(|settlement| settlement.id.as_str())
            .collect(),
    )?;
    let location_ids: HashSet<&str> = region
        .locations
        .iter()
        .map(|location| location.id.as_str())
        .collect();
    if settlements.settlements.is_empty()
        || settlements.settlements.iter().any(|settlement| {
            settlement.location.trim().is_empty()
                || settlement.name.trim().is_empty()
                || settlement.population == 0
                || settlement.food > 100
                || settlement.safety > 100
                || settlement.infrastructure > 100
                || settlement.industry > 100
                || settlement.governance > 100
                || settlement.player_activity > 100
                || !matches!(
                    settlement.condition.as_str(),
                    "flourishing" | "stable" | "strained" | "quiet" | "recovering"
                )
                || settlement.milestones.is_empty()
                || settlement
                    .milestones
                    .iter()
                    .any(|milestone| milestone.trim().is_empty())
                || settlement.vacancies.is_empty()
                || settlement
                    .vacancies
                    .iter()
                    .any(|vacancy| vacancy.trim().is_empty())
                || settlement.demand.is_empty()
                || settlement
                    .demand
                    .iter()
                    .any(|demand| demand.trim().is_empty())
                || settlement.abundant.is_empty()
                || settlement
                    .abundant
                    .iter()
                    .any(|good| good.trim().is_empty())
                || settlement.scarce.is_empty()
                || settlement.scarce.iter().any(|good| good.trim().is_empty())
                || settlement.price_index_percent == 0
                || !location_ids.contains(settlement.location.as_str())
        })
    {
        return Err(
            "settlements need complete conditions, opportunities, known locations, and supply notes"
                .to_owned(),
        );
    }
    Ok(())
}
