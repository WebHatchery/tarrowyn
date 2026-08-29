//! The authoritative three-location regional proof for Phase 5.

use super::models::RepositoryState;
use super::*;
use crate::config::ServerConfig;
use tarrowyn_protocol::{
    ApiResponse, LawBoundaryResponse, MarketOrder, MarketOrderAction, MarketOrderRequest,
    MarketOrderResponse, MarketOrderStatus, MarketSnapshot, RegionSnapshot, RegionalEvent,
    RegionalEventAction, RegionalEventRequest, RegionalEventResponse, RegionalEventsResponse,
    RegionalHouseholdsResponse, RouteAction, RouteRecord, RouteRequest, RouteResponse, RouteStatus,
    SettlementsResponse, TravelAction, TravelRequest, TravelResponse, TravelState, TravelStatus,
};

mod logic;
mod market;
mod recovery;
mod settlements;
mod state;
use logic::*;
pub(super) use market::{
    close_deleted_account_orders, create_order, reconcile_market_order, trim_market_orders,
};
pub(super) use recovery::clear_stuck_travel;
use settlements::{refresh_settlement_facilities, update_households, update_settlements};
pub(super) use state::{
    fresh, trim_event_history, trim_settlement_chronicles, Phase5Response, Phase5State,
};

const REGION_ID: &str = "hearthlands";
const INTEREST_RADIUS: u32 = 12;

pub(super) fn is_request_cache_for_identity(key: &str, identity_key: &str) -> bool {
    key.starts_with(&format!("phase5:{identity_key}:"))
}

