use quickdev::config::{projects_json, ProjectStatus};

fn status(name: &str, path_exists: bool, config_exists: bool) -> ProjectStatus {
    ProjectStatus {
        name: name.to_string(),
        path: format!("/p/{name}"),
        path_exists,
        config_exists,
    }
}

#[test]
fn projects_json_contains_expected_fields() {
    let json = projects_json(&[status("api", true, true)]);
    assert!(json.contains("\"name\": \"api\""));
    assert!(json.contains("\"path\": \"/p/api\""));
    assert!(json.contains("\"healthy\": true"));
    assert!(json.contains("\"path_exists\": true"));
    assert!(json.contains("\"config_exists\": true"));
}

#[test]
fn projects_json_marks_unhealthy() {
    let json = projects_json(&[status("gone", false, false)]);
    assert!(json.contains("\"healthy\": false"));
    assert!(json.contains("\"path_exists\": false"));
}

#[test]
fn projects_json_empty_is_array() {
    assert_eq!(projects_json(&[]).trim(), "[]");
}

#[test]
fn projects_json_escapes_double_quotes() {
    let json = projects_json(&[status("a\"b", true, true)]);
    assert!(
        json.contains("a\\\"b"),
        "quotes must be escaped, got: {json}"
    );
}
