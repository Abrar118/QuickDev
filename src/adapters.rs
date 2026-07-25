use std::process::Command;

struct ToolInfo {
    tool_id: &'static str,
    launch_command: &'static str,
}

const TOOLS: &[ToolInfo] = &[
    ToolInfo {
        tool_id: "vscode",
        launch_command: "code",
    },
    ToolInfo {
        tool_id: "cursor",
        launch_command: "cursor",
    },
    ToolInfo {
        tool_id: "zed",
        launch_command: "zed",
    },
    ToolInfo {
        tool_id: "ghostty",
        launch_command: "ghostty",
    },
];

pub fn launch_command_for_tool(_platform: &str, tool_id: &str) -> Option<&'static str> {
    TOOLS
        .iter()
        .find(|t| t.tool_id == tool_id)
        .map(|t| t.launch_command)
}

/// Identify a known tool from the launch PATH alone (never the display name).
///
/// Used to decide whether to substitute the tool's own CLI at launch time. A
/// wrapper path like `flatpak` or a Squirrel `Update.exe` must NOT be treated as
/// the tool just because the app's display name matches — otherwise the CLI gets
/// invoked with the wrapper's arguments (e.g. a Flatpak VS Code with
/// `path=flatpak, args=[run, com.visualstudio.code]` would run
/// `code run com.visualstudio.code`). Matching only on the path keeps such
/// wrapper entries on the generic launch path, where `flatpak run …` works.
pub fn infer_tool_id_from_path(path: &str) -> Option<String> {
    let path_lower = path.to_lowercase();

    if path_lower.contains("cursor") {
        return Some("cursor".to_string());
    }
    if path_lower.contains("visual studio code")
        || path_lower.ends_with("/code")
        || path_lower.ends_with("code.app")
        || path_lower.ends_with("code.exe")
    {
        return Some("vscode".to_string());
    }
    if path_lower.contains("zed") {
        return Some("zed".to_string());
    }
    if path_lower.contains("ghostty") {
        return Some("ghostty".to_string());
    }
    None
}

pub fn infer_tool_id(name: &str, path: &str) -> Option<String> {
    if let Some(tool_id) = infer_tool_id_from_path(path) {
        return Some(tool_id);
    }

    let name_lower = name.to_lowercase();
    if name_lower.contains("cursor") {
        return Some("cursor".to_string());
    }
    if name_lower.contains("vscode") || name_lower == "code" || name_lower == "visual studio code" {
        return Some("vscode".to_string());
    }
    if name_lower.contains("zed") {
        return Some("zed".to_string());
    }
    if name_lower.contains("ghostty") {
        return Some("ghostty".to_string());
    }
    None
}

pub fn is_editor_tool(tool_id: &str) -> bool {
    matches!(tool_id, "vscode" | "cursor" | "zed")
}

pub fn command_exists(command: &str) -> bool {
    resolve_command(command).is_some()
}

/// Candidate locations for a macOS kitty install that isn't on `PATH`.
///
/// kitty's macOS installers (the `.dmg` and the official `curl` installer) drop
/// an app bundle without putting `kitty` on `PATH` — kitty's own docs hand users
/// the full binary path for command-line use. A `PATH`-only probe therefore
/// misses a perfectly good installation, silently downgrading auto-detect to
/// another emulator and failing explicit `emulator = "kitty"` as "not found".
///
/// `home` is a parameter rather than read internally so this stays a pure,
/// testable function.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub fn macos_kitty_bundle_paths(home: Option<&str>) -> Vec<String> {
    let mut paths = vec!["/Applications/kitty.app/Contents/MacOS/kitty".to_string()];
    if let Some(home) = home {
        paths.push(format!(
            "{home}/Applications/kitty.app/Contents/MacOS/kitty"
        ));
        paths.push(format!("{home}/.local/kitty.app/bin/kitty"));
    }
    paths
}

/// Resolve the kitty executable to invoke: `PATH` first, then (on macOS) the
/// standard app-bundle locations. `None` means kitty isn't installed.
///
/// Every kitty probe and launch goes through this so that detection, the
/// Terminal.app tabbing prompt, and the launchers can never disagree about
/// whether kitty is available.
pub fn resolve_kitty() -> Option<String> {
    if let Some(path) = resolve_command("kitty") {
        return Some(path);
    }

    #[cfg(target_os = "macos")]
    {
        let home = dirs::home_dir().map(|p| p.to_string_lossy().into_owned());
        macos_kitty_bundle_paths(home.as_deref())
            .into_iter()
            .find(|p| std::path::Path::new(p).is_file())
    }

    #[cfg(not(target_os = "macos"))]
    {
        None
    }
}

pub fn resolve_command(command: &str) -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        let output = Command::new("where").arg(command).output().ok()?;
        if !output.status.success() {
            return None;
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        stdout
            .lines()
            .map(str::trim)
            .find(|l| !l.is_empty())
            .map(str::to_string)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let output = Command::new("which").arg(command).output().ok()?;
        if !output.status.success() {
            return None;
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        stdout
            .lines()
            .map(str::trim)
            .find(|l| !l.is_empty())
            .map(str::to_string)
    }
}
