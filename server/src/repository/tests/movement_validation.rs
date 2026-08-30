use super::{guest, repo};
use crate::{ServerConfig, WorldRepository};
use tarrowyn_protocol::{MovementIntent, Position};

#[test]
fn movement_rejects_extreme_deltas_without_overflow() {
    let repository = repo();
    let session = guest(&repository, "movement-overflow");
    let response = repository
        .movement(
            &session.account_token,
            MovementIntent {
                request_id: "movement-overflow-request".to_owned(),
                dx: i32::MIN,
                dy: i32::MAX,
            },
        )
        .expect("extreme movement should return a response")
        .data;
    assert!(!response.accepted);
    assert_eq!(response.position, Position { x: 8, y: 6 });
    assert!(response
        .reason
        .as_deref()
        .is_some_and(|reason| reason.contains("cardinal step")));
}

#[test]
fn movement_rejects_corrupt_position_without_overflow() {
    let repository = repo();
    let session = guest(&repository, "movement-corrupt-position");
    {
        let mut state = repository.state.lock().expect("repository state");
        let identity_key = state
            .sessions
            .get(&session.account_token)
            .expect("session")
            .identity_key
            .clone();
        state
            .identities
            .get_mut(&identity_key)
            .expect("identity")
            .position = Position { x: i32::MAX, y: 6 };
    }

    let response = repository
        .movement(
            &session.account_token,
            MovementIntent {
                request_id: "movement-corrupt-position-request".to_owned(),
                dx: 1,
                dy: 0,
            },
        )
        .expect("corrupt position should return a response")
        .data;

    assert!(!response.accepted);
    assert_eq!(response.position, Position { x: i32::MAX, y: 6 });
    assert!(response
        .reason
        .as_deref()
        .is_some_and(|reason| reason.contains("settlement edge")));
}

#[test]
fn movement_accepts_the_positive_i32_edge_when_world_width_exceeds_i32() {
    let repository = WorldRepository::new(ServerConfig {
        world_width: i32::MAX as u32 + 1,
        movement_cooldown_ticks: 0,
        ..ServerConfig::default()
    });
    let session = guest(&repository, "movement-wide-world");
    {
        let mut state = repository.state.lock().expect("repository state");
        let identity_key = state
            .sessions
            .get(&session.account_token)
            .expect("session")
            .identity_key
            .clone();
        state
            .identities
            .get_mut(&identity_key)
            .expect("identity")
            .position = Position {
            x: i32::MAX - 1,
            y: 6,
        };
        state.phase3.zone.threat_active = false;
        state.phase3.zone.monster_health = 0;
        state.phase3.zone.road_open = true;
    }

    let response = repository
        .movement(
            &session.account_token,
            MovementIntent {
                request_id: "movement-wide-world-request".to_owned(),
                dx: 1,
                dy: 0,
            },
        )
        .expect("wide-world movement should return a response")
        .data;

    assert!(response.accepted);
    assert_eq!(response.position, Position { x: i32::MAX, y: 6 });
}
