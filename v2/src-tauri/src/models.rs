use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub path: String,
    pub frontend_path: String,
    pub backend_path: String,
    pub frontend_cmd: String,
    pub backend_cmd: String,
    pub frontend_build: String,
    pub frontend_test_build: String,
    pub backend_build: String,
    pub backend_test_build: String,
    pub frontend_port: String,
    pub backend_port: String,
    pub frontend_pid: Option<u32>,
    pub backend_pid: Option<u32>,
    /// Where this project came from. `"file"` means it was discovered in the
    /// `projects` data directory (written by a coding agent or the user) and is
    /// persisted back to its own JSON file; any other value is a UI-authored
    /// project stored in `projects.json`. Empty defaults to UI-authored.
    #[serde(default)]
    pub source: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProjectView {
    #[serde(flatten)]
    pub project: Project,
    pub frontend_running: bool,
    pub backend_running: bool,
    /// True while the process is alive but its port is not yet listening — the
    /// "loading" state the UI shows as a yellow light.
    #[serde(default)]
    pub frontend_starting: bool,
    #[serde(default)]
    pub backend_starting: bool,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ServiceKind {
    Frontend,
    Backend,
}

impl ServiceKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Frontend => "frontend",
            Self::Backend => "backend",
        }
    }

    pub fn command<'a>(self, project: &'a Project) -> &'a str {
        match self {
            Self::Frontend => &project.frontend_cmd,
            Self::Backend => &project.backend_cmd,
        }
    }

    pub fn build_command<'a>(self, project: &'a Project, test: bool) -> &'a str {
        match self {
            Self::Frontend => {
                if test {
                    &project.frontend_test_build
                } else {
                    &project.frontend_build
                }
            }
            Self::Backend => {
                if test {
                    &project.backend_test_build
                } else {
                    &project.backend_build
                }
            }
        }
    }

    pub fn directory<'a>(self, project: &'a Project) -> &'a str {
        match self {
            Self::Frontend => &project.frontend_path,
            Self::Backend => &project.backend_path,
        }
    }

    pub fn port<'a>(self, project: &'a Project) -> &'a str {
        match self {
            Self::Frontend => &project.frontend_port,
            Self::Backend => &project.backend_port,
        }
    }

    pub fn pid(self, project: &Project) -> Option<u32> {
        match self {
            Self::Frontend => project.frontend_pid,
            Self::Backend => project.backend_pid,
        }
    }

    pub fn set_pid(self, project: &mut Project, pid: Option<u32>) {
        match self {
            Self::Frontend => project.frontend_pid = pid,
            Self::Backend => project.backend_pid = pid,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Theme {
    pub accent: String,
    pub bg: String,
    pub card: String,
    pub preset: String,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            accent: "#20bdb7".into(),
            bg: "#f2fbfb".into(),
            card: "#ffffff".into(),
            preset: "winglight".into(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Settings {
    pub language: String,
    pub theme: Theme,
    pub check_updates: bool,
    pub default_chat_model: String,
    pub providers: Vec<Provider>,
    pub onboarding_complete: bool,
    pub memory: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Provider {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub model: String,
    pub models: Vec<String>,
    pub model_names: std::collections::HashMap<String, String>,
    /// Persisted marker used by the settings UI. The secret itself stays in the OS keychain.
    pub key_configured: bool,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Chat {
    pub id: String,
    pub title: String,
    pub model: String,
    pub messages: Vec<crate::ai::ChatMessage>,
    pub updated_at: i64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            language: "zh-CN".into(),
            theme: Theme::default(),
            check_updates: true,
            default_chat_model: String::new(),
            providers: Vec::new(),
            onboarding_complete: false,
            memory: String::new(),
        }
    }
}
