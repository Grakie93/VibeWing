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
    pub frontend_port: String,
    pub backend_port: String,
    pub frontend_pid: Option<u32>,
    pub backend_pid: Option<u32>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProjectView {
    #[serde(flatten)]
    pub project: Project,
    pub frontend_running: bool,
    pub backend_running: bool,
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

    pub fn directory<'a>(self, project: &'a Project) -> &'a str {
        match self {
            Self::Frontend => &project.frontend_path,
            Self::Backend => &project.backend_path,
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
    pub providers: Vec<serde_json::Value>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            language: "zh-CN".into(),
            theme: Theme::default(),
            check_updates: true,
            default_chat_model: String::new(),
            providers: Vec::new(),
        }
    }
}
