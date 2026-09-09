//! The canvas: frames, nodes, and the document assembled from them.

use serde::Serialize;
use sqlx::SqlitePool;

use super::assets::Asset;
use crate::domain::{new_id, CanvasEdge, CanvasNode, CanvasNodeKind, Frame, Note, Question};
use crate::error::{AppError, Result};

use super::now;

/// Everything needed to render one topic's canvas in a single round trip.
#[derive(Debug, Serialize)]
pub struct CanvasSnapshot {
    pub frames: Vec<Frame>,
    pub nodes: Vec<CanvasNode>,
    pub edges: Vec<CanvasEdge>,
    /// The files any of those nodes point at, so an image can be drawn without
    /// a second round trip per card.
    pub assets: Vec<Asset>,
    /// The questions placed on this board, for the same reason.
    pub questions: Vec<Question>,
}

pub async fn snapshot(pool: &SqlitePool, topic_id: &str) -> Result<CanvasSnapshot> {
    let frames = sqlx::query_as::<_, Frame>(
        "SELECT id, topic_id, label, x, y, width, height, order_index, created_at, updated_at
           FROM frames WHERE topic_id = ?1 ORDER BY order_index",
    )
    .bind(topic_id)
    .fetch_all(pool)
    .await?;

    let nodes = sqlx::query_as::<_, CanvasNode>(
        "SELECT id, topic_id, kind, x, y, width, height, z, note_id, target_topic_id,
                asset_id, question_id, ink, text, url, color, frame_id, order_in_frame, created_at, updated_at
           FROM canvas_nodes WHERE topic_id = ?1 ORDER BY z, created_at",
    )
    .bind(topic_id)
    .fetch_all(pool)
    .await?;

    let edges = sqlx::query_as::<_, CanvasEdge>(
        "SELECT id, topic_id, source_id, target_id, label, created_at
           FROM canvas_edges WHERE topic_id = ?1 ORDER BY created_at",
    )
    .bind(topic_id)
    .fetch_all(pool)
    .await?;

    let assets = super::assets::for_topic(pool, topic_id).await?;

    let questions = sqlx::query_as::<_, Question>(
        "SELECT DISTINCT q.id, q.text, q.status, q.origin, q.topic_id, q.source_note_id,
                q.answer_note_id, q.created_at, q.updated_at
           FROM questions q
           JOIN canvas_nodes c ON c.question_id = q.id
          WHERE c.topic_id = ?1",
    )
    .bind(topic_id)
    .fetch_all(pool)
    .await?;

    Ok(CanvasSnapshot {
        frames,
        nodes,
        edges,
        assets,
        questions,
    })
}

// -- pointers -------------------------------------------------------------

/// Draws a pointer between two nodes on the same board.
///
/// Idempotent on the ordered pair: drawing the same arrow twice is not two
/// facts, so the second attempt returns the first arrow rather than failing.
pub async fn connect(
    pool: &SqlitePool,
    id: Option<String>,
    source_id: &str,
    target_id: &str,
) -> Result<CanvasEdge> {
    if source_id == target_id {
        return Err(AppError::Invalid("a card cannot point at itself".into()));
    }

    // The board comes from the nodes, so an arrow can never span two boards.
    let topics: Vec<String> = sqlx::query_scalar(
        "SELECT topic_id FROM canvas_nodes WHERE id IN (?1, ?2)",
    )
    .bind(source_id)
    .bind(target_id)
    .fetch_all(pool)
    .await?;

    let [a, b] = topics.as_slice() else {
        return Err(AppError::Invalid("both ends must be on the board".into()));
    };
    if a != b {
        return Err(AppError::Invalid("a pointer cannot span two boards".into()));
    }

    if let Some(existing) = sqlx::query_as::<_, CanvasEdge>(
        "SELECT id, topic_id, source_id, target_id, label, created_at
           FROM canvas_edges WHERE source_id = ?1 AND target_id = ?2",
    )
    .bind(source_id)
    .bind(target_id)
    .fetch_optional(pool)
    .await?
    {
        return Ok(existing);
    }

    let edge = sqlx::query_as::<_, CanvasEdge>(
        "INSERT INTO canvas_edges (id, topic_id, source_id, target_id, label, created_at)
              VALUES (?1, ?2, ?3, ?4, '', ?5)
           RETURNING id, topic_id, source_id, target_id, label, created_at",
    )
    .bind(id.unwrap_or_else(new_id))
    .bind(a)
    .bind(source_id)
    .bind(target_id)
    .bind(now())
    .fetch_one(pool)
    .await?;

    Ok(edge)
}

