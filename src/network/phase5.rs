//! Touch-driven client projection for the regional map and production boundary.

use super::*;
use serde::Deserialize;
use tarrowyn_protocol::{
    AccountDeletionRequest, AccountDeletionResponse, AccountResponse, ApiResponse, AuthLinkRequest,
    AuthLinkResponse, AuthRefreshResponse, AuthRevokeResponse, AuthSession, GuestSessionResponse,
    LawBoundaryResponse, MarketOrderAction, MarketOrderRequest, MarketSnapshot,
    ModerationReportRequest, ModerationReportResponse, RegionSnapshot, RegionalEventAction,
    RegionalEventRequest, RegionalEventResponse, RegionalEventsResponse, RouteAction, RouteRequest,
    RouteResponse, SettlementsResponse, TravelAction, TravelRequest, TravelResponse, TravelStatus,
};

mod summary;

enum Phase5Command {
    Travel(TravelRequest),
    Route(RouteRequest),
    Market(MarketOrderRequest),
    Event(RegionalEventRequest),
    Link(AuthLinkRequest),
    Revoke(tarrowyn_protocol::AuthRevokeRequest),
    Report(ModerationReportRequest),
    Delete(AccountDeletionRequest),
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Phase5CommandResponse {
    Travel(TravelResponse),
    Route(RouteResponse),
    Delete(AccountDeletionResponse),
    Market(tarrowyn_protocol::MarketOrderResponse),
    Event(RegionalEventResponse),
    Link(AuthLinkResponse),
    Revoke(AuthRevokeResponse),
    Report(ModerationReportResponse),
}

pub(super) struct Phase5Client {
    pending_region: Option<Pending<ApiResponse<RegionSnapshot>>>,
    pending_settlements: Option<Pending<ApiResponse<SettlementsResponse>>>,
    pending_market: Option<Pending<ApiResponse<MarketSnapshot>>>,
    pending_events: Option<Pending<ApiResponse<RegionalEventsResponse>>>,
    pending_law: Option<Pending<ApiResponse<LawBoundaryResponse>>>,
    pending_account: Option<Pending<ApiResponse<AccountResponse>>>,
    pending_refresh: Option<Pending<ApiResponse<AuthRefreshResponse>>>,
    pending_command: Option<Pending<ApiResponse<Phase5CommandResponse>>>,
    commands: VecDeque<Phase5Command>,
    region: Option<RegionSnapshot>,
    settlements: Option<SettlementsResponse>,
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
    next_request_id: u64,
}

impl Phase5Client {
    pub(super) fn new() -> Self {
        Self {
            pending_region: None,
            pending_settlements: None,
            pending_market: None,
            pending_events: None,
            pending_law: None,
            pending_account: None,
            pending_refresh: None,
            pending_command: None,
            commands: VecDeque::new(),
            region: None,
            settlements: None,
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
            next_request_id: 1,
        }
    }

    pub(super) fn set_account(&mut self, account_id: Option<&str>) {
        self.own_account_id = account_id.map(str::to_owned);
    }

    pub(super) fn update(
        &mut self,
        dt: f32,
        api: &mut HttpClient,
        online: bool,
        notices: &mut Vec<NetworkNotice>,
    ) {
        if !online {
            return;
        }
        self.refresh_timer = (self.refresh_timer - dt.max(0.0)).max(0.0);
        self.auth_refresh_timer = (self.auth_refresh_timer - dt.max(0.0)).max(0.0);
        poll(
            &mut self.pending_region,
            dt,
            |response| self.region = Some(response.data),
            notices,
            "regional map",
        );
        poll(
            &mut self.pending_settlements,
            dt,
            |response| self.settlements = Some(response.data),
            notices,
            "settlements",
        );
        poll(
            &mut self.pending_market,
            dt,
            |response| self.market = Some(response.data),
            notices,
            "market telemetry",
        );
        self.poll_events(dt, notices);
        poll(
            &mut self.pending_law,
            dt,
            |response| self.law = Some(response.data),
            notices,
            "law boundary",
        );
        poll(
            &mut self.pending_account,
            dt,
            |response| self.account = Some(response.data),
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
            match result {
                Ok(response) => self.apply_command(response.data, api, notices),
                Err(error) => notices.push(NetworkNotice::Warning(format!(
                    "The regional command could not be confirmed: {}",
                    short_error(&error)
                ))),
            }
        }
        self.dispatch(api);
    }

