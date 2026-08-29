use super::*;
use std::io::Cursor;
use std::time::{Duration, Instant};

#[test]
fn bounded_body_accepts_the_configured_limit() {
    let input = vec![b'a'; MAX_REQUEST_BODY_BYTES];
    let mut reader = Cursor::new(input);
    let body = read_bounded_body(&mut reader).unwrap();
    assert_eq!(body.len(), MAX_REQUEST_BODY_BYTES);
}

#[test]
fn bounded_body_rejects_oversized_input_before_json_decode() {
    let input = vec![b'a'; MAX_REQUEST_BODY_BYTES + 1];
    let mut reader = Cursor::new(input);
    let error = read_bounded_body(&mut reader).unwrap_err();
    assert!(error.contains("exceeds 65536 bytes"));
}

#[test]
fn tick_deadline_stays_fixed_until_an_overrun_needs_recovery() {
    let start = Instant::now();
    let interval = Duration::from_millis(250);

    assert_eq!(
        next_tick_deadline(start, start + Duration::from_millis(100), interval),
        start + interval
    );
    assert_eq!(
        next_tick_deadline(start, start + Duration::from_millis(600), interval),
        start + Duration::from_millis(850)
    );
}
