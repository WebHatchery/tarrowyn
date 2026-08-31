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
