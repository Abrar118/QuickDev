use std::process::Command;

struct ToolInfo {
    tool_id: &'static str,
    launch_command: &'static str,
}

const TOOLS: &[ToolInfo] = &[
    ToolInfo { tool_id: "vscode", launch_command: "code" },
    ToolInfo { tool_id: "cursor", launch_command: "cursor" },
    ToolInfo { tool_id: "zed", launch_command: "zed" },
    ToolInfo { tool_id: "ghostty", launch_command: "ghostty" },
];

pub fn launch_command_for_tool(_platform: &str, tool_id: &str) -> Option<&'static str> {
    TOOLS
        .iter()
        .find(|t| t.tool_id == tool_id)
        .map(|t| t.launch_command)
}

pub fn infer_tool_id(name: &str, path: &str) -> Option<String> {
    let haystack = format!("{} {}", name.to_lowercase(), path.to_lowercase());
    if haystack.contains("cursor") {
        return Some("cursor".to_string());
    }
    if haystack.contains("code") || haystack.contains("vscode") || haystack.contains("visual studio") {
        return Some("vscode".to_string());
    }
    if haystack.contains("zed") {
        return Some("zed".to_string());
    }
    if haystack.contains("ghostty") {
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
        stdout.lines().map(str::trim).find(|l| !l.is_empty()).map(str::to_string)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let output = Command::new("which").arg(command).output().ok()?;
        if !output.status.success() {
            return None;
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        stdout.lines().map(str::trim).find(|l| !l.is_empty()).map(str::to_string)
    }
}
