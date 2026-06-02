use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct GlobalConfig {
    pub emulator: Option<String>,
    #[serde(default)]
    pub terminal_app_tabbing_prompt_declined: bool,
    #[serde(default)]
    pub projects: Vec<GlobalProjectEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GlobalProjectEntry {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub project: ProjectEntry,
    #[serde(default)]
    pub terminals: Vec<TerminalEntry>,
    #[serde(default)]
    pub applications: Vec<AppEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectEntry {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TerminalEntry {
    pub name: String,
    pub path: String,
    pub command: Option<String>,
    pub emulator: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppEntry {
    pub name: String,
    pub path: String,
    pub args: Option<Vec<String>>,
}
