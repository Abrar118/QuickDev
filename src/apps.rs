/// Installed `.app` bundles as `(name, path)`, sorted by name and deduplicated
/// by name (keeping the first by sort order). Used by the `add` picker, where a
/// single entry per display name is what the user wants.
pub fn discover_apps() -> Vec<(String, String)> {
    let mut apps = installed_app_bundles();
    apps.dedup_by(|a, b| a.0 == b.0);
    apps
}

/// Installed `.app` bundles as `(name, path)`, sorted by name, unique by path
/// (NO name deduplication). `capture` matches running apps by path, so it must
/// see every bundle path even when `/Applications` and `~/Applications` hold a
/// same-named app — `discover_apps`'s name dedup would otherwise hide one of
/// them and capture would silently drop a running app installed there.
pub fn discover_apps_unique_by_path() -> Vec<(String, String)> {
    installed_app_bundles()
}

/// Scan `/Applications` and `~/Applications` for `.app` bundles, returning
/// `(name, path)` sorted by name. Bundle paths are inherently unique, so no
/// deduplication is applied here; callers dedup by name if they need to.
fn installed_app_bundles() -> Vec<(String, String)> {
    #[cfg(target_os = "macos")]
    {
        let mut apps = Vec::new();

        let dirs_to_scan = vec![
            "/Applications".to_string(),
            dirs::home_dir()
                .map(|h| format!("{}/Applications", h.display()))
                .unwrap_or_default(),
        ];

        for dir in dirs_to_scan {
            if dir.is_empty() {
                continue;
            }
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let file_name = entry.file_name().to_string_lossy().to_string();
                    if file_name.starts_with('.') {
                        continue;
                    }
                    if file_name.ends_with(".app") {
                        let name = file_name
                            .strip_suffix(".app")
                            .unwrap_or(&file_name)
                            .to_string();
                        apps.push((name, path.to_string_lossy().to_string()));
                    }
                }
            }
        }

        apps.sort_by_key(|a| a.0.to_lowercase());
        apps
    }

    #[cfg(not(target_os = "macos"))]
    {
        Vec::new()
    }
}
