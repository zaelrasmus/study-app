//! Integration tests against a real SQLite file.
//!
//! These exist to check the rules that live in SQL rather than in Rust:
//! placement-as-membership, question ordering, cycle detection, and the
//! guarantee that rearranging a canvas cannot rewrite the document.

use sqlx::SqlitePool;
use study_app_lib::domain::{NoteKind, QuestionOrigin, QuestionStatus};
use study_app_lib::{db, repo};

/// Each test gets its own database file, torn down at the end.
struct Sandbox {
    dir: std::path::PathBuf,
    pool: SqlitePool,
}

impl Sandbox {
    async fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "study-app-it-{name}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);

        let pool = db::connect(&dir).await.expect("connect + migrate");
        Self { dir, pool }
    }

    async fn close(self) {
        self.pool.close().await;
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// A note with a body, which is what distils it out of `raw`.
async fn written(db: &SqlitePool, title: &str) -> study_app_lib::domain::Note {
    let n = repo::notes::create(db, None, title, NoteKind::Atomic)
        .await
        .unwrap();
    repo::notes::save_body(db, &n.id, title, "", "{}", "<p>cuerpo</p>", "cuerpo")
        .await
        .unwrap()
}

#[tokio::test]
async fn placement_is_membership() {
    let sb = Sandbox::new("membership").await;
    let db = &sb.pool;

    let topic = repo::topics::create(db, None, "Spaced repetition")
        .await
        .unwrap();
    let note = repo::notes::create(db, None, "Forgetting curve", NoteKind::Long)
        .await
        .unwrap();

    // A note with no placement is in the inbox and belongs to no topic.
    let inbox = repo::notes::inbox(db).await.unwrap();
    assert_eq!(inbox.len(), 1);
    assert!(repo::notes::for_topic(db, &topic.id).await.unwrap().is_empty());

    let node = repo::canvas::add_note_node(db, None, &topic.id, &note.id, 40.0, 60.0)
        .await
        .unwrap();

    // Placing it is what makes it a member; nothing else was written.
    assert!(repo::notes::inbox(db).await.unwrap().is_empty());
    assert_eq!(repo::notes::for_topic(db, &topic.id).await.unwrap().len(), 1);

    // Unplacing returns it to the inbox rather than destroying it.
    repo::canvas::delete_node(db, &node.id).await.unwrap();
    assert_eq!(repo::notes::inbox(db).await.unwrap().len(), 1);
    assert!(repo::notes::get(db, &note.id).await.unwrap().is_some());

    sb.close().await;
}

#[tokio::test]
async fn a_note_can_live_on_more_than_one_topic() {
    let sb = Sandbox::new("multi-topic").await;
    let db = &sb.pool;

    let a = repo::topics::create(db, None, "Memory").await.unwrap();
    let b = repo::topics::create(db, None, "Study design").await.unwrap();
    let note = repo::notes::create(db, None, "Testing effect", NoteKind::Atomic)
        .await
        .unwrap();

    repo::canvas::add_note_node(db, None, &a.id, &note.id, 0.0, 0.0)
        .await
        .unwrap();
    repo::canvas::add_note_node(db, None, &b.id, &note.id, 0.0, 0.0)
        .await
        .unwrap();

    assert_eq!(repo::notes::for_topic(db, &a.id).await.unwrap().len(), 1);
    assert_eq!(repo::notes::for_topic(db, &b.id).await.unwrap().len(), 1);

    sb.close().await;
}

#[tokio::test]
async fn question_queue_orders_by_origin_not_by_date() {
    let sb = Sandbox::new("queue").await;
    let db = &sb.pool;

    let topic = repo::topics::create(db, None, "Optics").await.unwrap();

    // Inserted worst-priority first, so date ordering would invert the result.
    for (text, origin) in [
        ("jotted while reading", QuestionOrigin::Capture),
        ("failed in review", QuestionOrigin::ReviewFailure),
        ("gap found from memory", QuestionOrigin::RecallGap),
    ] {
        repo::questions::create(db, None, text, origin, Some(&topic.id), None)
            .await
            .unwrap();
    }

    let queue = repo::questions::queue(db, None).await.unwrap();
    let origins: Vec<_> = queue.iter().map(|q| q.origin).collect();

    assert_eq!(
        origins,
        vec![
            QuestionOrigin::RecallGap,
            QuestionOrigin::ReviewFailure,
            QuestionOrigin::Capture,
        ]
    );
    assert_eq!(repo::questions::open_count(db, None).await.unwrap(), 3);

    sb.close().await;
}

#[tokio::test]
async fn closing_a_question_requires_an_answer_note() {
    let sb = Sandbox::new("resolve").await;
    let db = &sb.pool;

    let q = repo::questions::create(db, None, "why?", QuestionOrigin::Capture, None, None)
        .await
        .unwrap();

    // No such note: refused.
    assert!(repo::questions::resolve(db, &q.id, "nope").await.is_err());

    // Resolved is not reachable through the plain status setter either.
    assert!(repo::questions::set_status(db, &q.id, QuestionStatus::Resolved)
        .await
        .is_err());

    let answer = repo::notes::create(db, None, "because", NoteKind::Atomic)
        .await
        .unwrap();
    let resolved = repo::questions::resolve(db, &q.id, &answer.id).await.unwrap();

    assert_eq!(resolved.status, QuestionStatus::Resolved);
    assert_eq!(resolved.answer_note_id.as_deref(), Some(answer.id.as_str()));
    assert_eq!(repo::questions::open_count(db, None).await.unwrap(), 0);

    sb.close().await;
}

#[tokio::test]
async fn abandoning_is_unconditional() {
    let sb = Sandbox::new("abandon").await;
    let db = &sb.pool;

    let q = repo::questions::create(db, None, "hm", QuestionOrigin::Capture, None, None)
        .await
        .unwrap();

    let out = repo::questions::set_status(db, &q.id, QuestionStatus::Abandoned)
        .await
        .unwrap();
    assert_eq!(out.status, QuestionStatus::Abandoned);
    assert_eq!(repo::questions::open_count(db, None).await.unwrap(), 0);

    sb.close().await;
}

#[tokio::test]
async fn topics_nest_but_cannot_form_a_cycle() {
    let sb = Sandbox::new("nesting").await;
    let db = &sb.pool;

    let a = repo::topics::create(db, None, "A").await.unwrap();
    let b = repo::topics::create(db, None, "B").await.unwrap();
    let c = repo::topics::create(db, None, "C").await.unwrap();

    // A contains B, B contains C.
    repo::canvas::add_topic_node(db, None, &a.id, &b.id, 0.0, 0.0)
        .await
        .unwrap();
    repo::canvas::add_topic_node(db, None, &b.id, &c.id, 0.0, 0.0)
        .await
        .unwrap();

    // Self-containment is refused.
    assert!(repo::canvas::add_topic_node(db, None, &a.id, &a.id, 0.0, 0.0)
        .await
        .is_err());

    // So is closing the loop, transitively: C cannot contain A.
    assert!(repo::canvas::add_topic_node(db, None, &c.id, &a.id, 0.0, 0.0)
        .await
        .is_err());

    // A sibling placement is still fine.
    assert!(repo::canvas::add_topic_node(db, None, &a.id, &c.id, 0.0, 0.0)
        .await
        .is_ok());

    sb.close().await;
}

#[tokio::test]
async fn the_document_comes_from_frames_and_ignores_coordinates() {
    let sb = Sandbox::new("document").await;
    let db = &sb.pool;

    let topic = repo::topics::create(db, None, "Waves").await.unwrap();

    let intro = repo::canvas::create_frame(db, None, &topic.id, "Intro", 0.0, 0.0, 600.0, 400.0)
        .await
        .unwrap();
    let detail =
        repo::canvas::create_frame(db, None, &topic.id, "Detail", 800.0, 0.0, 600.0, 400.0)
            .await
            .unwrap();

    assert_eq!(intro.order_index, 0);
    assert_eq!(detail.order_index, 1);

    let first = written(db, "What a wave is").await;
    let second = written(db, "Superposition").await;
    let scratch = written(db, "half-formed thought").await;

    // A note with no body is raw, and raw notes never enter a document even
    // when they sit inside a frame.
    let bare = repo::notes::create(db, None, "just captured", NoteKind::Long)
        .await
        .unwrap();

    let n1 = repo::canvas::add_note_node(db, None, &topic.id, &first.id, 10.0, 10.0)
        .await
        .unwrap();
    let n2 = repo::canvas::add_note_node(db, None, &topic.id, &second.id, 810.0, 10.0)
        .await
        .unwrap();
    // Deliberately left outside every frame.
    repo::canvas::add_note_node(db, None, &topic.id, &scratch.id, 400.0, 900.0)
        .await
        .unwrap();

    let bare_node = repo::canvas::add_note_node(db, None, &topic.id, &bare.id, 20.0, 20.0)
        .await
        .unwrap();
    repo::canvas::set_node_frame(db, &bare_node.id, Some(&intro.id), Some(1))
        .await
        .unwrap();

    repo::canvas::set_node_frame(db, &n1.id, Some(&intro.id), Some(0))
        .await
        .unwrap();
    repo::canvas::set_node_frame(db, &n2.id, Some(&detail.id), Some(0))
        .await
        .unwrap();

    let doc = repo::canvas::document(db, &topic.id).await.unwrap();
    assert_eq!(doc.len(), 2);
    assert_eq!(doc[0].frame.label, "Intro");
    assert_eq!(doc[0].notes[0].title, "What a wave is");
    assert_eq!(doc[1].notes[0].title, "Superposition");

    // The unframed note is absent: the canvas is a superset of the document.
    let titles: Vec<_> = doc
        .iter()
        .flat_map(|s| s.notes.iter().map(|n| n.title.as_str()))
        .collect();
    assert!(!titles.contains(&"half-formed thought"));
    // Framed, but raw: frame membership is necessary, not sufficient.
    assert!(!titles.contains(&"just captured"));

    // Dragging the frames past each other must NOT change the document.
    repo::canvas::move_frame(db, &intro.id, 5000.0, 5000.0, 600.0, 400.0)
        .await
        .unwrap();

    let after_move = repo::canvas::document(db, &topic.id).await.unwrap();
    assert_eq!(after_move[0].frame.label, "Intro");
    assert_eq!(after_move[1].frame.label, "Detail");

    // Only an explicit reorder does.
    repo::canvas::reorder_frames(db, &topic.id, &[detail.id.clone(), intro.id.clone()])
        .await
        .unwrap();

    let reordered = repo::canvas::document(db, &topic.id).await.unwrap();
    assert_eq!(reordered[0].frame.label, "Detail");
    assert_eq!(reordered[1].frame.label, "Intro");

    sb.close().await;
}

#[tokio::test]
async fn deleting_a_frame_releases_its_nodes() {
    let sb = Sandbox::new("frame-delete").await;
    let db = &sb.pool;

    let topic = repo::topics::create(db, None, "Thermo").await.unwrap();
    let frame = repo::canvas::create_frame(db, None, &topic.id, "F", 0.0, 0.0, 600.0, 400.0)
        .await
        .unwrap();
    let note = repo::notes::create(db, None, "Entropy", NoteKind::Atomic)
        .await
        .unwrap();
    let node = repo::canvas::add_note_node(db, None, &topic.id, &note.id, 0.0, 0.0)
        .await
        .unwrap();
    repo::canvas::set_node_frame(db, &node.id, Some(&frame.id), Some(0))
        .await
        .unwrap();

    repo::canvas::delete_frame(db, &frame.id).await.unwrap();

    let snap = repo::canvas::snapshot(db, &topic.id).await.unwrap();
    assert!(snap.frames.is_empty());
    // The node survives, merely un-committed from the document.
    assert_eq!(snap.nodes.len(), 1);
    assert!(snap.nodes[0].frame_id.is_none());
    assert!(repo::canvas::document(db, &topic.id).await.unwrap().is_empty());

    sb.close().await;
}

#[tokio::test]
async fn the_two_axes_are_written_by_two_separate_steps() {
    let sb = Sandbox::new("recall").await;
    let db = &sb.pool;

    use study_app_lib::note_state::RecallState;

    let note = repo::notes::create(db, None, "Ley de Ohm", NoteKind::Atomic)
        .await
        .unwrap();
    assert_eq!(note.recall_state(), RecallState::Raw);

    // Writing it gives it a body, which is what distils it.
    let note = repo::notes::save_body(
        db,
        &note.id,
        "Ley de Ohm",
        "",
        "{}",
        "<p>V es I por R</p>",
        "V es I por R",
    )
    .await
    .unwrap();
    assert_eq!(note.recall_state(), RecallState::Draft);
    assert!(note.content_edited_at.is_some());

    // A blind rewrite immediately after an edit is inside the cooling window:
    // the recall is recorded, but it does not get to claim the note is known.
    let cooling = repo::notes::record_recall(db, &note.id, "{}", "<p>V es I por R</p>", "V es I por R")
        .await
        .unwrap();
    assert!(cooling.last_recall_at.is_some(), "the recall is still recorded");
    assert_eq!(
        cooling.recall_state(),
        RecallState::EditedSinceRecall,
        "reciting text written moments ago must not count"
    );

    // Axis 2 is untouched by any of that: nothing has been contrasted yet.
    assert!(cooling.never_contrasted());

    let checked = repo::notes::record_contrast(db, &note.id).await.unwrap();
    assert!(!checked.never_contrasted());
    // Contrasting does not change axis 1.
    assert_eq!(checked.recall_state(), RecallState::EditedSinceRecall);

    sb.close().await;
}

#[tokio::test]
async fn a_review_failure_keeps_the_recall_it_supersedes() {
    let sb = Sandbox::new("failing").await;
    let db = &sb.pool;

    use study_app_lib::note_state::RecallState;

    let note = repo::notes::create(db, None, "Entropía", NoteKind::Atomic)
        .await
        .unwrap();
    let note = repo::notes::record_recall(db, &note.id, "{}", "<p>x</p>", "x")
        .await
        .unwrap();
    assert_eq!(note.recall_state(), RecallState::Recalled);

    let failed = repo::notes::record_review_failure(db, &note.id).await.unwrap();
    assert_eq!(failed.recall_state(), RecallState::Failing);
    // The recall is not erased: "failing" and "never recalled" stay distinct.
    assert!(failed.last_recall_at.is_some());
    assert!(failed.content_hash_at_recall.is_some());

    sb.close().await;
}

#[tokio::test]
async fn search_folds_spanish_accents_but_not_the_enye() {
    let sb = Sandbox::new("search").await;
    let db = &sb.pool;

    repo::notes::create(db, None, "Localización de memoria", NoteKind::Long)
        .await
        .unwrap();
    repo::notes::create(db, None, "El año pasado", NoteKind::Long)
        .await
        .unwrap();

    assert_eq!(repo::notes::search(db, "localizacion").await.unwrap().len(), 1);
    assert_eq!(repo::notes::search(db, "LOCALIZACIÓN").await.unwrap().len(), 1);

    // año and ano are different words and must not collapse.
    assert_eq!(repo::notes::search(db, "año").await.unwrap().len(), 1);
    assert_eq!(repo::notes::search(db, "ano").await.unwrap().len(), 0);

    sb.close().await;
}

#[tokio::test]
async fn the_sidebar_is_ordered_by_visit_not_by_name() {
    let sb = Sandbox::new("sidebar").await;
    let db = &sb.pool;

    let a = repo::topics::create(db, None, "Aardvark").await.unwrap();
    let z = repo::topics::create(db, None, "Zebra").await.unwrap();

    repo::topics::touch(db, &a.id).await.unwrap();
    repo::topics::touch(db, &z.id).await.unwrap();

    let awake = repo::topics::awake(db, 12).await.unwrap();
    assert_eq!(awake[0].id, z.id, "most recently opened comes first");

    repo::topics::touch(db, &a.id).await.unwrap();
    let awake = repo::topics::awake(db, 12).await.unwrap();
    assert_eq!(awake[0].id, a.id);

    // The limit is what stops the list growing without bound.
    assert_eq!(repo::topics::awake(db, 1).await.unwrap().len(), 1);

    sb.close().await;
}

// -- review ---------------------------------------------------------------

/// A note reproduced from memory, which is the only thing that makes a card.
async fn recalled(db: &SqlitePool, title: &str) -> study_app_lib::domain::Note {
    let n = repo::notes::create(db, None, title, NoteKind::Atomic)
        .await
        .unwrap();
    repo::notes::record_recall(db, &n.id, "{}", "<p>cuerpo</p>", "cuerpo")
        .await
        .unwrap()
}

#[tokio::test]
async fn a_card_exists_only_once_a_note_has_been_reproduced() {
    let sb = Sandbox::new("cards").await;
    let db = &sb.pool;

    // Written with the source visible: a draft, and no card.
    written(db, "Sólo un borrador").await;
    assert!(repo::review::queue(db).await.unwrap().is_empty());

    // Reproduced from memory: now it is a card. Nothing was authored.
    let note = recalled(db, "Ley de Ohm").await;
    let queue = repo::review::queue(db).await.unwrap();

    assert_eq!(queue.len(), 1);
    assert_eq!(queue[0].note.id, note.id);
    // Front is the title, back is the body -- both come straight off the note.
    assert_eq!(queue[0].note.title, "Ley de Ohm");
    assert_eq!(queue[0].note.body_text, "cuerpo");

    sb.close().await;
}

#[tokio::test]
async fn missing_a_card_files_a_question_and_marks_the_note_failing() {
    use study_app_lib::note_state::RecallState;
    use study_app_lib::review::Grade;

    let sb = Sandbox::new("grade-again").await;
    let db = &sb.pool;

    let note = recalled(db, "Entropía").await;
    repo::review::grade(db, &note.id, Grade::Again).await.unwrap();

    let after = repo::notes::get(db, &note.id).await.unwrap().unwrap();
    assert_eq!(after.recall_state(), RecallState::Failing);
    // The recall is not erased: "failing" and "never recalled" stay different.
    assert!(after.last_recall_at.is_some());
    assert!(after.content_hash_at_recall.is_some());

    let queue = repo::questions::queue(db, None).await.unwrap();
    assert_eq!(queue.len(), 1);
    assert_eq!(queue[0].origin, QuestionOrigin::ReviewFailure);

    sb.close().await;
}

#[tokio::test]
async fn a_graded_card_leaves_todays_queue() {
    use study_app_lib::review::Grade;

    let sb = Sandbox::new("grade-good").await;
    let db = &sb.pool;

    let note = recalled(db, "Difracción").await;
    assert_eq!(repo::review::due_today(db).await.unwrap(), 1);

    repo::review::grade(db, &note.id, Grade::Good).await.unwrap();

    // Scheduled a day out, so it is no longer today's work.
    assert_eq!(repo::review::due_today(db).await.unwrap(), 0);

    sb.close().await;
}

#[tokio::test]
async fn a_drifted_card_is_flagged_rather_than_suspended() {
    use study_app_lib::note_state::RecallState;

    let sb = Sandbox::new("drift").await;
    let db = &sb.pool;

    let note = recalled(db, "Refracción").await;

    // The text moves after the recall.
    repo::notes::save_body(db, &note.id, "Refracción", "", "{}", "<p>otro</p>", "otro")
        .await
        .unwrap();

    let after = repo::notes::get(db, &note.id).await.unwrap().unwrap();
    assert_eq!(after.recall_state(), RecallState::EditedSinceRecall);

    // Crucially it is still in the queue. Suspending it silently would switch
    // review off with no explanation.
    let queue = repo::review::queue(db).await.unwrap();
    assert_eq!(queue.len(), 1);
    assert_eq!(queue[0].flags.state, RecallState::EditedSinceRecall);

    sb.close().await;
}

#[tokio::test]
async fn a_drifted_card_leaves_by_being_acted_on() {
    use study_app_lib::note_state::RecallState;

    let sb = Sandbox::new("drift-resolve").await;
    let db = &sb.pool;

    let note = recalled(db, "Interferencia").await;
    repo::notes::save_body(db, &note.id, "Interferencia", "", "{}", "<p>otro</p>", "otro")
        .await
        .unwrap();

    // "It still holds" contrasts it and resumes normal scheduling.
    repo::review::resolve_drifted(db, &note.id, true).await.unwrap();

    let after = repo::notes::get(db, &note.id).await.unwrap().unwrap();
    assert!(after.last_checked_at.is_some(), "asserting it holds is a contrast");
    assert!(!after.never_contrasted());
    assert_eq!(repo::review::due_today(db).await.unwrap(), 0, "it left the queue");
    // Not marked as a miss: you demonstrated nothing about your memory.
    assert_ne!(after.recall_state(), RecallState::Failing);

    sb.close().await;
}

#[tokio::test]
async fn needing_work_on_a_drifted_card_files_a_question_without_failing_it() {
    use study_app_lib::note_state::RecallState;

    let sb = Sandbox::new("drift-work").await;
    let db = &sb.pool;

    let note = recalled(db, "Polarización").await;
    repo::notes::save_body(db, &note.id, "Polarización", "", "{}", "<p>otro</p>", "otro")
        .await
        .unwrap();

    repo::review::resolve_drifted(db, &note.id, false).await.unwrap();

    let questions = repo::questions::queue(db, None).await.unwrap();
    assert_eq!(questions.len(), 1);
    assert_eq!(questions[0].origin, QuestionOrigin::RecallGap);

    let after = repo::notes::get(db, &note.id).await.unwrap().unwrap();
    assert_ne!(after.recall_state(), RecallState::Failing, "drift is not a miss");
    assert_eq!(repo::review::due_today(db).await.unwrap(), 0, "deferred");

    sb.close().await;
}

#[tokio::test]
async fn the_queue_never_grows_past_a_days_worth() {
    let sb = Sandbox::new("cap").await;
    let db = &sb.pool;

    // Far more due than a day should ever show.
    for i in 0..(study_app_lib::review::DAILY_CAP + 12) {
        recalled(db, &format!("Concepto {i}")).await;
    }

    let queue = repo::review::queue(db).await.unwrap();
    assert_eq!(queue.len() as i64, study_app_lib::review::DAILY_CAP);

    // And the sidebar number is the visible queue, never the backlog behind it.
    assert_eq!(
        repo::review::due_today(db).await.unwrap(),
        study_app_lib::review::DAILY_CAP
    );

    sb.close().await;
}

// -- library --------------------------------------------------------------

#[tokio::test]
async fn the_library_holds_everything_and_unplaced_is_just_a_filter() {
    use study_app_lib::repo::notes::LibraryFilter;

    let sb = Sandbox::new("library").await;
    let db = &sb.pool;

    let topic = repo::topics::create(db, None, "Óptica").await.unwrap();
    let placed = written(db, "Difracción").await;
    let loose = written(db, "Idea suelta").await;
    repo::canvas::add_note_node(db, None, &topic.id, &placed.id, 0.0, 0.0)
        .await
        .unwrap();

    // Everything, by default. Being on no board is not a condition to filter out.
    let all = repo::notes::library(db, &LibraryFilter::default()).await.unwrap();
    assert_eq!(all.len(), 2);

    // The inbox is one filter among several, not a separate place.
    let inbox = repo::notes::library(
        db,
        &LibraryFilter { unplaced_only: Some(true), ..Default::default() },
    )
    .await
    .unwrap();
    assert_eq!(inbox.len(), 1);
    assert_eq!(inbox[0].id, loose.id);

    // Scoped to a board.
    let board = repo::notes::library(
        db,
        &LibraryFilter { topic_id: Some(topic.id.clone()), ..Default::default() },
    )
    .await
    .unwrap();
    assert_eq!(board.len(), 1);
    assert_eq!(board[0].id, placed.id);

    sb.close().await;
}

#[tokio::test]
async fn a_tag_filters_the_library_and_narrows_the_scope_it_is_combined_with() {
    use study_app_lib::repo::notes::LibraryFilter;

    let sb = Sandbox::new("library-tag").await;
    let db = &sb.pool;

    let topic = repo::topics::create(db, None, "Óptica").await.unwrap();
    let placed = written(db, "Difracción").await;
    let loose = written(db, "Idea suelta").await;
    repo::canvas::add_note_node(db, None, &topic.id, &placed.id, 0.0, 0.0)
        .await
        .unwrap();

    // Both carry the tag; only one is on the board.
    let tag = repo::tags::attach(db, &placed.id, "ondas").await.unwrap();
    repo::tags::attach(db, &loose.id, "ondas").await.unwrap();
    let other = repo::tags::attach(db, &loose.id, "pendiente").await.unwrap();

    let by_tag = |tag_id: String, topic_id: Option<String>| async move {
        repo::notes::library(
            db,
            &LibraryFilter { tag_id: Some(tag_id), topic_id, ..Default::default() },
        )
        .await
        .unwrap()
    };

    let both = by_tag(tag.id.clone(), None).await;
    assert_eq!(both.len(), 2, "a tag reaches notes on no board at all");

    // The point of the filter: the tag is not written anywhere in the text, so
    // pre-filling the search box would have found nothing.
    assert!(both.iter().all(|n| !n.body_text.contains("ondas")));

    let narrowed = by_tag(tag.id.clone(), Some(topic.id.clone())).await;
    assert_eq!(narrowed.len(), 1, "a tag narrows a scope rather than replacing it");
    assert_eq!(narrowed[0].id, placed.id);

    let single = by_tag(other.id.clone(), None).await;
    assert_eq!(single.len(), 1);
    assert_eq!(single[0].id, loose.id);

    // Detaching the last carrier drops the tag, so the filter empties with it.
    repo::tags::detach(db, &loose.id, &other.id).await.unwrap();
    assert!(by_tag(other.id, None).await.is_empty());

    sb.close().await;
}

#[tokio::test]
async fn the_library_search_folds_spanish_and_reads_the_summary() {
    use study_app_lib::repo::notes::LibraryFilter;

    let sb = Sandbox::new("library-search").await;
    let db = &sb.pool;

    let n = repo::notes::create(db, None, "Difracción", NoteKind::Atomic)
        .await
        .unwrap();
    repo::notes::save_body(
        db,
        &n.id,
        "Difracción",
        "Cómo se curva una onda al pasar por una rendija",
        "{}",
        "<p>cuerpo</p>",
        "cuerpo",
    )
    .await
    .unwrap();

    let find = |q: &str| {
        let q = q.to_owned();
        async move {
            repo::notes::library(
                db,
                &LibraryFilter { query: Some(q), ..Default::default() },
            )
            .await
            .unwrap()
            .len()
        }
    };

    assert_eq!(find("difraccion").await, 1, "accents fold");
    assert_eq!(find("rendija").await, 1, "the summary is searched too");
    assert_eq!(find("cuerpo").await, 1, "so is the body");
    assert_eq!(find("nada").await, 0);

    sb.close().await;
}

#[tokio::test]
async fn a_note_lists_every_board_it_appears_on() {
    let sb = Sandbox::new("boards").await;
    let db = &sb.pool;

    let a = repo::topics::create(db, None, "Memoria").await.unwrap();
    let b = repo::topics::create(db, None, "Diseño").await.unwrap();
    let note = written(db, "Efecto de prueba").await;

    // Boards import notes; they do not own them.
    repo::canvas::add_note_node(db, None, &a.id, &note.id, 0.0, 0.0).await.unwrap();
    repo::canvas::add_note_node(db, None, &b.id, &note.id, 0.0, 0.0).await.unwrap();

    let boards = repo::notes::boards(db, &note.id).await.unwrap();
    assert_eq!(boards.len(), 2);

    // Placing it twice on the same board does not duplicate the entry.
    repo::canvas::add_note_node(db, None, &a.id, &note.id, 40.0, 40.0).await.unwrap();
    assert_eq!(repo::notes::boards(db, &note.id).await.unwrap().len(), 2);

    sb.close().await;
}

// -- distillation and journal ---------------------------------------------

#[tokio::test]
async fn distilling_copies_and_titling_is_what_makes_it_atomic() {
    use study_app_lib::note_state::RecallState;

    let sb = Sandbox::new("distil").await;
    let db = &sb.pool;

    let source = written(db, "Nota larga").await;

    // The drag creates an untitled note pointing at its source.
    let extract = repo::notes::create(db, None, "", NoteKind::Long).await.unwrap();
    repo::notes::set_source(db, &extract.id, Some(&source.id)).await.unwrap();
    let extract = repo::notes::save_body(
        db,
        &extract.id,
        "",
        "",
        "{}",
        "<p>un concepto</p>",
        "un concepto",
    )
    .await
    .unwrap();

    // Untitled, so long and not atomic. A card with no front is not a card.
    assert_eq!(extract.kind, NoteKind::Long);
    assert_eq!(extract.recall_state(), RecallState::Draft);

    // The source keeps its body: pulling a concept out never damages it.
    let after = repo::notes::get(db, &source.id).await.unwrap().unwrap();
    assert_eq!(after.body_text, "cuerpo");

    // Titling it is the act that makes it atomic.
    let titled = repo::notes::save_body(
        db,
        &extract.id,
        "Un concepto",
        "",
        "{}",
        "<p>un concepto</p>",
        "un concepto",
    )
    .await
    .unwrap();
    assert_eq!(titled.kind, NoteKind::Atomic);

    sb.close().await;
}

#[tokio::test]
async fn titling_an_ordinary_long_note_leaves_it_long() {
    let sb = Sandbox::new("no-promote").await;
    let db = &sb.pool;

    // No source, so it was never an extract -- naming it must not reclassify it.
    let n = repo::notes::create(db, None, "", NoteKind::Long).await.unwrap();
    let named = repo::notes::save_body(db, &n.id, "Ahora con título", "", "{}", "<p>x</p>", "x")
        .await
        .unwrap();

    assert_eq!(named.kind, NoteKind::Long);

    sb.close().await;
}

#[tokio::test]
async fn the_journal_is_a_filter_over_captures_not_a_container() {
    let sb = Sandbox::new("journal").await;
    let db = &sb.pool;

    repo::notes::create(db, None, "Una idea", NoteKind::Long).await.unwrap();
    repo::notes::create(db, None, "Otra idea", NoteKind::Long).await.unwrap();

    let today = chrono::Utc::now().date_naive();
    let entries = repo::notes::captured_on(db, today).await.unwrap();
    assert_eq!(entries.len(), 2, "today's captures, with no day-document anywhere");

    // Nothing was created to hold them.
    let all = repo::notes::library(db, &Default::default()).await.unwrap();
    assert_eq!(all.len(), 2);

    let yesterday = today - chrono::Duration::days(1);
    assert!(repo::notes::captured_on(db, yesterday).await.unwrap().is_empty());

    let marks = repo::notes::days_with_captures(db, yesterday, today).await.unwrap();
    assert_eq!(marks.len(), 1, "only the day that has anything on it");

    sb.close().await;
}

// -- tags, mentions, tasks ------------------------------------------------

#[tokio::test]
async fn tags_fold_spanish_but_keep_the_enye_apart() {
    let sb = Sandbox::new("tags").await;
    let db = &sb.pool;

    let a = written(db, "Uno").await;
    let b = written(db, "Dos").await;

    // Same tag under different capitalisation and accents.
    let t1 = repo::tags::attach(db, &a.id, "Localización").await.unwrap();
    let t2 = repo::tags::attach(db, &b.id, "localizacion").await.unwrap();
    assert_eq!(t1.id, t2.id, "one tag, not two");

    // Different words stay different.
    let year = repo::tags::attach(db, &a.id, "año").await.unwrap();
    let anus = repo::tags::attach(db, &a.id, "ano").await.unwrap();
    assert_ne!(year.id, anus.id);

    assert_eq!(repo::tags::for_note(db, &a.id).await.unwrap().len(), 3);

    // Detaching the last carrier removes the tag itself: a tag list that only
    // grows is a list of everything you once thought worth naming.
    repo::tags::detach(db, &a.id, &year.id).await.unwrap();
    let names: Vec<_> = repo::tags::all(db).await.unwrap().into_iter().map(|t| t.tag.name).collect();
    assert!(!names.contains(&"año".to_string()));
    // Still carried by another note, so it survives.
    assert!(names.contains(&"Localización".to_string()));

    sb.close().await;
}

#[tokio::test]
async fn mentions_are_derived_from_the_body_and_rebuild_on_save() {
    let sb = Sandbox::new("mentions").await;
    let db = &sb.pool;

    let target = written(db, "Destino").await;
    let source = written(db, "Origen").await;

    let with_link = format!(
        r#"{{"type":"doc","content":[{{"type":"paragraph","content":[{{"type":"text","text":"ver","marks":[{{"type":"link","attrs":{{"href":"note:{}"}}}}]}}]}}]}}"#,
        target.id
    );

    repo::notes::save_body(db, &source.id, "Origen", "", &with_link, "<p>ver</p>", "ver")
        .await
        .unwrap();

    let back = repo::derived::backlinks(db, &target.id).await.unwrap();
    assert_eq!(back.len(), 1);
    assert_eq!(back[0].id, source.id);

    // Removing the mention from the body removes the edge: the body is the
    // truth and the table is only an index of it.
    repo::notes::save_body(db, &source.id, "Origen", "", "{}", "<p>ver</p>", "ver")
        .await
        .unwrap();
    assert!(repo::derived::backlinks(db, &target.id).await.unwrap().is_empty());

    sb.close().await;
}

#[tokio::test]
async fn only_tasks_with_an_explicit_date_ever_surface() {
    let sb = Sandbox::new("tasks").await;
    let db = &sb.pool;

    let note = written(db, "Pendientes").await;

    let doc = r#"{"type":"doc","content":[{"type":"taskList","content":[
        {"type":"taskItem","attrs":{"checked":false},
         "content":[{"type":"paragraph","content":[{"type":"text","text":"sin fecha"}]}]},
        {"type":"taskItem","attrs":{"checked":false},
         "content":[{"type":"paragraph","content":[{"type":"text","text":"con fecha due:2026-09-10"}]}]},
        {"type":"taskItem","attrs":{"checked":true},
         "content":[{"type":"paragraph","content":[{"type":"text","text":"hecha due:2026-09-10"}]}]}
    ]}]}"#;

    repo::notes::save_body(db, &note.id, "Pendientes", "", doc, "<p>x</p>", "x")
        .await
        .unwrap();

    let due = repo::derived::due_on(db, "2026-09-10").await.unwrap();
    assert_eq!(due.len(), 1, "undated and completed tasks never surface");
    assert!(due[0].task.text.contains("con fecha"));

    // Another day shows nothing: this view is a day's worth, never a backlog.
    assert!(repo::derived::due_on(db, "2026-09-11").await.unwrap().is_empty());

    sb.close().await;
}

