# Interactive Add Command & Terminal Tab Launch — Design Spec

**Date:** 2026-05-25
**Status:** Approved

## Overview

Two improvements: (1) make `quickdev add` interactive with fzf-based type selection and app discovery, and (2) fix terminal launching to use tabs where the platform supports them.

## 1. Interactive `quickdev add`

### Problem

`quickdev add` requires specifying `terminal` or `app` subcommand with all arguments upfront. No way to browse installed apps.

### Solution

`quickdev add` (no subcommand) becomes interactive. Direct `quickdev add terminal ...` and `quickdev add app ...` still work for scripting.

### Interactive Flow — Terminal

1. fzf prompts: "Select what to add:" → `Terminal` / `Application`
2. Stdin prompt: `Path (. for current directory):` — user types a relative path or `.`
3. Stdin prompt: `Name for this tab:` — user types a label
4. Stdin prompt: `Startup command (optional, press Enter to skip):` — user types command or presses Enter
5. Terminal entry added, config saved

### Interactive Flow — Application (macOS)

1. fzf prompts: "Select what to add:" → `Terminal` / `Application`
2. Scan `/Applications` and `~/Applications` for `.app` bundles
3. fzf list with `[Enter path manually]` as the first option, followed by discovered apps sorted alphabetically
4. If user selects an app: name is derived from the `.app` bundle name (e.g., "Cursor" from "Cursor.app"), path is the full `/Applications/Cursor.app` path
5. If user selects `[Enter path manually]`: stdin prompt for the path, then stdin prompt for the name
6. Application entry added, config saved

### Interactive Flow — Application (Windows/Linux)

1. fzf prompts: "Select what to add:" → `Terminal` / `Application`
2. Stdin prompt: `Application path:` — user types the executable path
3. Stdin prompt: `Application name:` — user types a display name
4. Application entry added, config saved

### Command Structure Change

`AddKind` becomes `Option<AddKind>` in the `Commands::Add` variant. If `None`, run interactive mode. If `Some`, use the existing direct path.

## 2. Terminal Tab Launch Fix

### Problem

Current code spawns each terminal as an independent process. On platforms that support tabs, terminals should open as tabs in the same window instead of separate windows. Also, rapid spawning may cause race conditions where only the first terminal appears or commands don't run.

### Solution

Refactor `launch_project` terminal launching to be tab-aware per platform.

### Ghostty (macOS/Linux)

No tab support from CLI ([ghostty-org/ghostty#12136](https://github.com/ghostty-org/ghostty/issues/12136) closed as not planned). Each terminal opens a separate window. Add 100ms delay between spawns to prevent race conditions.

### macOS Terminal.app Fallback

Use AppleScript tab support:
- First terminal: `do script "cd <path> && <command>"` (opens new window)
- Subsequent terminals: `do script "cd <path> && <command>" in front window` (opens tab in existing window)

### Windows Terminal

- First terminal: `wt -d <path> cmd /K <command>`
- Subsequent terminals: `wt -w 0 new-tab -d <path> cmd /K <command>` (opens tab in most recent window)

### PowerShell 7 (Windows fallback)

Each `pwsh -NoExit` opens a separate window. Add 100ms delay between spawns.

### gnome-terminal (Linux fallback)

- First terminal: `gnome-terminal -- sh -lc "cd <path> && <command>"`
- Subsequent terminals: `gnome-terminal --tab -- sh -lc "cd <path> && <command>"`

### Other Linux terminals (konsole, alacritty, xterm)

No tab support from CLI. Separate windows with 100ms delay.

### Implementation

Change `launch_project` to pass an index to `launch_terminal` so it knows whether it's the first terminal (index 0 = new window) or a subsequent one (index > 0 = tab if supported). The signature becomes:

```rust
fn launch_terminal(resolved_path: &str, command: Option<&str>, tab_index: usize) -> Result<(), String>
```

Add `std::thread::sleep(Duration::from_millis(100))` between terminal spawns in `launch_project`.

## Files Changed

| File | Changes |
|------|---------|
| `src/main.rs` | Make `AddKind` optional, add `cmd_add_interactive()` with fzf type selection, stdin prompts, app picker |
| `src/launch.rs` | Add `tab_index` parameter to terminal launch functions, implement tab logic per platform, add inter-spawn delay |
| `src/fzf.rs` | No changes needed — existing `fzf_select_one` works for type selection and app picking |
| `src/apps.rs` (new) | macOS app discovery: scan `/Applications` and `~/Applications`, return sorted `Vec<(name, path)>` |
| `src/lib.rs` | Export `pub mod apps;` |

## App Discovery Module — `src/apps.rs`

macOS only (gated behind `#[cfg(target_os = "macos")]`):

- `discover_apps() -> Vec<(String, String)>` — scans `/Applications` and `~/Applications` for `.app` bundles, returns `(display_name, full_path)` sorted by name
- Display name derived by stripping `.app` suffix from the filename
- Skips hidden directories (starting with `.`)
- Returns empty vec on non-macOS platforms
- If no apps found (empty `/Applications`), falls back to manual path entry (same as Windows/Linux flow)

## Stdin Prompts

For free-text input (terminal path, name, command, manual app path), use simple stdin line reading:

```rust
fn prompt(message: &str) -> Result<String, String> {
    eprint!("{message}");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).map_err(|e| e.to_string())?;
    Ok(input.trim().to_string())
}
```

Print prompts to stderr so they don't interfere with stdout piping.
