//! The IPC surface.
//!
//! Commands are thin: they translate arguments, call one repo function, and
//! wrap notes in [`NoteView`] so the frontend never has to know how status is
//! derived. All the rules live in `domain` and `repo`.

use sqlx::SqlitePool;
use tauri::State;

use crate::domain::{
    CanvasNodeKind, NoteKind, NoteView, Question, QuestionOrigin, QuestionStatus, Topic,
};
use crate::error::{AppError, Result};
use crate::repo::{self, canvas::CanvasSnapshot};

type Db<'a> = State<'a, SqlitePool>;
type Dir<'a> = State<'a, crate::DataDir>;

fn view(note: crate::domain::Note) -> NoteView {
    NoteView::of(note)
}

fn views(notes: Vec<crate::domain::Note>) -> Vec<NoteView> {
    notes.into_iter().map(NoteView::of).collect()
}

// -- topics ---------------------------------------------------------------

/// The sidebar. Time, not structure: no tree, no paths, no folders.
#[tauri::command]
pub async fn awake_topics(db: Db<'_>, limit: Option<i64>) -> Result<Vec<Topic>> {
    repo::topics::awake(&db, limit.unwrap_or(12)).await
}

#[tauri::command]
pub async fn all_topics(db: Db<'_>) -> Result<Vec<Topic>> {
    repo::topics::all(&db).await
}

#[tauri::command]
pub async fn create_topic(db: Db<'_>, id: Option<String>, title: String) -> Result<Topic> {
    repo::topics::create(&db, id, &title).await
}

#[tauri::command]
pub async fn rename_topic(db: Db<'_>, id: String, title: String) -> Result<()> {
    repo::topics::rename(&db, &id, &title).await
}

/// Opening a topic is the only thing that keeps it awake.
#[tauri::command]
pub async fn open_topic(db: Db<'_>, id: String) -> Result<Topic> {
    repo::topics::touch(&db, &id).await?;
    repo::topics::get(&db, &id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("topic {id}")))
}

/// Work board or study board. Presentation only -- it never gates review.
#[tauri::command]
pub async fn set_topic_mode(db: Db<'_>, id: String, study: bool) -> Result<Topic> {
    repo::topics::set_mode(&db, &id, study).await
}

#[tauri::command]
pub async fn delete_topic(db: Db<'_>, id: String) -> Result<()> {
    repo::topics::delete(&db, &id).await
}

// -- notes ----------------------------------------------------------------

/// Capture. Must stay instant; takes a title and nothing else.
#[tauri::command]
pub async fn create_note(
    db: Db<'_>,
    id: Option<String>,
    title: String,
    kind: Option<NoteKind>,
) -> Result<NoteView> {
    let note = repo::notes::create(&db, id, &title, kind.unwrap_or(NoteKind::Long)).await?;
    Ok(view(note))
}

#[tauri::command]
pub async fn get_note(db: Db<'_>, id: String) -> Result<Option<NoteView>> {
    Ok(repo::notes::get(&db, &id).await?.map(view))
}

/// Notes never placed on any canvas.
#[tauri::command]
pub async fn inbox_notes(db: Db<'_>) -> Result<Vec<NoteView>> {
    Ok(views(repo::notes::inbox(&db).await?))
}

#[tauri::command]
pub async fn topic_notes(db: Db<'_>, topic_id: String) -> Result<Vec<NoteView>> {
    Ok(views(repo::notes::for_topic(&db, &topic_id).await?))
}

#[tauri::command]
pub async fn save_note(
    db: Db<'_>,
    id: String,
    title: String,
    summary: String,
    body_json: String,
    body_html: String,
    body_text: String,
) -> Result<NoteView> {
    let note = repo::notes::save_body(
        &db, &id, &title, &summary, &body_json, &body_html, &body_text,
    )
    .await?;
    Ok(view(note))
}

#[tauri::command]
pub async fn set_note_kind(db: Db<'_>, id: String, kind: NoteKind) -> Result<NoteView> {
    Ok(view(repo::notes::set_kind(&db, &id, kind).await?))
}

/// Step 2 of memory mode: save the blind rewrite.
///
/// There is no `closed_book` argument any more. Memory mode is the only route
/// to this command, so a recall is closed-book by construction -- passing the
/// flag alongside would be storing the same fact twice.
#[tauri::command]
pub async fn record_recall(
    db: Db<'_>,
    id: String,
    body_json: String,
    body_html: String,
    body_text: String,
) -> Result<NoteView> {
    Ok(view(
        repo::notes::record_recall(&db, &id, &body_json, &body_html, &body_text).await?,
    ))
}

