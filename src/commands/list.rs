use crate::config::{
    global_config_path, load_global_config, load_project_config, missing_statuses,
    project_statuses, projects_json,
};
use std::path::PathBuf;

pub(crate) fn cmd_list(missing: bool, json: bool) -> Result<(), String> {
    let global_path = global_config_path();
    let global = load_global_config(&global_path)?;
    let statuses = project_statuses(&global);

    let selected: Vec<_> = if missing {
        missing_statuses(&statuses)
    } else {
        statuses.iter().collect()
    };

    if json {
        let owned: Vec<_> = selected.iter().map(|s| (*s).clone()).collect();
        println!("{}", projects_json(&owned));
        return Ok(());
    }

    if global.projects.is_empty() {
        println!("No projects registered. Run 'quickdev init' in a project directory.");
        return Ok(());
    }

    if missing && selected.is_empty() {
        println!("All registered projects are healthy.");
        return Ok(());
    }

    println!("Projects:");

    for status in &selected {
        if let Some(issue) = status.issue() {
            println!("  {}    {}  ({issue})", status.name, status.path);
            println!();
            continue;
        }

        let config_path = PathBuf::from(&status.path).join(".quickdev.toml");
        let cfg = match load_project_config(&config_path) {
            Ok(cfg) => cfg,
            Err(_) => {
                println!("  {}    {}  (config error)", status.name, status.path);
                println!();
                continue;
            }
        };

        println!("  {}    {}", status.name, status.path);

        if !cfg.terminals.is_empty() {
            let names: Vec<&str> = cfg.terminals.iter().map(|t| t.name.as_str()).collect();
            println!("    Terminals: {}", names.join(", "));
        }

        if !cfg.applications.is_empty() {
            let names: Vec<&str> = cfg.applications.iter().map(|a| a.name.as_str()).collect();
            println!("    Apps: {}", names.join(", "));
        }

        println!();
    }

    Ok(())
}
