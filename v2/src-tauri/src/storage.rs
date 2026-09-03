use std::{
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};

use tauri::{AppHandle, Manager};

use crate::models::{Chat, Project, Settings};

pub struct AppState {
    pub data_dir: PathBuf,
    /// Directory scanned for drop-in project JSON files written by a coding
    /// agent or the user. Each `*.json` file becomes a `source: "file"` project.
    pub projects_dir: PathBuf,
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
    // `rename` replaces an existing file on Unix, but fails on Windows.
    // Remove the old snapshot first so Save Project/Settings behaves the same
    // on both platforms.
    #[cfg(windows)]
    if path.exists() {
        fs::remove_file(path).map_err(|error| error.to_string())?;
    }
    fs::rename(temporary, path).map_err(|error| error.to_string())
}

fn migrate_legacy_files(data_dir: &Path) -> Result<(), String> {
    let Some(base) = dirs::data_dir() else {
        return Ok(());
    };
    let candidates = [base.join("VibeWing"), base.join("vibewing"), base.join("VibeWing Desktop")];
    let Some(legacy) = candidates.iter().find(|path| path.is_dir() && *path != data_dir) else { return Ok(()); };
    // Only migrate files that are absent in the Tauri data directory. This keeps
    // an existing Tauri profile authoritative while allowing Electron users to upgrade.
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

/// Scan the projects drop-in directory for `*.json` files and parse each into a
/// `Project`. Files are addressed by `file:<filename-stem>` so they never collide
/// with UI-authored projects (which use timestamp ids) and dedupe on rescan.
fn discover_file_projects(dir: &Path) -> Vec<Project> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
            continue;
        };
        if stem.eq_ignore_ascii_case("readme") {
            continue;
        }
        let Ok(raw) = fs::read_to_string(&path) else {
            continue;
        };
        let Ok(mut project) = serde_json::from_str::<Project>(&raw) else {
            continue;
        };
        project.id = format!("file:{stem}");
        project.source = "file".into();
        out.push(project);
    }
    out
}

const PROJECTS_README: &str = "\
VibeWing 项目目录 / VibeWing projects
=====================================

把「项目描述」写成一个 .json 文件丢进这个文件夹，VibeWing 启动时会自动读取，
无需在界面里手动填写。你的 coding agent（把仓库里的 SKILL.md 交给它）就能自动
帮你生成这些文件。

文件名任意（不含空格最好），例如：my-app.json
文件里至少要有 name 和 path 两个字段。示例：

{
  \"id\": \"file:my-app\",
  \"name\": \"我的项目\",
  \"path\": \"/Users/you/code/my-app\",
  \"frontend_path\": \"/Users/you/code/my-app/web\",
  \"frontend_cmd\": \"npm run dev\",
  \"frontend_port\": \"5173\",
  \"backend_path\": \"/Users/you/code/my-app/server\",
  \"backend_cmd\": \"python main.py\",
  \"backend_port\": \"8000\"
}

注意：path / *_path 必须是本机真实绝对路径。删除文件即移除该项目（界面里点停止不影响这里）。
";

impl AppState {
    pub fn load(app: &AppHandle) -> Result<Self, String> {
        let data_dir = app
            .path()
            .app_data_dir()
            .map_err(|error| error.to_string())?;
        fs::create_dir_all(data_dir.join("logs")).map_err(|error| error.to_string())?;
        let projects_dir = data_dir.join("projects");
        fs::create_dir_all(&projects_dir).map_err(|error| error.to_string())?;
        if !projects_dir.join("README.txt").exists() {
            let _ = fs::write(projects_dir.join("README.txt"), PROJECTS_README);
        }
        migrate_legacy_files(&data_dir)?;
        // Only UI-authored projects live in projects.json; file-sourced entries
        // (if any leaked in from an older build) are dropped and re-discovered.
        let mut ui: Vec<Project> = read_json(&data_dir.join("projects.json"));
        ui.retain(|project| project.source != "file");
        let state = Self {
            projects: Mutex::new(ui),
            settings: Mutex::new(read_json(&data_dir.join("settings.json"))),
            chats: Mutex::new(read_json(&data_dir.join("chats.json"))),
            data_dir,
            projects_dir,
        };
        state.merge_discovered()?;
        Ok(state)
    }

    /// Re-read the projects directory, adding new file projects and refreshing
    /// existing ones by id while preserving their live pids, then persist.
    pub fn merge_discovered(&self) -> Result<(), String> {
        let discovered = discover_file_projects(&self.projects_dir);
        let mut projects = self.projects.lock().map_err(|error| error.to_string())?;
        for incoming in discovered {
            if let Some(existing) = projects.iter_mut().find(|project| project.id == incoming.id) {
                let frontend_pid = existing.frontend_pid;
                let backend_pid = existing.backend_pid;
                *existing = incoming;
                existing.frontend_pid = frontend_pid;
                existing.backend_pid = backend_pid;
            } else {
                projects.push(incoming);
            }
        }
        let snapshot = projects.clone();
        drop(projects);
        self.persist(&snapshot)
    }

    /// Persist the in-memory projects: UI-authored ones go to `projects.json`,
    /// file-sourced ones are written back to their own files in `projects_dir`.
    pub fn persist(&self, projects: &[Project]) -> Result<(), String> {
        let ui: Vec<&Project> = projects.iter().filter(|project| project.source != "file").collect();
        write_json(&self.data_dir.join("projects.json"), &ui)?;
        for project in projects.iter().filter(|project| project.source == "file") {
            let stem = project.id.strip_prefix("file:").unwrap_or(&project.id);
            let file = self.projects_dir.join(format!("{stem}.json"));
            write_json(&file, project)?;
        }
        Ok(())
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
