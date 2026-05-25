mod adapters;
mod config;
mod launch;
mod models;

use clap::{Parser, Subcommand};
use config::{find_project_config, global_config_path, load_global_config, load_project_config, save_global_config, save_project_config, unique_project_name};
use launch::LaunchResult;
use models::{AppEntry, GlobalProjectEntry, ProjectConfig, ProjectEntry, TerminalEntry};
use std::path::PathBuf;
use std::process;

#[derive(Parser)]
#[command(name = "quickdev", about = "Manage and launch project terminal/app configurations")]
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
    /// Add a terminal or application entry
    Add {
        #[command(subcommand)]
        kind: AddKind,
    },
    /// Remove a terminal or application entry
    Remove {
        #[command(subcommand)]
        kind: RemoveKind,
    },
    /// Open .quickdev.toml in $EDITOR
    Edit,
}

#[derive(Subcommand)]
enum AddKind {
    /// Add a terminal entry
    Terminal {
        /// Name for this terminal tab
        name: String,
        /// Working directory relative to project root
        path: String,
        /// Startup command to run in the terminal
        #[arg(long)]
        command: Option<String>,
    },
    /// Add an application entry
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
        return Err(format!(".quickdev.toml already exists in {}", cwd.display()));
    }

    let dir_name = cwd
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "project".to_string());

    let global_path = global_config_path();
    let mut global = load_global_config(&global_path)?;
    let project_name = unique_project_name(&dir_name, &global);

    let project_config = ProjectConfig {
        project: ProjectEntry { name: project_name.clone() },
        terminals: vec![],
        applications: vec![],
    };

    save_project_config(&config_path, &project_config)?;

    global.projects.push(GlobalProjectEntry {
        name: project_name.clone(),
        path: cwd.to_string_lossy().to_string(),
    });
    save_global_config(&global_path, &global)?;

    println!("Initialized project '{}' in {}", project_name, cwd.display());
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
            let cwd = std::env::current_dir().map_err(|e| format!("cannot read current directory: {e}"))?;
            let (config_path, root) = find_project_config(&cwd)?;
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

    println!("{:<20} {:<50} {:>10} {:>6}", "Name", "Path", "Terminals", "Apps");
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

fn cmd_add(kind: AddKind) -> Result<(), String> {
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    let (config_path, _root) = find_project_config(&cwd)?;
    let mut config = load_project_config(&config_path)?;

    match kind {
        AddKind::Terminal { name, path, command } => {
            if config.terminals.iter().any(|t| t.name == name) {
                return Err(format!("terminal '{}' already exists", name));
            }
            config.terminals.push(TerminalEntry { name: name.clone(), path, command });
            println!("Added terminal '{}'", name);
        }
        AddKind::App { name, path, args } => {
            if config.applications.iter().any(|a| a.name == name) {
                return Err(format!("application '{}' already exists", name));
            }
            config.applications.push(AppEntry { name: name.clone(), path, args });
            println!("Added application '{}'", name);
        }
    }

    save_project_config(&config_path, &config)
}

fn cmd_remove(kind: RemoveKind) -> Result<(), String> {
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    let (config_path, _root) = find_project_config(&cwd)?;
    let mut config = load_project_config(&config_path)?;

    match kind {
        RemoveKind::Terminal { name } => {
            let before = config.terminals.len();
            config.terminals.retain(|t| t.name != name);
            if config.terminals.len() == before {
                return Err(format!("terminal '{}' not found", name));
            }
            println!("Removed terminal '{}'", name);
        }
        RemoveKind::App { name } => {
            let before = config.applications.len();
            config.applications.retain(|a| a.name != name);
            if config.applications.len() == before {
                return Err(format!("application '{}' not found", name));
            }
            println!("Removed application '{}'", name);
        }
    }

    save_project_config(&config_path, &config)
}

fn cmd_edit() -> Result<(), String> {
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    let (config_path, _root) = find_project_config(&cwd)?;

    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());

    std::process::Command::new(&editor)
        .arg(&config_path)
        .status()
        .map_err(|e| format!("failed to open editor '{}': {}", editor, e))?;

    Ok(())
}
