use macroquad_toolkit::grid::TilePos;

pub(super) const STALE_TICKS: u64 = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Connecting,
    Online,
    Degraded,
    Offline,
}

impl ConnectionState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Connecting => "CONNECTING",
            Self::Online => "ONLINE",
            Self::Degraded => "DEGRADED",
            Self::Offline => "OFFLINE",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetworkNotice {
    Info(String),
    Success(String),
    Warning(String),
    Danger(String),
}

pub struct RemotePlayer {
    pub account_id: String,
    pub character_id: String,
    pub display_name: String,
    pub position: TilePos,
    pub last_seen_tick: u64,
    pub online: bool,
}

impl RemotePlayer {
    pub fn stale(&self, server_tick: u64) -> bool {
        !self.online || server_tick.saturating_sub(self.last_seen_tick) > STALE_TICKS
    }
}