    fn dispatch(&mut self, api: &mut HttpClient) {
        if self.pending_refresh.is_none() && self.auth_refresh_timer <= 0.0 {
            if let Some(refresh_token) = self.refresh_token.clone() {
                let request_id = self.next_id();
                self.pending_refresh = Some(api.post_json(
                    "/v1/auth/refresh",
                    &tarrowyn_protocol::AuthRefreshRequest {
                        request_id,
                        refresh_token,
                    },
                ));
                self.auth_refresh_timer = f32::MAX;
            }
        }
        if self.refresh_timer <= 0.0 {
            if self.pending_region.is_none() {
                self.pending_region = Some(api.get("/v1/region"));
            }
            if self.pending_settlements.is_none() {
                self.pending_settlements = Some(api.get("/v1/settlements"));
            }
            if self.pending_market.is_none() {
                self.pending_market = Some(api.get("/v1/market/orders"));
            }
            if self.pending_events.is_none() {
                let cursor = self
                    .events
                    .as_ref()
                    .map(|events| events.cursor)
                    .unwrap_or(0);
                self.pending_events = Some(api.get(&format!("/v1/events/region?since={cursor}")));
            }
            if self.pending_law.is_none() {
                self.pending_law = Some(api.get("/v1/law"));
            }
            if self.pending_account.is_none() {
                self.pending_account = Some(api.get("/v1/account"));
            }
            self.refresh_timer = 1.5;
        }
        if self.pending_command.is_none() {
            if let Some(command) = self.commands.pop_front() {
                self.pending_command = Some(match command {
                    Phase5Command::Travel(request) => api.post_json("/v1/travel", &request),
                    Phase5Command::Route(request) => api.post_json("/v1/routes", &request),
                    Phase5Command::Market(request) => api.post_json("/v1/market/orders", &request),
                    Phase5Command::Event(request) => api.post_json("/v1/events/region", &request),
                    Phase5Command::Link(request) => api.post_json("/v1/auth/link", &request),
                    Phase5Command::Revoke(request) => api.post_json("/v1/auth/revoke", &request),
                    Phase5Command::Report(request) => {
                        api.post_json("/v1/moderation/report", &request)
                    }
                    Phase5Command::Delete(request) => api.post_json("/v1/account/delete", &request),
                });
            }
        }
    }

