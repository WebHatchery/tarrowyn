//! Touch-driven client projection for the regional map and production boundary.

use super::*;
use tarrowyn_protocol::{
    AccountDeletionRequest, AccountResponse, ApiResponse, AuthLinkRequest, AuthLinkResponse,
    AuthRefreshResponse, AuthSession, GuestSessionResponse, LawBoundaryResponse, MarketOrder,
    MarketOrderAction, MarketOrderRequest, MarketSnapshot, ModerationReportRequest, RegionSnapshot,
    RegionalEventAction, RegionalEventRequest, RegionalEventsResponse, RegionalHouseholdsResponse,
    RouteAction, RouteRequest, SettlementsResponse, TravelAction, TravelRequest, TravelStatus,
};

const MAX_CACHED_REGIONAL_EVENTS: usize = 2048;
const MAX_COMMAND_RETRIES: u8 = 3;
const COMMAND_RETRY_DELAY_SECONDS: f32 = 1.0;
const MAX_REFRESH_RETRIES: u8 = 3;
const REFRESH_RETRY_DELAY_SECONDS: f32 = 1.0;

mod auth;
mod commands;
mod events;
mod location;
mod market;
mod online;
mod routes;
mod summary;
mod sync;
mod travel;
use commands::{Phase5Command, Phase5CommandResponse};

pub(super) struct Phase5Client {
    pending_region: Option<Pending<ApiResponse<RegionSnapshot>>>,
    pending_settlements: Option<Pending<ApiResponse<SettlementsResponse>>>,
    pending_households: Option<Pending<ApiResponse<RegionalHouseholdsResponse>>>,
    pending_market: Option<Pending<ApiResponse<MarketSnapshot>>>,
    pending_events: Option<Pending<ApiResponse<RegionalEventsResponse>>>,
    pending_law: Option<Pending<ApiResponse<LawBoundaryResponse>>>,
    pending_account: Option<Pending<ApiResponse<AccountResponse>>>,
    pending_refresh: Option<Pending<ApiResponse<AuthRefreshResponse>>>,
    in_flight_refresh: Option<tarrowyn_protocol::AuthRefreshRequest>,
    pending_command: Option<Pending<ApiResponse<Phase5CommandResponse>>>,
    pending_market_action: Option<MarketOrderAction>,
    in_flight_command: Option<Phase5Command>,
    commands: VecDeque<Phase5Command>,
    region: Option<RegionSnapshot>,
    settlements: Option<SettlementsResponse>,
    households: Option<RegionalHouseholdsResponse>,
    market: Option<MarketSnapshot>,
    events: Option<RegionalEventsResponse>,
    law: Option<LawBoundaryResponse>,
    account: Option<AccountResponse>,
    linked_account: Option<AuthLinkResponse>,
    refreshed_session: Option<AuthSession>,
    refresh_token: Option<String>,
    logged_out: bool,
    deletion_armed: bool,
    own_account_id: Option<String>,
    refresh_timer: f32,
    auth_refresh_timer: f32,
    refresh_retry_timer: f32,
    refresh_retry_count: u8,
    command_retry_timer: f32,
    command_retry_count: u8,
    next_request_id: u64,
    projection_cursor: u64,
}

impl Phase5Client {
    pub(super) fn new() -> Self {
        Self {
            pending_region: None,
            pending_settlements: None,
            pending_households: None,
            pending_market: None,
            pending_events: None,
            pending_law: None,
            pending_account: None,
            pending_refresh: None,
            in_flight_refresh: None,
            pending_command: None,
            pending_market_action: None,
            in_flight_command: None,
            commands: VecDeque::new(),
            region: None,
            settlements: None,
            households: None,
            market: None,
            events: None,
            law: None,
            account: None,
            linked_account: None,
            refreshed_session: None,
            refresh_token: None,
            logged_out: false,
            deletion_armed: false,
            own_account_id: None,
            refresh_timer: 0.0,
            auth_refresh_timer: f32::MAX,
            refresh_retry_timer: 0.0,
            refresh_retry_count: 0,
            command_retry_timer: 0.0,
            command_retry_count: 0,
            next_request_id: 1,
            projection_cursor: 0,
        }
    }

