use super::*;
use serde::Deserialize;
use tarrowyn_protocol::{
    AccountDeletionRequest, AccountDeletionResponse, AuthLinkRequest, AuthLinkResponse,
    AuthRevokeRequest, AuthRevokeResponse, MarketOrderRequest, MarketOrderResponse,
    ModerationReportRequest, ModerationReportResponse, RegionalEvent, RegionalEventRequest,
    RegionalEventResponse, RegionalEventStage, RouteRecord, RouteRequest, RouteResponse,
    RouteStatus, TravelRequest, TravelResponse, TravelState, TravelStatus,
};

pub(super) enum Phase5Command {
    Travel(TravelRequest),
    Route(RouteRequest),
    Market(MarketOrderRequest),
    Event(RegionalEventRequest),
    Link(AuthLinkRequest),
    Revoke(AuthRevokeRequest),
    Report(ModerationReportRequest),
    Delete(AccountDeletionRequest),
}

pub(super) fn travel_success_message(travel: Option<&TravelState>, location_id: &str) -> String {
    let Some(travel) = travel else {
        return format!("The journey ledger is ready at {location_id}.");
    };
    match travel.status {
        TravelStatus::Travelling => format!(
            "Journey underway to {} • {}% complete • {}% risk.",
            travel.destination_location_id, travel.progress, travel.risk_percent
        ),
        TravelStatus::Interrupted => format!(
            "Journey interrupted before {}; tap Recover to continue.",
            travel.destination_location_id
        ),
        TravelStatus::Recovering => format!(
            "Journey recovery is underway toward {} • {}% complete.",
            travel.destination_location_id, travel.progress
        ),
        TravelStatus::Arrived => format!("Arrived at {}.", location_id),
        TravelStatus::Idle => format!("The journey ledger is ready at {location_id}."),
    }
}

pub(super) fn route_success_message(route: &RouteRecord) -> String {
    let status = match route.status {
        RouteStatus::Operational => "open",
        RouteStatus::Delayed => "delayed",
        RouteStatus::Threatened => "threatened",
        RouteStatus::Repairing => "under repair",
        RouteStatus::Closed => "closed",
    };
    format!(
        "{} is {status} • condition {} • {}% risk.",
        route.name, route.condition, route.risk_percent
    )
}

pub(super) fn event_success_message(event: Option<&RegionalEvent>) -> String {
    let Some(event) = event else {
        return "The regional event ledger updated.".to_owned();
    };
    match event.stage {
        RegionalEventStage::Signal => {
            format!("{} is now signalled across the region.", event.title)
        }
        RegionalEventStage::Escalation => {
            format!(
                "{} has escalated; choose a visible intervention.",
                event.title
            )
        }
        RegionalEventStage::Intervention => format!(
            "{} recorded: {}.",
            event.title,
            event
                .chosen_intervention
                .as_deref()
                .unwrap_or("the selected response")
        ),
        RegionalEventStage::Resolution => format!(
            "{} resolved: {}.",
            event.title,
            event
                .outcome
                .as_deref()
                .unwrap_or("the outcome is recorded")
                .trim_end_matches('.')
        ),
        RegionalEventStage::Aftermath => format!("{} aftermath is recorded.", event.title),
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
pub(super) enum Phase5CommandResponse {
    Travel(TravelResponse),
    Route(RouteResponse),
    Delete(AccountDeletionResponse),
    Market(MarketOrderResponse),
    Event(RegionalEventResponse),
    Link(AuthLinkResponse),
    Revoke(AuthRevokeResponse),
    Report(ModerationReportResponse),
}

impl Phase5Client {
    pub(super) fn apply_command(
        &mut self,
        response: Phase5CommandResponse,
        market_action: Option<MarketOrderAction>,
        api: &mut HttpClient,
        notices: &mut Vec<NetworkNotice>,
    ) {
        match response {
            Phase5CommandResponse::Travel(response) => {
                let message =
                    travel_success_message(response.travel.as_ref(), &response.location_id);
                phase5_notice(response.accepted, response.reason, &message, notices);
            }
            Phase5CommandResponse::Route(response) => {
                let message = route_success_message(&response.route);
                phase5_notice(response.accepted, response.reason, &message, notices);
            }
            Phase5CommandResponse::Market(response) => phase5_notice(
                response.accepted,
                response.reason,
                &market_result_message(
                    market_action,
                    response
                        .order
                        .as_ref()
                        .is_some_and(|order| order.fallback_used),
                    response.order.as_ref(),
                ),
                notices,
            ),
            Phase5CommandResponse::Event(response) => {
                let message = event_success_message(response.event.as_ref());
                phase5_notice(response.accepted, response.reason, &message, notices);
            }
            Phase5CommandResponse::Link(response) => {
                api.set_bearer_token(Some(&response.session.account_token));
                self.refresh_token = Some(response.session.refresh_token.clone());
                self.auth_refresh_timer = refresh_delay(response.session.expires_in_seconds);
                self.in_flight_refresh = None;
                self.refresh_retry_timer = 0.0;
                self.refresh_retry_count = 0;
                self.linked_account = Some(response.clone());
                self.account = None;
                notices.push(NetworkNotice::Success(
                    "Account linked; the character boundary and session are now production-ready."
                        .to_owned(),
                ));
            }
            Phase5CommandResponse::Revoke(response) => {
                api.set_bearer_token(None);
                self.deletion_armed = false;
                self.logged_out = true;
                self.pending_region = None;
                self.pending_settlements = None;
                self.pending_households = None;
                self.pending_market = None;
                self.pending_events = None;
                self.pending_law = None;
                self.pending_account = None;
                self.pending_refresh = None;
                self.in_flight_refresh = None;
                self.commands.clear();
                self.clear_cached_projections();
                self.account = None;
                self.refresh_token = None;
                self.refreshed_session = None;
                self.auth_refresh_timer = f32::MAX;
                self.refresh_retry_timer = 0.0;
                self.refresh_retry_count = 0;
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
                    self.pending_households = None;
                    self.pending_market = None;
                    self.pending_events = None;
                    self.pending_law = None;
                    self.pending_account = None;
                    self.pending_refresh = None;
                    self.in_flight_refresh = None;
                    self.commands.clear();
                    self.clear_cached_projections();
                    self.account = None;
                    self.refresh_token = None;
                    self.refreshed_session = None;
                    self.auth_refresh_timer = f32::MAX;
                    self.refresh_retry_timer = 0.0;
                    self.refresh_retry_count = 0;
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
}
