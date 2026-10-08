use crate::config::ProjectStatus;
use crate::ui::{self, BOLD, DIM, RED};
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
    let _ = writeln!(out, "{}\n", ui::paint(BOLD, "QuickDev doctor"));

    let _ = writeln!(
        out,
        "  {}",
        if report.global_config_ok {
            ui::ok("global config OK")
        } else {
            ui::fail("global config missing or unparseable")
        }
    );
    let _ = writeln!(
        out,
        "  {}",
        if report.fzf_available {
            ui::ok("fzf available")
        } else {
            ui::warn(format!(
                "fzf not found {}",
                ui::paint(DIM, "(interactive selection disabled)")
            ))
        }
    );

    if report.projects.is_empty() {
        let _ = writeln!(out, "  {}", ui::paint(DIM, "· no projects registered"));
        return out;
    }

    let _ = writeln!(
        out,
        "\n{}\n",
        ui::heading("Projects", &report.projects.len().to_string())
    );
    let name_width = report
        .projects
        .iter()
        .map(|p| p.name.chars().count())
        .max()
        .unwrap_or(0);
    for p in &report.projects {
        let path = ui::paint(DIM, ui::tilde(&p.path));
        let row = match p.issue() {
            None => ui::ok(format!("{}  {path}", ui::pad(BOLD, &p.name, name_width))),
            Some(issue) => ui::fail(format!(
                "{}  {path}  {}",
                ui::pad(BOLD, &p.name, name_width),
                ui::paint(RED, issue)
            )),
        };
        let _ = writeln!(out, "  {row}");
    }

    out
}
