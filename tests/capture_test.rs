use quickdev::capture::detected_to_apps;
use quickdev::models::AppEntry;

fn installed() -> Vec<(String, String)> {
    vec![
        ("Cursor".to_string(), "/Applications/Cursor.app".to_string()),
        ("Firefox".to_string(), "/Applications/Firefox.app".to_string()),
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
    assert_eq!(normalize_bundle_path("/Applications/Cursor.app/"), "/Applications/Cursor.app");
    assert_eq!(normalize_bundle_path("/Applications/Cursor.app"), "/Applications/Cursor.app");
}
