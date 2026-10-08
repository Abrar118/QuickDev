use crate::config::{load_project_config, resolve_project_config};
use crate::ui;
use crate::validate::validate_project_config;
use anstream::println;
use std::env;

pub(crate) fn cmd_validate() -> Result<(), String> {
    let cwd =
        env::current_dir().map_err(|e| format!("could not determine current directory: {e}"))?;
    let (config_path, project_root) = resolve_project_config(&cwd)?;
    let config = load_project_config(&config_path)?;

    let report = validate_project_config(&config, &project_root);

    for err in &report.errors {
        println!("{}", ui::fail(err));
    }
    for warn in &report.warnings {
        println!("{}", ui::warn(warn));
    }

    if report.is_ok() {
        let path = ui::tilde(&config_path.to_string_lossy());
        if report.warnings.is_empty() {
            println!("{}", ui::ok(format!("{path} is valid")));
        } else {
            println!(
                "{}",
                ui::ok(format!(
                    "{path} is valid {}",
                    ui::paint(
                        ui::YELLOW,
                        format!("({})", ui::count(report.warnings.len(), "warning"))
                    )
                ))
            );
        }
        Ok(())
    } else {
        Err(format!(
            "{} has {} error(s)",
            config_path.display(),
            report.errors.len()
        ))
    }
}