/// Links the long note being distilled from. What the source pane shows.
#[tauri::command]
pub async fn set_note_source(
    db: Db<'_>,
    id: String,
    source_note_id: Option<String>,
) -> Result<NoteView> {
    Ok(view(
        repo::notes::set_source(&db, &id, source_note_id.as_deref()).await?,
    ))
}

/// Step 4 of memory mode: the source was revealed and compared.
#[tauri::command]
pub async fn record_contrast(db: Db<'_>, id: String) -> Result<NoteView> {
    Ok(view(repo::notes::record_contrast(&db, &id).await?))
}

/// Accent-insensitive search over Spanish content.
#[tauri::command]
pub async fn search_notes(db: Db<'_>, query: String) -> Result<Vec<NoteView>> {
    Ok(views(repo::notes::search(&db, &query).await?))
}

#[tauri::command]
pub async fn delete_note(db: Db<'_>, id: String) -> Result<()> {
    repo::notes::delete(&db, &id).await
}

/// The library: every note there is, filtered. Being on no canvas is normal.
#[tauri::command]
pub async fn library_notes(
    db: Db<'_>,
    filter: repo::notes::LibraryFilter,
) -> Result<Vec<NoteView>> {
    Ok(views(repo::notes::library(&db, &filter).await?))
}

/// Which boards a note appears on. Boards import notes; they do not own them.
#[tauri::command]
pub async fn note_boards(db: Db<'_>, note_id: String) -> Result<Vec<Topic>> {
    repo::notes::boards(&db, &note_id).await
}

/// Distillation by drag: the selected blocks become a note on the canvas.
///
/// The blocks are **copied**, not moved -- the long note keeps them. That means
/// text is duplicated between a source and its extracts, which is deliberate:
/// pulling a concept out must never quietly damage the thing you pulled it from.
///
/// It arrives untitled, so it is a long `raw` note rather than an atomic one.
/// Giving it a title is the act that makes it atomic, because the title is the
/// review front and a card with no front is not a card.
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn distil_to_canvas(
    db: Db<'_>,
    topic_id: String,
    source_note_id: String,
    body_json: String,
    body_html: String,
    body_text: String,
    x: f64,
    y: f64,
) -> Result<(NoteView, crate::domain::CanvasNode)> {
    let note = repo::notes::create(&db, None, "", NoteKind::Long).await?;
    repo::notes::set_source(&db, &note.id, Some(&source_note_id)).await?;

    let note = repo::notes::save_body(&db, &note.id, "", "", &body_json, &body_html, &body_text)
        .await?;
    let node = repo::canvas::add_note_node(&db, None, &topic_id, &note.id, x, y).await?;

    Ok((view(note), node))
}

/// The journal for one day. A filter over captures, never a container.
#[tauri::command]
pub async fn journal_day(db: Db<'_>, day: String) -> Result<Vec<NoteView>> {
    let date = day
        .parse::<chrono::NaiveDate>()
        .map_err(|_| AppError::Invalid(format!("not a date: {day}")))?;

    Ok(views(repo::notes::captured_on(&db, date).await?))
}

/// The page you write on for a day. Absent until you have written something.
#[tauri::command]
pub async fn journal_page(db: Db<'_>, day: String) -> Result<Option<NoteView>> {
    Ok(repo::notes::day_page(&db, &day).await?.map(view))
}

/// Writes a day's page, creating it on the first keystroke.
///
/// Past days are writable: yesterday is a record you may correct. Writing
/// *forward* is what the journal refuses, and that is enforced where days are
/// chosen, not here.
#[tauri::command]
pub async fn save_journal_page(
    db: Db<'_>,
    day: String,
    body_json: String,
    body_html: String,
    body_text: String,
) -> Result<NoteView> {
    day.parse::<chrono::NaiveDate>()
        .map_err(|_| AppError::Invalid(format!("not a date: {day}")))?;

    Ok(view(
        repo::notes::save_day_page(&db, &day, &body_json, &body_html, &body_text).await?,
    ))
}

/// Which days in a range have anything on them, for the week strip.
#[tauri::command]
pub async fn journal_marks(db: Db<'_>, from: String, to: String) -> Result<Vec<String>> {
    let parse = |s: &str| {
        s.parse::<chrono::NaiveDate>()
            .map_err(|_| AppError::Invalid(format!("not a date: {s}")))
    };

    repo::notes::days_with_captures(&db, parse(&from)?, parse(&to)?).await
}

// -- tags, mentions, tasks ------------------------------------------------

