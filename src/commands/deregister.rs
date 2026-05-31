use crate::config::{
    global_config_path, load_global_config, resolve_project_config, save_global_config,
};

pub(crate) fn cmd_deregister(delete: bool) -> Result<(), String> {
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    let (config_path, project_root) = resolve_project_config(&cwd)?;

    let global_path = global_config_path();
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

    save_global_config(&global_path, &global)?;

    if delete {
        std::fs::remove_file(&config_path)
            .map_err(|e| format!("failed to delete {}: {e}", config_path.display()))?;
        println!(
            "Deregistered and deleted config for '{}'",
            removed_name.unwrap_or_default()
        );
    } else {
        println!(
            "Deregistered project '{}'",
            removed_name.unwrap_or_default()
        );
    }

    Ok(())
}
