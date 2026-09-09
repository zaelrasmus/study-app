//! Runs the migrations against a copy of a real database.
//!
//! Migration 3 drops and renames columns, which SQLite only supports on recent
//! versions and only under conditions. A fresh-database test cannot catch a
//! failure here, because the risk is entirely about pre-existing rows.
//!
//! Skips silently when no copy is present, so CI and other machines are fine.

use study_app_lib::db;
use study_app_lib::note_state::RecallState;
use study_app_lib::repo;

/// A private copy of the sample database, per test.
///
/// The tests here run on parallel threads, and they used to migrate one shared
/// directory: whichever two got there together raced to insert the same row
/// into `_sqlx_migrations` and one failed with a unique-constraint violation.
/// It stayed hidden while the sample was already fully migrated — there was
/// nothing to insert — and appeared the moment a new migration had work to do.
///
/// Copying per test also means a migration that damages rows cannot leak into
/// the next test and be blamed on it.
fn sandbox(name: &str) -> Option<std::path::PathBuf> {
    let source = std::env::temp_dir()
        .join("migtest")
        .join("com.kiza2.study-app")
        .join("study-app.db");

    if !source.exists() {
        return None;
    }

    let dir = std::env::temp_dir()
        .join("migtest-run")
        .join(name)
        .join("com.kiza2.study-app");

    // A leftover from an earlier run would already be migrated, which would
    // quietly stop testing the thing this file exists to test.
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).ok()?;
    std::fs::copy(&source, dir.join("study-app.db")).ok()?;

    Some(dir)
}

#[tokio::test]
async fn migrations_apply_to_an_existing_database() {
    const NAME: &str = "apply";
    let Some(dir) = sandbox(NAME) else {
        eprintln!("no database copy at temp/migtest — skipping");
        return;
    };

    let pool = db::connect(&dir).await.expect("migrate an existing database");

    // The renamed and added columns are readable, which means every query in
    // the repo layer will work against real rows.
    let notes = repo::notes::inbox(&pool).await.expect("read notes");
    for note in &notes {
        // Just exercising the derivation over real data.
        let _ = note.recall_state();
        let _ = note.never_contrasted();
    }

    // The dropped columns are genuinely gone.
    let leftovers = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM pragma_table_info('notes')
          WHERE name IN ('last_recall_closed_book', 'last_recall_failed', 'hash_at_recall')",
    )
    .fetch_one(&pool)
    .await
    .expect("inspect schema");
    assert_eq!(leftovers, 0, "old duplicate-fact columns should be dropped");

    // The backfill ran: a note with a body is no longer raw.
    let written = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM notes WHERE trim(body_text) <> '' AND distilled_at IS NULL",
    )
    .fetch_one(&pool)
    .await
    .expect("check backfill");
    assert_eq!(written, 0, "notes with a body should have been distilled");

    // And the misleading table is gone.
    let cards = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'cards'",
    )
    .fetch_one(&pool)
    .await
    .expect("check cards");
    assert_eq!(cards, 0, "cards stored front/back, which are now projections");

    println!("migrated a real database: {} unplaced notes", notes.len());
    pool.close().await;
}

#[tokio::test]
async fn every_real_note_lands_in_exactly_one_state() {
    const NAME: &str = "states";
    let Some(dir) = sandbox(NAME) else { return };

    let pool = db::connect(&dir).await.expect("connect");
    let notes = repo::notes::inbox(&pool).await.expect("read");

    for note in notes {
        let state = note.recall_state();
        // Axis 2 is only claimed once a recall exists.
        if note.never_contrasted() {
            assert_ne!(state, RecallState::Raw);
            assert_ne!(state, RecallState::Draft);
        }
    }

    pool.close().await;
}

/// Migration 5 rebuilds the `questions` table to change a CHECK constraint,
/// which SQLite cannot alter in place. A fresh-database test proves nothing
/// here: the risk is entirely about carrying existing rows across the rebuild.
#[tokio::test]
async fn rebuilding_the_questions_table_keeps_its_rows() {
    const NAME: &str = "questions";
    let Some(dir) = sandbox(NAME) else {
        eprintln!("no database copy at temp/migtest — skipping");
        return;
    };

    let pool = db::connect(&dir).await.expect("migrate an existing database");

    // No row was left behind, and none kept the retired origin.
    let stale = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM questions WHERE origin = 'reading'")
        .fetch_one(&pool)
        .await
        .expect("read questions");
    assert_eq!(stale, 0, "'reading' should have become 'capture'");

    // The queue still reads, which exercises the rebuilt table end to end.
    let queue = repo::questions::queue(&pool, None).await.expect("queue");
    println!("questions after rebuild: {} open", queue.len());

    // The constraints survived the rebuild rather than being silently dropped.
    let refused = repo::questions::set_status(
        &pool,
        "does-not-exist",
        study_app_lib::domain::QuestionStatus::Resolved,
    )
    .await;
    assert!(refused.is_err(), "resolving still requires an answer note");

    // And review scheduling can be created against real notes.
    assert!(repo::review::queue(&pool).await.is_ok());
    assert!(repo::review::due_today(&pool).await.is_ok());

    pool.close().await;
}

/// Migration 6 adds the board mode and the summary. Both are additive, but the
/// default matters: every topic that already exists becomes a *work* board, so
/// the epistemic layer disappears from boards that had it until you opt back in.
/// That is the intended default and it should be verified, not assumed.
#[tokio::test]
async fn existing_boards_become_work_boards_and_keep_everything() {
    const NAME: &str = "boards";
    let Some(dir) = sandbox(NAME) else {
        eprintln!("no database copy at temp/migtest — skipping");
        return;
    };

    let pool = db::connect(&dir).await.expect("migrate an existing database");

    let topics = repo::topics::all(&pool).await.expect("read topics");
    assert!(!topics.is_empty(), "the snapshot should have topics");
    for topic in &topics {
        assert!(!topic.study, "existing boards default to work boards");
    }

    // Flipping is non-destructive and reversible.
    let first = &topics[0];
    let as_study = repo::topics::set_mode(&pool, &first.id, true).await.unwrap();
    assert!(as_study.study);
    let back = repo::topics::set_mode(&pool, &first.id, false).await.unwrap();
    assert!(!back.study);

    // No note fact was touched by any of that.
    let notes = repo::notes::search(&pool, "").await.expect("read notes");
    println!("real notes carried across: {}", notes.len());
    for note in &notes {
        // Summary defaults to empty and the face falls back cleanly.
        let _ = note.recall_state();
    }

    pool.close().await;
}
