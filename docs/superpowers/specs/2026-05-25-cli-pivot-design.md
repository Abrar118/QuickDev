# QuickDev CLI — Design Spec

**Date:** 2026-05-25
**Status:** Approved
**Supersedes:** All prior desktop app specs. QuickDev is now a CLI tool.

## Overview

QuickDev is a cross-platform Rust CLI that manages per-project terminal and application configurations. You define terminals (working directory + optional startup command) and applications per project, then `quickdev launch` opens them all.

## Config Format

### Global Index — `~/Documents/quickdev/config.toml`

Auto-managed by the CLI. Lists all known projects so `quickdev list` and `quickdev launch <name>` work from anywhere.

```toml
[[projects]]
name = "QuickDev"
path = "/Users/orion-abrar/Code/Desktop-Application/QuickDev"

[[projects]]
name = "my-api"
path = "/Users/orion-abrar/Code/my-api"
```

Each entry has:
- `name` — project display name (derived from directory name on `init`, editable)
- `path` — absolute path to the project root (where `.quickdev.toml` lives)

### Per-Project — `.quickdev.toml` in project root

```toml
[project]
name = "QuickDev"

[[terminals]]
name = "server"
path = "."
command = "npm run dev"

[[terminals]]
name = "logs"
path = "./logs"

[[applications]]
name = "Cursor"
path = "/Applications/Cursor.app"
args = ["."]
```

#### `[project]` table

| Field | Type   | Required | Description            |
|-------|--------|----------|------------------------|
| name  | string | yes      | Project display name   |

#### `[[terminals]]` entries

| Field   | Type   | Required | Description                                    |
|---------|--------|----------|------------------------------------------------|
| name    | string | yes      | Label for this terminal tab                    |
| path    | string | yes      | Working directory, relative to project root     |
| command | string | no       | Startup command. If omitted, opens a shell.     |

#### `[[applications]]` entries

| Field | Type         | Required | Description                              |
|-------|--------------|----------|------------------------------------------|
| name  | string       | yes      | Application display name                 |
| path  | string       | yes      | Executable or .app bundle path           |
| args  | string array | no       | Arguments passed to the application      |

Path resolution rules:
- Terminal `path` is relative to the project root, resolved to absolute at launch time.
- Application `path` is absolute. `~` expansion is supported.
- Application `args` containing `"."` resolve to the project root at launch time.

## CLI Commands

### `quickdev init`

Creates `.quickdev.toml` in the current directory and registers the project in the global index.

- Derives project name from the current directory name.
- If `.quickdev.toml` already exists, print a warning and exit (no overwrite).
- Creates `~/Documents/quickdev/config.toml` and its parent directories if they don't exist.
- Adds the project entry to the global index.
- If a project with the same name already exists in the global index, append a numeric suffix (e.g., `my-api-2`).

### `quickdev launch [project]`

Opens all configured terminals and applications for a project.

- **No argument:** Look for `.quickdev.toml` in the current directory, then walk up parent directories until one is found. Error if none found.
- **With name argument:** Look up the project name in the global index, resolve its path, read its `.quickdev.toml`.
- **Execution order:** Terminals first, then applications.
- **Failure handling:** Each item launches independently. A failure in one does not abort the rest. Print a summary at the end.
- **Exit code:** 0 if at least one item launched successfully. 1 if nothing launched.

Output format:
```
Launched 3/4 items:
  [ok] server (terminal)
  [ok] logs (terminal)
  [ok] Cursor (app)
  [FAIL] Docker Desktop — launch_failed: No such file
```

### `quickdev list`

Show all indexed projects in a table.

```
Name        Path                                          Terminals  Apps
QuickDev    /Users/orion-abrar/Code/Desktop-App/QuickDev  2          1
my-api      /Users/orion-abrar/Code/my-api                3          2
```

Projects whose path no longer exists are marked with a warning indicator.

### `quickdev add terminal <name> <path> [--command "..."]`

Add a terminal entry to the local `.quickdev.toml`. Must be run from a directory containing `.quickdev.toml` (or a parent).

### `quickdev add app <name> <path> [--args "arg1" "arg2"]`

Add an application entry to the local `.quickdev.toml`.

### `quickdev remove terminal <name>`

Remove a terminal entry by name from the local `.quickdev.toml`.

