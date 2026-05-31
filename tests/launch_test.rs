use quickdev::launch::{
    escape_applescript_string, escape_powershell_single_quotes, normalize_path, resolve_app_args,
    resolve_terminal_path,
};
use std::path::Path;

#[test]
fn resolve_terminal_path_joins_relative() {
    let project_root = Path::new("/home/user/my-project");
    let result = resolve_terminal_path(project_root, ".").unwrap();
    assert_eq!(result, "/home/user/my-project");
}

#[test]
fn resolve_terminal_path_joins_subdir() {
    let project_root = Path::new("/home/user/my-project");
    let result = resolve_terminal_path(project_root, "./src/server").unwrap();
    assert_eq!(result, "/home/user/my-project/src/server");
}

#[test]
fn resolve_terminal_path_rejects_parent_escape() {
    let project_root = Path::new("/home/user/my-project");
    assert!(resolve_terminal_path(project_root, "../../outside").is_err());
}

#[test]
fn resolve_terminal_path_rejects_absolute() {
    let project_root = Path::new("/home/user/my-project");
    assert!(resolve_terminal_path(project_root, "/etc/passwd").is_err());
}

#[test]
fn resolve_terminal_path_rejects_embedded_parent() {
    let project_root = Path::new("/home/user/my-project");
    assert!(resolve_terminal_path(project_root, "src/../../escape").is_err());
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

#[test]
fn escape_applescript_escapes_backslash_and_quote() {
    assert_eq!(escape_applescript_string(r#"a\b"c"#), r#"a\\b\"c"#);
}

#[test]
fn escape_applescript_leaves_plain_text() {
    assert_eq!(escape_applescript_string("plain text"), "plain text");
}

#[test]
fn escape_powershell_doubles_single_quotes() {
    assert_eq!(escape_powershell_single_quotes("Abrar's PC"), "Abrar''s PC");
}
