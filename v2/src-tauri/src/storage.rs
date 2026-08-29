use std::{
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};

use tauri::{AppHandle, Manager};

use crate::models::{Chat, Project, Settings};

pub struct AppState {
    pub data_dir: PathBuf,
    pub projects: Mutex<Vec<Project>>,
    pub settings: Mutex<Settings>,
    pub chats: Mutex<Vec<Chat>>,
}

fn read_json<T: serde::de::DeserializeOwned + Default>(path: &Path) -> T {
    fs::read_to_string(path)
        .ok()
        .and_then(|value| serde_json::from_str(&value).ok())
        .unwrap_or_default()
}

fn write_json<T: serde::Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let temporary = path.with_extension("tmp");
    let content = serde_json::to_string_pretty(value).map_err(|error| error.to_string())?;
    fs::write(&temporary, content).map_err(|error| error.to_string())?;
    fs::rename(temporary, path).map_err(|error| error.to_string())
}

fn migrate_legacy_files(data_dir: &Path) -> Result<(), String> {
    let Some(base) = dirs::data_dir() else {
        return Ok(());
    };
    let legacy = base.join("VibeWing");
    if legacy == data_dir || data_dir.join("projects.json").exists() || !legacy.is_dir() {
        return Ok(());
    }
    for name in [
        "projects.json",
        "settings.json",
        "chats.json",
        "credentials.json",
    ] {
        let source = legacy.join(name);
        let target = data_dir.join(name);
        if source.is_file() && !target.exists() {
            fs::copy(source, target).map_err(|error| error.to_string())?;
        }
    }
    let source_logs = legacy.join("logs");
    let target_logs = data_dir.join("logs");
    if source_logs.is_dir() {
        for entry in fs::read_dir(source_logs).map_err(|error| error.to_string())? {
            let entry = entry.map_err(|error| error.to_string())?;
            let target = target_logs.join(entry.file_name());
            if entry.path().is_file() && !target.exists() {
                fs::copy(entry.path(), target).map_err(|error| error.to_string())?;
            }
        }
    }
    Ok(())
}

impl AppState {
    pub fn load(app: &AppHandle) -> Result<Self, String> {
        let data_dir = app
            .path()
            .app_data_dir()
            .map_err(|error| error.to_string())?;
        fs::create_dir_all(data_dir.join("logs")).map_err(|error| error.to_string())?;
        migrate_legacy_files(&data_dir)?;
        Ok(Self {
            projects: Mutex::new(read_json(&data_dir.join("projects.json"))),
            settings: Mutex::new(read_json(&data_dir.join("settings.json"))),
            chats: Mutex::new(read_json(&data_dir.join("chats.json"))),
            data_dir,
        })
    }

    pub fn save_projects(&self, projects: &[Project]) -> Result<(), String> {
        write_json(&self.data_dir.join("projects.json"), &projects)
    }

    pub fn save_settings(&self, settings: &Settings) -> Result<(), String> {
        write_json(&self.data_dir.join("settings.json"), settings)
    }

    pub fn save_chats(&self, chats: &[Chat]) -> Result<(), String> {
        write_json(&self.data_dir.join("chats.json"), &chats)
    }

    pub fn log_path(&self, project_id: &str, service: &str) -> PathBuf {
        self.data_dir
            .join("logs")
            .join(format!("{project_id}-{service}.log"))
    }
}
