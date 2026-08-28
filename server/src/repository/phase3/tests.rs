use super::*;
use crate::config::ServerConfig;
use crate::repository::WorldRepository;
use tarrowyn_protocol::GuestSessionRequest;

fn guest(repository: &WorldRepository) -> tarrowyn_protocol::GuestSessionResponse {
    repository
        .guest_session(GuestSessionRequest {
            client_key: Some("chronicle-archive-test".to_owned()),
            reset: false,
        })
        .expect("guest session")
        .data
}

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
