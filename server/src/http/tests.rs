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
fn query_values_decode_form_encoding_for_history_search() {
    assert_eq!(
        query_value_result("q=household+arrival&since=12", "q"),
        Ok(Some("household arrival".to_owned()))
    );
    assert_eq!(
        query_value_result("q=road%20repair", "q"),
        Ok(Some("road repair".to_owned()))
    );
}

#[test]
fn malformed_query_escapes_are_not_forwarded_to_authority() {
    assert!(query_value_result("q=bad%2", "q").is_err());
    assert!(query_value_result("q=bad%GG", "q").is_err());
    assert!(query_value_result("account_id=bad%GG", "account_id").is_err());
}

#[test]
fn bearer_scheme_is_case_insensitive_but_credentials_must_be_present() {
    assert_eq!(
        parse_bearer_header("bearer guest-session-1").as_deref(),
        Some("guest-session-1")
    );
    assert_eq!(
        parse_bearer_header("Bearer   guest-session-2").as_deref(),
        Some("guest-session-2")
    );
    assert!(parse_bearer_header("Basic guest-session").is_none());
    assert!(parse_bearer_header("Bearer").is_none());
    assert!(parse_bearer_header("Bearer guest\n-session").is_none());
}

#[test]
fn history_cursors_reject_malformed_values_instead_of_resetting() {
    assert_eq!(query_cursor("since=12", "since"), Ok(12));
    assert_eq!(query_cursor("q=road", "since"), Ok(0));
    assert!(query_cursor("since=-1", "since").is_err());
    assert!(query_cursor("since=bad%GG", "since").is_err());
}

#[test]
fn chronicle_queries_reject_malformed_encoding_but_allow_omission() {
    assert_eq!(query_value_result("since=12", "q"), Ok(None));
    assert_eq!(
        query_value_result("q=road%20repair", "q"),
        Ok(Some("road repair".to_owned()))
    );
    assert!(query_value_result("q=road%GG", "q").is_err());
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

#[test]
fn api_responses_disable_intermediary_caching() {
    let response = json_response(StatusCode(200), serde_json::json!({ "status": "ok" }));

    assert!(response.headers().iter().any(|header| {
        header.field.equiv("Cache-Control") && header.value.as_str() == "no-store"
    }));
}
