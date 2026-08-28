use serde::Deserialize;
use std::sync::OnceLock;
use tarrowyn_protocol::{HouseholdLifeStatus, HouseholdMemberRecord, HouseholdRecord};

#[derive(Debug, Deserialize)]
struct NpcHouseholdsManifest {
    npc_households: Vec<NpcHouseholdManifest>,
}

static NPC_HOUSEHOLD_CATALOG: OnceLock<Vec<NpcHouseholdManifest>> = OnceLock::new();

#[derive(Debug, Deserialize)]
struct NpcHouseholdManifest {
    id: String,
    household_id: String,
    household_name: String,
    members: Vec<NpcHouseholdMemberManifest>,
    home: String,
    needs: Vec<String>,
    work: String,
    service_quality: u8,
    demand: u8,
    housing: u8,
    safety: u8,
    food: u8,
    competition: u8,
    status: String,
    clue: String,
}

#[derive(Debug, Deserialize)]
struct NpcHouseholdMemberManifest {
    name: String,
    role: String,
    service: String,
}

pub(super) fn validate() -> Result<(), String> {
    let households: NpcHouseholdsManifest =
        serde_json::from_str(include_str!("../../../assets/data/npc_households.json"))
            .map_err(|error| format!("NPC households JSON is invalid: {error}"))?;
    validate_manifest(&households)
}

pub(crate) fn household(household_id: &str) -> HouseholdRecord {
    let households = NPC_HOUSEHOLD_CATALOG.get_or_init(|| {
        let households: NpcHouseholdsManifest =
            serde_json::from_str(include_str!("../../../assets/data/npc_households.json"))
                .expect("NPC households content JSON must be valid");
        validate_manifest(&households).expect("NPC household content must satisfy its schema");
        households.npc_households
    });
    let household = households
        .iter()
        .find(|household| household.id == household_id)
        .expect("validated NPC household catalog must contain the requested household");
    HouseholdRecord {
        household_id: household.household_id.clone(),
        household_name: household.household_name.clone(),
        members: household
            .members
            .iter()
            .map(|member| HouseholdMemberRecord {
                name: member.name.clone(),
                role: member.role.clone(),
                service: member.service.clone(),
            })
            .collect(),
        home: household.home.clone(),
        needs: household.needs.clone(),
        work: household.work.clone(),
        service_quality: household.service_quality,
        demand: household.demand,
        housing: household.housing,
        safety: household.safety,
        food: household.food,
        competition: household.competition,
        status: match household.status.as_str() {
            "arrived" => HouseholdLifeStatus::Arrived,
            "reduced_service" => HouseholdLifeStatus::ReducedService,
            "considering_departure" => HouseholdLifeStatus::ConsideringDeparture,
            "departed" => HouseholdLifeStatus::Departed,
            _ => panic!("validated NPC household catalog contains an unsupported status"),
        },
        clue: household.clue.clone(),
        last_decision_tick: 0,
    }
}

fn validate_manifest(households: &NpcHouseholdsManifest) -> Result<(), String> {
    super::validate_id_list(
        "NPC household",
        households
            .npc_households
            .iter()
            .map(|household| household.id.as_str())
            .collect(),
    )?;
    if households.npc_households.is_empty()
        || households.npc_households.iter().any(|household| {
            household.household_id.trim().is_empty()
                || household.household_name.trim().is_empty()
                || household.members.is_empty()
                || household.members.iter().any(|member| {
                    member.name.trim().is_empty()
                        || member.role.trim().is_empty()
                        || member.service.trim().is_empty()
                })
                || household.home.trim().is_empty()
                || household.needs.is_empty()
                || household.needs.iter().any(|need| need.trim().is_empty())
                || household.work.trim().is_empty()
                || household.service_quality > 100
                || household.demand > 100
                || household.housing > 100
                || household.safety > 100
                || household.food > 100
                || household.competition > 100
                || !matches!(
                    household.status.as_str(),
                    "arrived" | "reduced_service" | "considering_departure" | "departed"
                )
                || household.clue.trim().is_empty()
        })
    {
        return Err(
            "NPC households need members, service data, bounded conditions, and a lifecycle status"
                .to_owned(),
        );
    }
    if !households
        .npc_households
        .iter()
        .any(|household| household.id == "bellweather")
    {
        return Err("NPC households are missing the launch Bellweather household".to_owned());
    }
    Ok(())
}
