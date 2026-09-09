//! Files you brought in: images and PDFs.
//!
//! An asset is **not** a note. It carries no title, no knowledge state and no
//! card, and it never enters review — it is what you think *about*, not what you
//! have learned. Turning one into a note is a separate, deliberate act, the same
//! way loose text becomes a note only when you say so.
//!
//! The file is **copied** into the app's own directory rather than referenced
//! where it sits. A board that breaks because you tidied your Downloads folder
//! is a board you stop trusting, and the whole point of putting a diagram on the
//! canvas is that it is still there next year.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::domain::new_id;
use crate::error::{AppError, Result};

use super::now;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(rename_all = "lowercase", type_name = "TEXT")]
pub enum AssetKind {
    Image,
    Pdf,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Asset {
    pub id: String,
    pub kind: AssetKind,
    /// What it was called when it arrived. Shown on the card, never used as a
    /// path — two files called `diagram.png` must not collide.
    pub file_name: String,
    pub mime: String,
    pub byte_size: i64,
    /// Where the copy lives, relative to the asset directory. Relative so that
    /// moving or restoring the data directory does not break every image.
    pub rel_path: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Where copies live, under the app's data directory.
pub fn dir(data_dir: &Path) -> PathBuf {
    data_dir.join("assets")
}

/// What we accept, and what we call it.
///
/// Deliberately a short list. Anything the webview cannot render inline would
/// arrive as a card you could not look at, which is worse than a refusal that
/// tells you why.
fn classify(ext: &str) -> Option<(AssetKind, &'static str)> {
    match ext.to_ascii_lowercase().as_str() {
        "png" => Some((AssetKind::Image, "image/png")),
        "jpg" | "jpeg" => Some((AssetKind::Image, "image/jpeg")),
        "gif" => Some((AssetKind::Image, "image/gif")),
        "webp" => Some((AssetKind::Image, "image/webp")),
        "svg" => Some((AssetKind::Image, "image/svg+xml")),
        "avif" => Some((AssetKind::Image, "image/avif")),
        "pdf" => Some((AssetKind::Pdf, "application/pdf")),
        _ => None,
    }
}

/// Copies a file in and records it.
///
/// The copy is named by its id, so the original name can be anything at all —
/// including the same name as something already here.
pub async fn import(pool: &SqlitePool, data_dir: &Path, source: &Path) -> Result<Asset> {
    let ext = source
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_owned();

    let Some((kind, mime)) = classify(&ext) else {
        return Err(AppError::Invalid(format!(
            "cannot bring in a .{ext} — images and PDFs only"
        )));
    };

    let file_name = source
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file")
        .to_owned();

    let byte_size = std::fs::metadata(source)?.len() as i64;

    let id = new_id();
    let rel_path = format!("{id}.{}", ext.to_ascii_lowercase());

    let target_dir = dir(data_dir);
    std::fs::create_dir_all(&target_dir)?;
    std::fs::copy(source, target_dir.join(&rel_path))?;

    let asset = sqlx::query_as::<_, Asset>(
        "INSERT INTO assets (id, kind, file_name, mime, byte_size, rel_path, created_at)
              VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
           RETURNING id, kind, file_name, mime, byte_size, rel_path, created_at",
    )
    .bind(&id)
    .bind(kind)
    .bind(&file_name)
    .bind(mime)
    .bind(byte_size)
    .bind(&rel_path)
    .bind(now())
    .fetch_one(pool)
    .await?;

    Ok(asset)
}

pub async fn get(pool: &SqlitePool, id: &str) -> Result<Option<Asset>> {
    let asset = sqlx::query_as::<_, Asset>(
        "SELECT id, kind, file_name, mime, byte_size, rel_path, created_at
           FROM assets WHERE id = ?1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(asset)
}

/// Every asset placed on one board, so the canvas renders in one round trip.
pub async fn for_topic(pool: &SqlitePool, topic_id: &str) -> Result<Vec<Asset>> {
    let assets = sqlx::query_as::<_, Asset>(
        "SELECT DISTINCT a.id, a.kind, a.file_name, a.mime, a.byte_size, a.rel_path, a.created_at
           FROM assets a
           JOIN canvas_nodes c ON c.asset_id = a.id
          WHERE c.topic_id = ?1",
    )
    .bind(topic_id)
    .fetch_all(pool)
    .await?;

    Ok(assets)
}

/// Drops assets no node points at any more, and their files with them.
///
/// Called after a node is removed. Deleting the row without deleting the file
/// would leave the data directory growing forever with things nothing can ever
/// show you again.
pub async fn sweep(pool: &SqlitePool, data_dir: &Path) -> Result<u64> {
    let orphans = sqlx::query_as::<_, Asset>(
        "SELECT id, kind, file_name, mime, byte_size, rel_path, created_at
           FROM assets a
          WHERE NOT EXISTS (SELECT 1 FROM canvas_nodes c WHERE c.asset_id = a.id)",
    )
    .fetch_all(pool)
    .await?;

    let base = dir(data_dir);
    for asset in &orphans {
        // A file already gone is not an error: the row going is the point.
        let _ = std::fs::remove_file(base.join(&asset.rel_path));

        sqlx::query("DELETE FROM assets WHERE id = ?1")
            .bind(&asset.id)
            .execute(pool)
            .await?;
    }

    Ok(orphans.len() as u64)
}