    pub(super) fn set_account(&mut self, account_id: Option<&str>) {
        self.own_account_id = account_id.map(str::to_owned);
    }

    pub(super) fn auth_refresh_pending(&self) -> bool {
        self.pending_refresh.is_some() || self.in_flight_refresh.is_some()
    }

    pub(super) fn command_pending(&self) -> bool {
        self.pending_command.is_some()
    }

    pub(super) fn market_command_pending(&self) -> bool {
        self.in_flight_command
            .as_ref()
            .is_some_and(|command| matches!(command, Phase5Command::Market(_)))
            || self
                .commands
                .iter()
                .any(|command| matches!(command, Phase5Command::Market(_)))
    }

    pub(super) fn event_command_pending(&self) -> bool {
        self.in_flight_command
            .as_ref()
            .is_some_and(|command| matches!(command, Phase5Command::Event(_)))
            || self
                .commands
                .iter()
                .any(|command| matches!(command, Phase5Command::Event(_)))
    }

    pub(super) fn dispatch_blocked(&self) -> bool {
        self.logged_out || self.auth_refresh_pending()
    }

    #[cfg(test)]
    pub(super) fn prime_refresh_for_test(&mut self) {
        self.refresh_token = Some("refresh-secret".to_owned());
        self.auth_refresh_timer = 0.0;
    }

    #[cfg(test)]
    pub(super) fn refresh_request_pending_for_test(&self) -> bool {
        self.pending_refresh.is_some()
    }

    #[cfg(test)]
    pub(super) fn prime_linked_account_for_test(&mut self, response: AuthLinkResponse) {
        self.linked_account = Some(response);
    }

