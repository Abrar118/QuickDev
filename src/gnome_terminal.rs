//! Builds a GNOME Terminal `--load-config` session file that opens one window
//! with N tabs, each running a generated wrapper script. The wrapper carries the
//! per-tab working directory and command, so the keyfile only ever references a
//! fixed script path — sidestepping GKeyFile + g_shell_parse_argv quoting.

#[cfg(target_os = "linux")]
use crate::adapters::resolve_command;
#[cfg(target_os = "linux")]
use std::path::{Path, PathBuf};
#[cfg(target_os = "linux")]
use std::process::{Command, Stdio};

#[derive(Debug, Clone, Copy)]
pub struct GnomeTab<'a> {
    pub title: &'a str,
    pub cwd: &'a str,
    pub command: Option<&'a str>,
}

/// One tab's entry in the load-config file: its title and the path of the
/// wrapper script that `Command` will run.
#[derive(Debug, Clone, Copy)]
pub struct LoadConfigTab<'a> {
    pub title: &'a str,
    pub wrapper_path: &'a str,
}

/// Escape a single-line value for a GKeyFile string field (backslash + newline).
pub fn escape_gkeyfile_value(value: &str) -> String {
    value.replace('\\', "\\\\").replace('\n', "\\n")
}

/// Body of a per-tab wrapper script: cd into the dir, run the optional command,
/// then exec a login shell so the tab stays open after the command exits.
pub fn build_wrapper_script(cwd: &str, command: Option<&str>) -> String {
    let escaped_cwd = cwd.replace('\'', "'\\''");
    let mut script = String::from("#!/bin/sh\n");
    script.push_str(&format!("cd '{escaped_cwd}' || exit 1\n"));
    if let Some(cmd) = command {
        script.push_str(cmd);
        script.push('\n');
    }
    script.push_str("exec \"${SHELL:-/bin/sh}\" -l\n");
    script
}

/// The full `--load-config` keyfile: one window grouping all tabs.
///
/// # Precondition
///
/// The caller-supplied `wrapper_path` in each [`LoadConfigTab`] must not
/// contain single-quote characters (`'`). The path is emitted verbatim inside
/// single quotes (`Command=/bin/sh '<wrapper_path>'`) so a single-quote in the
/// path would produce a malformed GKeyFile line. QuickDev always generates
/// `wrapper_path` under a temp directory, so this invariant always holds in
/// practice.
pub fn build_load_config(tabs: &[LoadConfigTab<'_>]) -> String {
    let ids: Vec<String> = (0..tabs.len()).map(|i| format!("Terminal{i}")).collect();
    let mut s = String::new();
    s.push_str("[GNOME Terminal Configuration]\n");
    s.push_str("Version=1\n");
    s.push_str("CompatVersion=1\n");
    s.push_str("Windows=Window0;\n\n");
    s.push_str("[Window0]\n");
    s.push_str(&format!("Terminals={};\n", ids.join(";")));
    s.push_str("ActiveTerminal=Terminal0\n\n");
    for (i, tab) in tabs.iter().enumerate() {
        s.push_str(&format!("[Terminal{i}]\n"));
        s.push_str(&format!("Title={}\n", escape_gkeyfile_value(tab.title)));
        // Single-quote the path so a temp dir with spaces survives
        // g_shell_parse_argv.
        s.push_str(&format!("Command=/bin/sh '{}'\n\n", tab.wrapper_path));
    }
    s
}

/// Write the wrapper scripts and the load-config file into `dir`. Returns the
/// path of the load-config file. Exposed for testing.
#[cfg(target_os = "linux")]
pub fn write_session(dir: &Path, tabs: &[GnomeTab<'_>]) -> Result<PathBuf, String> {
    use std::os::unix::fs::PermissionsExt;

    let mut config_tabs: Vec<(String, String)> = Vec::with_capacity(tabs.len());
    for (i, tab) in tabs.iter().enumerate() {
        let wrapper_path = dir.join(format!("tab{i}.sh"));
        let body = build_wrapper_script(tab.cwd, tab.command);
        std::fs::write(&wrapper_path, body)
            .map_err(|e| format!("failed to write wrapper script: {e}"))?;
        std::fs::set_permissions(&wrapper_path, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("failed to chmod wrapper script: {e}"))?;
        config_tabs.push((
            tab.title.to_string(),
            wrapper_path.to_string_lossy().into_owned(),
        ));
    }

    let load_config_tabs: Vec<LoadConfigTab<'_>> = config_tabs
        .iter()
        .map(|(title, path)| LoadConfigTab {
            title,
            wrapper_path: path,
        })
        .collect();
    let conf_path = dir.join("tabs.conf");
    std::fs::write(&conf_path, build_load_config(&load_config_tabs))
        .map_err(|e| format!("failed to write load-config file: {e}"))?;
    Ok(conf_path)
}

/// Open one gnome-terminal window with one tab per `tabs` entry via
/// `--load-config`. Status-checked: a non-zero exit is reported as an error so
/// the caller can fall back to per-terminal windows.
#[cfg(target_os = "linux")]
pub fn launch_gnome_terminal_load_config(tabs: &[GnomeTab<'_>]) -> Result<(), String> {
    if tabs.is_empty() {
        return Err("no terminals to launch".to_string());
    }
    let dir = std::env::temp_dir().join(format!("quickdev-{}", std::process::id()));
    std::fs::create_dir_all(&dir).map_err(|e| format!("failed to create temp dir: {e}"))?;
    let conf = write_session(&dir, tabs)?;

    let resolved =
        resolve_command("gnome-terminal").ok_or("gnome-terminal not found".to_string())?;
    let output = Command::new(resolved)
        .arg(format!("--load-config={}", conf.display()))
        .stdout(Stdio::null())
        .output()
        .map_err(|e| format!("gnome-terminal launch failed: {e}"))?;
    if output.status.success() {
        return Ok(());
    }
    let detail = String::from_utf8_lossy(&output.stderr);
    let detail = detail.trim();
    if detail.is_empty() {
        Err("gnome-terminal --load-config reported an error".to_string())
    } else {
        Err(detail.to_string())
    }
}
