use crate::config::{global_config_path, load_global_config, load_project_config};
use std::path::PathBuf;

pub(crate) fn cmd_list() -> Result<(), String> {
    let global_path = global_config_path();
    let global = load_global_config(&global_path)?;

    if global.projects.is_empty() {
        println!("No projects registered. Run 'quickdev init' in a project directory.");
        return Ok(());
    }

    println!("Projects:");

    for entry in &global.projects {
        let root = PathBuf::from(&entry.path);
        let config_path = root.join(".quickdev.toml");

        if !config_path.exists() {
            println!("  {}    {}  (missing)", entry.name, entry.path);
            println!();
            continue;
        }

        let cfg = match load_project_config(&config_path) {
            Ok(cfg) => cfg,
            Err(_) => {
                println!("  {}    {}  (config error)", entry.name, entry.path);
                println!();
                continue;
            }
        };

        println!("  {}    {}", entry.name, entry.path);

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