impl WorldRepository {
    pub fn region(&self, token: &str) -> Result<ApiResponse<RegionSnapshot>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        expire_sessions(&mut state, &self.config);
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
                region_id: REGION_ID.to_owned(),
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
        expire_sessions(&mut state, &self.config);
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
        expire_sessions(&mut state, &self.config);
        authenticate(&mut state, token, &self.config)?;
        Ok(ApiResponse {
            meta: meta(state.tick, None, Some(state.cursor)),
            data: state.phase5.routes.clone(),
        })
    }

    pub fn route_action(
        &self,
        token: &str,
        request: RouteRequest,
    ) -> Result<ApiResponse<RouteResponse>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        expire_sessions(&mut state, &self.config);
        let key = authenticate(&mut state, token, &self.config)?;
        validate_request_id(&request.request_id)?;
        let cache_key = cache_key(&key, &request.request_id);
        if let Some(Phase5Response::Route(response)) = state.phase5.request_results.get(&cache_key)
        {
            return Ok(ApiResponse {
                meta: meta(state.tick, Some(request.request_id), Some(state.cursor)),
                data: response.clone(),
            });
        }
        let route_index = state
            .phase5
            .routes
            .iter()
            .position(|route| route.route_id == request.route_id);
        let Some(route_index) = route_index else {
            return Err(RepositoryError::new(
                404,
                "route_not_found",
                "That regional route is not recorded.",
            ));
        };
        let current = player_location(&state, &key);
        if state.phase5.routes[route_index].origin_location_id != current
            && state.phase5.routes[route_index].destination_location_id != current
        {
            return Err(RepositoryError::new(
                409,
                "route_out_of_interest",
                "Travel to either route endpoint before changing its logistics.",
            ));
        }
        let mut route = state.phase5.routes[route_index].clone();
        let mut accepted = false;
        let reason = match request.action {
            RouteAction::Repair => {
                let cost = route.repair_cost;
                let can_pay = state.identities.get(&key).expect("identity exists").gold >= cost;
                if !can_pay {
                    Some("The repair crew needs more gold than this character carries.".to_owned())
                } else {
                    state
                        .identities
                        .get_mut(&key)
                        .expect("identity exists")
                        .gold -= cost;
                    route.condition = route.condition.saturating_add(24).min(100);
                    route.risk_percent = route.risk_percent.saturating_sub(8);
                    route.status = RouteStatus::Operational;
                    accepted = true;
                    None
                }
            }
            RouteAction::Escort => {
                route.risk_percent = route.risk_percent.saturating_sub(10);
                route.status = RouteStatus::Delayed;
                accepted = true;
                None
            }
            RouteAction::Improve => {
                route.capacity = route.capacity.saturating_add(2);
                route.travel_ticks = route.travel_ticks.saturating_sub(1).max(1);
                route.condition = route.condition.saturating_add(8).min(100);
                accepted = true;
                None
            }
        };
        route.last_action_tick = state.tick;
        state.phase5.routes[route_index] = route.clone();
        let response = RouteResponse {
            request_id: request.request_id.clone(),
            accepted,
            route: route.clone(),
            reason,
        };
        if accepted {
            let locations = [
                route.origin_location_id.as_str(),
                route.destination_location_id.as_str(),
            ];
            record_regional(
                &mut state,
                &locations,
                "route logistics",
                "A player action changed the route's cost, risk, or capacity.",
            );
        }
        state
            .phase5
            .request_results
            .insert(cache_key, Phase5Response::Route(response.clone()));
        record_command_outcome(&mut state, response.accepted);
        self.persist(&state);
        Ok(ApiResponse {
            meta: meta(state.tick, Some(request.request_id), Some(state.cursor)),
            data: response,
        })
    }

    pub fn travel(
        &self,
        token: &str,
        request: TravelRequest,
    ) -> Result<ApiResponse<TravelResponse>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        expire_sessions(&mut state, &self.config);
        let key = authenticate(&mut state, token, &self.config)?;
        validate_request_id(&request.request_id)?;
        let cache_key = cache_key(&key, &request.request_id);
        if let Some(Phase5Response::Travel(response)) = state.phase5.request_results.get(&cache_key)
        {
            return Ok(ApiResponse {
                meta: meta(state.tick, Some(request.request_id), Some(state.cursor)),
                data: response.clone(),
            });
        }
        let current_location = player_location(&state, &key);
        let current_travel = state.phase5.travel.get(&key).cloned();
        if state
            .identities
            .get(&key)
            .expect("identity exists")
            .knocked_out
        {
            let response = travel_response(
                &mut state,
                &key,
                request,
                current_travel,
                false,
                Some("You are knocked out; choose a recovery prompt before travelling.".to_owned()),
            );
            record_command_outcome(&mut state, false);
            self.persist(&state);
            return response;
        }
        let mut accepted = false;
        let mut reason = None;
        let travel = match request.action {
            TravelAction::Start => {
                if current_travel.as_ref().is_some_and(|travel| {
                    matches!(
                        travel.status,
                        TravelStatus::Travelling
                            | TravelStatus::Interrupted
                            | TravelStatus::Recovering
                    )
                }) {
                    reason = Some(
                        "Finish or recover the current journey before starting another.".to_owned(),
                    );
                    current_travel
                } else {
                    let Some(route_id) = request.route_id.as_deref() else {
                        reason = Some("Choose a visible route before starting travel.".to_owned());
                        let response =
                            travel_response(&mut state, &key, request, None, false, reason);
                        record_command_outcome(&mut state, false);
                        self.persist(&state);
                        return response;
                    };
                    let Some(route) = state
                        .phase5
                        .routes
                        .iter()
                        .find(|route| {
                            route.route_id == route_id
                                && (route.origin_location_id == current_location
                                    || route.destination_location_id == current_location)
                        })
                        .cloned()
                    else {
                        reason = Some(
                            "That route does not connect to the player's current location."
                                .to_owned(),
                        );
                        let response =
                            travel_response(&mut state, &key, request, None, false, reason);
                        record_command_outcome(&mut state, false);
                        self.persist(&state);
                        return response;
                    };
                    if route.status == RouteStatus::Closed {
                        reason = Some(
                            "The route is closed; a repair or escort action is required."
                                .to_owned(),
                        );
                        None
                    } else {
                        let (origin_location_id, destination_location_id) =
                            if route.origin_location_id == current_location {
                                (
                                    route.origin_location_id.clone(),
                                    route.destination_location_id,
                                )
                            } else {
                                (
                                    route.destination_location_id.clone(),
                                    route.origin_location_id,
                                )
                            };
                        let travel = TravelState {
                            travel_id: format!("travel-{}", state.phase5.next_travel_id),
                            route_id: route.route_id,
                            origin_location_id,
                            destination_location_id,
                            departure_tick: state.tick,
                            eta_tick: state.tick.saturating_add(route.travel_ticks.max(1)),
                            progress: 0,
                            risk_percent: route.risk_percent,
                            status: TravelStatus::Travelling,
                            interruption: None,
                            recovery_note: None,
                        };
                        state.phase5.next_travel_id = state.phase5.next_travel_id.saturating_add(1);
                        state.phase5.travel.insert(key.clone(), travel.clone());
                        accepted = true;
                        record_regional(
                            &mut state,
                            &[current_location.as_str()],
                            "journey started",
                            "A durable travel command entered the regional ledger.",
                        );
                        Some(travel)
                    }
                }
            }
            TravelAction::Interrupt => {
                let Some(mut travel) = current_travel else {
                    reason = Some("There is no journey to interrupt.".to_owned());
                    let response = travel_response(&mut state, &key, request, None, false, reason);
                    record_command_outcome(&mut state, false);
                    self.persist(&state);
                    return response;
                };
                {
                    if travel.status != TravelStatus::Travelling {
                        reason = Some("Only a journey in progress can be interrupted.".to_owned());
                    } else {
                        travel.status = TravelStatus::Interrupted;
                        travel.interruption =
                            Some("A route warning stopped the caravan safely.".to_owned());
                        travel.recovery_note = Some(
                            "Tap Recover or Resume to continue; no cargo was lost.".to_owned(),
                        );
                        state.phase5.travel.insert(key.clone(), travel.clone());
                        accepted = true;
                        record_regional(
                            &mut state,
                            &[current_location.as_str()],
                            "journey interrupted",
                            "The caravan stopped with its cargo and character state recoverable.",
                        );
                    }
                }
                Some(travel)
            }
            TravelAction::Resume | TravelAction::Recover => {
                let Some(mut travel) = current_travel else {
                    reason = Some("There is no interrupted journey to recover.".to_owned());
                    let response = travel_response(&mut state, &key, request, None, false, reason);
                    record_command_outcome(&mut state, false);
                    self.persist(&state);
                    return response;
                };
                {
                    if travel.status != TravelStatus::Interrupted {
                        reason = Some("That journey is not waiting for recovery.".to_owned());
                    } else {
                        travel.status = TravelStatus::Travelling;
                        travel.recovery_note =
                            Some("The route crew found a safe continuation.".to_owned());
                        travel.eta_tick = state.tick.saturating_add(3);
                        accepted = true;
                        state.phase5.travel.insert(key.clone(), travel.clone());
                        record_regional(&mut state, &[current_location.as_str()], "journey recovered", "A player resumed an interrupted journey without duplicating cargo or rewards.");
                    }
                }
                Some(travel)
            }
        };
        let response = TravelResponse {
            request_id: request.request_id.clone(),
            accepted,
            travel,
            location_id: current_location,
            reason,
        };
        state
            .phase5
            .request_results
            .insert(cache_key, Phase5Response::Travel(response.clone()));
        record_command_outcome(&mut state, response.accepted);
        self.persist(&state);
        Ok(ApiResponse {
            meta: meta(state.tick, Some(request.request_id), Some(state.cursor)),
            data: response,
        })
    }

    pub fn market(&self, token: &str) -> Result<ApiResponse<MarketSnapshot>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        expire_sessions(&mut state, &self.config);
        let key = authenticate(&mut state, token, &self.config)?;
        let location = player_location(&state, &key);
        Ok(ApiResponse {
            meta: meta(state.tick, None, Some(state.cursor)),
            data: MarketSnapshot {
                orders: state.phase5.market_orders.clone(),
                stock_notes: stock_notes(&state, &location),
                prices: price_notes(&state, &location),
                cursor: state.cursor,
            },
        })
    }

    pub fn market_order(
        &self,
        token: &str,
        request: MarketOrderRequest,
    ) -> Result<ApiResponse<MarketOrderResponse>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        expire_sessions(&mut state, &self.config);
        let key = authenticate(&mut state, token, &self.config)?;
        validate_request_id(&request.request_id)?;
        let cache_key = cache_key(&key, &request.request_id);
        if let Some(Phase5Response::Market(response)) = state.phase5.request_results.get(&cache_key)
        {
            return Ok(ApiResponse {
                meta: meta(state.tick, Some(request.request_id), Some(state.cursor)),
                data: response.clone(),
            });
        }
        let location = player_location(&state, &key);
        let (accepted, order, reason) = match request.action {
            MarketOrderAction::Create => create_order(&mut state, &key, &location, &request),
            MarketOrderAction::Fulfil => {
                fulfil_order(&mut state, &key, request.order_id.as_deref())
            }
            MarketOrderAction::Cancel => {
                cancel_order(&mut state, &key, request.order_id.as_deref())
            }
        };
        let response = MarketOrderResponse {
            request_id: request.request_id.clone(),
            accepted,
            order,
            reason,
        };
        if accepted {
            let locations = response
                .order
                .as_ref()
                .map(|order| {
                    [
                        order.origin_location_id.as_str(),
                        order.destination_location_id.as_str(),
                    ]
                })
                .unwrap_or([location.as_str(), location.as_str()]);
            record_regional(
                &mut state,
                &locations,
                "regional market",
                "A cross-settlement order changed stock, price, and route demand.",
            );
        }
        state
            .phase5
            .request_results
            .insert(cache_key, Phase5Response::Market(response.clone()));
        record_command_outcome(&mut state, response.accepted);
        self.persist(&state);
        Ok(ApiResponse {
            meta: meta(state.tick, Some(request.request_id), Some(state.cursor)),
            data: response,
        })
    }

    pub fn events_region(
        &self,
        token: &str,
        since: u64,
    ) -> Result<ApiResponse<RegionalEventsResponse>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        expire_sessions(&mut state, &self.config);
        authenticate(&mut state, token, &self.config)?;
        super::validate_event_cursor(&state, since, "regional")?;
        if state.phase5.event_history_floor > since {
            return Err(RepositoryError::new(
                409,
                "cursor_stale",
                "The regional event history is no longer retained; reload authoritative state.",
            ));
        }
        Ok(ApiResponse {
            meta: meta(state.tick, None, Some(state.cursor)),
            data: RegionalEventsResponse {
                events: state
                    .phase5
                    .events
                    .iter()
                    .filter(|event| event.cursor > since)
                    .cloned()
                    .collect(),
                cursor: state.cursor,
            },
        })
    }

    pub fn event_action(
        &self,
        token: &str,
        request: RegionalEventRequest,
    ) -> Result<ApiResponse<RegionalEventResponse>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        expire_sessions(&mut state, &self.config);
        let key = authenticate(&mut state, token, &self.config)?;
        validate_request_id(&request.request_id)?;
        let cache_key = cache_key(&key, &request.request_id);
        if let Some(Phase5Response::Event(response)) = state.phase5.request_results.get(&cache_key)
        {
            return Ok(ApiResponse {
                meta: meta(state.tick, Some(request.request_id), Some(state.cursor)),
                data: response.clone(),
            });
        }
        let (accepted, event, reason) = match request.action {
            RegionalEventAction::Seed => seed_event(&mut state),
            RegionalEventAction::Intervene => intervene_event(
                &mut state,
                request.event_id.as_deref(),
                request.intervention.as_deref(),
            ),
            RegionalEventAction::Resolve => resolve_event(&mut state, request.event_id.as_deref()),
        };
        let response = RegionalEventResponse {
            request_id: request.request_id.clone(),
            accepted,
            event,
            reason,
        };
        state
            .phase5
            .request_results
            .insert(cache_key, Phase5Response::Event(response.clone()));
        record_command_outcome(&mut state, response.accepted);
        self.persist(&state);
        Ok(ApiResponse {
            meta: meta(state.tick, Some(request.request_id), Some(state.cursor)),
            data: response,
        })
    }

    pub fn households_region(
        &self,
        token: &str,
    ) -> Result<ApiResponse<RegionalHouseholdsResponse>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        expire_sessions(&mut state, &self.config);
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
        expire_sessions(&mut state, &self.config);
        authenticate(&mut state, token, &self.config)?;
        Ok(ApiResponse { meta: meta(state.tick, None, Some(state.cursor)), data: LawBoundaryResponse { pvp_enabled: false, theft_enabled: false, claims_protected: true, trade_protected: true, travel_protected: true, recovery_path: "Protected spaces, server-owned trades, and support repair preserve character and property state.".to_owned(), policy_version: "phase5-no-pvp-1".to_owned() } })
    }
}

