//! The local authoritative Tarrowyn server.
//!
//! The development backend is a versioned JSON repository behind
//! [`WorldRepository`]. It keeps the Phase 1–6 wire-and-authority fixtures
//! deterministic while preserving the protocol and HTTP boundaries for the
//! future managed production database.

mod config;
mod content;
mod http;
mod repository;

pub use config::ServerConfig;
pub use http::serve;
pub use repository::{RepositoryError, WorldRepository};
