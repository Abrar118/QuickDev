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

/// Strip characters that would let a title break out of its session-file line.
///
/// kitty parses the session file line-by-line, so a newline or carriage return
/// embedded in a title (e.g. from a hostile `.quickdev.toml`) could inject extra
/// directives such as a rogue `launch`. There is no line-continuation escape in
/// the session format, so control characters cannot be encoded — we drop them.
/// Titles are cosmetic, so stripping is preferable to failing the launch.
pub fn sanitize_title(title: &str) -> String {
    title.chars().filter(|c| !c.is_control()).collect()
}

/// Build a kitty session file: one OS window, one tab per entry.
///
/// The first tab is implicit — kitty already opens a tab for the first `launch`,
/// so emitting a leading `new_tab` would create an empty extra tab. Each
/// subsequent tab is introduced with `new_tab <title>`.
///
/// Titles are sanitized of control characters ([`sanitize_title`]) so a title
/// cannot break out of its directive line; the `--title` value is additionally
/// single-quote escaped, while `new_tab` takes the rest of the line literally.
///
/// # Precondition
///
/// Each `wrapper_path` must not contain single quotes (`'`); it is emitted
/// verbatim inside single quotes. QuickDev always generates `wrapper_path` under
/// a temp directory, so this holds in practice.
pub fn build_session(tabs: &[SessionTab<'_>]) -> String {
    let mut s = String::new();
    for (i, tab) in tabs.iter().enumerate() {
        let title = sanitize_title(tab.title);
        if i == 0 {
            s.push_str(&format!(
                "launch --title '{}' /bin/sh '{}'\n",
                escape_session_value(&title),
                tab.wrapper_path
            ));
        } else {
            s.push_str(&format!("new_tab {}\n", title));
            s.push_str(&format!("launch /bin/sh '{}'\n", tab.wrapper_path));
        }
    }
    s
}

/// Write per-tab wrapper scripts and the session file into `dir`. Returns the
/// session file path. Exposed for testing.
#[cfg(target_os = "linux")]
pub fn write_session(
    dir: &std::path::Path,
    tabs: &[KittyTab<'_>],
) -> Result<std::path::PathBuf, String> {
    use crate::gnome_terminal::build_wrapper_script;
    use std::os::unix::fs::PermissionsExt;

    let mut session_tabs: Vec<(String, String)> = Vec::with_capacity(tabs.len());
    for (i, tab) in tabs.iter().enumerate() {
        let wrapper_path = dir.join(format!("tab{i}.sh"));
        let body = build_wrapper_script(tab.cwd, tab.command);
        std::fs::write(&wrapper_path, body)
            .map_err(|e| format!("failed to write wrapper script: {e}"))?;
        std::fs::set_permissions(&wrapper_path, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("failed to chmod wrapper script: {e}"))?;
        session_tabs.push((
            tab.title.to_string(),
            wrapper_path.to_string_lossy().into_owned(),
        ));
    }

    let session_tabs_ref: Vec<SessionTab<'_>> = session_tabs
        .iter()
        .map(|(title, path)| SessionTab {
            title,
            wrapper_path: path,
        })
        .collect();
    let session_path = dir.join("session.kitty");
    std::fs::write(&session_path, build_session(&session_tabs_ref))
        .map_err(|e| format!("failed to write session file: {e}"))?;
    Ok(session_path)
}

/// Open one kitty window with one tab per `tabs` entry via `--session`.
///
/// Fire-and-forget: kitty is a foreground/GUI process (unlike gnome-terminal's
/// client/server model), so `.output()` would block until the window closes.
/// stdout/stderr are nulled to avoid macOS `SEL:` spam and tty coupling. `Err`
/// is returned only when the binary cannot be spawned, so the caller falls back
/// to per-window launches when kitty is missing.
#[cfg(target_os = "linux")]
pub fn launch_kitty_session(tabs: &[KittyTab<'_>]) -> Result<(), String> {
    use crate::adapters::resolve_command;
    use std::process::{Command, Stdio};

    if tabs.is_empty() {
        return Err("no terminals to launch".to_string());
    }
    let dir = std::env::temp_dir().join(format!("quickdev-{}", std::process::id()));
    std::fs::create_dir_all(&dir).map_err(|e| format!("failed to create temp dir: {e}"))?;
    let session = write_session(&dir, tabs)?;

    let resolved = resolve_command("kitty").ok_or("kitty not found".to_string())?;
    Command::new(resolved)
        .arg(format!("--session={}", session.display()))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("kitty launch failed: {e}"))
}
