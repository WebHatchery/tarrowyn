use macroquad_toolkit::net::Pending;
use tarrowyn_protocol::{ApiResponse, ChatResponse, FarmingResponse, MovementResponse};

pub(super) struct PendingMovement {
    pub(super) pending: Pending<ApiResponse<MovementResponse>>,
}

pub(super) struct PendingChat {
    pub(super) pending: Pending<ApiResponse<ChatResponse>>,
}

pub(super) struct PendingFarming {
    pub(super) pending: Pending<ApiResponse<FarmingResponse>>,
}
