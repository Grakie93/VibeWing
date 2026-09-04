use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub has_update: bool,
    pub html_url: String,
}

#[derive(Deserialize, Debug)]
struct GitHubRelease {
    tag_name: String,
    html_url: String,
}

fn parse_version(text: &str) -> Result<semver::Version, String> {
    let cleaned = text.trim().trim_start_matches('v');
    semver::Version::parse(cleaned).map_err(|e| format!("{e}"))
}

#[tauri::command]
pub async fn check_update(app: tauri::AppHandle) -> Result<UpdateInfo, String> {
    let current = app.package_info().version.to_string();

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    let response = client
        .get("https://api.github.com/repos/Grakie93/VibeWing/releases/latest")
        .header("User-Agent", "VibeWing")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(format!("GitHub API returned {status}: {body}"));
    }

    let release: GitHubRelease = response.json().await.map_err(|e| e.to_string())?;
    let latest = release.tag_name.trim().trim_start_matches('v').to_string();

    let current_sem = parse_version(&current)?;
    let latest_sem = parse_version(&latest)?;
    let has_update = latest_sem > current_sem;

    Ok(UpdateInfo {
        current_version: current,
        latest_version: latest,
        has_update,
        html_url: release.html_url,
    })
}
