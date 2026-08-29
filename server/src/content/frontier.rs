use macroquad_toolkit::data_loader::parse_json_labeled;
use serde::Deserialize;
use std::sync::OnceLock;
use tarrowyn_protocol::{MonsterKind, Position};

#[derive(Debug, Deserialize)]
struct ContractsManifest {
    contracts: Vec<ContractManifest>,
}

static CONTRACT_CATALOG: OnceLock<Vec<ContractManifest>> = OnceLock::new();

#[derive(Debug, Deserialize)]
struct ContractManifest {
    id: String,
    title: String,
    description: String,
    target: String,
    required_progress: u8,
    reward_gold: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ContractTemplate {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) description: String,
    pub(crate) target: MonsterKind,
    pub(crate) required_progress: u8,
    pub(crate) reward_gold: u32,
}

#[derive(Debug, Deserialize)]
pub(super) struct ThreatsManifest {
    pub(super) threats: Vec<ThreatManifest>,
}

static THREAT_CATALOG: OnceLock<Vec<ThreatManifest>> = OnceLock::new();

#[derive(Debug, Deserialize)]
pub(super) struct ThreatManifest {
    id: String,
    name: String,
    monster: String,
    monster_health: u8,
    pub(super) position: Position,
    price_modifier_percent: i16,
    resource_demand: String,
    rumour: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ThreatTemplate {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) monster: MonsterKind,
    pub(crate) monster_health: u8,
    pub(crate) position: Position,
    pub(crate) price_modifier_percent: i16,
    pub(crate) resource_demand: String,
    pub(crate) rumour: String,
}

pub(super) fn validate(world_width: u32, world_height: u32) -> Result<(), String> {
    let contracts: ContractsManifest = parse_json_labeled(
        "contracts.json",
        macroquad_toolkit::include_json_str!("../../../assets/data/contracts.json"),
    )
    .map_err(|error| format!("contracts JSON is invalid: {error}"))?;
    let threats: ThreatsManifest = parse_json_labeled(
        "threats.json",
        macroquad_toolkit::include_json_str!("../../../assets/data/threats.json"),
    )
    .map_err(|error| format!("threats JSON is invalid: {error}"))?;
    validate_contracts(&contracts)?;
    validate_threats(&threats, world_width, world_height)?;
    Ok(())
}

pub(crate) fn contract_template(contract_id: &str) -> ContractTemplate {
    let contracts = CONTRACT_CATALOG.get_or_init(|| {
        let contracts: ContractsManifest = parse_json_labeled(
            "contracts.json",
            macroquad_toolkit::include_json_str!("../../../assets/data/contracts.json"),
        )
        .expect("contracts content JSON must be valid");
        validate_contracts(&contracts).expect("contracts content must satisfy its schema");
        contracts.contracts
    });
    let contract = contracts
        .iter()
        .find(|contract| contract.id == contract_id)
        .expect("validated contract catalog must contain the requested contract");
    ContractTemplate {
        id: contract.id.clone(),
        title: contract.title.clone(),
        description: contract.description.clone(),
        target: match contract.target.as_str() {
            "brambleback" => MonsterKind::Brambleback,
            _ => panic!("validated contract catalog contains an unsupported target"),
        },
        required_progress: contract.required_progress,
        reward_gold: contract.reward_gold,
    }
}

pub(crate) fn threat_template(threat_id: &str) -> ThreatTemplate {
    let threats = THREAT_CATALOG.get_or_init(|| {
        let threats: ThreatsManifest = parse_json_labeled(
            "threats.json",
            macroquad_toolkit::include_json_str!("../../../assets/data/threats.json"),
        )
        .expect("threats content JSON must be valid");
        let config = super::game_config_defaults();
        validate_threats(&threats, config.world_width, config.world_height)
            .expect("threats content must satisfy its schema");
        threats.threats
    });
    let threat = threats
        .iter()
        .find(|threat| threat.id == threat_id)
        .expect("validated threat catalog must contain the requested threat");
    ThreatTemplate {
        id: threat.id.clone(),
        name: threat.name.clone(),
        monster: match threat.monster.as_str() {
            "brambleback" => MonsterKind::Brambleback,
            _ => panic!("validated threat catalog contains an unsupported monster"),
        },
        monster_health: threat.monster_health,
        position: threat.position,
        price_modifier_percent: threat.price_modifier_percent,
        resource_demand: threat.resource_demand.clone(),
        rumour: threat.rumour.clone(),
    }
}

fn validate_contracts(contracts: &ContractsManifest) -> Result<(), String> {
    super::validate_id_list(
        "contract",
        contracts
            .contracts
            .iter()
            .map(|contract| contract.id.as_str())
            .collect(),
    )?;
    if contracts.contracts.is_empty()
        || contracts.contracts.iter().any(|contract| {
            contract.title.trim().is_empty()
                || contract.description.trim().is_empty()
                || contract.required_progress == 0
                || contract.reward_gold == 0
                || contract.target != "brambleback"
        })
    {
        return Err(
            "contracts need IDs, narrative, a known target, progress, and a positive reward"
                .to_owned(),
        );
    }
    if !contracts
        .contracts
        .iter()
        .any(|contract| contract.id == "brambleback-watch")
    {
        return Err("contracts are missing the launch Brambleback watch".to_owned());
    }
    Ok(())
}

pub(super) fn validate_threats(
    threats: &ThreatsManifest,
    world_width: u32,
    world_height: u32,
) -> Result<(), String> {
    super::validate_id_list(
        "threat",
        threats
            .threats
            .iter()
            .map(|threat| threat.id.as_str())
            .collect(),
    )?;
    if threats.threats.is_empty()
        || threats.threats.iter().any(|threat| {
            threat.name.trim().is_empty()
                || threat.monster != "brambleback"
                || threat.monster_health == 0
                || threat.position.x < 0
                || threat.position.y < 0
                || threat.position.x as u32 >= world_width
                || threat.position.y as u32 >= world_height
                || threat.price_modifier_percent < 0
                || threat.resource_demand.trim().is_empty()
                || threat.rumour.trim().is_empty()
        })
    {
        return Err(
            "threats need IDs, names, bounded positions, a known monster, health, pricing, demand, and rumours"
                .to_owned(),
        );
    }
    if !threats
        .threats
        .iter()
        .any(|threat| threat.id == "whisperwood-edge")
    {
        return Err("threats are missing the launch Whisperwood Edge threat".to_owned());
    }
    Ok(())
}
