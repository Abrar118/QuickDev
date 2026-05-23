# Fix Project Launch & Remove Sessions — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix broken application and terminal launching, remove all session snapshot/restore code, and refactor the launch pipeline to use a clean `LaunchItem` abstraction.

**Architecture:** The launch pipeline currently converts project items into `SessionItem` structs (a session concept) and loses critical data (args, tool hints). We replace `SessionItem` with a purpose-built `LaunchItem` struct, fix application launching to pass folder targets to editors and args to generic apps, fix terminal launching to open in visible Ghostty windows, and strip all session/snapshot/restore code from both frontend and backend.

**Tech Stack:** Tauri 2 (Rust backend), React + TypeScript (frontend), SQLite + SQLx (persistence), Vite (build)

---

## File Map

### Files to Create
- `src-tauri/migrations/20260523000000_remove_sessions.sql` — Migration to drop session-related tables

### Files to Modify (Launch Fix)
- `src-tauri/src/models.rs` — Add `LaunchItem` struct, remove all session/restore types
- `src-tauri/src/services/launch_service.rs` — Rewrite to use `LaunchItem`, fix app + terminal launching
- `src-tauri/src/services/project_service.rs` — Rewrite `launch_project()` to build `LaunchItem`s with tool IDs and args, remove `project_preferences` insert from `create_project()`

### Files to Modify (Session Removal — Backend)
- `src-tauri/src/lib.rs` — Remove session/restore/preference command registrations
- `src-tauri/src/commands/mod.rs` — Remove session/restore/preference module declarations
- `src-tauri/src/services/mod.rs` — Remove session/restore service module declarations

### Files to Modify (Session Removal — Frontend)
- `src/router.tsx` — Remove sessions/restore routes
- `src/components/sidebar.tsx` — Remove Sessions/Restore nav items
- `src/components/layout.tsx` — Remove autosnapshot/shutdown hooks
- `src/lib/tauri-api.ts` — Remove session/restore/preference APIs and imports
- `src/pages/dashboard.tsx` — Remove session-related UI (snapshot buttons, snapshot card, restore buttons)

### Files to Delete
- `src/pages/sessions.tsx`
- `src/pages/restore.tsx`
- `src/hooks/use-autosnapshot.ts`
- `src/hooks/use-shutdown-snapshot.ts`
- `src/types/session.ts`
- `src/types/restore-run.ts`
- `src/types/preferences.ts`
- `src-tauri/src/services/session_service.rs`
- `src-tauri/src/services/restore_service.rs`
- `src-tauri/src/services/restore_run_service.rs`
- `src-tauri/src/services/preferences_service.rs`
- `src-tauri/src/commands/sessions.rs`
- `src-tauri/src/commands/restore.rs`
- `src-tauri/src/commands/restore_runs.rs`
- `src-tauri/src/commands/preferences.rs`

---

## Task 1: Add LaunchItem Model and Remove Session Types

**Files:**
- Modify: `src-tauri/src/models.rs`

- [ ] **Step 1: Add `LaunchItem` struct to `models.rs`**

Add this struct after the existing `Terminal` struct (after line 48):

```rust
#[derive(Debug)]
pub struct LaunchItem {
    pub item_type: String,
    pub label: String,
    pub path: Option<String>,
    pub command: Option<String>,
    pub args: Option<String>,
    pub tool_id: Option<String>,
}
```

- [ ] **Step 2: Remove all session/restore/preference types from `models.rs`**

Delete the following structs (lines 133–310):
- `Session`
- `SessionItem`
- `SessionDetail`
- `CreateSessionSnapshotInput`
- `CaptureProjectSnapshotInput`
- `CreateSessionItemInput`
- `ProjectPreference`
- `UpdateProjectPreferenceInput`
- `RestoreRun`
- `StartRestoreRunInput`
- `CompleteRestoreRunInput`
- `PreviewRestoreInput`
- `ExecuteRestoreInput`
- `RestoreLastSessionInput`
- `RestorePreviewResult`
- `RestoreExecutionItemResult`
- `RestoreExecutionResult`
- `Integration`

Keep everything from `Users` through `DataSettings` (lines 1–131).

- [ ] **Step 3: Verify the file compiles**

Run: `cargo check --manifest-path src-tauri/Cargo.toml 2>&1 | head -30`

Expected: Errors about missing `SessionItem` in `launch_service.rs` and `project_service.rs` — this is correct, we'll fix those next.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/models.rs
git commit -m "Add LaunchItem struct, remove session/restore types from models"
```

---

## Task 2: Rewrite launch_service.rs to Use LaunchItem

**Files:**
- Modify: `src-tauri/src/services/launch_service.rs`

- [ ] **Step 1: Replace the entire contents of `launch_service.rs`**

```rust
use crate::adapters::tool_adapter::launch_command_for_tool;
use crate::models::LaunchItem;
use std::borrow::Cow;
use std::path::Path;
use std::process::Command;

