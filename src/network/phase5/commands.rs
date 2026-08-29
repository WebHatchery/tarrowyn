use serde::Deserialize;
use tarrowyn_protocol::{
    AccountDeletionRequest, AccountDeletionResponse, AuthLinkRequest, AuthLinkResponse,
    AuthRevokeRequest, AuthRevokeResponse, MarketOrderRequest, MarketOrderResponse,
    ModerationReportRequest, ModerationReportResponse, RegionalEventRequest, RegionalEventResponse,
    RouteRequest, RouteResponse, TravelRequest, TravelResponse,
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
