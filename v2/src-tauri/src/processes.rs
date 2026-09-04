use std::{
    collections::HashMap,
    fs::OpenOptions,
    net::{SocketAddr, TcpStream},
    path::Path,
    process::{Child, Command, Stdio},
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

/// True when `pid` is (very likely) a process VibeWing spawned itself.
///
/// Every service is started with `process_group(0)`, so the child becomes the
/// leader of a brand-new process group whose id equals its own pid. A pid the
/// operating system recycled for an unrelated process is almost never a group
/// leader, which makes this a cheap guard against PID reuse: `kill(pid, 0)`
/// alone happily reports "alive" for whatever inherited the number while the app
/// was closed, so a service that died in the meantime would stay green forever
/// — and `stop()` would signal an innocent process (it kills by process group).
/// `getpgid` is a plain syscall, so this stays cheap enough for the 10s poll,
/// unlike shelling out to `ps`/`lsof` for every service.
pub fn spawned_by_us(pid: Option<u32>) -> bool {
    let Some(pid) = pid else { return false };
    if pid == 0 {
        return false;
    }
    #[cfg(unix)]
    unsafe {
        libc::getpgid(pid as i32) == pid as i32
    }
    #[cfg(windows)]
    {
        // Windows has no process groups here, so the pid is all we can compare.
        pid_alive(Some(pid))
    }
}

/// Whether the service is actually answering on its port.
///
/// An open port is the only signal a dead process cannot fake: a pid alone stays
/// "alive" once the OS recycles it, and a leftover child can keep a port bound
/// long after the process VibeWing started is gone. That is why a service counts
/// as running only while its port accepts connections — this is what makes
/// "the backend died while the app was closed" show up as stopped the moment the
/// app is opened again. The window where our process is up but has not bound its
/// port yet is the yellow "starting" light (`is_starting`), not a green one.
pub fn service_running(project: &Project, service: ServiceKind) -> bool {
    let port = match service {
        ServiceKind::Frontend => &project.frontend_port,
        ServiceKind::Backend => &project.backend_port,
    };
    if port_open(port) {
        return true;
    }
    // No port to observe (worker-style service): the process is all we have, and
    // only if it is really the one we started.
    if port.trim().is_empty() {
        return spawned_by_us(service.pid(project));
    }
    false
}

/// Spawn a child and stream its stdout/stderr straight to the log file.
///
/// Output is redirected to the log file (opened with O_APPEND) instead of a pipe
/// to the parent process. This is what lets the served project outlive VibeWing:
/// if stdout/stderr were piped to the app, closing VibeWing would close the pipe
/// write ends and the child would die on SIGPIPE the next time it logged. Writing
/// to a file keeps the child fully detached, so quitting the app leaves running
/// services running. The UI already reads this same log file for live output.
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
    // O_APPEND keeps "clear logs" working: after a truncate the next write still
    // lands at the (new) end of file, and the child never points at a stale inode.
    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .map_err(|error| format!("无法打开日志文件 {log_path:?}: {error}"))?;
    child_command
        .current_dir(directory)
        .env("BROWSER", "none")
        // Python block-buffers stdout when it isn't a tty (e.g. when we pipe it),
        // so print()/logging output stays stuck in the process until the buffer
        // fills or the process dies. Force line buffering so logs stream live.
        // Non-Python runtimes ignore this variable.
        .env("PYTHONUNBUFFERED", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::from(log_file.try_clone().map_err(|e| e.to_string())?))
        .stderr(Stdio::from(log_file));
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
    let child = child_command.spawn().map_err(|error| error.to_string())?;
    Ok(child)
}


