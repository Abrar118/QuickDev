use clap_complete::aot::Shell;
use clap_complete::env::Shells;

/// Print the registration script — the same one `COMPLETE=<shell> quickdev`
/// emits. It calls back into `quickdev` on each TAB, so it is meant to be
/// sourced at shell startup rather than saved: it then always matches the
/// installed version.
pub(crate) fn cmd_completions(shell: Shell) -> Result<(), String> {
    let name = shell.to_string();
    let shells = Shells::builtins();
    let completer = shells
        .completer(&name)
        .ok_or_else(|| format!("completions are not available for {name}"))?;
    completer
        .write_registration(
            "COMPLETE",
            "quickdev",
            "quickdev",
            "quickdev",
            &mut std::io::stdout(),
        )
        .map_err(|e| format!("failed to write completions: {e}"))
}
