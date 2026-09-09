//! Indexes derived from a note's body.
//!
//! Neither of these is a source of truth. The body is, and both tables are
//! rebuilt from it on every save — delete them and a re-save reconstructs them
//! exactly. They exist only because a TipTap document is opaque to SQL: you
//! cannot ask "what links here" or "what is due today" of a JSON blob.
//!
//! Keeping that clear matters, because the moment either table is edited
//! directly it stops being derived and starts being a second truth that can
//! disagree with the note.

use serde::Serialize;
use sqlx::SqlitePool;

use crate::error::Result;

/// A mention is a link whose target is a note rather than a URL.
///
/// Using the existing link mark rather than a bespoke node means mentions
/// survive copy-paste, export and every editor feature that already understands
/// links — and there is no new schema in the document to migrate later.
const MENTION_PREFIX: &str = "note:";

/// Pulls every mentioned note id out of a serialised TipTap document.
///
/// Scanning the JSON text is deliberate: it finds mentions wherever they are
/// nested — inside a list, a table cell, a callout — without walking every node
/// type the editor might grow later.
pub fn mentioned_ids(body_json: &str) -> Vec<String> {
    let mut ids = Vec::new();
    let mut rest = body_json;

    while let Some(at) = rest.find(MENTION_PREFIX) {
        rest = &rest[at + MENTION_PREFIX.len()..];

        let id: String = rest
            .chars()
            .take_while(|c| c.is_ascii_hexdigit() || *c == '-')
            .collect();

        // A UUID and nothing shorter, so a stray "note:" in prose is ignored.
        if id.len() == 36 && !ids.contains(&id) {
            ids.push(id);
        }
    }

    ids
}

/// One task, as it appears in a note.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Task {
    pub id: String,
    pub note_id: String,
    pub text: String,
    pub done: bool,
    /// Only set when the text carries an explicit `due:YYYY-MM-DD`.
    pub due_on: Option<String>,
    pub position: i64,
}

/// A task with enough context to be actionable from the panel.
///
/// `serde(flatten)` is load-bearing and was missing. Without it this serialised
/// as `{ task: { text, due_on, ... }, note_title }`, so every field the frontend
/// reads off a task came back `undefined` — which is why tasks appeared nowhere
/// even once the query returned them.
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct DueTask {
    #[serde(flatten)]
    #[sqlx(flatten)]
    pub task: Task,
    pub note_title: String,
    /// Set when the task lives on a day's page, which has no title. The date is
    /// what that note is called, so the UI can say where the task came from.
    pub journal_day: Option<String>,
}

/// The date a task carries, if it carries one.
///
/// `due:YYYY-MM-DD`, written inline. Explicit rather than parsed out of natural
/// language: a date the app guessed at is worse than no date.
///
/// A task without one is **not** invisible. It used to be — both queries asked
/// for one specific day — so a task typed into the day's page with no date was
/// indexed and then reachable from nowhere, which reads as the app losing your
/// writing. Undated tasks now live on the Tasks screen; the *panel* is what
/// stays bounded to a single day.
fn due_date(text: &str) -> Option<String> {
    let at = text.find("due:")?;
    let candidate: String = text[at + 4..].chars().take(10).collect();

    candidate
        .parse::<chrono::NaiveDate>()
        .ok()
        .map(|d| d.to_string())
}

/// Walks a TipTap document for `taskItem` nodes.
fn collect_tasks(node: &serde_json::Value, out: &mut Vec<(String, bool)>) {
    if node.get("type").and_then(|t| t.as_str()) == Some("taskItem") {
        let done = node
            .get("attrs")
            .and_then(|a| a.get("checked"))
            .and_then(|c| c.as_bool())
            .unwrap_or(false);

        let mut text = String::new();
        gather_text(node, &mut text);

        let text = text.trim().to_owned();
        if !text.is_empty() {
            out.push((text, done));
        }
        return;
    }

    if let Some(children) = node.get("content").and_then(|c| c.as_array()) {
        for child in children {
            collect_tasks(child, out);
        }
    }
}

