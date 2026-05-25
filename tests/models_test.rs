use quickdev::models::{AppEntry, GlobalConfig, GlobalProjectEntry, ProjectConfig, ProjectEntry, TerminalEntry};

#[test]
fn project_config_round_trip() {
    let config = ProjectConfig {
        project: ProjectEntry {
            name: "my-app".to_string(),
        },
        terminals: vec![
            TerminalEntry {
                name: "server".to_string(),
                path: ".".to_string(),
                command: Some("npm run dev".to_string()),
            },
            TerminalEntry {
                name: "logs".to_string(),
                path: "./logs".to_string(),
                command: None,
            },
        ],
        applications: vec![AppEntry {
            name: "Cursor".to_string(),
            path: "/Applications/Cursor.app".to_string(),
            args: Some(vec![".".to_string()]),
        }],
    };

    let toml_str = toml::to_string_pretty(&config).unwrap();
    let parsed: ProjectConfig = toml::from_str(&toml_str).unwrap();

    assert_eq!(parsed.project.name, "my-app");
    assert_eq!(parsed.terminals.len(), 2);
    assert_eq!(parsed.terminals[0].name, "server");
    assert_eq!(parsed.terminals[0].command, Some("npm run dev".to_string()));
    assert_eq!(parsed.terminals[1].command, None);
    assert_eq!(parsed.applications.len(), 1);
    assert_eq!(parsed.applications[0].args, Some(vec![".".to_string()]));
}

#[test]
fn global_config_round_trip() {
    let config = GlobalConfig {
        projects: vec![
            GlobalProjectEntry {
                name: "app-one".to_string(),
                path: "/tmp/app-one".to_string(),
            },
            GlobalProjectEntry {
                name: "app-two".to_string(),
                path: "/tmp/app-two".to_string(),
            },
        ],
    };

    let toml_str = toml::to_string_pretty(&config).unwrap();
    let parsed: GlobalConfig = toml::from_str(&toml_str).unwrap();

    assert_eq!(parsed.projects.len(), 2);
    assert_eq!(parsed.projects[0].name, "app-one");
    assert_eq!(parsed.projects[0].path, "/tmp/app-one");
}

#[test]
fn project_config_minimal_toml() {
    let toml_str = r#"
[project]
name = "bare"
"#;
    let parsed: ProjectConfig = toml::from_str(toml_str).unwrap();
    assert_eq!(parsed.project.name, "bare");
    assert!(parsed.terminals.is_empty());
    assert!(parsed.applications.is_empty());
}
