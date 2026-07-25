use crate::config::{
    global_config_path, load_global_config, remove_config_with, resolve_project_config,
    save_global_config,
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
        // Neither order is safe alone: deleting first can destroy a config the
        // user cannot get back if the index write then fails, and saving first
        // can leave the config orphaned on disk. remove_config_with stages the
        // config aside and restores it if the index write fails, so the project
        // ends up either fully deregistered or exactly as it started.
        remove_config_with(&config_path, || save_global_config(&global_path, &global))?;
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