// -- the day page ---------------------------------------------------------

#[tokio::test]
async fn a_day_page_appears_on_first_write_and_is_not_a_capture() {
    let sb = Sandbox::new("day-page").await;
    let db = &sb.pool;

    // Visiting a date must not manufacture a note.
    assert!(repo::notes::day_page(db, "2026-09-09").await.unwrap().is_none());

    let page = repo::notes::save_day_page(db, "2026-09-09", "{}", "<p>hola</p>", "hola")
        .await
        .unwrap();
    assert_eq!(page.journal_day.as_deref(), Some("2026-09-09"));
    assert_eq!(page.body_text, "hola");

    // Writing again edits the same page rather than making a second one.
    let again = repo::notes::save_day_page(db, "2026-09-09", "{}", "<p>hola dos</p>", "hola dos")
        .await
        .unwrap();
    assert_eq!(again.id, page.id);

    // Yesterday is a record you may correct, so past days write too.
    let past = repo::notes::save_day_page(db, "2026-09-01", "{}", "<p>ayer</p>", "ayer")
        .await
        .unwrap();
    assert_ne!(past.id, page.id);

    // The page is not one of the day's captures: it is what you wrote *on* the
    // day, not something you caught during it.
    let day = "2026-09-09".parse::<chrono::NaiveDate>().unwrap();
    let captures = repo::notes::captured_on(db, day).await.unwrap();
    assert!(captures.iter().all(|n| n.id != page.id));

    // In every other respect it is an ordinary note.
    assert!(repo::notes::get(db, &page.id).await.unwrap().is_some());

    sb.close().await;
}