fn gather_text(node: &serde_json::Value, out: &mut String) {
    if let Some(text) = node.get("text").and_then(|t| t.as_str()) {
        out.push_str(text);
    }

    if let Some(children) = node.get("content").and_then(|c| c.as_array()) {
        for child in children {
            gather_text(child, out);
        }
    }
}

/// Rebuilds both indexes for one note. Called on every body save.
pub async fn reindex(pool: &SqlitePool, note_id: &str, body_json: &str) -> Result<()> {
    let mut tx = pool.begin().await?;

    sqlx::query("DELETE FROM note_links WHERE from_note_id = ?1")
        .bind(note_id)
        .execute(&mut *tx)
        .await?;

    for target in mentioned_ids(body_json) {
        // A note mentioning itself is not a connection.
        if target == note_id {
            continue;
        }

        // The target may have been deleted since; the mention stays in the
        // prose, but there is no edge to record.
        sqlx::query(
            "INSERT OR IGNORE INTO note_links (from_note_id, to_note_id)
                  SELECT ?1, ?2 WHERE EXISTS (SELECT 1 FROM notes WHERE id = ?2)",
        )
        .bind(note_id)
        .bind(&target)
        .execute(&mut *tx)
        .await?;
    }

    sqlx::query("DELETE FROM note_tasks WHERE note_id = ?1")
        .bind(note_id)
        .execute(&mut *tx)
        .await?;

    if let Ok(doc) = serde_json::from_str::<serde_json::Value>(body_json) {
        let mut found = Vec::new();
        collect_tasks(&doc, &mut found);

        for (position, (text, done)) in found.into_iter().enumerate() {
            let due = due_date(&text);

            // Derived from where the task sits, not random.
            //
            // Every reindex deletes and reinserts these rows, so a fresh id each
            // time meant any id the interface was holding went stale the moment
            // anything in the note changed — and the next call using it did
            // nothing at all, silently. A position-derived id survives the
            // rebuild, which is what makes ticking a box twice work.
            sqlx::query(
                "INSERT INTO note_tasks (id, note_id, text, done, due_on, position)
                      VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            )
            .bind(format!("{note_id}:{position}"))
            .bind(note_id)
            .bind(&text)
            .bind(done)
            .bind(due)
            .bind(position as i64)
            .execute(&mut *tx)
            .await?;
        }
    }

    tx.commit().await?;
    Ok(())
}

/// Flips the `checked` attribute of the *n*th task item in a document.
///
/// Position is the index the reindex assigned, which is document order, so the
/// same walk finds the same item.
fn check_nth(node: &mut serde_json::Value, target: usize, done: bool, seen: &mut usize) -> bool {
    if node.get("type").and_then(|t| t.as_str()) == Some("taskItem") {
        if *seen == target {
            if let Some(attrs) = node.get_mut("attrs").and_then(|a| a.as_object_mut()) {
                attrs.insert("checked".into(), serde_json::Value::Bool(done));
            } else if let Some(obj) = node.as_object_mut() {
                obj.insert("attrs".into(), serde_json::json!({ "checked": done }));
            }
            return true;
        }
        *seen += 1;
        return false;
    }

    if let Some(children) = node.get_mut("content").and_then(|c| c.as_array_mut()) {
        for child in children {
            if check_nth(child, target, done, seen) {
                return true;
            }
        }
    }

    false
}

