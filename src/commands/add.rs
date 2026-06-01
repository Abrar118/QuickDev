use crate::apps;
use crate::cli::AddKind;
use crate::commands::shared::prompt;
use crate::config::{load_project_config, resolve_project_config, save_project_config};
use crate::fzf;
use crate::models::{AppEntry, ProjectConfig, TerminalEntry};
use crate::parse;
use std::path::PathBuf;

pub(crate) fn cmd_add(kind: Option<AddKind>) -> Result<(), String> {
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
            let app = pick_application()?;

            if config.applications.iter().any(|a| a.name == app.name) {
                return Err(format!("application '{}' already exists", app.name));
            }

            let args_input =
                prompt("Arguments (e.g., \".\" to open project root, Enter to skip): ")?;
            let user_args = if args_input.is_empty() {
                None
            } else {
                Some(parse::parse_shell_args(&args_input)?)
            };

            let args = apps::combine_app_args(app.args, user_args);

            config.applications.push(AppEntry {
                name: app.name.clone(),
                path: app.path,
                args,
            });
            save_project_config(&config_path, &config)?;
            println!("Added application '{}'", app.name);
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

fn pick_application() -> Result<AppEntry, String> {
    let discovered = apps::discover_apps();

    if discovered.is_empty() {
        return manual_app_entry();
    }

    let mut items: Vec<String> = vec!["[Enter path manually]".to_string()];
    for app in &discovered {
        items.push(format!("{}  ({})", app.name, app.path));
    }

    let selected = fzf::fzf_select_one(&items, "Select an application:")?;

    if selected == "[Enter path manually]" {
        return manual_app_entry();
    }

    let app_name = selected
        .split("  (")
        .next()
        .unwrap_or(&selected)
        .to_string();

    discovered
        .into_iter()
        .find(|a| a.name == app_name)
        .ok_or_else(|| format!("app '{}' not found in discovered list", app_name))
}

fn manual_app_entry() -> Result<AppEntry, String> {
    let app_path = prompt("Application path: ")?;
    if app_path.is_empty() {
        return Err("path cannot be empty".to_string());
    }
    let app_name = prompt("Application name: ")?;
    if app_name.is_empty() {
        return Err("name cannot be empty".to_string());
    }
    Ok(AppEntry {
        name: app_name,
        path: app_path,
        args: None,
    })
}
