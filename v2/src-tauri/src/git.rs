use crate::{models::Project, processes};
use std::io::Read;
use std::process::Stdio;
use std::thread;
use std::time::{Duration, Instant};

#[derive(Clone, Debug, serde::Serialize)]
pub struct GitFile {
    pub path: String,
    pub status: String,
    pub staged: bool,
    pub unstaged: bool,
}

/// How long a single git invocation may run before we give up. Pushes can hit
/// the network, but the only reason one should run this long is a credential
/// prompt that can never be answered (Windows-only symptom) or a dead network.
const GIT_TIMEOUT: Duration = Duration::from_secs(120);

fn run(cwd: &str, args: &[&str]) -> Result<String, String> {
    // git.exe is a console application on Windows: without CREATE_NO_WINDOW
    // every status poll, commit and push would flash a black window.
    // `-c core.quotepath=false` makes git print non-ASCII paths (e.g. Chinese
    // file/branch names) as raw UTF-8 instead of C-style octal escapes like
    // "\347\233\256". This is per-invocation only, so we don't touch the
    // user's own git config.
    // `GIT_TERMINAL_PROMPT=0` is the fix for the Windows "push silently does
    // nothing / app freezes" report: when the Git Credential Manager cannot
    // surface its dialog inside this headless console process, git used to
    // fall back to an interactive username/password prompt on a piped stdin,
    // which either hangs the whole (synchronous) Tauri command forever or fails
    // with an empty stderr. Disabling the terminal prompt makes git fail fast
    // so the UI can show a clear error instead of appearing to do nothing.
    let mut full_args: Vec<&str> = vec!["-c", "core.quotepath=false"];
    full_args.extend_from_slice(args);

    let mut command = processes::silent_command("git");
    command
        .args(full_args)
        .current_dir(cwd)
        .env("GIT_TERMINAL_PROMPT", "0")
        // Inject the user's full login-shell PATH so git hooks that call
        // pnpm/yarn/npm (e.g. husky pre-commit) can find them even when
        // VibeWing is launched as a GUI app with a minimal PATH.
        .env("PATH", processes::git_path())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = command.spawn().map_err(|e| e.to_string())?;

    // Drain stdout/stderr on background threads. A chatty git (or one whose
    // child credential helper inherits the pipe) would otherwise block on a
    // full pipe while we wait for it to exit, deadlocking the wait below.
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let out_thread = thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(mut s) = stdout {
            let _ = s.read_to_end(&mut buf);
        }
        buf
    });
    let err_thread = thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(mut s) = stderr {
            let _ = s.read_to_end(&mut buf);
        }
        buf
    });

    let deadline = Instant::now() + GIT_TIMEOUT;
    let status = loop {
        match child.try_wait().map_err(|e| e.to_string())? {
            Some(status) => break status,
            None => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    return Err(
                        "git 命令超时：可能正在等待凭据输入或网络不通。请在系统凭据管理器（如 Windows 凭据管理器中的 GitHub/GitLab 条目）中确认 Git 凭据有效后重试。"
                            .into(),
                    );
                }
                thread::sleep(Duration::from_millis(150));
            }
        }
    };

    let out = out_thread.join().unwrap_or_default();
    let err = err_thread.join().unwrap_or_default();

    if !status.success() {
        let stderr_str = String::from_utf8_lossy(&err).trim().to_string();
        let stdout_str = String::from_utf8_lossy(&out).trim().to_string();
        // Never return an empty error: an empty message is what made the push
        // look like it "succeeded with no feedback" on Windows. Prefer stderr,
        // fall back to stdout, then to a generic message carrying the exit code.
        let message = if !stderr_str.is_empty() {
            stderr_str
        } else if !stdout_str.is_empty() {
            stdout_str
        } else {
            format!(
                "git {} 执行失败（退出码 {}）",
                args.join(" "),
                status.code().unwrap_or(-1)
            )
        };
        return Err(message);
    }
    Ok(String::from_utf8_lossy(&out).into_owned())
}
pub fn root(project: &Project, scope: &str) -> Result<String, String> {
    let dir = if scope == "backend" {
        &project.backend_path
    } else {
        &project.frontend_path
    };
    run(
        if dir.is_empty() { &project.path } else { dir },
        &["rev-parse", "--show-toplevel"],
    )
    .map(|s| s.trim().to_string())
}
pub fn files(cwd: &str, pathspec: Option<&str>) -> Result<Vec<GitFile>, String> {
    let mut args: Vec<&str> = vec!["status", "--porcelain=v1"];
    if let Some(spec) = pathspec {
        args.push("--");
        args.push(spec);
    }
    Ok(run(cwd, &args)?
        .lines()
        .filter_map(|line| {
            if line.len() < 3 {
                return None;
            }
            let status = line[..2].to_string();
            let bytes = status.as_bytes();
            // '?' means untracked, not a real index/worktree status, so it must
            // not be counted as staged or unstaged.
            let is_change = |b: u8| b != b' ' && b != b'?';
            Some(GitFile {
                staged: is_change(bytes[0]),
                unstaged: is_change(bytes[1]),
                status,
                path: line[3..].trim().to_string(),
            })
        })
        .collect())
}
pub fn stage(cwd: &str, paths: &[String]) -> Result<(), String> {
    let mut args = vec!["add", "--"];
    args.extend(paths.iter().map(String::as_str));
    run(cwd, &args).map(|_| ())
}
pub fn unstage(cwd: &str, paths: &[String]) -> Result<(), String> {
    let mut args = vec!["reset", "HEAD", "--"];
    args.extend(paths.iter().map(String::as_str));
    run(cwd, &args).map(|_| ())
}
pub fn commit(cwd: &str, message: &str, pathspec: Option<&str>) -> Result<String, String> {
    let mut args: Vec<&str> = vec!["commit", "-m", message];
    if let Some(spec) = pathspec {
        args.push("--");
        args.push(spec);
    }
    run(cwd, &args)?;
    run(cwd, &["rev-parse", "HEAD"]).map(|s| s.trim().to_string())
}

/// Returns the textual diff for the given scope. `cached` selects the staged
/// diff (what `git commit` would record) instead of the working-tree diff.
pub fn diff(cwd: &str, cached: bool, pathspec: Option<&str>) -> Result<String, String> {
    let mut args: Vec<&str> = vec!["diff", "--no-color"];
    if cached {
        args.push("--cached");
    }
    if let Some(spec) = pathspec {
        args.push("--");
        args.push(spec);
    }
    run(cwd, &args)
}
pub fn push(cwd: &str) -> Result<String, String> {
    run(cwd, &["push"])
}
pub fn branches(cwd: &str) -> Result<Vec<String>, String> {
    Ok(run(cwd, &["branch", "--format=%(refname:short)"])?.lines().map(str::trim).filter(|s| !s.is_empty()).map(String::from).collect())
}
pub fn current_branch(cwd: &str) -> Result<String, String> { run(cwd, &["branch", "--show-current"]).map(|s| s.trim().to_string()) }
pub fn switch_branch(cwd: &str, branch: &str) -> Result<(), String> {
    if branch.trim().is_empty() || branch.contains([' ', ';', '&', '|']) { return Err("无效分支名称".into()); }
    run(cwd, &["switch", branch.trim()]).map(|_| ())
}
pub fn pull(cwd: &str) -> Result<(), String> { run(cwd, &["pull", "--ff-only"]).map(|_| ()) }
