use crate::config::ProjectStatus;
use std::fmt::Write;

/// Snapshot of global environment health for `quickdev doctor`.
pub struct DoctorReport {
    pub global_config_ok: bool,
    pub fzf_available: bool,
    pub projects: Vec<ProjectStatus>,
}

/// True when doctor should exit non-zero: a bad global config or any unhealthy project.
/// A missing `fzf` is a warning, not an error.
pub fn doctor_has_errors(report: &DoctorReport) -> bool {
    !report.global_config_ok || report.projects.iter().any(|p| !p.is_healthy())
}

pub fn render_doctor(report: &DoctorReport) -> String {
    let mut out = String::new();
    out.push_str("QuickDev doctor\n");

    if report.global_config_ok {
        let _ = writeln!(out, "  ✓ global config OK");
    } else {
        let _ = writeln!(out, "  ✗ global config missing or unparseable");
    }

    if report.fzf_available {
        let _ = writeln!(out, "  ✓ fzf available");
    } else {
        let _ = writeln!(out, "  ⚠ fzf not found (interactive selection disabled)");
    }

    if report.projects.is_empty() {
        let _ = writeln!(out, "  · no projects registered");
    } else {
        out.push_str("  Projects:\n");
        for p in &report.projects {
            match p.issue() {
                None => {
                    let _ = writeln!(out, "    ✓ {} ({})", p.name, p.path);
                }
                Some(issue) => {
                    let _ = writeln!(out, "    ✗ {} ({}) — {}", p.name, p.path, issue);
                }
            }
        }
    }

    out
}
