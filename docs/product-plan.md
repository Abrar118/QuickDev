# QuickDev Product Plan

## Product Summary

QuickDev should be a personal desktop app that helps a developer resume a project exactly, or as closely as possible, to where they left off before a shutdown, restart, or context switch.

The product is strongest when positioned as a cross-platform workspace session manager for local development projects, not as a general project-management dashboard. Its core value is preserving and restoring project context: open files, project folders, notes, commands, links, and supported editor sessions.

The nearest mainstream comparison is Microsoft PowerToys Workspaces, but QuickDev should be narrower and more developer-specific. Instead of mainly restoring app windows, QuickDev should center on project-aware recovery: files, editor targets, terminals, commands, links, and session history tied to a development project.

## 1. MVP Scope

### Primary Problem

The app solves a specific pain point:

- too many files and tools are open for one project
- the laptop shuts down or the user leaves the task
- reopening the full working context is manual and error-prone

### Target User

- primary user: solo developer
- secondary future audience: developers who work across multiple projects and tools locally

### Core Product Promise

QuickDev should let the user save and restore a project session with minimal friction.

### MVP Goals

- Create and manage local projects based on filesystem folders
- Save project sessions manually
- Autosave recent session snapshots
- Restore a saved session on demand
- Support a small set of reliable integrations first
- Work offline on macOS, Windows, and Linux

### MVP User Flows

#### Flow A: Create Project

1. User adds a local project folder
2. User selects preferred tools for that project
3. QuickDev stores basic metadata and restore preferences

#### Flow B: Save Session

1. User clicks `Save Session`
2. QuickDev stores:
   - selected/open file paths
   - important links
   - commands to relaunch
   - notes/checklist
   - last active timestamp
3. Session becomes available for future restore

#### Flow C: Restore Session

1. User opens a project
2. User clicks `Restore Last Session` or chooses a snapshot
3. QuickDev reopens supported tools and files
4. App shows which items were fully restored, partially restored, or skipped

### MVP Features

- Project list with local folder paths
- Session snapshot history per project
- Manual `Save Session`
- `Restore Last Session`
- Reopen files in supported editors
- Launch project folder in file explorer
- Launch terminal in project directory
- Store and reopen project links
- Per-project notes
- Per-project startup commands
- Restore result summary

### MVP Supported Integrations

Start narrow and stable:

- VS Code
- Cursor if CLI is available
- Zed if CLI is available
- system text editors for lightweight restore flows:
  - Notepad on Windows
  - Nano on macOS/Linux where available through terminal launch
- System terminal
- Default browser
- File explorer / Finder

### Explicitly Out of Scope for MVP

- Team collaboration
- Cloud sync
- Mobile app
- Automatic universal capture from every app on the machine
- Full project management suite
- Heavy analytics dashboard
- AI features without a clear restore-related use case

## 2. Database Schema

SQLite is a good fit for v1 because the app is local-first, relational, and needs durable snapshots.

### Recommended Tables

#### `projects`

Stores project-level metadata.

Suggested fields:

- `id`
- `name`
- `root_path`
- `description`
- `preferred_editor`
- `preferred_terminal`
- `color_tag`
- `is_archived`
- `created_at`
- `updated_at`
- `last_opened_at`

#### `sessions`

Stores a saved session or snapshot for a project.

Suggested fields:

- `id`
- `project_id`
- `name`
- `source`
- `status`
- `captured_at`
- `created_at`
- `notes`

Notes:

- `source` can be `manual`, `autosave`, or `shutdown`
- `status` can be `draft`, `ready`, or `restored`

#### `session_items`

Stores the individual items inside a session.

Suggested fields:

- `id`
- `session_id`
- `item_type`
- `label`
- `path_or_url`
- `command`
- `app_id`
- `position_index`
- `restore_mode`
- `metadata_json`
- `created_at`

Examples of `item_type`:

- `file`
- `folder`
- `url`
- `command`
- `terminal`
- `note`

Examples of `restore_mode`:

- `automatic`
- `manual`
- `tracked_only`

#### `integrations`

Stores installed or configured tool integrations.

Suggested fields:

- `id`
- `tool_id`
- `display_name`
- `platform`
- `detection_method`
- `launch_command`
- `is_available`
- `last_checked_at`
- `metadata_json`

#### `project_preferences`

Stores project-specific restore settings.

Suggested fields:

- `id`
- `project_id`
- `auto_snapshot_enabled`
- `snapshot_interval_minutes`
- `restore_files_on_launch`
- `restore_commands_on_launch`
- `restore_links_on_launch`
- `confirm_before_restore`
- `created_at`
- `updated_at`

#### `restore_runs`

Stores audit information for each restore attempt.

Suggested fields:

- `id`
- `project_id`
- `session_id`
- `started_at`
- `completed_at`
- `result`
- `summary_json`

### Schema Notes

- Use normalized tables for core entities
- Use `metadata_json` only for integration-specific data that may vary by tool
- Store timestamps in UTC
- Treat filesystem path casing carefully across operating systems
- Add indexes for `project_id`, `captured_at`, and `last_opened_at`

## 3. Cross-Platform Integration Strategy

