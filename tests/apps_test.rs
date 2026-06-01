use quickdev::apps::{
    combine_app_args, discover_apps, discover_apps_unique_by_path, parse_desktop_entry, parse_exec,
};
use std::collections::HashSet;

#[test]
fn discover_apps_returns_without_panic() {
    let apps = discover_apps();
    if cfg!(target_os = "macos") {
        assert!(!apps.is_empty(), "macOS should find at least one app");
    }
    // On Linux/Windows the result depends on what's installed; just ensure the
    // call returns and entries are well-formed (checked below).
    let _ = apps;
}

#[test]
fn discover_apps_entries_have_name_and_path() {
    let apps = discover_apps();
    for app in &apps {
        assert!(!app.name.is_empty(), "app name should not be empty");
        assert!(!app.path.is_empty(), "app path should not be empty");
        if cfg!(target_os = "macos") {
            assert!(
                app.path.ends_with(".app"),
                "macOS path should end with .app: {}",
                app.path
            );
        }
    }
}

#[test]
fn discover_apps_sorted_alphabetically() {
    let apps = discover_apps();
    if apps.len() >= 2 {
        for window in apps.windows(2) {
            assert!(
                window[0].name.to_lowercase() <= window[1].name.to_lowercase(),
                "apps should be sorted: '{}' should come before '{}'",
                window[0].name,
                window[1].name
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
fn parse_exec_keeps_embedded_percent_token() {
    let (path, args) = parse_exec("app http://x/%foo");
    assert_eq!(path, "app");
    assert_eq!(args, vec!["http://x/%foo".to_string()]);
}

#[test]
fn parse_desktop_entry_valid() {
    let c = "[Desktop Entry]\nType=Application\nName=Cursor\nExec=cursor %F\n";
    let e = parse_desktop_entry(c, |_| true).expect("should parse");
    assert_eq!(e.name, "Cursor");
    assert_eq!(e.path, "cursor");
    assert!(e.args.is_none());
}

#[test]
fn parse_desktop_entry_nodisplay_skipped() {
    let c = "[Desktop Entry]\nType=Application\nName=X\nExec=x\nNoDisplay=true\n";
    assert!(parse_desktop_entry(c, |_| true).is_none());
}

#[test]
fn parse_desktop_entry_hidden_skipped() {
    let c = "[Desktop Entry]\nType=Application\nName=X\nExec=x\nHidden=true\n";
    assert!(parse_desktop_entry(c, |_| true).is_none());
}

#[test]
fn parse_desktop_entry_non_application_skipped() {
    let c = "[Desktop Entry]\nType=Directory\nName=X\nExec=x\n";
    assert!(parse_desktop_entry(c, |_| true).is_none());
}

#[test]
fn parse_desktop_entry_missing_name_or_exec() {
    let no_name = "[Desktop Entry]\nType=Application\nExec=x\n";
    assert!(parse_desktop_entry(no_name, |_| true).is_none());
    let no_exec = "[Desktop Entry]\nType=Application\nName=X\n";
    assert!(parse_desktop_entry(no_exec, |_| true).is_none());
}

#[test]
fn parse_desktop_entry_tryexec_unresolvable_skipped() {
    let c = "[Desktop Entry]\nType=Application\nName=X\nExec=x\nTryExec=/nope/x\n";
    assert!(parse_desktop_entry(c, |_| false).is_none());
}

#[test]
fn parse_desktop_entry_keeps_flatpak_args() {
    let c = "[Desktop Entry]\nType=Application\nName=App\nExec=flatpak run com.example.App %U\n";
    let e = parse_desktop_entry(c, |_| true).unwrap();
    assert_eq!(e.path, "flatpak");
    assert_eq!(
        e.args.unwrap(),
        vec!["run".to_string(), "com.example.App".to_string()]
    );
}

#[test]
fn parse_desktop_entry_ignores_other_groups() {
    let c = "[Desktop Entry]\nType=Application\nName=Main\nExec=main\n[Desktop Action New]\nName=New\nExec=other\n";
    let e = parse_desktop_entry(c, |_| true).unwrap();
    assert_eq!(e.name, "Main");
    assert_eq!(e.path, "main");
}

#[test]
fn combine_app_args_none_none() {
    assert_eq!(combine_app_args(None, None), None);
}

#[test]
fn combine_app_args_discovered_only() {
    assert_eq!(
        combine_app_args(Some(vec!["run".to_string()]), None),
        Some(vec!["run".to_string()])
    );
}

#[test]
fn combine_app_args_user_only() {
    assert_eq!(
        combine_app_args(None, Some(vec![".".to_string()])),
        Some(vec![".".to_string()])
    );
}

#[test]
fn combine_app_args_appends_user_after_discovered() {
    let got = combine_app_args(
        Some(vec!["run".to_string(), "app".to_string()]),
        Some(vec![".".to_string()]),
    );
    assert_eq!(
        got,
        Some(vec!["run".to_string(), "app".to_string(), ".".to_string()])
    );
}
