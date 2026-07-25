use crate::cli::RemoveKind;
use crate::commands::shared::{build_item_display_list, selected_items};
use crate::config::{load_project_config, resolve_project_config, save_project_config};
use crate::fzf;
use crate::models::ProjectConfig;
use std::path::PathBuf;

pub(crate) fn cmd_remove(kind: Option<RemoveKind>) -> Result<(), String> {
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    let (config_path, _root) = resolve_project_config(&cwd)?;
    let mut config = load_project_config(&config_path)?;

    let announcement = match kind {
        Some(RemoveKind::Terminal { name }) => {
            let before = config.terminals.len();
            config.terminals.retain(|t| t.name != name);
            if config.terminals.len() == before {
                return Err(format!("terminal '{}' not found", name));
            }
            format!("Removed terminal '{name}'")
        }
        Some(RemoveKind::App { name }) => {
            let before = config.applications.len();
            config.applications.retain(|a| a.name != name);
            if config.applications.len() == before {
                return Err(format!("application '{}' not found", name));
            }
            format!("Removed application '{name}'")
        }
        None => {
            return cmd_remove_interactive(config_path, config);
        }
    };

    // Announce only once the write succeeded — see the note in `add`.
    save_project_config(&config_path, &config)?;
    println!("{announcement}");
    Ok(())
}

fn cmd_remove_interactive(config_path: PathBuf, mut config: ProjectConfig) -> Result<(), String> {
    let items = build_item_display_list(&config);

    if items.is_empty() {
        return Err("no terminals or applications configured".to_string());
    }

    let picked = fzf::fzf_select_multi_indexed(
        &items,
        "Select items to remove (TAB to toggle, ENTER to confirm):",
    )?;

    let (terminal_indices, app_indices) = selected_items(&config, &picked);

    let removed_terminals: Vec<String> = terminal_indices
        .iter()
        .map(|&i| config.terminals[i].name.clone())
        .collect();
    let removed_apps: Vec<String> = app_indices
        .iter()
        .map(|&i| config.applications[i].name.clone())
        .collect();

    let mut index = 0;
    config.terminals.retain(|_| {
        let keep = !terminal_indices.contains(&index);
        index += 1;
        keep
    });
    let mut index = 0;
    config.applications.retain(|_| {
        let keep = !app_indices.contains(&index);
        index += 1;
        keep
    });

    save_project_config(&config_path, &config)?;

    for name in &removed_terminals {
        println!("Removed terminal '{name}'");
    }
    for name in &removed_apps {
        println!("Removed application '{name}'");
    }

    Ok(())
}
