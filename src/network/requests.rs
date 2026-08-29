use macroquad_toolkit::net::Pending;
use tarrowyn_protocol::{
    ApiResponse, ChatRequest, ChatResponse, FarmingRequest, FarmingResponse, MovementIntent,
    MovementResponse, TradeRequest, TradeResponse,
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

pub(super) struct PendingTrade {
    pub(crate) pending: Option<Pending<ApiResponse<TradeResponse>>>,
    pub(crate) request: TradeRequest,
    pub(crate) retries: u8,
    pub(crate) retry_timer: f32,
}
