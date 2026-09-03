use std::{
    fs::OpenOptions,
    io::{Read, Write},
    net::{SocketAddr, TcpStream},
    path::Path,
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use crate::{
    models::{Project, ServiceKind},
    storage::AppState,
};

/// Win32 entry points for testing whether a pid exists. Declared locally so we
/// do not have to pull in an extra dependency.
#[cfg(windows)]
mod win_process {
    pub const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
    pub const ERROR_ACCESS_DENIED: u32 = 5;
    extern "system" {
        pub fn OpenProcess(
            desired_access: u32,
            inherit_handle: i32,
            process_id: u32,
        ) -> *mut core::ffi::c_void;
        pub fn CloseHandle(object: *mut core::ffi::c_void) -> i32;
        pub fn GetLastError() -> u32;
    }
}

pub fn pid_alive(pid: Option<u32>) -> bool {
    let Some(pid) = pid else { return false };
    if pid == 0 {
        return false;
    }
    #[cfg(unix)]
    unsafe {
        libc::kill(pid as i32, 0) == 0
    }
    #[cfg(windows)]
    unsafe {
        // Ask the kernel directly instead of spawning `tasklist.exe`. The UI
        // polls this for every service every few seconds, so each spawn was a
        // console process: a black window flashing endlessly, plus needless
        // process churn.
        let handle = win_process::OpenProcess(
            win_process::PROCESS_QUERY_LIMITED_INFORMATION,
            0,
            pid,
        );
        if handle.is_null() {
            // Access denied still means the process is there (owned by another
            // user/session). Treating it as dead would make a running service
            // look stopped, so keep it alive.
            return win_process::GetLastError() == win_process::ERROR_ACCESS_DENIED;
        }
        win_process::CloseHandle(handle);
        true
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

/// Spawn a child and stream its stdout/stderr to the log file in real time.
/// Piping through Rust guarantees line-buffered flushing, which redirecting
/// directly to a file does not (most runtimes block-buffer when stdout is not a tty).
fn spawn_with_logging(
    command: &str,
    directory: &str,
    log_path: &Path,
) -> Result<Child, String> {
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
        .env("BROWSER", "none")
        // Python block-buffers stdout when it isn't a TTY (e.g. when we pipe it),
        // so print()/logging output stays stuck in the process until the buffer
        // fills or the process dies. Force line buffering so logs stream live.
        // Non-Python runtimes ignore this variable.
        .env("PYTHONUNBUFFERED", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(unix)]
    std::os::unix::process::CommandExt::process_group(&mut child_command, 0);
    #[cfg(windows)]
    std::os::windows::process::CommandExt::creation_flags(
        &mut child_command,
        // CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW
        // The latter hides the flashing cmd.exe window every time a service
        // is started or restarted on Windows.
        0x00000200 | 0x08000000,
    );
    let mut child = child_command.spawn().map_err(|error| error.to_string())?;
    forward_logs(&mut child, log_path);
    Ok(child)
}

/// Move the child's stdout/stderr into background threads that append to the log
/// file and flush after every chunk so the UI can poll live output.
fn forward_logs(child: &mut Child, log_path: &Path) {
    if let Some(stdout) = child.stdout.take() {
        spawn_forwarder(stdout, log_path.to_path_buf());
    }
    if let Some(stderr) = child.stderr.take() {
        spawn_forwarder(stderr, log_path.to_path_buf());
    }
}

fn spawn_forwarder<R: Read + Send + 'static>(mut reader: R, log_path: std::path::PathBuf) {
    thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    // Re-open for every chunk: the file may have been truncated or
                    // removed by "clear logs" while the service is still running, and a
                    // long-lived handle would keep writing to a stale offset/inode.
                    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&log_path) {
                        let _ = f.write_all(&buf[..n]);
                        let _ = f.flush();
                    }
                }
                Err(_) => break,
            }
        }
    });
}

