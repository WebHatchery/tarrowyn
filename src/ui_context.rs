use crate::data::GameData;
use crate::network::{ConnectionState, CraftingView, RemotePlayer};
use crate::state::WorldState;
use macroquad_toolkit::grid::TilePos;
use macroquad_toolkit::ui::VirtualUi;
use tarrowyn_protocol::{
    ChatMessage, ChronicleEntry, ChronicleSummary, LocalCombatState, OpportunitySignal,
    RegionSnapshot, TimeOfDay, TradeOffer, WildernessZone,
};

#[derive(Debug, Clone, PartialEq)]
pub enum UiAction {
    NewEvening,
    UseOnline,
    UseOffline,
    Reconnect,
    Save,
    Load,
    DeleteSave,
    Move(i32, i32),
    MoveTo(TilePos),
    Interact(String),
    SendChat,
    QuickChat(String),
    Zoom(f32),
}

pub struct UiContext<'a> {
    pub data: &'a GameData,
    pub world: &'a WorldState,
    pub player_position: TilePos,
    pub day: u32,
    pub calendar_season: Option<&'a str>,
    pub clock_minutes: u32,
    pub time_of_day: TimeOfDay,
    pub night: bool,
    pub stats: &'a str,
    pub own_account_id: Option<&'a str>,
    pub remote_players: &'a [RemotePlayer],
    pub farm_animals: &'a [tarrowyn_protocol::FarmAnimal],
    pub trades: &'a [TradeOffer],
    pub chat: &'a [ChatMessage],
    pub chat_draft: &'a str,
    pub server_tick: u64,
    pub connection: ConnectionState,
    pub status_message: &'a str,
    pub identity_name: Option<&'a str>,
    pub offline: bool,
    pub save_exists: bool,
    pub save_slots: &'a [String],
    pub loaded_assets: usize,
    pub camera_zoom: f32,
    pub wilderness: Option<&'a WildernessZone>,
    pub regional_region: Option<&'a RegionSnapshot>,
    pub regional_inspection: Option<&'a str>,
    pub chronicle: &'a [ChronicleEntry],
    pub chronicle_summary: Option<&'a ChronicleSummary>,
    pub opportunities: &'a [OpportunitySignal],
    pub phase4_summary: &'a str,
    pub phase5_summary: &'a str,
    pub account_deletion_armed: bool,
    pub crafting: Option<CraftingView>,
    pub combat: Option<&'a LocalCombatState>,
    pub storm_magic_unlocked: bool,
    pub knocked_out: bool,
    pub has_open_market_order: bool,
    pub can_abandon_claim: bool,
    pub can_transfer_claim: bool,
    pub knowledge_label: &'a str,
    pub ui: &'a VirtualUi,
}
