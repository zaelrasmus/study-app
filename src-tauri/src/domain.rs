//! Stored domain types.
//!
//! This module holds facts only. The knowledge state of a note is derived from
//! those facts in [`crate::note_state`], never stored here.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Long and atomic are the same entity at different stages, not different
/// types. An atomic note is one concept plus its explanation -- which is also
/// the shape of a review card, which is why cards need no authoring.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(rename_all = "lowercase", type_name = "TEXT")]
pub enum NoteKind {
    Long,
    Atomic,
}

/// The stored facts a note's knowledge state is derived from.
///
/// Nothing here is a status. The two axes are computed from these facts in
/// [`crate::note_state`], which is also where the reasoning lives.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Note {
    pub id: String,
    pub kind: NoteKind,
    pub title: String,
    /// One line describing the concept. What the canvas and library render.
    pub summary: String,
    pub body_json: String,
    /// Rendered HTML, so the document view can show real formatting.
    pub body_html: String,
    /// Plaintext projection. Search reads this, and the hash is taken over it,
    /// so restyling a note never costs a recall.
    pub body_text: String,
    pub content_hash: String,

    /// When it first became a written note rather than a bare capture.
    pub distilled_at: Option<DateTime<Utc>>,
    /// Last blind reproduction. Memory mode is the only writer.
    pub last_recall_at: Option<DateTime<Utc>>,
    /// Last Contrast step: the source revealed, and me saying whether it matched.
    pub last_checked_at: Option<DateTime<Utc>>,
    /// When a review last failed. A timestamp rather than a flag, so a later
    /// recall supersedes it instead of erasing it.
    pub last_recall_failed_at: Option<DateTime<Utc>>,
    /// Last ordinary edit. A blind rewrite deliberately does not touch this.
    pub content_edited_at: Option<DateTime<Utc>>,
    /// Content hash at the moment of the last recall.
    pub content_hash_at_recall: Option<String>,
    /// The long note this one was distilled from, when there is one. What the
    /// source pane shows, and what memory mode removes from the DOM.
    pub source_note_id: Option<String>,
    /// The day this note is the page for, when it is one. An ordinary note in
    /// every other respect -- this only records which day you write it on.
    pub journal_day: Option<String>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A note plus both derived axes, which is what the frontend consumes. The
/// frontend never computes state itself.
#[derive(Debug, Clone, Serialize)]
pub struct NoteView {
    #[serde(flatten)]
    pub note: Note,
    #[serde(flatten)]
    pub flags: crate::note_state::NoteFlags,
}

impl NoteView {
    pub fn of(note: Note) -> Self {
        let flags = note.flags();
        Self { note, flags }
    }
}

/// Where a question came from. This *is* its priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case", type_name = "TEXT")]
pub enum QuestionOrigin {
    /// Found while reconstructing a topic from memory. Outranks everything.
    RecallGap,
    /// A card you failed in review.
    ReviewFailure,
    /// Anything you captured without the system observing a gap. The lowest
    /// priority, and honest about it.
    Capture,
}

impl QuestionOrigin {
    /// Lower sorts first. Ordering is by origin, never by date.
    pub fn rank(self) -> i64 {
        match self {
            Self::RecallGap => 0,
            Self::ReviewFailure => 1,
            Self::Capture => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case", type_name = "TEXT")]
pub enum QuestionStatus {
    Open,
    Investigating,
    /// Requires a linked answer note; enforced by a CHECK in the schema.
    Resolved,
    /// One click, no confirmation. If abandoning costs anything, you hoard.
    Abandoned,
    /// The question turned out to rest on a wrong premise. Distinct from
    /// abandoned because realising a question was malformed is a result.
    Dissolved,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Question {
    pub id: String,
    pub text: String,
    pub status: QuestionStatus,
    pub origin: QuestionOrigin,
    pub topic_id: Option<String>,
    pub source_note_id: Option<String>,
    pub answer_note_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Topic {
    pub id: String,
    pub title: String,
    /// Presentation only. Never gates reviewability -- see migration 6.
    pub study: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_visited_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Frame {
    pub id: String,
    pub topic_id: String,
    pub label: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    /// Explicit and shown on the frame. Never inferred from coordinates, so
    /// moving a frame to think better cannot rewrite the document.
    pub order_index: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(rename_all = "lowercase", type_name = "TEXT")]
pub enum CanvasNodeKind {
    Note,
    Ink,
    Image,
    Pdf,
    /// A whole topic, appearing as an object on another topic's canvas. This is
    /// zoom, not a folder tree.
    Topic,
    /// Loose text: a thought written straight onto the board, with no title, no
    /// knowledge state and no card. The cheapest thing you can put down, so
    /// that thinking beside your notes never starts with filing one.
    Text,
    /// A page on the web. Not an asset: nothing was copied, and the page can
    /// change under you — which is exactly why it is marked as a pointer
    /// outwards rather than as something you now have.
    Link,
    /// An open question, placed beside the cards that might answer it. Doubts
    /// arrive while you are writing about something, so they belong next to the
    /// thing they are about and not only in a queue.
    Question,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct CanvasNode {
    pub id: String,
    pub topic_id: String,
    pub kind: CanvasNodeKind,
    pub x: f64,
    pub y: f64,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub z: i64,
    pub note_id: Option<String>,
    pub target_topic_id: Option<String>,
    pub asset_id: Option<String>,
    /// The question a `question` node shows.
    pub question_id: Option<String>,
    pub ink: Option<String>,
    /// The body of a `text` node. Plain, deliberately: loose text that grows
    /// formatting is a note that has not admitted it yet.
    pub text: Option<String>,
    /// Where a `link` node points.
    pub url: Option<String>,
    /// A chosen card colour, per placement. Null means none, which is default.
    pub color: Option<String>,
    /// Membership in a frame is the act of committing a node to the document.
    pub frame_id: Option<String>,
    pub order_in_frame: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A drawn pointer from one canvas node to another.
///
/// Deliberately *not* a link between notes. Note-to-note links are derived from
/// body text and rebuilt on every save, so they can never drift from what you
/// wrote. A pointer is the other kind of claim: that these two cards, arranged
/// this way, on this board, belong together. It lives with the arrangement and
/// dies with it.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct CanvasEdge {
    pub id: String,
    pub topic_id: String,
    pub source_id: String,
    pub target_id: String,
    /// Usually empty. A label on every arrow makes a diagram; what matters here
    /// is the arrangement, not the annotation.
    pub label: String,
    pub created_at: DateTime<Utc>,
}

/// Hash taken over the plaintext projection, so that reformatting alone never
/// invalidates a recall.
pub fn content_hash(body_text: &str) -> String {
    use std::fmt::Write;

    let mut hasher = Sha256::new();
    hasher.update(body_text.trim().as_bytes());
    let digest = hasher.finalize();

    // sha2 0.11 returns a `hybrid_array::Array`, which has no `LowerHex` impl.
    digest.iter().fold(String::with_capacity(64), |mut acc, b| {
        let _ = write!(acc, "{b:02x}");
        acc
    })
}

pub fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}