    pub(super) fn update(
        &mut self,
        dt: f32,
        api: &mut HttpClient,
        projection: &mut WorldProjection,
        online: bool,
        another_mutation_pending: bool,
        notices: &mut Vec<NetworkNotice>,
    ) {
        if !online {
            return;
        }
        self.refresh_timer = (self.refresh_timer - dt.max(0.0)).max(0.0);
        self.auth_refresh_timer = (self.auth_refresh_timer - dt.max(0.0)).max(0.0);
        self.refresh_retry_timer = (self.refresh_retry_timer - dt.max(0.0)).max(0.0);
        self.command_retry_timer = (self.command_retry_timer - dt.max(0.0)).max(0.0);
        poll(
            &mut self.pending_region,
            dt,
            |response| {
                if projection
                    .accept_response_version(response.meta.server_tick, response.meta.cursor)
                    && accept_projection_cursor(&mut self.projection_cursor, response.meta.cursor)
                {
                    self.region = Some(response.data);
                }
            },
            notices,
            "regional map",
        );
        poll(
            &mut self.pending_settlements,
            dt,
            |response| {
                if projection
                    .accept_response_version(response.meta.server_tick, response.meta.cursor)
                    && accept_projection_cursor(&mut self.projection_cursor, response.meta.cursor)
                {
                    self.settlements = Some(response.data);
                }
            },
            notices,
            "settlements",
        );
        poll(
            &mut self.pending_households,
            dt,
            |response| {
                if projection
                    .accept_response_version(response.meta.server_tick, response.meta.cursor)
                    && accept_projection_cursor(&mut self.projection_cursor, response.meta.cursor)
                {
                    self.households = Some(response.data);
                }
            },
            notices,
            "regional households",
        );
        poll(
            &mut self.pending_market,
            dt,
            |response| {
                if projection
                    .accept_response_version(response.meta.server_tick, response.meta.cursor)
                    && accept_projection_cursor(&mut self.projection_cursor, response.meta.cursor)
                {
                    self.market = Some(response.data);
                }
            },
            notices,
            "market telemetry",
        );
        self.poll_events(dt, projection, notices);
        poll(
            &mut self.pending_law,
            dt,
            |response| {
                if projection
                    .accept_response_version(response.meta.server_tick, response.meta.cursor)
                    && accept_projection_cursor(&mut self.projection_cursor, response.meta.cursor)
                {
                    self.law = Some(response.data);
                }
            },
            notices,
            "law boundary",
        );
        poll(
            &mut self.pending_account,
            dt,
            |response| {
                if projection
                    .accept_response_version(response.meta.server_tick, response.meta.cursor)
                    && accept_projection_cursor(&mut self.projection_cursor, response.meta.cursor)
                {
                    self.account = Some(response.data);
                }
            },
            notices,
            "account boundary",
        );
        self.poll_refresh(dt, api, notices);
        if let Some(result) = self
            .pending_command
            .as_mut()
            .and_then(|pending| pending.poll_timed(dt, REQUEST_TIMEOUT_SECONDS))
        {
            self.pending_command = None;
            let in_flight_command = self.in_flight_command.take();
            let market_action = self.pending_market_action.take();
            match result {
                Ok(response) => {
                    self.command_retry_timer = 0.0;
                    self.command_retry_count = 0;
                    projection
                        .record_response_version(response.meta.server_tick, response.meta.cursor);
                    accept_projection_cursor(&mut self.projection_cursor, response.meta.cursor);
                    self.apply_command(response.data, market_action, api, notices);
                }
                Err(error) if is_transient_transport_error(&error) => {
                    if self.command_retry_count < MAX_COMMAND_RETRIES {
                        if let Some(command) = in_flight_command {
                            self.commands.push_front(command);
                            self.command_retry_count += 1;
                            self.command_retry_timer = COMMAND_RETRY_DELAY_SECONDS;
                            notices.push(NetworkNotice::Warning(format!(
                                "The regional command could not be confirmed; retrying the same request ({}/{}). {}",
                                self.command_retry_count,
                                MAX_COMMAND_RETRIES,
                                short_error(&error)
                            )));
                        }
                    } else {
                        self.command_retry_count = 0;
                        notices.push(NetworkNotice::Warning(format!(
                            "The regional command could not be confirmed: {}",
                            short_error(&error)
                        )));
                    }
                }
                Err(error) => {
                    self.command_retry_count = 0;
                    notices.push(NetworkNotice::Warning(format!(
                        "The regional command could not be confirmed: {}",
                        short_error(&error)
                    )));
                }
            }
        }
        self.dispatch(api, another_mutation_pending);
    }

    pub(super) fn queue_cycle(&mut self, id: &str) -> bool {
        let queue_len = self.commands.len();
        let refresh_region = id == "region-map";
        let request_id = self.next_id();
        match id {
            "region-map" => self.refresh_timer = 0.0,
            "travel" => self.queue_travel(request_id),
            "recover-travel" => self.queue_travel_action(request_id, TravelAction::Recover),
            "market-region" => self.queue_market(request_id),
            "cancel-market" => self.queue_market_cancel(request_id),
            "region-event" => self.queue_event(request_id),
            "account" => {
                if !self.account_link_available() {
                    return false;
                }
                let _ = super::queue::try_push(
                    &mut self.commands,
                    Phase5Command::Link(AuthLinkRequest {
                        request_id,
                        provider: "webhatchery-identity-oidc".to_owned(),
                        subject: self
                            .own_account_id
                            .clone()
                            .unwrap_or_else(|| "guest-subject".to_owned()),
                        display_name: None,
                    }),
                );
            }
            "logout" => {
                let _ = super::queue::try_push(
                    &mut self.commands,
                    Phase5Command::Revoke(tarrowyn_protocol::AuthRevokeRequest {
                        request_id,
                        revoke_all: true,
                    }),
                );
            }
            "report" => {
                self.queue_report(request_id, None, None);
            }
            "delete-account" => {
                let Some(account) = self
                    .account
                    .as_ref()
                    .filter(|account| !account.guest_fixture)
                else {
                    return false;
                };
                if self.deletion_armed {
                    self.deletion_armed = false;
                    super::queue::try_push(
                        &mut self.commands,
                        Phase5Command::Delete(AccountDeletionRequest {
                            request_id,
                            account_id: account.account_id.clone(),
                        }),
                    );
                } else {
                    self.deletion_armed = true;
                }
            }
            "route-repair" => {
                self.queue_route_action(request_id, RouteAction::Repair);
            }
            "route-escort" => {
                self.queue_route_action(request_id, RouteAction::Escort);
            }
            "route-improve" => {
                self.queue_route_action(request_id, RouteAction::Improve);
            }
            _ => {}
        }
        self.commands.len() > queue_len
            || refresh_region
            || (id == "delete-account" && self.deletion_armed)
    }

