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

#[test]
fn api_responses_emit_one_cors_origin_header() {
    let response = json_response(StatusCode(200), serde_json::json!({ "status": "ok" }));

    assert_eq!(
        response
            .headers()
            .iter()
            .filter(|header| header.field.equiv("Access-Control-Allow-Origin"))
            .count(),
        1
    );
}

#[test]
fn rate_limited_guest_sessions_advertise_the_retry_window() {
    let response = rate_limited_response(429, ApiMeta::at(7));

    assert_eq!(
        response
            .headers()
            .iter()
            .find(|header| header.field.equiv("Retry-After"))
            .map(|header| header.value.as_str()),
        Some(GUEST_SESSION_RETRY_AFTER_SECONDS)
    );
}

#[test]
fn request_worker_count_stays_within_the_bounded_pool() {
    assert!((MIN_REQUEST_WORKERS..=MAX_REQUEST_WORKERS).contains(&request_worker_count()));
}

#[test]
fn request_pool_telemetry_tracks_depth_peak_activity_and_saturation() {
    let telemetry = RequestPoolTelemetry::default();

    telemetry.record_enqueue();
    telemetry.record_enqueue();
    telemetry.record_queue_full();
    telemetry.record_dequeue();
    telemetry.record_request_start();

    let snapshot = telemetry.snapshot();
    assert_eq!(snapshot.active_requests, 1);
    assert_eq!(snapshot.queue_depth, 1);
    assert_eq!(snapshot.queue_peak, 2);
    assert_eq!(snapshot.queue_full_events, 1);

    telemetry.record_request_finish();
    telemetry.record_dequeue();
    assert_eq!(telemetry.snapshot().active_requests, 0);
    assert_eq!(telemetry.snapshot().queue_depth, 0);
}

#[test]
fn guest_session_rate_limiter_bounds_a_source_window() {
    let mut limiter = GuestSessionRateLimiter::default();
    let source = "192.0.2.10".parse().expect("test source address");
    let other_source = "192.0.2.11".parse().expect("test source address");
    let now = Instant::now();

    for _ in 0..GUEST_SESSION_BURST_LIMIT {
        assert!(limiter.allow_ip(Some(source), now));
    }
    assert!(!limiter.allow_ip(Some(source), now));
    assert!(limiter.allow_ip(Some(other_source), now));
    assert!(limiter.allow_ip(
        Some(source),
        now + GUEST_SESSION_RATE_WINDOW + Duration::from_millis(1)
    ));
}

#[test]
fn guest_session_rate_limiter_honours_a_configured_burst_limit() {
    let mut limiter = GuestSessionRateLimiter::new(3);
    let source = "192.0.2.12".parse().expect("test source address");
    let now = Instant::now();

    for _ in 0..3 {
        assert!(limiter.allow_ip(Some(source), now));
    }
    assert!(!limiter.allow_ip(Some(source), now));
}

#[test]
fn guest_session_rate_limiter_does_not_block_unaddressed_fixture_clients() {
    let mut limiter = GuestSessionRateLimiter::default();
    let now = Instant::now();

    for _ in 0..(GUEST_SESSION_BURST_LIMIT + 1) {
        assert!(limiter.allow_ip(None, now));
    }
}

#[test]
fn guest_session_rate_limiter_keeps_source_table_bounded_without_global_lockout() {
    let mut limiter = GuestSessionRateLimiter::default();
    let now = Instant::now();

    for source_id in 0..=MAX_TRACKED_GUEST_SOURCES {
        let source = IpAddr::V6(std::net::Ipv6Addr::from(source_id as u128));
        assert!(limiter.allow_ip(Some(source), now));
    }
    assert_eq!(limiter.windows.len(), MAX_TRACKED_GUEST_SOURCES);
}