#[tauri::command]
pub async fn all_tags(db: Db<'_>) -> Result<Vec<repo::tags::TagCount>> {
    repo::tags::all(&db).await
}

#[tauri::command]
pub async fn note_tags(db: Db<'_>, note_id: String) -> Result<Vec<repo::tags::Tag>> {
    repo::tags::for_note(&db, &note_id).await
}

#[tauri::command]
pub async fn attach_tag(db: Db<'_>, note_id: String, name: String) -> Result<repo::tags::Tag> {
    repo::tags::attach(&db, &note_id, &name).await
}

/// Detaching also drops a tag nothing carries any more.
#[tauri::command]
pub async fn detach_tag(db: Db<'_>, note_id: String, tag_id: String) -> Result<()> {
    repo::tags::detach(&db, &note_id, &tag_id).await
}

/// Drops a tag from every note that carries it, and then the tag itself.
#[tauri::command]
pub async fn delete_tag(db: Db<'_>, id: String) -> Result<()> {
    repo::tags::delete(&db, &id).await
}

/// Notes that mention this one. Derived from the body, never edited directly.
#[tauri::command]
pub async fn note_backlinks(db: Db<'_>, note_id: String) -> Result<Vec<NoteView>> {
    Ok(views(repo::derived::backlinks(&db, &note_id).await?))
}

/// Ticks or unticks a task, in the note that owns it.
///
/// The body stays the truth: this edits the document and rebuilds the index
/// from it, the same as saving from the editor would.
#[tauri::command]
pub async fn set_task_done(db: Db<'_>, task_id: String, done: bool) -> Result<()> {
    repo::derived::set_done(&db, &task_id, done).await
}

/// Every open task, dated or not. What the Tasks screen shows.
#[tauri::command]
pub async fn all_tasks(db: Db<'_>) -> Result<Vec<repo::derived::DueTask>> {
    repo::derived::all_open(&db).await
}

/// Tasks due on one day. What the panel shows.
#[tauri::command]
pub async fn tasks_due(db: Db<'_>, day: String) -> Result<Vec<repo::derived::DueTask>> {
    repo::derived::due_on(&db, &day).await
}

// -- questions ------------------------------------------------------------

#[tauri::command]
pub async fn question_queue(db: Db<'_>, topic_id: Option<String>) -> Result<Vec<Question>> {
    repo::questions::queue(&db, topic_id.as_deref()).await
}

/// Open questions attached to no board.
///
/// A doubt caught while writing in the journal has no topic and is still a
/// doubt. It is reachable as its own scope rather than hidden from every board.
#[tauri::command]
pub async fn unattached_questions(db: Db<'_>) -> Result<Vec<Question>> {
    repo::questions::unattached(&db).await
}

#[tauri::command]
pub async fn create_question(
    db: Db<'_>,
    id: Option<String>,
    text: String,
    origin: QuestionOrigin,
    topic_id: Option<String>,
    source_note_id: Option<String>,
) -> Result<Question> {
    repo::questions::create(
        &db,
        id,
        &text,
        origin,
        topic_id.as_deref(),
        source_note_id.as_deref(),
    )
    .await
}

#[tauri::command]
pub async fn resolve_question(
    db: Db<'_>,
    id: String,
    answer_note_id: String,
) -> Result<Question> {
    repo::questions::resolve(&db, &id, &answer_note_id).await
}

/// Abandoning is one click with no confirmation, by design.
#[tauri::command]
pub async fn set_question_status(
    db: Db<'_>,
    id: String,
    status: QuestionStatus,
) -> Result<Question> {
    repo::questions::set_status(&db, &id, status).await
}

#[tauri::command]
pub async fn set_question_topic(
    db: Db<'_>,
    id: String,
    topic_id: Option<String>,
) -> Result<Question> {
    repo::questions::set_topic(&db, &id, topic_id.as_deref()).await
}

#[tauri::command]
pub async fn open_question_count(db: Db<'_>, topic_id: Option<String>) -> Result<i64> {
    repo::questions::open_count(&db, topic_id.as_deref()).await
}

// -- review ---------------------------------------------------------------

/// Today's queue, capped. Never the backlog.
#[tauri::command]
pub async fn review_queue(db: Db<'_>) -> Result<Vec<repo::review::ReviewCard>> {
    repo::review::queue(&db).await
}

/// Bounded work, so it is safe to show in the sidebar.
#[tauri::command]
pub async fn review_due_today(db: Db<'_>) -> Result<i64> {
    repo::review::due_today(&db).await
}

