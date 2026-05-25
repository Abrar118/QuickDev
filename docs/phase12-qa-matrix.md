# Phase 12 QA Matrix

## Scope

This checklist validates restore reliability, fallback behavior, and migration health for QuickDev on:

- macOS
- Windows
- Ubuntu/Debian

Run these steps in order on each platform.

## 0. Environment Prep

1. Install dependencies: `npm install`
2. Run local automated gate:
   - `npm run qa:local`
3. Start app:
   - `npm run tauri dev`

Expected:
- build/test/check pass before manual validation
- app launches without crash

## 1. Fresh Install Migration Bootstrap

1. Close app.
2. Remove app data DB directory for platform:
   - macOS: `~/Library/Application Support/QuickDev`
   - Windows: `%LOCALAPPDATA%\\QuickDev`
   - Linux: `~/.local/share/QuickDev`
3. Start app again with `npm run tauri dev`.

Expected:
- app recreates `quickdev.db` automatically
- no migration errors at startup
- projects/sessions views load

## 2. No Integrations Installed Behavior

1. Open `Settings -> Integrations`.
2. Click `Detect Again`.
3. Ensure at least one tool in the table shows `Unavailable` (or all if nothing installed).

Expected:
- no crashes
- unavailable integrations remain visible with status
- restore UI still works and can preview snapshots

## 3. Invalid Path Handling

1. Create a project with one folder path that does not exist.
2. Save snapshot from Sessions.
3. Run restore for that snapshot.

Expected:
- restore run completes
- invalid items report `failed` with path-related reason
- app remains stable

## 4. Missing-Editor Fallback

1. Create project with one folder and one file-based item that can open in default file opener.
2. Assign an editor integration that is not available on this machine.
3. Save snapshot.
4. Run restore.

Expected:
- restore does not abort globally
- item result is either:
  - `restored` via fallback opener, or
  - `unavailable` when no fallback path is possible
- run result becomes `partial` instead of hard fail for mixed outcomes

## 5. Partial Restore Reporting

1. Create a session containing mixed item types:
   - folder/file
   - command
   - URL
2. In restore screen, disable one scope (for example commands).
3. Execute restore.

Expected:
- summary counts match scope selection:
  - skipped out-of-scope increments
  - restored increments only for enabled scopes
- final result is `partial` when any skipped/failed/unavailable exists

## 6. Restore History Integrity

1. Execute restore for same session multiple times.
2. Inspect restore history table.

Expected:
- each run is appended (no overwrite)
- status and timestamps are retained per run
- loading previous run context updates preview/scope correctly

## 7. Project/Snapshot Delete Integrity

1. Delete a snapshot from Sessions.
2. Verify it disappears from session list and restore selection.
3. Delete a project from Projects.

Expected:
- project, sessions, session_items, preferences, and restore_runs are cascade-deleted
- no orphan rows shown in UI

## 8. Work Timer Persistence

1. Start timer, run for at least 1 minute, reset/save.
2. Verify record appears in history and calendar.
3. Restart app.

Expected:
- time log persists after restart
- daily/weekly totals reflect saved entry

## 9. Pass/Fail Template

For each OS, record:

- QA date:
- Version/commit:
- Pass count:
- Fail count:
- Blocking issues:

