use crate::config::{find_project_config, load_project_config, resolve_project_config};
use crate::ui;
use crate::validate::{validate_project_config, ValidationReport};
use anstream::println;
use std::env;

pub(crate) fn cmd_validate(json: bool) -> Result<(), String> {
    let cwd =
        env::current_dir().map_err(|e| format!("could not determine current directory: {e}"))?;
    // JSON output is for scripts and CI: never open the interactive project
    // picker, and report a missing config in the JSON itself.
    let located = if json {
        find_project_config(&cwd)
    } else {
        resolve_project_config(&cwd)
    };
    let (config_path, project_root) = match located {
        Ok(found) => found,
        Err(e) if json => {
            let report = ValidationReport {
                errors: vec![e.clone()],
                warnings: vec![],
            };
            println!("{}", report.to_json(None));
            return Err(e);
        }
        Err(e) => return Err(e),
    };
    // A config that can't be loaded is the most basic validation failure:
    // report it like any other error, so `--json` consumers still get a report.
    let report = match load_project_config(&config_path) {
        Ok(config) => validate_project_config(&config, &project_root),
        Err(e) => ValidationReport {
            errors: vec![e],
            warnings: vec![],
        },
    };
    let result = if report.is_ok() {
        Ok(())
    } else {
        Err(format!(
            "{} has {} error(s)",
            config_path.display(),
            report.errors.len()
        ))
    };

    if json {
        println!("{}", report.to_json(Some(&config_path.to_string_lossy())));
        return result;
    }

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
    }
    result
}
