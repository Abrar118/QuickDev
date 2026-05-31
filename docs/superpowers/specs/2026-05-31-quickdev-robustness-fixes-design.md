# QuickDev Robustness Fixes — Design

**Date:** 2026-05-31
**Status:** Approved

## Summary

Seven correctness fixes to existing QuickDev behavior. No new features and no
architectural changes. One new dependency (`shell-words`). All fixes target
real defects verified against the current source.

Priority labels: **P0** = breaks real, realistic inputs; **P1** = fragile or
incorrect on less-common but valid inputs.

## Out of Scope

- New subcommands or flags (beyond honoring existing `--all`).
- Refactoring `main.rs` size or restructuring modules.
- Any change to `wt` or ghostty launch paths (already correct).

## Fixes

### 1. (P0) Project picker handles names with spaces — `config.rs`

**Problem:** `fzf_select_project` formats items as `format!("{:<20} {}", p.name,
p.path)` then extracts the project via `selected.split_whitespace().next()`.
Project names are derived from directory names, so spaces are realistic; the
current code treats only the first word as the name and fails to match.

**Fix:** Use index-prefixed display items and parse the index, not the name.

```rust
let items: Vec<String> = global.projects.iter().enumerate()
    .map(|(i, p)| format!("{i}: {}    {}", p.name, p.path))
    .collect();

let selected = fzf::fzf_select_one(&items, "Select a project:")?;
let index: usize = selected
    .split_once(':')
    .and_then(|(i, _)| i.trim().parse().ok())
    .ok_or("invalid selection")?;
let entry = global.projects.get(index).ok_or("invalid selection")?;
```

The `split_whitespace().next()` name-extraction path is removed entirely.

### 2. (P0) Named launch shows the picker — `main.rs`

**Problem:** `cmd_launch` gates the interactive multi-select on
`if !all && project.is_none()`. So `quickdev launch` is interactive but
`quickdev launch my-api` launches everything, contradicting the README.

**Decision:** Named launch shows the picker by default; `--all` is the only
bypass.

**Fix:** Change the gate to `if !all`. The picker runs whether the project came
from a name argument, the current directory, or the project picker. Update the
README so `quickdev launch my-api` is documented as "select items for a project
by name" and `--all` as the launch-everything path.

### 3. (P0) Re-register keeps local and global name in sync — `main.rs`

**Problem:** When `.quickdev.toml` exists but the project is not indexed,
`cmd_init` computes a unique global name (e.g. `api` → `api-2`) and pushes it to
the global index, but never updates the local config. The local file still says
`api` while global says `api-2`.

**Fix:** After computing the unique name, write it back to the local config
before updating the global index.

```rust
let mut existing = load_project_config(&config_path)?;
let project_name = unique_project_name(&existing.project.name, &global);
if existing.project.name != project_name {
    existing.project.name = project_name.clone();
    save_project_config(&config_path, &existing)?;
}
global.projects.push(GlobalProjectEntry { name: project_name.clone(), path: cwd_str });
save_global_config(&global_path, &global)?;
```

### 4. (P1) Interactive app args respect quotes — `main.rs`

**Problem:** The interactive app-add prompt parses arguments with
`args_input.split_whitespace()`, which splits inside quotes. `--profile "Dev
User"` becomes `["--profile", "\"Dev", "User\""]`. (The `quickdev add app` flag
path uses clap `num_args = 1..` and is already correct; only the interactive
prompt is affected.)

**Fix:** Use `shell_words::split`.

```rust
let args = if args_input.is_empty() {
    None
} else {
    Some(shell_words::split(&args_input).map_err(|e| format!("invalid arguments: {e}"))?)
};
```

### 5. (P1) `$EDITOR`/`$VISUAL` with arguments — `main.rs`

**Problem:** `cmd_edit` runs `Command::new(&editor).arg(&config_path)`, so
`EDITOR="code --wait"` tries to execute a binary literally named `code --wait`.
`$VISUAL` is also ignored.

**Fix:** Prefer `$VISUAL`, fall back to `$EDITOR`, then `vi`. Split the editor
string with `shell_words::split`; the first token is the program, remaining
tokens are leading arguments, then append the config path.

```rust
let editor = std::env::var("VISUAL")
    .or_else(|_| std::env::var("EDITOR"))
    .unwrap_or_else(|_| "vi".to_string());
let parts = shell_words::split(&editor).map_err(|e| format!("invalid editor command: {e}"))?;
let (program, leading) = parts.split_first().ok_or("editor command is empty")?;
std::process::Command::new(program)
    .args(leading)
    .arg(&config_path)
    .status()
    .map_err(|e| format!("failed to open editor '{}': {}", editor, e))?;
```

An empty or whitespace-only editor string errors cleanly.

### 6. (P1) Shell escaping helpers — `launch.rs`

**Problem:** Two launch sites build shell/script strings with incomplete
escaping:
- macOS Terminal.app (`run_in_platform_terminal`) escapes only `"` for
  AppleScript, not `\`.
- Windows pwsh embeds cwd in single quotes (`Set-Location '{cwd}'`) without
  escaping embedded `'`.

`wt` (passes `-d cwd` as a separate argument) and ghostty (already does POSIX
`'\''` escaping) are safe and unchanged.

**Fix:** Add two helpers and apply them at the two unsafe sites.

```rust
fn escape_applescript_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn escape_powershell_single_quotes(s: &str) -> String {
    s.replace('\'', "''")
}
```

- macOS: apply `escape_applescript_string` to the full script string before
  embedding it in the `do script "..."` AppleScript (the inner `cd '...'`
  POSIX single-quote escaping stays as-is).
- Windows pwsh: wrap cwd with `escape_powershell_single_quotes`.

### 7. (P1) Reject terminal paths that escape the project root — `launch.rs`

**Problem:** `resolve_terminal_path` joins and normalizes components, resolving
`..` and silently accepting absolute paths. A path like `../../outside` escapes
the root, contradicting the README's "relative to project root" wording.

**Decision:** Reject traversal and absolute paths.

**Fix:** `resolve_terminal_path` returns `Result<String, String>`. Reject any
absolute path or `..` component before normalizing.

```rust
use std::path::Component;
let rel = Path::new(relative_path);
if rel.is_absolute() || rel.components().any(|c| c == Component::ParentDir) {
    return Err("terminal path must stay inside the project root".to_string());
}
```

`launch_project` surfaces the error as a failed `LaunchResult` for that terminal
(consistent with existing per-item error handling) rather than aborting the
whole launch. Other terminals and apps still launch.

## Dependency

Add `shell-words` to `Cargo.toml`. Small, widely used, POSIX-correct word
splitting. Used by fixes #4 and #5.

## Testing

Unit tests in the existing `tests/` style:

- `resolve_terminal_path` — accepts `.`, `./src`, nested relatives; rejects
  `../x`, `/abs`, and `a/../../b`.
- `escape_applescript_string` and `escape_powershell_single_quotes` — direct
  assertions for backslash, double-quote, and single-quote inputs.
- App-arg splitting via `shell_words::split` — quoted args round-trip
  (`--profile "Dev User"` → `["--profile", "Dev User"]`).
- Picker index parsing — a name containing spaces selects the correct project.
- Re-register name sync — when the base name collides, the local config is
  rewritten to the unique name.

Run `cargo fmt`, `cargo clippy`, and the full `cargo test` suite before
committing. All existing tests must continue to pass.
