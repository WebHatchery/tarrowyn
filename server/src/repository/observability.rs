use super::models::RepositoryState;
use super::{phase3, world, ServerConfig, MAX_EVENTS, MAX_NOTICES};
use std::collections::VecDeque;
use tarrowyn_protocol::{
    ApiMeta, EventRecord, ExpeditionRequirements, PlayerPresence, Position, TavernFeedResponse,
    TavernNotice, WorldEvent, WorldSnapshot,
};

pub(super) fn snapshot(
    state: &RepositoryState,
    config: &ServerConfig,
    players: Vec<PlayerPresence>,
) -> WorldSnapshot {
    WorldSnapshot {
        width: config.world_width,
        height: config.world_height,
        tiles: world::world_tiles(config.world_width, config.world_height),
        clock: state.clock.clone(),
        players,
        plots: state.plots.clone(),
        animals: state.phase4.animals.clone(),
        tavern_position: Position { x: 8, y: 5 },
        cursor: state.cursor,
        wilderness: Some(state.phase3.zone.clone()),
        outpost: state.phase3.outpost,
        claim: state.phase3.claim.clone(),
        expedition: state.phase3.expedition.clone(),
        expedition_requirements: ExpeditionRequirements {
            food: config.expedition_min_food,
            tools: config.expedition_min_tools,
            materials: config.expedition_min_materials,
            safety: config.expedition_min_safety,
        },
        foundation: crate::content::foundation_baseline(),
    }
}

pub(super) fn feed(state: &RepositoryState) -> TavernFeedResponse {
    TavernFeedResponse {
        notices: state.notices.iter().cloned().collect(),
        rumours: phase3::rumours(&state.phase3),
        chat: state.chat_history.iter().cloned().collect(),
        cursor: state.cursor,
    }
}

pub(super) fn meta(tick: u64, request_id: Option<String>, cursor: Option<u64>) -> ApiMeta {
    let mut meta = ApiMeta::at(tick);
    meta.request_id = request_id;
    meta.cursor = cursor;
    meta
}

pub(super) fn push_event(state: &mut RepositoryState, event: WorldEvent) -> u64 {
    state.cursor = state.cursor.saturating_add(1);
    state.events.push_back(EventRecord {
        cursor: state.cursor,
        event,
    });
    trim_back(&mut state.events, MAX_EVENTS);
    state.cursor
}

pub(super) fn add_notice(state: &mut RepositoryState, kind: &str, text: &str) {
    let id = state.next_notice;
    state.next_notice = state.next_notice.saturating_add(1);
    let mut notice = TavernNotice {
        notice_id: id,
        kind: kind.to_owned(),
        text: text.to_owned(),
        created_tick: state.tick,
        cursor: 0,
    };
    let cursor = push_event(state, WorldEvent::TavernNotice(notice.clone()));
    notice.cursor = cursor;
    if let Some(EventRecord {
        event: WorldEvent::TavernNotice(stored),
        ..
    }) = state.events.back_mut()
    {
        *stored = notice.clone();
    }
    state.notices.push_back(notice);
    trim_back(&mut state.notices, MAX_NOTICES);
}

pub(super) fn trim_back<T>(queue: &mut VecDeque<T>, max: usize) {
    while queue.len() > max {
        queue.pop_front();
    }
}

pub(super) fn trim_queue<T>(mut queue: VecDeque<T>, max: usize) -> VecDeque<T> {
    trim_back(&mut queue, max);
    queue
}
