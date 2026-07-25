use crate::config::{
    global_config_path, load_global_config, resolve_project_config, save_global_config,
};

pub(crate) fn cmd_deregister(delete: bool) -> Result<(), String> {
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    let (config_path, project_root) = resolve_project_config(&cwd)?;

    let global_path = global_config_path()?;
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

    if delete {
        // Neither order is safe on its own: deleting first can destroy a config
        // the user cannot get back if the index write then fails, and saving
        // first can leave the config orphaned on disk. So move it aside, save,
        // and put it back on failure — the project ends up either fully
        // deregistered or exactly as it started.
        let staged = config_path.with_extension("toml.deregister");
        std::fs::rename(&config_path, &staged)
            .map_err(|e| format!("failed to move {} aside: {e}", config_path.display()))?;
        if let Err(e) = save_global_config(&global_path, &global) {
            let _ = std::fs::rename(&staged, &config_path);
            return Err(e);
        }
        std::fs::remove_file(&staged).map_err(|e| {
            format!(
                "deregistered '{}', but could not delete {}: {e}",
                removed_name.clone().unwrap_or_default(),
                staged.display()
            )
        })?;
        println!(
            "Deregistered and deleted config for '{}'",
            removed_name.unwrap_or_default()
        );
    } else {
        save_global_config(&global_path, &global)?;
        println!(
            "Deregistered project '{}'",
            removed_name.unwrap_or_default()
        );
    }

    Ok(())
}
