use crate::models::{GlobalConfig, ProjectConfig};
use std::fs;
use std::path::{Path, PathBuf};

pub fn global_config_path() -> PathBuf {
    let home = dirs::home_dir().expect("could not determine home directory");
    home.join("Documents").join("quickdev").join("config.toml")
}

pub fn load_global_config(path: &Path) -> Result<GlobalConfig, String> {
    if !path.exists() {
        return Ok(GlobalConfig { projects: vec![] });
    }
    let content =
        fs::read_to_string(path).map_err(|e| format!("failed to read global config: {e}"))?;
    toml::from_str(&content).map_err(|e| format!("failed to parse global config: {e}"))
}

pub fn save_global_config(path: &Path, config: &GlobalConfig) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create config directory: {e}"))?;
    }
    let content = toml::to_string_pretty(config)
        .map_err(|e| format!("failed to serialize global config: {e}"))?;
    fs::write(path, content).map_err(|e| format!("failed to write global config: {e}"))
}

pub fn load_project_config(path: &Path) -> Result<ProjectConfig, String> {
    let content =
        fs::read_to_string(path).map_err(|e| format!("failed to read project config: {e}"))?;
    toml::from_str(&content).map_err(|e| format!("failed to parse project config: {e}"))
}

const TOML_COMMENT_HEADER: &str = "\
# QuickDev project configuration
# Edit this file directly or use: quickdev add, quickdev remove
#
# [project]
#   name = Display name for this project
#
# [[terminals]]
#   name    = Label for this terminal tab
#   path    = Working directory relative to project root (e.g., \".\", \"./src\")
#   command = (optional) Startup command to run when the terminal opens
#
# [[applications]]
#   name = Application display name
#   path = Executable path or .app bundle (e.g., \"/Applications/Cursor.app\")
#   args = (optional) Arguments list (e.g., [\".\"] to open project root)
";

pub fn save_project_config(path: &Path, config: &ProjectConfig) -> Result<(), String> {
    let serialized = toml::to_string_pretty(config)
        .map_err(|e| format!("failed to serialize project config: {e}"))?;

    let content = format!("{TOML_COMMENT_HEADER}\n{serialized}");
    fs::write(path, content).map_err(|e| format!("failed to write project config: {e}"))
}

pub fn find_project_config(start: &Path) -> Result<(PathBuf, PathBuf), String> {
    let mut current = start.to_path_buf();
    loop {
        let candidate = current.join(".quickdev.toml");
        if candidate.exists() {
            return Ok((candidate, current));
        }
        if !current.pop() {
            return Err("no .quickdev.toml found in current or parent directories".to_string());
        }
    }
}

pub fn unique_project_name(base_name: &str, config: &GlobalConfig) -> String {
    let names: Vec<&str> = config.projects.iter().map(|p| p.name.as_str()).collect();
    if !names.contains(&base_name) {
        return base_name.to_string();
    }
    let mut suffix = 2;
    loop {
        let candidate = format!("{base_name}-{suffix}");
        if !names.contains(&candidate.as_str()) {
            return candidate;
        }
        suffix += 1;
    }
}
