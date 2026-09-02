use super::*;

#[test]
fn projection_adopts_server_tiles_without_local_movement_validation() {
    let mut projection = WorldProjection::new(&config());
    projection.world.tiles = FlatGrid::new(3, 2, TileKind::Meadow);
    let tile = WorldTile {
        position: Position { x: 1, y: 0 },
        kind: ProtocolTileKind::Water,
    };
    projection.world.tiles.set(
        TilePos::new(tile.position.x, tile.position.y),
        from_protocol_tile(tile.kind),
    );
    assert_eq!(
        projection.world.tiles.get(TilePos::new(1, 0)),
        Some(&TileKind::Water)
    );
}

#[test]
fn projection_exposes_the_server_clock_period_without_local_timekeeping() {
    let mut projection = WorldProjection::new(&config());
    projection.day_seconds = 45.0;

    assert_eq!(projection.clock_minutes(), 12 * 60);
    assert_eq!(
        projection.time_of_day(),
        tarrowyn_protocol::TimeOfDay::Afternoon
    );
    assert!(!projection.is_night());
}

#[test]
fn chronicle_cache_keeps_the_newest_entries_after_incremental_events() {
    let mut projection = WorldProjection::new(&config());
    let events = (0..(super::chronicle::MAX_CACHED_CHRONICLE + 1))
        .map(|index| {
            let cursor = (index + 1) as u64;
            EventRecord {
                cursor,
                event: WorldEvent::Chronicle(ChronicleEntry {
                    event_id: format!("chronicle-{index}"),
                    kind: "settlement".to_owned(),
                    title: format!("Settlement record {index}"),
                    text: "The latest work remains visible.".to_owned(),
                    created_tick: cursor,
                    cursor,
                }),
            }
        })
        .collect();
    projection.apply_events(
        EventsResponse {
            cursor: super::chronicle::MAX_CACHED_CHRONICLE as u64 + 1,
            clock: WorldClock {
                day: 1,
                seconds: 0.0,
                day_length_seconds: 180.0,
            },
            events,
        },
        "account",
        1,
    );

    assert_eq!(
        projection.chronicle.len(),
        super::chronicle::MAX_CACHED_CHRONICLE
    );
    assert_eq!(projection.chronicle[0].event_id, "chronicle-1");
    assert_eq!(
        projection.chronicle.last().unwrap().event_id,
        "chronicle-12"
    );
}

#[test]
fn stale_presence_is_visible_after_server_ticks_age() {
    let player = RemotePlayer {
        account_id: "a".to_owned(),
        character_id: "c".to_owned(),
        display_name: "Guest".to_owned(),
        position: TilePos::new(0, 0),
        last_seen_tick: 1,
        online: true,
    };
    assert!(player.stale(super::types::STALE_TICKS + 2));
    assert!(!player.stale(super::types::STALE_TICKS));
}

#[test]
fn projection_versions_accept_equal_state_but_reject_older_responses() {
    let mut projection = WorldProjection::new(&config());
    projection.record_response_version(12, Some(9));

    assert!(!projection.response_is_current(11, 8));
    assert!(projection.response_is_current(12, 9));
    assert!(projection.response_is_newer(12, 10));
    assert!(!projection.response_is_newer(12, 9));
    assert!(!projection.accept_response_version(11, Some(9)));
    assert_eq!(projection.server_tick, 12);
    assert_eq!(projection.cursor, 9);
    assert!(projection.accept_response_version(12, Some(9)));
}

#[test]
fn stale_events_in_a_mixed_response_cannot_overwrite_newer_projection() {
    let mut projection = WorldProjection::new(&config());
    projection.apply_presence(
        tarrowyn_protocol::PlayerPresence {
            account_id: "account".to_owned(),
            character_id: "character".to_owned(),
            display_name: "Traveller".to_owned(),
            position: Position { x: 8, y: 6 },
            last_seen_tick: 5,
            online: true,
        },
        "account",
    );
    projection.record_response_version(5, Some(5));

    projection.apply_events(
        EventsResponse {
            cursor: 6,
            clock: WorldClock {
                day: 1,
                seconds: 1.0,
                day_length_seconds: 180.0,
            },
            events: vec![
                EventRecord {
                    cursor: 4,
                    event: WorldEvent::Presence(tarrowyn_protocol::PlayerPresence {
                        account_id: "account".to_owned(),
                        character_id: "character".to_owned(),
                        display_name: "Traveller".to_owned(),
                        position: Position { x: 1, y: 1 },
                        last_seen_tick: 4,
                        online: true,
                    }),
                },
                EventRecord {
                    cursor: 6,
                    event: WorldEvent::Chat(ChatMessage {
                        message_id: 1,
                        account_id: "account".to_owned(),
                        display_name: "Traveller".to_owned(),
                        channel: "settlement".to_owned(),
                        text: "The newer record remains.".to_owned(),
                        cursor: 6,
                    }),
                },
            ],
        },
        "account",
        6,
    );

    assert_eq!(
        projection.authoritative_player_position(),
        Some(TilePos::new(8, 6))
    );
    assert_eq!(projection.cursor, 6);
    assert_eq!(projection.chat.len(), 1);
}

#[test]
fn older_state_snapshot_is_reloaded_instead_of_opening_the_world() {
    let mut projection = WorldProjection::new(&config());
    projection.record_response_version(12, Some(9));

    assert_eq!(
        state_snapshot_disposition(&projection, 11, 8),
        StateSnapshotDisposition::Reload
    );
    assert_eq!(
        state_snapshot_disposition(&projection, 12, 9),
        StateSnapshotDisposition::Apply
    );
}

