//! Topics: workspaces with no hierarchy and no paths.

use sqlx::SqlitePool;

use crate::domain::{new_id, Topic};
use crate::error::Result;

use super::now;

/// The sidebar query. Ordered by recency of visit, never by name or structure:
/// topics you have not touched fall asleep on their own and come back when you
/// return to them. `limit` is what keeps the list from growing without bound.
pub async fn awake(pool: &SqlitePool, limit: i64) -> Result<Vec<Topic>> {
    let topics = sqlx::query_as::<_, Topic>(
        "SELECT id, title, study, created_at, updated_at, last_visited_at
           FROM topics
          ORDER BY last_visited_at DESC
          LIMIT ?1",
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(topics)
}

/// Everything, for search and for the "wake a sleeping topic" path. Sleeping is
/// not failing, so this is a plain list with no marker of neglect.
pub async fn all(pool: &SqlitePool) -> Result<Vec<Topic>> {
    let topics = sqlx::query_as::<_, Topic>(
        "SELECT id, title, study, created_at, updated_at, last_visited_at
           FROM topics
          ORDER BY last_visited_at DESC",
    )
    .fetch_all(pool)
    .await?;

    Ok(topics)
}

pub async fn get(pool: &SqlitePool, id: &str) -> Result<Option<Topic>> {
    let topic = sqlx::query_as::<_, Topic>(
        "SELECT id, title, study, created_at, updated_at, last_visited_at
           FROM topics
          WHERE id = ?1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(topic)
}

pub async fn create(pool: &SqlitePool, id: Option<String>, title: &str) -> Result<Topic> {
    let id = id.unwrap_or_else(new_id);
    let ts = now();

    let topic = sqlx::query_as::<_, Topic>(
        "INSERT INTO topics (id, title, study, created_at, updated_at, last_visited_at)
              VALUES (?1, ?2, 0, ?3, ?3, ?3)
           RETURNING id, title, study, created_at, updated_at, last_visited_at",
    )
    .bind(&id)
    .bind(title)
    .bind(ts)
    .fetch_one(pool)
    .await?;

    Ok(topic)
}

pub async fn rename(pool: &SqlitePool, id: &str, title: &str) -> Result<()> {
    sqlx::query("UPDATE topics SET title = ?2, updated_at = ?3 WHERE id = ?1")
        .bind(id)
        .bind(title)
        .bind(now())
        .execute(pool)
        .await?;

    Ok(())
}

/// Called on open. This is the only thing that keeps a topic awake.
pub async fn touch(pool: &SqlitePool, id: &str) -> Result<()> {
    sqlx::query("UPDATE topics SET last_visited_at = ?2 WHERE id = ?1")
        .bind(id)
        .bind(now())
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn delete(pool: &SqlitePool, id: &str) -> Result<()> {
    sqlx::query("DELETE FROM topics WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await?;

    Ok(())
}

/// Switches a board between work and study.
///
/// Non-destructive in both directions: nothing is deleted and no note fact is
/// touched. Study mode only makes the epistemic layer visible; turning it off
/// hides it again, and a note recalled while the board was a study board keeps
/// that fact either way.
pub async fn set_mode(pool: &SqlitePool, id: &str, study: bool) -> Result<Topic> {
    let topic = sqlx::query_as::<_, Topic>(
        "UPDATE topics SET study = ?2, updated_at = ?3
          WHERE id = ?1
      RETURNING id, title, study, created_at, updated_at, last_visited_at",
    )
    .bind(id)
    .bind(study)
    .bind(now())
    .fetch_one(pool)
    .await?;

    Ok(topic)
}