/// The same flip, in the rendered HTML.
///
/// The document view renders `body_html` directly, so leaving it behind would
/// show a box that disagrees with the note. TipTap emits each item as
/// `<li data-checked="false" data-type="taskItem"><label><input type="checkbox">`,
/// and both halves have to move: the attribute is what the stylesheet reads,
/// the input is what actually draws ticked.
///
/// This writes one fixed shape for each state rather than trying to reproduce
/// whatever TipTap emitted, so the result is self-consistent either way — and
/// the next real edit in the editor rewrites the whole document regardless.
fn check_nth_html(html: &str, target: usize, done: bool) -> String {
    const MARK: &str = "data-checked=\"";

    let mut out = String::with_capacity(html.len() + 16);
    let mut rest = html;
    let mut seen = 0usize;

    while let Some(at) = rest.find(MARK) {
        let value_start = at + MARK.len();
        let Some(close) = rest[value_start..].find('"') else { break };
        let after_value = value_start + close + 1;

        if seen != target {
            seen += 1;
            out.push_str(&rest[..after_value]);
            rest = &rest[after_value..];
            continue;
        }

        out.push_str(&rest[..at]);
        out.push_str(MARK);
        out.push_str(if done { "true" } else { "false" });
        out.push('"');

        // The checkbox belongs to this item, so only the first one after the
        // attribute is rewritten; anything later is a different task.
        let tail = &rest[after_value..];
        let replaced = match tail.find("<input type=\"checkbox\"") {
            Some(input_at) => {
                let end = tail[input_at..].find('>').map(|e| input_at + e + 1);
                match end {
                    Some(end) => {
                        let mut piece = String::new();
                        piece.push_str(&tail[..input_at]);
                        piece.push_str(if done {
                            "<input type=\"checkbox\" checked=\"checked\">"
                        } else {
                            "<input type=\"checkbox\">"
                        });
                        piece.push_str(&tail[end..]);
                        Some(piece)
                    }
                    None => None,
                }
            }
            None => None,
        };

        out.push_str(&replaced.unwrap_or_else(|| tail.to_owned()));
        return out;
    }

    out.push_str(rest);
    out
}

/// Ticks or unticks a task, in the note that owns it.
///
/// The body stays the single truth: this edits the document and rebuilds the
/// index from it, exactly as saving from the editor would.
///
/// It deliberately does **not** touch `content_edited_at`. Ticking a box does
/// not change a word of the note — the content hash is taken over the plaintext
/// and comes out identical — so counting it as an edit would start the
/// twenty-four hour cooling window and quietly block a recall you had earned.
pub async fn set_done(pool: &SqlitePool, task_id: &str, done: bool) -> Result<()> {
    let Some((note_id, position)) = sqlx::query_as::<_, (String, i64)>(
        "SELECT note_id, position FROM note_tasks WHERE id = ?1",
    )
    .bind(task_id)
    .fetch_optional(pool)
    .await?
    else {
        return Ok(());
    };

    let Some((body_json, body_html)) = sqlx::query_as::<_, (String, String)>(
        "SELECT body_json, body_html FROM notes WHERE id = ?1",
    )
    .bind(&note_id)
    .fetch_optional(pool)
    .await?
    else {
        return Ok(());
    };

    let Ok(mut doc) = serde_json::from_str::<serde_json::Value>(&body_json) else {
        return Ok(());
    };

    let mut seen = 0usize;
    if !check_nth(&mut doc, position as usize, done, &mut seen) {
        return Ok(());
    }

    let html = check_nth_html(&body_html, position as usize, done);
    // Serialising a document we just parsed cannot fail in practice; if it
    // somehow did, leaving the note untouched is the right answer.
    let Ok(json) = serde_json::to_string(&doc) else { return Ok(()) };

    sqlx::query("UPDATE notes SET body_json = ?2, body_html = ?3, updated_at = ?4 WHERE id = ?1")
        .bind(&note_id)
        .bind(&json)
        .bind(&html)
        .bind(super::now())
        .execute(pool)
        .await?;

    reindex(pool, &note_id, &json).await
}

/// Notes that mention this one.
pub async fn backlinks(pool: &SqlitePool, note_id: &str) -> Result<Vec<crate::domain::Note>> {
    let notes = sqlx::query_as::<_, crate::domain::Note>(
        "SELECT n.id, n.kind, n.title, n.summary, n.body_json, n.body_html, n.body_text,
                n.content_hash, n.distilled_at, n.last_recall_at, n.last_checked_at,
                n.last_recall_failed_at, n.content_edited_at, n.content_hash_at_recall,
                n.source_note_id, n.journal_day, n.created_at, n.updated_at
           FROM notes n
           JOIN note_links l ON l.from_note_id = n.id
          WHERE l.to_note_id = ?1
          ORDER BY n.updated_at DESC",
    )
    .bind(note_id)
    .fetch_all(pool)
    .await?;

    Ok(notes)
}

