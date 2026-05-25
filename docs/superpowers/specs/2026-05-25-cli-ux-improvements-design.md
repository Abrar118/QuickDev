# QuickDev CLI UX Improvements — Design Spec

**Date:** 2026-05-25
**Status:** Approved

## Overview

Four targeted UX improvements to the QuickDev CLI: better help text, interactive fzf-based removal, fzf project picker fallback when no local config exists, and instructional comments in generated TOML files.

## 1. Help — Detailed Command Usage

### Problem

`quickdev --help` shows only command names. Users must run `quickdev add terminal --help` to discover arguments.

### Solution

Add `after_help` with examples to the top-level command and key subcommands. Add `arg_required_else_help` where appropriate.

Top-level help output:

```
quickdev — Manage and launch project terminal/app configurations

Usage: quickdev <COMMAND>

Commands:
  init      Create .quickdev.toml in current directory
  launch    Launch all terminals and apps for a project
  list      List all indexed projects
  add       Add a terminal or application entry
  remove    Remove terminals/apps (interactive picker)
  edit      Open .quickdev.toml in $EDITOR

Examples:
  quickdev init                                         Initialize a project
  quickdev launch                                       Launch current project
  quickdev launch my-api                                Launch a project by name
  quickdev add terminal server . --command "npm start"  Add a terminal tab
  quickdev add app Cursor /Applications/Cursor.app      Add an application
  quickdev remove                                       Interactive removal picker
```

Subcommand help (e.g., `quickdev add terminal --help`) already shows full arguments via clap derive — add examples via `after_help` to those as well.

## 2. Interactive Remove with fzf

### Problem

`quickdev remove terminal <name>` requires knowing the exact name. No way to browse what's configured.

### Solution

`quickdev remove` (no subcommand) becomes an interactive fzf picker.

**Flow:**
1. Find the project's `.quickdev.toml` (walk parents, or fzf project picker if not found — see Section 3)
2. Build a display list of all terminals and apps:
   ```
   [terminal] dev — . (cargo run -- --help)
   [terminal] tests — . (cargo test)
   [app] Cursor — /Applications/Cursor.app
   ```
3. Pipe the list to `fzf --multi` (space to toggle, enter to confirm)
4. Parse fzf output to determine which items were selected
5. Remove selected items from the config, save

**Non-interactive path preserved:** `quickdev remove terminal <name>` and `quickdev remove app <name>` still work for scripting.

**Command structure change:** `remove`'s subcommand becomes optional. In clap, change `RemoveKind` from a required subcommand to `Option<RemoveKind>`. If `None`, run interactive mode. If `Some(Terminal { name })` or `Some(App { name })`, use the direct removal path. Use `#[command(subcommand_required = false)]` on the `Remove` variant.

**fzf not installed:** Print error with install instructions:
```
error: fzf is required for interactive selection
Install: brew install fzf (macOS), apt install fzf (Linux), choco install fzf (Windows)
```

## 3. fzf Project Picker Fallback

### Problem

Commands that need a `.quickdev.toml` (launch, add, remove, edit) error out when no config is found in cwd or parents. User must `cd` to the project directory first.

### Solution

When `find_project_config` fails, fall back to an fzf-based project picker.

**Flow:**
1. Load global index (`~/Documents/quickdev/config.toml`)
2. If empty, error: `"No projects registered. Run 'quickdev init' in a project directory."`
3. Build display lines: `"ProjectName   /absolute/path"`
4. Pipe to `fzf` (single-select, no `--multi`)
5. Parse selected line to get project name/path
6. Use that project's `.quickdev.toml` for the command

**Affected commands:** `launch`, `add`, `remove`, `edit` — all four commands that call `find_project_config`.

**fzf not installed:** Fall back to current behavior — error with "no .quickdev.toml found" plus a note: `"Tip: install fzf for interactive project selection"`.

### Implementation

Extract a shared helper function `resolve_project_config()` that:
1. Tries `find_project_config(&cwd)` first
2. On failure, calls `fzf_select_project()` which loads global index and runs fzf
3. Returns `(config_path, project_root)` or errors

All four commands call this instead of `find_project_config` directly.

## 4. TOML Comments

### Problem

Generated `.quickdev.toml` has no documentation. Users don't know what fields are available or what they mean.

### Solution

Prepend an instructional comment header to all generated `.quickdev.toml` files.

**Comment header:**

```toml
# QuickDev project configuration
# Edit this file directly or use: quickdev add, quickdev remove
#
# [project]
#   name = Display name for this project
#
# [[terminals]]
#   name    = Label for this terminal tab
#   path    = Working directory relative to project root (e.g., ".", "./src")
#   command = (optional) Startup command to run when the terminal opens
#
# [[applications]]
#   name = Application display name
#   path = Executable path or .app bundle (e.g., "/Applications/Cursor.app")
#   args = (optional) Arguments list (e.g., ["."] to open project root)
```

**Implementation:**

Modify `save_project_config` in `config.rs`:
1. Serialize the config with `toml::to_string_pretty`
2. When writing, check if the file already has a comment header (starts with `# QuickDev`)
3. If no header exists, prepend the comment block
4. If header exists, preserve it (read existing comment lines, re-prepend after serialization)

This means `init` creates a commented file, and `add`/`remove` preserve the comments through edits.

## Files Changed

| File | Changes |
|------|---------|
| `src/main.rs` | Add `after_help` examples to clap structs, refactor `remove` to support both interactive and direct modes, replace `find_project_config` calls with `resolve_project_config` |
| `src/config.rs` | Add `resolve_project_config()`, modify `save_project_config()` to handle comment headers, add `fzf_select_project()` |
| `src/fzf.rs` (new) | fzf integration: `check_fzf()`, `fzf_select_one()`, `fzf_select_multi()`, `fzf_install_hint()` |

## fzf Integration Module

New file `src/fzf.rs` encapsulates all fzf interaction:

- `check_fzf() -> bool` — checks if fzf is installed via `which`/`where`
- `fzf_select_one(items: &[String], header: &str) -> Result<String, String>` — pipes items to `fzf --header="..."`, returns selected line
- `fzf_select_multi(items: &[String], header: &str) -> Result<Vec<String>, String>` — pipes items to `fzf --multi --header="..."`, returns selected lines
- `fzf_install_hint() -> String` — returns OS-appropriate install instructions
