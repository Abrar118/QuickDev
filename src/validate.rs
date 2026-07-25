use crate::adapters::resolve_command;
use crate::config::is_supported_emulator;
use crate::launch::{normalize_path, resolve_terminal_path, KNOWN_PLACEHOLDERS};
use crate::models::{AppEntry, ProjectConfig, TerminalEntry};
use std::path::Path;

/// Reject a display name QuickDev cannot round-trip.
///
/// Names identify entries in `add`/`remove`/`launch`, and appear in fzf pickers
/// one item per line. An empty name matches nothing; a name carrying a newline
/// or other control character would forge extra picker rows and break the
/// selection mapping. The interactive paths already refuse empty names — this
/// makes the direct `--name` flags agree with them.
fn validate_name(kind: &str, name: &str) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err(format!("{kind} name cannot be empty"));
    }
    if name.chars().any(char::is_control) {
        return Err(format!(
            "{kind} name cannot contain control characters or newlines"
        ));
    }
    Ok(())
}

/// Validate a terminal before it is written to a config, so `add` cannot persist
/// an entry that `validate` or `launch` would reject afterwards.
pub fn validate_terminal_entry(entry: &TerminalEntry, project_root: &Path) -> Result<(), String> {
    validate_name("terminal", &entry.name)?;
    if entry.path.trim().is_empty() {
        return Err("terminal path cannot be empty".to_string());
    }
    resolve_terminal_path(project_root, &entry.path)
        .map_err(|e| format!("invalid path '{}': {e}", entry.path))?;
    if let Some(emulator) = &entry.emulator {
        if !is_supported_emulator(emulator) {
            return Err(format!(
                "unsupported emulator '{emulator}' (supported: {})",
                crate::config::SUPPORTED_EMULATORS.join(", ")
            ));
        }
    }
    Ok(())
}

/// Validate an application before it is written to a config.
pub fn validate_app_entry(entry: &AppEntry) -> Result<(), String> {
    validate_name("application", &entry.name)?;
    if entry.path.trim().is_empty() {
        return Err("application path cannot be empty".to_string());
    }
    Ok(())
}

/// Whether an application target is launchable: a path on disk, or a bare
/// command name found on `PATH`.
///
/// Launch hands the value to `Command::new`, which resolves bare names through
/// `PATH`. Checking only `Path::exists` therefore warned about perfectly valid
/// entries such as the `flatpak` command in a Linux desktop file.
pub fn app_target_resolvable(path: &str) -> bool {
    let normalized = normalize_path(path);
    if Path::new(&normalized).exists() {
        return true;
    }
    !normalized.contains(std::path::MAIN_SEPARATOR) && resolve_command(&normalized).is_some()
}

/// Result of validating a project config: hard errors and softer warnings.
pub struct ValidationReport {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl ValidationReport {
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }
}

pub fn validate_project_config(config: &ProjectConfig, project_root: &Path) -> ValidationReport {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    if config.project.name.trim().is_empty() {
        errors.push("project.name is empty".to_string());
    }

    // Same rules `add` enforces, so a hand-edited config cannot hold an entry
    // the CLI would have refused to create. `validate` is where those configs
    // are caught, since nothing stops a user from writing the file directly.
    for term in &config.terminals {
        if let Err(e) = validate_terminal_entry(term, project_root) {
            errors.push(format!("terminal '{}': {e}", term.name));
        }
    }

    for app in &config.applications {
        if let Err(e) = validate_app_entry(app) {
            errors.push(format!("application '{}': {e}", app.name));
        }
        if !app_target_resolvable(&app.path) {
            warnings.push(format!(
                "application '{}': path does not exist and is not on PATH: {}",
                app.name, app.path
            ));
        }
        if let Some(args) = &app.args {
            for arg in args {
                for token in unknown_placeholders(arg) {
                    warnings.push(format!(
                        "application '{}': unknown placeholder '{{{}}}'",
                        app.name, token
                    ));
                }
            }
        }
    }

    ValidationReport { errors, warnings }
}

/// Token names appearing as `{token}` in `arg` that are not known placeholders.
fn unknown_placeholders(arg: &str) -> Vec<String> {
    let mut unknown = Vec::new();
    let mut rest = arg;
    while let Some(open) = rest.find('{') {
        if let Some(close_rel) = rest[open..].find('}') {
            let close = open + close_rel;
            let token = &rest[open + 1..close];
            if !token.is_empty() && !KNOWN_PLACEHOLDERS.contains(&token) {
                unknown.push(token.to_string());
            }
            rest = &rest[close + 1..];
        } else {
            break;
        }
    }
    unknown
}
