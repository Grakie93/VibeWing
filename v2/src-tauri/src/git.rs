use crate::models::Project;
use std::process::Command;

#[derive(Clone, Debug, serde::Serialize)]
pub struct GitFile {
    pub path: String,
    pub status: String,
    pub staged: bool,
    pub unstaged: bool,
}

fn run(cwd: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
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
pub fn files(cwd: &str) -> Result<Vec<GitFile>, String> {
    Ok(run(cwd, &["status", "--porcelain=v1"])?
        .lines()
        .filter_map(|line| {
            if line.len() < 3 {
                return None;
            }
            let status = line[..2].to_string();
            Some(GitFile {
                staged: status.as_bytes()[0] != b' ',
                unstaged: status.as_bytes()[1] != b' ',
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
pub fn commit(cwd: &str, message: &str) -> Result<String, String> {
    run(cwd, &["commit", "-m", message])?;
    run(cwd, &["rev-parse", "HEAD"]).map(|s| s.trim().to_string())
}
pub fn push(cwd: &str) -> Result<(), String> {
    run(cwd, &["push"]).map(|_| ())
}
