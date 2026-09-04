use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

use tauri::State;

use crate::{
    ai, credentials, git,
    models::{Chat, Project, ProjectView, ServiceKind, Settings},
    processes,
    storage::AppState,
};

/// True while the process is alive but its port is not yet listening, i.e. the
/// service is "loading". Without a configured port we cannot observe readiness,
/// so it falls back to the pid-only `running` state immediately.
///
/// The pid is checked with `spawned_by_us` rather than `pid_alive`: a pid the OS
/// recycled for another process is still "alive", and a service that died while
/// the app was closed would otherwise sit there pulsing a phantom "starting"
/// light forever.
fn is_starting(project: &Project, service: ServiceKind) -> bool {
    let port = service.port(project);
    if port.trim().is_empty() {
        return false;
    }
    processes::spawned_by_us(service.pid(project)) && !processes::port_open(port)
}

fn views(projects: &[Project]) -> Vec<ProjectView> {
    projects
        .iter()
        .cloned()
        .map(|project| ProjectView {
            frontend_running: processes::service_running(&project, ServiceKind::Frontend),
            backend_running: processes::service_running(&project, ServiceKind::Backend),
            frontend_starting: is_starting(&project, ServiceKind::Frontend),
            backend_starting: is_starting(&project, ServiceKind::Backend),
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
    state.persist(&projects)?;
    Ok(ProjectView {
        frontend_running: processes::service_running(&project, ServiceKind::Frontend),
        backend_running: processes::service_running(&project, ServiceKind::Backend),
        frontend_starting: is_starting(&project, ServiceKind::Frontend),
        backend_starting: is_starting(&project, ServiceKind::Backend),
        project,
    })
}

#[tauri::command]
pub fn delete_project(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let mut projects = state.projects.lock().map_err(|error| error.to_string())?;
    let before = projects.len();
    projects.retain(|project| project.id != id);
    if projects.len() == before {
        return Err("项目不存在或已经被移除".into());
    }
    // A file-sourced project is removed from disk too; deleting the file is what
    // actually un-imports it on the next rescan.
    if let Some(stem) = id.strip_prefix("file:") {
        let _ = fs::remove_file(state.projects_dir.join(format!("{stem}.json")));
    }
    state.persist(&projects)
}

#[tauri::command]
pub fn open_url(url: String) -> Result<(), String> {
    let parsed = url::Url::parse(&url).map_err(|e| e.to_string())?;
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err("仅支持打开 HTTP/HTTPS 地址".into());
    }
    #[cfg(target_os = "macos")]
    let status = processes::silent_command("open").arg(&url).status();
    #[cfg(target_os = "windows")]
    let status = processes::silent_command("cmd").args(["/C", "start", "", &url]).status();
    #[cfg(all(unix, not(target_os = "macos")))]
    let status = processes::silent_command("xdg-open").arg(&url).status();
    status.map_err(|e| e.to_string()).and_then(|s| if s.success() { Ok(()) } else { Err(format!("无法打开地址：{url}")) })
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
        frontend_starting: is_starting(project, ServiceKind::Frontend),
        backend_starting: is_starting(project, ServiceKind::Backend),
        project: project.clone(),
    };
    state.persist(&projects)?;
    Ok(view)
}

/// The process tree a service is running as, flattened breadth-first.
/// Answers "why does this service show up as N processes in Task Manager?"
/// without making the customer or the support agent open the OS task list.
#[tauri::command]
pub fn service_processes(
    state: State<'_, AppState>,
    id: String,
    service: ServiceKind,
) -> Result<Vec<processes::ProcessInfo>, String> {
    let projects = state.projects.lock().map_err(|e| e.to_string())?;
    let project = projects
        .iter()
        .find(|project| project.id == id)
        .ok_or("项目不存在")?;
    Ok(processes::process_tree(service.pid(project)))
}

#[tauri::command]
pub fn build_project(
    state: State<'_, AppState>,
    id: String,
    service: ServiceKind,
    test: bool,
) -> Result<ProjectView, String> {
    let mut projects = state.projects.lock().map_err(|error| error.to_string())?;
    let project = projects
        .iter_mut()
        .find(|project| project.id == id)
        .ok_or("项目不存在")?;
    processes::build(&state, project, service, test)?;
    let view = ProjectView {
        frontend_running: processes::service_running(project, ServiceKind::Frontend),
        backend_running: processes::service_running(project, ServiceKind::Backend),
        frontend_starting: is_starting(project, ServiceKind::Frontend),
        backend_starting: is_starting(project, ServiceKind::Backend),
        project: project.clone(),
    };
    Ok(view)
}

