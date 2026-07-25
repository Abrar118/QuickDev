use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct GlobalConfig {
    pub emulator: Option<String>,
    #[serde(default)]
    pub terminal_app_tabbing_prompt_declined: bool,
    // Skipped when empty so a freshly generated file has no `projects = []` line
    // ahead of its `[...]` sections. Comments in a TOML document attach to the
    // key that follows them, and a placeholder that later turns into
    // `[[projects]]` drags the file's header comment down with it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
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
    // See the note on `GlobalConfig::projects`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub terminals: Vec<TerminalEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
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
