//! The local authoritative Tarrowyn server.
//!
//! The repository boundary supports a deterministic versioned JSON fixture and
//! the selected transactional MySQL snapshot backend. Both preserve the same
//! Phase 1–6 protocol and HTTP authority boundary.

mod config;
mod content;
mod http;
mod repository;

pub use config::ServerConfig;
pub use http::serve;
pub use repository::{RepositoryError, WorldRepository};