pub fn launch_item(item: &LaunchItem) -> Result<(), String> {
    match item.item_type.as_str() {
        "folder" => {
            let target = item
                .path
                .as_deref()
                .ok_or_else(|| "missing_target".to_string())?;
            let normalized = normalize_path(target);
            if !Path::new(normalized.as_ref()).exists() {
                return Err("path_not_found".to_string());
            }
            open_target(normalized.as_ref())
        }
        "application" => launch_application_item(item),
        "command" => launch_terminal_item(item),
        _ => {
            let target = item
                .path
                .as_deref()
                .ok_or_else(|| "unsupported_item_type".to_string())?;
            open_target(target)
        }
    }
}

fn launch_application_item(item: &LaunchItem) -> Result<(), String> {
    let executable = item
        .path
        .as_deref()
        .ok_or_else(|| "missing_target".to_string())?;

    if let Some(tool) = item.tool_id.as_deref() {
        let platform = std::env::consts::OS;
        if let Some(cli) = launch_command_for_tool(platform, tool) {
            if command_exists(cli) {
                let resolved = resolve_command(cli).unwrap_or_else(|| cli.to_string());
                let mut cmd = Command::new(&resolved);
                if let Some(args_json) = item.args.as_deref() {
                    if let Ok(args) = serde_json::from_str::<Vec<String>>(args_json) {
                        for arg in &args {
                            cmd.arg(arg);
                        }
                    }
                }
                return cmd
                    .spawn()
                    .map(|_| ())
                    .map_err(|e| format!("tool_launch_failed: {}", e));
            }
        }
    }

    launch_application_generic(executable, item.args.as_deref())
}

fn launch_application_generic(executable: &str, args_json: Option<&str>) -> Result<(), String> {
    let parsed_args: Vec<String> = args_json
        .and_then(|j| serde_json::from_str(j).ok())
        .unwrap_or_default();

    #[cfg(target_os = "macos")]
    {
        if executable.ends_with(".app") || Path::new(executable).is_dir() {
            let mut cmd = Command::new("open");
            cmd.arg("-a").arg(executable);
            if !parsed_args.is_empty() {
                cmd.arg("--args");
                for arg in &parsed_args {
                    cmd.arg(arg);
                }
            }
            return cmd
                .spawn()
                .map(|_| ())
                .map_err(|e| format!("launch_failed: {}", e));
        }
    }

    let mut cmd = Command::new(executable);
    for arg in &parsed_args {
        cmd.arg(arg);
    }
    cmd.spawn()
        .map(|_| ())
        .map_err(|e| format!("launch_failed: {}", e))
}

fn launch_terminal_item(item: &LaunchItem) -> Result<(), String> {
    let command = item
        .command
        .as_deref()
        .ok_or_else(|| "missing_command".to_string())?;
    let cwd = item.path.as_deref();

    if try_ghostty(command, cwd).is_ok() {
        return Ok(());
    }

    run_in_platform_terminal(command, cwd)
}

fn try_ghostty(command: &str, cwd: Option<&str>) -> Result<(), String> {
    let platform = std::env::consts::OS;
    let ghostty = launch_command_for_tool(platform, "ghostty")
        .ok_or_else(|| "tool_unavailable".to_string())?;
    if !command_exists(ghostty) {
        return Err("tool_unavailable".to_string());
    }

    #[cfg(target_os = "windows")]
    {
        let _ = command;
        let _ = cwd;
        return Err("tool_unavailable".to_string());
    }

    #[cfg(not(target_os = "windows"))]
    {
        let shell_command = if let Some(path) = cwd {
            if Path::new(path).exists() {
                format!("cd '{}' && {}", path.replace('\'', "'\\''"), command)
            } else {
                command.to_string()
            }
        } else {
            command.to_string()
        };

        Command::new(resolve_command(ghostty).unwrap_or_else(|| ghostty.to_string()))
            .args(["-e", "sh", "-lc", &shell_command])
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("tool_launch_failed: {}", e))
    }
}

