mod adapters;
mod apps;
mod config;
mod fzf;
mod launch;
mod models;
mod parse;

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
  quickdev init --from my-api                           Clone config from another project
  quickdev launch                                       Select items to launch
  quickdev launch --all                                 Launch everything
  quickdev launch my-api                                Launch a project by name
  quickdev add                                          Interactive add
  quickdev remove                                       Interactive removal picker
  quickdev list                                         Show all projects
  quickdev edit                                         Edit project config
  quickdev edit --global                                Edit global config
  quickdev deregister                                   Unregister project"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create .quickdev.toml in the current directory and register the project
    Init {
        /// Clone config from another project by name
        #[arg(long)]
        from: Option<String>,
    },
    /// Launch terminals and applications for a project
    Launch {
        /// Project name from the global index (omit to use current directory)
        project: Option<String>,
        /// Launch all items without interactive selection
        #[arg(long)]
        all: bool,
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
    /// Open .quickdev.toml in $EDITOR (or --global for global config)
    Edit {
        /// Edit global config instead of project config
        #[arg(long)]
        global: bool,
    },
    /// Remove current project from global index
    Deregister {
        /// Also delete the .quickdev.toml file
        #[arg(long)]
        delete: bool,
    },
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
        Commands::Init { from } => cmd_init(from),
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

fn cmd_init(from: Option<String>) -> Result<(), String> {
    let cwd = std::env::current_dir().map_err(|e| format!("cannot read current directory: {e}"))?;
    let config_path = cwd.join(".quickdev.toml");

    let dir_name = cwd
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "project".to_string());

    let global_path = global_config_path();
    let mut global = load_global_config(&global_path)?;

    let cwd_str = cwd.to_string_lossy().to_string();
    let already_indexed = global.projects.iter().any(|p| p.path == cwd_str);

    if config_path.exists() && already_indexed {
        return Err(format!(
            ".quickdev.toml already exists and is registered in {}",
            cwd.display()
        ));
    }

    if config_path.exists() && !already_indexed {
        let existing = load_project_config(&config_path)?;
        let project_name = unique_project_name(&existing.project.name, &global);
        global.projects.push(GlobalProjectEntry {
            name: project_name.clone(),
            path: cwd_str,
        });
        save_global_config(&global_path, &global)?;
        println!("Re-registered project '{}' in global index", project_name);
        return Ok(());
    }

    let project_name = unique_project_name(&dir_name, &global);

    let project_config = match from {
        Some(ref source_name) => {
            let source_entry = global
                .projects
                .iter()
                .find(|p| p.name == *source_name)
                .ok_or_else(|| {
                    format!("source project '{}' not found in global index", source_name)
                })?;
            let source_path = PathBuf::from(&source_entry.path).join(".quickdev.toml");
            let source = load_project_config(&source_path)?;
            ProjectConfig {
                project: ProjectEntry {
                    name: project_name.clone(),
                },
                terminals: source.terminals,
                applications: source.applications,
            }
        }
        None => ProjectConfig {
            project: ProjectEntry {
                name: project_name.clone(),
            },
            terminals: vec![],
            applications: vec![],
        },
    };

    save_project_config(&config_path, &project_config)?;

    global.projects.push(GlobalProjectEntry {
        name: project_name.clone(),
        path: cwd_str,
    });
    save_global_config(&global_path, &global)?;

    if from.is_some() {
        println!(
            "Initialized project '{}' from template in {}",
            project_name,
            cwd.display()
        );
    } else {
        println!(
            "Initialized project '{}' in {}",
            project_name,
            cwd.display()
        );
    }
    println!("Global index updated at {}", global_path.display());
    Ok(())
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

    let config = if !all && project.is_none() {
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

fn build_item_display_list(config: &ProjectConfig) -> Vec<String> {
    let mut items = Vec::new();
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
    items
}

fn parse_selected_items(selected: &[String]) -> (Vec<String>, Vec<String>) {
    let mut terminal_names = Vec::new();
    let mut app_names = Vec::new();

    for line in selected {
        if line.starts_with("[terminal] ") {
            let name = line
                .strip_prefix("[terminal] ")
                .and_then(|s| s.split(" — ").next())
                .unwrap_or("")
                .to_string();
            terminal_names.push(name);
        } else if line.starts_with("[app] ") {
            let name = line
                .strip_prefix("[app] ")
                .and_then(|s| s.split(" — ").next())
                .unwrap_or("")
                .to_string();
            app_names.push(name);
        }
    }

    (terminal_names, app_names)
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
