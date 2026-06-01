use crate::models::AppEntry;

/// Parse the `[Desktop Entry]` group of a `.desktop` file into an `AppEntry`.
///
/// Returns `None` for entries the OS launcher would hide: `Type != Application`,
/// `NoDisplay=true`, `Hidden=true`, missing `Name`/`Exec`, or a present but
/// unresolvable `TryExec`. `try_exec_resolvable` decides whether a `TryExec`
/// value is launchable (injected so this function stays pure/testable). Only the
/// `[Desktop Entry]` group is read; later groups (e.g. `[Desktop Action …]`) are
/// ignored.
pub fn parse_desktop_entry(
    contents: &str,
    try_exec_resolvable: impl Fn(&str) -> bool,
) -> Option<AppEntry> {
    let mut in_group = false;
    let mut type_ = None;
    let mut name = None;
    let mut exec = None;
    let mut try_exec = None;
    let mut no_display = false;
    let mut hidden = false;

    for line in contents.lines() {
        let line = line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            in_group = line == "[Desktop Entry]";
            continue;
        }
        if !in_group {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim();
            match key {
                "Type" => type_ = Some(value.to_string()),
                "Name" if name.is_none() => name = Some(value.to_string()),
                "Exec" => exec = Some(value.to_string()),
                "TryExec" => try_exec = Some(value.to_string()),
                "NoDisplay" => no_display = value == "true",
                "Hidden" => hidden = value == "true",
                _ => {}
            }
        }
    }

    if type_.as_deref() != Some("Application") || no_display || hidden {
        return None;
    }
    if let Some(te) = &try_exec {
        if !try_exec_resolvable(te) {
            return None;
        }
    }

    let name = name.filter(|n| !n.is_empty())?;
    let exec = exec.filter(|e| !e.is_empty())?;
    let (path, args) = parse_exec(&exec);
    if path.is_empty() {
        return None;
    }
    let args = if args.is_empty() { None } else { Some(args) };
    Some(AppEntry { name, path, args })
}

/// Clean a Freedesktop `Exec=` value into `(executable, args)`.
///
/// Field codes (`%f %F %u %U %i %c %k …`) are stripped; a token that becomes
/// empty after stripping is dropped. `%%` is preserved as a literal `%` via a
/// sentinel so it is never mistaken for a field code. The first surviving token
/// is the executable; the rest are arguments. Pure — no filesystem I/O.
pub fn parse_exec(exec: &str) -> (String, Vec<String>) {
    const SENTINEL: char = '\u{0}';
    const FIELD_CODES: [&str; 13] = [
        "%f", "%F", "%u", "%U", "%i", "%c", "%k", "%d", "%D", "%n", "%N", "%v", "%m",
    ];

    let protected = exec.replace("%%", "\u{0}");
    let tokens = shell_words::split(&protected).unwrap_or_default();

    let mut cleaned: Vec<String> = Vec::new();
    for token in tokens {
        if FIELD_CODES.contains(&token.as_str()) {
            continue;
        }
        let t = token.replace(SENTINEL, "%");
        if !t.is_empty() {
            cleaned.push(t);
        }
    }

    if cleaned.is_empty() {
        return (String::new(), Vec::new());
    }
    let path = cleaned.remove(0);
    (path, cleaned)
}

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
