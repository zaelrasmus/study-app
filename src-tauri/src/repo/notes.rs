//! Notes.
//!
//! Nothing here writes a status column, because there isn't one. The writers
//! record *events* — a body was edited, a recall happened, a source was
//! contrasted, a review failed — and the state follows from those facts the
//! next time it is read.
//!
//! Every query is a `&'static str`: sqlx 0.9 only accepts static SQL without an
//! explicit `AssertSqlSafe`, which is a good constraint to keep.

use sqlx::SqlitePool;

use crate::domain::{content_hash, new_id, Note, NoteKind};
use crate::error::Result;

use super::now;

const COLS: &str = "id, kind, title, summary, body_json, body_html, body_text, content_hash,
                    distilled_at, last_recall_at, last_checked_at, last_recall_failed_at,
                    content_edited_at, content_hash_at_recall, source_note_id, journal_day, created_at, updated_at";

/// Capture. The only path in the app that must stay instant, so it takes a
/// title and nothing else: no topic, no placement, no type decision.
pub async fn create(
    pool: &SqlitePool,
    id: Option<String>,
    title: &str,
    kind: NoteKind,
) -> Result<Note> {
    let id = id.unwrap_or_else(new_id);

    let note = sqlx::query_as::<_, Note>(
        "INSERT INTO notes (id, kind, title, summary, body_json, body_html, body_text, content_hash,
                            created_at, updated_at)
              VALUES (?1, ?2, ?3, '', '', '', '', ?4, ?5, ?5)
           RETURNING id, kind, title, summary, body_json, body_html, body_text, content_hash,
                     distilled_at, last_recall_at, last_checked_at, last_recall_failed_at,
                     content_edited_at, content_hash_at_recall, source_note_id, journal_day, created_at, updated_at",
    )
    .bind(&id)
    .bind(kind)
    .bind(title)
    .bind(content_hash(""))
    .bind(now())
    .fetch_one(pool)
    .await?;

    Ok(note)
}

