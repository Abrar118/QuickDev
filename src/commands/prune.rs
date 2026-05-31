use crate::config::{global_config_path, load_global_config, prune_projects, save_global_config};

pub(crate) fn cmd_prune() -> Result<(), String> {
    let global_path = global_config_path();
    let mut global = load_global_config(&global_path)?;
    let total = global.projects.len();

    let removed = prune_projects(&mut global);

    if removed.is_empty() {
        println!("Nothing to prune — all {total} registered project(s) healthy.");
        return Ok(());
    }

    save_global_config(&global_path, &global)?;
    println!("Pruned {} project(s):", removed.len());
    for name in &removed {
        println!("  - {name}");
    }
    Ok(())
}