fn run_in_platform_terminal(command: &str, cwd: Option<&str>) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let cd_part = if let Some(path) = cwd {
            if Path::new(path).exists() {
                format!("cd '{}' && ", path.replace('\'', "'\\''"))
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        let script = format!(
            "tell application \"Terminal\"\n\
                activate\n\
                do script \"{}{}\"\n\
            end tell",
            cd_part.replace('"', "\\\""),
            command.replace('"', "\\\"")
        );
        return Command::new("osascript")
            .args(["-e", &script])
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("terminal_launch_failed: {}", e));
    }

    #[cfg(target_os = "windows")]
    {
        let full_command = if let Some(path) = cwd {
            if Path::new(path).exists() {
                format!("cd /d \"{}\" && {}", path, command)
            } else {
                command.to_string()
            }
        } else {
            command.to_string()
        };

        if command_exists("wt") {
            let wt_resolved = resolve_command("wt").unwrap_or_else(|| "wt".to_string());
            let mut cmd = Command::new(wt_resolved);
            if let Some(path) = cwd {
                if Path::new(path).exists() {
                    cmd.args(["-d", path]);
                }
            }
            cmd.args(["cmd", "/K", command]);
            return cmd
                .spawn()
                .map(|_| ())
                .map_err(|e| format!("terminal_launch_failed: {}", e));
        }

        return Command::new("cmd")
            .args(["/C", "start", "cmd", "/K", &full_command])
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("terminal_launch_failed: {}", e));
    }

    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        let candidates: &[(&str, &[&str])] = &[
            ("gnome-terminal", &["--"]),
            ("konsole", &["-e"]),
            ("alacritty", &["-e"]),
            ("xterm", &["-e"]),
        ];

        let shell_command = if let Some(path) = cwd {
            if Path::new(path).exists() {
                format!("cd '{}' && {}", path.replace('\'', "'\\''"), command)
            } else {
                command.to_string()
            }
        } else {
            command.to_string()
        };

        for (bin, prefix_args) in candidates {
            if !command_exists(bin) {
                continue;
            }
            let resolved = resolve_command(bin).unwrap_or_else(|| (*bin).to_string());
            let mut cmd = Command::new(resolved);
            for arg in *prefix_args {
                cmd.arg(arg);
            }
            cmd.args(["sh", "-lc", &shell_command]);
            if cmd.spawn().is_ok() {
                return Ok(());
            }
        }

        Err("no_terminal_found".to_string())
    }
}

fn open_target(target: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(target)
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("open_failed: {}", e))
    }

    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/C", "start", "", target])
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("open_failed: {}", e))
    }

    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        Command::new("xdg-open")
            .arg(target)
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("open_failed: {}", e))
    }
}

fn command_exists(command: &str) -> bool {
    resolve_command(command).is_some()
}

fn resolve_command(command: &str) -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        let output = Command::new("where").arg(command).output().ok()?;
        if !output.status.success() {
            return None;
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        stdout
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty())
            .map(str::to_string)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let output = Command::new("which").arg(command).output().ok()?;
        if !output.status.success() {
            return None;
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        stdout
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty())
            .map(str::to_string)
    }
}

fn normalize_path(target: &str) -> Cow<'_, str> {
    #[cfg(not(target_os = "windows"))]
    {
        if let Some(rest) = target.strip_prefix("~/") {
            if let Ok(home) = std::env::var("HOME") {
                return Cow::Owned(format!("{home}/{rest}"));
            }
        }
    }

    Cow::Borrowed(target)
}

pub fn infer_tool_id(name: &str, path: &str) -> Option<String> {
    let haystack = format!("{} {}", name.to_lowercase(), path.to_lowercase());
    if haystack.contains("cursor") {
        return Some("cursor".to_string());
    }
    if haystack.contains("code")
        || haystack.contains("vscode")
        || haystack.contains("visual studio")
    {
        return Some("vscode".to_string());
    }
    if haystack.contains("zed") {
        return Some("zed".to_string());
    }
    if haystack.contains("ghostty") || haystack.contains("ghosty") {
        return Some("ghostty".to_string());
    }
    if haystack.contains("notepad") {
        return Some("notepad".to_string());
    }
    if haystack.contains("nano") {
        return Some("nano".to_string());
    }
    None
}

