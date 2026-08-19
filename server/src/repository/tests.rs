use super::*;
use crate::config::ServerConfig;
use tarrowyn_protocol::{ChatRequest, GuestSessionRequest, MovementIntent, Position, TileKind};

fn repo() -> WorldRepository {
    WorldRepository::new(ServerConfig {
        session_ttl_seconds: 5,
        ..ServerConfig::default()
    })
}

fn guest(repo: &WorldRepository, key: &str) -> GuestSessionResponse {
    repo.guest_session(GuestSessionRequest {
        client_key: Some(key.to_owned()),
        reset: false,
    })
    .data
}

#[test]
fn guest_sessions_are_distinct_but_resume_by_client_key() {
    let repo = repo();
    let first = guest(&repo, "one");
    let second = guest(&repo, "two");
    let resumed = guest(&repo, "one");
    assert_ne!(first.character_id, second.character_id);
    assert_eq!(first.character_id, resumed.character_id);
    assert_ne!(first.account_token, resumed.account_token);
}

#[test]
fn movement_is_server_authoritative_and_rejects_water() {
    let repo = repo();
    let session = guest(&repo, "walker");
    let accepted = repo
        .movement(
            &session.account_token,
            MovementIntent {
                request_id: "valid".to_owned(),
                dx: 0,
                dy: 1,
            },
        )
        .unwrap()
        .data;
    assert!(accepted.accepted);
    assert_eq!(accepted.position, Position { x: 8, y: 7 });

    repo.tick();
    let rejected = repo
        .movement(
            &session.account_token,
            MovementIntent {
                request_id: "invalid".to_owned(),
                dx: 8,
                dy: 0,
            },
        )
        .unwrap()
        .data;
    assert!(!rejected.accepted);
    assert_eq!(rejected.position, Position { x: 8, y: 7 });
}

#[test]
fn chat_events_are_ordered_and_clock_ticks_once_for_the_world() {
    let repo = repo();
    let one = guest(&repo, "one");
    let two = guest(&repo, "two");
    let before = repo.world(&one.account_token).unwrap().data.cursor;
    repo.chat(
        &one.account_token,
        ChatRequest {
            request_id: "chat-one".to_owned(),
            channel: "settlement".to_owned(),
            text: "Hello from one".to_owned(),
        },
    )
    .unwrap();
    repo.chat(
        &two.account_token,
        ChatRequest {
            request_id: "chat-two".to_owned(),
            channel: "settlement".to_owned(),
            text: "Hello from two".to_owned(),
        },
    )
    .unwrap();
    let events = repo.events(&one.account_token, before).unwrap().data.events;
    let texts: Vec<&str> = events
        .iter()
        .filter_map(|record| match &record.event {
            WorldEvent::Chat(message) => Some(message.text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(texts, ["Hello from one", "Hello from two"]);

    let day_before = repo.world(&one.account_token).unwrap().data.clock.seconds;
    repo.tick();
    let day_after = repo.world(&one.account_token).unwrap().data.clock.seconds;
    assert_eq!(
        day_after - day_before,
        ServerConfig::default().world_seconds_per_tick
    );
    assert_eq!(repo.server_tick(), 1);
    assert_eq!(
        repo.world(&two.account_token).unwrap().data.players.len(),
        2
    );
}

#[test]
fn world_contains_the_phase_zero_collision_map() {
    let repo = repo();
    let session = guest(&repo, "map");
    let world = repo.world(&session.account_token).unwrap().data;
    assert_eq!(world.width, 18);
    assert_eq!(world.height, 11);
    assert!(world.tiles.iter().any(|tile| tile.kind == TileKind::Water));
    assert!(world.tiles.iter().any(|tile| tile.kind == TileKind::Field));
}

#[test]
fn movement_rate_limit_chat_bound_and_session_expiry_are_server_rules() {
    let repo = repo();
    let session = guest(&repo, "rules");
    let first = repo
        .movement(
            &session.account_token,
            MovementIntent {
                request_id: "step-one".to_owned(),
                dx: 1,
                dy: 0,
            },
        )
        .unwrap()
        .data;
    assert!(first.accepted);
    let too_fast = repo
        .movement(
            &session.account_token,
            MovementIntent {
                request_id: "step-two".to_owned(),
                dx: 1,
                dy: 0,
            },
        )
        .unwrap()
        .data;
    assert!(!too_fast.accepted);

    repo.tick();
    let too_long = repo
        .chat(
            &session.account_token,
            ChatRequest {
                request_id: "too-long".to_owned(),
                channel: "settlement".to_owned(),
                text: "x".repeat(161),
            },
        )
        .unwrap()
        .data;
    assert!(!too_long.accepted);

    for _ in 0..21 {
        repo.tick();
    }
    assert!(repo.world(&session.account_token).is_err());
}
