# QuickDev Technical Architecture

## Overview

QuickDev is a local-first desktop application built to save and restore developer project sessions across macOS, Windows, and Linux. The architecture should optimize for reliability, offline behavior, and explicit support for a defined set of tools rather than attempting universal machine-wide session recovery.

The closest reference point is Microsoft PowerToys Workspaces, but QuickDev should be more opinionated around software development workflows. It should understand projects, sessions, restore targets, editors, commands, and project-specific preferences.

## Architecture Goals

- local-first and offline-capable
- cross-platform support through adapter isolation
- reliable persistence of project and session state
- transparent restore behavior with clear partial-failure reporting
- incremental rollout of tool support without destabilizing the core app

## System Layers

### 1. Frontend Layer

Technology:

- React
- TypeScript
- Vite
- Tailwind CSS
- shadcn/ui primitives where useful

Responsibilities:

- project management UI
- session capture and restore flows
- restore review and result reporting
- integration status display
- settings and platform-specific preferences

Frontend should remain thin. It should orchestrate workflows and render state, but core restore logic should live in Rust.

### 2. Desktop Application Shell

Technology:

- Tauri 2

Responsibilities:

- bridge between frontend and native capabilities
- window lifecycle hooks
- plugin-based access to shell, filesystem, dialog, and notifications
- packaging for macOS, Windows, and Linux

Tauri is a good fit because QuickDev does not need a browser-only deployment and benefits from native filesystem and process launch capabilities.

### 3. Native Core Layer

Technology:

- Rust

Responsibilities:

- project and session services
- restore engine
- integration adapter registry
- OS-specific command execution and path normalization
- persistence access layer
- validation and error classification

This layer should own the product's core rules. The frontend should not contain restore-specific business logic beyond presentation-level decisions.

### 4. Persistence Layer

Technology:

- SQLite
- SQLx migrations

Responsibilities:

- durable local storage for projects, sessions, items, preferences, and restore history
- indexing for fast project and session queries
- migration safety as the schema evolves

## Core Modules

### Project Service

Responsibilities:

- create, update, archive, and list projects
- validate project paths
- store preferred tools and restore defaults
- update last-opened metadata

Primary outputs:

- project summaries
- project details
- project preferences

### Session Service

Responsibilities:

- create manual snapshots
- create autosave snapshots
- create shutdown snapshots
- load session history
- serialize tracked items into persistent session records

Session capture should support both explicit user actions and app-driven autosave flows.

### Restore Engine

Responsibilities:

- load a session snapshot
- group items by adapter/tool
- validate item availability before launch
- execute restores in a controlled order
- record per-item outcomes
- produce a restore summary

Recommended restore order:

1. validate project path and adapter availability
2. open root folder
3. open editor targets
4. launch terminals and commands
5. open URLs
6. write restore results to `restore_runs`

### Integration Registry

Responsibilities:

- detect supported tools
- register adapter capabilities
- expose platform-aware launch strategies
- report whether a tool is available, partially supported, or unavailable

This registry should make it easy to add adapters over time without reshaping the entire app.

### Settings Service

Responsibilities:

- global defaults
- per-project overrides
- autosnapshot cadence
- default editor and terminal selection
- restore confirmation rules

## Adapter Architecture

Supported tools should be implemented as adapters behind a shared interface.

### Shared Adapter Interface

Each adapter should define:

- `tool_id`
- `display_name`
- supported platforms
- detection logic
- supported item types
- launch/restore logic
- fallback behavior

Conceptual operations:

- `detect()`
- `is_available()`
- `restore_items()`
- `open_project()`
- `open_files()`
- `build_launch_plan()`

### V1 Adapter Targets

- `vscode`
- `cursor`
- `zed`
- `notepad`
- `nano`
- `terminal`
- `browser`
- `file_explorer`

### Adapter Notes

#### VS Code

- best initial editor integration
- launch via `code`
- supports opening folders and files reliably

#### Cursor

- similar workflow if CLI is available
- should be treated as capability-based, not assumed installed

#### Zed

- v1 support should focus on launch-based restore if CLI is available
- deeper workspace/session awareness can come later

#### Notepad

- Windows-only lightweight text restore path
- useful as a fallback for simple file opening

#### Nano

- terminal-based restore path
- should open through a terminal adapter, not as a standalone GUI integration
- availability depends on system install and terminal setup

#### Terminal

- terminal adapter should support project-root launch
- command execution should be opt-in and explicit
- different terminal apps may need different launch strategies per platform

## Cross-Platform Strategy

### General Rule

Platform differences should be isolated in native adapters and utility modules, not leaked across the frontend or core services.

