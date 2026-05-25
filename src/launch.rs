use crate::adapters::{
    command_exists, infer_tool_id, is_editor_tool, launch_command_for_tool, resolve_command,
};
use crate::models::ProjectConfig;
use std::path::Path;
use std::process::Command;

pub struct LaunchResult {
    pub label: String,
    pub kind: &'static str,
    pub success: bool,
    pub error: Option<String>,
}

pub fn launch_project(config: &ProjectConfig, project_root: &Path) -> Vec<LaunchResult> {
    let mut results = Vec::new();

    for (i, terminal) in config.terminals.iter().enumerate() {
        if i > 0 {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        let resolved_path = resolve_terminal_path(project_root, &terminal.path);
        let result = launch_terminal(&resolved_path, terminal.command.as_deref(), i);
        results.push(LaunchResult {
            label: terminal.name.clone(),
            kind: "terminal",
            success: result.is_ok(),
            error: result.err(),
        });
    }

    for app in &config.applications {
        let resolved_args: Option<Vec<String>> =
            app.args.as_ref().map(|a| resolve_app_args(project_root, a));
        let result =
            launch_application(&app.name, &app.path, resolved_args.as_deref(), project_root);
        results.push(LaunchResult {
            label: app.name.clone(),
            kind: "app",
            success: result.is_ok(),
            error: result.err(),
        });
    }

    results
}

pub fn resolve_terminal_path(project_root: &Path, relative_path: &str) -> String {
    let joined = project_root.join(relative_path);
    // Normalize away `.` and `..` components without hitting the filesystem
    let mut components: Vec<std::ffi::OsString> = Vec::new();
    for component in joined.components() {
        use std::path::Component;
        match component {
            Component::CurDir => {} // skip `.`
            Component::ParentDir => {
                components.pop();
            } // resolve `..`
            c => components.push(c.as_os_str().to_os_string()),
        }
    }
    let normalized: std::path::PathBuf = components.iter().collect();
    normalized.to_string_lossy().to_string()
}

pub fn resolve_app_args(project_root: &Path, args: &[String]) -> Vec<String> {
    let root_str = project_root.to_string_lossy();
    args.iter()
        .map(|arg| {
            if arg == "." {
                root_str.to_string()
            } else {
                arg.clone()
            }
        })
        .collect()
}

pub fn normalize_path(path: &str) -> String {
    #[cfg(not(target_os = "windows"))]
    {
        if let Some(rest) = path.strip_prefix("~/") {
            if let Ok(home) = std::env::var("HOME") {
                return format!("{home}/{rest}");
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(rest) = path.strip_prefix("~/") {
            if let Some(home) = dirs::home_dir() {
                return format!("{}/{rest}", home.display());
            }
        }
    }

    path.to_string()
}

fn launch_terminal(resolved_path: &str, command: Option<&str>, tab_index: usize) -> Result<(), String> {
    if !Path::new(resolved_path).exists() {
        return Err(format!("path not found: {resolved_path}"));
    }

    if try_ghostty(resolved_path, command).is_ok() {
        return Ok(());
    }

    run_in_platform_terminal(resolved_path, command, tab_index)
}

fn try_ghostty(cwd: &str, command: Option<&str>) -> Result<(), String> {
    let ghostty_cmd =
        launch_command_for_tool(std::env::consts::OS, "ghostty").ok_or("ghostty not registered")?;

    if !command_exists(ghostty_cmd) {
        return Err("ghostty not found".to_string());
    }

    let resolved = resolve_command(ghostty_cmd).unwrap_or_else(|| ghostty_cmd.to_string());

    let shell_command = match command {
        Some(cmd) => format!("cd '{}' && {}", cwd.replace('\'', "'\\''"), cmd),
        None => format!("cd '{}' && exec $SHELL", cwd.replace('\'', "'\\''")),
    };

    Command::new(resolved)
        .args(["-e", "sh", "-lc", &shell_command])
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("ghostty launch failed: {e}"))
}

fn run_in_platform_terminal(cwd: &str, command: Option<&str>, tab_index: usize) -> Result<(), String> {
    let cmd_str = command.unwrap_or("");

    #[cfg(target_os = "macos")]
    {
        let cd_part = format!("cd '{}'", cwd.replace('\'', "'\\''"));
        let full = if cmd_str.is_empty() {
            cd_part
        } else {
            format!("{cd_part} && {cmd_str}")
        };
        let escaped = full.replace('"', "\\\"");
        let script = if tab_index == 0 {
            format!(
                "tell application \"Terminal\"\n    activate\n    do script \"{escaped}\"\nend tell"
            )
        } else {
            format!(
                "tell application \"Terminal\"\n    activate\n    do script \"{escaped}\" in front window\nend tell"
            )
        };
        Command::new("osascript")
            .args(["-e", &script])
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("Terminal.app launch failed: {e}"))
    }

    #[cfg(target_os = "windows")]
    {
        if command_exists("wt") {
            let wt_resolved = resolve_command("wt").unwrap_or_else(|| "wt".to_string());
            if tab_index == 0 {
                let mut cmd = Command::new(&wt_resolved);
                cmd.args(["-d", cwd]);
                if !cmd_str.is_empty() {
                    cmd.args(["cmd", "/K", cmd_str]);
                }
                return cmd
                    .spawn()
                    .map(|_| ())
                    .map_err(|e| format!("wt launch failed: {e}"));
            } else {
                let mut cmd = Command::new(&wt_resolved);
                cmd.args(["-w", "0", "new-tab", "-d", cwd]);
                if !cmd_str.is_empty() {
                    cmd.args(["cmd", "/K", cmd_str]);
                }
                return cmd
                    .spawn()
                    .map(|_| ())
                    .map_err(|e| format!("wt launch failed: {e}"));
            }
        }

        if command_exists("pwsh") {
            let ps_cmd = if cmd_str.is_empty() {
                format!("Set-Location '{cwd}'")
            } else {
                format!("Set-Location '{cwd}'; {cmd_str}")
            };
            return Command::new(resolve_command("pwsh").unwrap_or_else(|| "pwsh".to_string()))
                .args(["-NoExit", "-Command", &ps_cmd])
                .spawn()
                .map(|_| ())
                .map_err(|e| format!("pwsh launch failed: {e}"));
        }

        let full = if cmd_str.is_empty() {
            format!("cd /d \"{cwd}\"")
        } else {
            format!("cd /d \"{cwd}\" && {cmd_str}")
        };
        Command::new("cmd")
            .args(["/C", "start", "cmd", "/K", &full])
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("cmd launch failed: {e}"))
    }

    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        let shell_command = if cmd_str.is_empty() {
            format!("cd '{}' && exec $SHELL", cwd.replace('\'', "'\\''"))
        } else {
            format!("cd '{}' && {}", cwd.replace('\'', "'\\''"), cmd_str)
        };

        if command_exists("gnome-terminal") {
            let resolved =
                resolve_command("gnome-terminal").unwrap_or_else(|| "gnome-terminal".to_string());
            let mut cmd = Command::new(resolved);
            if tab_index > 0 {
                cmd.arg("--tab");
            }
            cmd.args(["--", "sh", "-lc", &shell_command]);
            if cmd.spawn().is_ok() {
                return Ok(());
            }
        }

        let candidates: &[(&str, &[&str])] = &[
            ("konsole", &["-e"]),
            ("alacritty", &["-e"]),
            ("xterm", &["-e"]),
        ];

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

        Err("no terminal emulator found".to_string())
    }
}