pub async fn label_edge(pool: &SqlitePool, id: &str, label: &str) -> Result<()> {
    sqlx::query("UPDATE canvas_edges SET label = ?2 WHERE id = ?1")
        .bind(id)
        .bind(label.trim())
        .execute(pool)
        .await?;

    Ok(())
}

/// One click, no confirmation. An arrow is an arrangement, not a document.
pub async fn disconnect(pool: &SqlitePool, id: &str) -> Result<()> {
    sqlx::query("DELETE FROM canvas_edges WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await?;

    Ok(())
}

// -- frames ---------------------------------------------------------------

/// New frames take the next order slot. The number is shown on the frame, so
/// the ordering is never a hidden fact the canvas disagrees with.
#[allow(clippy::too_many_arguments)]
pub async fn create_frame(
    pool: &SqlitePool,
    id: Option<String>,
    topic_id: &str,
    label: &str,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<Frame> {
    let id = id.unwrap_or_else(new_id);

    let next = sqlx::query_scalar::<_, i64>(
        "SELECT COALESCE(MAX(order_index), -1) + 1 FROM frames WHERE topic_id = ?1",
    )
    .bind(topic_id)
    .fetch_one(pool)
    .await?;

    let frame = sqlx::query_as::<_, Frame>(
        "INSERT INTO frames (id, topic_id, label, x, y, width, height, order_index,
                             created_at, updated_at)
              VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)
           RETURNING id, topic_id, label, x, y, width, height, order_index,
                     created_at, updated_at",
    )
    .bind(&id)
    .bind(topic_id)
    .bind(label)
    .bind(x)
    .bind(y)
    .bind(width)
    .bind(height)
    .bind(next)
    .bind(now())
    .fetch_one(pool)
    .await?;

    Ok(frame)
}

