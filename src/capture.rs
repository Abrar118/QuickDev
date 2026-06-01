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

/// Return the captured apps not already present in `existing`, compared by
/// normalized `path` (trailing-slash-insensitive, via `normalize_bundle_path`),
/// preserving the order of `captured`.
pub fn merge_apps(existing: &[AppEntry], captured: &[AppEntry]) -> Vec<AppEntry> {
    captured
        .iter()
        .filter(|c| {
            let cn = normalize_bundle_path(&c.path);
            !existing
                .iter()
                .any(|e| normalize_bundle_path(&e.path) == cn)
        })
        .cloned()
        .collect()
}

/// Bundle POSIX paths of currently-running foreground (GUI) applications.
///
/// macOS only; returns `Ok(empty)` elsewhere. Returns `Err` with the osascript
/// stderr when osascript fails (e.g. Automation/Accessibility permission
/// denied), so a permission failure is not mistaken for "nothing running".
/// Isolated I/O — not unit-tested.
pub fn detect_running_apps() -> Result<Vec<String>, String> {
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        let output = Command::new("osascript")
            .args([
                "-e",
                "set text item delimiters to linefeed",
                "-e",
                "tell application \"System Events\" to get (POSIX path of (application file of every process whose background only is false)) as text",
            ])
            .output()
            .map_err(|e| format!("failed to run osascript: {e}"))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let detail = stderr.trim();
            let detail = if detail.is_empty() {
                "osascript failed (is Automation/Accessibility permission granted?)"
            } else {
                detail
            };
            return Err(format!("could not list running apps: {detail}"));
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout
            .lines()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect())
    }

    #[cfg(not(target_os = "macos"))]
    {
        Ok(Vec::new())
    }
}
