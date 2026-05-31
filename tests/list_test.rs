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
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(v[0]["name"], "api");
    assert_eq!(v[0]["path"], "/p/api");
    assert_eq!(v[0]["healthy"], true);
    assert_eq!(v[0]["path_exists"], true);
    assert_eq!(v[0]["config_exists"], true);
}

#[test]
fn projects_json_marks_unhealthy() {
    let json = projects_json(&[status("gone", false, false)]);
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(v[0]["healthy"], false);
    assert_eq!(v[0]["path_exists"], false);
}

#[test]
fn projects_json_empty_is_array() {
    assert_eq!(projects_json(&[]).trim(), "[]");
}