#[tauri::command]
pub async fn grade_card(db: Db<'_>, note_id: String, grade: crate::review::Grade) -> Result<()> {
    repo::review::grade(&db, &note_id, grade).await
}

/// A drifted card leaves the queue by being acted on, not by being graded.
#[tauri::command]
pub async fn resolve_drifted_card(
    db: Db<'_>,
    note_id: String,
    still_holds: bool,
) -> Result<()> {
    repo::review::resolve_drifted(&db, &note_id, still_holds).await
}

// -- canvas ---------------------------------------------------------------

#[tauri::command]
pub async fn canvas_snapshot(db: Db<'_>, topic_id: String) -> Result<CanvasSnapshot> {
    repo::canvas::snapshot(&db, &topic_id).await
}

#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn create_frame(
    db: Db<'_>,
    id: Option<String>,
    topic_id: String,
    label: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<crate::domain::Frame> {
    repo::canvas::create_frame(&db, id, &topic_id, &label, x, y, width, height).await
}

#[tauri::command]
pub async fn move_frame(
    db: Db<'_>,
    id: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<()> {
    repo::canvas::move_frame(&db, &id, x, y, width, height).await
}

#[tauri::command]
pub async fn label_frame(db: Db<'_>, id: String, label: String) -> Result<()> {
    repo::canvas::label_frame(&db, &id, &label).await
}

/// The only operation that changes document order.
#[tauri::command]
pub async fn reorder_frames(db: Db<'_>, topic_id: String, ids: Vec<String>) -> Result<()> {
    repo::canvas::reorder_frames(&db, &topic_id, &ids).await
}

#[tauri::command]
pub async fn delete_frame(db: Db<'_>, id: String) -> Result<()> {
    repo::canvas::delete_frame(&db, &id).await
}

#[tauri::command]
pub async fn add_note_node(
    db: Db<'_>,
    id: Option<String>,
    topic_id: String,
    note_id: String,
    x: f64,
    y: f64,
) -> Result<crate::domain::CanvasNode> {
    repo::canvas::add_note_node(&db, id, &topic_id, &note_id, x, y).await
}

/// Capture straight onto the canvas: makes the note and places it in one call
/// so the gesture stays a single action.
#[tauri::command]
pub async fn create_note_on_canvas(
    db: Db<'_>,
    topic_id: String,
    title: String,
    x: f64,
    y: f64,
) -> Result<(NoteView, crate::domain::CanvasNode)> {
    let note = repo::notes::create(&db, None, &title, NoteKind::Long).await?;
    let node = repo::canvas::add_note_node(&db, None, &topic_id, &note.id, x, y).await?;
    Ok((view(note), node))
}

#[tauri::command]
pub async fn add_topic_node(
    db: Db<'_>,
    id: Option<String>,
    topic_id: String,
    target_topic_id: String,
    x: f64,
    y: f64,
) -> Result<crate::domain::CanvasNode> {
    repo::canvas::add_topic_node(&db, id, &topic_id, &target_topic_id, x, y).await
}

#[tauri::command]
pub async fn add_ink_node(
    db: Db<'_>,
    id: Option<String>,
    topic_id: String,
    ink: String,
    x: f64,
    y: f64,
) -> Result<crate::domain::CanvasNode> {
    repo::canvas::add_ink_node(&db, id, &topic_id, &ink, x, y).await
}

// -- files and links ------------------------------------------------------

/// Brings a file onto a board: an image or a PDF, copied in and placed.
///
/// The file is *copied* rather than referenced where it sits, so a board does
/// not break because you tidied a folder. An asset is not a note — no title, no
/// knowledge state, no card, and it never enters review. It is what you think
/// about, not what you have learned.
#[tauri::command]
pub async fn import_asset(
    db: Db<'_>,
    dir: Dir<'_>,
    topic_id: String,
    path: String,
    x: f64,
    y: f64,
) -> Result<(repo::assets::Asset, crate::domain::CanvasNode)> {
    let source = std::path::PathBuf::from(&path);
    let asset = repo::assets::import(&db, &dir.0, &source).await?;

    let kind = match asset.kind {
        repo::assets::AssetKind::Image => CanvasNodeKind::Image,
        repo::assets::AssetKind::Pdf => CanvasNodeKind::Pdf,
    };

    let node = repo::canvas::add_asset_node(&db, None, &topic_id, &asset.id, kind, x, y).await?;

    Ok((asset, node))
}

/// Where the copies live. The frontend turns this into a URL it can render.
#[tauri::command]
pub fn asset_dir(dir: Dir<'_>) -> String {
    repo::assets::dir(&dir.0).to_string_lossy().into_owned()
}

/// Puts an open question on a board, and attaches it to that board.
///
/// Placement is membership: a doubt caught while writing has no board, and
/// dropping it on a canvas is what gives it one.
#[tauri::command]
pub async fn add_question_node(
    db: Db<'_>,
    id: Option<String>,
    topic_id: String,
    question_id: String,
    x: f64,
    y: f64,
) -> Result<crate::domain::CanvasNode> {
    repo::canvas::add_question_node(&db, id, &topic_id, &question_id, x, y).await
}

/// A page on the web, placed on the board. Nothing is copied.
#[tauri::command]
pub async fn add_link_node(
    db: Db<'_>,
    id: Option<String>,
    topic_id: String,
    url: String,
    x: f64,
    y: f64,
) -> Result<crate::domain::CanvasNode> {
    repo::canvas::add_link_node(&db, id, &topic_id, &url, x, y).await
}

/// Loose text on a board: no title, no state, no card, no note behind it.
#[tauri::command]
pub async fn add_text_node(
    db: Db<'_>,
    id: Option<String>,
    topic_id: String,
    text: String,
    x: f64,
    y: f64,
) -> Result<crate::domain::CanvasNode> {
    repo::canvas::add_text_node(&db, id, &topic_id, &text, x, y).await
}

#[tauri::command]
pub async fn set_node_text(db: Db<'_>, id: String, text: String) -> Result<()> {
    repo::canvas::set_node_text(&db, &id, &text).await
}

/// Turns loose text into a note, in place: same node, same spot, same board.
#[tauri::command]
pub async fn promote_text_node(db: Db<'_>, id: String) -> Result<(NoteView, crate::domain::CanvasNode)> {
    let (note, node) = repo::canvas::promote_text_node(&db, &id).await?;
    Ok((view(note), node))
}

// -- pointers -------------------------------------------------------------

/// Draws an arrow between two cards. Idempotent on the ordered pair.
#[tauri::command]
pub async fn connect_nodes(
    db: Db<'_>,
    id: Option<String>,
    source_id: String,
    target_id: String,
) -> Result<crate::domain::CanvasEdge> {
    repo::canvas::connect(&db, id, &source_id, &target_id).await
}

#[tauri::command]
pub async fn label_edge(db: Db<'_>, id: String, label: String) -> Result<()> {
    repo::canvas::label_edge(&db, &id, &label).await
}

#[tauri::command]
pub async fn disconnect_nodes(db: Db<'_>, id: String) -> Result<()> {
    repo::canvas::disconnect(&db, &id).await
}

#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn add_asset_node(
    db: Db<'_>,
    id: Option<String>,
    topic_id: String,
    asset_id: String,
    kind: CanvasNodeKind,
    x: f64,
    y: f64,
) -> Result<crate::domain::CanvasNode> {
    repo::canvas::add_asset_node(&db, id, &topic_id, &asset_id, kind, x, y).await
}