pub(super) fn phase5_tick(state: &mut RepositoryState, config: &ServerConfig) {
    advance_travel(state);
    if state
        .tick
        .is_multiple_of(config.household_decision_interval_ticks.max(1))
    {
        update_settlements(state);
        update_households(state);
    }
    advance_events(state);
    expire_market_orders(state);
    refresh_settlement_facilities(state);
}

fn fulfil_order(
    state: &mut RepositoryState,
    key: &str,
    order_id: Option<&str>,
) -> (bool, Option<MarketOrder>, Option<String>) {
    let Some(index) = state
        .phase5
        .market_orders
        .iter()
        .position(|order| Some(order.order_id.as_str()) == order_id)
    else {
        return (
            false,
            None,
            Some("That market order is not recorded.".to_owned()),
        );
    };
    let location = player_location(state, key);
    let order = state.phase5.market_orders[index].clone();
    if order.status != MarketOrderStatus::Open {
        return (
            false,
            Some(order),
            Some("That order has already been settled or closed.".to_owned()),
        );
    }
    if order.destination_location_id != location {
        return (
            false,
            Some(order),
            Some("Arrive at the order destination before settling the shipment.".to_owned()),
        );
    }
    let owner_key = state
        .identities
        .iter()
        .find(|(_, identity)| identity.account_id == order.owner_account_id)
        .map(|(key, _)| key.clone());
    if let Some(owner_key) = owner_key {
        give_commodity(
            state,
            &owner_key,
            &order.destination_location_id,
            order.commodity,
            order.quantity,
        );
    }
    if let Some(identity) = state.identities.get_mut(key) {
        identity.gold = identity.gold.saturating_add(order.total_price);
        identity.reputation = identity.reputation.saturating_add(1);
    }
    state.phase5.market_orders[index].status = MarketOrderStatus::Fulfilled;
    state.phase5.market_orders[index].settled_tick = Some(state.tick);
    (true, Some(state.phase5.market_orders[index].clone()), None)
}

