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
use config::{global_config_path, load_global_config, resolve_project_config, save_global_config};
use std::process;

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Init { from } => commands::cmd_init(from),
        Commands::Launch { project, all } => commands::cmd_launch(project, all),
        Commands::List => commands::cmd_list(),
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