fn is_editor_tool(tool_id: &str) -> bool {
    matches!(tool_id, "vscode" | "cursor" | "zed")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::LaunchItem;

    fn make_folder(path: Option<&str>) -> LaunchItem {
        LaunchItem {
            item_type: "folder".to_string(),
            label: "test".to_string(),
            path: path.map(str::to_string),
            command: None,
            args: None,
            tool_id: None,
        }
    }

    #[test]
    fn missing_folder_path_returns_error() {
        let item = make_folder(None);
        let result = launch_item(&item);
        assert_eq!(result, Err("missing_target".to_string()));
    }

    #[test]
    fn invalid_folder_path_returns_path_not_found() {
        let item = make_folder(Some("/definitely/missing/path/for/quickdev"));
        let result = launch_item(&item);
        assert_eq!(result, Err("path_not_found".to_string()));
    }

    #[test]
    fn infer_tool_id_detects_vscode() {
        assert_eq!(
            infer_tool_id("Visual Studio Code", "/Applications/Visual Studio Code.app"),
            Some("vscode".to_string())
        );
    }

    #[test]
    fn infer_tool_id_detects_cursor() {
        assert_eq!(
            infer_tool_id("Cursor", "/Applications/Cursor.app"),
            Some("cursor".to_string())
        );
    }

    #[test]
    fn infer_tool_id_detects_zed() {
        assert_eq!(
            infer_tool_id("Zed", "/usr/local/bin/zed"),
            Some("zed".to_string())
        );
    }

    #[test]
    fn infer_tool_id_returns_none_for_unknown() {
        assert_eq!(
            infer_tool_id("My Custom App", "/usr/local/bin/myapp"),
            None
        );
    }

    #[test]
    fn is_editor_tool_recognizes_editors() {
        assert!(is_editor_tool("vscode"));
        assert!(is_editor_tool("cursor"));
        assert!(is_editor_tool("zed"));
        assert!(!is_editor_tool("ghostty"));
        assert!(!is_editor_tool("notepad"));
    }

    #[test]
    fn missing_command_returns_error() {
        let item = LaunchItem {
            item_type: "command".to_string(),
            label: "test".to_string(),
            path: Some("/tmp".to_string()),
            command: None,
            args: None,
            tool_id: None,
        };
        let result = launch_item(&item);
        assert_eq!(result, Err("missing_command".to_string()));
    }
}
```

- [ ] **Step 2: Run tests to verify**

Run: `cargo test --manifest-path src-tauri/Cargo.toml 2>&1 | tail -20`

Expected: All tests pass (the `launch_item` tests exercise error paths that don't require actual tool availability).

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/services/launch_service.rs
git commit -m "Rewrite launch_service to use LaunchItem with tool-aware app and terminal launching"
```

---

## Task 3: Rewrite launch_project() in project_service.rs

**Files:**
- Modify: `src-tauri/src/services/project_service.rs`

- [ ] **Step 1: Update imports**

Replace the top of the file (lines 1–4):

```rust
use crate::models::{Application, Folder, Project, Terminal};
use crate::models::SessionItem;
use crate::services::launch_service;
use serde_json::{json, Value};
use sqlx::SqlitePool;
use std::path::Path;
```

with:

```rust
use crate::models::{Application, Folder, LaunchItem, Project, Terminal};
use crate::services::launch_service;
use serde_json::{json, Value};
use sqlx::SqlitePool;
use std::path::Path;
```

- [ ] **Step 2: Replace the `launch_project` function (lines 242–373)**

Replace the entire `launch_project` function with:

```rust
pub async fn launch_project(pool: &SqlitePool, project_id: i32) -> Result<bool, String> {
    let project = get_project(pool, project_id).await?;

    let applications = sqlx::query_as::<_, Application>(
        r#"
        SELECT id, project_id, name, path, args
        FROM applications
        WHERE project_id = $1
        ORDER BY id ASC
        "#,
    )
    .bind(project_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    let folders = sqlx::query_as::<_, Folder>(
        r#"
        SELECT id, project_id, name, path
        FROM folders
        WHERE project_id = $1
        ORDER BY id ASC
        "#,
    )
    .bind(project_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    let terminals = sqlx::query_as::<_, Terminal>(
        r#"
        SELECT id, project_id, name, path, command
        FROM terminals
        WHERE project_id = $1
        ORDER BY id ASC
        "#,
    )
    .bind(project_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    let folder_paths: Vec<String> = folders.iter().map(|f| f.path.clone()).collect();

    let folder_items: Vec<LaunchItem> = folders
        .into_iter()
        .map(|folder| LaunchItem {
            item_type: "folder".to_string(),
            label: folder.name,
            path: Some(folder.path),
            command: None,
            args: None,
            tool_id: None,
        })
        .collect();

    let app_items: Vec<LaunchItem> = applications
        .into_iter()
        .map(|app| {
            let tool_id = launch_service::infer_tool_id(&app.name, &app.path);
            let args = if let Some(ref tid) = tool_id {
                if launch_service::is_editor_tool(tid) {
                    serde_json::to_string(&folder_paths).ok()
                } else {
                    parse_stored_args(&app.args)
                }
            } else {
                parse_stored_args(&app.args)
            };
            LaunchItem {
                item_type: "application".to_string(),
                label: app.name,
                path: Some(app.path),
                command: None,
                args,
                tool_id,
            }
        })
        .collect();

    let terminal_items: Vec<LaunchItem> = terminals
        .into_iter()
        .map(|terminal| LaunchItem {
            item_type: "command".to_string(),
            label: terminal.name,
            path: Some(terminal.path),
            command: Some(terminal.command),
            args: None,
            tool_id: None,
        })
        .collect();

    let all_items_empty = folder_items.is_empty() && app_items.is_empty() && terminal_items.is_empty();
    if all_items_empty {
        return Err("No launch items configured for this project".to_string());
    }

    let mut failed = Vec::new();

    for item in &folder_items {
        if let Err(reason) = launch_service::launch_item(item) {
            failed.push(format!("{} ({})", item.label, reason));
        }
    }

    for item in &app_items {
        if let Err(reason) = launch_service::launch_item(item) {
            failed.push(format!("{} ({})", item.label, reason));
        }
    }

    for item in &terminal_items {
        if let Err(reason) = launch_service::launch_item(item) {
            failed.push(format!("{} ({})", item.label, reason));
        }
    }

    if failed.is_empty() {
        return Ok(true);
    }

    if failed.len() == 1 {
        return Err(format!("Failed to launch {}", failed[0]));
    }

    Err(format!(
        "Project '{}' launched with {} failures: {}",
        project.name,
        failed.len(),
        failed.join("; ")
    ))
}

fn parse_stored_args(args_str: &str) -> Option<String> {
    let trimmed = args_str.trim();
    if trimmed.is_empty() || trimmed == "\"\"" || trimmed == "[]" {
        return None;
    }
    if let Ok(arr) = serde_json::from_str::<Vec<String>>(trimmed) {
        if arr.is_empty() {
            return None;
        }
        return Some(serde_json::to_string(&arr).unwrap_or_default());
    }
    None
}
```

