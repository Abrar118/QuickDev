use crate::adapters::{
    command_exists, infer_tool_id, infer_tool_id_from_path, is_editor_tool,
    launch_command_for_tool, resolve_command,
};
use crate::ghostty_applescript::{build_script as build_ghostty_script, ResolvedTerminal};
use crate::models::ProjectConfig;
use crate::tab_strategy::{select_tab_strategy, TabCapabilities, TabStrategy};
use crate::terminal_app::{
    build_auto_tab_script, build_system_events_tab_script, build_window_script,
    read_tabbing_preference, TabbingPreference,
};
use std::path::Path;
use std::process::Command;

#[allow(dead_code)]
pub fn escape_applescript_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

#[allow(dead_code)]
pub fn escape_powershell_single_quotes(s: &str) -> String {
    s.replace('\'', "''")
}

pub struct LaunchResult {
    pub label: String,
    pub kind: &'static str,
    pub success: bool,
    pub error: Option<String>,
    pub detail: Option<String>,
}

/// Render a launch/plan summary: a header line followed by one ✓/✗ line per
/// item. Success lines append ` — {detail}` when a detail is present; failure
/// lines append ` — {error}`. Returns the full block (trailing newline included).
pub fn render_results(header: &str, results: &[LaunchResult]) -> String {
    let mut out = format!("{header}\n");
    for r in results {
        if r.success {
            match &r.detail {
                Some(detail) => out.push_str(&format!("  ✓ {} {} — {}\n", r.kind, r.label, detail)),
                None => out.push_str(&format!("  ✓ {} {}\n", r.kind, r.label)),
            }
        } else {
            let err = r.error.as_deref().unwrap_or("unknown error");
            out.push_str(&format!("  ✗ {} {} — {}\n", r.kind, r.label, err));
        }
    }
    out
}

/// Long-running process name to watch as a readiness proxy for the emulator
/// that terminal #0 will cold-start. Returns `None` when we can't determine it,
/// in which case the caller treats the emulator as already warm (no extra wait).
pub fn emulator_watch_process(
    emulator: Option<&str>,
    ghostty_available: bool,
    ptyxis_available: bool,
    os: &str,
) -> Option<&'static str> {
    match emulator {
        Some("ghostty") => Some("ghostty"),
        Some("terminal") => native_terminal_process(os, ptyxis_available),
        Some(_) => None,
        None => {
            // Mirror launch_terminal's auto-selection: an unspecified emulator
            // only attempts Ghostty on macOS (the try_ghostty fallback is
            // cfg-gated there). On other platforms None always resolves to the
            // native terminal, so we must not watch Ghostty even when its
            // binary is installed — otherwise we'd poll a process we never
            // launch while Ptyxis/gnome-terminal actually cold-starts.
            if os == "macos" && ghostty_available {
                Some("ghostty")
            } else {
                native_terminal_process(os, ptyxis_available)
            }
        }
    }
}

fn native_terminal_process(os: &str, ptyxis_available: bool) -> Option<&'static str> {
    match os {
        "macos" => Some("Terminal"),
        // Ptyxis is single-instance; when it's the emulator that will actually
        // cold-start (no ghostty), watch `ptyxis` so tab #2+ don't race its
        // startup. Otherwise fall back to gnome-terminal's server process.
        "linux" if ptyxis_available => Some("ptyxis"),
        "linux" => Some("gnome-terminal-server"),
        "windows" => Some("WindowsTerminal.exe"),
        _ => None,
    }
}

/// Poll `ready` up to `attempts` times, sleeping `interval` between checks.
/// Returns true as soon as `ready` is satisfied, false if it never is.
pub fn poll_until(
    mut ready: impl FnMut() -> bool,
    attempts: u32,
    interval: std::time::Duration,
) -> bool {
    for _ in 0..attempts {
        if ready() {
            return true;
        }
        std::thread::sleep(interval);
    }
    false
}

#[cfg(not(target_os = "windows"))]
pub fn pgrep_args_for_process(name: &str) -> Vec<&str> {
    if name == "gnome-terminal-server" {
        vec!["-f", "gnome-terminal-server"]
    } else {
        vec!["-x", name]
    }
}

