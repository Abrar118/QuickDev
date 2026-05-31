# QuickDev `main.rs` Refactor — Design

**Date:** 2026-05-31
**Status:** Approved

## Goal

Split the 718-line `src/main.rs` into focused modules (`cli.rs` + a `commands/` directory) with **zero behavior change**. This is a pure structural move: relocate code, adjust paths and visibility, nothing else.

## Non-Goals

- No dependency injection, no IO seams, no handler rewrites.
- No new tests; the existing 53 tests stay as-is.
- No move into the library crate. Command handlers remain binary-private.
- No logic, signature, or output changes. Handler bodies are byte-for-byte identical apart from `use` paths.

## Why

`main.rs` holds the CLI definitions and every command handler inline. It is the largest file in the project and mixes unrelated responsibilities. Splitting by command makes each unit focused and easier to reason about, and sets up later work (e.g. `doctor`/`validate`, richer summaries) to land in small files instead of growing one giant one.

## Target Structure

```
src/
  main.rs          # module decls + thin main(): parse -> dispatch -> error-exit
  cli.rs           # clap structs: Cli, Commands, AddKind, RemoveKind
  commands/
    mod.rs         # declares submodules + re-exports handlers
    init.rs        # cmd_init
    launch.rs      # cmd_launch + print_launch_summary
    list.rs        # cmd_list
    add.rs         # cmd_add + cmd_add_interactive + pick_emulator + pick_application
    remove.rs      # cmd_remove + cmd_remove_interactive
    edit.rs        # cmd_edit
    deregister.rs  # cmd_deregister
    shared.rs      # prompt, build_item_display_list, parse_selected_items
  (config.rs, launch.rs, adapters.rs, apps.rs, fzf.rs, models.rs, parse.rs unchanged)
```

## Code Mapping

Every item currently in `main.rs` moves to exactly one destination:

| Current item (`main.rs`) | Destination |
|---|---|
| `Cli`, `Commands`, `AddKind`, `RemoveKind` (+ their `after_help` text) | `cli.rs` |
| `fn main()` + dispatch `match` + error-exit | `main.rs` (thin) |
| `cmd_init` | `commands/init.rs` |
| `cmd_launch` | `commands/launch.rs` |
| `print_launch_summary` | `commands/launch.rs` |
| `cmd_list` | `commands/list.rs` |
| `cmd_add` | `commands/add.rs` |
| `cmd_add_interactive` | `commands/add.rs` |
| `pick_emulator` | `commands/add.rs` |
| `pick_application` | `commands/add.rs` |
| `cmd_remove` | `commands/remove.rs` |
| `cmd_remove_interactive` | `commands/remove.rs` |
| `cmd_edit` | `commands/edit.rs` |
| `cmd_deregister` | `commands/deregister.rs` |
| `prompt` | `commands/shared.rs` |
| `build_item_display_list` | `commands/shared.rs` |
| `parse_selected_items` | `commands/shared.rs` |

### Rationale for shared placement

- `build_item_display_list` and `parse_selected_items` are used by **both** `cmd_launch` (launch.rs) and `cmd_remove_interactive` (remove.rs) → `shared.rs`.
- `prompt` is a generic stdin helper → `shared.rs`.
- `print_launch_summary` is used only by launch → stays with `cmd_launch`.
- `pick_emulator` / `pick_application` are add-only → `add.rs`.

## Key Decisions

1. **Binary crate, not library.** `commands` and `cli` are `pub(crate)` modules of the binary. `lib.rs` is untouched. This matches the existing pattern (the binary already declares its own `mod config;` etc., separate from the library crate) and keeps the diff minimal and the public API unchanged.
2. **Visibility:** handler fns, helper fns, and the `cli` enums/fields that `commands/` destructures become `pub(crate)`. Nothing becomes `pub`.
3. **Path rewrites only:** handler bodies switch `use config::…` → `use crate::config::…`, reference shared helpers via `crate::commands::shared::…` (or a local `use`), and reference CLI types via `crate::cli::{AddKind, RemoveKind}`. No other edits.
4. **`commands/mod.rs` is a re-export hub** so `main.rs` dispatch can call `commands::cmd_init(...)` etc. without deep paths.

## Module Wiring

- `main.rs` declares: `mod cli; mod commands; mod adapters; mod apps; mod config; mod fzf; mod launch; mod models; mod parse;` (the existing leaf-module declarations stay).
- `commands/mod.rs` declares: `pub(crate) mod init; … mod shared;` and re-exports each `cmd_*` (e.g. `pub(crate) use init::cmd_init;`).
- `main()` keeps the exact dispatch `match cli.command { … }` and the `eprintln!("error: {e}"); process::exit(1);` behavior.

## Verification

Pure-move means the safety net is "everything still compiles and behaves identically":

- `cargo fmt --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo build`
- `cargo test` → all 53 pass, **unchanged**
- Manual smoke after completion: `quickdev init`, `quickdev list`, `quickdev add` (interactive), `quickdev launch` still work as before.

Each task in the plan ends by running fmt + clippy + build (and test where applicable) so a broken move is caught immediately, file by file.

## Risks

- **Visibility churn:** forgetting a `pub(crate)` on a moved item surfaces as a compile error — caught immediately, not a silent behavior change.
- **Branch strategy:** `main.rs` is also heavily edited on the open `robustness-fixes` branch (PR #1). This refactor must be sequenced relative to that merge to avoid a large conflict; to be resolved at plan/execution time, not a design concern.
