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
