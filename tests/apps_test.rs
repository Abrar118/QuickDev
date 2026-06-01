use quickdev::apps::{discover_apps, discover_apps_unique_by_path, parse_exec};
use std::collections::HashSet;

#[test]
fn parse_exec_strips_field_codes() {
    let (path, args) = parse_exec("code %F");
    assert_eq!(path, "code");
    assert!(args.is_empty());
}

#[test]
fn parse_exec_keeps_flatpak_args() {
    let (path, args) = parse_exec("flatpak run com.example.App");
    assert_eq!(path, "flatpak");
    assert_eq!(args, vec!["run".to_string(), "com.example.App".to_string()]);
}

#[test]
fn parse_exec_preserves_literal_percent() {
    let (path, args) = parse_exec("foo %% bar");
    assert_eq!(path, "foo");
    assert_eq!(args, vec!["%".to_string(), "bar".to_string()]);
}

#[test]
fn parse_exec_drops_emptied_token_keeps_flag() {
    let (path, args) = parse_exec("myapp --flag %U");
    assert_eq!(path, "myapp");
    assert_eq!(args, vec!["--flag".to_string()]);
}

#[test]
fn parse_exec_handles_quoted_args() {
    let (path, args) = parse_exec("env \"VAR=a b\" /snap/bin/app");
    assert_eq!(path, "env");
    assert_eq!(
        args,
        vec!["VAR=a b".to_string(), "/snap/bin/app".to_string()]
    );
}

#[test]
fn discover_apps_returns_vec() {
    let apps = discover_apps();
    if cfg!(target_os = "macos") {
        assert!(!apps.is_empty(), "macOS should find at least one app");
    } else {
        assert!(apps.is_empty(), "non-macOS should return empty vec");
    }
}

#[test]
fn discover_apps_entries_have_name_and_path() {
    let apps = discover_apps();
    for (name, path) in &apps {
        assert!(!name.is_empty(), "app name should not be empty");
        assert!(!path.is_empty(), "app path should not be empty");
        if cfg!(target_os = "macos") {
            assert!(path.ends_with(".app"), "path should end with .app: {path}");
        }
    }
}

#[test]
fn discover_apps_sorted_alphabetically() {
    let apps = discover_apps();
    if apps.len() >= 2 {
        for window in apps.windows(2) {
            assert!(
                window[0].0.to_lowercase() <= window[1].0.to_lowercase(),
                "apps should be sorted: '{}' should come before '{}'",
                window[0].0,
                window[1].0
            );
        }
    }
}

#[test]
fn discover_apps_unique_by_path_has_no_duplicate_paths() {
    let apps = discover_apps_unique_by_path();
    let mut seen = HashSet::new();
    for (_, path) in &apps {
        assert!(
            seen.insert(path),
            "duplicate path in path-unique list: {path}"
        );
    }
}

#[test]
fn discover_apps_unique_by_path_is_superset_of_discover_apps() {
    // Name-dedup keeps a subset of the path-unique list, so the path-unique
    // list must be at least as large.
    assert!(discover_apps_unique_by_path().len() >= discover_apps().len());
}
