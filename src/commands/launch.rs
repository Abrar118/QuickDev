use crate::adapters::command_exists;
use crate::commands::shared::{build_item_display_list, parse_selected_items};
use crate::config::{
    global_config_path, load_global_config, load_project_config, resolve_project_config,
    save_global_config,
};
use crate::fzf;
use crate::launch::{launch_project, plan_launch, render_results};
use crate::models::ProjectConfig;
use crate::terminal_app::{
    prompt_to_enable_terminal_tabbing, read_tabbing_preference, PromptOutcome, TabbingPreference,
};
use std::path::PathBuf;
use std::process;

pub(crate) fn cmd_launch(project: Option<String>, all: bool, dry_run: bool) -> Result<(), String> {
    let global_path = global_config_path();
    let mut global = load_global_config(&global_path)?;

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

    if dry_run {
        let plan = plan_launch(&config, &project_root);
        let would = plan.iter().filter(|r| r.success).count();
        print!(
            "{}",
            render_results(&format!("Would launch {would} items:"), &plan)
        );
        return Ok(());
    }

    if should_prompt_for_terminal_app_tabbing(&config, global.emulator.as_deref()) {
        match prompt_to_enable_terminal_tabbing(global.terminal_app_tabbing_prompt_declined) {
            PromptOutcome::Declined => {
                global.terminal_app_tabbing_prompt_declined = true;
                save_global_config(&global_path, &global)?;
            }
            PromptOutcome::WriteFailed(e) => eprintln!("{e}"),
            PromptOutcome::Accepted | PromptOutcome::NoChange => {}
        }
    }

    let results = launch_project(&config, &project_root, global.emulator.as_deref());
    let success = results.iter().filter(|r| r.success).count();
    print!(
        "{}",
        render_results(
            &format!("Launched {success}/{} items:", results.len()),
            &results
        )
    );

    let any_success = results.iter().any(|r| r.success);
    if !any_success {
        process::exit(1);
    }

    Ok(())
}

fn should_prompt_for_terminal_app_tabbing(
    config: &ProjectConfig,
    global_emulator: Option<&str>,
) -> bool {
    if config.terminals.len() <= 1 || read_tabbing_preference() == Some(TabbingPreference::Always) {
        return false;
    }

    config.terminals.iter().any(|terminal| {
        let effective = terminal.emulator.as_deref().or(global_emulator);
        matches!(effective, Some("terminal")) || (effective.is_none() && !command_exists("ghostty"))
    })
}
