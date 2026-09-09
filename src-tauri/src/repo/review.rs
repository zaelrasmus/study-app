//! The review queue.
//!
//! A card exists exactly when a note has been reproduced from memory. There is
//! nothing to author, so nothing here creates cards — it schedules notes.

use serde::Serialize;
use sqlx::SqlitePool;

use crate::domain::{Note, QuestionOrigin};
use crate::error::Result;
use crate::note_state::NoteFlags;
use crate::review::{due_after, interleave, schedule, Grade, ReviewState, DAILY_CAP};

use super::now;

/// One card, with everything the review screen needs to show it cold.
#[derive(Debug, Serialize)]
pub struct ReviewCard {
    #[serde(flatten)]
    pub note: Note,
    #[serde(flatten)]
    pub flags: NoteFlags,
    pub topic_id: Option<String>,
    pub topic_title: Option<String>,
    pub reps: i64,
}

#[derive(sqlx::FromRow)]
struct Candidate {
    #[sqlx(flatten)]
    note: Note,
    topic_id: Option<String>,
    topic_title: Option<String>,
    reps: Option<i64>,
}

/// Today's queue, and only today's.
///
/// Capped at [`DAILY_CAP`]. Nothing in this module returns the size of the
/// backlog behind the cap: a number that grows while you are away is the exact
/// accumulating-debt feeling the app exists to avoid, and in a review system it
/// is the classic reason people stop.
pub async fn queue(pool: &SqlitePool) -> Result<Vec<ReviewCard>> {
    let ts = now();

    let candidates = sqlx::query_as::<_, Candidate>(
        "SELECT n.id, n.kind, n.title, n.summary, n.body_json, n.body_html, n.body_text, n.content_hash,
                n.distilled_at, n.last_recall_at, n.last_checked_at, n.last_recall_failed_at,
                n.content_edited_at, n.content_hash_at_recall, n.source_note_id, n.journal_day,
                n.created_at, n.updated_at,
                t.id AS topic_id,
                t.title AS topic_title,
                r.reps AS reps
           FROM notes n
           LEFT JOIN canvas_nodes c ON c.note_id = n.id
           LEFT JOIN topics t ON t.id = c.topic_id
           LEFT JOIN review_state r ON r.note_id = n.id
          WHERE n.last_recall_at IS NOT NULL
            AND (r.due_at IS NULL OR r.due_at <= ?1)
          GROUP BY n.id
          ORDER BY COALESCE(r.due_at, n.last_recall_at)",
    )
    .bind(ts)
    .fetch_all(pool)
    .await?;

    // `may_review` lives in Rust because the state is derived, not stored.
    let cards: Vec<ReviewCard> = candidates
        .into_iter()
        .filter(|c| c.note.recall_state().may_review())
        .map(|c| ReviewCard {
            flags: c.note.flags(),
            note: c.note,
            topic_id: c.topic_id,
            topic_title: c.topic_title,
            reps: c.reps.unwrap_or(0),
        })
        .collect();

    let interleaved = interleave(cards, |c| c.topic_id.clone().unwrap_or_default());

    Ok(interleaved
        .into_iter()
        .take(DAILY_CAP as usize)
        .collect())
}

/// The number shown in the sidebar.
///
/// This is the size of today's queue, never the backlog — bounded work rather
/// than a debt that accrues.
pub async fn due_today(pool: &SqlitePool) -> Result<i64> {
    Ok(queue(pool).await?.len() as i64)
}

async fn state_for(pool: &SqlitePool, note_id: &str) -> Result<Option<ReviewState>> {
    let state = sqlx::query_as::<_, ReviewState>(
        "SELECT note_id, due_at, interval_days, ease, reps, lapses, last_reviewed_at,
                created_at, updated_at
           FROM review_state WHERE note_id = ?1",
    )
    .bind(note_id)
    .fetch_optional(pool)
    .await?;

    Ok(state)
}

