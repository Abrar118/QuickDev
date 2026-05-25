use quickdev::launch::{normalize_path, resolve_terminal_path, resolve_app_args};
use std::path::Path;

#[test]
fn resolve_terminal_path_joins_relative() {
    let project_root = Path::new("/home/user/my-project");
    let result = resolve_terminal_path(project_root, ".");
    assert_eq!(result, "/home/user/my-project");
}

#[test]
fn resolve_terminal_path_joins_subdir() {
    let project_root = Path::new("/home/user/my-project");
    let result = resolve_terminal_path(project_root, "./src/server");
    assert_eq!(result, "/home/user/my-project/src/server");
}

#[test]
fn resolve_app_args_replaces_dot() {
    let project_root = Path::new("/home/user/my-project");
    let args = vec![".".to_string(), "--flag".to_string()];
    let result = resolve_app_args(project_root, &args);
    assert_eq!(result, vec!["/home/user/my-project", "--flag"]);
}

#[test]
fn resolve_app_args_no_dot() {
    let project_root = Path::new("/home/user/my-project");
    let args = vec!["--verbose".to_string()];
    let result = resolve_app_args(project_root, &args);
    assert_eq!(result, vec!["--verbose"]);
}

#[test]
fn normalize_path_expands_tilde() {
    let result = normalize_path("~/Documents/test");
    assert!(
        !result.starts_with('~'),
        "tilde should be expanded, got: {result}"
    );
}

#[test]
fn normalize_path_leaves_absolute_unchanged() {
    let result = normalize_path("/usr/local/bin/app");
    assert_eq!(result, "/usr/local/bin/app");
}
