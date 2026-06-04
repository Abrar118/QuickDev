#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TabStrategy {
    CliTab,
    AppleScriptTab,
    TerminalAppTab,
    GnomeTerminalLoadConfig,
    KittySession,
    WindowOnly,
}

#[derive(Debug, Clone, Default)]
pub struct TabCapabilities {
    /// Whether the `ghostty` binary is on PATH — mirrors the `command_exists`
    /// gate the per-terminal fallback launcher uses, distinct from whether
    /// Ghostty's AppleScript API is usable.
    pub ghostty_available: bool,
    pub ghostty_version: Option<String>,
    pub ghostty_applescript: bool,
    pub ptyxis_available: bool,
    pub gnome_terminal_available: bool,
    pub kitty_available: bool,
    pub wt_available: bool,
}

pub fn select_tab_strategy(
    os: &str,
    emulator: Option<&str>,
    caps: &TabCapabilities,
) -> TabStrategy {
    match os {
        "macos" => match emulator {
            Some("ghostty") | None if macos_ghostty_applescript_supported(caps) => {
                TabStrategy::AppleScriptTab
            }
            Some("ghostty") => TabStrategy::WindowOnly,
            // None with Ghostty installed but AppleScript unsupported (old
            // version, macos-applescript = false, or probe failure): defer to
            // the per-terminal fallback, which auto-detects Ghostty CLI windows
            // first. Choosing TerminalAppTab here would silently switch the
            // user off Ghostty. Only None with no Ghostty falls to Terminal.app.
            None if caps.ghostty_available => TabStrategy::WindowOnly,
            Some("terminal") | None => TabStrategy::TerminalAppTab,
            _ => TabStrategy::WindowOnly,
        },
        "linux" => match emulator {
            // kitty is preferred on Linux: a `--session` file opens one window
            // with per-tab dir+command. Checked before ptyxis/gnome-terminal.
            Some("kitty") if caps.kitty_available => TabStrategy::KittySession,
            Some("kitty") => TabStrategy::WindowOnly,
            Some("gnome-terminal") if caps.gnome_terminal_available => {
                TabStrategy::GnomeTerminalLoadConfig
            }
            Some("gnome-terminal") => TabStrategy::WindowOnly,
            // Explicit Ptyxis/Ghostty open windows (no single-window CLI tabs).
            Some("ptyxis") | Some("ghostty") => TabStrategy::WindowOnly,
            // Auto: kitty first, then never silently prefer gnome-terminal over
            // Ptyxis. Only tab when gnome-terminal is the terminal actually used.
            Some("terminal") | None if caps.kitty_available => TabStrategy::KittySession,
            Some("terminal") | None if caps.ptyxis_available => TabStrategy::WindowOnly,
            Some("terminal") | None if caps.gnome_terminal_available => {
                TabStrategy::GnomeTerminalLoadConfig
            }
            _ => TabStrategy::WindowOnly,
        },
        "windows" => {
            if caps.wt_available || matches!(emulator, Some("terminal") | None) {
                TabStrategy::CliTab
            } else {
                TabStrategy::WindowOnly
            }
        }
        _ => TabStrategy::WindowOnly,
    }
}

pub fn macos_ghostty_applescript_supported(caps: &TabCapabilities) -> bool {
    caps.ghostty_applescript
        && caps
            .ghostty_version
            .as_deref()
            .and_then(parse_version_major_minor)
            .map(|(major, minor)| major > 1 || (major == 1 && minor >= 3))
            .unwrap_or(false)
}

fn parse_version_major_minor(version: &str) -> Option<(u32, u32)> {
    let mut parts = version
        .split(|c: char| !(c.is_ascii_digit() || c == '.'))
        .find(|part| !part.is_empty())?
        .split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next().unwrap_or("0").parse().ok()?;
    Some((major, minor))
}