/// Records a grade and moves the card.
///
/// A miss writes `last_recall_failed_at` on the note rather than clearing the
/// recall, so "recalled but failing" stays distinguishable from "never
/// recalled", and files a Question at the review-failure origin.
pub async fn grade(pool: &SqlitePool, note_id: &str, grade_value: Grade) -> Result<()> {
    let ts = now();
    let current = state_for(pool, note_id).await?;
    let next = schedule(current.as_ref(), grade_value);
    let due = due_after(ts, next.interval_days);

    sqlx::query(
        "INSERT INTO review_state (note_id, due_at, interval_days, ease, reps, lapses,
                                   last_reviewed_at, created_at, updated_at)
              VALUES (?1, ?2, ?3, ?4, 1, ?5, ?6, ?6, ?6)
         ON CONFLICT (note_id) DO UPDATE SET
              due_at = ?2,
              interval_days = ?3,
              ease = ?4,
              reps = review_state.reps + 1,
              lapses = review_state.lapses + ?5,
              last_reviewed_at = ?6,
              updated_at = ?6",
    )
    .bind(note_id)
    .bind(due)
    .bind(next.interval_days)
    .bind(next.ease)
    .bind(next.lapses_delta)
    .bind(ts)
    .execute(pool)
    .await?;

    if grade_value == Grade::Again {
        super::notes::record_review_failure(pool, note_id).await?;

        let note = super::notes::get(pool, note_id).await?;
        let title = note
            .as_ref()
            .map(|n| n.title.clone())
            .unwrap_or_else(|| "this note".into());

        let topic_id = sqlx::query_scalar::<_, Option<String>>(
            "SELECT topic_id FROM canvas_nodes WHERE note_id = ?1 LIMIT 1",
        )
        .bind(note_id)
        .fetch_optional(pool)
        .await?
        .flatten();

        super::questions::create(
            pool,
            None,
            &format!("Why did I miss: {title}?"),
            QuestionOrigin::ReviewFailure,
            topic_id.as_deref(),
            Some(note_id),
        )
        .await?;
    }

    Ok(())
}

/// The two outcomes offered for a card whose note has drifted since the recall.
///
/// Such a card is *not* suspended. Silent local decay plus suspension would
/// switch the review queue off without telling you — you would come back to an
/// empty queue and no explanation. Instead it appears flagged, and leaves by
/// being acted on rather than by being graded, so it never becomes an
/// ungradeable card that returns forever.
pub async fn resolve_drifted(pool: &SqlitePool, note_id: &str, still_holds: bool) -> Result<()> {
    if still_holds {
        // Contrasted by assertion: it is still right, so the flag clears and
        // normal scheduling resumes.
        super::notes::record_contrast(pool, note_id).await?;
        return grade(pool, note_id, Grade::Good).await;
    }

    let note = super::notes::get(pool, note_id).await?;
    let title = note
        .as_ref()
        .map(|n| n.title.clone())
        .unwrap_or_else(|| "this note".into());

    let topic_id = sqlx::query_scalar::<_, Option<String>>(
        "SELECT topic_id FROM canvas_nodes WHERE note_id = ?1 LIMIT 1",
    )
    .bind(note_id)
    .fetch_optional(pool)
    .await?
    .flatten();

    super::questions::create(
        pool,
        None,
        &format!("What changed in: {title}?"),
        QuestionOrigin::RecallGap,
        topic_id.as_deref(),
        Some(note_id),
    )
    .await?;

    // Deferred, not failed: you have not demonstrated anything about your
    // memory, only that the text moved.
    let ts = now();
    sqlx::query(
        "INSERT INTO review_state (note_id, due_at, interval_days, ease, reps, lapses,
                                   last_reviewed_at, created_at, updated_at)
              VALUES (?1, ?2, 1, 2.5, 0, 0, ?3, ?3, ?3)
         ON CONFLICT (note_id) DO UPDATE SET due_at = ?2, updated_at = ?3",
    )
    .bind(note_id)
    .bind(due_after(ts, 1.0))
    .bind(ts)
    .execute(pool)
    .await?;

    Ok(())
}