    pub(super) fn queue_cycle(&mut self, id: &str) {
        let request_id = self.next_id();
        match id {
            "region-map" => self.refresh_timer = 0.0,
            "travel" => self.queue_travel(request_id),
            "recover-travel" => self.queue_travel_action(request_id, TravelAction::Recover),
            "market-region" => self.queue_market(request_id),
            "region-event" => self.queue_event(request_id),
            "account" => self
                .commands
                .push_back(Phase5Command::Link(AuthLinkRequest {
                    request_id,
                    provider: "webhatchery-identity-oidc".to_owned(),
                    subject: self
                        .own_account_id
                        .clone()
                        .unwrap_or_else(|| "guest-subject".to_owned()),
                    display_name: None,
                })),
            "logout" => self.commands.push_back(Phase5Command::Revoke(
                tarrowyn_protocol::AuthRevokeRequest {
                    request_id,
                    revoke_all: true,
                },
            )),
            "report" => self
                .commands
                .push_back(Phase5Command::Report(ModerationReportRequest {
                    request_id,
                    target_account_id: None,
                    message_id: None,
                    category: "player_report".to_owned(),
                    note: "Report submitted from the visible touch control.".to_owned(),
                })),
            "delete-account" => {
                let Some(account) = self
                    .account
                    .as_ref()
                    .filter(|account| !account.guest_fixture)
                else {
                    return;
                };
                if self.deletion_armed {
                    self.deletion_armed = false;
                    self.commands
                        .push_back(Phase5Command::Delete(AccountDeletionRequest {
                            request_id,
                            account_id: account.account_id.clone(),
                        }));
                } else {
                    self.deletion_armed = true;
                }
            }
            "route-repair" => {
                let Some(route_id) = self.region.as_ref().and_then(|region| {
                    region
                        .routes
                        .iter()
                        .find(|route| {
                            route.origin_location_id == region.player_location_id
                                && route.status != tarrowyn_protocol::RouteStatus::Operational
                        })
                        .map(|route| route.route_id.clone())
                }) else {
                    return;
                };
                self.commands.push_back(Phase5Command::Route(RouteRequest {
                    request_id,
                    route_id,
                    action: RouteAction::Repair,
                }));
            }
            _ => {}
        }
    }

    fn queue_travel(&mut self, request_id: String) {
        let Some(region) = self.region.as_ref() else {
            return;
        };
        if let Some(travel) = region.travel.as_ref() {
            let action = if travel.status == TravelStatus::Interrupted {
                TravelAction::Recover
            } else {
                TravelAction::Interrupt
            };
            self.queue_travel_action(request_id, action);
            return;
        }
        let Some(route) = region.routes.iter().find(|route| {
            route.origin_location_id == region.player_location_id
                && route.status != tarrowyn_protocol::RouteStatus::Closed
        }) else {
            return;
        };
        self.commands
            .push_back(Phase5Command::Travel(TravelRequest {
                request_id,
                action: TravelAction::Start,
                route_id: Some(route.route_id.clone()),
                travel_id: None,
            }));
    }

    fn queue_travel_action(&mut self, request_id: String, action: TravelAction) {
        let travel_id = self
            .region
            .as_ref()
            .and_then(|region| region.travel.as_ref())
            .map(|travel| travel.travel_id.clone());
        self.commands
            .push_back(Phase5Command::Travel(TravelRequest {
                request_id,
                action,
                route_id: None,
                travel_id,
            }));
    }

    fn queue_market(&mut self, request_id: String) {
        let Some(region) = self.region.as_ref() else {
            return;
        };
        if let Some(order) = self.market.as_ref().and_then(|market| {
            market
                .orders
                .iter()
                .find(|order| order.status == tarrowyn_protocol::MarketOrderStatus::Open)
        }) {
            self.commands
                .push_back(Phase5Command::Market(MarketOrderRequest {
                    request_id,
                    action: MarketOrderAction::Fulfil,
                    order_id: Some(order.order_id.clone()),
                    destination_location_id: None,
                    commodity: None,
                    quantity: None,
                }));
        } else if region.player_location_id == "hearth" {
            self.commands
                .push_back(Phase5Command::Market(MarketOrderRequest {
                    request_id,
                    action: MarketOrderAction::Create,
                    order_id: None,
                    destination_location_id: Some("saltmere".to_owned()),
                    commodity: Some(tarrowyn_protocol::CommodityKind::Seeds),
                    quantity: Some(1),
                }));
        }
    }