/// What is on for one day.
///
/// Two slices, both a single day wide:
///
/// 1. Tasks **dated** for that day, wherever they were written.
/// 2. Tasks written **on that day's own page**, dated or not.
///
/// The second exists because of a real dead end: you type three checkboxes into
/// today's journal, and the Tasks panel sitting beside them is empty — which
/// reads as the app not seeing what is plainly on screen. They are still bounded
/// to one day, so this is "what is on for this day", never "everything
/// outstanding". A task written on Tuesday's page does not follow you into
/// Wednesday unless you gave it a date.
pub async fn due_on(pool: &SqlitePool, day: &str) -> Result<Vec<DueTask>> {
    let tasks = sqlx::query_as::<_, DueTask>(
        "SELECT t.id, t.note_id, t.text, t.done, t.due_on, t.position,
                n.title AS note_title, n.journal_day
           FROM note_tasks t
           JOIN notes n ON n.id = t.note_id
          WHERE t.done = 0
            AND (t.due_on = ?1 OR (t.due_on IS NULL AND n.journal_day = ?1))
          ORDER BY t.due_on IS NULL, n.title, t.position",
    )
    .bind(day)
    .fetch_all(pool)
    .await?;

    Ok(tasks)
}

/// Every open task, dated or not.
///
/// The Tasks screen shows all of them because that is what a screen called
/// Tasks is for, and because a task you wrote and cannot find again is worse
/// than a list that is longer than you would like. The *panel* is the bounded
/// view: it stays one day wide, which is where the "a day's worth is work, a
/// backlog is a reproach" rule still applies.
pub async fn all_open(pool: &SqlitePool) -> Result<Vec<DueTask>> {
    let tasks = sqlx::query_as::<_, DueTask>(
        "SELECT t.id, t.note_id, t.text, t.done, t.due_on, t.position,
                n.title AS note_title, n.journal_day
           FROM note_tasks t
           JOIN notes n ON n.id = t.note_id
          WHERE t.done = 0
          ORDER BY t.due_on IS NULL, t.due_on, n.title, t.position",
    )
    .fetch_all(pool)
    .await?;

    Ok(tasks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mentions_are_found_wherever_they_are_nested() {
        let id = "3f2504e0-4f89-41d3-9a0c-0305e82c3301";
        let json = format!(
            r#"{{"content":[{{"content":[{{"marks":[{{"attrs":{{"href":"note:{id}"}}}}]}}]}}]}}"#
        );

        assert_eq!(mentioned_ids(&json), vec![id.to_string()]);
    }

    #[test]
    fn the_same_mention_twice_is_one_edge() {
        let id = "3f2504e0-4f89-41d3-9a0c-0305e82c3301";
        let json = format!(r#"["note:{id}", "note:{id}"]"#);
        assert_eq!(mentioned_ids(&json).len(), 1);
    }

    #[test]
    fn prose_that_merely_says_note_is_not_a_mention() {
        assert!(mentioned_ids(r#"{"text":"see note: the appendix"}"#).is_empty());
        assert!(mentioned_ids(r#"{"text":"note:1234"}"#).is_empty());
    }

    #[test]
    fn a_task_without_a_date_never_surfaces() {
        assert_eq!(due_date("comprar pan"), None);
        assert_eq!(due_date("comprar pan due:mañana"), None);
    }

    #[test]
    fn an_explicit_date_is_read() {
        assert_eq!(due_date("revisar el capítulo due:2026-09-10").as_deref(), Some("2026-09-10"));
    }

    #[test]
    fn tasks_are_collected_from_anywhere_in_the_document() {
        let doc: serde_json::Value = serde_json::from_str(
            r#"{"type":"doc","content":[
                {"type":"taskList","content":[
                    {"type":"taskItem","attrs":{"checked":false},
                     "content":[{"type":"paragraph","content":[{"type":"text","text":"uno"}]}]},
                    {"type":"taskItem","attrs":{"checked":true},
                     "content":[{"type":"paragraph","content":[{"type":"text","text":"dos"}]}]}
                ]}]}"#,
        )
        .unwrap();

        let mut found = Vec::new();
        collect_tasks(&doc, &mut found);

        assert_eq!(found, vec![("uno".into(), false), ("dos".into(), true)]);
    }
}