pub fn start(state: &AppState, project: &mut Project, service: ServiceKind) -> Result<u32, String> {
    // 1) The process we launched is still alive. It may not have bound its port
    //    yet (the yellow "starting" light), but it is up, and starting again
    //    would spawn a second instance that dies with "address already in use"
    //    and overwrites the recorded pid with a dead one.
    if let Some(pid) = service.pid(project) {
        if spawned_by_us(Some(pid)) {
            return Ok(pid);
        }
    }
    // 2) Something is listening that is not ours: a leftover from a previous
    //    run, or a service started outside VibeWing. Adopt it so stop/restart
    //    keep working, and tell the user exactly what is holding the port
    //    instead of failing with a vague "already listening".
    let port = service.port(project).to_string();
    if port_open(&port) {
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
    // 3) Nothing of ours is running and the port is free: launch it.
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
    silent_command("taskkill")
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
    silent_command("taskkill")
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

/// `Command::new`, except that on Windows the child is created with
/// CREATE_NO_WINDOW.
///
/// Every helper this app shells out to is a console application there —
/// taskkill, netstat, cmd.exe and git.exe — so without that flag each port
/// lookup, service stop, git status or "open in folder" popped a black console
/// window over the GUI. Route *every* helper through this instead of
/// `Command::new`; on other platforms it is exactly `Command::new`.
pub fn silent_command(program: &str) -> Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let mut cmd = Command::new(program);
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        cmd
    }
    #[cfg(not(windows))]
    Command::new(program)
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
        let output = silent_command("lsof")
            .args(["-nP", &format!("-iTCP:{port}"), "-sTCP:LISTEN", "-t"])
            .output()
            .ok()?;
        let text = String::from_utf8_lossy(&output.stdout);
        text.lines()
            .filter_map(|line| line.trim().parse::<u32>().ok())
            .find(|pid| *pid > 0)
    }
}

/// One entry of the process tree a service is actually running as.
#[derive(Clone, Debug, serde::Serialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub memory_mb: f64,
    /// True for the wrapper shell VibeWing spawned. Every other entry was
    /// started by the project's own toolchain, not by VibeWing.
    pub ours: bool,
}

/// The process tree rooted at `root`, flattened breadth-first.
///
/// Answers the question customers keep asking when they open Task Manager:
/// "why does starting one service show up as two Node / two Python processes?"
/// VibeWing spawns exactly one wrapper shell per service — the entry marked
/// `ours` — and everything below it is the project's own tooling: npm -> node ->
/// vite, the esbuild helper a dev server forks, or uvicorn's `--reload`
/// parent/worker pair. Showing this tree is what makes that visible without
/// having to guess from a flat process list.
pub fn process_tree(root: Option<u32>) -> Vec<ProcessInfo> {
    use std::collections::VecDeque;

    let Some(root) = root else {
        return Vec::new();
    };
    let mut system = sysinfo::System::new_all();
    system.refresh_processes();

    // parent -> children, so the walk below never rescans the process table.
    let mut children: HashMap<u32, Vec<u32>> = HashMap::new();
    for (pid, process) in system.processes() {
        if let Some(parent) = process.parent() {
            children.entry(parent.as_u32()).or_default().push(pid.as_u32());
        }
    }

    let mut out = Vec::new();
    let mut queue: VecDeque<u32> = VecDeque::from(vec![root]);
    while let Some(pid) = queue.pop_front() {
        let Some(process) = system.process(sysinfo::Pid::from_u32(pid)) else {
            continue;
        };
        out.push(ProcessInfo {
            pid,
            name: process.name().to_string(),
            memory_mb: process.memory() as f64 / 1024.0 / 1024.0,
            ours: pid == root,
        });
        if let Some(kids) = children.get_mut(&pid) {
            kids.sort_unstable();
            queue.extend(kids.iter().copied());
        }
    }
    out
}

pub fn stop(project: &mut Project, service: ServiceKind) -> Result<(), String> {
    let port = service.port(project).to_string();
    let mut targets: Vec<u32> = Vec::new();

    // Prefer the pid we recorded, but also resolve whatever is holding the port.
    // The recorded pid goes stale whenever the app restarts or crashes while a
    // service is running, and killing only by that pid left orphaned processes
    // squatting on the port forever (stop became a no-op, start always refused).
    // Only signal the pid we recorded when we are sure it is the process we
    // launched. `kill_pid` signals the whole process group, so a pid the OS
    // recycled in the meantime would take down whatever unrelated process
    // inherited the number (a shell, an editor). Services we spawned are always
    // group leaders — that is what `spawned_by_us` checks. Anything we merely
    // adopted is reached through the port right below, and a service without a
    // port can only ever be stopped by pid.
    let portless = service.port(project).trim().is_empty();
    if let Some(pid) = service.pid(project) {
        if spawned_by_us(Some(pid)) || (portless && pid_alive(Some(pid))) {
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
