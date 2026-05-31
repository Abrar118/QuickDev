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
use cli::{AddKind, Cli, Commands, RemoveKind};
use commands::shared::{build_item_display_list, parse_selected_items, prompt};
use config::{
    global_config_path, load_global_config, load_project_config, resolve_project_config,
    save_global_config, save_project_config,
};
use launch::LaunchResult;
use models::{AppEntry, ProjectConfig, TerminalEntry};
use std::path::PathBuf;
use std::process;

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Init { from } => commands::cmd_init(from),
        Commands::Launch { project, all } => cmd_launch(project, all),
        Commands::List => cmd_list(),
        Commands::Add { kind } => cmd_add(kind),
        Commands::Remove { kind } => cmd_remove(kind),
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

fn cmd_add(kind: Option<AddKind>) -> Result<(), String> {
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    let (config_path, _root) = resolve_project_config(&cwd)?;
    let mut config = load_project_config(&config_path)?;

    match kind {
        Some(AddKind::Terminal {
            name,
            path,
            command,
            emulator,
        }) => {
            if config.terminals.iter().any(|t| t.name == name) {
                return Err(format!("terminal '{}' already exists", name));
            }
            config.terminals.push(TerminalEntry {
                name: name.clone(),
                path,
                command,
                emulator,
            });
            println!("Added terminal '{}'", name);
        }
        Some(AddKind::App { name, path, args }) => {
            if config.applications.iter().any(|a| a.name == name) {
                return Err(format!("application '{}' already exists", name));
            }
            config.applications.push(AppEntry {
                name: name.clone(),
                path,
                args,
            });
            println!("Added application '{}'", name);
        }
        None => {
            return cmd_add_interactive(config_path, config);
        }
    }

    save_project_config(&config_path, &config)
}

fn cmd_add_interactive(config_path: PathBuf, mut config: ProjectConfig) -> Result<(), String> {
    let types = vec!["Terminal".to_string(), "Application".to_string()];
    let selected = fzf::fzf_select_one(&types, "Select what to add:")?;

    match selected.as_str() {
        "Terminal" => {
            let path = prompt("Path (. for current directory): ")?;
            let path = if path.is_empty() {
                ".".to_string()
            } else {
                path
            };

            let name = prompt("Name for this tab: ")?;
            if name.is_empty() {
                return Err("name cannot be empty".to_string());
            }
            if config.terminals.iter().any(|t| t.name == name) {
                return Err(format!("terminal '{}' already exists", name));
            }

            let command_input = prompt("Startup command (optional, press Enter to skip): ")?;
            let command = if command_input.is_empty() {
                None
            } else {
                Some(command_input)
            };

            let emulator = pick_emulator()?;

            config.terminals.push(TerminalEntry {
                name: name.clone(),
                path,
                command,
                emulator,
            });
            save_project_config(&config_path, &config)?;
            println!("Added terminal '{name}'");
        }
        "Application" => {
            let (app_name, app_path) = pick_application()?;

            if config.applications.iter().any(|a| a.name == app_name) {
                return Err(format!("application '{}' already exists", app_name));
            }

            let args_input =
                prompt("Arguments (e.g., \".\" to open project root, Enter to skip): ")?;
            let args = if args_input.is_empty() {
                None
            } else {
                Some(parse::parse_shell_args(&args_input)?)
            };

            config.applications.push(AppEntry {
                name: app_name.clone(),
                path: app_path,
                args,
            });
            save_project_config(&config_path, &config)?;
            println!("Added application '{app_name}'");
        }
        _ => return Err("invalid selection".to_string()),
    }

    Ok(())
}

fn pick_emulator() -> Result<Option<String>, String> {
    let options = vec![
        "Auto-detect (default)".to_string(),
        "ghostty".to_string(),
        "terminal".to_string(),
    ];
    let selected = fzf::fzf_select_one(&options, "Select terminal emulator:")?;

    match selected.as_str() {
        "Auto-detect (default)" => Ok(None),
        other => Ok(Some(other.to_string())),
    }
}

fn pick_application() -> Result<(String, String), String> {
    let discovered = apps::discover_apps();

    if discovered.is_empty() {
        let app_path = prompt("Application path: ")?;
        if app_path.is_empty() {
            return Err("path cannot be empty".to_string());
        }
        let app_name = prompt("Application name: ")?;
        if app_name.is_empty() {
            return Err("name cannot be empty".to_string());
        }
        return Ok((app_name, app_path));
    }

    let mut items: Vec<String> = vec!["[Enter path manually]".to_string()];
    for (name, path) in &discovered {
        items.push(format!("{name}  ({path})"));
    }

    let selected = fzf::fzf_select_one(&items, "Select an application:")?;

    if selected == "[Enter path manually]" {
        let app_path = prompt("Application path: ")?;
        if app_path.is_empty() {
            return Err("path cannot be empty".to_string());
        }
        let app_name = prompt("Application name: ")?;
        if app_name.is_empty() {
            return Err("name cannot be empty".to_string());
        }
        return Ok((app_name, app_path));
    }

    let app_name = selected
        .split("  (")
        .next()
        .unwrap_or(&selected)
        .to_string();

    let entry = discovered
        .iter()
        .find(|(name, _)| *name == app_name)
        .ok_or_else(|| format!("app '{}' not found in discovered list", app_name))?;

    Ok((entry.0.clone(), entry.1.clone()))
}

fn cmd_remove(kind: Option<RemoveKind>) -> Result<(), String> {
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    let (config_path, _root) = resolve_project_config(&cwd)?;
    let mut config = load_project_config(&config_path)?;

    match kind {
        Some(RemoveKind::Terminal { name }) => {
            let before = config.terminals.len();
            config.terminals.retain(|t| t.name != name);
            if config.terminals.len() == before {
                return Err(format!("terminal '{}' not found", name));
            }
            println!("Removed terminal '{}'", name);
        }
        Some(RemoveKind::App { name }) => {
            let before = config.applications.len();
            config.applications.retain(|a| a.name != name);
            if config.applications.len() == before {
                return Err(format!("application '{}' not found", name));
            }
            println!("Removed application '{}'", name);
        }
        None => {
            return cmd_remove_interactive(config_path, config);
        }
    }

    save_project_config(&config_path, &config)
}

fn cmd_remove_interactive(config_path: PathBuf, mut config: ProjectConfig) -> Result<(), String> {
    let items = build_item_display_list(&config);

    if items.is_empty() {
        return Err("no terminals or applications configured".to_string());
    }

    let selected = fzf::fzf_select_multi(
        &items,
        "Select items to remove (TAB to toggle, ENTER to confirm):",
    )?;

    let (removed_terminals, removed_apps) = parse_selected_items(&selected);

    config
        .terminals
        .retain(|t| !removed_terminals.contains(&t.name));
    config
        .applications
        .retain(|a| !removed_apps.contains(&a.name));

    save_project_config(&config_path, &config)?;

    for name in &removed_terminals {
        println!("Removed terminal '{name}'");
    }
    for name in &removed_apps {
        println!("Removed application '{name}'");
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
