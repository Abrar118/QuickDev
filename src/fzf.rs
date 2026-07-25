use crate::adapters::command_exists;
use std::io::Write;
use std::process::{Command, Stdio};

/// Sentinel error returned when the user cancels an fzf picker (ESC / ctrl-c or
/// an empty selection). `main()` maps this to a clean "Cancelled." + exit 0.
pub const CANCELLED: &str = "__quickdev_fzf_cancelled__";

pub fn is_cancellation(err: &str) -> bool {
    err == CANCELLED
}

pub fn check_fzf() -> bool {
    command_exists("fzf")
}

pub fn fzf_install_hint() -> String {
    let os_hint = if cfg!(target_os = "macos") {
        "brew install fzf"
    } else if cfg!(target_os = "windows") {
        "choco install fzf"
    } else {
        "apt install fzf"
    };
    format!("fzf is required for interactive selection.\nInstall: {os_hint}")
}

/// Run fzf over `items` with `extra_args`, returning the selected lines.
fn run_fzf(items: &[String], header: &str, extra_args: &[&str]) -> Result<Vec<String>, String> {
    if !check_fzf() {
        return Err(fzf_install_hint());
    }
    if items.is_empty() {
        return Err("no items to select from".to_string());
    }

    let input = items.join("\n");

    let mut child = Command::new("fzf")
        .args(["--header", header, "--height", "~50%", "--reverse"])
        .args(extra_args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| format!("failed to start fzf: {e}"))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(input.as_bytes())
            .map_err(|e| format!("failed to write to fzf: {e}"))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|e| format!("fzf failed: {e}"))?;

    if !output.status.success() {
        return Err(CANCELLED.to_string());
    }

    let selected: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();

    if selected.is_empty() {
        return Err(CANCELLED.to_string());
    }

    Ok(selected)
}

pub fn fzf_select_one(items: &[String], header: &str) -> Result<String, String> {
    let mut selected = run_fzf(items, header, &[])?;
    Ok(selected.remove(0))
}

/// Strip characters that would break the one-item-per-line picker protocol.
///
/// fzf reads one row per line and splits the hidden index on a tab, so a newline
/// or tab inside user data would forge or corrupt rows. Every picker that
/// displays config-derived text runs it through here.
pub fn sanitize_row(value: &str) -> String {
    value.chars().filter(|c| !c.is_control()).collect()
}

/// Args that hide the leading index column from the user.
const INDEXED_ARGS: [&str; 4] = ["--delimiter", "\t", "--with-nth", "2.."];

fn indexed_rows(items: &[String]) -> Vec<String> {
    items
        .iter()
        .enumerate()
        .map(|(i, item)| format!("{i}\t{}", sanitize_row(item)))
        .collect()
}

/// Present `items` and return the *positions* of the rows the user picked.
///
/// Each row is sent as `<index>\t<display>` with fzf told to render only field 2
/// onward, so the index round-trips invisibly. Recovering a selection by parsing
/// its display text is unreliable — an item's name may legally contain whatever
/// separator the display uses, in which case the item can never be selected.
pub fn fzf_select_multi_indexed(items: &[String], header: &str) -> Result<Vec<usize>, String> {
    let rows = indexed_rows(items);
    let mut args = vec!["--multi"];
    args.extend_from_slice(&INDEXED_ARGS);

    let selected = run_fzf(&rows, header, &args)?;

    parse_indexed_selection(&selected, items.len())
}

/// Single-selection counterpart of [`fzf_select_multi_indexed`].
pub fn fzf_select_one_indexed(items: &[String], header: &str) -> Result<usize, String> {
    let rows = indexed_rows(items);

    let selected = run_fzf(&rows, header, &INDEXED_ARGS)?;

    parse_indexed_selection(&selected, items.len())?
        .into_iter()
        .next()
        .ok_or_else(|| CANCELLED.to_string())
}

/// Recover row positions from the hidden index column of fzf's output.
/// Anything that isn't an in-range index is an error rather than a silent skip:
/// acting on a partial selection would remove or launch the wrong items.
pub fn parse_indexed_selection(selected: &[String], count: usize) -> Result<Vec<usize>, String> {
    selected
        .iter()
        .map(|line| {
            line.split_once('\t')
                .and_then(|(index, _)| index.trim().parse::<usize>().ok())
                .filter(|index| *index < count)
                .ok_or_else(|| format!("unrecognized selection: {line}"))
        })
        .collect()
}
