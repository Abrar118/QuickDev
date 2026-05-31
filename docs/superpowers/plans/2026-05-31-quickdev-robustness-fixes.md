# QuickDev Robustness Fixes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix seven verified correctness defects in QuickDev (picker name parsing, named-launch behavior, re-register name desync, quote-aware arg/editor parsing, shell escaping, and path traversal).

**Architecture:** Pure, testable helpers live in lib modules (`parse.rs`, `config.rs`, `launch.rs`); `main.rs` stays thin glue that calls them. One shared `parse_shell_args` helper backs both the interactive-app-args fix and the `$EDITOR` fix (DRY). Path validation moves `resolve_terminal_path` to a `Result` return; the launcher surfaces a rejected path as a per-terminal failure, matching the existing `LaunchResult` error flow.

**Tech Stack:** Rust 2021, clap, serde, toml, dirs; new dependency `shell-words`. Tests use the existing `tests/*_test.rs` integration style with `tempfile`.

---

## File Structure

- **Create:** `src/parse.rs` — `parse_shell_args(input) -> Result<Vec<String>, String>`. One shared quote-aware splitter, used by `main.rs` for app args (#4) and editor command (#5).
- **Create:** `tests/parse_test.rs` — tests for `parse_shell_args`.
- **Modify:** `Cargo.toml` — add `shell-words` dependency.
- **Modify:** `src/lib.rs` — export `pub mod parse;`.
- **Modify:** `src/config.rs` — add `parse_project_selection` (#1); use it in `fzf_select_project`.
- **Modify:** `src/launch.rs` — add `escape_applescript_string` / `escape_powershell_single_quotes` (#6); apply them; change `resolve_terminal_path` to `Result` and validate (#7); update `launch_project` caller.
- **Modify:** `src/main.rs` — add `mod parse;`; picker index wiring is in `config.rs`; re-register writeback in `cmd_init` (#3); app-args via `parse_shell_args` (#4); editor `$VISUAL`/`$EDITOR` + split (#5); named-launch gate (#2).
- **Modify:** `tests/launch_test.rs` — update two existing `resolve_terminal_path` tests to `.unwrap()`; add rejection + escaping tests.
- **Modify:** `tests/config_test.rs` — add `parse_project_selection` and re-register name-sync tests.
- **Modify:** `README.md` — document named-launch picker behavior (#2).

---

## Task 1: Shared quote-aware arg splitter (`parse.rs`)

**Files:**
- Modify: `Cargo.toml`
- Create: `src/parse.rs`
- Modify: `src/lib.rs`
- Create: `tests/parse_test.rs`

- [ ] **Step 1: Add the dependency**

In `Cargo.toml`, under `[dependencies]`, add the line after `dirs = "6"`:

```toml
shell-words = "1"
```

- [ ] **Step 2: Write the failing test**

Create `tests/parse_test.rs`:

```rust
use quickdev::parse::parse_shell_args;

#[test]
fn parse_shell_args_splits_quoted_value() {
    let result = parse_shell_args(r#"--profile "Dev User""#).unwrap();
    assert_eq!(result, vec!["--profile", "Dev User"]);
}

#[test]
fn parse_shell_args_plain_words() {
    let result = parse_shell_args("--flag value").unwrap();
    assert_eq!(result, vec!["--flag", "value"]);
}

#[test]
fn parse_shell_args_errors_on_unbalanced_quote() {
    let result = parse_shell_args(r#"--name "unterminated"#);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("invalid"));
}
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cargo test --test parse_test`
Expected: FAIL — compile error, `unresolved import quickdev::parse` / cannot find module `parse`.

- [ ] **Step 4: Create the module**

Create `src/parse.rs`:

```rust
/// Split a string into arguments, honoring shell-style quoting.
pub fn parse_shell_args(input: &str) -> Result<Vec<String>, String> {
    shell_words::split(input).map_err(|e| format!("invalid arguments: {e}"))
}
```

- [ ] **Step 5: Export the module**

In `src/lib.rs`, add after `pub mod models;`:

```rust
pub mod parse;
```

- [ ] **Step 6: Run test to verify it passes**

Run: `cargo test --test parse_test`
Expected: PASS (3 tests).

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml Cargo.lock src/parse.rs src/lib.rs tests/parse_test.rs
git commit -m "Add shell-words-backed parse_shell_args helper"
```

---

## Task 2: Quote-aware interactive app args and editor (#4, #5)

**Files:**
- Modify: `src/main.rs` (add `mod parse;`; `cmd_add_interactive` ~line 463-469; `cmd_edit` ~line 660-665)

This task is `main.rs` glue over the Task 1 helper; its parsing is covered by `tests/parse_test.rs`. Verify the wiring with manual runs.

- [ ] **Step 1: Declare the module in the binary**

In `src/main.rs`, in the `mod` block at the top (after `mod models;`), add:

```rust
mod parse;
```

- [ ] **Step 2: Use the splitter for interactive app args (#4)**

In `src/main.rs`, in `cmd_add_interactive`, replace this block:

```rust
            let args_input =
                prompt("Arguments (e.g., \".\" to open project root, Enter to skip): ")?;
            let args = if args_input.is_empty() {
                None
            } else {
                Some(args_input.split_whitespace().map(String::from).collect())
            };
```

with:

```rust
            let args_input =
                prompt("Arguments (e.g., \".\" to open project root, Enter to skip): ")?;
            let args = if args_input.is_empty() {
                None
            } else {
                Some(parse::parse_shell_args(&args_input)?)
            };
```

- [ ] **Step 3: Prefer `$VISUAL`, fall back to `$EDITOR`, and split (#5)**

In `src/main.rs`, in `cmd_edit`, replace:

```rust
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());

    std::process::Command::new(&editor)
        .arg(&config_path)
        .status()
        .map_err(|e| format!("failed to open editor '{}': {}", editor, e))?;

    Ok(())
```

with:

```rust
    let editor = std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .unwrap_or_else(|_| "vi".to_string());

    let parts = parse::parse_shell_args(&editor)?;
    let (program, leading) = parts
        .split_first()
        .ok_or("editor command is empty")?;

    std::process::Command::new(program)
        .args(leading)
        .arg(&config_path)
        .status()
        .map_err(|e| format!("failed to open editor '{}': {}", editor, e))?;

    Ok(())
```

- [ ] **Step 4: Build and verify it compiles**

Run: `cargo build`
Expected: compiles with no errors.

- [ ] **Step 5: Manually verify the editor fix**

Run: `EDITOR="echo opened" cargo run -- edit --global`
Expected: prints `opened <path-to>/config.toml` (proves `echo` ran with the path as an arg, not a binary literally named `echo opened`).

- [ ] **Step 6: Commit**

```bash
git add src/main.rs
git commit -m "Parse interactive app args and \$EDITOR with shell-words"
```

---

## Task 3: Index-based project picker (#1)

**Files:**
- Modify: `src/config.rs` (add `parse_project_selection`; rewrite item formatting and selection in `fzf_select_project` ~lines 127-144)
- Modify: `tests/config_test.rs`

- [ ] **Step 1: Write the failing test**

In `tests/config_test.rs`, add `parse_project_selection` to the import list from `quickdev::config` (first `use` block), then add:

```rust
#[test]
fn parse_project_selection_extracts_index() {
    assert_eq!(parse_project_selection("3: my-proj    /tmp/x"), Ok(3));
}

#[test]
fn parse_project_selection_handles_name_with_spaces() {
    assert_eq!(
        parse_project_selection("0: Client A Project    /tmp/client a"),
        Ok(0)
    );
}

#[test]
fn parse_project_selection_rejects_garbage() {
    assert!(parse_project_selection("not-an-index").is_err());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test config_test parse_project_selection`
Expected: FAIL — compile error, cannot find function `parse_project_selection`.

- [ ] **Step 3: Add the parser**

In `src/config.rs`, add this `pub fn` (e.g. directly above `fn fzf_select_project`):

```rust
pub fn parse_project_selection(selected: &str) -> Result<usize, String> {
    selected
        .split_once(':')
        .and_then(|(index, _)| index.trim().parse::<usize>().ok())
        .ok_or_else(|| "invalid selection".to_string())
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test config_test parse_project_selection`
Expected: PASS (3 tests).

- [ ] **Step 5: Wire it into the picker**

In `src/config.rs`, in `fzf_select_project`, replace this block:

```rust
    let items: Vec<String> = global
        .projects
        .iter()
        .map(|p| format!("{:<20} {}", p.name, p.path))
        .collect();

    let selected = fzf::fzf_select_one(&items, "Select a project:")?;

    let project_name = selected
        .split_whitespace()
        .next()
        .ok_or("invalid selection")?;

    let entry = global
        .projects
        .iter()
        .find(|p| p.name == project_name)
        .ok_or_else(|| format!("project '{}' not found", project_name))?;
```

with:

```rust
    let items: Vec<String> = global
        .projects
        .iter()
        .enumerate()
        .map(|(i, p)| format!("{i}: {}    {}", p.name, p.path))
        .collect();

    let selected = fzf::fzf_select_one(&items, "Select a project:")?;

    let index = parse_project_selection(&selected)?;
    let entry = global
        .projects
        .get(index)
        .ok_or("invalid selection")?;
```

- [ ] **Step 6: Build and run full tests**

Run: `cargo build && cargo test --test config_test`
Expected: compiles; all config tests PASS.

- [ ] **Step 7: Commit**

```bash
git add src/config.rs tests/config_test.rs
git commit -m "Use index-based selection in project picker"
```

---

## Task 4: Re-register name sync (#3)

**Files:**
- Modify: `src/main.rs` (`cmd_init`, the `config_path.exists() && !already_indexed` branch, ~lines 168-178)
- Modify: `tests/config_test.rs`

The writeback is glue in `cmd_init` (not unit-testable directly because it reads cwd and `~/Documents`); the test below locks the two library primitives it composes (`unique_project_name` + persisting a renamed config). Verify the integration with a manual run.

- [ ] **Step 1: Write the failing test**

In `tests/config_test.rs`, add:

```rust
#[test]
fn renamed_project_config_persists() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join(".quickdev.toml");

    let cfg = ProjectConfig {
        project: ProjectEntry {
            name: "api".to_string(),
        },
        terminals: vec![],
        applications: vec![],
    };
    save_project_config(&config_path, &cfg).unwrap();

    // Global already has "api" → init must pick a unique name.
    let global = GlobalConfig {
        emulator: None,
        projects: vec![GlobalProjectEntry {
            name: "api".to_string(),
            path: "/tmp/other".to_string(),
        }],
    };
    let unique = unique_project_name("api", &global);
    assert_eq!(unique, "api-2");

    // The fix writes the unique name back to the local config.
    let mut existing = load_project_config(&config_path).unwrap();
    existing.project.name = unique.clone();
    save_project_config(&config_path, &existing).unwrap();

    let reloaded = load_project_config(&config_path).unwrap();
    assert_eq!(reloaded.project.name, "api-2");
}
```

- [ ] **Step 2: Run test to verify it passes against the helpers**

Run: `cargo test --test config_test renamed_project_config_persists`
Expected: PASS (the primitives already exist). This test guards the mechanism the next step relies on.

- [ ] **Step 3: Add the writeback in `cmd_init`**

In `src/main.rs`, replace this block:

```rust
    if config_path.exists() && !already_indexed {
        let existing = load_project_config(&config_path)?;
        let project_name = unique_project_name(&existing.project.name, &global);
        global.projects.push(GlobalProjectEntry {
            name: project_name.clone(),
            path: cwd_str,
        });
        save_global_config(&global_path, &global)?;
        println!("Re-registered project '{}' in global index", project_name);
        return Ok(());
    }
```

with:

```rust
    if config_path.exists() && !already_indexed {
        let mut existing = load_project_config(&config_path)?;
        let project_name = unique_project_name(&existing.project.name, &global);
        if existing.project.name != project_name {
            existing.project.name = project_name.clone();
            save_project_config(&config_path, &existing)?;
        }
        global.projects.push(GlobalProjectEntry {
            name: project_name.clone(),
            path: cwd_str,
        });
        save_global_config(&global_path, &global)?;
        println!("Re-registered project '{}' in global index", project_name);
        return Ok(());
    }
```

- [ ] **Step 4: Build to verify**

Run: `cargo build`
Expected: compiles with no errors.

- [ ] **Step 5: Commit**

```bash
git add src/main.rs tests/config_test.rs
git commit -m "Sync local config name on re-register"
```

---

## Task 5: Shell escaping helpers (#6)

**Files:**
- Modify: `src/launch.rs` (add helpers; apply at macOS Terminal.app and Windows pwsh sites)
- Modify: `tests/launch_test.rs`

- [ ] **Step 1: Write the failing test**

In `tests/launch_test.rs`, change the first `use` line to include the two helpers:

```rust
use quickdev::launch::{
    escape_applescript_string, escape_powershell_single_quotes, normalize_path, resolve_app_args,
    resolve_terminal_path,
};
```

Add these tests:

```rust
#[test]
fn escape_applescript_escapes_backslash_and_quote() {
    assert_eq!(escape_applescript_string(r#"a\b"c"#), r#"a\\b\"c"#);
}

#[test]
fn escape_applescript_leaves_plain_text() {
    assert_eq!(escape_applescript_string("plain text"), "plain text");
}

#[test]
fn escape_powershell_doubles_single_quotes() {
    assert_eq!(escape_powershell_single_quotes("Abrar's PC"), "Abrar''s PC");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test launch_test`
Expected: FAIL — compile error, unresolved imports `escape_applescript_string`, `escape_powershell_single_quotes`.

- [ ] **Step 3: Add the helpers**

In `src/launch.rs`, add at module top level (after the `use` lines, before `launch_project`). The `#[allow(dead_code)]` prevents unused-warnings on platforms that don't call a given helper:

```rust
#[allow(dead_code)]
pub fn escape_applescript_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

#[allow(dead_code)]
pub fn escape_powershell_single_quotes(s: &str) -> String {
    s.replace('\'', "''")
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test launch_test`
Expected: PASS — the three new escaping tests pass (existing tests unaffected).

- [ ] **Step 5: Apply the AppleScript helper (macOS site)**

In `src/launch.rs`, in `run_in_platform_terminal`, in the `#[cfg(target_os = "macos")]` block, replace:

```rust
        let escaped = full.replace('"', "\\\"");
```

with:

```rust
        let escaped = escape_applescript_string(&full);
```

- [ ] **Step 6: Apply the PowerShell helper (Windows pwsh site)**

In `src/launch.rs`, in `run_in_platform_terminal`, in the `#[cfg(target_os = "windows")]` block, replace:

```rust
        if command_exists("pwsh") {
            let ps_cmd = if cmd_str.is_empty() {
                format!("Set-Location '{cwd}'")
            } else {
                format!("Set-Location '{cwd}'; {cmd_str}")
            };
```

with:

```rust
        if command_exists("pwsh") {
            let safe_cwd = escape_powershell_single_quotes(cwd);
            let ps_cmd = if cmd_str.is_empty() {
                format!("Set-Location '{safe_cwd}'")
            } else {
                format!("Set-Location '{safe_cwd}'; {cmd_str}")
            };
```

- [ ] **Step 7: Build and test**

Run: `cargo build && cargo test --test launch_test`
Expected: compiles; all launch tests PASS.

- [ ] **Step 8: Commit**

```bash
git add src/launch.rs tests/launch_test.rs
git commit -m "Add and apply shell escaping helpers"
```

---

## Task 6: Reject terminal paths that escape the root (#7)

**Files:**
- Modify: `src/launch.rs` (`resolve_terminal_path` ~lines 58-74; `launch_project` caller ~line 26)
- Modify: `tests/launch_test.rs` (update two existing tests; add rejection tests)

- [ ] **Step 1: Update existing tests and add rejection tests (failing)**

In `tests/launch_test.rs`, the two existing `resolve_terminal_path` tests must unwrap the new `Result`. Replace:

```rust
#[test]
fn resolve_terminal_path_joins_relative() {
    let project_root = Path::new("/home/user/my-project");
    let result = resolve_terminal_path(project_root, ".");
    assert_eq!(result, "/home/user/my-project");
}

#[test]
fn resolve_terminal_path_joins_subdir() {
    let project_root = Path::new("/home/user/my-project");
    let result = resolve_terminal_path(project_root, "./src/server");
    assert_eq!(result, "/home/user/my-project/src/server");
}
```

with:

```rust
#[test]
fn resolve_terminal_path_joins_relative() {
    let project_root = Path::new("/home/user/my-project");
    let result = resolve_terminal_path(project_root, ".").unwrap();
    assert_eq!(result, "/home/user/my-project");
}

#[test]
fn resolve_terminal_path_joins_subdir() {
    let project_root = Path::new("/home/user/my-project");
    let result = resolve_terminal_path(project_root, "./src/server").unwrap();
    assert_eq!(result, "/home/user/my-project/src/server");
}

#[test]
fn resolve_terminal_path_rejects_parent_escape() {
    let project_root = Path::new("/home/user/my-project");
    assert!(resolve_terminal_path(project_root, "../../outside").is_err());
}

#[test]
fn resolve_terminal_path_rejects_absolute() {
    let project_root = Path::new("/home/user/my-project");
    assert!(resolve_terminal_path(project_root, "/etc/passwd").is_err());
}

#[test]
fn resolve_terminal_path_rejects_embedded_parent() {
    let project_root = Path::new("/home/user/my-project");
    assert!(resolve_terminal_path(project_root, "src/../../escape").is_err());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test launch_test`
Expected: FAIL — compile error (the `.unwrap()` calls reject the current `String` return type, and rejection tests call `.is_err()` on a `String`).

- [ ] **Step 3: Change `resolve_terminal_path` to validate and return `Result`**

In `src/launch.rs`, replace the whole function:

```rust
pub fn resolve_terminal_path(project_root: &Path, relative_path: &str) -> String {
    let joined = project_root.join(relative_path);
    // Normalize away `.` and `..` components without hitting the filesystem
    let mut components: Vec<std::ffi::OsString> = Vec::new();
    for component in joined.components() {
        use std::path::Component;
        match component {
            Component::CurDir => {} // skip `.`
            Component::ParentDir => {
                components.pop();
            } // resolve `..`
            c => components.push(c.as_os_str().to_os_string()),
        }
    }
    let normalized: std::path::PathBuf = components.iter().collect();
    normalized.to_string_lossy().to_string()
}
```

with:

```rust
pub fn resolve_terminal_path(project_root: &Path, relative_path: &str) -> Result<String, String> {
    use std::path::Component;

    let rel = Path::new(relative_path);
    if rel.is_absolute() || rel.components().any(|c| c == Component::ParentDir) {
        return Err("terminal path must stay inside the project root".to_string());
    }

    let joined = project_root.join(relative_path);
    // Normalize away `.` components without hitting the filesystem.
    let mut components: Vec<std::ffi::OsString> = Vec::new();
    for component in joined.components() {
        match component {
            Component::CurDir => {} // skip `.`
            Component::ParentDir => {
                components.pop();
            }
            c => components.push(c.as_os_str().to_os_string()),
        }
    }
    let normalized: std::path::PathBuf = components.iter().collect();
    Ok(normalized.to_string_lossy().to_string())
}
```

- [ ] **Step 4: Update the caller in `launch_project`**

In `src/launch.rs`, in `launch_project`, replace:

```rust
        let resolved_path = resolve_terminal_path(project_root, &terminal.path);
        let effective_emulator = terminal.emulator.as_deref().or(global_emulator);
        let result = launch_terminal(
            &resolved_path,
            terminal.command.as_deref(),
            i,
            effective_emulator,
        );
        results.push(LaunchResult {
            label: terminal.name.clone(),
            kind: "terminal",
            success: result.is_ok(),
            error: result.err(),
        });
```

with:

```rust
        let resolved_path = match resolve_terminal_path(project_root, &terminal.path) {
            Ok(path) => path,
            Err(e) => {
                results.push(LaunchResult {
                    label: terminal.name.clone(),
                    kind: "terminal",
                    success: false,
                    error: Some(e),
                });
                continue;
            }
        };
        let effective_emulator = terminal.emulator.as_deref().or(global_emulator);
        let result = launch_terminal(
            &resolved_path,
            terminal.command.as_deref(),
            i,
            effective_emulator,
        );
        results.push(LaunchResult {
            label: terminal.name.clone(),
            kind: "terminal",
            success: result.is_ok(),
            error: result.err(),
        });
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo build && cargo test --test launch_test`
Expected: compiles; all launch tests PASS (5 `resolve_terminal_path` tests + escaping tests).

- [ ] **Step 6: Commit**

```bash
git add src/launch.rs tests/launch_test.rs
git commit -m "Reject terminal paths that escape the project root"
```

---

## Task 7: Named launch shows the picker (#2)

**Files:**
- Modify: `src/main.rs` (`cmd_launch`, the selection gate ~line 264)
- Modify: `README.md`

- [ ] **Step 1: Change the selection gate**

In `src/main.rs`, in `cmd_launch`, replace:

```rust
    let config = if !all && project.is_none() {
```

with:

```rust
    let config = if !all {
```

(The branch already handles `items.len() <= 1` by launching everything, so single-item projects still launch directly.)

- [ ] **Step 2: Build to verify**

Run: `cargo build`
Expected: compiles with no errors. Note `project` is still used earlier in the function (the `match project` block), so no unused-variable warning.

- [ ] **Step 3: Update the README**

In `README.md`, find the description of `quickdev launch <name>` / `launch my-api` and update it so it states: named launch opens the same interactive multi-select picker as bare `quickdev launch`; only `quickdev launch --all` (or `quickdev launch <name> --all`) launches every item without prompting. If the README claims named launch launches everything, correct that sentence.

- [ ] **Step 4: Commit**

```bash
git add src/main.rs README.md
git commit -m "Show launch picker for named projects too"
```

---

## Task 8: Full verification pass

**Files:** none (verification only)

- [ ] **Step 1: Format**

Run: `cargo fmt`
Then: `cargo fmt --check`
Expected: no diff (clean).

- [ ] **Step 2: Lint**

Run: `cargo clippy --all-targets`
Expected: no warnings. If clippy flags the `#[allow(dead_code)]` helpers or anything else, fix before continuing.

- [ ] **Step 3: Full test suite**

Run: `cargo test`
Expected: all tests pass — the original 34 plus the new `parse_test` (3), `config_test` additions (4), and `launch_test` additions (6 net new).

- [ ] **Step 4: Commit any formatting fixups**

```bash
git add -A
git commit -m "cargo fmt" || echo "nothing to commit"
```

---

## Self-Review Notes

- **Spec coverage:** #1 → Task 3; #2 → Task 7; #3 → Task 4; #4 → Task 2 (Step 2); #5 → Task 2 (Step 3); #6 → Task 5; #7 → Task 6. Dependency (`shell-words`) → Task 1. Testing section → covered across Tasks 1, 3, 4, 5, 6 plus Task 8 final pass.
- **Type consistency:** `parse_shell_args` and `parse_project_selection` signatures match between definition and call sites; `resolve_terminal_path` returns `Result<String, String>` consistently in the function, the caller, and all tests.
- **Known non-unit-tested glue:** #3 writeback and #4/#5 wiring live in `main.rs` (binary, not reachable from `tests/`); they are covered by the underlying library-primitive tests plus the manual verification step in Task 2 Step 5. This is intentional, not a gap.
