use crate::config::{
    global_config_path, load_global_config, load_project_config, register_existing_project_config,
    save_global_config, save_project_config, unique_project_name,
};
use crate::models::{GlobalProjectEntry, ProjectConfig, ProjectEntry};
use crate::ui;
use anstream::println;
use std::path::PathBuf;

pub(crate) fn cmd_init(from: Option<String>) -> Result<(), String> {
    let cwd = std::env::current_dir().map_err(|e| format!("cannot read current directory: {e}"))?;
    let config_path = cwd.join(".quickdev.toml");

    let dir_name = cwd
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "project".to_string());

    let global_path = global_config_path()?;
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
        let project_name = register_existing_project_config(&config_path, cwd_str, &mut global)?;
        save_global_config(&global_path, &global)?;
        println!(
            "{}",
            ui::ok(format!(
                "Re-registered project {} in the global index",
                ui::paint(ui::BOLD, &project_name)
            ))
        );
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
                    ui::with_suggestion(
                        format!("source project '{source_name}' not found in global index"),
                        ui::closest(source_name, global.projects.iter().map(|p| p.name.as_str())),
                    )
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
        last_launched: None,
    });
    if let Err(e) = save_global_config(&global_path, &global) {
        // Roll back the config we just created rather than leaving an
        // unregistered .quickdev.toml that a retried `init` would refuse.
        let _ = std::fs::remove_file(&config_path);
        return Err(e);
    }

    let template = if from.is_some() { " from template" } else { "" };
    println!(
        "{}",
        ui::ok(format!(
            "Initialized project {}{template} in {}",
            ui::paint(ui::BOLD, &project_name),
            ui::tilde(&cwd.to_string_lossy())
        ))
    );
    println!(
        "  {}",
        ui::paint(
            ui::DIM,
            format!(
                "Global index: {}",
                ui::tilde(&global_path.to_string_lossy())
            )
        )
    );
    println!(
        "  {}",
        ui::paint(
            ui::DIM,
            "Next: 'quickdev add' to configure terminals and apps"
        )
    );
    Ok(())
}
