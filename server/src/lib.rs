//! The local authoritative Tarrowyn server.
//!
//! Phase 1 deliberately keeps persistence behind [`WorldRepository`]. The
//! selected development backend is an in-memory repository: it makes three-
//! client fixtures deterministic and keeps restart persistence out of the
//! first wire-and-authority proof. Phase 2 can replace this implementation
//! with SQLite without changing the protocol or HTTP handlers.

mod config;
mod http;
mod repository;

pub use config::ServerConfig;
pub use http::serve;
pub use repository::{RepositoryError, WorldRepository};
