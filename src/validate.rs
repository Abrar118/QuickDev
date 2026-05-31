use crate::config::is_supported_emulator;
use crate::launch::{normalize_path, resolve_terminal_path, KNOWN_PLACEHOLDERS};
use crate::models::ProjectConfig;
use std::path::Path;

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

    for term in &config.terminals {
        if let Err(e) = resolve_terminal_path(project_root, &term.path) {
            errors.push(format!(
                "terminal '{}': invalid path '{}': {}",
                term.name, term.path, e
            ));
        }
        if let Some(emu) = &term.emulator {
            if !is_supported_emulator(emu) {
                errors.push(format!(
                    "terminal '{}': unsupported emulator '{}'",
                    term.name, emu
                ));
            }
        }
    }

    for app in &config.applications {
        if !Path::new(&normalize_path(&app.path)).exists() {
            warnings.push(format!(
                "application '{}': path does not exist: {}",
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
