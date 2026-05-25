use quickdev::config::{
    find_project_config, load_global_config, load_project_config, resolve_project_config,
    save_global_config, save_project_config, unique_project_name,
};
use quickdev::models::{
    GlobalConfig, GlobalProjectEntry, ProjectConfig, ProjectEntry, TerminalEntry,
};
use std::fs;

#[test]
fn save_and_load_project_config() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join(".quickdev.toml");

    let config = ProjectConfig {
        project: ProjectEntry {
            name: "test-proj".to_string(),
        },
        terminals: vec![TerminalEntry {
            name: "dev".to_string(),
            path: ".".to_string(),
            command: Some("cargo run".to_string()),
        }],
        applications: vec![],
    };

    save_project_config(&config_path, &config).unwrap();
    let loaded = load_project_config(&config_path).unwrap();

    assert_eq!(loaded.project.name, "test-proj");
    assert_eq!(loaded.terminals.len(), 1);
    assert_eq!(loaded.terminals[0].name, "dev");
}

#[test]
fn save_and_load_global_config() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config.toml");

    let config = GlobalConfig {
        projects: vec![GlobalProjectEntry {
            name: "proj-a".to_string(),
            path: "/tmp/proj-a".to_string(),
        }],
    };

    save_global_config(&config_path, &config).unwrap();
    let loaded = load_global_config(&config_path).unwrap();

    assert_eq!(loaded.projects.len(), 1);
    assert_eq!(loaded.projects[0].name, "proj-a");
}

#[test]
fn load_global_config_creates_empty_if_missing() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("nonexistent").join("config.toml");

    let loaded = load_global_config(&config_path).unwrap();
    assert!(loaded.projects.is_empty());
}

#[test]
fn find_project_config_walks_parents() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let nested = root.join("a").join("b").join("c");
    fs::create_dir_all(&nested).unwrap();

    let config = ProjectConfig {
        project: ProjectEntry {
            name: "root-proj".to_string(),
        },
        terminals: vec![],
        applications: vec![],
    };
    save_project_config(&root.join(".quickdev.toml"), &config).unwrap();

    let found = find_project_config(&nested).unwrap();
    assert_eq!(found.0, root.join(".quickdev.toml"));
    assert_eq!(found.1, root.to_path_buf());
}

#[test]
fn find_project_config_returns_error_if_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let result = find_project_config(dir.path());
    assert!(result.is_err());
}

#[test]
fn unique_project_name_appends_suffix() {
    let config = GlobalConfig {
        projects: vec![
            GlobalProjectEntry {
                name: "my-app".to_string(),
                path: "/a".to_string(),
            },
            GlobalProjectEntry {
                name: "my-app-2".to_string(),
                path: "/b".to_string(),
            },
        ],
    };

    assert_eq!(unique_project_name("my-app", &config), "my-app-3");
    assert_eq!(unique_project_name("new-proj", &config), "new-proj");
}

#[test]
fn save_project_config_adds_comment_header() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join(".quickdev.toml");

    let config = ProjectConfig {
        project: ProjectEntry {
            name: "test-proj".to_string(),
        },
        terminals: vec![],
        applications: vec![],
    };

    save_project_config(&config_path, &config).unwrap();
    let content = std::fs::read_to_string(&config_path).unwrap();

    assert!(
        content.starts_with("# QuickDev project configuration"),
        "should start with comment header, got:\n{content}"
    );
    assert!(content.contains("[project]"));
    assert!(content.contains("name = \"test-proj\""));
}

#[test]
fn save_project_config_preserves_existing_header() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join(".quickdev.toml");

    let config = ProjectConfig {
        project: ProjectEntry {
            name: "test-proj".to_string(),
        },
        terminals: vec![],
        applications: vec![],
    };

    save_project_config(&config_path, &config).unwrap();

    let config2 = ProjectConfig {
        project: ProjectEntry {
            name: "test-proj".to_string(),
        },
        terminals: vec![TerminalEntry {
            name: "dev".to_string(),
            path: ".".to_string(),
            command: None,
        }],
        applications: vec![],
    };
    save_project_config(&config_path, &config2).unwrap();

    let content = std::fs::read_to_string(&config_path).unwrap();

    assert!(
        content.starts_with("# QuickDev project configuration"),
        "header should be preserved after re-save"
    );
    assert!(content.contains("[[terminals]]"));
    assert!(content.contains("name = \"dev\""));
    assert_eq!(
        content.matches("# QuickDev project configuration").count(),
        1,
        "header should not be duplicated"
    );
}

#[test]
fn resolve_project_config_finds_local() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    let config = ProjectConfig {
        project: ProjectEntry {
            name: "local-proj".to_string(),
        },
        terminals: vec![],
        applications: vec![],
    };
    save_project_config(&root.join(".quickdev.toml"), &config).unwrap();

    let result = resolve_project_config(root);
    assert!(result.is_ok());
    let (config_path, project_root) = result.unwrap();
    assert_eq!(config_path, root.join(".quickdev.toml"));
    assert_eq!(project_root, root.to_path_buf());
}
