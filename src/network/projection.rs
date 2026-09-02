//! Authoritative world and event projection applied by the online client.

use super::*;
use std::collections::HashSet;

impl WorldProjection {
    pub fn new(config: &GameConfig) -> Self {
        Self {
            world: WorldState::new(config),
            player_position: TilePos::new(8, 6),
            player_position_authoritative: false,
            players: Vec::new(),
            chat: Vec::new(),
            day: 1,
            day_seconds: 0.0,
            day_length_seconds: config.day_length_seconds,
            server_tick: 0,
            cursor: 0,
            player: None,
            animals: Vec::new(),
            feed: TavernFeedResponse {
                notices: Vec::new(),
                rumours: Vec::new(),
                chat: Vec::new(),
                cursor: 0,
            },
            trades: Vec::new(),
            wilderness: None,
            chronicle: Vec::new(),
            chronicle_summary: None,
            chronicle_search: Vec::new(),
            chronicle_search_summary: None,
            chronicle_search_query: None,
            chronicle_search_next_cursor: None,
            opportunities: Vec::new(),
            claim: None,
            outpost: None,
            expedition: None,
            expedition_requirements: ExpeditionRequirements::default(),
            foundation: FoundationBaseline::default(),
            foundation_activity: FoundationActivityState::default(),
            journey: None,
            property: property_projection_default(),
        }
    }

    pub fn clock_minutes(&self) -> u32 {
        let fraction = (self.day_seconds / self.day_length_seconds.max(1.0)).clamp(0.0, 1.0);
        (((6.0 + fraction * 24.0) % 24.0) * 60.0) as u32
    }

    pub fn time_of_day(&self) -> TimeOfDay {
        TimeOfDay::from_clock_minutes(self.clock_minutes())
    }

    pub fn is_night(&self) -> bool {
        self.time_of_day().is_night()
    }

    fn apply_snapshot(
        &mut self,
        snapshot: WorldSnapshot,
        own_account: &str,
        server_tick: u64,
    ) -> bool {
        let Some(width) = usize::try_from(snapshot.width).ok() else {
            return false;
        };
        let Some(height) = usize::try_from(snapshot.height).ok() else {
            return false;
        };
        let Some(tile_count) = width.checked_mul(height) else {
            return false;
        };
        if width == 0 || height == 0 || tile_count != snapshot.tiles.len() {
            return false;
        }
        let mut seen_positions = HashSet::new();
        for tile in &snapshot.tiles {
            let Some(x) = usize::try_from(tile.position.x).ok() else {
                return false;
            };
            let Some(y) = usize::try_from(tile.position.y).ok() else {
                return false;
            };
            if x >= width || y >= height || !seen_positions.insert((x, y)) {
                return false;
            }
        }
        let mut tiles = FlatGrid::new(width, height, TileKind::Meadow);
        for tile in snapshot.tiles {
            let position = TilePos::new(tile.position.x, tile.position.y);
            tiles.set(position, from_protocol_tile(tile.kind));
        }
        self.world = WorldState {
            tiles,
            crops: FlatGrid::new(snapshot.width as usize, snapshot.height as usize, None),
            reachable: Default::default(),
        };
        self.apply_plots(&snapshot.plots);
        self.animals = snapshot.animals;
        self.apply_clock(snapshot.clock);
        self.server_tick = server_tick;
        self.cursor = snapshot.cursor;
        self.players = snapshot.players.into_iter().map(remote_player).collect();
        self.wilderness = snapshot.wilderness;
        self.outpost = snapshot
            .outpost
            .map(|position| TilePos::new(position.x, position.y));
        self.claim = snapshot.claim;
        self.expedition = snapshot.expedition;
        self.expedition_requirements = snapshot.expedition_requirements;
        self.foundation = snapshot.foundation;
        self.foundation_activity = snapshot.foundation_activity;
        if let Some(player) = self
            .players
            .iter()
            .find(|player| player.account_id == own_account)
        {
            self.set_authoritative_player_position(player.position);
        }
        true
    }

    pub(super) fn apply_state(&mut self, snapshot: StateSnapshot, server_tick: u64) -> bool {
        let position = snapshot.player.position;
        if !self.apply_snapshot(snapshot.world, &snapshot.player.account_id, server_tick) {
            return false;
        }
        self.player = Some(snapshot.player);
        self.set_authoritative_player_position(TilePos::new(position.x, position.y));
        self.feed = snapshot.feed;
        self.chat = self.feed.chat.clone();
        true
    }

    pub(super) fn apply_events(
        &mut self,
        response: EventsResponse,
        own_account: &str,
        server_tick: u64,
    ) {
        self.server_tick = self.server_tick.max(server_tick);
        self.apply_clock(response.clock);
        for record in response.events {
            if record.cursor <= self.cursor {
                continue;
            }
            self.cursor = self.cursor.max(record.cursor);
            match record.event {
                WorldEvent::Presence(presence) => self.apply_presence(presence, own_account),
                WorldEvent::Clock(clock) => self.apply_clock(clock),
                WorldEvent::Chat(message) => self.push_chat(message),
                WorldEvent::Farming(plot) => self.apply_plot(plot),
                WorldEvent::Trade(_) => {}
                WorldEvent::TavernNotice(notice) => {
                    self.feed.notices.push(notice);
                    if self.feed.notices.len() > 8 {
                        self.feed.notices.remove(0);
                    }
                }
                WorldEvent::Chronicle(entry) => {
                    merge_chronicle_entry(&mut self.chronicle, entry);
                }
                WorldEvent::Frontier(event) => match event {
                    FrontierEvent::Threat(zone) => self.wilderness = Some(zone),
                    FrontierEvent::Opportunity(opportunity) => {
                        self.opportunities
                            .retain(|existing| existing.household_id != opportunity.household_id);
                        self.opportunities.push(opportunity);
                    }
                    FrontierEvent::Claim(claim) => self.claim = Some(claim),
                    FrontierEvent::Expedition(expedition) => {
                        self.expedition = Some(expedition.clone());
                        self.outpost = (expedition.status
                            == tarrowyn_protocol::ExpeditionStatus::Succeeded)
                            .then_some(TilePos::new(
                                expedition.outpost_position.x,
                                expedition.outpost_position.y,
                            ));
                    }
                },
            }
        }
        self.cursor = self.cursor.max(response.cursor);
    }

    pub(super) fn apply_presence(&mut self, presence: PlayerPresence, own_account: &str) {
        let remote = remote_player(presence);
        if remote.account_id == own_account {
            if remote.online {
                self.set_authoritative_player_position(remote.position);
            } else {
                self.forget_authoritative_player_position();
            }
        }
        if let Some(existing) = self
            .players
            .iter_mut()
            .find(|player| player.character_id == remote.character_id)
        {
            *existing = remote;
        } else {
            self.players.push(remote);
        }
    }

    fn apply_clock(&mut self, clock: WorldClock) {
        self.day = clock.day;
        self.day_seconds = clock.seconds;
        self.day_length_seconds = clock.day_length_seconds;
    }

    pub(super) fn push_chat(&mut self, message: ChatMessage) {
        if self
            .chat
            .iter()
            .any(|existing| existing.message_id == message.message_id)
        {
            return;
        }
        self.chat.push(message);
        self.chat.sort_by_key(|message| message.cursor);
        if self.chat.len() > 8 {
            let keep_from = self.chat.len() - 8;
            self.chat.drain(0..keep_from);
        }
    }
}
