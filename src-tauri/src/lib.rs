pub mod commands;
pub mod db;
pub mod domain;
pub mod error;
pub mod note_state;
pub mod repo;
pub mod review;

use tauri::Manager;

/// The app's data directory, kept in managed state.
///
/// Assets live beside the database, and commands that copy files in need to
/// know where that is. Resolving it per call would mean every one of them
/// needed an `AppHandle`, which is a lot of plumbing for one path.
pub struct DataDir(pub std::path::PathBuf);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;

            // This blocks the setup hook, and a setup hook that never returns
            // means no window ever appears. `db::connect` sets a busy timeout
            // so a locked database surfaces as an error instead of a hang.
            let pool = tauri::async_runtime::block_on(db::connect(&data_dir))?;

            app.manage(pool);
            app.manage(DataDir(data_dir));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // topics
            commands::awake_topics,
            commands::all_topics,
            commands::create_topic,
            commands::rename_topic,
            commands::open_topic,
            commands::set_topic_mode,
            commands::delete_topic,
            // notes
            commands::create_note,
            commands::get_note,
            commands::inbox_notes,
            commands::topic_notes,
            commands::save_note,
            commands::set_note_kind,
            commands::record_recall,
            commands::set_note_source,
            commands::record_contrast,
            commands::search_notes,
            commands::library_notes,
            commands::note_boards,
            commands::distil_to_canvas,
            commands::all_tags,
            commands::note_tags,
            commands::attach_tag,
            commands::detach_tag,
            commands::delete_tag,
            commands::note_backlinks,
            commands::tasks_due,
            commands::all_tasks,
            commands::set_task_done,
            commands::journal_day,
            commands::journal_page,
            commands::save_journal_page,
            commands::journal_marks,
            commands::delete_note,
            // questions
            commands::question_queue,
            commands::unattached_questions,
            commands::create_question,
            commands::resolve_question,
            commands::set_question_status,
            commands::set_question_topic,
            commands::open_question_count,
            // review
            commands::review_queue,
            commands::review_due_today,
            commands::grade_card,
            commands::resolve_drifted_card,
            // canvas
            commands::canvas_snapshot,
            commands::create_frame,
            commands::move_frame,
            commands::label_frame,
            commands::reorder_frames,
            commands::delete_frame,
            commands::add_note_node,
            commands::create_note_on_canvas,
            commands::add_topic_node,
            commands::add_ink_node,
            commands::add_link_node,
            commands::add_question_node,
            commands::import_asset,
            commands::asset_dir,
            commands::add_text_node,
            commands::set_node_text,
            commands::promote_text_node,
            commands::connect_nodes,
            commands::label_edge,
            commands::disconnect_nodes,
            commands::add_asset_node,
            commands::move_node,
            commands::set_node_frame,
            commands::set_node_color,
            commands::delete_node,
            commands::topic_document,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
