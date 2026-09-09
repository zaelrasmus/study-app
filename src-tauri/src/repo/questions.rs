//! Questions: the engine of the loop, not a byproduct.

use sqlx::SqlitePool;

use crate::domain::{new_id, Question, QuestionOrigin, QuestionStatus};
use crate::error::{AppError, Result};

use super::now;

/// The queue, and the reason the ordering rule exists.
///
/// A gap found while reconstructing from memory outranks a review failure,
/// which outranks something jotted down while reading. Date never drives this
/// order; it only breaks ties inside a single origin bucket, and even then it
/// is the *topic's* recency rather than the question's age -- a question about
/// something you are working on right now is answerable, one from eight months
/// ago is not.
const QUEUE_SQL: &str = "SELECT q.id, q.text, q.status, q.origin, q.topic_id,
                                q.source_note_id, q.answer_note_id,
                                q.created_at, q.updated_at
                           FROM questions q
                           LEFT JOIN topics t ON t.id = q.topic_id
                          WHERE q.status IN ('open', 'investigating')
                            AND (?1 IS NULL OR q.topic_id = ?1)
                          ORDER BY CASE q.origin
                                     WHEN 'recall_gap'     THEN 0
                                     WHEN 'review_failure' THEN 1
                                     ELSE 2
                                   END,
                                   COALESCE(t.last_visited_at, '') DESC,
                                   q.created_at DESC";

pub async fn queue(pool: &SqlitePool, topic_id: Option<&str>) -> Result<Vec<Question>> {
    let questions = sqlx::query_as::<_, Question>(QUEUE_SQL)
        .bind(topic_id)
        .fetch_all(pool)
        .await?;

    Ok(questions)
}

/// Open questions that belong to no board.
///
/// A doubt caught while writing in the journal, or anywhere with nothing else
/// open, has no topic — and it is still a doubt. Filtering it out of every
/// board's panel made it findable only on one screen, which for a question you
/// had *while thinking about something* is the wrong place. So it is reachable
/// as its own scope rather than hidden.
pub async fn unattached(pool: &SqlitePool) -> Result<Vec<Question>> {
    let questions = sqlx::query_as::<_, Question>(
        "SELECT q.id, q.text, q.status, q.origin, q.topic_id,
                q.source_note_id, q.answer_note_id,
                q.created_at, q.updated_at
           FROM questions q
          WHERE q.status IN ('open', 'investigating')
            AND q.topic_id IS NULL
          ORDER BY CASE q.origin
                     WHEN 'recall_gap'     THEN 0
                     WHEN 'review_failure' THEN 1
                     ELSE 2
                   END,
                   q.created_at DESC",
    )
    .fetch_all(pool)
    .await?;

    Ok(questions)
}

pub async fn create(
    pool: &SqlitePool,
    id: Option<String>,
    text: &str,
    origin: QuestionOrigin,
    topic_id: Option<&str>,
    source_note_id: Option<&str>,
) -> Result<Question> {
    let id = id.unwrap_or_else(new_id);

    let question = sqlx::query_as::<_, Question>(
        "INSERT INTO questions (id, text, status, origin, topic_id, source_note_id,
                                created_at, updated_at)
              VALUES (?1, ?2, 'open', ?3, ?4, ?5, ?6, ?6)
           RETURNING id, text, status, origin, topic_id, source_note_id,
                     answer_note_id, created_at, updated_at",
    )
    .bind(&id)
    .bind(text)
    .bind(origin)
    .bind(topic_id)
    .bind(source_note_id)
    .bind(now())
    .fetch_one(pool)
    .await?;

    Ok(question)
}

/// Closing a question requires linking the note that answers it. This is the
/// one place in the app that refuses an action, and it refuses it because a
/// question closed without an answer is just a question you lost.
pub async fn resolve(pool: &SqlitePool, id: &str, answer_note_id: &str) -> Result<Question> {
    let exists = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM notes WHERE id = ?1")
        .bind(answer_note_id)
        .fetch_one(pool)
        .await?;

    if exists == 0 {
        return Err(AppError::Invalid(
            "a question can only be resolved by linking a note that answers it".into(),
        ));
    }

    let question = sqlx::query_as::<_, Question>(
        "UPDATE questions
            SET status = 'resolved', answer_note_id = ?2, updated_at = ?3
          WHERE id = ?1
      RETURNING id, text, status, origin, topic_id, source_note_id,
                answer_note_id, created_at, updated_at",
    )
    .bind(id)
    .bind(answer_note_id)
    .bind(now())
    .fetch_one(pool)
    .await?;

    Ok(question)
}

/// Abandon and dissolve are both one call, with no confirmation anywhere above
/// them. If abandoning costs anything, you hoard.
pub async fn set_status(pool: &SqlitePool, id: &str, status: QuestionStatus) -> Result<Question> {
    if status == QuestionStatus::Resolved {
        return Err(AppError::Invalid(
            "resolving requires an answer note; use resolve instead".into(),
        ));
    }

    let question = sqlx::query_as::<_, Question>(
        "UPDATE questions
            SET status = ?2, answer_note_id = NULL, updated_at = ?3
          WHERE id = ?1
      RETURNING id, text, status, origin, topic_id, source_note_id,
                answer_note_id, created_at, updated_at",
    )
    .bind(id)
    .bind(status)
    .bind(now())
    .fetch_one(pool)
    .await?;

    Ok(question)
}

pub async fn set_topic(pool: &SqlitePool, id: &str, topic_id: Option<&str>) -> Result<Question> {
    let question = sqlx::query_as::<_, Question>(
        "UPDATE questions SET topic_id = ?2, updated_at = ?3
          WHERE id = ?1
      RETURNING id, text, status, origin, topic_id, source_note_id,
                answer_note_id, created_at, updated_at",
    )
    .bind(id)
    .bind(topic_id)
    .bind(now())
    .fetch_one(pool)
    .await?;

    Ok(question)
}

/// Open-question counts are work, not debt, so this number is safe to show.
/// Anything that would only ever go down is not.
pub async fn open_count(pool: &SqlitePool, topic_id: Option<&str>) -> Result<i64> {
    let count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*)
           FROM questions
          WHERE status IN ('open', 'investigating')
            AND (?1 IS NULL OR topic_id = ?1)",
    )
    .bind(topic_id)
    .fetch_one(pool)
    .await?;

    Ok(count)
}
