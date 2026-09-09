//! Tags.
//!
//! A classification you apply on purpose, and the only new entity in this pass.
//! Uniqueness is on the Spanish fold rather than the raw name, so `Localización`
//! and `localizacion` are one tag — while `año` and `ano` stay two, because they
//! are different words.

use serde::Serialize;
use sqlx::SqlitePool;

use crate::domain::new_id;
use crate::error::{AppError, Result};
use crate::note_state::fold_es;

use super::now;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Tag {
    pub id: String,
    pub name: String,
    pub fold: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Every tag, with how many notes carry it.
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct TagCount {
    #[sqlx(flatten)]
    pub tag: Tag,
    pub notes: i64,
}

pub async fn all(pool: &SqlitePool) -> Result<Vec<TagCount>> {
    let tags = sqlx::query_as::<_, TagCount>(
        "SELECT t.id, t.name, t.fold, t.created_at, COUNT(nt.note_id) AS notes
           FROM tags t
           LEFT JOIN note_tags nt ON nt.tag_id = t.id
          GROUP BY t.id
          ORDER BY t.fold",
    )
    .fetch_all(pool)
    .await?;

    Ok(tags)
}

/// Finds or creates a tag by name. Idempotent, so attaching the same tag twice
/// under different capitalisation does not produce a second one.
pub async fn ensure(pool: &SqlitePool, name: &str) -> Result<Tag> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::Invalid("a tag needs a name".into()));
    }

    let fold = fold_es(name);

    if let Some(existing) = sqlx::query_as::<_, Tag>(
        "SELECT id, name, fold, created_at FROM tags WHERE fold = ?1",
    )
    .bind(&fold)
    .fetch_optional(pool)
    .await?
    {
        return Ok(existing);
    }

    let tag = sqlx::query_as::<_, Tag>(
        "INSERT INTO tags (id, name, fold, created_at)
              VALUES (?1, ?2, ?3, ?4)
           RETURNING id, name, fold, created_at",
    )
    .bind(new_id())
    .bind(name)
    .bind(&fold)
    .bind(now())
    .fetch_one(pool)
    .await?;

    Ok(tag)
}

pub async fn for_note(pool: &SqlitePool, note_id: &str) -> Result<Vec<Tag>> {
    let tags = sqlx::query_as::<_, Tag>(
        "SELECT t.id, t.name, t.fold, t.created_at
           FROM tags t
           JOIN note_tags nt ON nt.tag_id = t.id
          WHERE nt.note_id = ?1
          ORDER BY t.fold",
    )
    .bind(note_id)
    .fetch_all(pool)
    .await?;

    Ok(tags)
}

pub async fn attach(pool: &SqlitePool, note_id: &str, name: &str) -> Result<Tag> {
    let tag = ensure(pool, name).await?;

    sqlx::query("INSERT OR IGNORE INTO note_tags (note_id, tag_id) VALUES (?1, ?2)")
        .bind(note_id)
        .bind(&tag.id)
        .execute(pool)
        .await?;

    Ok(tag)
}

/// Detaching also removes a tag that nothing carries any more.
///
/// A tag list that only ever grows is a list of everything you once thought
/// worth naming, which is not the same as a classification you use.
pub async fn detach(pool: &SqlitePool, note_id: &str, tag_id: &str) -> Result<()> {
    sqlx::query("DELETE FROM note_tags WHERE note_id = ?1 AND tag_id = ?2")
        .bind(note_id)
        .bind(tag_id)
        .execute(pool)
        .await?;

    sqlx::query(
        "DELETE FROM tags
          WHERE id = ?1
            AND NOT EXISTS (SELECT 1 FROM note_tags WHERE tag_id = ?1)",
    )
    .bind(tag_id)
    .execute(pool)
    .await?;

    Ok(())
}

/// Removes a tag from every note that carries it, and then the tag.
///
/// The counterpart to `detach`, which only lets a tag go when its last carrier
/// does. A classification you have stopped using should be droppable in one
/// act rather than by hunting down every note that still mentions it.
pub async fn delete(pool: &SqlitePool, id: &str) -> Result<()> {
    sqlx::query("DELETE FROM note_tags WHERE tag_id = ?1")
        .bind(id)
        .execute(pool)
        .await?;

    sqlx::query("DELETE FROM tags WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await?;

    Ok(())
}

/// The ids of every note carrying a tag.
///
/// The library filters in memory, so it wants a set rather than a join: a tag
/// is a *filter* over the corpus, never a container that owns what it holds.
pub async fn note_ids(pool: &SqlitePool, tag_id: &str) -> Result<Vec<String>> {
    let ids = sqlx::query_scalar::<_, String>(
        "SELECT note_id FROM note_tags WHERE tag_id = ?1",
    )
    .bind(tag_id)
    .fetch_all(pool)
    .await?;

    Ok(ids)
}
