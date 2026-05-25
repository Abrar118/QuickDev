pub fn discover_apps() -> Vec<(String, String)> {
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

        apps.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
        apps.dedup_by(|a, b| a.0 == b.0);
        apps
    }

    #[cfg(not(target_os = "macos"))]
    {
        Vec::new()
    }
}
