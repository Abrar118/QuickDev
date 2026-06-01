use crate::apps;
use crate::capture::{detect_running_apps, detected_to_apps, merge_apps};
use crate::config;
use crate::fzf;
use crate::models::AppEntry;

pub(crate) fn cmd_capture(all: bool) -> Result<(), String> {
    let cwd = std::env::current_dir().map_err(|e| format!("failed to read current dir: {e}"))?;
    let (config_path, _root) = config::find_project_config(&cwd)?;

    if !cfg!(target_os = "macos") {
        return Err("capture is only supported on macOS".to_string());
    }

    // Propagate osascript failures (e.g. denied Automation permission) instead
    // of treating them as "nothing running".
    let running = detect_running_apps()?;
    let installed = apps::discover_apps();
    let candidates = detected_to_apps(&running, &installed);

    let mut project = config::load_project_config(&config_path)?;
    let new_candidates = merge_apps(&project.applications, &candidates);

    if new_candidates.is_empty() {
        println!("Nothing new to capture (no running apps, or all already configured).");
        return Ok(());
    }

    // select_apps returns the CANCELLED sentinel (via fzf) if the user cancels
    // or selects nothing; main.rs renders that as "Cancelled." and exit 0.
    let selected = select_apps(&new_candidates, all)?;

    let count = selected.len();
    project.applications.extend(selected);
    config::save_project_config(&config_path, &project)?;
    println!("✓ Added {count} app(s) to .quickdev.toml");
    Ok(())
}

/// Choose which candidate apps to add. With `all`, take everything; otherwise
/// fzf multi-select on "Name\tpath" lines, matching selections back by path.
/// An empty/cancelled selection surfaces as the `fzf::CANCELLED` sentinel.
fn select_apps(candidates: &[AppEntry], all: bool) -> Result<Vec<AppEntry>, String> {
    if all {
        return Ok(candidates.to_vec());
    }
    if !fzf::check_fzf() {
        return Err(
            "fzf is not installed. Install it, or re-run with --all. https://github.com/junegunn/fzf"
                .to_string(),
        );
    }
    let items: Vec<String> = candidates
        .iter()
        .map(|a| format!("{}\t{}", a.name, a.path))
        .collect();
    let picked = fzf::fzf_select_multi(&items, "Select apps to capture")?;
    let picked_paths: Vec<&str> = picked
        .iter()
        .filter_map(|line| line.split('\t').nth(1))
        .collect();
    Ok(candidates
        .iter()
        .filter(|a| picked_paths.contains(&a.path.as_str()))
        .cloned()
        .collect())
}
