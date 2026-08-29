use macroquad_toolkit::net::Pending;
use tarrowyn_protocol::{
    ApiResponse, ChatRequest, ChatResponse, FarmingRequest, FarmingResponse, MovementIntent,
    MovementResponse, TradeRequest, TradeResponse,
};

pub(super) struct PendingMovement {
    pub(crate) pending: Pending<ApiResponse<MovementResponse>>,
    pub(crate) request: MovementIntent,
    pub(crate) retries: u8,
}

pub(super) struct PendingChat {
    pub(crate) pending: Pending<ApiResponse<ChatResponse>>,
    pub(crate) request: ChatRequest,
    pub(crate) retries: u8,
}

pub(super) struct PendingFarming {
    pub(crate) pending: Pending<ApiResponse<FarmingResponse>>,
    pub(crate) request: FarmingRequest,
    pub(crate) retries: u8,
}

pub(super) struct PendingTrade {
    pub(crate) pending: Pending<ApiResponse<TradeResponse>>,
    pub(crate) request: TradeRequest,
    pub(crate) retries: u8,
}