    pub(super) fn queue_report(
        &mut self,
        request_id: String,
        target_account_id: Option<String>,
        message_id: Option<u64>,
    ) -> bool {
        super::queue::try_push(
            &mut self.commands,
            Phase5Command::Report(ModerationReportRequest {
                request_id,
                target_account_id,
                message_id,
                category: "player_report".to_owned(),
                note: "Report submitted from the visible touch control.".to_owned(),
            }),
        )
    }

    pub(super) fn travel_control(&self) -> (&'static str, bool, bool) {
        self.travel_control_details()
    }

    pub(super) fn account_link_available(&self) -> bool {
        self.refresh_token.is_none()
            && self.linked_account.is_none()
            && !self
                .in_flight_command
                .as_ref()
                .is_some_and(|command| matches!(command, Phase5Command::Link(_)))
            && !self
                .commands
                .iter()
                .any(|command| matches!(command, Phase5Command::Link(_)))
            && self
                .account
                .as_ref()
                .is_none_or(|account| account.guest_fixture)
    }

    pub(super) fn account_deletion_available(&self) -> bool {
        self.account
            .as_ref()
            .is_some_and(|account| !account.guest_fixture)
    }

    pub(super) fn clear(&mut self) {
        self.pending_region = None;
        self.pending_settlements = None;
        self.pending_households = None;
        self.pending_market = None;
        self.pending_events = None;
        self.pending_law = None;
        self.pending_account = None;
        self.pending_refresh = None;
        self.in_flight_refresh = None;
        self.pending_command = None;
        self.pending_market_action = None;
        self.in_flight_command = None;
        self.commands.clear();
        self.clear_cached_projections();
        self.account = None;
        self.linked_account = None;
        self.refreshed_session = None;
        self.refresh_token = None;
        self.logged_out = false;
        self.deletion_armed = false;
        self.refresh_timer = 0.0;
        self.auth_refresh_timer = f32::MAX;
        self.refresh_retry_timer = 0.0;
        self.refresh_retry_count = 0;
        self.command_retry_timer = 0.0;
        self.command_retry_count = 0;
        self.projection_cursor = 0;
    }

    fn clear_cached_projections(&mut self) {
        self.region = None;
        self.settlements = None;
        self.households = None;
        self.market = None;
        self.events = None;
        self.law = None;
    }

    pub(super) fn reset_identity_projections(&mut self) {
        self.pending_region = None;
        self.pending_settlements = None;
        self.pending_households = None;
        self.pending_market = None;
        self.pending_events = None;
        self.pending_law = None;
        self.pending_account = None;
        self.pending_market_action = None;
        self.commands.clear();
        self.command_retry_timer = 0.0;
        self.command_retry_count = 0;
        self.clear_cached_projections();
        self.refresh_timer = 0.0;
    }

    pub(super) fn reset_event_cursor(&mut self) {
        self.pending_region = None;
        self.pending_settlements = None;
        self.pending_households = None;
        self.pending_market = None;
        self.pending_events = None;
        self.pending_law = None;
        self.pending_account = None;
        self.pending_command = None;
        self.pending_market_action = None;
        self.in_flight_command = None;
        self.commands.clear();
        self.clear_cached_projections();
        self.refresh_timer = 0.0;
        self.command_retry_timer = 0.0;
        self.command_retry_count = 0;
        self.projection_cursor = 0;
    }