### OS Utility Modules

Recommended native modules:

- `path_utils`
- `platform_detection`
- `process_launcher`
- `editor_detection`
- `terminal_detection`
- `shell_command_builder`

### macOS

Key concerns:

- app bundle and CLI detection
- terminal choice differences
- app-launch permissions for some automation patterns

Recommended approach:

- prefer CLI-based launches when available
- fall back to opener-based file/folder launches where appropriate

### Windows

Key concerns:

- executable path discovery
- path quoting and escaping
- differences between PowerShell, `cmd`, and Windows Terminal

Recommended approach:

- standardize launch argument construction in one Windows-specific utility layer
- treat Notepad as a supported lightweight fallback editor

### Linux

Key concerns:

- desktop environment variation
- terminal emulator fragmentation
- install path variation

Recommended approach:

- support Debian/Ubuntu first
- use capability detection rather than hard assumptions
- prefer common CLI/editor patterns

## Data Model

The main persistence entities are:

- `projects`
- `sessions`
- `session_items`
- `integrations`
- `project_preferences`
- `restore_runs`

See [product-plan.md](/Users/orion-abrar/Code/Desktop-Application/QuickDev/docs/product-plan.md) for the high-level schema definitions.

### Important Data Rules

- paths must be stored as absolute paths
- timestamps must be stored in UTC
- restore records must be append-only for debugging and trust
- integration-specific metadata should live in JSON fields only when normalization is not practical

## Frontend State Model

The frontend should separate:

- server-like persisted state coming from Rust commands
- transient UI state such as selection, filters, or dialogs
- restore workflow state such as validation, progress, and result summaries

Recommended page-level domains:

- projects
- project detail
- sessions
- restore review
- settings

## Tauri Command Boundaries

Frontend should call focused native commands instead of broad catch-all endpoints.

Recommended command groups:

- `projects`
- `sessions`
- `restore`
- `integrations`
- `settings`

Examples:

- `create_project`
- `list_projects`
- `get_project_detail`
- `create_session_snapshot`
- `list_project_sessions`
- `preview_restore`
- `execute_restore`
- `detect_integrations`
- `update_project_preferences`

## Restore Execution Model

Restore should be a two-step process.

### Step 1: Preview

The app builds a launch plan and reports:

- items to restore
- required adapters
- unavailable tools
- user confirmation needs

### Step 2: Execute

The app launches supported items and returns:

- restored items
- skipped items
- failed items
- human-readable reasons

This model improves safety and trust, especially when commands or multiple apps are involved.

## Error Handling

Errors should be classified, not just logged.

Recommended categories:

- invalid project path
- unsupported platform
- adapter unavailable
- missing executable
- launch failure
- permission issue
- malformed session data

The UI should turn these into plain restore results instead of generic exception messages.

## Security and Privacy

QuickDev should default to a local-only posture.

Rules:

- do not upload project metadata anywhere
- do not run startup commands automatically without explicit user consent
- treat saved commands and file paths as sensitive local data
- avoid reading external app internals unless a supported integration requires it

## UI Architecture Direction

The UI should reflect the product's real job: session recovery.

### Layout

- fixed left sidebar for projects and navigation
- compact top bar for project context and primary actions
- main content focused on session history and restore flows
- optional right rail for restore preview or notes on wide screens

### Style

- dark, structured, utility-first interface
- inspired by the reference image but with less rounding
- mostly solid surfaces
- restrained accent use
- no metric-card-first dashboard

### Visual Scale

- card and panel radius: 8px to 12px
- button radius: 8px
- compact spacing and strong alignment

## Recommended Directory Responsibilities

### Frontend

- `src/pages/`
  - project-level screens
- `src/components/`
  - reusable UI and domain components
- `src/lib/`
  - frontend utilities and Tauri API wrappers
- `src/types/`
  - shared request/response and domain types

### Native

- `src-tauri/src/commands/`
  - Tauri command handlers
- `src-tauri/src/services/`
  - project, session, restore, integration services
- `src-tauri/src/adapters/`
  - tool-specific adapter implementations
- `src-tauri/src/platform/`
  - OS-specific launch and detection helpers
- `src-tauri/src/db.rs`
  - database bootstrap and connection wiring
- `src-tauri/migrations/`
  - schema migrations

## Suggested Delivery Order

1. persistence and project CRUD
2. session snapshot creation and history
3. integration detection and registry
4. restore preview flow
5. restore execution for VS Code, Zed, terminal, browser, file explorer
6. Cursor, Notepad, and Nano support
7. cross-platform hardening

This order keeps the app useful early while reducing risk from tool-specific complexity.
