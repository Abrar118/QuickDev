use quickdev::capture::detected_to_apps;
use quickdev::capture::merge_apps;
use quickdev::models::AppEntry;

fn installed() -> Vec<(String, String)> {
    vec![
        ("Cursor".to_string(), "/Applications/Cursor.app".to_string()),
        (
            "Firefox".to_string(),
            "/Applications/Firefox.app".to_string(),
        ),
        ("Slack".to_string(), "/Applications/Slack.app".to_string()),
    ]
}

#[test]
fn detected_to_apps_matches_running_path_to_installed_bundle() {
    let running = vec!["/Applications/Cursor.app".to_string()];
    let apps = detected_to_apps(&running, &installed());
    assert_eq!(
        apps,
        vec![AppEntry {
            name: "Cursor".to_string(),
            path: "/Applications/Cursor.app".to_string(),
            args: None,
        }]
    );
}

#[test]
fn detected_to_apps_drops_unmatched_running_paths() {
    let running = vec![
        "/Applications/Cursor.app".to_string(),
        "/System/Library/CoreServices/Finder.app".to_string(),
    ];
    let apps = detected_to_apps(&running, &installed());
    assert_eq!(apps.len(), 1);
    assert_eq!(apps[0].name, "Cursor");
}

#[test]
fn detected_to_apps_matches_with_trailing_slash() {
    let running = vec!["/Applications/Cursor.app/".to_string()];
    let apps = detected_to_apps(&running, &installed());
    assert_eq!(apps.len(), 1);
    assert_eq!(apps[0].path, "/Applications/Cursor.app");
}

#[test]
fn detected_to_apps_unique_by_path() {
    let running = vec![
        "/Applications/Cursor.app".to_string(),
        "/Applications/Cursor.app".to_string(),
    ];
    let apps = detected_to_apps(&running, &installed());
    assert_eq!(apps.len(), 1);
}

#[test]
fn detected_to_apps_sorts_by_name_case_insensitive() {
    let running = vec![
        "/Applications/Slack.app".to_string(),
        "/Applications/Firefox.app".to_string(),
    ];
    let apps = detected_to_apps(&running, &installed());
    let names: Vec<&str> = apps.iter().map(|a| a.name.as_str()).collect();
    assert_eq!(names, vec!["Firefox", "Slack"]);
}

#[test]
fn detected_to_apps_empty_running_is_empty() {
    let apps = detected_to_apps(&[], &installed());
    assert!(apps.is_empty());
}

#[test]
fn normalize_bundle_path_strips_single_trailing_slash() {
    use quickdev::capture::normalize_bundle_path;
    assert_eq!(
        normalize_bundle_path("/Applications/Cursor.app/"),
        "/Applications/Cursor.app"
    );
    assert_eq!(
        normalize_bundle_path("/Applications/Cursor.app"),
        "/Applications/Cursor.app"
    );
}

fn app(name: &str, path: &str) -> AppEntry {
    AppEntry {
        name: name.to_string(),
        path: path.to_string(),
        args: None,
    }
}

#[test]
fn merge_apps_skips_already_present_by_path() {
    let existing = vec![app("Cursor", "/Applications/Cursor.app")];
    let captured = vec![
        app("Cursor", "/Applications/Cursor.app"),
        app("Firefox", "/Applications/Firefox.app"),
    ];
    let new = merge_apps(&existing, &captured);
    assert_eq!(new, vec![app("Firefox", "/Applications/Firefox.app")]);
}

#[test]
fn merge_apps_matches_on_path_not_name() {
    // Same path, different display name in config -> still considered present.
    let existing = vec![app("My Editor", "/Applications/Cursor.app")];
    let captured = vec![app("Cursor", "/Applications/Cursor.app")];
    let new = merge_apps(&existing, &captured);
    assert!(new.is_empty());
}

#[test]
fn merge_apps_treats_trailing_slash_as_same_path() {
    // Hand-edited config with a trailing slash must not produce a duplicate.
    let existing = vec![app("Cursor", "/Applications/Cursor.app/")];
    let captured = vec![app("Cursor", "/Applications/Cursor.app")];
    let new = merge_apps(&existing, &captured);
    assert!(new.is_empty());
}

#[test]
fn merge_apps_preserves_captured_order() {
    let existing: Vec<AppEntry> = vec![];
    let captured = vec![
        app("Slack", "/Applications/Slack.app"),
        app("Firefox", "/Applications/Firefox.app"),
    ];
    let new = merge_apps(&existing, &captured);
    let names: Vec<&str> = new.iter().map(|a| a.name.as_str()).collect();
    assert_eq!(names, vec!["Slack", "Firefox"]);
}

#[test]
fn merge_apps_empty_captured_is_empty() {
    let existing = vec![app("Cursor", "/Applications/Cursor.app")];
    let new = merge_apps(&existing, &[]);
    assert!(new.is_empty());
}
