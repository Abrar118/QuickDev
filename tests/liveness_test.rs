use quickdev::config::{
    missing_statuses, project_status, project_statuses, prune_projects, ProjectStatus,
};
use quickdev::models::{GlobalConfig, GlobalProjectEntry};
use std::fs;

fn entry(name: &str, path: &str) -> GlobalProjectEntry {
    GlobalProjectEntry {
        name: name.to_string(),
        path: path.to_string(),
    }
}

#[test]
fn project_status_healthy_when_dir_and_config_exist() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join(".quickdev.toml"), "").unwrap();

    let status = project_status(&entry("proj", dir.path().to_str().unwrap()));

    assert!(status.path_exists);
    assert!(status.config_exists);
    assert!(status.is_healthy());
    assert_eq!(status.issue(), None);
}

#[test]
fn project_status_path_missing() {
    let status = project_status(&entry("gone", "/no/such/path/quickdev-xyz"));

    assert!(!status.path_exists);
    assert!(!status.config_exists);
    assert!(!status.is_healthy());
    assert_eq!(status.issue(), Some("path missing"));
}

#[test]
fn project_status_config_missing() {
    let dir = tempfile::tempdir().unwrap();
    // directory exists but has no .quickdev.toml

    let status = project_status(&entry("proj", dir.path().to_str().unwrap()));

    assert!(status.path_exists);
    assert!(!status.config_exists);
    assert!(!status.is_healthy());
    assert_eq!(status.issue(), Some(".quickdev.toml missing"));
}

#[test]
fn project_statuses_maps_all_entries() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join(".quickdev.toml"), "").unwrap();

    let global = GlobalConfig {
        emulator: None,
        projects: vec![
            entry("ok", dir.path().to_str().unwrap()),
            entry("gone", "/no/such/path/quickdev-xyz"),
        ],
    };

    let statuses = project_statuses(&global);
    assert_eq!(statuses.len(), 2);
    assert!(statuses[0].is_healthy());
    assert!(!statuses[1].is_healthy());
}

#[test]
fn missing_statuses_filters_unhealthy() {
    let healthy = ProjectStatus {
        name: "ok".to_string(),
        path: "/ok".to_string(),
        path_exists: true,
        config_exists: true,
    };
    let broken = ProjectStatus {
        name: "broken".to_string(),
        path: "/broken".to_string(),
        path_exists: false,
        config_exists: false,
    };
    let statuses = vec![healthy, broken];

    let missing = missing_statuses(&statuses);
    assert_eq!(missing.len(), 1);
    assert_eq!(missing[0].name, "broken");
}

#[test]
fn prune_projects_removes_unhealthy_and_returns_names_in_order() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join(".quickdev.toml"), "").unwrap();

    let mut global = GlobalConfig {
        emulator: None,
        projects: vec![
            entry("dead-1", "/no/such/path/quickdev-a"),
            entry("alive", dir.path().to_str().unwrap()),
            entry("dead-2", "/no/such/path/quickdev-b"),
        ],
    };

    let removed = prune_projects(&mut global);

    assert_eq!(removed, vec!["dead-1".to_string(), "dead-2".to_string()]);
    assert_eq!(global.projects.len(), 1);
    assert_eq!(global.projects[0].name, "alive");
}

#[test]
fn prune_projects_keeps_all_when_healthy() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join(".quickdev.toml"), "").unwrap();

    let mut global = GlobalConfig {
        emulator: None,
        projects: vec![entry("alive", dir.path().to_str().unwrap())],
    };

    let removed = prune_projects(&mut global);
    assert!(removed.is_empty());
    assert_eq!(global.projects.len(), 1);
}