#[tauri::command]
pub async fn move_node(
    db: Db<'_>,
    id: String,
    x: f64,
    y: f64,
    width: Option<f64>,
    height: Option<f64>,
) -> Result<()> {
    repo::canvas::move_node(&db, &id, x, y, width, height).await
}

/// Committing a node to the document, or taking it back out.
#[tauri::command]
pub async fn set_node_frame(
    db: Db<'_>,
    id: String,
    frame_id: Option<String>,
    order_in_frame: Option<i64>,
) -> Result<()> {
    repo::canvas::set_node_frame(&db, &id, frame_id.as_deref(), order_in_frame).await
}

/// Sets or clears a card's colour on this board. Per placement, not per note.
#[tauri::command]
pub async fn set_node_color(db: Db<'_>, id: String, color: Option<String>) -> Result<()> {
    repo::canvas::set_node_color(&db, &id, color.as_deref()).await
}

#[tauri::command]
pub async fn delete_node(db: Db<'_>, dir: Dir<'_>, id: String) -> Result<()> {
    repo::canvas::delete_node(&db, &id).await?;

    // A file nothing points at any more is dead weight, and leaving it behind
    // grows the data directory forever with things nothing can show you.
    repo::assets::sweep(&db, &dir.0).await?;
    Ok(())
}

/// The document, assembled from the arrangement rather than written by hand.
#[tauri::command]
pub async fn topic_document(
    db: Db<'_>,
    topic_id: String,
) -> Result<Vec<repo::canvas::DocumentSection>> {
    repo::canvas::document(&db, &topic_id).await
}
