use crate::apps;
use crate::cli::AddKind;
use crate::commands::shared::prompt;
use crate::config::{load_project_config, resolve_project_config, save_project_config};
use crate::fzf;
use crate::models::{AppEntry, ProjectConfig, TerminalEntry};
use crate::parse;
use crate::validate::{validate_app_entry, validate_terminal_entry};
use std::path::PathBuf;

pub(crate) fn cmd_add(kind: Option<AddKind>) -> Result<(), String> {
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    let (config_path, root) = resolve_project_config(&cwd)?;
    let mut config = load_project_config(&config_path)?;

    let announcement = match kind {
        Some(AddKind::Terminal {
            name,
            path,
            command,
            emulator,
        }) => {
            if config.terminals.iter().any(|t| t.name == name) {
                return Err(format!("terminal '{}' already exists", name));
            }
            let entry = TerminalEntry {
                name: name.clone(),
                path,
                command,
                emulator,
            };
            validate_terminal_entry(&entry, &root)?;
            config.terminals.push(entry);
            format!("Added terminal '{name}'")
        }
        Some(AddKind::App { name, path, args }) => {
            if config.applications.iter().any(|a| a.name == name) {
                return Err(format!("application '{}' already exists", name));
            }
            let entry = AppEntry {
                name: name.clone(),
                path,
                args,
            };
            validate_app_entry(&entry)?;
            config.applications.push(entry);
            format!("Added application '{name}'")
        }
        None => {
            return cmd_add_interactive(config_path, root, config);
        }
    };

    // Only after the write succeeds: a save can fail (a concurrent invocation
    // changed the file), and announcing the addition first would print "Added …"
    // immediately above the error explaining that nothing was added.
    save_project_config(&config_path, &config)?;
    println!("{announcement}");
    Ok(())
}

fn cmd_add_interactive(
    config_path: PathBuf,
    root: PathBuf,
    mut config: ProjectConfig,
) -> Result<(), String> {
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

            let entry = TerminalEntry {
                name: name.clone(),
                path,
                command,
                emulator,
            };
            validate_terminal_entry(&entry, &root)?;
            config.terminals.push(entry);
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

            let entry = AppEntry {
                name: app.name.clone(),
                path: app.path,
                args,
            };
            validate_app_entry(&entry)?;
            config.applications.push(entry);
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
        "gnome-terminal".to_string(),
        "ptyxis".to_string(),
        "kitty".to_string(),
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

    // Indexed picker: names and paths come from the filesystem, so a bundle
    // named with a newline or tab could otherwise forge picker rows, and
    // matching the returned text back against the list would fail for any name
    // containing the "  (" separator. `items[0]` is the manual-entry row;
    // `items[i + 1]` is `discovered[i]`.
    let index = fzf::fzf_select_one_indexed(&items, "Select an application:")?;

    if index == 0 {
        return manual_app_entry();
    }

    Ok(discovered
        .into_iter()
        .nth(index - 1)
        .expect("selected index maps to a discovered app"))
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
