use quickdev::apps::discover_apps;

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
