# QuickDev CLI Refinements — Design Spec

**Date:** 2026-05-25
**Status:** Approved

## Overview

Six refinements to the existing QuickDev CLI: selective launch with fzf, richer list output, init templates, global default emulator, args prompt in interactive add, and project deregister.

## 1. Selective Launch with fzf

### Current behavior

`quickdev launch` (no args, in project dir) launches all terminals and apps.

### New behavior

- **`quickdev launch`** (no args, in project dir) — opens fzf multi-select showing all terminals and apps. User picks which to launch. Ctrl+C cancels.
- **`quickdev launch --all`** — launches everything without the picker (current behavior).
- **`quickdev launch <name>`** — looks up project by name in global index, launches all items.

fzf display format (same as remove picker):
```
[terminal] dev — . (quickdev --help)
[terminal] Global config — /Users/.../quickdev (code .)
[app] Codex — /Applications/Codex.app
```

After selection, only the selected items are launched. The launch summary shows results for selected items only. If there's only 1 item configured, launch it directly without fzf.

### Implementation

Add `--all` flag to the `Launch` command variant. In `cmd_launch`:
- If `--all` is set OR a project name is given: launch everything (current behavior)
- If neither: load config, build display list, run fzf multi-select, filter config to selected items, then launch

Reuse the same display-building logic from `cmd_remove_interactive` — extract into a shared helper function.

## 2. Richer List Output

### Current behavior

Table format with columns: Name, Path, Terminals count, Apps count.

### New behavior

Vertical layout:
```
Projects:
  QuickDev    /Users/orion-abrar/Code/Desktop-Application/QuickDev
    Terminals: dev, Global config
    Apps: Codex

  my-api      /Users/orion-abrar/Code/my-api
    Terminals: server, worker, logs
    Apps: Cursor

  old-project  /Users/orion-abrar/Code/old  (missing)
```

- Show terminal and app names inline (comma-separated)
- Mark projects whose path doesn't exist with `(missing)`
- Plain text, no color — safe for piping

### Implementation

Replace `cmd_list` formatting. Load each project's `.quickdev.toml` to get terminal/app names (already done for counts).

## 3. Init Templates

### `quickdev init --from <project-name>`

Clone another project's terminal and app entries into a new project.

**Flow:**
1. Look up source project by name in global index
2. Read its `.quickdev.toml`
3. Create `.quickdev.toml` in current directory with same terminals and apps, new project name (from current dir name)
4. Register in global index

**Path handling:** Relative paths (`.`, `./src`) are copied as-is — they resolve to the new project root at launch time. Absolute paths are copied verbatim.

### `quickdev init` smart re-register

If `.quickdev.toml` exists but the project is NOT in the global index, re-register it instead of erroring. Only error if `.quickdev.toml` exists AND the project is already in the global index.

### Implementation

Add `--from` option to the `Init` command variant. Modify `cmd_init` to handle three cases:
1. No `.quickdev.toml`, no `--from`: create empty config (current behavior)
2. No `.quickdev.toml`, `--from <name>`: clone from source project
3. `.quickdev.toml` exists, not in global index: re-register
4. `.quickdev.toml` exists, already in global index: error (current behavior)

## 4. Global Default Emulator

### Config change

Add optional `emulator` field to `~/Documents/quickdev/config.toml`:

```toml
emulator = "ghostty"

[[projects]]
name = "QuickDev"
path = "/Users/orion-abrar/Code/Desktop-Application/QuickDev"
```

### Priority order

1. Per-terminal `emulator` field in `.quickdev.toml` (highest)
2. Global `emulator` in `~/Documents/quickdev/config.toml`
3. Auto-detect: try Ghostty, then platform fallback (lowest)

### Editing

`quickdev edit --global` opens `~/Documents/quickdev/config.toml` in `$EDITOR`.

### TOML comment header

Add a comment header to the global config when first created:

```toml
# QuickDev global configuration
#
# emulator = (optional) Default terminal emulator: "ghostty", "terminal"
#
# Projects are auto-managed by quickdev init / deregister
```

### Implementation

- Add `emulator: Option<String>` to `GlobalConfig` model
- Add `--global` flag to `Edit` command
- Modify `launch_terminal` to accept global emulator as fallback
- Modify `launch_project` to read global config and pass emulator down
- Add comment header to `save_global_config`

## 5. Add Args Prompt for Apps

### Current behavior

Interactive `quickdev add` → Application skips the `args` field. Apps are added with `args: None`.

### New behavior

After selecting an app, prompt:
```
Arguments (e.g., "." to open project root, Enter to skip): . --new-window
```

- Input is split by spaces into `Vec<String>`
- Empty input = `None` (no args)
- The resulting TOML entry includes `args = ["."]` or whatever was typed

### Implementation

Add a `prompt()` call in `cmd_add_interactive` after `pick_application()`, before pushing the `AppEntry`. Parse the input by splitting on whitespace.

## 6. Deregister Command

### `quickdev deregister`

Remove the current project from the global index without deleting the `.quickdev.toml` file.

**Flow:**
1. Find `.quickdev.toml` in cwd/parents (or fzf picker if not found)
2. Load global index, find matching project by path
3. Remove from global index, save
4. Print: `Deregistered project 'QuickDev' from global index`

### `quickdev deregister --delete`

Same as above, but also deletes the `.quickdev.toml` file.

### Implementation

Add `Deregister` variant to `Commands` enum with `--delete` flag. Add `cmd_deregister` function. Uses `resolve_project_config` for project discovery (gets fzf fallback for free).

## Files Changed

| File | Changes |
|------|---------|
| `src/main.rs` | Add `--all` to Launch, `--from` to Init, `--global` to Edit, `Deregister` command, selective launch logic, richer list, args prompt, shared display builder |
| `src/models.rs` | Add `emulator: Option<String>` to `GlobalConfig` |
| `src/config.rs` | Add comment header to `save_global_config`, modify `global_config_path` handling |
| `src/launch.rs` | Accept global emulator fallback in `launch_terminal` and `launch_project` |
