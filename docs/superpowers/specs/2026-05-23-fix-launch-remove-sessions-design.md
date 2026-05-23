# Fix Project Launch & Remove Session Snapshots

## Problem

QuickDev's primary feature — creating a project and launching its applications, folders, and terminals — is partially broken. Applications and terminal commands fail to launch correctly. Meanwhile, the session snapshot/restore feature adds complexity without being the core value. The app needs to be refocused on reliable project launching.

## Goals

1. Fix application launching so editors and generic apps open correctly with the right targets
2. Fix terminal launching so commands run in visible terminal windows
3. Remove all session snapshot/restore code to simplify the codebase
4. Refactor the launch pipeline to use a simple `LaunchItem` abstraction instead of the session-coupled `SessionItem`

## Non-Goals

- Redesigning the project data model (applications/folders/terminals structure stays)
- Adding new tool integrations beyond what's already supported
- Changing the project creation UI
- Modifying task management or time logging features

---

## 1. Fix Application Launching

### Root Cause

When `launch_project` in `project_service.rs` converts an `Application` to a `SessionItem`, it sets `app_id: None` and `command: None` — discarding the args field and providing no tool hint. The launch_service then:

- Falls back to fragile substring matching on the app name/path to infer a tool ID
- If it recognizes a tool (e.g., "vscode"), calls `launch_tool_app()` which opens the editor **without any target folder** — just the bare application
- If it doesn't recognize a tool, calls `launch_application(executable)` which tries to run the raw path directly — this works for `.app` bundles on macOS via `open` but fails to pass arguments

### Fix

#### Tool-aware editor launch

When the inferred tool ID is a recognized editor (`vscode`, `cursor`, `zed`), the launch should open the editor **with the project's folders as targets**. This means `launch_project` needs to pass folder paths alongside the application launch.

Implementation:
- `LaunchItem` for applications will carry an `args` field
- At the `project_service` level, when building application `LaunchItem`s, if the app's tool ID is a recognized editor, populate `args` with the project's folder paths
- `launch_service` will pass these args to the tool's CLI command (e.g., `code /path/to/folder`)

#### Generic application launch

For applications that don't match a known tool:
- Parse the stored `args` field (JSON array of strings)
- On macOS: use `open -a <app-path> --args <parsed-args>` for `.app` bundles
- On other platforms: use `Command::new(executable).args(parsed_args)`

#### Tool ID inference improvement

Move tool ID inference to `project_service.rs` when building `LaunchItem`s, and pass it explicitly via the `tool_id` field. The `launch_service` should not need to guess.

---

## 2. Fix Terminal/Command Launching

### Root Cause

`run_shell_command()` runs `sh -lc <command>` as a child process of the Tauri app. This executes headlessly — no visible terminal window appears. The process also dies when the Tauri app closes.

The Ghostty-specific code path (`run_command_in_ghostty`) is the right pattern but is only triggered when the tool ID is explicitly "ghostty", which never happens because terminal `LaunchItem`s don't carry a tool ID.

### Fix

#### Primary path: Ghostty

Since Ghostty is the user's terminal, it becomes the default for command execution:
- Check if `ghostty` CLI is available
- If available, open a new Ghostty window with `-e sh -lc "<cd path && command>"`
- This is already implemented in `run_command_in_ghostty()` — just needs to be the default path

#### Fallback path: platform default

If Ghostty is not available:
- **macOS**: Use `osascript` to tell Terminal.app to open a new window, `cd` to the working directory, and execute the command
- **Windows**: Use `start cmd /K "cd /d <path> && <command>"` or `wt -d <path> <command>` if Windows Terminal is available
- **Linux**: Try detected terminal emulators in order: `gnome-terminal`, `konsole`, `alacritty`, `xterm`

#### Execution order

Terminal commands should launch **after** applications and folders, giving editors time to start before dev servers run.

---

## 3. Remove Session Snapshot/Restore Code

### Files to Delete

**Frontend (6 files):**
- `src/pages/sessions.tsx`
- `src/pages/restore.tsx`
- `src/hooks/use-autosnapshot.ts`
- `src/hooks/use-shutdown-snapshot.ts`
- `src/types/session.ts`
- `src/types/restore-run.ts`

**Backend (7 files):**
- `src-tauri/src/services/session_service.rs`
- `src-tauri/src/services/restore_service.rs`
- `src-tauri/src/services/restore_run_service.rs`
- `src-tauri/src/commands/sessions.rs`
- `src-tauri/src/commands/restore.rs`
- `src-tauri/src/commands/restore_runs.rs`
- `src-tauri/src/commands/preferences.rs`

### Files to Modify

