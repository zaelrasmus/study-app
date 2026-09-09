//! Error type shared by every Tauri command.
//!
//! Commands return `Result<T, AppError>`; the `Serialize` impl is what lets the
//! error cross the IPC boundary and surface as a rejected promise in Svelte.

/// Anything that can go wrong while serving a command.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("tauri error: {0}")]
    Tauri(#[from] tauri::Error),

    /// A request the domain refuses. Kept separate from the rest so the UI can
    /// show it as a plain statement of fact rather than a failure.
    #[error("{0}")]
    Invalid(String),

    #[error("not found: {0}")]
    NotFound(String),
}

impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

/// Convenience alias for command signatures.
pub type Result<T> = std::result::Result<T, AppError>;
