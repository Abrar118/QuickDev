# QuickDev Implementation Checklist

## Goal

Deliver a local-first desktop app that can save and restore developer project sessions across macOS, Windows, and Ubuntu/Debian, with an initial focus on reliable support for a defined set of editors and tools.

## Phase 1: Product Framing and UX Baseline

- Reframe the app UI and copy around session recovery, not generic dashboard analytics.
- Align naming across the app: `project`, `session`, `snapshot`, `restore`, `integration`.
- Update navigation to prioritize Projects, Sessions, Restore, and Settings.
- Reduce global border radius and remove decorative dashboard treatments from the main layout.
- Define a consistent dark palette with restrained accent use.

## Phase 2: Persistence Foundation

- Finalize SQLite schema for:
  - `projects`
  - `sessions`
  - `session_items`
  - `integrations`
  - `project_preferences`
  - `restore_runs`
- Add SQLx migrations for the initial schema.
- Implement database bootstrap in `src-tauri/src/db.rs`.
- Add repository/service functions for creating and reading projects.
- Add repository/service functions for creating and reading session snapshots.
- Store timestamps in UTC.
- Add indexes for project lookup and session history retrieval.

## Phase 3: Native Domain Services

- Create `project_service` for CRUD and path validation.
- Create `session_service` for manual, autosave, and shutdown snapshots.
- Create `integration_service` for detection and capability lookup.
- Create `restore_service` for preview and execution flows.
- Add shared Rust types for projects, sessions, restore plans, and restore results.
- Standardize error types so the frontend receives structured failures.

## Phase 4: Adapter System

- Create a shared adapter trait/interface for supported tools.
- Implement adapter registry and detection flow.
- Define adapter capabilities by:
  - platform
  - item types supported
  - restore modes supported
  - CLI/path detection result
- Add adapters for v1:
  - VS Code
  - Cursor
  - Zed
  - Notepad
  - Nano
  - Terminal
  - Browser
  - File explorer / Finder

## Phase 5: Cross-Platform Launching

- Implement path normalization utilities for macOS, Windows, and Linux.
- Implement executable detection helpers.
- Implement OS-aware process launch helpers.
- Add terminal launch strategies per platform.
- Add fallback logic when a preferred editor is unavailable.
- Return per-item restore status as:
  - restored
  - skipped
  - failed
  - unavailable

## Phase 6: Tauri Command Surface

- Add project commands:
  - `create_project`
  - `list_projects`
  - `get_project`
  - `update_project`
- Add session commands:
  - `create_session_snapshot`
  - `list_project_sessions`
  - `get_session_detail`
- Add restore commands:
  - `preview_restore`
  - `execute_restore`
- Add integration commands:
  - `detect_integrations`
  - `get_available_integrations`
- Add settings commands:
  - `get_project_preferences`
  - `update_project_preferences`

## Phase 7: Frontend Information Architecture

- Replace KPI-first dashboard content with project/session-focused screens.
- Build a compact app shell with:
  - fixed sidebar
  - compact top bar
  - project-centered main content
- Create pages for:
  - Dashboard
  - Project Detail
  - Restore Review
  - Settings
- Use the reference design language only as inspiration for layout density and dark tone.
- Keep corners tighter and surfaces flatter than the reference image.

## Phase 8: Frontend Components

- Build project list and project card/table views.
- Build session timeline/history list.
- Build restore preview panel.
- Build tracked items table for files, commands, links, and notes.
- Build integration status list.
- Build preferences forms for per-project restore settings.
- Build result summary UI for restore runs.

## Phase 9: Capture and Restore Workflows

- Implement `Save Session` flow from the project detail screen.
- Implement autosnapshot scheduling.
- Implement shutdown-safe snapshot creation if the app is open when exiting.
- Implement `Restore Last Session`.
- Implement restore-from-history.
- Add selective restore controls for:
  - files only
  - files and links
  - full restore
- Add restore confirmation for commands and multi-app restore actions.

## Phase 10: V1 Integration Behavior

- VS Code:
  - open project folder
  - open tracked files
- Cursor:
  - open project folder and tracked files when CLI is available
- Zed:
  - open project folder and tracked files when CLI is available
- Notepad:
  - open text files on Windows as lightweight fallback
- Nano:
  - open files through a terminal adapter where `nano` is available
- Browser:
  - open stored URLs
- File explorer:
  - open the project root
- Terminal:
  - open in project root
  - optionally run explicit startup commands after confirmation

## Phase 11: Dependency and Tooling Cleanup

Status: completed on March 9, 2026.

- Resolved the `react-day-picker` and `date-fns` version conflict (`date-fns@3.6.0`).
- Refreshed `package-lock.json` after dependency changes.
- Confirmed current Tauri plugin versions are compatible with Tauri 2 and compile in the current backend build.
- Removed unused npm packages that were not referenced in source/config.
- Rust dependencies are pinned to concrete versions in `src-tauri/Cargo.toml`.

## Phase 12: Validation and QA

Status: local automation complete on March 10, 2026.  
Cross-OS execution checklist prepared in `docs/phase12-qa-matrix.md`.

- Test restore flows on macOS.
- Test restore flows on Windows.
- Test restore flows on Ubuntu/Debian.
- Validate missing-editor fallback behavior.
- Validate invalid-path handling.
- Validate partial restore reporting.
- Validate migration bootstrapping on a fresh install.
- Validate behavior when no integrations are installed.

## V1 Exit Criteria

- User can create a project from a local folder.
- User can save a session snapshot for that project.
- User can preview a restore plan.
- User can restore a saved session with clear result reporting.
- Supported integrations work on their intended platforms.
- The UI is focused on session recovery and does not read like a generic admin dashboard.

## Immediate Next Build Order

1. finalize schema and migrations
2. implement project and session services
3. build adapter registry and detection
4. expose Tauri commands
5. redesign the shell and project detail screen
6. implement restore preview and execution
7. harden macOS, Windows, and Ubuntu/Debian behavior
