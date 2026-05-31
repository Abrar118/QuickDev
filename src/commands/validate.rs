use crate::config::{load_project_config, resolve_project_config};
use crate::validate::validate_project_config;
use std::env;

pub(crate) fn cmd_validate() -> Result<(), String> {
    let cwd =
        env::current_dir().map_err(|e| format!("could not determine current directory: {e}"))?;
    let (config_path, project_root) = resolve_project_config(&cwd)?;
    let config = load_project_config(&config_path)?;

    let report = validate_project_config(&config, &project_root);

    for err in &report.errors {
        println!("✗ {err}");
    }
    for warn in &report.warnings {
        println!("⚠ {warn}");
    }

    if report.is_ok() {
        if report.warnings.is_empty() {
            println!("✓ {} is valid", config_path.display());
        } else {
            println!(
                "✓ {} is valid ({} warning(s))",
                config_path.display(),
                report.warnings.len()
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
