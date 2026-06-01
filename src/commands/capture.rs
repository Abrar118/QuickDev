use crate::apps;
use crate::capture::{detect_running_apps, detected_to_apps, merge_apps};
use crate::config;
use crate::fzf;
use crate::models::AppEntry;

pub(crate) fn cmd_capture(all: bool) -> Result<(), String> {
    // Fail fast off-macOS before any filesystem or picker work.
    if !cfg!(target_os = "macos") {
        return Err("capture is only supported on macOS".to_string());
    }

    let cwd = std::env::current_dir().map_err(|e| format!("failed to read current dir: {e}"))?;
    // resolve_project_config (not find_project_config) so that, like add/edit,
    // running outside a project tree opens the fzf project picker.
    let (config_path, _root) = config::resolve_project_config(&cwd)?;

    // Propagate osascript failures (e.g. denied Automation permission) instead
    // of treating them as "nothing running".
    let running = detect_running_apps()?;
    // Path-unique (not discover_apps, which dedups by name): capture matches by
    // path, so it must see a same-named app in both /Applications and
    // ~/Applications rather than silently dropping the one name-dedup hides.
    let installed = apps::discover_apps_unique_by_path();
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
/// fzf multi-select. Each line is prefixed with its candidate index ("0\tName —
/// path") and selections are mapped back by that leading index, so a tab in an
/// app name or path can't corrupt the round-trip. An empty/cancelled selection
/// surfaces as the `fzf::CANCELLED` sentinel.
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
        .enumerate()
        .map(|(i, a)| format!("{i}\t{} — {}", a.name, a.path))
        .collect();
    let picked = fzf::fzf_select_multi(
        &items,
        "Select apps to capture (TAB to toggle, ENTER to confirm):",
    )?;
    let picked_indices: Vec<usize> = picked
        .iter()
        .filter_map(|line| line.split('\t').next())
        .filter_map(|idx| idx.parse::<usize>().ok())
        .collect();
    Ok(candidates
        .iter()
        .enumerate()
        .filter(|(i, _)| picked_indices.contains(i))
        .map(|(_, a)| a.clone())
        .collect())
}