#[tauri::command]
pub fn rescan_projects(state: State<'_, AppState>) -> Result<Vec<ProjectView>, String> {
    state.merge_discovered()?;
    let projects = state.projects.lock().map_err(|error| error.to_string())?;
    Ok(views(&projects))
}

#[tauri::command]
pub fn get_projects_dir(state: State<'_, AppState>) -> String {
    state.projects_dir.to_string_lossy().into_owned()
}

#[tauri::command]
pub fn open_path(path: String) -> Result<(), String> {
    let target = path.trim();
    if target.is_empty() {
        return Err("路径为空".into());
    }
    #[cfg(target_os = "macos")]
    let status = processes::silent_command("open").arg(target).status();
    #[cfg(target_os = "windows")]
    let status = processes::silent_command("cmd").args(["/C", "start", "", target]).status();
    #[cfg(all(unix, not(target_os = "macos")))]
    let status = processes::silent_command("xdg-open").arg(target).status();
    status
        .map_err(|error| error.to_string())
        .and_then(|status| {
            if status.success() {
                Ok(())
            } else {
                Err(format!("无法打开路径：{target}"))
            }
        })
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
pub fn clear_log(
    state: State<'_, AppState>,
    id: String,
    service: ServiceKind,
) -> Result<(), String> {
    let path = state.log_path(&id, service.name());
    fs::write(path, "").map_err(|error| error.to_string())
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

#[tauri::command]
pub fn list_chats(state: State<'_, AppState>) -> Result<Vec<Chat>, String> {
    state.chats.lock().map(|v| v.clone()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_chats(state: State<'_, AppState>, chats: Vec<Chat>) -> Result<(), String> {
    state.save_chats(&chats)?;
    *state.chats.lock().map_err(|e| e.to_string())? = chats;
    Ok(())
}

#[tauri::command]
pub fn git_status(
    state: State<'_, AppState>,
    id: String,
    scope: String,
) -> Result<Vec<git::GitFile>, String> {
    let projects = state.projects.lock().map_err(|e| e.to_string())?;
    let project = projects.iter().find(|p| p.id == id).ok_or("项目不存在")?;
    git::root(project, &scope).and_then(|cwd| git::files(&cwd))
}
#[tauri::command]
pub fn git_stage(
    state: State<'_, AppState>,
    id: String,
    scope: String,
    paths: Vec<String>,
) -> Result<(), String> {
    let projects = state.projects.lock().map_err(|e| e.to_string())?;
    let project = projects.iter().find(|p| p.id == id).ok_or("项目不存在")?;
    git::root(project, &scope).and_then(|cwd| git::stage(&cwd, &paths))
}
#[tauri::command]
pub fn git_commit(
    state: State<'_, AppState>,
    id: String,
    scope: String,
    message: String,
) -> Result<String, String> {
    if message.trim().is_empty() {
        return Err("提交信息不能为空".into());
    }
    let projects = state.projects.lock().map_err(|e| e.to_string())?;
    let project = projects.iter().find(|p| p.id == id).ok_or("项目不存在")?;
    git::root(project, &scope).and_then(|cwd| git::commit(&cwd, message.trim()))
}
#[tauri::command]
pub fn git_push(state: State<'_, AppState>, id: String, scope: String) -> Result<(), String> {
    let projects = state.projects.lock().map_err(|e| e.to_string())?;
    let project = projects.iter().find(|p| p.id == id).ok_or("项目不存在")?;
    git::root(project, &scope).and_then(|cwd| git::push(&cwd))
}
#[tauri::command]
pub fn git_branches(state: State<'_, AppState>, id: String, scope: String) -> Result<Vec<String>, String> {
    let projects = state.projects.lock().map_err(|e| e.to_string())?;
    let project = projects.iter().find(|p| p.id == id).ok_or("项目不存在")?;
    git::root(project, &scope).and_then(|cwd| git::branches(&cwd))
}
#[tauri::command]
pub fn git_current_branch(state: State<'_, AppState>, id: String, scope: String) -> Result<String, String> {
    let projects = state.projects.lock().map_err(|e| e.to_string())?;
    let project = projects.iter().find(|p| p.id == id).ok_or("项目不存在")?;
    git::root(project, &scope).and_then(|cwd| git::current_branch(&cwd))
}
#[tauri::command]
pub fn git_switch_branch(state: State<'_, AppState>, id: String, scope: String, branch: String) -> Result<(), String> {
    let projects = state.projects.lock().map_err(|e| e.to_string())?;
    let project = projects.iter().find(|p| p.id == id).ok_or("项目不存在")?;
    git::root(project, &scope).and_then(|cwd| git::switch_branch(&cwd, &branch))
}
#[tauri::command]
pub fn git_pull(state: State<'_, AppState>, id: String, scope: String) -> Result<(), String> {
    let projects = state.projects.lock().map_err(|e| e.to_string())?;
    let project = projects.iter().find(|p| p.id == id).ok_or("项目不存在")?;
    git::root(project, &scope).and_then(|cwd| git::pull(&cwd))
}

#[tauri::command]
pub fn provider_key_status(state: State<'_, AppState>, provider_id: String) -> Result<bool, String> {
    // Do not read the OS keychain while rendering Settings: macOS can show an
    // access prompt for every read. The marker is persisted with the provider;
    // the keychain is accessed only when saving a key or sending a request.
    let settings = state.settings.lock().map_err(|e| e.to_string())?;
    Ok(settings.providers.iter().find(|p| p.id == provider_id).map(|p| p.key_configured).unwrap_or(false))
}

#[tauri::command]
pub fn save_provider_key(state: State<'_, AppState>, provider_id: String, key: String) -> Result<(), String> {
    credentials::set(&state.data_dir, &provider_id, &key)?;
    let mut settings = state.settings.lock().map_err(|e| e.to_string())?;
    if let Some(provider) = settings.providers.iter_mut().find(|p| p.id == provider_id) {
        provider.key_configured = !key.is_empty();
    }
    state.save_settings(&settings)
}

#[tauri::command]
pub async fn ask_ai(
    state: State<'_, AppState>,
    request: ai::ChatRequest,
) -> Result<ai::ChatResponse, String> {
    let settings = state.settings.lock().map_err(|e| e.to_string())?.clone();
    let provider = settings
        .providers
        .iter()
        .find(|provider| {
            provider.id == request.provider_id
        })
        .ok_or("模型平台不存在")?;
    let base = provider
        .base_url.as_str();
    if base.is_empty() { return Err("模型平台缺少 API 地址".into()); }
    let key = credentials::get(&state.data_dir, &request.provider_id)
        .map_err(|_| "请先配置该平台的 API Key".to_string())?;
    if key.is_empty() {
        return Err("请先配置该平台的 API Key".into());
    }
    let mut system_prompt = if settings.language == "zh-CN" {
        "你是VibeWing内置的AI，VibeWing是用于给广大Coding用户做项目管理的应用，开发者是Grakie。默认使用中文回答，用户有特殊语言要求则采用用户要求。先判断用户是在什么项目组进行提问、诊断或讨论。执行目标或环境不完整时，追问最必要的问题，不猜测参数。普通回答要准确、简洁、具体；分析日志时优先结合附加的项目上下文给出明确结论，信息不足时明确说缺少什么，避免罗列大量无关可能性。使用清晰的Markdown标题、列表和代码块。".to_string()
    } else {
        "You are the AI built into VibeWing, an app for Coding users to manage their projects, developed by Grakie. Reply in English by default; if the user requests a specific language, follow that request. Do not answer topics unrelated to VibeWing and projects. Determine which project group the user is asking about before answering. If the target or environment is incomplete, ask the most necessary clarifying question instead of guessing. Keep normal answers accurate, concise, and specific. When analyzing logs, prioritize using the attached project context to reach a clear conclusion; explicitly state what information is missing when it is insufficient, and avoid listing many unrelated possibilities. Use clear Markdown headings, lists, and code blocks.".to_string()
    };
    if !settings.memory.is_empty() {
        system_prompt.push_str(if settings.language == "zh-CN" {
            "\n\n以下是你已经记住的用户长期偏好与约束，必须遵守：\n"
        } else {
            "\n\nThe following are long-term user preferences and constraints you must remember and follow:\n"
        });
        system_prompt.push_str(&settings.memory);
    }

    let mut request = request;
    request.messages.insert(
        0,
        ai::ChatMessage {
            role: "system".into(),
            content: system_prompt.into(),
        },
    );
    ai::complete(request, base, &key).await
}
