use crate::fzf;
use crate::models::{GlobalConfig, GlobalProjectEntry, ProjectConfig};
use std::fs;
use std::path::{Path, PathBuf};

pub fn global_config_path() -> PathBuf {
    let home = dirs::home_dir().expect("could not determine home directory");
    home.join("Documents").join("quickdev").join("config.toml")
}

pub fn load_global_config(path: &Path) -> Result<GlobalConfig, String> {
    if !path.exists() {
        return Ok(GlobalConfig {
            emulator: None,
            projects: vec![],
        });
    }
    let content =
        fs::read_to_string(path).map_err(|e| format!("failed to read global config: {e}"))?;
    toml::from_str(&content).map_err(|e| format!("failed to parse global config: {e}"))
}

const GLOBAL_COMMENT_HEADER: &str = "\
# QuickDev global configuration
#
# emulator = (optional) Default terminal emulator: \"ghostty\", \"terminal\"
#
# Projects are auto-managed by quickdev init / deregister
";

pub fn save_global_config(path: &Path, config: &GlobalConfig) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create config directory: {e}"))?;
    }
    let serialized = toml::to_string_pretty(config)
        .map_err(|e| format!("failed to serialize global config: {e}"))?;
    let content = format!("{GLOBAL_COMMENT_HEADER}\n{serialized}");
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
#   command  = (optional) Startup command to run when the terminal opens
#   emulator = (optional) Terminal emulator: \"ghostty\", \"terminal\". Omit for auto-detect
#
# [[applications]]
#   name = Application display name
#   path = Executable path or .app bundle (e.g., \"/Applications/Cursor.app\")
#   args = (optional) Arguments list. Placeholders: {root} {config} {name} {cwd}
#          e.g., [\"{root}\"] opens project root; [\"{config}\"] opens this file
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

pub fn register_existing_project_config(
    config_path: &Path,
    project_path: String,
    global: &mut GlobalConfig,
) -> Result<String, String> {
    let mut existing = load_project_config(config_path)?;
    let project_name = unique_project_name(&existing.project.name, global);
    if existing.project.name != project_name {
        existing.project.name = project_name.clone();
        save_project_config(config_path, &existing)?;
    }
    global.projects.push(GlobalProjectEntry {
        name: project_name.clone(),
        path: project_path,
    });
    Ok(project_name)
}

pub fn resolve_project_config(start: &Path) -> Result<(PathBuf, PathBuf), String> {
    match find_project_config(start) {
        Ok(result) => Ok(result),
        Err(_) => fzf_select_project(),
    }
}

pub fn parse_project_selection(selected: &str) -> Result<usize, String> {
    selected
        .split_once(':')
        .and_then(|(index, _)| index.trim().parse::<usize>().ok())
        .ok_or_else(|| "invalid selection".to_string())
}

pub const SUPPORTED_EMULATORS: &[&str] = &["ghostty", "terminal"];

pub fn is_supported_emulator(value: &str) -> bool {
    SUPPORTED_EMULATORS.contains(&value)
}

fn unknown_key_error(key: &str) -> String {
    format!("unknown config key {key:?} (supported: emulator)")
}

pub fn set_global_setting(
    config: &mut GlobalConfig,
    key: &str,
    value: &str,
) -> Result<String, String> {
    match key {
        "emulator" => {
            if !is_supported_emulator(value) {
                return Err(format!(
                    "unsupported emulator {value:?} (supported: {})",
                    SUPPORTED_EMULATORS.join(", ")
                ));
            }
            config.emulator = Some(value.to_string());
            Ok(format!("Set emulator = {value}"))
        }
        other => Err(unknown_key_error(other)),
    }
}

pub fn get_global_setting(config: &GlobalConfig, key: &str) -> Result<String, String> {
    match key {
        "emulator" => Ok(match &config.emulator {
            Some(v) => format!("emulator = {v}"),
            None => "emulator is not set (auto-detect)".to_string(),
        }),
        other => Err(unknown_key_error(other)),
    }
}