#[test]
fn oversized_state_snapshot_is_rejected_before_grid_allocation() {
    let mut projection = WorldProjection::new(&config());
    let snapshot = StateSnapshot {
        world: WorldSnapshot {
            width: u32::MAX,
            height: u32::MAX,
            tiles: Vec::new(),
            clock: WorldClock {
                day: 1,
                seconds: 0.0,
                day_length_seconds: 180.0,
            },
            players: Vec::new(),
            plots: Vec::new(),
            animals: Vec::new(),
            tavern_position: Position { x: 8, y: 5 },
            cursor: 1,
            wilderness: None,
            outpost: None,
            claim: None,
            expedition: None,
            expedition_requirements: ExpeditionRequirements::default(),
            foundation: tarrowyn_protocol::FoundationBaseline::default(),
        },
        player: PlayerProjection {
            account_id: "account".to_owned(),
            character_id: "character".to_owned(),
            display_name: "Traveller".to_owned(),
            position: Position { x: 8, y: 6 },
            gold: 12,
            field_tool_condition: 3,
            field_weather: tarrowyn_protocol::FieldWeather::Clear,
            field_pest_pressure: 0,
            animal_condition: 10,
            animal_max_condition: 10,
            skill: 1,
            reputation: 0,
            adventurer_rank: tarrowyn_protocol::AdventurerRank::Unproven,
            adventurer_credentials: Vec::new(),
            inventory: tarrowyn_protocol::Inventory::default(),
            weapon: tarrowyn_protocol::WeaponKind::IronSword,
            knocked_out: false,
            injuries: 0,
            recovery_cost: 0,
        },
        feed: TavernFeedResponse {
            notices: Vec::new(),
            rumours: Vec::new(),
            chat: Vec::new(),
            cursor: 0,
        },
        cursor: 1,
    };

    assert!(!projection.apply_state(snapshot, 1));
    assert_eq!(projection.world.tiles.width, config().world_width);
    assert_eq!(projection.world.tiles.height, config().world_height);
    assert_eq!(projection.cursor, 0);
    assert!(projection.player.is_none());
}

fn complete_test_tiles() -> Vec<WorldTile> {
    (0..2)
        .flat_map(|y| {
            (0..3).map(move |x| WorldTile {
                position: Position { x, y },
                kind: ProtocolTileKind::Meadow,
            })
        })
        .collect()
}

fn small_state_snapshot(tiles: Vec<WorldTile>) -> StateSnapshot {
    StateSnapshot {
        world: WorldSnapshot {
            width: 3,
            height: 2,
            tiles,
            clock: WorldClock {
                day: 1,
                seconds: 0.0,
                day_length_seconds: 180.0,
            },
            players: Vec::new(),
            plots: Vec::new(),
            animals: Vec::new(),
            tavern_position: Position { x: 1, y: 1 },
            cursor: 1,
            wilderness: None,
            outpost: None,
            claim: None,
            expedition: None,
            expedition_requirements: ExpeditionRequirements::default(),
            foundation: tarrowyn_protocol::FoundationBaseline::default(),
        },
        player: PlayerProjection {
            account_id: "account".to_owned(),
            character_id: "character".to_owned(),
            display_name: "Traveller".to_owned(),
            position: Position { x: 1, y: 1 },
            gold: 12,
            field_tool_condition: 3,
            field_weather: tarrowyn_protocol::FieldWeather::Clear,
            field_pest_pressure: 0,
            animal_condition: 10,
            animal_max_condition: 10,
            skill: 1,
            reputation: 0,
            adventurer_rank: tarrowyn_protocol::AdventurerRank::Unproven,
            adventurer_credentials: Vec::new(),
            inventory: tarrowyn_protocol::Inventory::default(),
            weapon: tarrowyn_protocol::WeaponKind::IronSword,
            knocked_out: false,
            injuries: 0,
            recovery_cost: 0,
        },
        feed: TavernFeedResponse {
            notices: Vec::new(),
            rumours: Vec::new(),
            chat: Vec::new(),
            cursor: 0,
        },
        cursor: 1,
    }
}

#[test]
fn foundation_baseline_is_retained_from_the_authoritative_snapshot() {
    let mut projection = WorldProjection::new(&config());
    let mut snapshot = small_state_snapshot(complete_test_tiles());
    snapshot.world.foundation.fixture_id = "first-beacon-baseline-v1".to_owned();
    snapshot.world.foundation.schema_version = 1;
    snapshot.world.foundation.settlement_id = "hearth-settlement".to_owned();

    assert!(projection.apply_state(snapshot, 1));
    assert_eq!(projection.foundation.fixture_id, "first-beacon-baseline-v1");
    assert_eq!(projection.foundation.schema_version, 1);
    assert_eq!(projection.foundation.settlement_id, "hearth-settlement");
}

#[test]
fn malformed_tile_coordinates_cannot_partially_replace_projection() {
    let mut out_of_bounds = complete_test_tiles();
    out_of_bounds[5].position = Position { x: 3, y: 1 };
    let mut duplicate = complete_test_tiles();
    duplicate[5].position = duplicate[0].position;

    for tiles in [out_of_bounds, duplicate] {
        let mut projection = WorldProjection::new(&config());
        assert!(!projection.apply_state(small_state_snapshot(tiles), 1));
        assert_eq!(projection.world.tiles.width, config().world_width);
        assert_eq!(projection.world.tiles.height, config().world_height);
        assert_eq!(projection.cursor, 0);
        assert!(projection.player.is_none());
    }
}
