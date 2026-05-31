# QuickDev `main.rs` Refactor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Split the 718-line `src/main.rs` into `cli.rs` + a `commands/` module directory with zero behavior change.

**Architecture:** Pure structural move. CLI clap structs go to `cli.rs`; each command handler goes to its own file under `commands/`; three cross-command helpers go to `commands/shared.rs`. Everything stays in the **binary crate** as `pub(crate)` items — `lib.rs` and `tests/` are untouched. Handler bodies are moved byte-for-byte; the sole body edit is one call rename in `commands/launch.rs`.

**Tech Stack:** Rust 2021, clap (derive). No new dependencies.

---

## Verification model (read first)

This is a refactor with **no new tests**. The regression guard is the existing suite plus the compiler/linter. After every task, the standard check is:

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo build
cargo test
```

Expected every time: clippy clean (no warnings), build succeeds, **all 53 tests pass**. If clippy flags an unused import in `src/main.rs` after a handler moves out, delete exactly the import(s) it names — that is the intended import cleanup for that task.

Source line ranges below refer to the **current** `src/main.rs` (718 lines). Move the referenced lines **verbatim** unless a step explicitly says to edit them.

---

## File Structure

```
src/
  main.rs          # module decls + thin main(): parse -> dispatch -> error-exit
  cli.rs           # Cli, Commands, AddKind, RemoveKind  (NEW)
  commands/
    mod.rs         # submodule decls + handler re-exports  (NEW)
    shared.rs      # prompt, build_item_display_list, parse_selected_items  (NEW)
    init.rs        # cmd_init  (NEW)
    launch.rs      # cmd_launch + print_launch_summary  (NEW)
    list.rs        # cmd_list  (NEW)
    add.rs         # cmd_add + cmd_add_interactive + pick_emulator + pick_application  (NEW)
    remove.rs      # cmd_remove + cmd_remove_interactive  (NEW)
    edit.rs        # cmd_edit  (NEW)
    deregister.rs  # cmd_deregister  (NEW)
  (config.rs, launch.rs, adapters.rs, apps.rs, fzf.rs, models.rs, parse.rs unchanged)
```

---

## Task 1: Extract CLI definitions to `cli.rs`

**Files:**
- Create: `src/cli.rs`
- Modify: `src/main.rs` (lines 1-145 region)

- [ ] **Step 1: Create `src/cli.rs`**

Add this header, then move the clap type definitions **verbatim** from `src/main.rs` lines **19-126** (`Cli`, `Commands`, `AddKind`, `RemoveKind`, with all their `after_help` text). Mark each top-level type `pub(crate)`, and mark the `Cli.command` field `pub(crate)`.

File starts with:

```rust
use clap::{Parser, Subcommand};
```

Then the moved types, with visibility added. The first type becomes:

```rust
#[derive(Parser)]
#[command(
    name = "quickdev",
    about = "Manage and launch project terminal/app configurations",
    after_help = "\
Examples:
  quickdev init                                         Initialize a project
  quickdev init --from my-api                           Clone config from another project
  quickdev launch                                       Select items to launch
  quickdev launch --all                                 Launch everything
  quickdev launch my-api                                Interactive picker for a named project
  quickdev add                                          Interactive add
  quickdev remove                                       Interactive removal picker
  quickdev list                                         Show all projects
  quickdev edit                                         Edit project config
  quickdev edit --global                                Edit global config
  quickdev deregister                                   Unregister project"
)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Commands,
}
```

The three enums change only their declaration line to add visibility (variant fields are auto-visible for a `pub(crate)` enum — do not touch them):

```rust
#[derive(Subcommand)]
pub(crate) enum Commands { /* variants verbatim from lines 43-82 */ }

#[derive(Subcommand)]
pub(crate) enum AddKind { /* variants verbatim from lines 85-118 */ }

#[derive(Subcommand)]
pub(crate) enum RemoveKind { /* variants verbatim from lines 121-126 */ }
```

- [ ] **Step 2: Rewrite the top of `src/main.rs`**

Replace `src/main.rs` lines **1-126** (the leaf `mod` declarations, the `use` block, and the four type definitions) with:

```rust
mod adapters;
mod apps;
mod cli;
mod config;
mod fzf;
mod launch;
mod models;
mod parse;