    fn queue_event(&mut self, request_id: String) {
        let event = self.events.as_ref().and_then(|events| {
            events.events.iter().rev().find(|event| {
                !matches!(
                    event.stage,
                    tarrowyn_protocol::RegionalEventStage::Aftermath
                )
            })
        });
        let request = match event {
            None => RegionalEventRequest {
                request_id,
                action: RegionalEventAction::Seed,
                event_id: None,
                intervention: None,
            },
            Some(event)
                if matches!(
                    event.stage,
                    tarrowyn_protocol::RegionalEventStage::Signal
                        | tarrowyn_protocol::RegionalEventStage::Escalation
                ) =>
            {
                RegionalEventRequest {
                    request_id,
                    action: RegionalEventAction::Intervene,
                    event_id: Some(event.event_id.clone()),
                    intervention: Some("repair ferry markers".to_owned()),
                }
            }
            Some(event) => RegionalEventRequest {
                request_id,
                action: RegionalEventAction::Resolve,
                event_id: Some(event.event_id.clone()),
                intervention: None,
            },
        };
        self.commands.push_back(Phase5Command::Event(request));
    }

    fn apply_command(
        &mut self,
        response: Phase5CommandResponse,
        api: &mut HttpClient,
        notices: &mut Vec<NetworkNotice>,
    ) {
        match response {
            Phase5CommandResponse::Travel(response) => phase5_notice(
                response.accepted,
                response.reason,
                "The server recorded the journey and kept recovery available.",
                notices,
            ),
            Phase5CommandResponse::Route(response) => phase5_notice(
                response.accepted,
                response.reason,
                "The route ledger recorded the logistics action.",
                notices,
            ),
            Phase5CommandResponse::Market(response) => phase5_notice(
                response.accepted,
                response.reason,
                "The regional market settled through the authoritative ledger.",
                notices,
            ),
            Phase5CommandResponse::Event(response) => phase5_notice(
                response.accepted,
                response.reason,
                "The regional chronicle recorded the event intervention.",
                notices,
            ),
            Phase5CommandResponse::Link(response) => {
                api.set_bearer_token(Some(&response.session.account_token));
                self.refresh_token = Some(response.session.refresh_token.clone());
                self.auth_refresh_timer = refresh_delay(response.session.expires_in_seconds);
                self.linked_account = Some(response.clone());
                self.account = None;
                notices.push(NetworkNotice::Success(
                    "Account linked; the character boundary and session are now production-ready."
                        .to_owned(),
                ));
            }
            Phase5CommandResponse::Revoke(response) => {
                self.deletion_armed = false;
                self.logged_out = true;
                self.pending_region = None;
                self.pending_settlements = None;
                self.pending_market = None;
                self.pending_events = None;
                self.pending_law = None;
                self.pending_account = None;
                self.pending_refresh = None;
                self.commands.clear();
                self.account = None;
                self.refresh_token = None;
                self.refreshed_session = None;
                self.auth_refresh_timer = f32::MAX;
                notices.push(NetworkNotice::Info(format!(
                    "{} session(s) revoked; tap Reconnect to return safely.",
                    response.revoked_sessions
                )));
            }
            Phase5CommandResponse::Report(response) => phase5_notice(
                response.accepted,
                response.reason,
                "The moderation report is queued with an audit ID.",
                notices,
            ),
            Phase5CommandResponse::Delete(response) => {
                self.deletion_armed = false;
                if response.accepted {
                    self.logged_out = true;
                    self.pending_region = None;
                    self.pending_settlements = None;
                    self.pending_market = None;
                    self.pending_events = None;
                    self.pending_law = None;
                    self.pending_account = None;
                    self.pending_refresh = None;
                    self.commands.clear();
                    self.account = None;
                    self.refresh_token = None;
                    self.refreshed_session = None;
                    self.auth_refresh_timer = f32::MAX;
                    api.set_bearer_token(None);
                    notices.push(NetworkNotice::Success(
                        "Account deletion is scheduled; tap Reconnect to return as a new guest."
                            .to_owned(),
                    ));
                } else {
                    phase5_notice(
                        false,
                        response.reason,
                        "The account deletion request was accepted.",
                        notices,
                    );
                }
            }
        }
        self.refresh_timer = if self.logged_out { f32::MAX } else { 0.0 };
    }