// -- loose text -----------------------------------------------------------

#[tokio::test]
async fn loose_text_becomes_a_note_in_place() {
    let sb = Sandbox::new("loose-text").await;
    let db = &sb.pool;

    let topic = repo::topics::create(db, None, "Óptica").await.unwrap();
    let node = repo::canvas::add_text_node(db, None, &topic.id, "una idea suelta", 40.0, 60.0)
        .await
        .unwrap();

    assert_eq!(node.text.as_deref(), Some("una idea suelta"));
    assert!(node.note_id.is_none(), "loose text has no note behind it");

    repo::canvas::set_node_text(db, &node.id, "una idea mejor").await.unwrap();

    let (note, promoted) = repo::canvas::promote_text_node(db, &node.id).await.unwrap();

    // Same node, same place: the arrangement survives the decision.
    assert_eq!(promoted.id, node.id);
    assert_eq!(promoted.x, 40.0);
    assert_eq!(promoted.y, 60.0);
    assert_eq!(promoted.note_id.as_deref(), Some(note.id.as_str()));
    assert!(promoted.text.is_none());

    // The text is the body, not the title: titling is a separate act, and it is
    // the act that makes a note atomic.
    assert_eq!(note.body_text, "una idea mejor");
    assert_eq!(note.title, "");
    assert!(matches!(note.kind, NoteKind::Long));

    // It only works on loose text.
    assert!(repo::canvas::promote_text_node(db, &node.id).await.is_err());

    sb.close().await;
}

