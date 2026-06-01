use crate::models::AppEntry;

/// Parse the `[Desktop Entry]` group of a `.desktop` file into an `AppEntry`.
///
/// Returns `None` for entries the OS launcher would hide: `Type != Application`,
/// `NoDisplay=true`, `Hidden=true`, missing `Name`/`Exec`, or a present but
/// unresolvable `TryExec`. `try_exec_resolvable` decides whether a `TryExec`
/// value is launchable (injected so this function stays pure/testable). Only the
/// `[Desktop Entry]` group is read; later groups (e.g. `[Desktop Action …]`) are
/// ignored.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
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

/// Merge discovered launch args with user-entered args, keeping discovered args
/// first (they are launch-critical, e.g. `flatpak run app.id` or
/// `Update.exe --processStart App.exe`). Returns `None` when the result is empty.
pub fn combine_app_args(
    discovered: Option<Vec<String>>,
    user: Option<Vec<String>>,
) -> Option<Vec<String>> {
    let mut combined = discovered.unwrap_or_default();
    combined.extend(user.unwrap_or_default());
    if combined.is_empty() {
        None
    } else {
        Some(combined)
    }
}

/// Clean a Freedesktop `Exec=` value into `(executable, args)`.
///
/// Field codes (`%f %F %u %U %i %c %k …`) are stripped; a token that becomes
/// empty after stripping is dropped. `%%` is preserved as a literal `%` via a
/// sentinel so it is never mistaken for a field code. The first surviving token
/// is the executable; the rest are arguments. Pure — no filesystem I/O.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
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

/// Installed applications as `AppEntry`, sorted by name (case-insensitive) and
/// deduplicated by name (keeping the first by sort order). Used by the `add`
/// picker, where a single entry per display name is what the user wants.
pub fn discover_apps() -> Vec<AppEntry> {
    let mut apps = installed_apps();
    apps.dedup_by(|a, b| a.name.eq_ignore_ascii_case(&b.name));
    apps
}

/// Installed applications as `(name, path)`, sorted by name, unique by path (NO
/// name deduplication). `capture` matches running apps by path, so it must see
/// every path even when two directories hold a same-named app — name dedup would
/// otherwise hide one and capture would silently drop a running app.
pub fn discover_apps_unique_by_path() -> Vec<(String, String)> {
    installed_apps()
        .into_iter()
        .map(|a| (a.name, a.path))
        .collect()
}

/// Discover installed applications for the current platform, sorted by name
/// (case-insensitive). macOS scans `.app` bundles; Linux parses `.desktop`
/// files; Windows resolves Start Menu `.lnk` shortcuts. Other platforms return
/// an empty list. macOS entries always have `args: None`.
fn installed_apps() -> Vec<AppEntry> {
    #[cfg(target_os = "macos")]
    {
        let mut apps: Vec<AppEntry> = Vec::new();

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
                        apps.push(AppEntry {
                            name,
                            path: path.to_string_lossy().to_string(),
                            args: None,
                        });
                    }
                }
            }
        }

        apps.sort_by_key(|a| a.name.to_lowercase());
        apps
    }

    #[cfg(target_os = "linux")]
    {
        let mut dirs: Vec<String> = vec![
            "/usr/share/applications".to_string(),
            "/usr/local/share/applications".to_string(),
            "/var/lib/flatpak/exports/share/applications".to_string(),
            "/var/lib/snapd/desktop/applications".to_string(),
        ];
        if let Some(home) = dirs::home_dir() {
            dirs.push(format!("{}/.local/share/applications", home.display()));
        }

        let mut apps: Vec<AppEntry> = Vec::new();
        for dir in &dirs {
            let entries = match std::fs::read_dir(dir) {
                Ok(e) => e,
                Err(_) => continue,
            };
            for entry in entries.flatten() {
                let path = entry.path();
                let file_name = entry.file_name().to_string_lossy().to_string();
                if !file_name.to_lowercase().ends_with(".desktop") {
                    continue;
                }
                let contents = match std::fs::read_to_string(&path) {
                    Ok(c) => c,
                    Err(_) => continue,
                };
                if let Some(app) = parse_desktop_entry(&contents, try_exec_resolvable) {
                    apps.push(app);
                }
            }
        }
        apps.sort_by_key(|a| a.name.to_lowercase());
        apps
    }

    #[cfg(target_os = "windows")]
    {
        let mut dirs: Vec<String> = Vec::new();
        if let Ok(program_data) = std::env::var("ProgramData") {
            dirs.push(format!(
                "{program_data}\\Microsoft\\Windows\\Start Menu\\Programs"
            ));
        }
        if let Ok(app_data) = std::env::var("AppData") {
            dirs.push(format!(
                "{app_data}\\Microsoft\\Windows\\Start Menu\\Programs"
            ));
        }

        let mut apps: Vec<AppEntry> = Vec::new();
        for dir in &dirs {
            collect_lnk_apps(std::path::Path::new(dir), &mut apps);
        }
        apps.sort_by_key(|a| a.name.to_lowercase());
        apps
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        Vec::new()
    }
}

/// Recursively collect Start Menu `.lnk` shortcuts under `dir` whose target is
/// an `.exe`. Windows I/O — not unit-tested.
#[cfg(target_os = "windows")]
fn collect_lnk_apps(dir: &std::path::Path, out: &mut Vec<AppEntry>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_lnk_apps(&path, out);
            continue;
        }
        let file_name = entry.file_name().to_string_lossy().to_string();
        if !file_name.to_lowercase().ends_with(".lnk") {
            continue;
        }
        if let Some(app) = resolve_lnk(&path) {
            out.push(app);
        }
    }
}

/// Resolve a `.lnk` shortcut into an `AppEntry`, keeping only `.exe` targets and
/// carrying the shortcut's arguments. Windows/crate I/O — not unit-tested.
#[cfg(target_os = "windows")]
fn resolve_lnk(path: &std::path::Path) -> Option<AppEntry> {
    let link = lnk::ShellLink::open(path).ok()?;

    // Obtain the absolute target .exe path via LinkInfo.
    let target = link
        .link_info()
        .as_ref()
        .and_then(|li| li.local_base_path().clone())?;

    if !target.to_lowercase().ends_with(".exe") {
        return None;
    }

    let name = path.file_stem()?.to_string_lossy().to_string();

    // Obtain the arguments string, split to Vec.
    let args = link
        .arguments()
        .as_ref()
        .map(|s| {
            s.split_whitespace()
                .map(str::to_string)
                .collect::<Vec<String>>()
        })
        .filter(|v| !v.is_empty());

    Some(AppEntry {
        name,
        path: target,
        args,
    })
}

/// Whether a `.desktop` `TryExec` value points at a launchable binary: an
/// absolute/relative path that exists, or a bare name found on `$PATH`. Linux
/// I/O — not unit-tested.
#[cfg(target_os = "linux")]
fn try_exec_resolvable(cmd: &str) -> bool {
    use std::path::Path;
    if cmd.contains('/') {
        return Path::new(cmd).exists();
    }
    if let Ok(path_var) = std::env::var("PATH") {
        for dir in path_var.split(':') {
            if Path::new(dir).join(cmd).exists() {
                return true;
            }
        }
    }
    false
}