    pub(super) fn clear(&mut self) {
        self.pending_region = None;
        self.pending_settlements = None;
        self.pending_market = None;
        self.pending_events = None;
        self.pending_law = None;
        self.pending_account = None;
        self.pending_refresh = None;
        self.pending_command = None;
        self.commands.clear();
        self.events = None;
        self.account = None;
        self.linked_account = None;
        self.refreshed_session = None;
        self.refresh_token = None;
        self.logged_out = false;
        self.deletion_armed = false;
        self.refresh_timer = 0.0;
        self.auth_refresh_timer = f32::MAX;
    }

    pub(super) fn reset_event_cursor(&mut self) {
        self.pending_events = None;
        self.events = None;
        self.refresh_timer = 0.0;
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

    pub(super) fn season(&self) -> Option<&str> {
        self.region.as_ref().map(|region| region.season.as_str())
    }

    pub(super) fn region_snapshot(&self) -> Option<&RegionSnapshot> {
        self.region.as_ref()
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

impl Phase5Client {
    fn poll_refresh(&mut self, dt: f32, api: &mut HttpClient, notices: &mut Vec<NetworkNotice>) {
        let result = self
            .pending_refresh
            .as_mut()
            .and_then(|pending| pending.poll_timed(dt, REQUEST_TIMEOUT_SECONDS));
        let Some(result) = result else { return };
        self.pending_refresh = None;
        match result {
            Ok(response) => {
                let session = response.data.session;
                api.set_bearer_token(Some(&session.account_token));
                self.refresh_token = Some(session.refresh_token.clone());
                self.auth_refresh_timer = refresh_delay(session.expires_in_seconds);
                self.refreshed_session = Some(session);
                notices.push(NetworkNotice::Success(
                    "The production session was refreshed safely.".to_owned(),
                ));
            }
            Err(error) => {
                self.pending_region = None;
                self.pending_settlements = None;
                self.pending_market = None;
                self.pending_events = None;
                self.pending_law = None;
                self.pending_account = None;
                self.pending_command = None;
                self.commands.clear();
                self.account = None;
                self.refresh_token = None;
                self.auth_refresh_timer = f32::MAX;
                self.refresh_timer = f32::MAX;
                self.deletion_armed = false;
                self.logged_out = true;
                notices.push(NetworkNotice::Warning(format!(
                    "The production session could not be refreshed: {}; provider sign-in is required.",
                    short_error(&error)
                )));
            }
        }
    }
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
    fn poll_events(&mut self, dt: f32, notices: &mut Vec<NetworkNotice>) {
        let result = self
            .pending_events
            .as_mut()
            .and_then(|pending| pending.poll_timed(dt, REQUEST_TIMEOUT_SECONDS));
        let Some(result) = result else { return };
        self.pending_events = None;
        match result {
            Ok(response) => merge_regional_events(&mut self.events, response.data),
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
}

fn phase5_notice(
    accepted: bool,
    reason: Option<String>,
    success: &str,
    notices: &mut Vec<NetworkNotice>,
) {
    if accepted {
        notices.push(NetworkNotice::Success(success.to_owned()));
    } else if let Some(reason) = reason {
        notices.push(NetworkNotice::Warning(reason));
    }
}

#[cfg(test)]
mod tests;

impl OnlineClient {
    pub(crate) fn queue_phase5(&mut self, id: &str) {
        if self.state == super::ConnectionState::Online {
            self.phase4.queue_region_cycle(id);
        }
    }
    pub(crate) fn phase5_summary(&self) -> String {
        self.phase4.region_summary()
    }

    pub(crate) fn phase5_season(&self) -> Option<&str> {
        self.phase4.regional_season()
    }

    pub(crate) fn phase5_region(&self) -> Option<&RegionSnapshot> {
        self.phase4.regional_region()
    }

    pub(crate) fn account_deletion_armed(&self) -> bool {
        self.phase4.deletion_armed()
    }
}
