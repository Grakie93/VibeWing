use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

use tauri::State;

use crate::{
    models::{Project, ProjectView, ServiceKind, Settings},
    processes,
    storage::AppState,
};

fn views(projects: &[Project]) -> Vec<ProjectView> {
    projects
        .iter()
        .cloned()
        .map(|project| ProjectView {
            frontend_running: processes::service_running(&project, ServiceKind::Frontend),
            backend_running: processes::service_running(&project, ServiceKind::Backend),
            project,
        })
        .collect()
}

#[tauri::command]
pub fn list_projects(state: State<'_, AppState>) -> Result<Vec<ProjectView>, String> {
    let projects = state.projects.lock().map_err(|error| error.to_string())?;
    Ok(views(&projects))
}

#[tauri::command]
pub fn save_project(
    state: State<'_, AppState>,
    mut project: Project,
) -> Result<ProjectView, String> {
    if project.name.trim().is_empty() || project.path.trim().is_empty() {
        return Err("请填写项目名称和主目录".into());
    }
    if project.frontend_path.is_empty() {
        project.frontend_path = project.path.clone();
    }
    if project.backend_path.is_empty() {
        project.backend_path = project.path.clone();
    }
    let mut projects = state.projects.lock().map_err(|error| error.to_string())?;
    if project.id.is_empty() {
        project.id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| error.to_string())?
            .as_millis()
            .to_string();
        projects.push(project.clone());
    } else if let Some(existing) = projects
        .iter_mut()
        .find(|existing| existing.id == project.id)
    {
        let frontend_pid = existing.frontend_pid;
        let backend_pid = existing.backend_pid;
        *existing = project.clone();
        existing.frontend_pid = frontend_pid;
        existing.backend_pid = backend_pid;
        project = existing.clone();
    } else {
        return Err("项目不存在".into());
    }
    state.save_projects(&projects)?;
    Ok(ProjectView {
        frontend_running: processes::service_running(&project, ServiceKind::Frontend),
        backend_running: processes::service_running(&project, ServiceKind::Backend),
        project,
    })
}

#[tauri::command]
pub fn delete_project(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let mut projects = state.projects.lock().map_err(|error| error.to_string())?;
    projects.retain(|project| project.id != id);
    state.save_projects(&projects)
}

#[tauri::command]
pub fn service_action(
    state: State<'_, AppState>,
    id: String,
    service: ServiceKind,
    action: String,
) -> Result<ProjectView, String> {
    let mut projects = state.projects.lock().map_err(|error| error.to_string())?;
    let project = projects
        .iter_mut()
        .find(|project| project.id == id)
        .ok_or("项目不存在")?;
    match action.as_str() {
        "start" => {
            processes::start(&state, project, service)?;
        }
        "stop" => processes::stop(project, service)?,
        "restart" => {
            processes::stop(project, service)?;
            processes::start(&state, project, service)?;
        }
        _ => return Err("未知服务操作".into()),
    }
    let view = ProjectView {
        frontend_running: processes::service_running(project, ServiceKind::Frontend),
        backend_running: processes::service_running(project, ServiceKind::Backend),
        project: project.clone(),
    };
    state.save_projects(&projects)?;
    Ok(view)
}

#[tauri::command]
pub fn read_log(
    state: State<'_, AppState>,
    id: String,
    service: ServiceKind,
) -> Result<String, String> {
    let path = state.log_path(&id, service.name());
    let bytes = fs::read(path).unwrap_or_default();
    let start = bytes.len().saturating_sub(30_000);
    Ok(String::from_utf8_lossy(&bytes[start..]).into_owned())
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<Settings, String> {
    state
        .settings
        .lock()
        .map(|settings| settings.clone())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn save_settings(state: State<'_, AppState>, settings: Settings) -> Result<(), String> {
    state.save_settings(&settings)?;
    *state.settings.lock().map_err(|error| error.to_string())? = settings;
    Ok(())
}
