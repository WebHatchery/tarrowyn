use super::super::super::{ServerConfig, WorldRepository};
use super::super::logic::record_regional;
use tarrowyn_protocol::ChronicleEntry;

fn chronicle_entry(index: usize) -> ChronicleEntry {
    ChronicleEntry {
        event_id: format!("regional-history-{index}"),
        kind: "regional".to_owned(),
        title: format!("Regional record {index}"),
        text: "The settlement keeps its latest road history.".to_owned(),
        created_tick: index as u64,
        cursor: index as u64,
    }
}

#[test]
fn settlement_chronicle_keeps_the_newest_local_records() {
    let repository = WorldRepository::new(ServerConfig::default());
    {
        let mut state = repository.state.lock().expect("repository lock");
        state.phase5.settlements[0].chronicle = (0..64).map(chronicle_entry).collect();
        record_regional(
            &mut state,
            &["hearth"],
            "retention test",
            "A new local record arrived.",
        );
    }

    let state = repository.state.lock().expect("repository lock");
    let chronicle = &state.phase5.settlements[0].chronicle;
    assert_eq!(chronicle.len(), 64);
    assert_eq!(chronicle[0].event_id, "regional-history-1");
    assert_eq!(chronicle.last().unwrap().event_id, "chronicle-1");
}
