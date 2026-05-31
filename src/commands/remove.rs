use crate::cli::RemoveKind;
use crate::commands::shared::{build_item_display_list, parse_selected_items};
use crate::config::{load_project_config, resolve_project_config, save_project_config};
use crate::fzf;
use crate::models::ProjectConfig;
use std::path::PathBuf;

pub(crate) fn cmd_remove(kind: Option<RemoveKind>) -> Result<(), String> {
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