Cross-platform support is viable, but only if integrations are treated as adapters with different capabilities per operating system and per external tool.

### Guiding Rule

QuickDev should promise reliable restore for supported tools, not universal restore for every app.

### Integration Layers

#### Layer 1: Internal State

This is fully controllable by QuickDev and should always work consistently.

- projects
- session history
- notes
- links
- startup commands
- preferences

#### Layer 2: OS-Level Launching

This is generally reliable on all supported platforms.

- open folder in file explorer
- open URL in default browser
- open terminal at project path
- launch supported editor with file paths

#### Layer 3: Tool-Specific Restore

This is useful but varies by app.

- VS Code file reopening
- Cursor file reopening
- JetBrains support later
- Zed support later

### Platform Notes

#### macOS

Feasible actions:

- open files with known apps
- launch Terminal or iTerm with working directory
- open Finder at project path

Main challenges:

- app bundle detection
- permission differences for some automation flows
- inconsistent behavior between terminal apps

#### Windows

Feasible actions:

- open files and folders with known executables
- launch Windows Terminal or PowerShell in a project directory

Main challenges:

- path escaping
- install location discovery
- app launch differences between `cmd`, PowerShell, and direct executable paths

#### Linux (Debian/Ubuntu)

Feasible actions:

- open files, folders, and URLs through standard utilities
- launch supported editors and terminals if installed

Main challenges:

- desktop environment differences
- package/install variation
- terminal emulator fragmentation

### Adapter Model

Each supported tool should have a small adapter that defines:

- how to detect availability
- how to build launch arguments
- what restore actions are supported
- what fallback behavior should occur

Suggested adapter targets for v1:

- `vscode`
- `cursor`
- `terminal`
- `browser`
- `file_explorer`

### Failure Handling

Restore is not binary. The UI should report:

- restored
- partially restored
- skipped
- unavailable on this system

This is important for trust. Users should always know what happened.

## 4. UI Structure Based on the Reference Style

The reference image is useful as a direction for tone and layout, but QuickDev should adapt it to the real product purpose.

### Design Direction

Use the reference image's structured dark product UI, but make it more grounded:

- less rounded edges
- less decorative color
- more emphasis on restore workflows
- less “concept dashboard” styling

### Visual Principles

- dark neutral base with restrained accent color
- compact, structured layout
- clear hierarchy through spacing and contrast, not glow
- mostly solid surfaces with subtle borders
- radius around 8px to 12px for cards, controls, and panels

### Layout Recommendation

#### Left Sidebar

Use a fixed-width sidebar for:

- projects
- recent sessions
- pinned projects
- preferences shortcut

Keep it simple:

- solid background
- single border-right
- no floating shell
- no oversized brand block

#### Top Bar

Use a compact header for:

- project breadcrumb
- current session state
- save session action
- restore action
- search

Avoid decorative chips that do not help the workflow.

#### Main Workspace

The main content area should focus on session recovery, not analytics.

Recommended sections:

- project header
- last session summary
- restore options
- session timeline / snapshot history
- tracked items list

#### Right Panel

Optional for desktop widths only:

- restore preview
- integration status
- notes

Remove it on smaller screens instead of stacking too much low-priority content.

### Suggested Screens

#### Dashboard

Purpose:

- show projects needing attention
- show latest restorable sessions
- surface failed or partial restores

This should not be a KPI-heavy analytics page.

#### Project Detail

Purpose:

- central place to save, inspect, and restore sessions

Suggested modules:

- project info
- last snapshot
- snapshot history
- tracked files
- commands
- links
- notes

#### Restore Review

Purpose:

- preview what will be reopened before restore

Suggested modules:

- files to reopen
- commands to run
- links to open
- unavailable items

#### Settings

Purpose:

- integration detection
- default editor and terminal settings
- autosave preferences
- platform-specific behavior

### Color Direction

Since the product is a utility app, keep the palette calm and dark.

Suggested base direction:

- background near charcoal, not pure black
- surfaces slightly lifted but close in tone
- one primary accent for main actions
- semantic colors only where needed for success, warning, or failure

Avoid:

- multiple competing gradient cards
- oversized rounded chips everywhere
- decorative glass panels
- fake data widgets

## Delivery Roadmap

### Phase 1: Foundation

- project model
- SQLite schema
- local project CRUD
- session and snapshot persistence
- basic dark UI shell

### Phase 2: Restore Engine

- adapter interface
- VS Code and terminal support
- manual save and restore flows
- restore result reporting

### Phase 3: Quality and Cross-Platform Hardening

- macOS validation
- Windows validation
- Ubuntu/Debian validation
- path normalization improvements
- error handling and fallback UX

### Phase 4: Expansion

- more editor integrations
- smarter autosnapshots
- startup command groups
- project activity timeline

## Success Criteria

The MVP is successful if:

- a user can add a project in under 1 minute
- saving a session is clear and reliable
- restoring a session reliably reopens supported tools
- failures are transparent, not hidden
- the app feels faster and easier than manually rebuilding context

## Product Positioning

QuickDev should be described as:

- a personal developer workspace recovery tool
- a local project session manager
- a resume-where-you-left-off desktop app

It should not be positioned as a generic project-management platform.