// -- pointers -------------------------------------------------------------

#[tokio::test]
async fn a_pointer_joins_two_cards_and_dies_with_them() {
    let sb = Sandbox::new("pointers").await;
    let db = &sb.pool;

    let topic = repo::topics::create(db, None, "Óptica").await.unwrap();
    let other = repo::topics::create(db, None, "Ondas").await.unwrap();

    let a = written(db, "Difracción").await;
    let b = written(db, "Interferencia").await;
    let na = repo::canvas::add_note_node(db, None, &topic.id, &a.id, 0.0, 0.0).await.unwrap();
    let nb = repo::canvas::add_note_node(db, None, &topic.id, &b.id, 200.0, 0.0).await.unwrap();

    let edge = repo::canvas::connect(db, None, &na.id, &nb.id).await.unwrap();
    assert_eq!(edge.topic_id, topic.id, "the board comes from the nodes");

    // Drawing the same arrow twice is not two facts.
    let again = repo::canvas::connect(db, None, &na.id, &nb.id).await.unwrap();
    assert_eq!(again.id, edge.id);
    assert_eq!(repo::canvas::snapshot(db, &topic.id).await.unwrap().edges.len(), 1);

    // The reverse direction is a different claim.
    repo::canvas::connect(db, None, &nb.id, &na.id).await.unwrap();
    assert_eq!(repo::canvas::snapshot(db, &topic.id).await.unwrap().edges.len(), 2);

    assert!(repo::canvas::connect(db, None, &na.id, &na.id).await.is_err());

    // A pointer cannot span two boards.
    let elsewhere = repo::canvas::add_note_node(db, None, &other.id, &a.id, 0.0, 0.0).await.unwrap();
    assert!(repo::canvas::connect(db, None, &na.id, &elsewhere.id).await.is_err());

    // It is an arrangement, so removing either end removes it.
    repo::canvas::delete_node(db, &nb.id).await.unwrap();
    assert!(repo::canvas::snapshot(db, &topic.id).await.unwrap().edges.is_empty());

    sb.close().await;
}

