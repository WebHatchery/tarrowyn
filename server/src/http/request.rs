use serde::de::DeserializeOwned;
use std::io::Read;
use tiny_http::Request;

pub(crate) const MAX_REQUEST_BODY_BYTES: usize = 64 * 1024;
pub(crate) const MAX_BEARER_TOKEN_CHARS: usize = 512;
pub(crate) const MAX_REQUEST_URL_BYTES: usize = 8 * 1024;

pub(super) fn request_url_is_bounded(url: &str) -> bool {
    url.len() <= MAX_REQUEST_URL_BYTES
}

pub(super) fn read_json<T: DeserializeOwned>(request: &mut Request) -> Result<T, String> {
    let mut reader = request.as_reader();
    let body = read_bounded_body(&mut reader)?;
    serde_json::from_str(&body).map_err(|error| format!("Could not decode request JSON: {error}"))
}

pub(super) fn read_json_or_default<T: DeserializeOwned + Default>(
    request: &mut Request,
) -> Result<T, String> {
    let mut reader = request.as_reader();
    let body = read_bounded_body(&mut reader)?;
    if body.trim().is_empty() {
        Ok(T::default())
    } else {
        serde_json::from_str(&body)
            .map_err(|error| format!("Could not decode request JSON: {error}"))
    }
}

pub(super) fn read_bounded_body<R: Read>(reader: &mut R) -> Result<String, String> {
    let mut bytes = Vec::new();
    reader
        .take((MAX_REQUEST_BODY_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("Could not read request body: {error}"))?;
    if bytes.len() > MAX_REQUEST_BODY_BYTES {
        return Err(format!(
            "Request body exceeds {MAX_REQUEST_BODY_BYTES} bytes."
        ));
    }
    String::from_utf8(bytes).map_err(|_| "Request body must be valid UTF-8.".to_owned())
}

pub(super) fn bearer_token(request: &Request) -> Option<String> {
    request
        .headers()
        .iter()
        .find(|header| header.field.equiv("Authorization"))
        .and_then(|header| parse_bearer_header(header.value.as_str()))
}

pub(super) fn parse_bearer_header(value: &str) -> Option<String> {
    let (scheme, credentials) = value.split_once(' ')?;
    if !scheme.eq_ignore_ascii_case("Bearer") {
        return None;
    }
    let credentials = credentials.trim();
    (!credentials.is_empty()
        && credentials.chars().count() <= MAX_BEARER_TOKEN_CHARS
        && !credentials.chars().any(char::is_control))
    .then(|| credentials.to_owned())
}

pub(super) fn split_url(url: &str) -> (&str, &str) {
    url.split_once('?').unwrap_or((url, ""))
}

pub(super) fn query_value_result(query: &str, name: &str) -> Result<Option<String>, String> {
    query
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .find_map(|(key, value)| (key == name).then_some(value))
        .map(|value| {
            decode_query_value(value)
                .ok_or_else(|| "The query value is not valid form encoding.".to_owned())
        })
        .transpose()
}

pub(super) fn query_cursor(query: &str, name: &str) -> Result<u64, String> {
    let Some(value) = query_value_result(query, name)? else {
        return Ok(0);
    };
    value
        .parse::<u64>()
        .map_err(|_| "The history cursor query value must be a non-negative integer.".to_owned())
}

fn decode_query_value(value: &str) -> Option<String> {
    let mut bytes = Vec::with_capacity(value.len());
    let mut chars = value.as_bytes().iter().copied();
    while let Some(byte) = chars.next() {
        match byte {
            b'+' => bytes.push(b' '),
            b'%' => {
                let high = chars.next().and_then(hex_digit)?;
                let low = chars.next().and_then(hex_digit)?;
                bytes.push(high << 4 | low);
            }
            byte => bytes.push(byte),
        }
    }
    String::from_utf8(bytes).ok()
}

fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}