    pub(super) fn take_linked_account(
        &mut self,
        client_key: Option<&str>,
    ) -> Option<GuestSessionResponse> {
        let linked = self.linked_account.take()?;
        Some(GuestSessionResponse {
            client_key: client_key.unwrap_or("linked-client").to_owned(),
            account_id: linked.account_id,
            character_id: linked.character_id,
            display_name: linked.display_name,
            account_token: linked.session.account_token,
            expires_in_seconds: linked.session.expires_in_seconds,
        })
    }

    pub(super) fn take_logged_out(&mut self) -> bool {
        std::mem::take(&mut self.logged_out)
    }

    pub(super) fn deletion_armed(&self) -> bool {
        self.deletion_armed
    }

    pub(super) fn take_refreshed_session(&mut self) -> Option<AuthSession> {
        self.refreshed_session.take()
    }

    pub(super) fn summary(&self) -> String {
        summary::render(self)
    }

    pub(super) fn account_details(&self) -> String {
        summary::account_details(self)
    }

    pub(super) fn inspection(&self) -> String {
        summary::inspection(self)
    }

    pub(super) fn season(&self) -> Option<&str> {
        self.region.as_ref().map(|region| region.season.as_str())
    }

    pub(super) fn region_snapshot(&self) -> Option<&RegionSnapshot> {
        self.region.as_ref()
    }

    pub(super) fn has_open_market_order(&self) -> bool {
        let Some(account_id) = self.own_account_id.as_deref() else {
            return false;
        };
        self.market.as_ref().is_some_and(|market| {
            market.orders.iter().any(|order| {
                order.status == tarrowyn_protocol::MarketOrderStatus::Open
                    && order.owner_account_id == account_id
            })
        })
    }

    fn next_id(&mut self) -> String {
        let id = format!("phase5-ui-{}", self.next_request_id);
        self.next_request_id = self.next_request_id.saturating_add(1);
        id
    }
}

fn refresh_delay(expires_in_seconds: u32) -> f32 {
    (expires_in_seconds as f32 * 0.75).max(1.0)
}

fn accept_projection_cursor(current: &mut u64, incoming: Option<u64>) -> bool {
    let Some(incoming) = incoming else {
        return true;
    };
    if incoming < *current {
        return false;
    }
    *current = incoming;
    true
}

fn poll<T, F>(
    pending: &mut Option<Pending<ApiResponse<T>>>,
    dt: f32,
    apply: F,
    notices: &mut Vec<NetworkNotice>,
    label: &str,
) where
    T: serde::de::DeserializeOwned,
    F: FnOnce(ApiResponse<T>),
{
    if let Some(result) = pending
        .as_mut()
        .and_then(|pending| pending.poll_timed(dt, REQUEST_TIMEOUT_SECONDS))
    {
        *pending = None;
        match result {
            Ok(response) => apply(response),
            Err(error) => notices.push(NetworkNotice::Warning(format!(
                "The regional {label} could not be refreshed: {}",
                short_error(&error)
            ))),
        }
    }
}

impl Phase5Client {
    fn poll_events(
        &mut self,
        dt: f32,
        projection: &mut WorldProjection,
        notices: &mut Vec<NetworkNotice>,
    ) {
        let result = self
            .pending_events
            .as_mut()
            .and_then(|pending| pending.poll_timed(dt, REQUEST_TIMEOUT_SECONDS));
        let Some(result) = result else { return };
        self.pending_events = None;
        match result {
            Ok(response) => {
                let projection_current = projection
                    .accept_response_version(response.meta.server_tick, response.meta.cursor);
                if projection_current
                    && accept_projection_cursor(&mut self.projection_cursor, response.meta.cursor)
                {
                    merge_regional_events(&mut self.events, response.data);
                }
            }
            Err(error) if super::cursor::is_cursor_recovery_error(&error) => {
                self.reset_event_cursor();
                notices.push(NetworkNotice::Warning(
                    "The regional history window changed; reloading its latest events.".to_owned(),
                ));
            }
            Err(error) => notices.push(NetworkNotice::Warning(format!(
                "The regional events could not be refreshed: {}",
                short_error(&error)
            ))),
        }
    }
}

