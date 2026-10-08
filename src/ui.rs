//! Terminal styling shared by every command's output.
//!
//! Renderers always emit ANSI styles; printing goes through `anstream`, which
//! strips them when stdout isn't a terminal or `NO_COLOR` is set.

use anstyle::{AnsiColor, Style};
use std::fmt::Display;

pub const GREEN: Style = AnsiColor::Green.on_default();
pub const RED: Style = AnsiColor::Red.on_default();
pub const YELLOW: Style = AnsiColor::Yellow.on_default();
pub const CYAN: Style = AnsiColor::Cyan.on_default();
pub const BOLD: Style = Style::new().bold();
pub const DIM: Style = Style::new().dimmed();

pub fn paint(style: Style, text: impl Display) -> String {
    format!("{style}{text}{style:#}")
}

pub fn ok(msg: impl Display) -> String {
    format!("{} {msg}", paint(GREEN, "✓"))
}

pub fn warn(msg: impl Display) -> String {
    format!("{} {msg}", paint(YELLOW, "⚠"))
}

pub fn fail(msg: impl Display) -> String {
    format!("{} {msg}", paint(RED, "✗"))
}

/// Section heading: bold title with an optional dimmed note after it.
pub fn heading(title: &str, note: &str) -> String {
    if note.is_empty() {
        paint(BOLD, title)
    } else {
        format!("{}  {}", paint(BOLD, title), paint(DIM, note))
    }
}

/// Left-align `text` to `width` columns. Pads after styling so ANSI codes
/// don't count toward the width.
pub fn pad(style: Style, text: &str, width: usize) -> String {
    let fill = width.saturating_sub(text.chars().count());
    format!("{}{}", paint(style, text), " ".repeat(fill))
}

/// Display form of a path: home directory abbreviated to `~`.
pub fn tilde(path: &str) -> String {
    match dirs::home_dir().and_then(|home| {
        std::path::Path::new(path)
            .strip_prefix(&home)
            .ok()
            .map(|rest| rest.to_path_buf())
    }) {
        Some(rest) if rest.as_os_str().is_empty() => "~".to_string(),
        Some(rest) => format!("~{}{}", std::path::MAIN_SEPARATOR, rest.display()),
        None => path.to_string(),
    }
}

/// `n` followed by `word`, pluralized with a trailing "s" when `n != 1`.
pub fn count(n: usize, word: &str) -> String {
    if n == 1 {
        format!("{n} {word}")
    } else {
        format!("{n} {word}s")
    }
}

/// The candidate most similar to `input`, if any is close enough to be worth
/// suggesting. Same measure and cutoff clap uses for mistyped subcommands.
pub fn closest<'a>(input: &str, candidates: impl IntoIterator<Item = &'a str>) -> Option<&'a str> {
    candidates
        .into_iter()
        .map(|c| (strsim::jaro(input, c), c))
        .filter(|(score, _)| *score > 0.7)
        .max_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, c)| c)
}

/// `message`, plus a clap-style "did you mean" tip when there is a suggestion.
pub fn with_suggestion(message: String, suggestion: Option<&str>) -> String {
    match suggestion {
        Some(s) => format!(
            "{message}\n\n  {} did you mean '{s}'?",
            paint(GREEN, "tip:")
        ),
        None => message,
    }
}

/// Coarse "how long ago" for a Unix timestamp: `just now`, `5m ago`, `3h ago`,
/// `2d ago`. A timestamp in the future (clock skew) reads as `just now`.
pub fn ago(then: u64, now: u64) -> String {
    let secs = now.saturating_sub(then);
    match secs {
        0..60 => "just now".to_string(),
        60..3_600 => format!("{}m ago", secs / 60),
        3_600..86_400 => format!("{}h ago", secs / 3_600),
        _ => format!("{}d ago", secs / 86_400),
    }
}
