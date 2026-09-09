//! Query layer. One module per entity; all SQL lives here and nowhere else.

pub mod assets;
pub mod canvas;
pub mod derived;
pub mod notes;
pub mod questions;
pub mod review;
pub mod tags;
pub mod topics;

/// Timestamps are stored as RFC 3339 text so that lexical ordering matches
/// chronological ordering in SQLite.
pub fn now() -> chrono::DateTime<chrono::Utc> {
    chrono::Utc::now()
}
