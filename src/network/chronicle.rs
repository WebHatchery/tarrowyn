use tarrowyn_protocol::ChronicleEntry;

pub(crate) const MAX_CACHED_CHRONICLE: usize = 12;

pub(crate) fn merge_chronicle_entry(chronicle: &mut Vec<ChronicleEntry>, entry: ChronicleEntry) {
    if chronicle
        .iter()
        .any(|existing| existing.event_id == entry.event_id)
    {
        return;
    }
    chronicle.push(entry);
    chronicle.sort_by_key(|entry| entry.cursor);
    if chronicle.len() > MAX_CACHED_CHRONICLE {
        let excess = chronicle.len() - MAX_CACHED_CHRONICLE;
        chronicle.drain(..excess);
    }
}
