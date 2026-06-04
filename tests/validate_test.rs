use quickdev::models::{AppEntry, ProjectConfig, ProjectEntry, TerminalEntry};
use quickdev::validate::validate_project_config;
use std::path::Path;

fn root() -> &'static Path {
    Path::new("/home/user/project")
}

fn config(name: &str, terminals: Vec<TerminalEntry>, applications: Vec<AppEntry>) -> ProjectConfig {
    ProjectConfig {
        project: ProjectEntry {
            name: name.to_string(),
        },
        terminals,
        applications,
    }
}

fn term(name: &str, path: &str, emulator: Option<&str>) -> TerminalEntry {
    TerminalEntry {
        name: name.to_string(),
        path: path.to_string(),
        command: None,
        emulator: emulator.map(|s| s.to_string()),
    }
}

#[test]
fn clean_config_is_ok() {
    let cfg = config("proj", vec![term("dev", "./src", None)], vec![]);
    let report = validate_project_config(&cfg, root());
    assert!(report.is_ok());
    assert!(report.warnings.is_empty());
}

#[test]
fn empty_project_name_is_error() {
    let cfg = config("   ", vec![], vec![]);
    let report = validate_project_config(&cfg, root());
    assert!(!report.is_ok());
    assert!(report.errors.iter().any(|e| e.contains("project.name")));
}

#[test]
fn escaping_terminal_path_is_error() {
    let cfg = config("proj", vec![term("bad", "../../etc", None)], vec![]);
    let report = validate_project_config(&cfg, root());
    assert!(!report.is_ok());
    assert!(report.errors.iter().any(|e| e.contains("bad")));
}

#[test]
fn unsupported_emulator_is_error() {
    let cfg = config(
        "proj",
        vec![term("dev", ".", Some("nonexistent-terminal"))],
        vec![],
    );
    let report = validate_project_config(&cfg, root());
    assert!(!report.is_ok());
    assert!(report
        .errors
        .iter()
        .any(|e| e.contains("nonexistent-terminal") && e.contains("emulator")));
}

#[test]
fn missing_app_path_is_warning() {
    let app = AppEntry {
        name: "Ghost".to_string(),
        path: "/no/such/app-xyz-quickdev.app".to_string(),
        args: None,
    };
    let cfg = config("proj", vec![], vec![app]);
    let report = validate_project_config(&cfg, root());
    assert!(report.is_ok(), "missing app path must not be a hard error");
    assert!(report
        .warnings
        .iter()
        .any(|w| w.contains("Ghost") && w.contains("does not exist")));
}

#[test]
fn unknown_placeholder_is_warning() {
    let app = AppEntry {
        name: "Editor".to_string(),
        path: "/no/such/app-xyz-quickdev.app".to_string(),
        args: Some(vec!["{root}".to_string(), "{bogus}".to_string()]),
    };
    let cfg = config("proj", vec![], vec![app]);
    let report = validate_project_config(&cfg, root());
    assert!(report.is_ok());
    assert!(report.warnings.iter().any(|w| w.contains("bogus")));
    // a known placeholder must NOT be flagged
    assert!(!report.warnings.iter().any(|w| w.contains("{root}")));
}
