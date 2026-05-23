use crate::adapters::tool_adapter::launch_command_for_tool;
use crate::models::LaunchItem;
use std::borrow::Cow;
use std::path::Path;
use std::process::Command;

pub fn launch_item(item: &LaunchItem) -> Result<(), String> {
    match item.item_type.as_str() {
        "folder" => {
            let target = item
                .path
                .as_deref()
                .ok_or_else(|| "missing_target".to_string())?;
            let normalized = normalize_path(target);
            if !Path::new(normalized.as_ref()).exists() {
                return Err("path_not_found".to_string());
            }
            open_target(normalized.as_ref())
        }
        "application" => launch_application_item(item),
        "command" => launch_terminal_item(item),
        _ => {
            let target = item
                .path
                .as_deref()
                .ok_or_else(|| "unsupported_item_type".to_string())?;
            open_target(target)
        }
    }
}

fn launch_application_item(item: &LaunchItem) -> Result<(), String> {
    let executable = item
        .path
        .as_deref()
        .ok_or_else(|| "missing_target".to_string())?;

    if let Some(tool) = item.tool_id.as_deref() {
        let platform = std::env::consts::OS;
        if let Some(cli) = launch_command_for_tool(platform, tool) {
            if command_exists(cli) {
                let resolved = resolve_command(cli).unwrap_or_else(|| cli.to_string());
                let mut cmd = Command::new(&resolved);
                if let Some(args_json) = item.args.as_deref() {
                    if let Ok(args) = serde_json::from_str::<Vec<String>>(args_json) {
                        for arg in &args {
                            cmd.arg(arg);
                        }
                    }
                }
                return cmd
                    .spawn()
                    .map(|_| ())
                    .map_err(|e| format!("tool_launch_failed: {}", e));
            }
        }
    }

    launch_application_generic(executable, item.args.as_deref())
}

fn launch_application_generic(executable: &str, args_json: Option<&str>) -> Result<(), String> {
    let parsed_args: Vec<String> = args_json
        .and_then(|j| serde_json::from_str(j).ok())
        .unwrap_or_default();

    #[cfg(target_os = "macos")]
    {
        if executable.ends_with(".app") || Path::new(executable).is_dir() {
            let mut cmd = Command::new("open");
            cmd.arg("-a").arg(executable);
            if !parsed_args.is_empty() {
                cmd.arg("--args");
                for arg in &parsed_args {
                    cmd.arg(arg);
                }
            }
            return cmd
                .spawn()
                .map(|_| ())
                .map_err(|e| format!("launch_failed: {}", e));
        }
    }

    let mut cmd = Command::new(executable);
    for arg in &parsed_args {
        cmd.arg(arg);
    }
    cmd.spawn()
        .map(|_| ())
        .map_err(|e| format!("launch_failed: {}", e))
}

fn launch_terminal_item(item: &LaunchItem) -> Result<(), String> {
    let command = item
        .command
        .as_deref()
        .ok_or_else(|| "missing_command".to_string())?;
    let cwd = item.path.as_deref();

    if try_ghostty(command, cwd).is_ok() {
        return Ok(());
    }

    run_in_platform_terminal(command, cwd)
}

fn try_ghostty(command: &str, cwd: Option<&str>) -> Result<(), String> {
    let platform = std::env::consts::OS;
    let ghostty = launch_command_for_tool(platform, "ghostty")
        .ok_or_else(|| "tool_unavailable".to_string())?;
    if !command_exists(ghostty) {
        return Err("tool_unavailable".to_string());
    }

    #[cfg(target_os = "windows")]
    {
        let _ = command;
        let _ = cwd;
        return Err("tool_unavailable".to_string());
    }

    #[cfg(not(target_os = "windows"))]
    {
        let shell_command = if let Some(path) = cwd {
            if Path::new(path).exists() {
                format!("cd '{}' && {}", path.replace('\'', "'\\''"), command)
            } else {
                command.to_string()
            }
        } else {
            command.to_string()
        };

        Command::new(resolve_command(ghostty).unwrap_or_else(|| ghostty.to_string()))
            .args(["-e", "sh", "-lc", &shell_command])
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("tool_launch_failed: {}", e))
    }
}