/// Geometry only. Moving or resizing a frame deliberately does not touch
/// `order_index`, which is what keeps rearranging the canvas from rewriting the
/// document.
pub async fn move_frame(
    pool: &SqlitePool,
    id: &str,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<()> {
    sqlx::query(
        "UPDATE frames SET x = ?2, y = ?3, width = ?4, height = ?5, updated_at = ?6
          WHERE id = ?1",
    )
    .bind(id)
    .bind(x)
    .bind(y)
    .bind(width)
    .bind(height)
    .bind(now())
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn label_frame(pool: &SqlitePool, id: &str, label: &str) -> Result<()> {
    sqlx::query("UPDATE frames SET label = ?2, updated_at = ?3 WHERE id = ?1")
        .bind(id)
        .bind(label)
        .bind(now())
        .execute(pool)
        .await?;

    Ok(())
}

/// Reorder is explicit and deliberate: the caller sends the full ordered list.
/// This is the only thing that changes document order.
pub async fn reorder_frames(pool: &SqlitePool, topic_id: &str, ids: &[String]) -> Result<()> {
    let ts = now();
    let mut tx = pool.begin().await?;

    for (index, id) in ids.iter().enumerate() {
        sqlx::query(
            "UPDATE frames SET order_index = ?3, updated_at = ?4
              WHERE id = ?1 AND topic_id = ?2",
        )
        .bind(id)
        .bind(topic_id)
        .bind(index as i64)
        .bind(ts)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

/// Deleting a frame releases its nodes back to the canvas rather than deleting
/// them: `canvas_nodes.frame_id` is ON DELETE SET NULL. Losing a narrative
/// should never cost you the thinking.
pub async fn delete_frame(pool: &SqlitePool, id: &str) -> Result<()> {
    sqlx::query("DELETE FROM frames WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await?;

    Ok(())
}

// -- nodes ----------------------------------------------------------------

const INSERT_NODE: &str = "INSERT INTO canvas_nodes
        (id, topic_id, kind, x, y, z, note_id, target_topic_id, asset_id, question_id,
         ink, text, url, created_at, updated_at)
        VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?13)
     RETURNING id, topic_id, kind, x, y, width, height, z, note_id, target_topic_id,
               asset_id, question_id, ink, text, url, color, frame_id, order_in_frame, created_at, updated_at";

#[allow(clippy::too_many_arguments)]
async fn insert_node(
    pool: &SqlitePool,
    id: Option<String>,
    topic_id: &str,
    kind: CanvasNodeKind,
    x: f64,
    y: f64,
    note_id: Option<&str>,
    target_topic_id: Option<&str>,
    asset_id: Option<&str>,
    question_id: Option<&str>,
    ink: Option<&str>,
    text: Option<&str>,
    url: Option<&str>,
) -> Result<CanvasNode> {
    let id = id.unwrap_or_else(new_id);

    let node = sqlx::query_as::<_, CanvasNode>(INSERT_NODE)
        .bind(&id)
        .bind(topic_id)
        .bind(kind)
        .bind(x)
        .bind(y)
        .bind(note_id)
        .bind(target_topic_id)
        .bind(asset_id)
        .bind(question_id)
        .bind(ink)
        .bind(text)
        .bind(url)
        .bind(now())
        .fetch_one(pool)
        .await?;

    Ok(node)
}

/// Places an existing note on a topic's canvas. Because placement is
/// membership, this is also what makes the note part of the topic.
pub async fn add_note_node(
    pool: &SqlitePool,
    id: Option<String>,
    topic_id: &str,
    note_id: &str,
    x: f64,
    y: f64,
) -> Result<CanvasNode> {
    insert_node(
        pool,
        id,
        topic_id,
        CanvasNodeKind::Note,
        x,
        y,
        Some(note_id),
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .await
}

/// Places a whole topic on another topic's canvas. This is zoom, not a folder
/// tree -- but it still has to not eat itself, hence the cycle check.
pub async fn add_topic_node(
    pool: &SqlitePool,
    id: Option<String>,
    topic_id: &str,
    target_topic_id: &str,
    x: f64,
    y: f64,
) -> Result<CanvasNode> {
    if topic_id == target_topic_id {
        return Err(AppError::Invalid("a topic cannot contain itself".into()));
    }

    if reaches(pool, target_topic_id, topic_id).await? {
        return Err(AppError::Invalid(
            "that would nest these topics inside each other".into(),
        ));
    }

    insert_node(
        pool,
        id,
        topic_id,
        CanvasNodeKind::Topic,
        x,
        y,
        None,
        Some(target_topic_id),
        None,
        None,
        None,
        None,
        None,
    )
    .await
}

pub async fn add_asset_node(
    pool: &SqlitePool,
    id: Option<String>,
    topic_id: &str,
    asset_id: &str,
    kind: CanvasNodeKind,
    x: f64,
    y: f64,
) -> Result<CanvasNode> {
    if !matches!(kind, CanvasNodeKind::Image | CanvasNodeKind::Pdf) {
        return Err(AppError::Invalid("asset nodes must be image or pdf".into()));
    }

    insert_node(pool, id, topic_id, kind, x, y, None, None, Some(asset_id), None, None, None, None).await
}

/// `ink` is the serialised stroke: the input points plus the settings the
/// outline was generated with, so a stroke can be re-rendered at any zoom
/// rather than baked into a bitmap.
pub async fn add_ink_node(
    pool: &SqlitePool,
    id: Option<String>,
    topic_id: &str,
    ink: &str,
    x: f64,
    y: f64,
) -> Result<CanvasNode> {
    insert_node(
        pool,
        id,
        topic_id,
        CanvasNodeKind::Ink,
        x,
        y,
        None,
        None,
        None,
        None,
        Some(ink),
        None,
        None,
    )
    .await
}

/// Loose text: a thought written straight onto the board.
///
/// No title, no knowledge state, no card — and no note behind it. It exists so
/// that thinking out loud beside your cards does not first cost you a decision
/// about whether the thought is a note. When it turns out to be one,
/// [`promote_text_node`] makes it one without moving it.
pub async fn add_text_node(
    pool: &SqlitePool,
    id: Option<String>,
    topic_id: &str,
    text: &str,
    x: f64,
    y: f64,
) -> Result<CanvasNode> {
    insert_node(
        pool,
        id,
        topic_id,
        CanvasNodeKind::Text,
        x,
        y,
        None,
        None,
        None,
        None,
        None,
        Some(text),
        None,
    )
    .await
}

/// A page on the web, placed on the board.
///
/// Deliberately not an asset: nothing is copied, and the page can change under
/// you. It is a pointer outwards, which is a different claim from "I have this".
pub async fn add_link_node(
    pool: &SqlitePool,
    id: Option<String>,
    topic_id: &str,
    url: &str,
    x: f64,
    y: f64,
) -> Result<CanvasNode> {
    let url = url.trim();
    if url.is_empty() {
        return Err(AppError::Invalid("a link needs an address".into()));
    }

    // Typing "example.com" means the web, not a relative path.
    let url = if url.contains("://") {
        url.to_owned()
    } else {
        format!("https://{url}")
    };

    insert_node(
        pool,
        id,
        topic_id,
        CanvasNodeKind::Link,
        x,
        y,
        None,
        None,
        None,
        None,
        None,
        None,
        Some(&url),
    )
    .await
}

/// Puts an open question on a board, beside the cards that might answer it.
///
/// Placement is membership here too: a question caught while writing has no
/// board, and dropping it on a canvas is what gives it one. That is the whole
/// reason a doubt you wrote down while thinking about a topic can afterwards be
/// found from that topic.
pub async fn add_question_node(
    pool: &SqlitePool,
    id: Option<String>,
    topic_id: &str,
    question_id: &str,
    x: f64,
    y: f64,
) -> Result<CanvasNode> {
    // The question now belongs to this board, so the board's panel finds it.
    super::questions::set_topic(pool, question_id, Some(topic_id)).await?;

    insert_node(
        pool,
        id,
        topic_id,
        CanvasNodeKind::Question,
        x,
        y,
        None,
        None,
        None,
        Some(question_id),
        None,
        None,
        None,
    )
    .await
}

pub async fn set_node_text(pool: &SqlitePool, id: &str, text: &str) -> Result<()> {
    sqlx::query("UPDATE canvas_nodes SET text = ?2, updated_at = ?3 WHERE id = ?1 AND kind = 'text'")
        .bind(id)
        .bind(text)
        .bind(now())
        .execute(pool)
        .await?;

    Ok(())
}

/// Turns loose text into a note, in place.
///
/// The same node, the same position, the same board — only its kind changes. A
/// thought earns a title when you decide it has one, and the arrangement you
/// already made around it should survive that decision untouched.
///
/// The text becomes the body, not the title: it is what you actually wrote.
/// Titling it is a separate act, and it is the act that makes a note atomic.
pub async fn promote_text_node(pool: &SqlitePool, id: &str) -> Result<(Note, CanvasNode)> {
    let text: String = sqlx::query_scalar("SELECT text FROM canvas_nodes WHERE id = ?1 AND kind = 'text'")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::Invalid("that node is not loose text".into()))?;

    let note = super::notes::create(pool, None, "", crate::domain::NoteKind::Long).await?;

    let body = text.trim();
    super::notes::save_body(
        pool,
        &note.id,
        "",
        "",
        "{}",
        &format!("<p>{}</p>", html_escape(body)),
        body,
    )
    .await?;

    let node = sqlx::query_as::<_, CanvasNode>(
        "UPDATE canvas_nodes
            SET kind = 'note', note_id = ?2, text = NULL, updated_at = ?3
          WHERE id = ?1
      RETURNING id, topic_id, kind, x, y, width, height, z, note_id, target_topic_id,
                asset_id, question_id, ink, text, url, color, frame_id, order_in_frame, created_at, updated_at",
    )
    .bind(id)
    .bind(&note.id)
    .bind(now())
    .fetch_one(pool)
    .await?;

    let note = super::notes::get(pool, &note.id)
        .await?
        .ok_or_else(|| AppError::Invalid("the promoted note vanished".into()))?;

    Ok((note, node))
}

/// Minimal, and enough: the body is plain text taken from a textarea, so the
/// only characters that can break the markup are these three.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

pub async fn move_node(
    pool: &SqlitePool,
    id: &str,
    x: f64,
    y: f64,
    width: Option<f64>,
    height: Option<f64>,
) -> Result<()> {
    sqlx::query(
        "UPDATE canvas_nodes
            SET x = ?2, y = ?3,
                width  = COALESCE(?4, width),
                height = COALESCE(?5, height),
                updated_at = ?6
          WHERE id = ?1",
    )
    .bind(id)
    .bind(x)
    .bind(y)
    .bind(width)
    .bind(height)
    .bind(now())
    .execute(pool)
    .await?;

    Ok(())
}

/// Putting a node in a frame is the act of committing it to the document;
/// taking it out returns it to scratch. Both are one drag, and neither
/// destroys anything.
pub async fn set_node_frame(
    pool: &SqlitePool,
    id: &str,
    frame_id: Option<&str>,
    order_in_frame: Option<i64>,
) -> Result<()> {
    sqlx::query(
        "UPDATE canvas_nodes SET frame_id = ?2, order_in_frame = ?3, updated_at = ?4
          WHERE id = ?1",
    )
    .bind(id)
    .bind(frame_id)
    .bind(order_in_frame)
    .bind(now())
    .execute(pool)
    .await?;

    Ok(())
}

/// Removes the node from the canvas. The note itself survives -- unplacing is
/// not deleting, and a note with no placement is simply back in the inbox.
pub async fn delete_node(pool: &SqlitePool, id: &str) -> Result<()> {
    sqlx::query("DELETE FROM canvas_nodes WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await?;

    Ok(())
}

/// True when `to` is reachable from `from` by descending topic nodes.
async fn reaches(pool: &SqlitePool, from: &str, to: &str) -> Result<bool> {
    let hit = sqlx::query_scalar::<_, i64>(
        "WITH RECURSIVE reachable(id) AS (
             SELECT ?1
             UNION
             SELECT c.target_topic_id
               FROM canvas_nodes c
               JOIN reachable r ON c.topic_id = r.id
              WHERE c.kind = 'topic' AND c.target_topic_id IS NOT NULL
         )
         SELECT COUNT(*) FROM reachable WHERE id = ?2",
    )
    .bind(from)
    .bind(to)
    .fetch_one(pool)
    .await?;

    Ok(hit > 0)
}

// -- document -------------------------------------------------------------

/// One frame's worth of document. The frame is the unit of completion: it has
/// visible edges, so it can actually be finished. The topic-level document is
/// only ever the concatenation of these and is never a task in itself.
#[derive(Debug, Serialize)]
pub struct DocumentSection {
    pub frame: Frame,
    pub notes: Vec<Note>,
}

/// Assembles the readable document from the arrangement.
///
/// Order comes from `frames.order_index` and `canvas_nodes.order_in_frame` --
/// never from raw coordinates. Nodes outside every frame are absent by
/// construction, which is what makes the canvas a superset of the document.
pub async fn document(pool: &SqlitePool, topic_id: &str) -> Result<Vec<DocumentSection>> {
    let frames = sqlx::query_as::<_, Frame>(
        "SELECT id, topic_id, label, x, y, width, height, order_index, created_at, updated_at
           FROM frames WHERE topic_id = ?1 ORDER BY order_index",
    )
    .bind(topic_id)
    .fetch_all(pool)
    .await?;

    let mut sections = Vec::with_capacity(frames.len());

    for frame in frames {
        let notes = sqlx::query_as::<_, Note>(
            "SELECT n.id, n.kind, n.title, n.summary, n.body_json, n.body_html, n.body_text,
                    n.content_hash, n.distilled_at, n.last_recall_at, n.last_checked_at,
                    n.last_recall_failed_at, n.content_edited_at, n.content_hash_at_recall, n.source_note_id,
                    n.journal_day, n.created_at, n.updated_at
               FROM notes n
               JOIN canvas_nodes c ON c.note_id = n.id
              WHERE c.frame_id = ?1
                -- A raw note has no worked body, so it never enters a document
                -- even when it sits inside a frame.
                AND n.distilled_at IS NOT NULL
              ORDER BY COALESCE(c.order_in_frame, 0), c.created_at",
        )
        .bind(&frame.id)
        .fetch_all(pool)
        .await?;

        sections.push(DocumentSection { frame, notes });
    }

    Ok(sections)
}

/// Sets or clears a card's colour on this board.
///
/// Per placement, so the same note can sit in a yellow cluster here and be
/// uncoloured there. Passing `None` clears it.
pub async fn set_node_color(pool: &SqlitePool, id: &str, color: Option<&str>) -> Result<()> {
    sqlx::query("UPDATE canvas_nodes SET color = ?2, updated_at = ?3 WHERE id = ?1")
        .bind(id)
        .bind(color)
        .bind(now())
        .execute(pool)
        .await?;

    Ok(())
}