fn launch_application(
    name: &str,
    path: &str,
    args: Option<&[String]>,
    project_root: &Path,
) -> Result<(), String> {
    let normalized_path = normalize_path(path);

    let tool_id = infer_tool_id(name, &normalized_path);

    if let Some(ref tid) = tool_id {
        if let Some(cli) = launch_command_for_tool(std::env::consts::OS, tid) {
            if command_exists(cli) {
                let resolved = resolve_command(cli).unwrap_or_else(|| cli.to_string());
                let mut cmd = Command::new(&resolved);

                if is_editor_tool(tid) {
                    cmd.arg(project_root.to_string_lossy().as_ref());
                } else if let Some(a) = args {
                    for arg in a {
                        cmd.arg(arg);
                    }
                }

                return cmd
                    .spawn()
                    .map(|_| ())
                    .map_err(|e| format!("launch failed: {e}"));
            }
        }
    }

    launch_application_generic(&normalized_path, args)
}

fn launch_application_generic(executable: &str, args: Option<&[String]>) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        if executable.ends_with(".app") || Path::new(executable).is_dir() {
            let mut cmd = Command::new("open");
            cmd.arg("-a").arg(executable);
            if let Some(a) = args {
                if !a.is_empty() {
                    cmd.arg("--args");
                    for arg in a {
                        cmd.arg(arg);
                    }
                }
            }
            return cmd
                .spawn()
                .map(|_| ())
                .map_err(|e| format!("launch failed: {e}"));
        }
    }

    let mut cmd = Command::new(executable);
    if let Some(a) = args {
        for arg in a {
            cmd.arg(arg);
        }
    }
    cmd.spawn()
        .map(|_| ())
        .map_err(|e| format!("launch failed: {e}"))
}
