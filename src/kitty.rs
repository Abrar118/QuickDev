//! Builds a kitty `--session` file that opens one OS window with N tabs, each
//! running a generated wrapper script. The wrapper carries the per-tab working
//! directory and command, so the session file only references a fixed script
//! path — sidestepping kitty's shell-word parsing of `launch` lines.

#[derive(Debug, Clone, Copy)]
pub struct KittyTab<'a> {
    pub title: &'a str,
    pub cwd: &'a str,
    pub command: Option<&'a str>,
}

/// One tab's entry in the session file: its title and the wrapper script path
/// that the tab's `launch` line will run.
#[derive(Debug, Clone, Copy)]
pub struct SessionTab<'a> {
    pub title: &'a str,
    pub wrapper_path: &'a str,
}

/// Escape a value for inclusion inside single quotes on a kitty session
/// `launch`/`--title` line (kitty parses these lines with shell-like quoting).
pub fn escape_session_value(value: &str) -> String {
    value.replace('\'', "'\\''")
}

/// Build a kitty session file: one OS window, one tab per entry.
///
/// The first tab is implicit — kitty already opens a tab for the first `launch`,
/// so emitting a leading `new_tab` would create an empty extra tab. Each
/// subsequent tab is introduced with `new_tab <title>`.
///
/// # Precondition
///
/// Each `wrapper_path` must not contain single quotes (`'`); it is emitted
/// verbatim inside single quotes. QuickDev always generates `wrapper_path` under
/// a temp directory, so this holds in practice.
pub fn build_session(tabs: &[SessionTab<'_>]) -> String {
    let mut s = String::new();
    for (i, tab) in tabs.iter().enumerate() {
        if i == 0 {
            s.push_str(&format!(
                "launch --title '{}' /bin/sh '{}'\n",
                escape_session_value(tab.title),
                tab.wrapper_path
            ));
        } else {
            s.push_str(&format!("new_tab {}\n", tab.title));
            s.push_str(&format!("launch /bin/sh '{}'\n", tab.wrapper_path));
        }
    }
    s
}
