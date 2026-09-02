use macroquad_toolkit::net::Pending;
use tarrowyn_protocol::{
    ApiResponse, ChatRequest, ChatResponse, FarmingRequest, FarmingResponse,
    FoundationCacheRequest, FoundationCacheResponse, FoundationForgeRequest,
    FoundationForgeResponse, FoundationPropertyRequest, FoundationPropertyResponse,
    FoundationResourceRequest, FoundationResourceResponse, FoundationStorehouseRequest,
    FoundationStorehouseResponse, MovementIntent, MovementResponse, TradeRequest, TradeResponse,
};

pub(super) struct PendingMovement {
    pub(crate) pending: Option<Pending<ApiResponse<MovementResponse>>>,
    pub(crate) request: MovementIntent,
    pub(crate) retries: u8,
    pub(crate) retry_timer: f32,
}

pub(super) struct PendingChat {
    pub(crate) pending: Option<Pending<ApiResponse<ChatResponse>>>,
    pub(crate) request: ChatRequest,
    pub(crate) retries: u8,
    pub(crate) retry_timer: f32,
}

pub(super) struct PendingFarming {
    pub(crate) pending: Option<Pending<ApiResponse<FarmingResponse>>>,
    pub(crate) request: FarmingRequest,
    pub(crate) retries: u8,
    pub(crate) retry_timer: f32,
}

pub(super) struct PendingFoundationResource {
    pub(crate) pending: Option<Pending<ApiResponse<FoundationResourceResponse>>>,
    pub(crate) request: FoundationResourceRequest,
    pub(crate) retries: u8,
    pub(crate) retry_timer: f32,
}

pub(super) struct PendingFoundationCache {
    pub(crate) pending: Option<Pending<ApiResponse<FoundationCacheResponse>>>,
    pub(crate) request: FoundationCacheRequest,
    pub(crate) retries: u8,
    pub(crate) retry_timer: f32,
}

pub(super) struct PendingFoundationForge {
    pub(crate) pending: Option<Pending<ApiResponse<FoundationForgeResponse>>>,
    pub(crate) request: FoundationForgeRequest,
    pub(crate) retries: u8,
    pub(crate) retry_timer: f32,
}

pub(super) struct PendingFoundationStorehouse {
    pub(crate) pending: Option<Pending<ApiResponse<FoundationStorehouseResponse>>>,
    pub(crate) request: FoundationStorehouseRequest,
    pub(crate) retries: u8,
    pub(crate) retry_timer: f32,
}

pub(super) struct PendingFoundationProperty {
    pub(crate) pending: Option<Pending<ApiResponse<FoundationPropertyResponse>>>,
    pub(crate) request: FoundationPropertyRequest,
    pub(crate) retries: u8,
    pub(crate) retry_timer: f32,
}

pub(super) struct PendingTrade {
    pub(crate) pending: Option<Pending<ApiResponse<TradeResponse>>>,
    pub(crate) request: TradeRequest,
    pub(crate) retries: u8,
    pub(crate) retry_timer: f32,
}
