use crate::config::{global_config_path, load_global_config, prune_projects, save_global_config};
use crate::ui;
use anstream::println;

pub(crate) fn cmd_prune() -> Result<(), String> {
    let global_path = global_config_path()?;
    let mut global = load_global_config(&global_path)?;
    let total = global.projects.len();

    let removed = prune_projects(&mut global);

    if removed.is_empty() {
        println!(
            "{}",
            ui::ok(format!(
                "Nothing to prune — all {} healthy",
                ui::count(total, "registered project")
            ))
        );
        return Ok(());
    }

    save_global_config(&global_path, &global)?;
    println!(
        "{}",
        ui::ok(format!("Pruned {}", ui::count(removed.len(), "project")))
    );
    for name in &removed {
        println!("  {} {name}", ui::paint(ui::DIM, "-"));
    }
    Ok(())
}