**Frontend:**
- `src/router.tsx` — Remove sessions, restore routes. Remove or redirect the `/tasks` route (currently points to `/sessions`; redirect to `/` instead or remove entirely)
- `src/components/sidebar.tsx` — Remove Sessions and Restore nav items
- `src/lib/tauri-api.ts` — Remove all session/restore/preference API functions and type imports
- `src/pages/dashboard.tsx` — Remove any session-related data fetching or widgets
- `src/components/layout.tsx` — Remove `useAutosnapshot` and `useShutdownSnapshot` hook usage

**Backend:**
- `src-tauri/src/lib.rs` — Remove session/restore/preference command registrations and imports
- `src-tauri/src/models.rs` — Remove `Session`, `SessionItem`, `SessionDetail`, all snapshot/restore input types, all restore result types, `ProjectPreference`, `UpdateProjectPreferenceInput`
- `src-tauri/src/commands/mod.rs` — Remove `sessions`, `restore`, `restore_runs`, `preferences` module declarations
- `src-tauri/src/services/mod.rs` — Remove `session_service`, `restore_service`, `restore_run_service` module declarations

### What Stays

- `src-tauri/src/services/launch_service.rs` — Refactored to use `LaunchItem` instead of `SessionItem`
- `src-tauri/src/adapters/tool_adapter.rs` — Still needed for tool detection and launch command lookup
- `src-tauri/src/services/integration_service.rs` — Still needed for detecting available tools
- `src-tauri/src/commands/integrations.rs` — Still needed for frontend to query available tools
- `integrations` database table — Still needed

### Database Migration

Add a new migration `YYYYMMDDHHMMSS_remove_sessions.sql`:
```sql
DROP TABLE IF EXISTS restore_runs;
DROP TABLE IF EXISTS session_items;
DROP TABLE IF EXISTS sessions;
DROP TABLE IF EXISTS project_preferences;
```

The `integrations` table is kept.

---

## 4. Launch Pipeline Refactor

### New LaunchItem Struct

Replace `SessionItem` usage in launch_service with a dedicated struct in `models.rs`:

```rust
pub struct LaunchItem {
    pub item_type: String,       // "application", "folder", "command"
    pub label: String,           // display name
    pub path: Option<String>,    // executable path, folder path, or working directory
    pub command: Option<String>, // shell command (for terminals)
    pub args: Option<String>,    // application arguments (JSON array string)
    pub tool_id: Option<String>, // explicit tool hint ("vscode", "cursor", "ghostty", etc.)
}
```

### Refactored launch_service.rs

- `restore_item(&SessionItem)` becomes `launch_item(&LaunchItem)`
- `resolve_tool_id()` simplified: just reads `item.tool_id` directly, with optional fallback to path inference
- Application handler: uses `tool_id` + `args` to decide how to launch
- Terminal handler: defaults to Ghostty, falls back to platform terminal
- All internal helpers stay (`launch_with_tool`, `launch_application`, `open_target`, `command_exists`, etc.) but operate on `LaunchItem` fields

### Refactored project_service.rs launch_project()

```
launch_project(pool, project_id):
  1. Fetch project from DB
  2. Fetch folders, applications, terminals for project
  3. Build Vec<LaunchItem>:
     - Folders: item_type="folder", path=folder.path, tool_id=None
     - Applications: item_type="application", path=app.path, args=app.args,
       tool_id=infer_tool_id(app.name, app.path)
       If tool_id is a recognized editor, override args with project folder paths
       (editors should open the project folders, not use the stored args)
     - Terminals: item_type="command", path=terminal.path, command=terminal.command,
       tool_id=detected terminal ("ghostty" if available, else None)
  4. Launch folders first, then applications, then terminals
  5. Collect failures, return result
```

### Launch Order

1. **Folders** — Open in file explorer (fast, no dependencies)
2. **Applications** — Open editors and other apps (may take a moment to start)
3. **Terminals** — Run commands (dev servers, etc.) last so editors are ready

---

## Testing Strategy

### Manual testing (primary)

- Create a project with: a folder, VS Code as an application, and a terminal command (`npm run dev`)
- Launch the project and verify:
  - Folder opens in Finder
  - VS Code opens with the project folder
  - Ghostty opens with the command running in the correct directory
- Test with Cursor and Zed if available
- Test with a generic app (e.g., a browser path) to verify non-editor launch
- Test with missing tools to verify graceful failure reporting

### Rust unit tests

- `launch_service`: test that `LaunchItem` with missing path returns appropriate error
- `project_service`: test `infer_tool_id` with various app names
- Keep existing `invalid_folder_path_returns_path_not_found` test, adapted for `LaunchItem`

### Build verification

- `npm run qa:local` passes (TypeScript check + Vite build + cargo test + cargo check)
- No session/snapshot imports remain in any kept file
- App launches without errors after session table removal (migration runs cleanly)