pub fn unset_global_setting(config: &mut GlobalConfig, key: &str) -> Result<String, String> {
    match key {
        "emulator" => {
            config.emulator = None;
            Ok("Unset emulator".to_string())
        }
        other => Err(unknown_key_error(other)),
    }
}

/// Health of a registered project: its directory and its `.quickdev.toml` must both exist.
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectStatus {
    pub name: String,
    pub path: String,
    pub path_exists: bool,
    pub config_exists: bool,
}

impl ProjectStatus {
    pub fn is_healthy(&self) -> bool {
        self.path_exists && self.config_exists
    }

    pub fn issue(&self) -> Option<&'static str> {
        if !self.path_exists {
            Some("path missing")
        } else if !self.config_exists {
            Some(".quickdev.toml missing")
        } else {
            None
        }
    }
}

pub fn project_status(entry: &GlobalProjectEntry) -> ProjectStatus {
    let path = Path::new(&entry.path);
    let path_exists = path.exists();
    let config_exists = path.join(".quickdev.toml").exists();
    ProjectStatus {
        name: entry.name.clone(),
        path: entry.path.clone(),
        path_exists,
        config_exists,
    }
}

pub fn project_statuses(global: &GlobalConfig) -> Vec<ProjectStatus> {
    global.projects.iter().map(project_status).collect()
}

/// Subset of statuses that are not healthy (path or config missing).
pub fn missing_statuses(statuses: &[ProjectStatus]) -> Vec<&ProjectStatus> {
    statuses.iter().filter(|s| !s.is_healthy()).collect()
}

/// Removes registrations whose path or `.quickdev.toml` is missing.
/// Returns the names of removed projects, in their original order.
pub fn prune_projects(global: &mut GlobalConfig) -> Vec<String> {
    let mut removed = Vec::new();
    global.projects.retain(|entry| {
        if project_status(entry).is_healthy() {
            true
        } else {
            removed.push(entry.name.clone());
            false
        }
    });
    removed
}

/// Serialize project statuses to a JSON array string for `list --json`.
pub fn projects_json(statuses: &[ProjectStatus]) -> String {
    fn esc(s: &str) -> String {
        use std::fmt::Write as _;

        let mut out = String::with_capacity(s.len());
        for c in s.chars() {
            match c {
                '"' => out.push_str("\\\""),
                '\\' => out.push_str("\\\\"),
                c if c < ' ' => {
                    let _ = write!(out, "\\u{:04x}", c as u32);
                }
                c => out.push(c),
            }
        }
        out
    }

    if statuses.is_empty() {
        return "[]".to_string();
    }

    let mut out = String::from("[");
    for (i, s) in statuses.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&format!(
            "\n  {{\"name\": \"{}\", \"path\": \"{}\", \"healthy\": {}, \"path_exists\": {}, \"config_exists\": {}}}",
            esc(&s.name),
            esc(&s.path),
            s.is_healthy(),
            s.path_exists,
            s.config_exists
        ));
    }
    out.push_str("\n]");
    out
}

fn fzf_select_project() -> Result<(PathBuf, PathBuf), String> {
    let global_path = global_config_path();
    let global = load_global_config(&global_path)?;

    if global.projects.is_empty() {
        return Err(
            "No projects registered. Run 'quickdev init' in a project directory.".to_string(),
        );
    }

    if !fzf::check_fzf() {
        return Err(
            "no .quickdev.toml found in current or parent directories.\nTip: install fzf for interactive project selection"
                .to_string(),
        );
    }

    let items: Vec<String> = global
        .projects
        .iter()
        .enumerate()
        .map(|(i, p)| format!("{i}: {}    {}", p.name, p.path))
        .collect();

    let selected = fzf::fzf_select_one(&items, "Select a project:")?;

    let index = parse_project_selection(&selected)?;
    let entry = global.projects.get(index).ok_or("invalid selection")?;

    let root = PathBuf::from(&entry.path);
    let config_path = root.join(".quickdev.toml");

    if !config_path.exists() {
        return Err(format!(
            ".quickdev.toml not found at {}",
            config_path.display()
        ));
    }

    Ok((config_path, root))
}
