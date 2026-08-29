use crate::{ServerConfig, WorldRepository};

#[test]
fn regional_household_history_keeps_only_the_latest_bounded_window() {
    let repository = WorldRepository::new(ServerConfig::default());
    {
        let mut state = repository.state.lock().expect("repository lock");
        state.phase5.households[0].history = (0..=super::super::state::MAX_HOUSEHOLD_HISTORY)
            .map(|index| format!("household-history-{index}"))
            .collect();
        super::super::state::trim_household_histories(&mut state.phase5);
    }

    let state = repository.state.lock().expect("repository lock");
    let history = &state.phase5.households[0].history;
    assert_eq!(history.len(), super::super::state::MAX_HOUSEHOLD_HISTORY);
    assert_eq!(
        history.first().map(String::as_str),
        Some("household-history-1")
    );
    assert_eq!(
        history.last().map(String::as_str),
        Some("household-history-64")
    );
}