pub fn start(state: &AppState, project: &mut Project, service: ServiceKind) -> Result<u32, String> {
    if service_running(project, service) {
        // Already tracked and alive: nothing to do.
        if let Some(pid) = service.pid(project) {
            return Ok(pid);
        }
        // The port is bound but we have no pid for it (leftover from a previous
        // run, or started outside VibeWing). Adopt it so stop/restart keep
        // working, and tell the user exactly what is holding the port instead of
        // failing with a vague "already listening".
        let port = service.port(project).to_string();
        return match pid_on_port(&port) {
            Some(pid) => {
                service.set_pid(project, Some(pid));
                Err(format!(
                    "端口 {port} 已被进程 {pid} 占用（上次遗留的服务）。已接管，请点「停止」后再启动。"
                ))
            }
            None => Err(format!(
                "端口 {port} 已被占用，但找不到占用进程，无法启动。"
            )),
        };
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
    let child = spawn_with_logging(command, directory, &log_path)?;
    let pid = child.id();
    // Detach: drop the child handle without waiting so the process keeps running.
    drop(child);
    service.set_pid(project, Some(pid));
    Ok(pid)
}

/// Terminate a pid, trying its whole process group first so wrapper shells and
/// their children (npm -> node, shell -> python) all go down together.
fn kill_pid(pid: u32) -> bool {
    #[cfg(unix)]
    unsafe {
        libc::kill(-(pid as i32), libc::SIGTERM) == 0 || libc::kill(pid as i32, libc::SIGTERM) == 0
    }
    #[cfg(windows)]
    Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/T"])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

/// Unconditional kill (SIGKILL / taskkill /F). Only used after a graceful
/// SIGTERM was given a chance: some runtimes ignore SIGTERM or hang while
/// flushing, and without this fallback "restart" keeps failing because the
/// outgoing instance never actually goes away.
fn kill_pid_force(pid: u32) -> bool {
    #[cfg(unix)]
    unsafe {
        libc::kill(-(pid as i32), libc::SIGKILL) == 0 || libc::kill(pid as i32, libc::SIGKILL) == 0
    }
    #[cfg(windows)]
    Command::new("taskkill")
        .args(["/F", "/PID", &pid.to_string(), "/T"])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

/// Poll until every listed pid is gone.
/// A closed port does NOT mean the process is gone: servers stop listening
/// first and then spend time flushing logs and reaping children. Starting the
/// replacement during that window is exactly what makes a restart collide with
/// the outgoing instance and report "already running".
fn wait_for_exit(pids: &[u32], timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        if pids.iter().all(|pid| !pid_alive(Some(*pid))) {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// Poll until the port stops accepting connections.
fn wait_for_port_closed(port: &str, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        if !port_open(port) {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// On Windows, helper tools (tasklist, netstat) are console applications.
/// Without CREATE_NO_WINDOW, every alive/port lookup flashes a black window
/// on top of the GUI. This wrapper hides them.
#[cfg(windows)]
fn silent_command(program: &str) -> Command {
    use std::os::windows::process::CommandExt;
    let mut cmd = Command::new(program);
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    cmd
}

/// Find the pid currently listening on a TCP port.
/// Used to recover services whose recorded pid was lost (app restart / crash) or
/// that were started outside VibeWing — otherwise the app would report "running"
/// forever while being unable to stop or restart them.
pub fn pid_on_port(port: &str) -> Option<u32> {
    if port.trim().is_empty() {
        return None;
    }
    #[cfg(windows)]
    {
        let output = silent_command("netstat")
            .args(["-ano", "-p", "TCP"])
            .output()
            .ok()?;
        let text = String::from_utf8_lossy(&output.stdout);
        for line in text.lines() {
            let cols: Vec<&str> = line.split_whitespace().collect();
            if cols.len() >= 5
                && cols[1].ends_with(&format!(":{port}"))
                && cols[3].eq_ignore_ascii_case("LISTENING")
            {
                if let Ok(pid) = cols[4].parse::<u32>() {
                    if pid > 0 {
                        return Some(pid);
                    }
                }
            }
        }
        None
    }
    #[cfg(not(windows))]
    {
        let output = Command::new("lsof")
            .args(["-nP", &format!("-iTCP:{port}"), "-sTCP:LISTEN", "-t"])
            .output()
            .ok()?;
        let text = String::from_utf8_lossy(&output.stdout);
        text.lines()
            .filter_map(|line| line.trim().parse::<u32>().ok())
            .find(|pid| *pid > 0)
    }
}

pub fn stop(project: &mut Project, service: ServiceKind) -> Result<(), String> {
    let port = service.port(project).to_string();
    let mut targets: Vec<u32> = Vec::new();

    // Prefer the pid we recorded, but also resolve whatever is holding the port.
    // The recorded pid goes stale whenever the app restarts or crashes while a
    // service is running, and killing only by that pid left orphaned processes
    // squatting on the port forever (stop became a no-op, start always refused).
    if let Some(pid) = service.pid(project) {
        if pid_alive(Some(pid)) {
            targets.push(pid);
        }
    }
    if let Some(pid) = pid_on_port(&port) {
        if !targets.contains(&pid) {
            targets.push(pid);
        }
    }

    // 1) Ask nicely first so the service can flush logs and exit cleanly.
    for pid in &targets {
        kill_pid(*pid);
    }

    // 2) Wait for the processes themselves to disappear. A closed listener only
    //    means shutdown has started — the process and its children may still be
    //    running, and starting the replacement now is what made a restart
    //    collide with the outgoing instance.
    wait_for_exit(&targets, Duration::from_millis(3000));

    // 3) Whatever survived the graceful signal is force-killed.
    let stubborn: Vec<u32> = targets
        .iter()
        .copied()
        .filter(|pid| pid_alive(Some(*pid)))
        .collect();
    for pid in &stubborn {
        kill_pid_force(*pid);
    }
    if !stubborn.is_empty() {
        wait_for_exit(&stubborn, Duration::from_millis(2000));
    }

    service.set_pid(project, None);

    // 4) Finally wait for the listener to actually go away.
    if !wait_for_port_closed(&port, Duration::from_millis(3000)) {
        // Still bound: something outside our target list is holding it, e.g. a
        // --reload grandchild that left the process group. Clear it too.
        if let Some(pid) = pid_on_port(&port) {
            kill_pid_force(pid);
            if wait_for_port_closed(&port, Duration::from_millis(2000)) {
                return Ok(());
            }
            return Err(format!(
                "端口 {port} 仍被进程 {pid} 占用，无法停止。请手动结束该进程后重试。"
            ));
        }
        return Err(format!("端口 {port} 仍在监听，无法停止。"));
    }

    Ok(())
}

pub fn stop_all(projects: &mut [Project]) {
    for project in projects {
        let _ = stop(project, ServiceKind::Frontend);
        let _ = stop(project, ServiceKind::Backend);
    }
}

fn infer_build_command(run_cmd: &str, test: bool) -> Option<String> {
    let cmd = run_cmd.trim().to_lowercase();
    if cmd.is_empty() || cmd.contains("python") || cmd.contains(".venv") {
        return None;
    }
    if cmd.starts_with("npm ") {
        return Some(if test { "npm run build:test" } else { "npm run build" }.into());
    }
    if cmd.starts_with("pnpm ") {
        return Some(if test { "pnpm build:test" } else { "pnpm build" }.into());
    }
    if cmd.starts_with("yarn ") {
        return Some(if test { "yarn build:test" } else { "yarn build" }.into());
    }
    if cmd.starts_with("cargo ") {
        return Some(if test { "cargo test" } else { "cargo build" }.into());
    }
    if cmd.starts_with("go ") {
        return Some(if test { "go test ./..." } else { "go build ." }.into());
    }
    if cmd.starts_with("mvn ") {
        return Some(if test { "mvn test" } else { "mvn package" }.into());
    }
    None
}

/// Run a one-shot build command and stream its output to the service log.
/// Builds are not tracked as long-running processes, so the pid is not stored.
pub fn build(state: &AppState, project: &Project, service: ServiceKind, test: bool) -> Result<(), String> {
    let stored = service.build_command(project, test).trim();
    let command = if stored.is_empty() {
        infer_build_command(service.command(project), test)
            .ok_or_else(|| if test { "未配置测试构建命令" } else { "未配置生产构建命令" })?
    } else {
        stored.to_string()
    };
    let directory = service.directory(project).trim();
    if !Path::new(directory).is_dir() {
        return Err(format!("工作目录不存在：{directory}"));
    }
    let log_path = state.log_path(&project.id, service.name());
    let child = spawn_with_logging(command.as_str(), directory, &log_path)?;
    drop(child);
    Ok(())
}