use clap::Parser;
use cli::{Cli, Commands};
use config::{
    global_config_path, load_global_config, load_project_config, resolve_project_config,
    save_global_config, save_project_config, unique_project_name,
};
use launch::LaunchResult;
use models::{AppEntry, GlobalProjectEntry, ProjectConfig, ProjectEntry, TerminalEntry};
use std::path::PathBuf;
use std::process;
```

Leave `fn main()` (currently lines 128-145) and every `fn cmd_*` / helper below it exactly as-is. `Cli` and `Commands` now resolve through the `use cli::{Cli, Commands};` import, so `fn main()` is unchanged.

- [ ] **Step 3: Verify**

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo build
cargo test
```

Expected: clippy clean, build ok, 53 tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/cli.rs src/main.rs
git commit -m "refactor: extract CLI definitions into cli.rs"
```

---

## Task 2: Create `commands` module with `shared.rs`

**Files:**
- Create: `src/commands/mod.rs`, `src/commands/shared.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Create `src/commands/shared.rs`**

Header plus the three helpers moved **verbatim** from `src/main.rs`: `prompt` (lines 308-315), `build_item_display_list` (582-596), `parse_selected_items` (598-621). Each gains `pub(crate)` on its `fn` line; bodies are unchanged.

```rust
use crate::models::ProjectConfig;

pub(crate) fn prompt(message: &str) -> Result<String, String> {
    eprint!("{message}");
    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .map_err(|e| format!("failed to read input: {e}"))?;
    Ok(input.trim().to_string())
}

pub(crate) fn build_item_display_list(config: &ProjectConfig) -> Vec<String> {
    let mut items = Vec::new();
    for t in &config.terminals {
        let cmd_part = t
            .command
            .as_ref()
            .map(|c| format!(" ({c})"))
            .unwrap_or_default();
        items.push(format!("[terminal] {} — {}{}", t.name, t.path, cmd_part));
    }
    for a in &config.applications {
        items.push(format!("[app] {} — {}", a.name, a.path));
    }
    items
}

pub(crate) fn parse_selected_items(selected: &[String]) -> (Vec<String>, Vec<String>) {
    let mut terminal_names = Vec::new();
    let mut app_names = Vec::new();

    for line in selected {
        if line.starts_with("[terminal] ") {
            let name = line
                .strip_prefix("[terminal] ")
                .and_then(|s| s.split(" — ").next())
                .unwrap_or("")
                .to_string();
            terminal_names.push(name);
        } else if line.starts_with("[app] ") {
            let name = line
                .strip_prefix("[app] ")
                .and_then(|s| s.split(" — ").next())
                .unwrap_or("")
                .to_string();
            app_names.push(name);
        }
    }

    (terminal_names, app_names)
}
```

- [ ] **Step 2: Create `src/commands/mod.rs`**

```rust
pub(crate) mod shared;
```

- [ ] **Step 3: Wire `commands` into `src/main.rs` and delete the moved helpers**

In `src/main.rs`:
1. Add `mod commands;` to the module-declaration block (keep alphabetical: after `mod cli;`).
2. Add `use commands::shared::{build_item_display_list, parse_selected_items, prompt};` to the `use` block.
3. Delete the three function definitions now living in `shared.rs`: `prompt` (old lines 308-315), `build_item_display_list` (582-596), `parse_selected_items` (598-621).

All existing call sites (`prompt(...)`, `build_item_display_list(...)`, `parse_selected_items(...)`) are unchanged — the `use` brings the same names into scope.

- [ ] **Step 4: Verify**

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo build
cargo test
```

Expected: clippy clean, build ok, 53 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/commands/mod.rs src/commands/shared.rs src/main.rs
git commit -m "refactor: add commands module with shared helpers"
```

---

## Task 3: Move `cmd_init` to `commands/init.rs`

**Files:**
- Create: `src/commands/init.rs`
- Modify: `src/commands/mod.rs`, `src/main.rs`

- [ ] **Step 1: Create `src/commands/init.rs`**

Header below, then move `cmd_init` **verbatim** from `src/main.rs` lines **147-238**, changing only `fn cmd_init` → `pub(crate) fn cmd_init`.

```rust
use crate::config::{
    global_config_path, load_global_config, load_project_config, save_global_config,
    save_project_config, unique_project_name,
};
use crate::models::{GlobalProjectEntry, ProjectConfig, ProjectEntry};
use std::path::PathBuf;
```

- [ ] **Step 2: Wire and delete from `main.rs`**

In `src/commands/mod.rs` add:

```rust
pub(crate) mod init;
pub(crate) use init::cmd_init;
```

In `src/main.rs`: delete `cmd_init` (old lines 147-238) and change the dispatch arm to:

```rust
        Commands::Init { from } => commands::cmd_init(from),
```

- [ ] **Step 3: Verify and clean imports**

```bash
cargo clippy --all-targets -- -D warnings
```

If clippy flags any now-unused import in `src/main.rs`, delete exactly those. Then:

```bash
cargo fmt
cargo build
cargo test
```

Expected: clippy clean, build ok, 53 tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/commands/init.rs src/commands/mod.rs src/main.rs
git commit -m "refactor: move cmd_init into commands/init.rs"
```

---

## Task 4: Move `cmd_launch` + `print_launch_summary` to `commands/launch.rs`

**Files:**
- Create: `src/commands/launch.rs`
- Modify: `src/commands/mod.rs`, `src/main.rs`

- [ ] **Step 1: Create `src/commands/launch.rs`**

Header below, then move `cmd_launch` (lines **240-306**) and `print_launch_summary` (lines **317-330**) **verbatim**, with these two edits:
- `fn cmd_launch` → `pub(crate) fn cmd_launch` (leave `print_launch_summary` private).
- Inside `cmd_launch`, change the call `launch::launch_project(` (old line 297) → `launch_project(` (it now comes from the `use` below; the bare `launch::` would otherwise refer to this new module).

```rust
use crate::commands::shared::{build_item_display_list, parse_selected_items};
use crate::config::{
    global_config_path, load_global_config, load_project_config, resolve_project_config,
};
use crate::fzf;
use crate::launch::{launch_project, LaunchResult};
use crate::models::ProjectConfig;
use std::path::PathBuf;
use std::process;
```

- [ ] **Step 2: Wire and delete from `main.rs`**

In `src/commands/mod.rs` add:

```rust
pub(crate) mod launch;
pub(crate) use launch::cmd_launch;
```

In `src/main.rs`: delete `cmd_launch` (240-306) and `print_launch_summary` (317-330), and change the dispatch arm to:

```rust
        Commands::Launch { project, all } => commands::cmd_launch(project, all),
```

Note: `mod launch;` (the leaf terminal module) and `commands::launch` coexist — distinct paths, no conflict.

- [ ] **Step 3: Verify and clean imports**

```bash
cargo clippy --all-targets -- -D warnings
```

Delete any `src/main.rs` imports clippy now reports unused (expect `LaunchResult` and possibly `process` to remain used by `main()`; `process` stays). Then:

```bash
cargo fmt
cargo build
cargo test
```

Expected: clippy clean, build ok, 53 tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/commands/launch.rs src/commands/mod.rs src/main.rs
git commit -m "refactor: move cmd_launch into commands/launch.rs"
```

---

## Task 5: Move `cmd_list` to `commands/list.rs`

**Files:**
- Create: `src/commands/list.rs`
- Modify: `src/commands/mod.rs`, `src/main.rs`

- [ ] **Step 1: Create `src/commands/list.rs`**

Header below, then move `cmd_list` **verbatim** from lines **332-378**, changing only `fn cmd_list` → `pub(crate) fn cmd_list`.

```rust
use crate::config::{global_config_path, load_global_config, load_project_config};
use std::path::PathBuf;
```

- [ ] **Step 2: Wire and delete from `main.rs`**

In `src/commands/mod.rs` add:

```rust
pub(crate) mod list;
pub(crate) use list::cmd_list;
```

In `src/main.rs`: delete `cmd_list` (332-378) and change the dispatch arm to:

```rust
        Commands::List => commands::cmd_list(),
```

- [ ] **Step 3: Verify and clean imports**

```bash
cargo clippy --all-targets -- -D warnings
```

Delete any newly-unused `src/main.rs` imports clippy names. Then:

```bash
cargo fmt
cargo build
cargo test
```

Expected: clippy clean, build ok, 53 tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/commands/list.rs src/commands/mod.rs src/main.rs
git commit -m "refactor: move cmd_list into commands/list.rs"
```

---

## Task 6: Move `cmd_add` (+ interactive helpers) to `commands/add.rs`

**Files:**
- Create: `src/commands/add.rs`
- Modify: `src/commands/mod.rs`, `src/main.rs`

- [ ] **Step 1: Create `src/commands/add.rs`**

Header below, then move **verbatim**: `cmd_add` (lines **380-420**), `cmd_add_interactive` (422-488), `pick_emulator` (490-502), `pick_application` (504-550). Change only `fn cmd_add` → `pub(crate) fn cmd_add` (the other three stay private — they are only used inside `add.rs`).

```rust
use crate::apps;
use crate::cli::AddKind;
use crate::commands::shared::prompt;
use crate::config::{load_project_config, resolve_project_config, save_project_config};
use crate::fzf;
use crate::models::{AppEntry, ProjectConfig, TerminalEntry};
use crate::parse;
use std::path::PathBuf;
```

- [ ] **Step 2: Wire and delete from `main.rs`**

In `src/commands/mod.rs` add:

```rust
pub(crate) mod add;
pub(crate) use add::cmd_add;
```

In `src/main.rs`: delete `cmd_add`, `cmd_add_interactive`, `pick_emulator`, `pick_application` (the contiguous block, old lines 380-550), and change the dispatch arm to:

```rust
        Commands::Add { kind } => commands::cmd_add(kind),
```

- [ ] **Step 3: Verify and clean imports**

```bash
cargo clippy --all-targets -- -D warnings
```

`prompt` is no longer used in `src/main.rs` — clippy will flag it. Update the shared import in `main.rs` to drop it:

```rust
use commands::shared::{build_item_display_list, parse_selected_items};
```

Delete any other imports clippy names (e.g. `AppEntry`, `TerminalEntry` if now unused in `main.rs`). Then:

```bash
cargo fmt
cargo build
cargo test
```

Expected: clippy clean, build ok, 53 tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/commands/add.rs src/commands/mod.rs src/main.rs
git commit -m "refactor: move cmd_add and pickers into commands/add.rs"
```

---

## Task 7: Move `cmd_remove` (+ interactive) to `commands/remove.rs`

**Files:**
- Create: `src/commands/remove.rs`
- Modify: `src/commands/mod.rs`, `src/main.rs`

- [ ] **Step 1: Create `src/commands/remove.rs`**

Header below, then move **verbatim**: `cmd_remove` (lines **552-580**) and `cmd_remove_interactive` (lines **623-654**). Change only `fn cmd_remove` → `pub(crate) fn cmd_remove` (keep `cmd_remove_interactive` private).

```rust
use crate::cli::RemoveKind;
use crate::commands::shared::{build_item_display_list, parse_selected_items};
use crate::config::{load_project_config, resolve_project_config, save_project_config};
use crate::fzf;
use crate::models::ProjectConfig;
use std::path::PathBuf;
```

- [ ] **Step 2: Wire and delete from `main.rs`**

In `src/commands/mod.rs` add:

```rust
pub(crate) mod remove;
pub(crate) use remove::cmd_remove;
```

In `src/main.rs`: delete `cmd_remove` (552-580) and `cmd_remove_interactive` (623-654), and change the dispatch arm to:

```rust
        Commands::Remove { kind } => commands::cmd_remove(kind),
```

- [ ] **Step 3: Verify and clean imports**

```bash
cargo clippy --all-targets -- -D warnings
```

`build_item_display_list` and `parse_selected_items` are no longer used in `src/main.rs` — delete the whole line `use commands::shared::{build_item_display_list, parse_selected_items};`. Delete any other imports clippy names (e.g. `ProjectConfig` if now unused). Then:

```bash
cargo fmt
cargo build
cargo test
```

Expected: clippy clean, build ok, 53 tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/commands/remove.rs src/commands/mod.rs src/main.rs
git commit -m "refactor: move cmd_remove into commands/remove.rs"
```

---

## Task 8: Move `cmd_edit` to `commands/edit.rs`

**Files:**
- Create: `src/commands/edit.rs`
- Modify: `src/commands/mod.rs`, `src/main.rs`

- [ ] **Step 1: Create `src/commands/edit.rs`**

Header below, then move `cmd_edit` **verbatim** from lines **656-679**, changing only `fn cmd_edit` → `pub(crate) fn cmd_edit`.

```rust
use crate::config::{global_config_path, resolve_project_config};
use crate::parse;
```

- [ ] **Step 2: Wire and delete from `main.rs`**

In `src/commands/mod.rs` add:

```rust
pub(crate) mod edit;
pub(crate) use edit::cmd_edit;
```

In `src/main.rs`: delete `cmd_edit` (656-679) and change the dispatch arm to:

```rust
        Commands::Edit { global } => commands::cmd_edit(global),
```

- [ ] **Step 3: Verify and clean imports**

```bash
cargo clippy --all-targets -- -D warnings
```

Delete any newly-unused `src/main.rs` imports clippy names. Then:

```bash
cargo fmt
cargo build
cargo test
```

Expected: clippy clean, build ok, 53 tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/commands/edit.rs src/commands/mod.rs src/main.rs
git commit -m "refactor: move cmd_edit into commands/edit.rs"
```

---

## Task 9: Move `cmd_deregister` to `commands/deregister.rs`

**Files:**
- Create: `src/commands/deregister.rs`
- Modify: `src/commands/mod.rs`, `src/main.rs`

- [ ] **Step 1: Create `src/commands/deregister.rs`**

Header below, then move `cmd_deregister` **verbatim** from lines **681-718**, changing only `fn cmd_deregister` → `pub(crate) fn cmd_deregister`.

```rust
use crate::config::{
    global_config_path, load_global_config, resolve_project_config, save_global_config,
};
```

- [ ] **Step 2: Wire and delete from `main.rs`**

In `src/commands/mod.rs` add:

```rust
pub(crate) mod deregister;
pub(crate) use deregister::cmd_deregister;
```

In `src/main.rs`: delete `cmd_deregister` (681-718) and change the dispatch arm to:

```rust
        Commands::Deregister { delete } => commands::cmd_deregister(delete),
```

- [ ] **Step 3: Verify and clean imports**

```bash
cargo clippy --all-targets -- -D warnings
```

At this point every `cmd_*` has moved. The only imports `src/main.rs` should still need are `use clap::Parser;`, `use cli::{Cli, Commands};`, and `use std::process;`. Delete anything else clippy flags (expect `config::*`, `models::*`, `launch::LaunchResult`, `PathBuf` to all be gone). Then:

```bash
cargo fmt
cargo build
cargo test
```

Expected: clippy clean, build ok, 53 tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/commands/deregister.rs src/commands/mod.rs src/main.rs
git commit -m "refactor: move cmd_deregister into commands/deregister.rs"
```

---

## Task 10: Final verification

**Files:** none (verification only)

- [ ] **Step 1: Confirm `main.rs` is thin**

`src/main.rs` should now be only the module declarations, three `use` lines, and `fn main()`. Confirm it matches this shape:

```rust
mod adapters;
mod apps;
mod cli;
mod commands;
mod config;
mod fzf;
mod launch;
mod models;
mod parse;

use clap::Parser;
use cli::{Cli, Commands};
use std::process;

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Init { from } => commands::cmd_init(from),
        Commands::Launch { project, all } => commands::cmd_launch(project, all),
        Commands::List => commands::cmd_list(),
        Commands::Add { kind } => commands::cmd_add(kind),
        Commands::Remove { kind } => commands::cmd_remove(kind),
        Commands::Edit { global } => commands::cmd_edit(global),
        Commands::Deregister { delete } => commands::cmd_deregister(delete),
    };

    if let Err(e) = result {
        eprintln!("error: {e}");
        process::exit(1);
    }
}
```

Run: `wc -l src/main.rs` — expected: roughly 30 lines.

- [ ] **Step 2: Full suite + lint**

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo build
cargo test
```

