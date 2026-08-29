use std::{
    fs::OpenOptions,
    net::{SocketAddr, TcpStream},
    path::Path,
    process::{Command, Stdio},
    time::Duration,
};

use crate::{
    models::{Project, ServiceKind},
    storage::AppState,
};

pub fn pid_alive(pid: Option<u32>) -> bool {
    let Some(pid) = pid else { return false };
    #[cfg(unix)]
    unsafe {
        libc::kill(pid as i32, 0) == 0
    }
    #[cfg(windows)]
    {
        Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/NH"])
            .output()
            .map(|output| String::from_utf8_lossy(&output.stdout).contains(&pid.to_string()))
            .unwrap_or(false)
    }
}

pub fn port_open(port: &str) -> bool {
    let Ok(port) = port.parse::<u16>() else {
        return false;
    };
    TcpStream::connect_timeout(
        &SocketAddr::from(([127, 0, 0, 1], port)),
        Duration::from_millis(180),
    )
    .is_ok()
}

pub fn service_running(project: &Project, service: ServiceKind) -> bool {
    let port = match service {
        ServiceKind::Frontend => &project.frontend_port,
        ServiceKind::Backend => &project.backend_port,
    };
    pid_alive(service.pid(project)) || port_open(port)
}

pub fn start(state: &AppState, project: &mut Project, service: ServiceKind) -> Result<u32, String> {
    if service_running(project, service) {
        return service
            .pid(project)
            .ok_or_else(|| "服务端口已经在监听".into());
    }
    let command = service.command(project).trim();
    let directory = service.directory(project).trim();
    if command.is_empty() {
        return Err("未配置启动命令".into());
    }
    if !Path::new(directory).is_dir() {
        return Err(format!("工作目录不存在：{directory}"));
    }

    let log_path = state.log_path(&project.id, service.name());
    let log = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .map_err(|error| error.to_string())?;
    let error_log = log.try_clone().map_err(|error| error.to_string())?;
    #[cfg(windows)]
    let mut child_command = {
        let mut value = Command::new("cmd.exe");
        value.args(["/d", "/s", "/c", command]);
        value
    };
    #[cfg(not(windows))]
    let mut child_command = {
        let mut value = Command::new(std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into()));
        value.args(["-l", "-c", command]);
        value
    };
    child_command
        .current_dir(directory)
        .stdin(Stdio::null())
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(error_log));
    #[cfg(unix)]
    std::os::unix::process::CommandExt::process_group(&mut child_command, 0);
    #[cfg(windows)]
    std::os::windows::process::CommandExt::creation_flags(&mut child_command, 0x00000200);
    let child = child_command.spawn().map_err(|error| error.to_string())?;
    let pid = child.id();
    service.set_pid(project, Some(pid));
    Ok(pid)
}

pub fn stop(project: &mut Project, service: ServiceKind) -> Result<(), String> {
    let Some(pid) = service.pid(project) else {
        return Ok(());
    };
    #[cfg(unix)]
    let success = unsafe {
        libc::kill(-(pid as i32), libc::SIGTERM) == 0 || libc::kill(pid as i32, libc::SIGTERM) == 0
    };
    #[cfg(windows)]
    let success = Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/T"])
        .status()
        .map(|status| status.success())
        .unwrap_or(false);
    service.set_pid(project, None);
    if success || !pid_alive(Some(pid)) {
        Ok(())
    } else {
        Err("无法停止服务进程".into())
    }
}
