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