fn run_in_platform_terminal(command: &str, cwd: Option<&str>) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let cd_part = if let Some(path) = cwd {
            if Path::new(path).exists() {
                format!("cd '{}' && ", path.replace('\'', "'\\''"))
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        let script = format!(
            "tell application \"Terminal\"\n\
                activate\n\
                do script \"{}{}\"\n\
            end tell",
            cd_part.replace('"', "\\\""),
            command.replace('"', "\\\"")
        );
        return Command::new("osascript")
            .args(["-e", &script])
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("terminal_launch_failed: {}", e));
    }

    #[cfg(target_os = "windows")]
    {
        let full_command = if let Some(path) = cwd {
            if Path::new(path).exists() {
                format!("cd /d \"{}\" && {}", path, command)
            } else {
                command.to_string()
            }
        } else {
            command.to_string()
        };

        if command_exists("wt") {
            let wt_resolved = resolve_command("wt").unwrap_or_else(|| "wt".to_string());
            let mut cmd = Command::new(wt_resolved);
            if let Some(path) = cwd {
                if Path::new(path).exists() {
                    cmd.args(["-d", path]);
                }
            }
            cmd.args(["cmd", "/K", command]);
            return cmd
                .spawn()
                .map(|_| ())
                .map_err(|e| format!("terminal_launch_failed: {}", e));
        }

        return Command::new("cmd")
            .args(["/C", "start", "cmd", "/K", &full_command])
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("terminal_launch_failed: {}", e));
    }

    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        let candidates: &[(&str, &[&str])] = &[
            ("gnome-terminal", &["--"]),
            ("konsole", &["-e"]),
            ("alacritty", &["-e"]),
            ("xterm", &["-e"]),
        ];

        let shell_command = if let Some(path) = cwd {
            if Path::new(path).exists() {
                format!("cd '{}' && {}", path.replace('\'', "'\\''"), command)
            } else {
                command.to_string()
            }
        } else {
            command.to_string()
        };

        for (bin, prefix_args) in candidates {
            if !command_exists(bin) {
                continue;
            }
            let resolved = resolve_command(bin).unwrap_or_else(|| (*bin).to_string());
            let mut cmd = Command::new(resolved);
            for arg in *prefix_args {
                cmd.arg(arg);
            }
            cmd.args(["sh", "-lc", &shell_command]);
            if cmd.spawn().is_ok() {
                return Ok(());
            }
        }

        Err("no_terminal_found".to_string())
    }
}

fn open_target(target: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(target)
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("open_failed: {}", e))
    }

    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/C", "start", "", target])
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("open_failed: {}", e))
    }

    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        Command::new("xdg-open")
            .arg(target)
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("open_failed: {}", e))
    }
}

fn command_exists(command: &str) -> bool {
    resolve_command(command).is_some()
}

fn resolve_command(command: &str) -> Option<String> {
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
            .find(|line| !line.is_empty())
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
            .find(|line| !line.is_empty())
            .map(str::to_string)
    }
}

fn normalize_path(target: &str) -> Cow<'_, str> {
    #[cfg(not(target_os = "windows"))]
    {
        if let Some(rest) = target.strip_prefix("~/") {
            if let Ok(home) = std::env::var("HOME") {
                return Cow::Owned(format!("{home}/{rest}"));
            }
        }
    }

    Cow::Borrowed(target)
}

pub fn infer_tool_id(name: &str, path: &str) -> Option<String> {
    let haystack = format!("{} {}", name.to_lowercase(), path.to_lowercase());
    if haystack.contains("cursor") {
        return Some("cursor".to_string());
    }
    if haystack.contains("code")
        || haystack.contains("vscode")
        || haystack.contains("visual studio")
    {
        return Some("vscode".to_string());
    }
    if haystack.contains("zed") {
        return Some("zed".to_string());
    }
    if haystack.contains("ghostty") || haystack.contains("ghosty") {
        return Some("ghostty".to_string());
    }
    if haystack.contains("notepad") {
        return Some("notepad".to_string());
    }
    if haystack.contains("nano") {
        return Some("nano".to_string());
    }
    None
}

pub fn is_editor_tool(tool_id: &str) -> bool {
    matches!(tool_id, "vscode" | "cursor" | "zed")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::LaunchItem;

    fn make_folder(path: Option<&str>) -> LaunchItem {
        LaunchItem {
            item_type: "folder".to_string(),
            label: "test".to_string(),
            path: path.map(str::to_string),
            command: None,
            args: None,
            tool_id: None,
        }
    }

    #[test]
    fn missing_folder_path_returns_error() {
        let item = make_folder(None);
        let result = launch_item(&item);
        assert_eq!(result, Err("missing_target".to_string()));
    }

    #[test]
    fn invalid_folder_path_returns_path_not_found() {
        let item = make_folder(Some("/definitely/missing/path/for/quickdev"));
        let result = launch_item(&item);
        assert_eq!(result, Err("path_not_found".to_string()));
    }

    #[test]
    fn infer_tool_id_detects_vscode() {
        assert_eq!(
            infer_tool_id("Visual Studio Code", "/Applications/Visual Studio Code.app"),
            Some("vscode".to_string())
        );
    }

    #[test]
    fn infer_tool_id_detects_cursor() {
        assert_eq!(
            infer_tool_id("Cursor", "/Applications/Cursor.app"),
            Some("cursor".to_string())
        );
    }

    #[test]
    fn infer_tool_id_detects_zed() {
        assert_eq!(
            infer_tool_id("Zed", "/usr/local/bin/zed"),
            Some("zed".to_string())
        );
    }

    #[test]
    fn infer_tool_id_returns_none_for_unknown() {
        assert_eq!(
            infer_tool_id("My Custom App", "/usr/local/bin/myapp"),
            None
        );
    }

    #[test]
    fn is_editor_tool_recognizes_editors() {
        assert!(is_editor_tool("vscode"));
        assert!(is_editor_tool("cursor"));
        assert!(is_editor_tool("zed"));
        assert!(!is_editor_tool("ghostty"));
        assert!(!is_editor_tool("notepad"));
    }

    #[test]
    fn missing_command_returns_error() {
        let item = LaunchItem {
            item_type: "command".to_string(),
            label: "test".to_string(),
            path: Some("/tmp".to_string()),
            command: None,
            args: None,
            tool_id: None,
        };
        let result = launch_item(&item);
        assert_eq!(result, Err("missing_command".to_string()));
    }
}
