use super::*;

#[test]
fn chronicle_search_returns_a_bounded_page_with_a_continuation_cursor() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("chronicle-search-boundary".to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data;
    {
        let mut state = repository.state.lock().expect("repository lock");
        for index in 0..(super::super::super::phase3::MAX_CHRONICLE + 160) {
            super::super::super::phase3::record(
                &mut state,
                "searchable achievement",
                &format!("Searchable achievement {index:03}"),
                "The archive remains available to the settlement.",
            );
        }
    }

    let search = repository
        .chronicle_search(&session.account_token, "", 0)
        .expect("chronicle search")
        .data;
    assert_eq!(search.entries.len(), 128);
    assert_eq!(
        search.next_cursor,
        search.entries.last().map(|entry| entry.cursor)
    );
}

#[test]
fn chronicle_search_summarises_recent_matches_too() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("chronicle-search-recent-summary".to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data;
    {
        let mut state = repository.state.lock().expect("repository lock");
        super::super::super::phase3::record(
            &mut state,
            "recent signal",
            "Only recent search match",
            "The newest record still carries its archive summary.",
        );
    }

    let search = repository
        .chronicle_search(&session.account_token, "Only recent search match", 0)
        .expect("chronicle search")
        .data;
    assert_eq!(search.entries.len(), 1);
    let summary = search.summary.expect("recent match summary");
    assert_eq!(summary.entry_count, 1);
    assert_eq!(summary.from_cursor, search.entries[0].cursor);
    assert_eq!(summary.to_cursor, search.entries[0].cursor);
}

#[test]
fn chronicle_search_rejects_unbounded_or_controlled_queries() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("chronicle-search-input".to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data;

    for query in ["x".repeat(81), "search\nwith-control".to_owned()] {
        let error = repository
            .chronicle_search(&session.account_token, &query, 0)
            .expect_err("invalid chronicle query should be rejected");
        assert_eq!(error.status, 400);
        assert_eq!(error.error.code, "invalid_chronicle_query");
    }
}

#[test]
fn chronicle_search_rejects_a_cursor_ahead_of_the_authoritative_world() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("chronicle-search-ahead".to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data;
    let ahead = {
        let state = repository.state.lock().expect("repository lock");
        state.cursor.saturating_add(1)
    };

    let error = repository
        .chronicle_search(&session.account_token, "", ahead)
        .expect_err("an ahead chronicle cursor should be rejected");

    assert_eq!(error.status, 409);
    assert_eq!(error.error.code, "cursor_ahead");
}
