mod ai;
mod commands;
mod credentials;
mod git;
mod models;
mod processes;
mod storage;
mod updater;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            app.manage(storage::AppState::load(app.handle()).map_err(std::io::Error::other)?);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_projects,
            commands::save_project,
            commands::delete_project,
            commands::open_url,
            commands::open_path,
            commands::rescan_projects,
            commands::get_projects_dir,
            commands::service_action,
            commands::service_processes,
            commands::build_project,
            commands::read_log,
            commands::clear_log,
            commands::get_settings,
            commands::save_settings,
            commands::list_chats,
            commands::save_chats,
            commands::git_status,
            commands::git_stage,
            commands::git_commit,
            commands::git_push,
            commands::git_branches,
            commands::git_current_branch,
            commands::git_switch_branch,
            commands::git_pull,
            commands::provider_key_status,
            commands::save_provider_key,
            commands::ask_ai,
            updater::check_update,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