fn merge_regional_events(
    current: &mut Option<RegionalEventsResponse>,
    incoming: RegionalEventsResponse,
) {
    let Some(current) = current else {
        let mut incoming = incoming;
        let excess = incoming
            .events
            .len()
            .saturating_sub(MAX_CACHED_REGIONAL_EVENTS);
        if excess > 0 {
            incoming.events.drain(..excess);
        }
        *current = Some(incoming);
        return;
    };
    current.cursor = incoming.cursor;
    for event in incoming.events {
        if let Some(existing) = current
            .events
            .iter_mut()
            .find(|existing| existing.event_id == event.event_id)
        {
            *existing = event;
        } else {
            current.events.push(event);
        }
    }
    current.events.sort_by_key(|event| event.cursor);
    let excess = current
        .events
        .len()
        .saturating_sub(MAX_CACHED_REGIONAL_EVENTS);
    if excess > 0 {
        current.events.drain(..excess);
    }
}

fn phase5_notice(
    accepted: bool,
    reason: Option<String>,
    success: &str,
    notices: &mut Vec<NetworkNotice>,
) {
    if accepted {
        notices.push(NetworkNotice::Success(success.to_owned()));
    } else {
        notices.push(NetworkNotice::Warning(reason.unwrap_or_else(|| {
            "The regional action was not accepted.".to_owned()
        })));
    }
}

fn market_success_message(action: Option<MarketOrderAction>, fallback_used: bool) -> &'static str {
    match action {
        Some(MarketOrderAction::Create) if fallback_used => {
            "The limited travelling service accepted the shipment at a surcharge."
        }
        Some(MarketOrderAction::Create) => "The shipment is on the regional ledger.",
        Some(MarketOrderAction::Fulfil) if fallback_used => {
            "The travelling shipment reached its destination and settled."
        }
        Some(MarketOrderAction::Fulfil) => "The shipment reached its destination and settled.",
        Some(MarketOrderAction::Cancel) if fallback_used => {
            "The fallback shipment was cancelled; no player goods were escrowed."
        }
        Some(MarketOrderAction::Cancel) => "The shipment was cancelled and its escrow returned.",
        None => "The regional market accepted the command.",
    }
}

fn market_result_message(
    action: Option<MarketOrderAction>,
    fallback_used: bool,
    order: Option<&MarketOrder>,
) -> String {
    let message = market_success_message(action, fallback_used);
    let Some(order) = order else {
        return message.to_owned();
    };
    format!(
        "{message} Details: {} from {} to {} • {} gold.",
        market_quantity_label(order),
        order.origin_location_id,
        order.destination_location_id,
        order.total_price
    )
}

fn market_quantity_label(order: &MarketOrder) -> String {
    let unit = match (order.commodity, order.quantity) {
        (tarrowyn_protocol::CommodityKind::Turnips, 1) => "turnip",
        (tarrowyn_protocol::CommodityKind::Moonberries, 1) => "moonberry",
        (tarrowyn_protocol::CommodityKind::Seeds, 1) => "seed",
        (tarrowyn_protocol::CommodityKind::Bandages, 1) => "bandage",
        (tarrowyn_protocol::CommodityKind::Wheat, _) => "wheat",
        (tarrowyn_protocol::CommodityKind::Turnips, _) => "turnips",
        (tarrowyn_protocol::CommodityKind::Moonberries, _) => "moonberries",
        (tarrowyn_protocol::CommodityKind::Seeds, _) => "seeds",
        (tarrowyn_protocol::CommodityKind::Timber, _) => "timber",
        (tarrowyn_protocol::CommodityKind::Stone, _) => "stone",
        (tarrowyn_protocol::CommodityKind::Bandages, _) => "bandages",
    };
    format!("{} {unit}", order.quantity)
}

#[cfg(test)]
mod tests;