- [ ] **Step 3: Remove `project_preferences` insert from `create_project`**

In the `create_project` function, delete lines 166–176 (the `INSERT INTO project_preferences` query block):

```rust
    // DELETE THIS BLOCK:
    sqlx::query(
        r#"
        INSERT INTO project_preferences (project_id)
        VALUES ($1)
        ON CONFLICT(project_id) DO NOTHING
        "#,
    )
    .bind(project_id)
    .execute(&mut *txn)
    .await
    .map_err(|e| e.to_string())?;
```

- [ ] **Step 4: Make `is_editor_tool` public in launch_service.rs**

In `src-tauri/src/services/launch_service.rs`, change `fn is_editor_tool` to `pub fn is_editor_tool` so `project_service.rs` can call it.

- [ ] **Step 5: Verify compilation**

Run: `cargo check --manifest-path src-tauri/Cargo.toml 2>&1 | head -20`

Expected: Errors only about session/restore modules that we haven't removed yet — not about `launch_service` or `project_service`.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/services/project_service.rs src-tauri/src/services/launch_service.rs
git commit -m "Rewrite launch_project to use LaunchItem with editor folder targets and ordered execution"
```

---

## Task 4: Remove Backend Session/Restore Code

**Files:**
- Delete: `src-tauri/src/services/session_service.rs`
- Delete: `src-tauri/src/services/restore_service.rs`
- Delete: `src-tauri/src/services/restore_run_service.rs`
- Delete: `src-tauri/src/services/preferences_service.rs`
- Delete: `src-tauri/src/commands/sessions.rs`
- Delete: `src-tauri/src/commands/restore.rs`
- Delete: `src-tauri/src/commands/restore_runs.rs`
- Delete: `src-tauri/src/commands/preferences.rs`
- Modify: `src-tauri/src/services/mod.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Delete backend session/restore files**

```bash
rm src-tauri/src/services/session_service.rs
rm src-tauri/src/services/restore_service.rs
rm src-tauri/src/services/restore_run_service.rs
rm src-tauri/src/services/preferences_service.rs
rm src-tauri/src/commands/sessions.rs
rm src-tauri/src/commands/restore.rs
rm src-tauri/src/commands/restore_runs.rs
rm src-tauri/src/commands/preferences.rs
```

- [ ] **Step 2: Update `src-tauri/src/services/mod.rs`**

Replace entire file with:

```rust
pub mod integration_service;
pub mod launch_service;
pub mod project_service;
pub mod time_log_service;
```

- [ ] **Step 3: Update `src-tauri/src/commands/mod.rs`**

Replace entire file with:

```rust
pub mod integrations;
pub mod projects;
pub mod time_logs;
```

- [ ] **Step 4: Update `src-tauri/src/lib.rs`**

Replace entire file with:

```rust
mod adapters;
mod commands;
mod db;
mod models;
mod services;

use commands::{
    integrations::detect_integrations, integrations::get_available_integrations,
    integrations::list_integrations, projects::create_project, projects::delete_project,
    projects::get_project, projects::get_projects, projects::launch_project, projects::list_projects,
    projects::update_project,
    time_logs::create_time_log, time_logs::get_time_logs,
};
use db::init_database;
use sqlx::SqlitePool;
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray, Builder,
};

struct AppState {
    db_pool: Option<SqlitePool>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub async fn run() {
    let pool = init_database().await.expect("failed to init database");
    let app_state = AppState {
        db_pool: Some(pool),
    };

    Builder::default()
        .manage(app_state)
        .setup(|app| {
            let quit_item = MenuItemBuilder::with_id("quit", "Quit").build(app)?;

            let tray_menu = MenuBuilder::new(app).items(&[&quit_item]).build()?;

            tray::TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&tray_menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .build(app)?;

            Ok(())
        })
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            create_project,
            get_projects,
            list_projects,
            get_project,
            update_project,
            delete_project,
            launch_project,
            detect_integrations,
            list_integrations,
            get_available_integrations,
            get_time_logs,
            create_time_log,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 5: Verify Rust compiles and tests pass**

Run: `cargo check --manifest-path src-tauri/Cargo.toml && cargo test --manifest-path src-tauri/Cargo.toml 2>&1 | tail -20`

Expected: Compiles clean. All tests pass.

- [ ] **Step 6: Commit**

```bash
git add -A src-tauri/src/
git commit -m "Remove all session/restore/preference backend code"
```

---

## Task 5: Add Database Migration to Drop Session Tables

**Files:**
- Create: `src-tauri/migrations/20260523000000_remove_sessions.sql`

- [ ] **Step 1: Create the migration file**

```sql
DROP TABLE IF EXISTS restore_runs;
DROP TABLE IF EXISTS session_items;
DROP TABLE IF EXISTS sessions;
DROP TABLE IF EXISTS project_preferences;
```

- [ ] **Step 2: Verify the migration runs with existing DB tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml 2>&1 | tail -20`

Expected: All tests pass (the DB bootstrap test in `db.rs` runs migrations and should succeed).

- [ ] **Step 3: Commit**

```bash
git add src-tauri/migrations/20260523000000_remove_sessions.sql
git commit -m "Add migration to drop session/restore/preference tables"
```

---

## Task 6: Remove Frontend Session/Restore Code

**Files:**
- Delete: `src/pages/sessions.tsx`, `src/pages/restore.tsx`
- Delete: `src/hooks/use-autosnapshot.ts`, `src/hooks/use-shutdown-snapshot.ts`
- Delete: `src/types/session.ts`, `src/types/restore-run.ts`, `src/types/preferences.ts`
- Modify: `src/router.tsx`
- Modify: `src/components/sidebar.tsx`
- Modify: `src/components/layout.tsx`
- Modify: `src/lib/tauri-api.ts`
- Modify: `src/pages/dashboard.tsx`

- [ ] **Step 1: Delete frontend session/restore files**

```bash
rm src/pages/sessions.tsx
rm src/pages/restore.tsx
rm src/hooks/use-autosnapshot.ts
rm src/hooks/use-shutdown-snapshot.ts
rm src/types/session.ts
rm src/types/restore-run.ts
rm src/types/preferences.ts
```

- [ ] **Step 2: Update `src/router.tsx`**

Replace entire file with:

```tsx
import { createBrowserRouter } from "react-router-dom";
import { Toaster } from "./lib/toast";
import Layout from "./components/layout";
import { ThemeProvider } from "./components/theme-provider";
import Dashboard from "./pages/dashboard";
import ProjectManagement from "./pages/project-management";
import Settings from "./pages/settings";
import WorkTimer from "./pages/work-timer";
import ErrorPage from "./pages/error-page";

const router = createBrowserRouter([
  {
    path: "/",
    errorElement: (
      <ThemeProvider
        attribute="class"
        defaultTheme="system"
        enableSystem
        disableTransitionOnChange
      >
        <ErrorPage />
        <Toaster position="top-center" />
      </ThemeProvider>
    ),
    element: (
      <ThemeProvider
        attribute="class"
        defaultTheme="system"
        enableSystem
        disableTransitionOnChange
      >
        <Layout />
        <Toaster position="top-center" />
      </ThemeProvider>
    ),
    children: [
      {
        index: true,
        element: <Dashboard />,
      },
      {
        path: "projects",
        element: <ProjectManagement />,
      },
      {
        path: "settings",
        element: <Settings />,
      },
      {
        path: "timer",
        element: <WorkTimer />,
      },
    ],
  },
]);

export { router };
```

- [ ] **Step 3: Update `src/components/sidebar.tsx`**

Replace entire file with:

```tsx
import { Link, useLocation } from "react-router-dom";
import { cn } from "../lib/utils";
import {
  LayoutDashboard,
  FolderKanban,
  Settings,
  Code,
  Clock3,
} from "lucide-react";

const navItems = [
  { path: "/", label: "Overview", icon: LayoutDashboard },
  { path: "/projects", label: "Projects", icon: FolderKanban },
  { path: "/timer", label: "Work Timer", icon: Clock3 },
  { path: "/settings", label: "Settings", icon: Settings },
];

export default function Sidebar() {
  const location = useLocation();

  return (
    <div className="w-[17rem] border-r border-sidebar-border bg-sidebar h-screen flex flex-col">
      <div className="px-4 py-4 border-b border-sidebar-border flex items-center gap-3">
        <div className="h-9 w-9 rounded-lg bg-primary/20 flex items-center justify-center border border-primary/30">
          <Code className="h-5 w-5 text-primary" />
        </div>
        <div>
          <h1 className="text-lg font-semibold tracking-tight">QuickDev</h1>
          <p className="text-xs text-muted-foreground">Project launcher</p>
        </div>
      </div>

      <nav className="flex-1 px-3 py-4 overflow-auto">
        <ul className="space-y-1.5">
          {navItems.map((item) => (
            <li key={item.path}>
              <Link
                to={item.path}
                className={cn(
                  "flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm transition-colors border",
                  location.pathname === item.path
                    ? "bg-sidebar-primary text-sidebar-primary-foreground border-sidebar-primary/70 shadow-md shadow-primary/20"
                    : "text-sidebar-foreground/80 border-transparent hover:bg-sidebar-accent hover:border-sidebar-border hover:text-sidebar-foreground"
                )}
              >
                <item.icon className="h-4 w-4" />
                {item.label}
              </Link>
            </li>
          ))}
        </ul>
      </nav>

      <div className="px-4 py-3 border-t border-sidebar-border text-xs text-muted-foreground">
        Local first. No cloud dependency.
      </div>
    </div>
  );
}
```

- [ ] **Step 4: Update `src/components/layout.tsx`**

Replace entire file with:

```tsx
import { Outlet } from "react-router-dom";
import Sidebar from "./sidebar";
import { ModeToggle } from "./mode-toggle";
import { Search } from "lucide-react";
import { Input } from "./ui/input";

export default function Layout() {
  return (
    <div className="flex h-screen bg-background text-foreground">
      <Sidebar />

      <div className="flex flex-col flex-1 overflow-hidden">
        <header className="flex items-center justify-between h-16 px-5 border-b border-border/80 bg-card/70 backdrop-blur-sm">
          <div className="relative w-full max-w-md hidden md:block">
            <Search className="absolute left-3 top-2.5 h-4 w-4 text-muted-foreground" />
            <Input
              placeholder="Search projects..."
              className="pl-9 w-full bg-muted/40 border-border/70"
            />
          </div>

          <div className="flex items-center ml-auto gap-2">
            <ModeToggle />
          </div>
        </header>

        <main className="flex-1 overflow-auto p-5">
          <Outlet />
        </main>
      </div>
    </div>
  );
}
```

- [ ] **Step 5: Update `src/lib/tauri-api.ts`**

Remove session/restore/preference imports and functions. Delete lines 18–38 (the session/restore/preference type imports) and lines 203–357 (all session/restore/preference API functions: `listProjectSessions`, `createSessionSnapshot`, `captureProjectSnapshot`, `getSessionItems`, `getSessionDetail`, `deleteSessionSnapshot`, `getProjectPreferences`, `updateProjectPreferences`, `startRestoreRun`, `completeRestoreRun`, `listRestoreRuns`, `previewRestore`, `executeRestore`, `restoreLastSession`).

Keep the Integration import on line 39 and the integration API functions (`detectIntegrations`, `listIntegrations`, `getAvailableIntegrations`) on lines 359–384.

After cleanup, the imports should be:

```typescript
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-shell";
import {
  ask,
  message,
  save,
  open as DialogOpen,
} from "@tauri-apps/plugin-dialog";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";
import type { Project } from "../types/project";
import type { Task } from "../types/task";
import type { TimeLog } from "../types/time-log";
import type { Integration } from "../types/integration";
```

- [ ] **Step 6: Update `src/pages/dashboard.tsx`**

Replace entire file with:

