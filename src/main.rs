mod adapters;
mod apps;
mod config;
mod fzf;
mod launch;
mod models;

use clap::{Parser, Subcommand};
use config::{
    global_config_path, load_global_config, load_project_config, resolve_project_config,
    save_global_config, save_project_config, unique_project_name,
};
use launch::LaunchResult;
use models::{AppEntry, GlobalProjectEntry, ProjectConfig, ProjectEntry, TerminalEntry};
use std::path::PathBuf;
use std::process;

#[derive(Parser)]
#[command(
    name = "quickdev",
    about = "Manage and launch project terminal/app configurations",
    after_help = "\
Examples:
  quickdev init                                         Initialize a project
  quickdev launch                                       Launch current project
  quickdev launch my-api                                Launch a project by name
  quickdev add terminal server . --command \"npm start\"  Add a terminal tab
  quickdev add app Cursor /Applications/Cursor.app      Add an application
  quickdev remove                                       Interactive removal picker"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create .quickdev.toml in the current directory and register the project
    Init,
    /// Launch all terminals and applications for a project
    Launch {
        /// Project name from the global index (omit to use current directory)
        project: Option<String>,
    },
    /// List all indexed projects
    List,
    /// Add a terminal or application entry (interactive if no subcommand given)
    Add {
        #[command(subcommand)]
        kind: Option<AddKind>,
    },
    /// Remove terminals/apps (interactive picker, or specify: remove terminal <name>)
    Remove {
        #[command(subcommand)]
        kind: Option<RemoveKind>,
    },
    /// Open .quickdev.toml in $EDITOR
    Edit,
}

#[derive(Subcommand)]
enum AddKind {
    /// Add a terminal entry
    #[command(after_help = "\
Examples:
  quickdev add terminal server .                        Open shell in project root
  quickdev add terminal dev . --command \"npm run dev\"   Run a command on open
  quickdev add terminal logs ./logs                     Open shell in subdirectory")]
    Terminal {
        /// Name for this terminal tab
        name: String,
        /// Working directory relative to project root
        path: String,
        /// Startup command to run in the terminal
        #[arg(long)]
        command: Option<String>,
        /// Terminal emulator to use (ghostty, terminal). Omit for auto-detect.
        #[arg(long)]
        emulator: Option<String>,
    },
    /// Add an application entry
    #[command(after_help = "\
Examples:
  quickdev add app Cursor /Applications/Cursor.app --args \".\"
  quickdev add app Firefox /usr/bin/firefox")]
    App {
        /// Application display name
        name: String,
        /// Executable or .app bundle path
        path: String,
        /// Arguments passed to the application
        #[arg(long, num_args = 1..)]
        args: Option<Vec<String>>,
    },
}

#[derive(Subcommand)]
enum RemoveKind {
    /// Remove a terminal entry by name
    Terminal { name: String },
    /// Remove an application entry by name
    App { name: String },
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Init => cmd_init(),
        Commands::Launch { project } => cmd_launch(project),
        Commands::List => cmd_list(),
        Commands::Add { kind } => cmd_add(kind),
        Commands::Remove { kind } => cmd_remove(kind),
        Commands::Edit => cmd_edit(),
    };

    if let Err(e) = result {
        eprintln!("error: {e}");
        process::exit(1);
    }
}

fn cmd_init() -> Result<(), String> {
    let cwd = std::env::current_dir().map_err(|e| format!("cannot read current directory: {e}"))?;
    let config_path = cwd.join(".quickdev.toml");

    if config_path.exists() {
        return Err(format!(
            ".quickdev.toml already exists in {}",
            cwd.display()
        ));
    }

    let dir_name = cwd
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "project".to_string());

    let global_path = global_config_path();
    let mut global = load_global_config(&global_path)?;
    let project_name = unique_project_name(&dir_name, &global);

    let project_config = ProjectConfig {
        project: ProjectEntry {
            name: project_name.clone(),
        },
        terminals: vec![],
        applications: vec![],
    };

    save_project_config(&config_path, &project_config)?;

    global.projects.push(GlobalProjectEntry {
        name: project_name.clone(),
        path: cwd.to_string_lossy().to_string(),
    });
    save_global_config(&global_path, &global)?;

    println!(
        "Initialized project '{}' in {}",
        project_name,
        cwd.display()
    );
    println!("Global index updated at {}", global_path.display());
    Ok(())
}

fn cmd_launch(project: Option<String>) -> Result<(), String> {
    let (config, project_root) = match project {
        Some(name) => {
            let global_path = global_config_path();
            let global = load_global_config(&global_path)?;
            let entry = global
                .projects
                .iter()
                .find(|p| p.name == name)
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

    let results = launch::launch_project(&config, &project_root);
    print_launch_summary(&results);

    let any_success = results.iter().any(|r| r.success);
    if !any_success {
        process::exit(1);
    }

    Ok(())
}

fn prompt(message: &str) -> Result<String, String> {
    eprint!("{message}");
    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .map_err(|e| format!("failed to read input: {e}"))?;
    Ok(input.trim().to_string())
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

    println!(
        "{:<20} {:<50} {:>10} {:>6}",
        "Name", "Path", "Terminals", "Apps"
    );
    println!("{}", "-".repeat(90));

    for entry in &global.projects {
        let root = PathBuf::from(&entry.path);
        let config_path = root.join(".quickdev.toml");

        let (terminals, apps, warning) = if config_path.exists() {
            match load_project_config(&config_path) {
                Ok(cfg) => (cfg.terminals.len(), cfg.applications.len(), ""),
                Err(_) => (0, 0, " (config error)"),
            }
        } else {
            (0, 0, " (missing)")
        };

        println!(
            "{:<20} {:<50} {:>10} {:>6}{}",
            entry.name, entry.path, terminals, apps, warning
        );
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

            config.applications.push(AppEntry {
                name: app_name.clone(),
                path: app_path,
                args: None,
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
    let mut items: Vec<String> = Vec::new();

    for t in &config.terminals {
        let cmd_part = t
            .command
            .as_ref()
            .map(|c| format!(" ({c})"))
            .unwrap_or_default();
        items.push(format!("[terminal] {} — {}{}", t.name, t.path, cmd_part));
    }
    for a in &config.applications {
        items.push(format!("[app] {} — {}", a.name, a.path));
    }

    if items.is_empty() {
        return Err("no terminals or applications configured".to_string());
    }

    let selected = fzf::fzf_select_multi(
        &items,
        "Select items to remove (TAB to toggle, ENTER to confirm):",
    )?;

    let mut removed_terminals = Vec::new();
    let mut removed_apps = Vec::new();

    for line in &selected {
        if line.starts_with("[terminal] ") {
            let name = line
                .strip_prefix("[terminal] ")
                .and_then(|s| s.split(" — ").next())
                .unwrap_or("")
                .to_string();
            removed_terminals.push(name);
        } else if line.starts_with("[app] ") {
            let name = line
                .strip_prefix("[app] ")
                .and_then(|s| s.split(" — ").next())
                .unwrap_or("")
                .to_string();
            removed_apps.push(name);
        }
    }

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

fn cmd_edit() -> Result<(), String> {
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    let (config_path, _root) = resolve_project_config(&cwd)?;

    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());

    std::process::Command::new(&editor)
        .arg(&config_path)
        .status()
        .map_err(|e| format!("failed to open editor '{}': {}", editor, e))?;

    Ok(())
}
