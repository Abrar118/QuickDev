use crate::config::{
    global_config_path, load_global_config, load_project_config, save_global_config,
    save_project_config, unique_project_name,
};
use crate::models::{GlobalProjectEntry, ProjectConfig, ProjectEntry};
use std::path::PathBuf;

pub(crate) fn cmd_init(from: Option<String>) -> Result<(), String> {
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
        let mut existing = load_project_config(&config_path)?;
        let project_name = unique_project_name(&existing.project.name, &global);
        if existing.project.name != project_name {
            existing.project.name = project_name.clone();
            save_project_config(&config_path, &existing)?;
        }
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
