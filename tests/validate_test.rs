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

#[test]
fn validate_terminal_entry_rejects_configs_quickdev_would_later_refuse() {
    use quickdev::models::TerminalEntry;
    use quickdev::validate::validate_terminal_entry;

    let root = std::path::Path::new("/tmp/project");
    let ok = TerminalEntry {
        name: "api".to_string(),
        path: "./api".to_string(),
        command: None,
        emulator: Some("kitty".to_string()),
    };
    assert!(validate_terminal_entry(&ok, root).is_ok());

    let blank_name = TerminalEntry {
        name: "   ".to_string(),
        path: ".".to_string(),
        command: None,
        emulator: None,
    };
    assert!(validate_terminal_entry(&blank_name, root)
        .unwrap_err()
        .contains("cannot be empty"));

    // A newline would forge an extra row in the one-item-per-line fzf picker.
    let newline_name = TerminalEntry {
        name: "api\nweb".to_string(),
        path: ".".to_string(),
        command: None,
        emulator: None,
    };
    assert!(validate_terminal_entry(&newline_name, root)
        .unwrap_err()
        .contains("control characters"));

    let bad_emulator = TerminalEntry {
        name: "api".to_string(),
        path: ".".to_string(),
        command: None,
        emulator: Some("nonexistent-terminal".to_string()),
    };
    assert!(validate_terminal_entry(&bad_emulator, root)
        .unwrap_err()
        .contains("unsupported emulator"));

    let escaping = TerminalEntry {
        name: "api".to_string(),
        path: "../outside".to_string(),
        command: None,
        emulator: None,
    };
    assert!(validate_terminal_entry(&escaping, root)
        .unwrap_err()
        .contains("must stay inside the project root"));
}

#[test]
fn validate_app_entry_requires_a_usable_name_and_path() {
    use quickdev::models::AppEntry;
    use quickdev::validate::validate_app_entry;

    assert!(validate_app_entry(&AppEntry {
        name: "Cursor".to_string(),
        path: "/Applications/Cursor.app".to_string(),
        args: None,
    })
    .is_ok());

    assert!(validate_app_entry(&AppEntry {
        name: String::new(),
        path: "/Applications/Cursor.app".to_string(),
        args: None,
    })
    .is_err());

    assert!(validate_app_entry(&AppEntry {
        name: "Cursor".to_string(),
        path: "  ".to_string(),
        args: None,
    })
    .is_err());
}

#[test]
fn validate_rejects_hand_authored_entries_that_add_would_refuse() {
    use quickdev::models::{AppEntry, ProjectConfig, ProjectEntry, TerminalEntry};
    use quickdev::validate::validate_project_config;

    // A config written by hand rather than through `quickdev add`, so it never
    // passed entry validation.
    let config = ProjectConfig {
        project: ProjectEntry {
            name: "p".to_string(),
        },
        terminals: vec![
            TerminalEntry {
                name: "  ".to_string(),
                path: ".".to_string(),
                command: None,
                emulator: None,
            },
            TerminalEntry {
                name: "api\nweb".to_string(),
                path: ".".to_string(),
                command: None,
                emulator: None,
            },
        ],
        applications: vec![AppEntry {
            name: String::new(),
            path: "/bin/sh".to_string(),
            args: None,
        }],
    };

    let report = validate_project_config(&config, std::path::Path::new("/tmp/project"));
    assert!(
        !report.is_ok(),
        "empty and control-character names are errors"
    );
    assert!(report.errors.iter().any(|e| e.contains("cannot be empty")));
    assert!(report
        .errors
        .iter()
        .any(|e| e.contains("control characters")));
}