Expected: fmt clean, clippy clean, build ok, 53 tests pass.

- [ ] **Step 3: Manual smoke test**

In a throwaway directory, confirm behavior is unchanged:

```bash
cargo run -- list
cargo run -- --help
cargo run -- add --help
```

Expected: `list` prints the project index (or the "No projects registered" message), `--help` shows the same examples block as before, `add --help` shows the terminal/app subcommands. No panics, no changed output.

- [ ] **Step 4: Commit any formatting-only changes (if `cargo fmt --check` made edits)**

```bash
git add -A && git commit -m "style: cargo fmt after refactor" || echo "nothing to commit"
```

---

## Self-Review Notes

- **Spec coverage:** Every item in the design's Code Mapping table is assigned to a task (cli → T1; shared → T2; init → T3; launch+summary → T4; list → T5; add+interactive+pickers → T6; remove+interactive → T7; edit → T8; deregister → T9). Binary-crate decision honored (no `lib.rs`/`tests/` edits). Verification matches the design (`fmt`/`clippy`/`build`/`test` + manual smoke).
- **Behavior preservation:** Only non-mechanical edit is `launch::launch_project` → `launch_project` in T4, required by the module-name shadow; called out explicitly.
- **Type/visibility consistency:** Handlers exposed via `pub(crate)`; internal helpers (`print_launch_summary`, `cmd_add_interactive`, `pick_emulator`, `pick_application`, `cmd_remove_interactive`) stay private to their files. `cli` types and `Cli.command` are `pub(crate)`.