fn cancel_order(
    state: &mut RepositoryState,
    key: &str,
    order_id: Option<&str>,
) -> (bool, Option<MarketOrder>, Option<String>) {
    let Some(index) = state
        .phase5
        .market_orders
        .iter()
        .position(|order| Some(order.order_id.as_str()) == order_id)
    else {
        return (
            false,
            None,
            Some("That market order is not recorded.".to_owned()),
        );
    };
    let account = state
        .identities
        .get(key)
        .expect("identity exists")
        .account_id
        .clone();
    let order = state.phase5.market_orders[index].clone();
    if order.owner_account_id != account {
        return (
            false,
            Some(order.clone()),
            Some("Only the order owner can cancel an escrow.".to_owned()),
        );
    }
    if order.status != MarketOrderStatus::Open {
        return (
            false,
            Some(order.clone()),
            Some("Only an open order can be cancelled.".to_owned()),
        );
    }
    give_commodity(
        state,
        key,
        &order.origin_location_id,
        order.commodity,
        order.quantity,
    );
    state.phase5.market_orders[index].status = MarketOrderStatus::Cancelled;
    state.phase5.market_orders[index].settled_tick = Some(state.tick);
    (true, Some(state.phase5.market_orders[index].clone()), None)
}

#[cfg(test)]
mod tests;
