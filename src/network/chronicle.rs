use tarrowyn_protocol::ChronicleEntry;

pub(crate) const MAX_CACHED_CHRONICLE: usize = 12;

pub(crate) fn encode_query_value(value: &str) -> String {
    value.bytes().fold(String::new(), |mut encoded, byte| {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(byte as char);
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
        encoded
    })
}

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

#[cfg(test)]
#[path = "chronicle/tests.rs"]
mod tests;
