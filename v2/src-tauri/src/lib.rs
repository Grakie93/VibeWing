mod ai;
mod commands;
mod credentials;
mod git;
mod models;
mod processes;
mod storage;

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
            commands::service_action,
            commands::read_log,
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
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if let Some(state) = window.app_handle().try_state::<storage::AppState>() {
                    if let Ok(mut projects) = state.projects.lock() {
                        processes::stop_all(&mut projects);
                        let _ = state.save_projects(&projects);
                    }
                }
                api.prevent_close();
                window.app_handle().exit(0);
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
