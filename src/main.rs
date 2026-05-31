mod adapters;
mod apps;
mod cli;
mod commands;
mod config;
mod fzf;
mod launch;
mod models;
mod parse;

use clap::Parser;
use cli::{Cli, Commands};
use commands::shared::{build_item_display_list, parse_selected_items};
use config::{
    global_config_path, load_global_config, load_project_config, resolve_project_config,
    save_global_config,
};
use launch::LaunchResult;
use models::ProjectConfig;
use std::path::PathBuf;
use std::process;

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Init { from } => commands::cmd_init(from),
        Commands::Launch { project, all } => cmd_launch(project, all),
        Commands::List => cmd_list(),
        Commands::Add { kind } => commands::cmd_add(kind),
        Commands::Remove { kind } => commands::cmd_remove(kind),
        Commands::Edit { global } => cmd_edit(global),
        Commands::Deregister { delete } => cmd_deregister(delete),
    };

    if let Err(e) = result {
        eprintln!("error: {e}");
        process::exit(1);
    }
}

fn cmd_launch(project: Option<String>, all: bool) -> Result<(), String> {
    let global_path = global_config_path();
    let global = load_global_config(&global_path)?;

    let (config, project_root) = match project {
        Some(ref name) => {
            let entry = global
                .projects
                .iter()
                .find(|p| p.name == *name)
                .ok_or_else(|| format!("project '{}' not found in global index", name))?;
            let root = PathBuf::from(&entry.path);
            let config_path = root.join(".quickdev.toml");
            let config = load_project_config(&config_path)?;
            (config, root)
        }
        None => {
            let cwd = std::env::current_dir()
                .map_err(|e| format!("cannot read current directory: {e}"))?;
            let (config_path, root) = resolve_project_config(&cwd)?;
            let config = load_project_config(&config_path)?;
            (config, root)
        }
    };

    if config.terminals.is_empty() && config.applications.is_empty() {
        return Err("no terminals or applications configured".to_string());
    }

    let config = if !all {
        let items = build_item_display_list(&config);
        if items.len() <= 1 {
            config
        } else {
            let selected = fzf::fzf_select_multi(
                &items,
                "Select items to launch (TAB to toggle, ENTER to confirm):",
            )?;
            let (terminal_names, app_names) = parse_selected_items(&selected);
            ProjectConfig {
                project: config.project,
                terminals: config
                    .terminals
                    .into_iter()
                    .filter(|t| terminal_names.contains(&t.name))
                    .collect(),
                applications: config
                    .applications
                    .into_iter()
                    .filter(|a| app_names.contains(&a.name))
                    .collect(),
            }
        }
    } else {
        config
    };

    let results = launch::launch_project(&config, &project_root, global.emulator.as_deref());
    print_launch_summary(&results);

    let any_success = results.iter().any(|r| r.success);
    if !any_success {
        process::exit(1);
    }

    Ok(())
}

fn print_launch_summary(results: &[LaunchResult]) {
    let success_count = results.iter().filter(|r| r.success).count();
    let total = results.len();

    println!("Launched {}/{} items:", success_count, total);
    for r in results {
        if r.success {
            println!("  [ok] {} ({})", r.label, r.kind);
        } else {
            let err = r.error.as_deref().unwrap_or("unknown error");
            println!("  [FAIL] {} — {}", r.label, err);
        }
    }
}

fn cmd_list() -> Result<(), String> {
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

fn cmd_edit(global: bool) -> Result<(), String> {
    let config_path = if global {
        global_config_path()
    } else {
        let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
        let (path, _root) = resolve_project_config(&cwd)?;
        path
    };

    let editor = std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .unwrap_or_else(|_| "vi".to_string());

    let parts = parse::parse_shell_args(&editor)?;
    let (program, leading) = parts.split_first().ok_or("editor command is empty")?;

    std::process::Command::new(program)
        .args(leading)
        .arg(&config_path)
        .status()
        .map_err(|e| format!("failed to open editor '{}': {}", editor, e))?;

    Ok(())
}

fn cmd_deregister(delete: bool) -> Result<(), String> {
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    let (config_path, project_root) = resolve_project_config(&cwd)?;

    let global_path = global_config_path();
    let mut global = load_global_config(&global_path)?;

    let root_str = project_root.to_string_lossy().to_string();
    let before = global.projects.len();
    let removed_name = global
        .projects
        .iter()
        .find(|p| p.path == root_str)
        .map(|p| p.name.clone());
    global.projects.retain(|p| p.path != root_str);

    if global.projects.len() == before {
        return Err("project not found in global index".to_string());
    }

    save_global_config(&global_path, &global)?;

    if delete {
        std::fs::remove_file(&config_path)
            .map_err(|e| format!("failed to delete {}: {e}", config_path.display()))?;
        println!(
            "Deregistered and deleted config for '{}'",
            removed_name.unwrap_or_default()
        );
    } else {
        println!(
            "Deregistered project '{}'",
            removed_name.unwrap_or_default()
        );
    }

    Ok(())
}
