use crate::models::AppEntry;
use std::collections::HashSet;

/// Normalize a bundle path for comparison: strip a single trailing `/` so
/// `…/Cursor.app/` and `…/Cursor.app` compare equal. Pure (no filesystem I/O);
/// symlink/`.` canonicalization is intentionally not done here.
pub fn normalize_bundle_path(path: &str) -> &str {
    path.strip_suffix('/').unwrap_or(path)
}

/// Map running GUI app bundle paths to installed bundles.
///
/// `running_paths` are `.app` POSIX paths from `detect_running_apps`;
/// `installed` is the `(name, path)` shape from `apps::discover_apps()`.
/// Matching is by normalized path; the installed bundle's canonical name and
/// (un-normalized) path are used in the output. Running paths with no installed
/// bundle are dropped. The result is sorted by name (case-insensitive) and
/// unique by normalized path.
pub fn detected_to_apps(running_paths: &[String], installed: &[(String, String)]) -> Vec<AppEntry> {
    let mut apps: Vec<AppEntry> = Vec::new();
    let mut seen: HashSet<&str> = HashSet::new();
    for path in running_paths {
        let norm = normalize_bundle_path(path);
        if !seen.insert(norm) {
            continue;
        }
        if let Some((inst_name, inst_path)) = installed
            .iter()
            .find(|(_, p)| normalize_bundle_path(p) == norm)
        {
            apps.push(AppEntry {
                name: inst_name.clone(),
                path: inst_path.clone(),
                args: None,
            });
        }
    }
    apps.sort_by_key(|a| a.name.to_lowercase());
    apps
}
