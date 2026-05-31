use quickdev::config::ProjectStatus;
use quickdev::doctor::{doctor_has_errors, render_doctor, DoctorReport};

fn status(name: &str, healthy: bool) -> ProjectStatus {
    ProjectStatus {
        name: name.to_string(),
        path: format!("/p/{name}"),
        path_exists: healthy,
        config_exists: healthy,
    }
}

#[test]
fn all_healthy_has_no_errors_and_no_cross_mark() {
    let report = DoctorReport {
        global_config_ok: true,
        fzf_available: true,
        projects: vec![status("api", true), status("web", true)],
    };
    assert!(!doctor_has_errors(&report));
    let out = render_doctor(&report);
    assert!(!out.contains('✗'), "no ✗ expected, got:\n{out}");
    assert!(out.contains("✓ api"));
}

#[test]
fn unhealthy_project_is_an_error() {
    let report = DoctorReport {
        global_config_ok: true,
        fzf_available: true,
        projects: vec![status("api", true), status("dead", false)],
    };
    assert!(doctor_has_errors(&report));
    let out = render_doctor(&report);
    assert!(out.contains("✗ dead"));
}

#[test]
fn missing_fzf_is_warning_not_error() {
    let report = DoctorReport {
        global_config_ok: true,
        fzf_available: false,
        projects: vec![status("api", true)],
    };
    assert!(!doctor_has_errors(&report));
    let out = render_doctor(&report);
    assert!(out.contains("⚠ fzf not found"));
}

#[test]
fn bad_global_config_is_error() {
    let report = DoctorReport {
        global_config_ok: false,
        fzf_available: true,
        projects: vec![],
    };
    assert!(doctor_has_errors(&report));
    let out = render_doctor(&report);
    assert!(out.contains("✗ global config"));
}
