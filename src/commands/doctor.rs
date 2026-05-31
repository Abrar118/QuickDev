use crate::config::{
    global_config_path, load_global_config, load_project_config, project_statuses, prune_projects,
    save_global_config, save_project_config,
};
use crate::doctor::{doctor_has_errors, render_doctor, DoctorReport};
use crate::fzf;
use std::path::PathBuf;

pub(crate) fn cmd_doctor(fix: bool) -> Result<(), String> {
    if fix {
        run_fix()?;
    }

    let report = gather_report();
    print!("{}", render_doctor(&report));

    if doctor_has_errors(&report) {
        Err("doctor found problems (run 'quickdev doctor --fix' to repair)".to_string())
    } else {
        Ok(())
    }
}

fn gather_report() -> DoctorReport {
    let global_path = global_config_path();
    let (global_config_ok, projects) = match load_global_config(&global_path) {
        Ok(global) => (true, project_statuses(&global)),
        Err(_) => (false, vec![]),
    };

    DoctorReport {
        global_config_ok,
        fzf_available: fzf::check_fzf(),
        projects,
    }
}

fn run_fix() -> Result<(), String> {
    let global_path = global_config_path();

    // 1. Create the global config dir/file if missing.
    if !global_path.exists() {
        let global = load_global_config(&global_path)?; // empty config when absent
        save_global_config(&global_path, &global)?;
        println!("✓ created global config at {}", global_path.display());
    }

    // 2. Prune registrations whose path or .quickdev.toml is missing.
    let mut global = load_global_config(&global_path)?;
    let removed = prune_projects(&mut global);
    if !removed.is_empty() {
        save_global_config(&global_path, &global)?;
        println!(
            "✓ pruned {} dead registration(s): {}",
            removed.len(),
            removed.join(", ")
        );
    }

    // 3. Normalize each remaining (healthy) project config to canonical form.
    for entry in &global.projects {
        let config_path = PathBuf::from(&entry.path).join(".quickdev.toml");
        match load_project_config(&config_path) {
            Ok(cfg) => match save_project_config(&config_path, &cfg) {
                Ok(()) => println!("✓ normalized {}", config_path.display()),
                Err(e) => println!("⚠ could not normalize {}: {e}", config_path.display()),
            },
            Err(e) => println!("⚠ skipped {} (parse error): {e}", config_path.display()),
        }
    }

    Ok(())
}