```tsx
import { useEffect, useMemo, useState } from "react";
import { useNavigate } from "react-router-dom";
import { Button } from "../components/ui/button";
import { Badge } from "../components/ui/badge";
import { getProjects, launchProject } from "../lib/tauri-api";
import type { Project } from "../types/project";
import { toast } from "@/lib/toast";
import { Plus, Rocket } from "lucide-react";

export default function Dashboard() {
  const navigate = useNavigate();
  const [projects, setProjects] = useState<Project[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [launchingId, setLaunchingId] = useState<number | null>(null);

  useEffect(() => {
    const loadProjects = async () => {
      try {
        const loadedProjects = await getProjects();
        setProjects(loadedProjects);
      } catch (error) {
        toast.error("Failed to load overview", {
          description:
            error instanceof Error ? error.message : "Unknown error occurred",
        });
      } finally {
        setIsLoading(false);
      }
    };

    void loadProjects();
  }, []);

  const sortedProjects = useMemo(
    () =>
      [...projects].sort(
        (a, b) =>
          new Date(b.last_opened).getTime() - new Date(a.last_opened).getTime()
      ),
    [projects]
  );

  const handleLaunch = async (projectId: number) => {
    setLaunchingId(projectId);
    try {
      await launchProject(projectId);
      toast.success("Project launched successfully");
    } catch (error) {
      toast.error("Launch failed", {
        description:
          error instanceof Error ? error.message : "Unknown error occurred",
      });
    } finally {
      setLaunchingId(null);
    }
  };

  return (
    <div className="space-y-5">
      <div className="flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
        <div>
          <h1 className="text-3xl font-semibold tracking-tight">Overview</h1>
          <p className="text-sm text-muted-foreground">
            Your projects on this device.
          </p>
        </div>
        <div className="flex items-center gap-2">
          <Button
            className="gap-2"
            onClick={() => navigate("/projects")}
          >
            <Plus className="h-4 w-4" />
            New Project
          </Button>
        </div>
      </div>

      <div className="grid gap-3 md:grid-cols-2">
        <div className="rounded-lg border border-border/70 bg-card/70 p-4">
          <p className="text-xs text-muted-foreground">Total Projects</p>
          <p className="mt-2 text-3xl font-semibold">{projects.length}</p>
        </div>
        <div className="rounded-lg border border-border/70 bg-card/70 p-4">
          <p className="text-xs text-muted-foreground">Last Opened</p>
          <p className="mt-2 text-sm font-medium">
            {sortedProjects[0]?.last_opened
              ? new Date(sortedProjects[0].last_opened).toLocaleString()
              : "No projects yet"}
          </p>
        </div>
      </div>

      <div className="rounded-lg border border-border/70 bg-card/70">
        <div className="border-b border-border/60 px-4 py-3">
          <h2 className="text-lg font-medium">Recent Projects</h2>
        </div>
        <div className="px-4 py-2">
          {isLoading ? (
            <p className="py-6 text-sm text-muted-foreground">Loading projects...</p>
          ) : sortedProjects.length === 0 ? (
            <p className="py-6 text-sm text-muted-foreground">
              No projects yet. Create one to get started.
            </p>
          ) : (
            <div className="divide-y divide-border/50">
              {sortedProjects.slice(0, 12).map((project) => (
                <div
                  key={project.id}
                  className="flex flex-col gap-3 py-3 sm:flex-row sm:items-center sm:justify-between"
                >
                  <div className="space-y-1">
                    <p className="font-medium">{project.name}</p>
                    <p className="text-xs text-muted-foreground">
                      Last opened {new Date(project.last_opened).toLocaleString()}
                    </p>
                  </div>
                  <div className="flex items-center gap-2">
                    <Badge variant={project.is_active ? "default" : "secondary"}>
                      {project.is_active ? "Active" : "Idle"}
                    </Badge>
                    <Button
                      size="sm"
                      className="gap-1.5"
                      disabled={launchingId === project.id}
                      onClick={() => handleLaunch(project.id)}
                    >
                      <Rocket className="h-3.5 w-3.5" />
                      {launchingId === project.id ? "Launching..." : "Launch"}
                    </Button>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
```

- [ ] **Step 7: Verify frontend builds**

Run: `npm run build 2>&1 | tail -20`

Expected: TypeScript check passes and Vite build succeeds with no session-related import errors.

- [ ] **Step 8: Commit**

```bash
git add -A src/
git commit -m "Remove all session/restore/preference frontend code, update dashboard with launch button"
```

---

## Task 7: Full Build Verification

**Files:** None (verification only)

- [ ] **Step 1: Run the full QA check**

Run: `npm run qa:local 2>&1 | tail -30`

Expected: TypeScript check passes, Vite build succeeds, `cargo test` passes, `cargo check` passes.

- [ ] **Step 2: Verify no stale session references**

Run: `rg -l "session|snapshot|restore_run|SessionItem|RestoreRun|CaptureProject|autosnapshot|shutdownSnapshot" src/ src-tauri/src/ --type-add 'code:*.{rs,ts,tsx}' --type code 2>/dev/null | rg -v "node_modules"`

Expected: No results (zero files contain session-related references). Integration-related files may mention "restore_mode" in the tool adapter — that's fine, it's part of the adapter capability description.

- [ ] **Step 3: Launch the app for manual testing**

Run: `npm run tauri dev`

Verify:
1. App opens without errors
2. Sidebar shows: Overview, Projects, Work Timer, Settings (no Sessions or Restore)
3. Dashboard shows projects with Launch buttons (no Snapshot/Restore buttons)
4. Create a test project with a folder, an application (e.g., VS Code or Cursor), and a terminal command
5. Click Launch — verify all items open correctly:
   - Folder opens in Finder
   - Editor opens with the project folder loaded
   - Terminal command runs in a visible Ghostty window

- [ ] **Step 4: Final commit if any fixes were needed**

```bash
git add -A
git commit -m "Fix issues found during manual verification"
```
