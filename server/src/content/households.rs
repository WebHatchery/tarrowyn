use macroquad_toolkit::data_loader::parse_json_labeled;
use serde::Deserialize;
use std::sync::OnceLock;
use tarrowyn_protocol::{HouseholdMember, HouseholdStatus};

#[derive(Debug, Deserialize)]
struct HouseholdsManifest {
    households: Vec<HouseholdManifest>,
}

static HOUSEHOLD_CATALOG: OnceLock<Vec<HouseholdManifest>> = OnceLock::new();

#[derive(Debug, Deserialize)]
struct HouseholdManifest {
    id: String,
    opportunity_id: String,
    regional_id: String,
    name: String,
    members: Vec<HouseholdMemberManifest>,
    occupation: String,
    home_settlement: String,
    opportunity_score: i16,
    opportunity_status: String,
    service: String,
    clue: String,
    origin_location_id: String,
    destination_location_id: String,
    regional_status: String,
    reason: String,
    regional_service: String,
    history: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct HouseholdMemberManifest {
    name: String,
    occupation: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OpportunityTemplate {
    pub(crate) household_id: String,
    pub(crate) household_name: String,
    pub(crate) members: Vec<HouseholdMember>,
    pub(crate) occupation: String,
    pub(crate) home_settlement: String,
    pub(crate) opportunity_score: i16,
    pub(crate) status: HouseholdStatus,
    pub(crate) service: String,
    pub(crate) clue: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RegionalHouseholdTemplate {
    pub(crate) household_id: String,
    pub(crate) household_name: String,
    pub(crate) origin_location_id: String,
    pub(crate) destination_location_id: Option<String>,
    pub(crate) status: String,
    pub(crate) reason: String,
    pub(crate) service: String,
    pub(crate) history: Vec<String>,
}

pub(super) fn validate(region: &super::RegionManifest) -> Result<(), String> {
    let households: HouseholdsManifest = parse_json_labeled(
        "households.json",
        macroquad_toolkit::include_json_str!("../../../assets/data/households.json"),
    )
    .map_err(|error| format!("households JSON is invalid: {error}"))?;
    super::validate_id_list(
        "household",
        households
            .households
            .iter()
            .map(|household| household.id.as_str())
            .collect(),
    )?;
    let location_ids: std::collections::HashSet<&str> = region
        .locations
        .iter()
        .map(|location| location.id.as_str())
        .collect();
    if households.households.is_empty()
        || households.households.iter().any(|household| {
            household.opportunity_id.trim().is_empty()
                || household.regional_id.trim().is_empty()
                || household.name.trim().is_empty()
                || household.members.is_empty()
                || household.members.iter().any(|member| {
                    member.name.trim().is_empty() || member.occupation.trim().is_empty()
                })
                || household.occupation.trim().is_empty()
                || household.home_settlement.trim().is_empty()
                || !(0..=100).contains(&household.opportunity_score)
                || !matches!(
                    household.opportunity_status.as_str(),
                    "travelling" | "candidate" | "arrived" | "departed"
                )
                || household.service.trim().is_empty()
                || household.clue.trim().is_empty()
                || !location_ids.contains(household.origin_location_id.as_str())
                || !location_ids.contains(household.destination_location_id.as_str())
                || household.regional_status.trim().is_empty()
                || household.reason.trim().is_empty()
                || household.regional_service.trim().is_empty()
                || household.history.is_empty()
                || household
                    .history
                    .iter()
                    .any(|entry| entry.trim().is_empty())
        })
    {
        return Err(
            "households need members, opportunity data, known locations, and regional history"
                .to_owned(),
        );
    }
    Ok(())
}

pub(crate) fn opportunity_template(household_id: &str) -> OpportunityTemplate {
    let household = household(household_id);
    OpportunityTemplate {
        household_id: household.opportunity_id.clone(),
        household_name: household.name.clone(),
        members: household
            .members
            .iter()
            .map(|member| HouseholdMember {
                name: member.name.clone(),
                occupation: member.occupation.clone(),
            })
            .collect(),
        occupation: household.occupation.clone(),
        home_settlement: household.home_settlement.clone(),
        opportunity_score: household.opportunity_score,
        status: match household.opportunity_status.as_str() {
            "travelling" => HouseholdStatus::Travelling,
            "candidate" => HouseholdStatus::Candidate,
            "arrived" => HouseholdStatus::Arrived,
            "departed" => HouseholdStatus::Departed,
            _ => panic!("validated household catalog contains an unsupported opportunity status"),
        },
        service: household.service.clone(),
        clue: household.clue.clone(),
    }
}

pub(crate) fn regional_household_template(household_id: &str) -> RegionalHouseholdTemplate {
    let household = household(household_id);
    RegionalHouseholdTemplate {
        household_id: household.regional_id.clone(),
        household_name: household.name.clone(),
        origin_location_id: household.origin_location_id.clone(),
        destination_location_id: Some(household.destination_location_id.clone()),
        status: household.regional_status.clone(),
        reason: household.reason.clone(),
        service: household.regional_service.clone(),
        history: household.history.clone(),
    }
}

fn household(household_id: &str) -> &HouseholdManifest {
    let households = HOUSEHOLD_CATALOG.get_or_init(|| {
        let households: HouseholdsManifest = parse_json_labeled(
            "households.json",
            macroquad_toolkit::include_json_str!("../../../assets/data/households.json"),
        )
        .expect("households content JSON must be valid");
        let region = super::region_catalog();
        validate(region).expect("households content must satisfy its schema");
        households.households
    });
    households
        .iter()
        .find(|household| household.id == household_id)
        .expect("validated household catalog must contain the requested household")
}
