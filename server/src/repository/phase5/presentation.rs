use super::super::*;
use super::logic::{player_location, season};
use super::settlements::refresh_settlement_facilities;
use tarrowyn_protocol::{
    ApiResponse, LawBoundaryResponse, RegionSnapshot, RegionalHouseholdsResponse, RouteRecord,
    SettlementsResponse,
};

const INTEREST_RADIUS: u32 = 12;

impl WorldRepository {
    pub fn region(&self, token: &str) -> Result<ApiResponse<RegionSnapshot>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        self.expire_and_persist_sessions(&mut state);
        let key = authenticate(&mut state, token, &self.config)?;
        refresh_settlement_facilities(&mut state);
        let location_id = player_location(&state, &key);
        let location = state
            .phase5
            .locations
            .iter()
            .find(|location| location.location_id == location_id)
            .expect("hearth location exists");
        let visible = state
            .phase5
            .locations
            .iter()
            .filter(|candidate| {
                location.position.manhattan_distance(candidate.position) <= INTEREST_RADIUS
            })
            .cloned()
            .collect::<Vec<_>>();
        let visible_ids: Vec<_> = visible
            .iter()
            .map(|item| item.location_id.as_str())
            .collect();
        let routes = state
            .phase5
            .routes
            .iter()
            .filter(|route| {
                visible_ids.contains(&route.origin_location_id.as_str())
                    || visible_ids.contains(&route.destination_location_id.as_str())
            })
            .cloned()
            .collect();
        let settlements = state
            .phase5
            .settlements
            .iter()
            .filter(|settlement| visible_ids.contains(&settlement.location_id.as_str()))
            .cloned()
            .collect();
        Ok(ApiResponse {
            meta: meta(state.tick, None, Some(state.cursor)),
            data: RegionSnapshot {
                region_id: crate::content::region_id(),
                season: season(state.clock.day),
                calendar_day: state.clock.day,
                locations: visible,
                routes,
                visible_settlements: settlements,
                player_location_id: location_id,
                travel: state.phase5.travel.get(&key).cloned(),
                interest_radius: INTEREST_RADIUS,
                cursor: state.cursor,
            },
        })
    }

    pub fn settlements(
        &self,
        token: &str,
    ) -> Result<ApiResponse<SettlementsResponse>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        self.expire_and_persist_sessions(&mut state);
        authenticate(&mut state, token, &self.config)?;
        refresh_settlement_facilities(&mut state);
        Ok(ApiResponse {
            meta: meta(state.tick, None, Some(state.cursor)),
            data: SettlementsResponse {
                settlements: state.phase5.settlements.clone(),
                cursor: state.cursor,
            },
        })
    }

    pub fn routes(&self, token: &str) -> Result<ApiResponse<Vec<RouteRecord>>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        self.expire_and_persist_sessions(&mut state);
        authenticate(&mut state, token, &self.config)?;
        Ok(ApiResponse {
            meta: meta(state.tick, None, Some(state.cursor)),
            data: state.phase5.routes.clone(),
        })
    }

    pub fn households_region(
        &self,
        token: &str,
    ) -> Result<ApiResponse<RegionalHouseholdsResponse>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        self.expire_and_persist_sessions(&mut state);
        authenticate(&mut state, token, &self.config)?;
        Ok(ApiResponse {
            meta: meta(state.tick, None, Some(state.cursor)),
            data: RegionalHouseholdsResponse {
                households: state.phase5.households.clone(),
                vacancies: state
                    .phase5
                    .settlements
                    .iter()
                    .flat_map(|settlement| settlement.vacancies.clone())
                    .collect(),
                cursor: state.cursor,
            },
        })
    }

    pub fn law_boundary(
        &self,
        token: &str,
    ) -> Result<ApiResponse<LawBoundaryResponse>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        self.expire_and_persist_sessions(&mut state);
        authenticate(&mut state, token, &self.config)?;
        Ok(ApiResponse {
            meta: meta(state.tick, None, Some(state.cursor)),
            data: LawBoundaryResponse {
                pvp_enabled: false,
                theft_enabled: false,
                claims_protected: true,
                trade_protected: true,
                travel_protected: true,
                recovery_path: "Protected spaces, server-owned trades, and support repair preserve character and property state.".to_owned(),
                policy_version: "phase5-no-pvp-1".to_owned(),
            },
        })
    }
}