#[tokio::test]
async fn a_tag_can_be_dropped_in_one_act() {
    let sb = Sandbox::new("delete-tag").await;
    let db = &sb.pool;

    let a = written(db, "Difracción").await;
    let b = written(db, "Interferencia").await;

    let tag = repo::tags::attach(db, &a.id, "ondas").await.unwrap();
    repo::tags::attach(db, &b.id, "ondas").await.unwrap();

    // Detaching one carrier leaves the tag alive, because the other still holds it.
    repo::tags::detach(db, &a.id, &tag.id).await.unwrap();
    assert_eq!(repo::tags::all(db).await.unwrap().len(), 1);

    // Dropping it takes it off everything at once.
    repo::tags::delete(db, &tag.id).await.unwrap();
    assert!(repo::tags::all(db).await.unwrap().is_empty());
    assert!(repo::tags::for_note(db, &b.id).await.unwrap().is_empty());

    // The notes themselves are untouched: a classification is not the thing.
    assert!(repo::notes::get(db, &b.id).await.unwrap().is_some());

    sb.close().await;
}

// -- files and links ------------------------------------------------------

#[tokio::test]
async fn an_imported_file_is_copied_and_swept_when_nothing_points_at_it() {
    use study_app_lib::repo::assets;

    let sb = Sandbox::new("assets").await;
    let db = &sb.pool;

    let data = std::env::temp_dir().join(format!("study-assets-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&data).unwrap();

    // Two files with the same name must not collide: the copy is named by id.
    let one = data.join("diagram.png");
    let two = data.join("nested");
    std::fs::create_dir_all(&two).unwrap();
    let two = two.join("diagram.png");
    std::fs::write(&one, b"first").unwrap();
    std::fs::write(&two, b"second").unwrap();

    let a = assets::import(db, &data, &one).await.unwrap();
    let b = assets::import(db, &data, &two).await.unwrap();

    assert_ne!(a.rel_path, b.rel_path);
    assert_eq!(a.file_name, "diagram.png", "the original name is kept for display");
    assert!(assets::dir(&data).join(&a.rel_path).exists(), "the file was copied in");
    assert_eq!(a.byte_size, 5);

    // Only what the webview can actually render inline.
    let bad = data.join("thing.docx");
    std::fs::write(&bad, b"x").unwrap();
    assert!(assets::import(db, &data, &bad).await.is_err());

    // Placed on a board, it survives the sweep.
    let topic = repo::topics::create(db, None, "Óptica").await.unwrap();
    let node = repo::canvas::add_asset_node(
        db,
        None,
        &topic.id,
        &a.id,
        study_app_lib::domain::CanvasNodeKind::Image,
        0.0,
        0.0,
    )
    .await
    .unwrap();

    let swept = assets::sweep(db, &data).await.unwrap();
    assert_eq!(swept, 1, "only the unplaced one goes");
    assert!(assets::get(db, &a.id).await.unwrap().is_some());
    assert!(assets::get(db, &b.id).await.unwrap().is_none());
    assert!(!assets::dir(&data).join(&b.rel_path).exists(), "its file went too");

    // The snapshot carries it, so the canvas draws in one round trip.
    let snap = repo::canvas::snapshot(db, &topic.id).await.unwrap();
    assert_eq!(snap.assets.len(), 1);
    assert_eq!(snap.assets[0].id, a.id);

    // Taking it off the board orphans it, and the next sweep collects it.
    repo::canvas::delete_node(db, &node.id).await.unwrap();
    assert_eq!(assets::sweep(db, &data).await.unwrap(), 1);
    assert!(!assets::dir(&data).join(&a.rel_path).exists());

    std::fs::remove_dir_all(&data).ok();
    sb.close().await;
}

#[tokio::test]
async fn a_link_points_outwards_and_is_not_an_asset() {
    let sb = Sandbox::new("links").await;
    let db = &sb.pool;

    let topic = repo::topics::create(db, None, "Óptica").await.unwrap();

    // Typing a bare host means the web, not a relative path.
    let node = repo::canvas::add_link_node(db, None, &topic.id, "example.com/paper", 0.0, 0.0)
        .await
        .unwrap();
    assert_eq!(node.url.as_deref(), Some("https://example.com/paper"));
    assert!(node.asset_id.is_none(), "nothing was copied");

    // An address already carrying a scheme is left alone.
    let https = repo::canvas::add_link_node(db, None, &topic.id, "http://x.test", 10.0, 0.0)
        .await
        .unwrap();
    assert_eq!(https.url.as_deref(), Some("http://x.test"));

    assert!(repo::canvas::add_link_node(db, None, &topic.id, "   ", 0.0, 0.0).await.is_err());

    sb.close().await;
}

#[tokio::test]
async fn placing_a_question_on_a_board_is_what_attaches_it() {
    let sb = Sandbox::new("question-nodes").await;
    let db = &sb.pool;

    let topic = repo::topics::create(db, None, "Óptica").await.unwrap();

    // A doubt caught while writing, with no board: exactly what capture makes
    // when nothing is open, and the reason the board panel used to look empty.
    let q = repo::questions::create(db, None, "¿Por qué se curva?", QuestionOrigin::Capture, None, None)
        .await
        .unwrap();
    assert!(q.topic_id.is_none());
    assert!(
        repo::questions::queue(db, Some(&topic.id)).await.unwrap().is_empty(),
        "it is not the board's until it is placed"
    );

    let node = repo::canvas::add_question_node(db, None, &topic.id, &q.id, 20.0, 30.0)
        .await
        .unwrap();

    assert_eq!(node.question_id.as_deref(), Some(q.id.as_str()));

    // Placement is membership: the board's panel now finds it.
    let queue = repo::questions::queue(db, Some(&topic.id)).await.unwrap();
    assert_eq!(queue.len(), 1);
    assert_eq!(queue[0].id, q.id);

    // And the canvas draws it without a second round trip.
    let snap = repo::canvas::snapshot(db, &topic.id).await.unwrap();
    assert_eq!(snap.questions.len(), 1);
    assert_eq!(snap.questions[0].id, q.id);

    // Answering it removes it from the queue; the card is stale, not the truth.
    let answer = written(db, "Difracción").await;
    repo::questions::resolve(db, &q.id, &answer.id).await.unwrap();
    assert!(repo::questions::queue(db, Some(&topic.id)).await.unwrap().is_empty());

    sb.close().await;
}

#[tokio::test]
async fn a_task_with_no_date_is_findable_instead_of_lost() {
    let sb = Sandbox::new("undated-tasks").await;
    let db = &sb.pool;

    // A day's page with two tasks: one dated, one not — which is what typing a
    // checkbox into the journal without thinking about a date produces.
    let body = serde_json::json!({
        "type": "doc",
        "content": [{
            "type": "taskList",
            "content": [
                {
                    "type": "taskItem",
                    "attrs": { "checked": false },
                    "content": [{ "type": "paragraph", "content": [
                        { "type": "text", "text": "llamar al laboratorio due:2026-09-09" }
                    ]}]
                },
                {
                    "type": "taskItem",
                    "attrs": { "checked": false },
                    "content": [{ "type": "paragraph", "content": [
                        { "type": "text", "text": "releer el capítulo de ondas" }
                    ]}]
                }
            ]
        }]
    })
    .to_string();

    let page = repo::notes::save_day_page(db, "2026-09-09", &body, "<p/>", "tareas")
        .await
        .unwrap();

    // The panel stays one day wide, and that day means both things written on
    // its page and things dated for it — so both of these belong to it.
    let today = repo::derived::due_on(db, "2026-09-09").await.unwrap();
    assert_eq!(today.len(), 2);
    assert!(today[0].task.text.contains("laboratorio"), "dated first");
    assert!(today[1].task.due_on.is_none());
    assert_eq!(
        today[0].journal_day.as_deref(),
        Some("2026-09-09"),
        "a day page has no title, so the screen needs the date to say where it came from"
    );

    // The screen shows everything, which is the whole point: the undated one
    // used to be indexed and then reachable from nowhere.
    let all = repo::derived::all_open(db).await.unwrap();
    assert_eq!(all.len(), 2);
    assert!(all.iter().any(|t| t.task.text.contains("ondas") && t.task.due_on.is_none()));

    // Dated first, undated last, so "when" still orders the screen.
    assert!(all[0].task.due_on.is_some());
    assert!(all[1].task.due_on.is_none());

    // Ticking it in the body takes it off both, because the body is the truth.
    let done = body.replace("\"checked\":false", "\"checked\":true");
    repo::notes::save_day_page(db, "2026-09-09", &done, "<p/>", "tareas")
        .await
        .unwrap();
    assert!(repo::derived::all_open(db).await.unwrap().is_empty());
    assert!(repo::notes::get(db, &page.id).await.unwrap().is_some());

    sb.close().await;
}

#[tokio::test]
async fn a_task_crosses_the_wire_as_flat_fields() {
    // `sqlx(flatten)` and `serde(flatten)` are different attributes, and only
    // the first was there. The row read fine and then serialised as
    // `{ task: { text, due_on, ... }, note_title }`, so every field the
    // frontend reads off a task arrived undefined — tasks that existed, were
    // indexed, and were returned by the query still appeared nowhere.
    let sb = Sandbox::new("task-shape").await;
    let db = &sb.pool;

    let body = serde_json::json!({
        "type": "doc",
        "content": [{
            "type": "taskList",
            "content": [{
                "type": "taskItem",
                "attrs": { "checked": false },
                "content": [{ "type": "paragraph", "content": [
                    { "type": "text", "text": "esto es una tarea" }
                ]}]
            }]
        }]
    })
    .to_string();

    repo::notes::save_day_page(db, "2026-09-09", &body, "<p/>", "t")
        .await
        .unwrap();

    let tasks = repo::derived::all_open(db).await.unwrap();
    let json = serde_json::to_value(&tasks[0]).unwrap();

    for field in ["id", "note_id", "text", "done", "due_on", "position", "note_title"] {
        assert!(json.get(field).is_some(), "`{field}` must be a top-level field");
    }
    assert!(json.get("task").is_none(), "nothing may be nested under `task`");
    assert_eq!(json["text"], "esto es una tarea");
    assert_eq!(json["due_on"], serde_json::Value::Null);

    sb.close().await;
}

#[tokio::test]
async fn a_day_shows_its_own_undated_tasks_but_they_do_not_follow_you() {
    let sb = Sandbox::new("day-tasks").await;
    let db = &sb.pool;

    let undated = |text: &str| {
        serde_json::json!({
            "type": "doc",
            "content": [{
                "type": "taskList",
                "content": [{
                    "type": "taskItem",
                    "attrs": { "checked": false },
                    "content": [{ "type": "paragraph", "content": [
                        { "type": "text", "text": text }
                    ]}]
                }]
            }]
        })
        .to_string()
    };

    repo::notes::save_day_page(db, "2026-09-08", &undated("lo de ayer"), "<p/>", "a")
        .await
        .unwrap();
    repo::notes::save_day_page(db, "2026-09-09", &undated("lo de hoy"), "<p/>", "b")
        .await
        .unwrap();

    // Each day sees what was written on its own page.
    let eighth = repo::derived::due_on(db, "2026-09-08").await.unwrap();
    assert_eq!(eighth.len(), 1);
    assert_eq!(eighth[0].task.text, "lo de ayer");

    // And nothing else's: an undated task does not follow you into tomorrow.
    // That is the line between "what is on today" and a backlog.
    let ninth = repo::derived::due_on(db, "2026-09-09").await.unwrap();
    assert_eq!(ninth.len(), 1);
    assert_eq!(ninth[0].task.text, "lo de hoy");

    // A note that is not a day page contributes only when its task is dated.
    let loose = written(db, "Difracción").await;
    repo::notes::save_body(db, &loose.id, "Difracción", "", &undated("suelta"), "<p/>", "x")
        .await
        .unwrap();
    assert_eq!(repo::derived::due_on(db, "2026-09-09").await.unwrap().len(), 1);
    assert_eq!(repo::derived::all_open(db).await.unwrap().len(), 3);

    sb.close().await;
}

#[tokio::test]
async fn ticking_a_task_edits_the_note_and_costs_no_recall() {
    let sb = Sandbox::new("tick").await;
    let db = &sb.pool;

    let body = serde_json::json!({
        "type": "doc",
        "content": [{
            "type": "taskList",
            "content": [
                { "type": "taskItem", "attrs": { "checked": false },
                  "content": [{ "type": "paragraph", "content": [
                      { "type": "text", "text": "primera" }]}]},
                { "type": "taskItem", "attrs": { "checked": false },
                  "content": [{ "type": "paragraph", "content": [
                      { "type": "text", "text": "segunda" }]}]}
            ]
        }]
    })
    .to_string();

    let html = concat!(
        "<ul data-type=\"taskList\">",
        "<li data-checked=\"false\" data-type=\"taskItem\"><label>",
        "<input type=\"checkbox\"><span></span></label><div><p>primera</p></div></li>",
        "<li data-checked=\"false\" data-type=\"taskItem\"><label>",
        "<input type=\"checkbox\"><span></span></label><div><p>segunda</p></div></li></ul>"
    );

    let note = repo::notes::create(db, None, "Pendientes", NoteKind::Long).await.unwrap();
    repo::notes::save_body(db, &note.id, "Pendientes", "", &body, html, "primera segunda")
        .await
        .unwrap();

    let before = repo::notes::get(db, &note.id).await.unwrap().unwrap();
    let tasks = repo::derived::all_open(db).await.unwrap();
    assert_eq!(tasks.len(), 2);

    // Tick the second one.
    let second = tasks.iter().find(|t| t.task.text == "segunda").unwrap();
    repo::derived::set_done(db, &second.task.id, true).await.unwrap();

    // It leaves the open list, and the first is untouched.
    let after = repo::derived::all_open(db).await.unwrap();
    assert_eq!(after.len(), 1);
    assert_eq!(after[0].task.text, "primera");

    let updated = repo::notes::get(db, &note.id).await.unwrap().unwrap();

    // The document moved, both halves of it: the attribute the stylesheet reads
    // and the input that actually draws ticked. Only the second item.
    assert!(updated.body_json.contains("\"checked\":true"));
    assert!(updated.body_html.contains(
        "<li data-checked=\"false\" data-type=\"taskItem\"><label><input type=\"checkbox\">"
    ));
    assert!(updated.body_html.contains(
        "<li data-checked=\"true\" data-type=\"taskItem\"><label><input type=\"checkbox\" checked=\"checked\">"
    ));
    assert!(updated.body_html.contains("primera"));
    assert!(updated.body_html.contains("segunda"));

    // Ticking is not writing: the text is identical, so the hash is, and the
    // cooling window must not have started — otherwise checking a box would
    // silently block a recall you had earned.
    assert_eq!(updated.content_hash, before.content_hash);
    assert_eq!(updated.content_edited_at, before.content_edited_at);

    // And it goes back — with the same id, because a task's id is derived from
    // where it sits rather than regenerated by every reindex.
    let same = repo::derived::all_open(db).await.unwrap();
    assert!(same.iter().all(|t| t.task.id != second.task.id), "ticked one is closed");
    repo::derived::set_done(db, &second.task.id, false).await.unwrap();
    assert_eq!(repo::derived::all_open(db).await.unwrap().len(), 2);
    let back = repo::notes::get(db, &note.id).await.unwrap().unwrap();
    assert!(!back.body_html.contains("checked=\"checked\""));

    sb.close().await;
}