### `quickdev remove app <name>`

Remove an application entry by name from the local `.quickdev.toml`.

### `quickdev edit`

Open `.quickdev.toml` in `$EDITOR`. Falls back to `vi` if `$EDITOR` is unset.

## Launch Logic

### Terminal Launching

**macOS/Linux — Ghostty (primary):**
```
ghostty -e sh -lc "cd '<resolved_path>' && <command>"
```
If no command is configured:
```
ghostty -e sh -lc "cd '<resolved_path>' && exec $SHELL"
```

**macOS fallback — Terminal.app:**
AppleScript via `osascript`:
```
tell application "Terminal"
    activate
    do script "cd '<path>' && <command>"
end tell
```

**Linux fallback chain:** `gnome-terminal`, `konsole`, `alacritty`, `xterm` (in order, first available).

**Windows — PowerShell 7 (primary):**
```
pwsh -NoExit -Command "Set-Location '<resolved_path>'; <command>"
```

**Windows fallback:** Windows Terminal (`wt`) then `cmd`.

### Application Launching

- Tool detection at launch time via name/path heuristics (detects Cursor, VS Code, Zed, Ghostty).
- Editors automatically receive project folder paths as arguments.
- macOS `.app` bundles launch via `open -a <path> --args <args>`.
- Other platforms run the executable directly.

### Path Resolution

1. Terminal `path` values are joined with the project root to produce absolute paths.
2. Application `args` containing `"."` are replaced with the project root path.
3. `~` in application paths is expanded to the user's home directory.
4. Paths that don't exist at launch time produce a warning (skip, don't abort).

### Tool Detection

Inferred from application name and path at launch time:

| Pattern in name/path       | Tool ID  | Behavior                    |
|---------------------------|----------|-----------------------------|
| "cursor"                  | cursor   | Editor — pass folder args   |
| "code", "vscode"          | vscode   | Editor — pass folder args   |
| "zed"                     | zed      | Editor — pass folder args   |
| "ghostty"                 | ghostty  | Terminal emulator           |
| (no match)                | —        | Generic application launch  |

## Project Structure

```
Cargo.toml
src/
  main.rs           # clap entry point, subcommand dispatch
  config.rs         # Global index + per-project TOML read/write
  launch.rs         # Terminal & app launching
  adapters.rs       # Tool detection, CLI resolution, command_exists
  models.rs         # Project, TerminalEntry, AppEntry structs
```

### Dependencies

| Crate   | Purpose                              |
|---------|--------------------------------------|
| clap    | CLI parsing (derive feature)         |
| serde   | Serialization/deserialization        |
| toml    | TOML config read/write               |
| dirs    | Cross-platform home directory        |

No async runtime. No SQLite. No Tauri. Pure synchronous Rust using `std::process::Command`.

### Code Carried Over

From the existing codebase:

| Source file            | Destination    | What's reused                                                     |
|------------------------|----------------|-------------------------------------------------------------------|
| `launch_service.rs`   | `launch.rs`    | `try_ghostty`, `run_in_platform_terminal`, `launch_application_generic`, `open_target`, `command_exists`, `resolve_command`, `normalize_path` |
| `tool_adapter.rs`     | `adapters.rs`  | Tool-to-CLI-command mappings per platform                         |
| `launch_service.rs`   | `adapters.rs`  | `infer_tool_id`, `is_editor_tool`                                 |

### Code Dropped

Everything else: SQLite/SQLx, Tauri IPC, all React/TypeScript frontend, node_modules, package.json, vite config, all models for Tasks, TimeLogs, Settings, Integrations, ChecklistItems, Users, Sessions, Preferences.

## Repo Transition

1. Delete: `src/`, `src-tauri/` (except cherry-picked Rust), `node_modules/`, `package.json`, `package-lock.json`, `vite.config.ts`, `tsconfig*.json`, `index.html`, `components.json`, `styles/`, `public/`, `dist/`, `.vscode/`, `.codex/`.
2. Keep: `.git/`, `docs/`, `.remember/`, `CLAUDE.md` (updated), `AGENTS.md`.
3. Create: `Cargo.toml`, `src/` with the 5 files listed above.
4. Update `CLAUDE.md` to reflect the new CLI project.
5. Update `.gitignore` for Rust (`/target/`, etc.).