pub async fn get(pool: &SqlitePool, id: &str) -> Result<Option<Note>> {
    let note = sqlx::query_as::<_, Note>(
        "SELECT id, kind, title, summary, body_json, body_html, body_text, content_hash,
                distilled_at, last_recall_at, last_checked_at, last_recall_failed_at,
                content_edited_at, content_hash_at_recall, source_note_id, journal_day, created_at, updated_at
           FROM notes WHERE id = ?1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(note)
}

/// The capture inbox: notes never placed on any canvas. Falls out of the schema
/// rather than being a flag, because placement is membership.
pub async fn inbox(pool: &SqlitePool) -> Result<Vec<Note>> {
    let notes = sqlx::query_as::<_, Note>(
        "SELECT id, kind, title, summary, body_json, body_html, body_text, content_hash,
                distilled_at, last_recall_at, last_checked_at, last_recall_failed_at,
                content_edited_at, content_hash_at_recall, source_note_id, journal_day, created_at, updated_at
           FROM notes n
          WHERE NOT EXISTS (SELECT 1 FROM canvas_nodes c WHERE c.note_id = n.id)
          ORDER BY created_at DESC",
    )
    .fetch_all(pool)
    .await?;

    Ok(notes)
}

pub async fn for_topic(pool: &SqlitePool, topic_id: &str) -> Result<Vec<Note>> {
    let notes = sqlx::query_as::<_, Note>(
        "SELECT DISTINCT n.id, n.kind, n.title, n.summary, n.body_json, n.body_html, n.body_text,
                n.content_hash, n.distilled_at, n.last_recall_at, n.last_checked_at,
                n.last_recall_failed_at, n.content_edited_at, n.content_hash_at_recall,
                n.source_note_id, n.journal_day, n.created_at, n.updated_at
           FROM notes n
           JOIN canvas_nodes c ON c.note_id = n.id
          WHERE c.topic_id = ?1",
    )
    .bind(topic_id)
    .fetch_all(pool)
    .await?;

    Ok(notes)
}

/// An ordinary edit, with the source available.
///
/// Sets `content_edited_at`, which is what the cooling rule reads — so a blind
/// rewrite must never come through here. Also distils the note the first time
/// it gains a body: `raw` means captured and never written into, and that
/// should stop being true the moment you write something.
pub async fn save_body(
    pool: &SqlitePool,
    id: &str,
    title: &str,
    summary: &str,
    body_json: &str,
    body_html: &str,
    body_text: &str,
) -> Result<Note> {
    let note = sqlx::query_as::<_, Note>(
        "UPDATE notes
            SET title = ?2, summary = ?3,
                body_json = ?4, body_html = ?5, body_text = ?6,
                content_hash = ?7,
                content_edited_at = ?8,
                distilled_at = COALESCE(
                    distilled_at,
                    CASE WHEN trim(?6) <> '' THEN ?8 END
                ),
                -- Giving a dragged-out note its title is the act that makes it
                -- atomic. An untitled note cannot be atomic, because the title
                -- is the review front and a card with no front is not a card.
                -- SQLite evaluates the right-hand side against pre-update
                -- values, so `title` here is the old one.
                kind = CASE
                    WHEN kind = 'long'
                     AND source_note_id IS NOT NULL
                     AND trim(title) = ''
                     AND trim(?2) <> ''
                    THEN 'atomic'
                    ELSE kind
                END,
                updated_at = ?8
          WHERE id = ?1
      RETURNING id, kind, title, summary, body_json, body_html, body_text, content_hash,
                distilled_at, last_recall_at, last_checked_at, last_recall_failed_at,
                content_edited_at, content_hash_at_recall, source_note_id, journal_day, created_at, updated_at",
    )
    .bind(id)
    .bind(title)
    .bind(summary)
    .bind(body_json)
    .bind(body_html)
    .bind(body_text)
    .bind(content_hash(body_text))
    .bind(now())
    .fetch_one(pool)
    .await?;

    super::derived::reindex(pool, id, body_json).await?;

    Ok(note)
}

/// Step 2 of memory mode: the blind rewrite is saved.
///
/// Writes the body *and* the recall in one event, because in memory mode they
/// are the same act. `content_edited_at` is deliberately untouched — otherwise
/// every recall would immediately disqualify the next one.
///
/// The cooling rule lives here: if the note was edited normally within the last
/// day, the recall is still recorded, but `content_hash_at_recall` is left
/// alone so the note keeps reading as edited. The app records what happened and
/// declines to draw a conclusion from it; it does not refuse the action.
pub async fn record_recall(
    pool: &SqlitePool,
    id: &str,
    body_json: &str,
    body_html: &str,
    body_text: &str,
) -> Result<Note> {
    let ts = now();

    let current = get(pool, id)
        .await?
        .ok_or_else(|| crate::error::AppError::NotFound(format!("note {id}")))?;

    let cooling = current.recall_is_cooling(ts);

    let note = sqlx::query_as::<_, Note>(
        "UPDATE notes
            SET title = title,
                body_json = ?2, body_html = ?3, body_text = ?4,
                content_hash = ?5,
                last_recall_at = ?6,
                distilled_at = COALESCE(distilled_at, ?6),
                content_hash_at_recall = CASE WHEN ?7 THEN content_hash_at_recall ELSE ?5 END,
                updated_at = ?6
          WHERE id = ?1
      RETURNING id, kind, title, summary, body_json, body_html, body_text, content_hash,
                distilled_at, last_recall_at, last_checked_at, last_recall_failed_at,
                content_edited_at, content_hash_at_recall, source_note_id, journal_day, created_at, updated_at",
    )
    .bind(id)
    .bind(body_json)
    .bind(body_html)
    .bind(body_text)
    .bind(content_hash(body_text))
    .bind(ts)
    .bind(cooling)
    .fetch_one(pool)
    .await?;

    super::derived::reindex(pool, id, body_json).await?;

    Ok(note)
}

/// Step 4 of memory mode: the source was revealed and compared.
///
/// This is the only writer of `last_checked_at`, and it is a separate step by
/// design — you can close the sheet after saving and never contrast, which is
/// exactly why the two facts can legitimately diverge.
pub async fn record_contrast(pool: &SqlitePool, id: &str) -> Result<Note> {
    let note = sqlx::query_as::<_, Note>(
        "UPDATE notes SET last_checked_at = ?2, updated_at = ?2
          WHERE id = ?1
      RETURNING id, kind, title, summary, body_json, body_html, body_text, content_hash,
                distilled_at, last_recall_at, last_checked_at, last_recall_failed_at,
                content_edited_at, content_hash_at_recall, source_note_id, journal_day, created_at, updated_at",
    )
    .bind(id)
    .bind(now())
    .fetch_one(pool)
    .await?;

    Ok(note)
}

/// A review failure.
///
/// Records *when* rather than clearing the recall: "recalled but failing" and
/// "never recalled" are different facts and both matter. A later successful
/// recall supersedes this simply by being more recent.
pub async fn record_review_failure(pool: &SqlitePool, id: &str) -> Result<Note> {
    let note = sqlx::query_as::<_, Note>(
        "UPDATE notes SET last_recall_failed_at = ?2, updated_at = ?2
          WHERE id = ?1
      RETURNING id, kind, title, summary, body_json, body_html, body_text, content_hash,
                distilled_at, last_recall_at, last_checked_at, last_recall_failed_at,
                content_edited_at, content_hash_at_recall, source_note_id, journal_day, created_at, updated_at",
    )
    .bind(id)
    .bind(now())
    .fetch_one(pool)
    .await?;

    Ok(note)
}

pub async fn set_kind(pool: &SqlitePool, id: &str, kind: NoteKind) -> Result<Note> {
    let note = sqlx::query_as::<_, Note>(
        "UPDATE notes SET kind = ?2, updated_at = ?3
          WHERE id = ?1
      RETURNING id, kind, title, summary, body_json, body_html, body_text, content_hash,
                distilled_at, last_recall_at, last_checked_at, last_recall_failed_at,
                content_edited_at, content_hash_at_recall, source_note_id, journal_day, created_at, updated_at",
    )
    .bind(id)
    .bind(kind)
    .bind(now())
    .fetch_one(pool)
    .await?;

    Ok(note)
}

pub async fn delete(pool: &SqlitePool, id: &str) -> Result<()> {
    sqlx::query("DELETE FROM notes WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await?;

    Ok(())
}

/// Accent- and case-insensitive search over Spanish content.
///
/// Done in Rust rather than SQL because SQLite's `LIKE` has no useful notion of
/// Spanish folding, and because the fold has to keep `ñ` intact.
pub async fn search(pool: &SqlitePool, query: &str) -> Result<Vec<Note>> {
    use crate::note_state::matches_es;

    let all = sqlx::query_as::<_, Note>(
        "SELECT id, kind, title, summary, body_json, body_html, body_text, content_hash,
                distilled_at, last_recall_at, last_checked_at, last_recall_failed_at,
                content_edited_at, content_hash_at_recall, source_note_id, journal_day, created_at, updated_at
           FROM notes ORDER BY updated_at DESC",
    )
    .fetch_all(pool)
    .await?;

    Ok(all
        .into_iter()
        .filter(|n| matches_es(&n.title, query) || matches_es(&n.body_text, query))
        .collect())
}

// `COLS` documents the canonical column order the queries above spell out.
const _: &str = COLS;

/// Links the long note this one is being distilled from.
///
/// This is what the source pane reads, and therefore what memory mode removes
/// from the DOM. Setting it is an ordinary edit of metadata, not of content, so
/// it deliberately leaves `content_edited_at` alone: choosing a source must not
/// disqualify the recall you are about to attempt.
pub async fn set_source(pool: &SqlitePool, id: &str, source_note_id: Option<&str>) -> Result<Note> {
    if source_note_id == Some(id) {
        return Err(crate::error::AppError::Invalid(
            "a note cannot be its own source".into(),
        ));
    }

    let note = sqlx::query_as::<_, Note>(
        "UPDATE notes SET source_note_id = ?2, updated_at = ?3
          WHERE id = ?1
      RETURNING id, kind, title, summary, body_json, body_html, body_text, content_hash,
                distilled_at, last_recall_at, last_checked_at, last_recall_failed_at,
                content_edited_at, content_hash_at_recall, source_note_id, journal_day, created_at, updated_at",
    )
    .bind(id)
    .bind(source_note_id)
    .bind(now())
    .fetch_one(pool)
    .await?;

    Ok(note)
}

/// What the note library is filtered by.
///
/// Deliberately not a tag filter yet — tags are not an entity. `state` filters
/// on the memory axis only, and only ever narrows to notes that *have* one:
/// there is no "show me the undistilled", because that would be the pipeline
/// label reappearing as a filter.
#[derive(Debug, Default, serde::Deserialize)]
pub struct LibraryFilter {
    /// Accent-insensitive, Spanish-folded.
    pub query: Option<String>,
    /// Notes placed on this board.
    pub topic_id: Option<String>,
    /// The inbox: captured, never placed anywhere. A filter, not a place.
    pub unplaced_only: Option<bool>,
    /// Only notes that have been reproduced from memory.
    pub recalled_only: Option<bool>,
    /// Only notes carrying this tag. A filter over the corpus, never a place.
    pub tag_id: Option<String>,
}

/// The library.
///
/// Every note there is, newest first. Being on no canvas is the normal state of
/// a note, not a condition to be fixed — which is why "unplaced" is one filter
/// among several here rather than a separate screen with a count.
pub async fn library(pool: &SqlitePool, filter: &LibraryFilter) -> Result<Vec<Note>> {
    use crate::note_state::matches_es;

    let notes = if let Some(topic_id) = filter.topic_id.as_deref() {
        for_topic(pool, topic_id).await?
    } else if filter.unplaced_only.unwrap_or(false) {
        inbox(pool).await?
    } else {
        sqlx::query_as::<_, Note>(
            "SELECT id, kind, title, summary, body_json, body_html, body_text, content_hash,
                    distilled_at, last_recall_at, last_checked_at, last_recall_failed_at,
                    content_edited_at, content_hash_at_recall, source_note_id, journal_day,
                    created_at, updated_at
               FROM notes ORDER BY updated_at DESC",
        )
        .fetch_all(pool)
        .await?
    };

    let query = filter.query.as_deref().unwrap_or("").trim().to_owned();
    let recalled_only = filter.recalled_only.unwrap_or(false);

    // A tag narrows whatever scope is already chosen rather than replacing it,
    // so "this board, tagged X" is one thought and not two screens.
    let tagged: Option<std::collections::HashSet<String>> = match filter.tag_id.as_deref() {
        Some(tag_id) => Some(super::tags::note_ids(pool, tag_id).await?.into_iter().collect()),
        None => None,
    };

    Ok(notes
        .into_iter()
        .filter(|n| {
            if recalled_only && n.last_recall_at.is_none() {
                return false;
            }
            if tagged.as_ref().is_some_and(|ids| !ids.contains(&n.id)) {
                return false;
            }
            query.is_empty()
                || matches_es(&n.title, &query)
                || matches_es(&n.summary, &query)
                || matches_es(&n.body_text, &query)
        })
        .collect())
}

/// Which boards a note appears on.
///
/// Many-to-many by construction: boards do not own notes, they import them. A
/// note can sit on several at once, and this is what the Info slot lists.
pub async fn boards(pool: &SqlitePool, note_id: &str) -> Result<Vec<crate::domain::Topic>> {
    let topics = sqlx::query_as::<_, crate::domain::Topic>(
        "SELECT DISTINCT t.id, t.title, t.study, t.created_at, t.updated_at, t.last_visited_at
           FROM topics t
           JOIN canvas_nodes c ON c.topic_id = t.id
          WHERE c.note_id = ?1
          ORDER BY t.last_visited_at DESC",
    )
    .bind(note_id)
    .fetch_all(pool)
    .await?;

    Ok(topics)
}

/// Captures created on a given day, oldest first.
///
/// The journal is a **filter**, not a container. There is no day-document to
/// open and nothing is stored per day — appending to today just creates another
/// capture, which is why capture stays free of any decision about where a
/// thought goes.
///
/// Bounded by construction: one day at a time, reachable by date. There is no
/// scrollable archive of every day you have ever had, because that is an
/// accumulating surface wearing a calendar.
/// The page you write on for a given day, if you have written one.
///
/// Read-only, and absent until you type: visiting a date must not manufacture
/// an empty note, or a year of idle browsing would leave 365 of them behind.
pub async fn day_page(pool: &SqlitePool, day: &str) -> Result<Option<Note>> {
    let note = sqlx::query_as::<_, Note>(
        "SELECT id, kind, title, summary, body_json, body_html, body_text, content_hash,
                distilled_at, last_recall_at, last_checked_at, last_recall_failed_at,
                content_edited_at, content_hash_at_recall, source_note_id, journal_day,
                created_at, updated_at
           FROM notes WHERE journal_day = ?1",
    )
    .bind(day)
    .fetch_optional(pool)
    .await?;

    Ok(note)
}

/// Writes the day's page, creating it on the first keystroke.
///
/// Any day can be written, not only today. Yesterday is a record you are
/// allowed to correct — the thing the journal refuses is writing *forward*,
/// because a journal you can fill in ahead is a planner, and that is enforced
/// where days are chosen, not here.
///
/// The page is an ordinary note in every other respect: it can be distilled,
/// recalled, tagged and placed on a board. Only `journal_day` marks it.
pub async fn save_day_page(
    pool: &SqlitePool,
    day: &str,
    body_json: &str,
    body_html: &str,
    body_text: &str,
) -> Result<Note> {
    let id = match day_page(pool, day).await? {
        Some(existing) => existing.id,
        None => {
            let note = create(pool, None, "", crate::domain::NoteKind::Long).await?;
            sqlx::query("UPDATE notes SET journal_day = ?2 WHERE id = ?1")
                .bind(&note.id)
                .bind(day)
                .execute(pool)
                .await?;
            note.id
        }
    };

    // Title and summary stay empty: the date is the heading, and titling a note
    // is what makes it atomic. A day is not one concept.
    save_body(pool, &id, "", "", body_json, body_html, body_text).await
}

pub async fn captured_on(pool: &SqlitePool, day: chrono::NaiveDate) -> Result<Vec<Note>> {
    let start = day
        .and_hms_opt(0, 0, 0)
        .expect("midnight exists")
        .and_utc();
    let end = start + chrono::Duration::days(1);

    let notes = sqlx::query_as::<_, Note>(
        "SELECT id, kind, title, summary, body_json, body_html, body_text, content_hash,
                distilled_at, last_recall_at, last_checked_at, last_recall_failed_at,
                content_edited_at, content_hash_at_recall, source_note_id, journal_day,
                created_at, updated_at
           FROM notes
          WHERE created_at >= ?1 AND created_at < ?2
            AND journal_day IS NULL
          ORDER BY created_at",
    )
    .bind(start)
    .bind(end)
    .fetch_all(pool)
    .await?;

    Ok(notes)
}

/// Which days in a range have anything on them, so the week strip can mark
/// them without loading every note.
pub async fn days_with_captures(
    pool: &SqlitePool,
    from: chrono::NaiveDate,
    to: chrono::NaiveDate,
) -> Result<Vec<String>> {
    let start = from.and_hms_opt(0, 0, 0).expect("midnight").and_utc();
    let end = to.and_hms_opt(0, 0, 0).expect("midnight").and_utc() + chrono::Duration::days(1);

    let days = sqlx::query_scalar::<_, String>(
        "SELECT DISTINCT date(created_at) FROM notes
          WHERE created_at >= ?1 AND created_at < ?2",
    )
    .bind(start)
    .bind(end)
    .fetch_all(pool)
    .await?;

    Ok(days)
}