/// Whether a process with the expected name or command line is currently running.
fn process_running(name: &str) -> bool {
    #[cfg(not(target_os = "windows"))]
    {
        Command::new("pgrep")
            .args(pgrep_args_for_process(name))
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    #[cfg(target_os = "windows")]
    {
        Command::new("tasklist")
            .args(["/FI", &format!("IMAGENAME eq {name}")])
            .output()
            .map(|o| {
                String::from_utf8_lossy(&o.stdout)
                    .to_lowercase()
                    .contains(&name.to_lowercase())
            })
            .unwrap_or(false)
    }
}

/// After cold-starting the emulator, wait until its process is up and has had a
/// brief moment to settle, so subsequent terminals don't race its startup.
fn wait_for_emulator_ready(name: &str) {
    const ATTEMPTS: u32 = 30;
    let appeared = poll_until(
        || process_running(name),
        ATTEMPTS,
        std::time::Duration::from_millis(100),
    );
    if appeared {
        // Process exists; give it a moment to be ready to accept new windows.
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}

/// Human-readable description of a terminal: its resolved directory plus the
/// startup command when one is set. Shared by launch_project and plan_launch.
fn terminal_detail(resolved_path: &str, command: Option<&str>) -> String {
    match command {
        Some(cmd) => format!("{resolved_path} · {cmd}"),
        None => resolved_path.to_string(),
    }
}

fn make_placeholder_ctx(config: &ProjectConfig, project_root: &Path) -> PlaceholderContext {
    PlaceholderContext {
        root: project_root.to_string_lossy().to_string(),
        config: project_root
            .join(".quickdev.toml")
            .to_string_lossy()
            .to_string(),
        name: config.project.name.clone(),
        cwd: std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| project_root.to_string_lossy().to_string()),
    }
}

fn app_detail(path: &str, args: Option<&[String]>) -> String {
    match args {
        Some(args) if !args.is_empty() => format!("{path} · args: {}", args.join(" ")),
        _ => path.to_string(),
    }
}

/// Resolve what `launch_project` would launch, without spawning anything.
/// Terminals that fail path resolution (e.g. escaping the project root) are
/// returned as failures; everything else is a success carrying its `detail`.
pub fn plan_launch(config: &ProjectConfig, project_root: &Path) -> Vec<LaunchResult> {
    let mut results = Vec::new();
    for terminal in &config.terminals {
        match resolve_terminal_path(project_root, &terminal.path) {
            Ok(resolved_path) => results.push(LaunchResult {
                label: terminal.name.clone(),
                kind: "terminal",
                success: true,
                error: None,
                detail: Some(terminal_detail(&resolved_path, terminal.command.as_deref())),
            }),
            Err(e) => results.push(LaunchResult {
                label: terminal.name.clone(),
                kind: "terminal",
                success: false,
                error: Some(e),
                detail: None,
            }),
        }
    }
    let placeholder_ctx = make_placeholder_ctx(config, project_root);
    let root_str = project_root.to_string_lossy().to_string();
    for app in &config.applications {
        let resolved_args: Option<Vec<String>> = app
            .args
            .as_ref()
            .map(|a| resolve_app_args(a, &placeholder_ctx));
        let effective_args =
            effective_app_args(&app.name, &app.path, resolved_args.as_deref(), &root_str);
        results.push(LaunchResult {
            label: app.name.clone(),
            kind: "app",
            success: true,
            error: None,
            detail: Some(app_detail(&app.path, effective_args.as_deref())),
        });
    }
    results
}

pub fn launch_project(
    config: &ProjectConfig,
    project_root: &Path,
    global_emulator: Option<&str>,
) -> Vec<LaunchResult> {
    let mut results = Vec::new();

    // Every supported emulator (Ghostty, Terminal.app, gnome-terminal, Windows
    // Terminal) is single-instance. Terminal #0 cold-starts it; if the rest fire
    // before that instance is ready, they race its startup and open bare shells.
    // So when the emulator wasn't already running, wait for it to become ready
    // after terminal #0 before launching the others. Warm runs skip the wait.
    let first_emulator = config
        .terminals
        .first()
        .and_then(|t| t.emulator.as_deref())
        .or(global_emulator);
    let ghostty_available = launch_command_for_tool(std::env::consts::OS, "ghostty")
        .map(command_exists)
        .unwrap_or(false);
    let ptyxis_available = command_exists("ptyxis");
    let watch = emulator_watch_process(
        first_emulator,
        ghostty_available,
        ptyxis_available,
        std::env::consts::OS,
    );
    let emulator_was_running = watch.map(process_running).unwrap_or(true);
    let multiple = config.terminals.len() > 1;

    let mut terminal_results: Vec<Option<LaunchResult>> =
        config.terminals.iter().map(|_| None).collect();
    let mut prepared_terminals = Vec::new();

    for (i, terminal) in config.terminals.iter().enumerate() {
        let resolved_path = match resolve_terminal_path(project_root, &terminal.path) {
            Ok(path) => path,
            Err(e) => {
                terminal_results[i] = Some(LaunchResult {
                    label: terminal.name.clone(),
                    kind: "terminal",
                    success: false,
                    error: Some(e),
                    detail: None,
                });
                continue;
            }
        };
        prepared_terminals.push(PreparedTerminal {
            original_index: i,
            name: terminal.name.clone(),
            path: resolved_path,
            command: terminal.command.clone(),
            emulator: terminal.emulator.clone(),
        });
    }

    let tabs_launched = if prepared_terminals.len() > 1 {
        launch_terminal_tabs(&prepared_terminals, global_emulator).is_ok()
    } else {
        false
    };

    if tabs_launched {
        for terminal in &prepared_terminals {
            terminal_results[terminal.original_index] = Some(LaunchResult {
                label: terminal.name.clone(),
                kind: "terminal",
                success: true,
                error: None,
                detail: Some(terminal_detail(&terminal.path, terminal.command.as_deref())),
            });
        }
    } else {
        for (launch_index, terminal) in prepared_terminals.iter().enumerate() {
            if launch_index > 0 {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            let effective_emulator = terminal.emulator.as_deref().or(global_emulator);
            let result = launch_terminal(
                &terminal.path,
                terminal.command.as_deref(),
                launch_index,
                effective_emulator,
            );
            terminal_results[terminal.original_index] = Some(LaunchResult {
                label: terminal.name.clone(),
                kind: "terminal",
                success: result.is_ok(),
                error: result.err(),
                detail: Some(terminal_detail(&terminal.path, terminal.command.as_deref())),
            });

            if launch_index == 0 && multiple && !emulator_was_running {
                if let Some(name) = watch {
                    wait_for_emulator_ready(name);
                }
            }
        }
    }

    for result in terminal_results.into_iter().flatten() {
        results.push(result);
    }

    if prepared_terminals.is_empty() && multiple && !emulator_was_running {
        if let Some(name) = watch {
            wait_for_emulator_ready(name);
        }
    }

    if !prepared_terminals.is_empty() && tabs_launched && !emulator_was_running {
        if let Some(name) = watch {
            wait_for_emulator_ready(name);
        }
    }

    let placeholder_ctx = make_placeholder_ctx(config, project_root);
    let root_str = project_root.to_string_lossy().to_string();
    for app in &config.applications {
        let resolved_args: Option<Vec<String>> = app
            .args
            .as_ref()
            .map(|a| resolve_app_args(a, &placeholder_ctx));
        let effective_args =
            effective_app_args(&app.name, &app.path, resolved_args.as_deref(), &root_str);
        let result = launch_application(&app.path, effective_args.as_deref());
        results.push(LaunchResult {
            label: app.name.clone(),
            kind: "app",
            success: result.is_ok(),
            error: result.err(),
            detail: Some(app_detail(&app.path, effective_args.as_deref())),
        });
    }

    results
}

#[derive(Debug)]
struct PreparedTerminal {
    original_index: usize,
    name: String,
    path: String,
    command: Option<String>,
    emulator: Option<String>,
}

fn launch_terminal_tabs(
    terminals: &[PreparedTerminal],
    global_emulator: Option<&str>,
) -> Result<(), String> {
    let first_emulator = terminals
        .first()
        .and_then(|t| t.emulator.as_deref())
        .or(global_emulator);
    if !terminals
        .iter()
        .all(|t| t.emulator.as_deref().or(global_emulator) == first_emulator)
    {
        return Err("mixed terminal emulators cannot share tabs".to_string());
    }

    #[cfg(target_os = "macos")]
    {
        let caps = probe_tab_capabilities();
        match select_tab_strategy(std::env::consts::OS, first_emulator, &caps) {
            TabStrategy::AppleScriptTab => launch_ghostty_applescript_tabs(terminals),
            TabStrategy::TerminalAppTab => launch_terminal_app_tabs(terminals),
            TabStrategy::CliTab | TabStrategy::WindowOnly => {
                Err("unsupported tab emulator".to_string())
            }
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = terminals;
        let _ = global_emulator;
        Err("batch tabs are only handled on macOS".to_string())
    }
}

#[allow(dead_code)]
fn probe_tab_capabilities() -> TabCapabilities {
    TabCapabilities {
        ghostty_available: command_exists("ghostty"),
        ghostty_version: ghostty_version(),
        ghostty_applescript: ghostty_applescript_enabled(),
        ptyxis_available: command_exists("ptyxis"),
        gnome_terminal_available: command_exists("gnome-terminal"),
        wt_available: command_exists("wt"),
    }
}

fn ghostty_version() -> Option<String> {
    let ghostty_cmd = launch_command_for_tool(std::env::consts::OS, "ghostty")?;
    let output = Command::new(ghostty_cmd).arg("+version").output().ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn ghostty_applescript_enabled() -> bool {
    let Some(ghostty_cmd) = launch_command_for_tool(std::env::consts::OS, "ghostty") else {
        return false;
    };
    let output = Command::new(ghostty_cmd).arg("+show-config").output().ok();
    let Some(output) = output else {
        return true;
    };
    if !output.status.success() {
        return true;
    }
    !String::from_utf8_lossy(&output.stdout)
        .lines()
        .any(|line| line.trim() == "macos-applescript = false")
}

#[cfg(target_os = "macos")]
fn launch_ghostty_applescript_tabs(terminals: &[PreparedTerminal]) -> Result<(), String> {
    let resolved: Vec<ResolvedTerminal<'_>> = terminals
        .iter()
        .map(|terminal| ResolvedTerminal {
            cwd: &terminal.path,
            command: terminal.command.as_deref(),
        })
        .collect();
    let script = build_ghostty_script(&resolved)?;
    run_osascript(&script).map_err(|e| format!("Ghostty AppleScript tabs failed: {e}"))
}

#[cfg(target_os = "macos")]
fn launch_terminal_app_tabs(terminals: &[PreparedTerminal]) -> Result<(), String> {
    let resolved: Vec<ResolvedTerminal<'_>> = terminals
        .iter()
        .map(|terminal| ResolvedTerminal {
            cwd: &terminal.path,
            command: terminal.command.as_deref(),
        })
        .collect();
    let script = match read_tabbing_preference() {
        Some(TabbingPreference::Always) => build_auto_tab_script(&resolved),
        _ => build_system_events_tab_script(&resolved),
    };
    run_osascript(&script).map_err(|e| format!("Terminal.app tab launch failed: {e}"))
}

#[cfg(target_os = "macos")]
fn run_osascript(script: &str) -> Result<(), String> {
    // Wait for osascript and inspect its exit status: a failing AppleScript
    // (e.g. Automation/Accessibility not yet granted, or a script error) exits
    // non-zero. We must surface that so the caller falls back to separate
    // windows instead of reporting phantom success. `.spawn()` would ignore it.
    let output = Command::new("osascript")
        .args(["-e", script])
        .output()
        .map_err(|e| format!("osascript failed: {e}"))?;
    if output.status.success() {
        return Ok(());
    }
    let detail = String::from_utf8_lossy(&output.stderr);
    let detail = detail.trim();
    if detail.is_empty() {
        Err("osascript reported an error".to_string())
    } else {
        Err(detail.to_string())
    }
}

pub fn resolve_terminal_path(project_root: &Path, relative_path: &str) -> Result<String, String> {
    use std::path::Component;

    let rel = Path::new(relative_path);
    // `has_root()` catches POSIX-style absolute paths (e.g. "/etc/passwd") even on
    // Windows, where `is_absolute()` is false for them (it requires a drive prefix).
    if rel.is_absolute()
        || rel.has_root()
        || rel
            .components()
            .any(|c| matches!(c, Component::ParentDir | Component::Prefix(_)))
    {
        return Err(format!(
            "terminal path {relative_path:?} must stay inside the project root"
        ));
    }

    let joined = project_root.join(relative_path);
    // Normalize away `.` components without hitting the filesystem.
    let mut components: Vec<std::ffi::OsString> = Vec::new();
    for component in joined.components() {
        match component {
            Component::CurDir => {} // skip `.`
            // relative_path is already rejected if it contains `..`; this only
            // applies to any `..` in project_root itself (canonical in practice).
            Component::ParentDir => {
                components.pop();
            }
            c => components.push(c.as_os_str().to_os_string()),
        }
    }
    let normalized: std::path::PathBuf = components.iter().collect();
    Ok(normalized.to_string_lossy().to_string())
}

/// Values available for `{...}` placeholder substitution in application args.
#[derive(Debug)]
pub struct PlaceholderContext {
    pub root: String,
    pub config: String,
    pub name: String,
    pub cwd: String,
}

/// Substitute `{root}`, `{config}`, `{name}`, `{cwd}` placeholders inside each
/// app arg. A whole arg equal to "." is treated as `{root}` for backward
/// compatibility. Unknown `{...}` tokens are left untouched.
/// Placeholder token names recognized in application args (without braces).
pub const KNOWN_PLACEHOLDERS: &[&str] = &["root", "config", "name", "cwd"];

pub fn resolve_app_args(args: &[String], ctx: &PlaceholderContext) -> Vec<String> {
    args.iter()
        .map(|arg| {
            if arg == "." {
                return ctx.root.clone();
            }
            substitute_placeholders(arg, ctx)
        })
        .collect()
}

/// Single-pass placeholder substitution: each `{token}` is replaced at most
/// once and replacement values are never re-scanned (so a value containing a
/// token, e.g. a project name of "{cwd}", is not double-expanded). Unknown
/// `{...}` tokens and unmatched `{` are left untouched.
fn substitute_placeholders(input: &str, ctx: &PlaceholderContext) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(start) = rest.find('{') {
        out.push_str(&rest[..start]);
        let after = &rest[start..];
        if let Some(end) = after.find('}') {
            let token = &after[..=end]; // includes braces
            let replacement = match token {
                "{root}" => Some(ctx.root.as_str()),
                "{config}" => Some(ctx.config.as_str()),
                "{name}" => Some(ctx.name.as_str()),
                "{cwd}" => Some(ctx.cwd.as_str()),
                _ => None,
            };
            match replacement {
                Some(r) => out.push_str(r),
                None => out.push_str(token),
            }
            rest = &after[end + 1..];
        } else {
            out.push_str(after);
            rest = "";
        }
    }
    out.push_str(rest);
    out
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

fn launch_terminal(
    resolved_path: &str,
    command: Option<&str>,
    tab_index: usize,
    emulator: Option<&str>,
) -> Result<(), String> {
    if !Path::new(resolved_path).exists() {
        return Err(format!("path not found: {resolved_path}"));
    }

    match emulator {
        Some("ghostty") => return try_ghostty(resolved_path, command),
        Some("terminal") => return run_in_platform_terminal(resolved_path, command, tab_index),
        Some(other) => return Err(format!("unknown emulator: {other}")),
        None => {}
    }

    #[cfg(target_os = "macos")]
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

    let user_shell = std::env::var("SHELL").unwrap_or_else(|_| "sh".to_string());
    let escaped_cwd = cwd.replace('\'', "'\\''");
    let shell_command = match command {
        Some(cmd) => format!("cd '{escaped_cwd}' && {cmd}; exec {user_shell}"),
        None => format!("cd '{escaped_cwd}' && exec {user_shell}"),
    };

    Command::new(resolved)
        .args(["-e", &user_shell, "-lc", &shell_command])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("ghostty launch failed: {e}"))
}

fn run_in_platform_terminal(
    cwd: &str,
    command: Option<&str>,
    tab_index: usize,
) -> Result<(), String> {
    #[cfg(any(
        target_os = "windows",
        all(not(target_os = "macos"), not(target_os = "windows"))
    ))]
    let cmd_str = command.unwrap_or("");

    #[cfg(target_os = "macos")]
    {
        let _ = tab_index;
        let script = build_window_script(ResolvedTerminal {
            cwd,
            command: command.filter(|cmd| !cmd.is_empty()),
        });
        // Use the status-checking helper, not a fire-and-forget spawn: a denied
        // Automation permission makes osascript exit non-zero, and we must
        // report that as a failure rather than phantom success.
        run_osascript(&script).map_err(|e| format!("Terminal.app launch failed: {e}"))
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
            let safe_cwd = escape_powershell_single_quotes(cwd);
            let ps_cmd = if cmd_str.is_empty() {
                format!("Set-Location '{safe_cwd}'")
            } else {
                format!("Set-Location '{safe_cwd}'; {cmd_str}")
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
        let user_shell = std::env::var("SHELL").unwrap_or_else(|_| "sh".to_string());
        let escaped_cwd = cwd.replace('\'', "'\\''");
        let shell_command = if cmd_str.is_empty() {
            format!("cd '{escaped_cwd}' && exec {user_shell}")
        } else {
            format!("cd '{escaped_cwd}' && {cmd_str}; exec {user_shell}")
        };

        if command_exists("ptyxis") {
            let resolved = resolve_command("ptyxis").unwrap_or_else(|| "ptyxis".to_string());
            let mut cmd = Command::new(resolved);
            if tab_index > 0 {
                cmd.arg("--tab");
            }
            cmd.args(["-d", cwd, "--", &user_shell, "-lc", &shell_command]);
            if cmd.spawn().is_ok() {
                return Ok(());
            }
        }

        if command_exists("gnome-terminal") {
            let resolved =
                resolve_command("gnome-terminal").unwrap_or_else(|| "gnome-terminal".to_string());
            let mut cmd = Command::new(resolved);
            if tab_index > 0 {
                cmd.arg("--tab");
            }
            cmd.args(["--", &user_shell, "-lc", &shell_command]);
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
            cmd.args([&*user_shell, "-lc", &shell_command]);
            if cmd.spawn().is_ok() {
                return Ok(());
            }
        }

        Err("no terminal emulator found".to_string())
    }
}

/// Args to pass an editor tool (VS Code / Cursor / Zed): the configured args
/// when present and non-empty, otherwise the project root.
pub fn editor_args(args: Option<&[String]>, project_root: &str) -> Vec<String> {
    match args {
        Some(a) if !a.is_empty() => a.to_vec(),
        _ => vec![project_root.to_string()],
    }
}

pub fn effective_app_args(
    name: &str,
    path: &str,
    args: Option<&[String]>,
    project_root: &str,
) -> Option<Vec<String>> {
    match infer_tool_id(name, &normalize_path(path)) {
        Some(tid) if is_editor_tool(&tid) => Some(editor_args(args, project_root)),
        _ => args.map(|a| a.to_vec()),
    }
}

fn launch_application(path: &str, args: Option<&[String]>) -> Result<(), String> {
    let normalized_path = normalize_path(path);

    // Use path-only inference here: substituting the tool's CLI is only safe
    // when the configured path IS that tool. Wrapper paths (flatpak, Snap,
    // Squirrel Update.exe) whose display name merely matches must fall through
    // to the generic launch so their saved args (e.g. `flatpak run app.id`) run
    // against the wrapper, not against the CLI.
    let tool_id = infer_tool_id_from_path(&normalized_path);

    if let Some(ref tid) = tool_id {
        if let Some(cli) = launch_command_for_tool(std::env::consts::OS, tid) {
            if command_exists(cli) {
                let resolved = resolve_command(cli).unwrap_or_else(|| cli.to_string());
                let mut cmd = Command::new(&resolved);

                if let Some(a) = args {
                    for arg in a {
                        cmd.arg(arg);
                    }
                }

                return cmd
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
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
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
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
    cmd.stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("launch failed: {e}"))
}
