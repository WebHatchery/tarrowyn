//! Minimal vendored `quad-net` HTTP transport.

pub mod http_request;

#[no_mangle]
pub extern "C" fn quad_net_crate_version() -> u32 {
    1
}
