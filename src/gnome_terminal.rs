//! Builds a GNOME Terminal `--load-config` session file that opens one window
//! with N tabs, each running a generated wrapper script. The wrapper carries the
//! per-tab working directory and command, so the keyfile only ever references a
//! fixed script path — sidestepping GKeyFile + g_shell_parse_argv quoting.

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
