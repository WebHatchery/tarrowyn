use super::super::{ServerConfig, WorldRepository};
use tarrowyn_protocol::{
    ChatRequest, ClaimStatus, EventRecord, FrontierEvent, LandClaim, Position, WorldClock,
    WorldEvent,
};

#[test]
fn future_clock_event_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    {
        let mut state = repository.state.lock().expect("repository lock");
        let future_day = state.clock.day.saturating_add(1);
        let day_length_seconds = state.clock.day_length_seconds;
        let event = state
            .events
            .iter_mut()
            .find(|record| matches!(record.event, WorldEvent::TavernNotice(_)))
            .expect("startup notice event");
        event.event = WorldEvent::Clock(WorldClock {
            day: future_day,
            seconds: 0.0,
            day_length_seconds,
        });
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn malformed_chat_event_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = repository
        .guest_session(tarrowyn_protocol::GuestSessionRequest {
            client_key: Some("core-event-chat-integrity".to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data;
    repository
        .chat(
            &session.account_token,
            ChatRequest {
                request_id: "core-event-chat".to_owned(),
                channel: "settlement".to_owned(),
                text: "A durable message.".to_owned(),
            },
        )
        .expect("chat message");
    {
        let mut state = repository.state.lock().expect("repository lock");
        let event = state
            .events
            .iter_mut()
            .find(|record| matches!(record.event, WorldEvent::Chat(_)))
            .expect("chat event");
        if let WorldEvent::Chat(message) = &mut event.event {
            message.text = "malformed\nmessage".to_owned();
        }
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn frontier_event_must_reference_a_known_account() {
    let repository = WorldRepository::new(ServerConfig::default());
    {
        let mut state = repository.state.lock().expect("repository lock");
        let tick = state.tick;
        let cursor = state.cursor.saturating_add(1);
        state.cursor = cursor;
        state.events.push_back(EventRecord {
            cursor,
            event: WorldEvent::Frontier(FrontierEvent::Claim(LandClaim {
                claim_id: "event-integrity-claim".to_owned(),
                owner_account_id: "missing-account".to_owned(),
                owner_name: "Missing resident".to_owned(),
                position: Position { x: 10, y: 8 },
                lease_days: 3,
                last_active_tick: tick,
                reclaim_after_ticks: 10,
                status: ClaimStatus::Active,
            })),
        });
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}
