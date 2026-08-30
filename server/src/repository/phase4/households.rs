use super::record;
use crate::config::ServerConfig;
use tarrowyn_protocol::{
    ApiResponse, HouseholdLifeStatus, HouseholdsResponse, InfrastructureStatus,
};

impl super::super::WorldRepository {
    pub fn households(
        &self,
        token: &str,
    ) -> Result<ApiResponse<HouseholdsResponse>, super::super::RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        self.expire_and_persist_sessions(&mut state);
        super::super::authenticate(&mut state, token, &self.config)?;
        Ok(ApiResponse {
            meta: super::super::meta(state.tick, None, Some(state.cursor)),
            data: HouseholdsResponse {
                households: state.phase4.households.clone(),
                cursor: state.cursor,
            },
        })
    }
}

pub(super) fn tick(state: &mut super::super::models::RepositoryState, config: &ServerConfig) {
    let interval = config.household_decision_interval_ticks.max(1);
    if state.tick == 0 || !state.tick.is_multiple_of(interval) {
        return;
    }
    let road = state
        .phase4
        .infrastructure
        .iter()
        .find(|record| record.infrastructure_id == "north-road");
    let road_safe = road.is_some_and(|record| record.status != InfrastructureStatus::Failed);
    let open_orders = state
        .phase4
        .orders
        .iter()
        .filter(|order| {
            matches!(
                order.status,
                tarrowyn_protocol::ServiceOrderStatus::Open
                    | tarrowyn_protocol::ServiceOrderStatus::Accepted
            )
        })
        .count() as u8;
    let funded = state.phase4.governance.service_funding_until_tick >= state.tick;
    let mut transitions = Vec::new();
    for household in &mut state.phase4.households {
        let old_status = household.status;
        household.last_decision_tick = state.tick;
        household.demand = household
            .demand
            .saturating_add(open_orders.saturating_mul(4))
            .min(100);
        household.safety = if road_safe { 78 } else { 38 };
        household.housing = if state.phase4.available_plots.is_empty() {
            48
        } else {
            72
        };
        household.food = if state.plots.iter().any(|plot| plot.crop.is_some()) {
            76
        } else {
            58
        };
        household.competition = open_orders.saturating_mul(10).min(80);
        let poor = household.safety < 50 || household.housing < 50 || household.food < 50;
        if poor {
            household.service_quality = household.service_quality.saturating_sub(8);
            household.clue = if household.safety < 50 {
                "The Bellweather household has reduced service: the road no longer feels safe."
                    .to_owned()
            } else {
                "The Bellweather household asks for housing, food, or demand before it commits."
                    .to_owned()
            };
            household.status = match household.status {
                HouseholdLifeStatus::Arrived => HouseholdLifeStatus::ReducedService,
                HouseholdLifeStatus::ReducedService => HouseholdLifeStatus::ConsideringDeparture,
                HouseholdLifeStatus::ConsideringDeparture => HouseholdLifeStatus::Departed,
                HouseholdLifeStatus::Departed => HouseholdLifeStatus::Departed,
            };
        } else {
            household.service_quality = household
                .service_quality
                .saturating_add(if funded { 6 } else { 2 })
                .min(100);
            household.clue = "The miller and healer are staying because demand, food, housing, and roads are holding.".to_owned();
            if household.status == HouseholdLifeStatus::Departed && household.demand >= 60 {
                household.status = HouseholdLifeStatus::Arrived;
                transitions.push((
                    "household arrival",
                    "The Bellweather household returns when the settlement can use both services.",
                ));
            } else if household.status != HouseholdLifeStatus::Arrived {
                household.status = HouseholdLifeStatus::Arrived;
                transitions.push((
                    "household service restored",
                    "The complementary miller and healer restore their local service.",
                ));
            }
        }
        if household.status != old_status {
            match household.status {
                HouseholdLifeStatus::ReducedService => transitions.push(("household reduced service", "A household reduced service after exposing the conditions that caused the strain.")),
                HouseholdLifeStatus::ConsideringDeparture => transitions.push(("household departure warning", "A household is considering departure; its demand, housing, food, and safety clues remain visible.")),
                HouseholdLifeStatus::Departed => transitions.push(("household departure", "The Bellweather household left after sustained poor conditions.")),
                HouseholdLifeStatus::Arrived => {}
            }
        }
    }
    for (kind, text) in transitions {
        record(
            state,
            kind,
            "Local life responds to settlement conditions",
            text,
        );
    }
}
