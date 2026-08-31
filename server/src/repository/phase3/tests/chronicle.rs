use super::*;

#[test]
fn chronicle_keeps_archived_entries_searchable_and_summarised() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = guest(&repository);
    {
        let mut state = repository.state.lock().expect("repository lock");
        for index in 0..(MAX_CHRONICLE + 3) {
            record(
                &mut state,
                "archived achievement",
                &format!("Achievement {index:02}"),
                "An achievement remains in the regional record.",
            );
        }
    }

    let recent = repository
        .chronicle(&session.account_token, 0)
        .expect("chronicle")
        .data;
    assert_eq!(recent.entries.len(), MAX_CHRONICLE);
    let summary = recent.summary.expect("archived summary");
    assert_eq!(summary.entry_count, 3);
    assert!(summary.from_cursor < recent.entries[0].cursor);
    assert!(summary.to_cursor > summary.from_cursor);

    let search = repository
        .chronicle_search(&session.account_token, "Achievement 00", 0)
        .expect("chronicle search")
        .data;
    assert!(search
        .entries
        .iter()
        .any(|entry| entry.title == "Achievement 00"));
    assert_eq!(search.summary.expect("search summary").entry_count, 1);
}

#[test]
fn chronicle_rejects_cursors_outside_the_retained_event_window() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = guest(&repository);
    let (oldest, current) = {
        let mut state = repository.state.lock().expect("repository lock");
        for index in 0..=super::super::MAX_EVENTS {
            record(
                &mut state,
                "cursor boundary",
                &format!("Boundary entry {index}"),
                "The chronicle boundary remains explicit.",
            );
        }
        (
            state.events.front().expect("retained event window").cursor,
            state.cursor,
        )
    };

    let stale = repository
        .chronicle(&session.account_token, oldest - 2)
        .expect_err("a stale chronicle cursor must fail closed");
    assert_eq!(stale.status, 409);
    assert_eq!(stale.error.code, "cursor_stale");

    let ahead = repository
        .chronicle(&session.account_token, current + 1)
        .expect_err("a chronicle cursor ahead of the world must fail closed");
    assert_eq!(ahead.status, 409);
    assert_eq!(ahead.error.code, "cursor_ahead");
}
