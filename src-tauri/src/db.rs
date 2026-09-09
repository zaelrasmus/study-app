//! SQLite persistence backed by `sqlx`.
//!
//! The pool is created once during `setup` and handed to commands through Tauri
//! managed state. `SqlitePool` is already an `Arc` internally, so commands can
//! clone it freely and no `Mutex` is needed.

use std::path::Path;

use sqlx::migrate::Migrator;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::SqlitePool;

use crate::error::Result;

/// Migrations in `src-tauri/migrations` are embedded into the binary at compile
/// time, so the shipped app never has to locate them on disk.
static MIGRATOR: Migrator = sqlx::migrate!();

/// Database file name inside the platform app-data directory.
const DB_FILE_NAME: &str = "study-app.db";

/// Opens the database in `data_dir`, creating the file if it doesn't exist yet,
/// and applies any pending migrations.
pub async fn connect(data_dir: &Path) -> Result<SqlitePool> {
    std::fs::create_dir_all(data_dir)?;

    let options = SqliteConnectOptions::new()
        .filename(data_dir.join(DB_FILE_NAME))
        .create_if_missing(true)
        // WAL keeps reads from blocking the single writer, which matters once
        // the canvas autosaves while the UI is reading.
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .foreign_keys(true)
        // Without this, a stale process still holding the database lock makes
        // startup block forever -- and because the pool is built in Tauri's
        // `setup`, "blocks forever" means the window is never created and the
        // app looks like it simply did not launch. Failing loudly beats that.
        .busy_timeout(std::time::Duration::from_secs(5));

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;

    MIGRATOR.run(&pool).await?;

    Ok(pool)
}
