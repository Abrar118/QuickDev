# CLI UX Improvements Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add fzf-based interactive selection, better help text, TOML comments, and project picker fallback to the QuickDev CLI.

**Architecture:** New `fzf.rs` module encapsulates all fzf interaction. `config.rs` gains `resolve_project_config()` as a shared fallback helper. `main.rs` gets richer clap annotations and refactored `remove` command. `save_project_config` in `config.rs` gains comment header preservation.

**Tech Stack:** Rust, clap (derive), fzf (external binary via stdin/stdout pipe)

---

## File Map

| File | Responsibility |
|------|---------------|
| `src/fzf.rs` (new) | fzf availability check, single-select, multi-select, install hint |
| `src/config.rs` (modify) | Add `resolve_project_config()`, update `save_project_config()` for TOML comment headers |
| `src/main.rs` (modify) | Richer help text, refactored remove with interactive mode, use `resolve_project_config` everywhere |
| `src/lib.rs` (modify) | Export `fzf` module |
| `tests/fzf_test.rs` (new) | Tests for fzf module (check_fzf, install_hint, format helpers) |
| `tests/config_test.rs` (modify) | Tests for comment header preservation |

---

### Task 1: fzf Module — Core Integration

Create `src/fzf.rs` with fzf availability checking, piping, and install hints.

**Files:**
- Create: `src/fzf.rs`
- Modify: `src/lib.rs`
- Modify: `src/main.rs` (add `mod fzf;`)
- Create: `tests/fzf_test.rs`

- [ ] **Step 1: Write the fzf tests**

Create `tests/fzf_test.rs`:

```rust
use quickdev::fzf::{check_fzf, fzf_install_hint};

#[test]
fn check_fzf_returns_bool() {
    // Just verify it doesn't panic — actual result depends on system
    let _available = check_fzf();
}

#[test]
fn install_hint_contains_instructions() {
    let hint = fzf_install_hint();
    // Should mention at least one package manager
    assert!(
        hint.contains("brew") || hint.contains("apt") || hint.contains("choco"),
        "install hint should mention a package manager, got: {hint}"
    );
}

#[test]
fn install_hint_mentions_fzf() {
    let hint = fzf_install_hint();
    assert!(hint.contains("fzf"), "install hint should mention fzf");
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test --test fzf_test
```

Expected: compilation error — `quickdev::fzf` doesn't exist.

- [ ] **Step 3: Write `src/fzf.rs`**

```rust
use crate::adapters::command_exists;
use std::io::Write;
use std::process::{Command, Stdio};

pub fn check_fzf() -> bool {
    command_exists("fzf")
}

pub fn fzf_install_hint() -> String {
    let os_hint = if cfg!(target_os = "macos") {
        "brew install fzf"
    } else if cfg!(target_os = "windows") {
        "choco install fzf"
    } else {
        "apt install fzf"
    };
    format!("fzf is required for interactive selection.\nInstall: {os_hint}")
}

pub fn fzf_select_one(items: &[String], header: &str) -> Result<String, String> {
    if !check_fzf() {
        return Err(fzf_install_hint());
    }
    if items.is_empty() {
        return Err("no items to select from".to_string());
    }

    let input = items.join("\n");

    let mut child = Command::new("fzf")
        .args(["--header", header, "--height", "~50%", "--reverse"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| format!("failed to start fzf: {e}"))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(input.as_bytes())
            .map_err(|e| format!("failed to write to fzf: {e}"))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|e| format!("fzf failed: {e}"))?;

    if !output.status.success() {
        return Err("selection cancelled".to_string());
    }

    let selected = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if selected.is_empty() {
        return Err("no selection made".to_string());
    }

    Ok(selected)
}

pub fn fzf_select_multi(items: &[String], header: &str) -> Result<Vec<String>, String> {
    if !check_fzf() {
        return Err(fzf_install_hint());
    }
    if items.is_empty() {
        return Err("no items to select from".to_string());
    }

    let input = items.join("\n");

    let mut child = Command::new("fzf")
        .args([
            "--multi", "--header", header, "--height", "~50%", "--reverse",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| format!("failed to start fzf: {e}"))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(input.as_bytes())
            .map_err(|e| format!("failed to write to fzf: {e}"))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|e| format!("fzf failed: {e}"))?;

    if !output.status.success() {
        return Err("selection cancelled".to_string());
    }

    let selected: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();

    if selected.is_empty() {
        return Err("no selection made".to_string());
    }

    Ok(selected)
}
```

- [ ] **Step 4: Update `src/lib.rs`**

```rust
pub mod adapters;
pub mod config;
pub mod fzf;
pub mod launch;
pub mod models;
```

- [ ] **Step 5: Add `mod fzf;` to `src/main.rs`**

Add `mod fzf;` to the module declarations at the top of `src/main.rs`, after `mod config;`:

```rust
mod adapters;
mod config;
mod fzf;
mod launch;
mod models;
```

- [ ] **Step 6: Run tests and verify they pass**

```bash
cargo test --test fzf_test
```

Expected: all 3 tests pass.

- [ ] **Step 7: Commit**

```bash
git add src/fzf.rs src/lib.rs src/main.rs tests/fzf_test.rs
git commit -m "Add fzf integration module"
```

---

### Task 2: TOML Comment Headers

Update `save_project_config` to prepend/preserve instructional comments in `.quickdev.toml`.

**Files:**
- Modify: `src/config.rs`
- Modify: `tests/config_test.rs`

- [ ] **Step 1: Add comment header tests to `tests/config_test.rs`**

Append these tests to the existing file:

```rust
#[test]
fn save_project_config_adds_comment_header() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join(".quickdev.toml");

    let config = ProjectConfig {
        project: ProjectEntry {
            name: "test-proj".to_string(),
        },
        terminals: vec![],
        applications: vec![],
    };

    save_project_config(&config_path, &config).unwrap();
    let content = std::fs::read_to_string(&config_path).unwrap();

    assert!(
        content.starts_with("# QuickDev project configuration"),
        "should start with comment header, got:\n{content}"
    );
    assert!(content.contains("[project]"));
    assert!(content.contains("name = \"test-proj\""));
}

#[test]
fn save_project_config_preserves_existing_header() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join(".quickdev.toml");

    let config = ProjectConfig {
        project: ProjectEntry {
            name: "test-proj".to_string(),
        },
        terminals: vec![],
        applications: vec![],
    };

    // Save once (creates header)
    save_project_config(&config_path, &config).unwrap();

    // Save again with a terminal added
    let config2 = ProjectConfig {
        project: ProjectEntry {
            name: "test-proj".to_string(),
        },
        terminals: vec![TerminalEntry {
            name: "dev".to_string(),
            path: ".".to_string(),
            command: None,
        }],
        applications: vec![],
    };
    save_project_config(&config_path, &config2).unwrap();

    let content = std::fs::read_to_string(&config_path).unwrap();

    // Header still present
    assert!(
        content.starts_with("# QuickDev project configuration"),
        "header should be preserved after re-save"
    );
    // New content present
    assert!(content.contains("[[terminals]]"));
    assert!(content.contains("name = \"dev\""));
    // Header appears exactly once
    assert_eq!(
        content.matches("# QuickDev project configuration").count(),
        1,
        "header should not be duplicated"
    );
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test --test config_test save_project_config_adds_comment_header
```

Expected: FAIL — current `save_project_config` doesn't add comments.

- [ ] **Step 3: Update `save_project_config` in `src/config.rs`**

Replace the existing `save_project_config` function with:

```rust
const TOML_COMMENT_HEADER: &str = "\
# QuickDev project configuration
# Edit this file directly or use: quickdev add, quickdev remove
#
# [project]
#   name = Display name for this project
#
# [[terminals]]
#   name    = Label for this terminal tab
#   path    = Working directory relative to project root (e.g., \".\", \"./src\")
#   command = (optional) Startup command to run when the terminal opens
#
# [[applications]]
#   name = Application display name
#   path = Executable path or .app bundle (e.g., \"/Applications/Cursor.app\")
#   args = (optional) Arguments list (e.g., [\".\"] to open project root)
";

pub fn save_project_config(path: &Path, config: &ProjectConfig) -> Result<(), String> {
    let serialized = toml::to_string_pretty(config)
        .map_err(|e| format!("failed to serialize project config: {e}"))?;

    let content = format!("{TOML_COMMENT_HEADER}\n{serialized}");
    fs::write(path, content).map_err(|e| format!("failed to write project config: {e}"))
}
```

Also update `load_project_config` to handle comment lines (TOML already ignores `#` lines, so no change needed there — `toml::from_str` natively skips comments).

- [ ] **Step 4: Run tests and verify they pass**

```bash
cargo test --test config_test
```

Expected: all 8 tests pass (6 existing + 2 new).

- [ ] **Step 5: Commit**

```bash
git add src/config.rs tests/config_test.rs
git commit -m "Add instructional comment header to generated TOML files"
```

---

### Task 3: Project Picker Fallback — `resolve_project_config`

Add `resolve_project_config()` to `config.rs` that tries local discovery first, then falls back to fzf project selection.

**Files:**
- Modify: `src/config.rs`
- Modify: `tests/config_test.rs`

- [ ] **Step 1: Add test for resolve fallback (no local config, fzf unavailable)**

Append to `tests/config_test.rs`:

```rust
use quickdev::config::resolve_project_config;

#[test]
fn resolve_project_config_finds_local() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    let config = ProjectConfig {
        project: ProjectEntry {
            name: "local-proj".to_string(),
        },
        terminals: vec![],
        applications: vec![],
    };
    save_project_config(&root.join(".quickdev.toml"), &config).unwrap();

    let result = resolve_project_config(root);
    assert!(result.is_ok());
    let (config_path, project_root) = result.unwrap();
    assert_eq!(config_path, root.join(".quickdev.toml"));
    assert_eq!(project_root, root.to_path_buf());
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cargo test --test config_test resolve_project_config_finds_local
```

Expected: compilation error — `resolve_project_config` doesn't exist yet.

- [ ] **Step 3: Add `resolve_project_config` to `src/config.rs`**

Add this function and the necessary import at the top:

```rust
use crate::fzf;
```

Then the function:

```rust
pub fn resolve_project_config(start: &Path) -> Result<(PathBuf, PathBuf), String> {
    match find_project_config(start) {
        Ok(result) => Ok(result),
        Err(_) => fzf_select_project(),
    }
}

fn fzf_select_project() -> Result<(PathBuf, PathBuf), String> {
    let global_path = global_config_path();
    let global = load_global_config(&global_path)?;

    if global.projects.is_empty() {
        return Err("No projects registered. Run 'quickdev init' in a project directory.".to_string());
    }

    if !fzf::check_fzf() {
        return Err(
            "no .quickdev.toml found in current or parent directories.\nTip: install fzf for interactive project selection"
                .to_string(),
        );
    }

    let items: Vec<String> = global
        .projects
        .iter()
        .map(|p| format!("{:<20} {}", p.name, p.path))
        .collect();

    let selected = fzf::fzf_select_one(&items, "Select a project:")?;

    let project_name = selected.split_whitespace().next().ok_or("invalid selection")?;

    let entry = global
        .projects
        .iter()
        .find(|p| p.name == project_name)
        .ok_or_else(|| format!("project '{}' not found", project_name))?;

    let root = PathBuf::from(&entry.path);
    let config_path = root.join(".quickdev.toml");

    if !config_path.exists() {
        return Err(format!(
            ".quickdev.toml not found at {}",
            config_path.display()
        ));
    }

    Ok((config_path, root))
}
```

- [ ] **Step 4: Run tests and verify they pass**

```bash
cargo test --test config_test
```

Expected: all 9 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/config.rs tests/config_test.rs
git commit -m "Add resolve_project_config with fzf project picker fallback"
```

---

### Task 4: Refactor main.rs — Use `resolve_project_config` Everywhere

Replace all `find_project_config` calls in `main.rs` with `resolve_project_config` so all commands get the fzf fallback.

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Update imports in `src/main.rs`**

Change the config import line from:

```rust
use config::{
    find_project_config, global_config_path, load_global_config, load_project_config,
    save_global_config, save_project_config, unique_project_name,
};
```

to:

```rust
use config::{
    global_config_path, load_global_config, load_project_config, resolve_project_config,
    save_global_config, save_project_config, unique_project_name,
};
```

- [ ] **Step 2: Update `cmd_launch` — replace `find_project_config` in the `None` branch**

Change the `None` branch in `cmd_launch` from:

```rust
        None => {
            let cwd = std::env::current_dir()
                .map_err(|e| format!("cannot read current directory: {e}"))?;
            let (config_path, root) = find_project_config(&cwd)?;
            let config = load_project_config(&config_path)?;
            (config, root)
        }
```

to:

```rust
        None => {
            let cwd = std::env::current_dir()
                .map_err(|e| format!("cannot read current directory: {e}"))?;
            let (config_path, root) = resolve_project_config(&cwd)?;
            let config = load_project_config(&config_path)?;
            (config, root)
        }
```

- [ ] **Step 3: Update `cmd_add` — replace `find_project_config`**

Change:

```rust
fn cmd_add(kind: AddKind) -> Result<(), String> {
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    let (config_path, _root) = find_project_config(&cwd)?;
```

to:

```rust
fn cmd_add(kind: AddKind) -> Result<(), String> {
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    let (config_path, _root) = resolve_project_config(&cwd)?;
```

- [ ] **Step 4: Update `cmd_remove` — replace `find_project_config`**

Change:

```rust
fn cmd_remove(kind: RemoveKind) -> Result<(), String> {
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    let (config_path, _root) = find_project_config(&cwd)?;
```

to:

```rust
fn cmd_remove(kind: Option<RemoveKind>) -> Result<(), String> {
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    let (config_path, _root) = resolve_project_config(&cwd)?;
```

(Note: the `Option<RemoveKind>` change comes in Task 5 — for now just change `find_project_config` to `resolve_project_config`.)

- [ ] **Step 5: Update `cmd_edit` — replace `find_project_config`**

Change:

```rust
fn cmd_edit() -> Result<(), String> {
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    let (config_path, _root) = find_project_config(&cwd)?;
```

to:

```rust
fn cmd_edit() -> Result<(), String> {
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    let (config_path, _root) = resolve_project_config(&cwd)?;
```

- [ ] **Step 6: Verify it compiles and all tests pass**

```bash
cargo build && cargo test
```

Expected: compiles, all tests pass.

- [ ] **Step 7: Commit**

```bash
git add src/main.rs
git commit -m "Use resolve_project_config for fzf fallback in all commands"
```

---

### Task 5: Interactive Remove with fzf

Refactor the `remove` command to support both interactive (no args) and direct (with subcommand) modes.

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Change `RemoveKind` to optional in `Commands` enum**

Replace:

```rust
    /// Remove a terminal or application entry
    Remove {
        #[command(subcommand)]
        kind: RemoveKind,
    },
```

with:

```rust
    /// Remove terminals/apps (interactive picker, or specify: remove terminal <name>)
    Remove {
        #[command(subcommand)]
        kind: Option<RemoveKind>,
    },
```

- [ ] **Step 2: Update the `main` match arm**

The match arm already passes `kind` — no change needed since `kind` is now `Option<RemoveKind>`.

- [ ] **Step 3: Rewrite `cmd_remove` to handle both modes**

Replace the entire `cmd_remove` function with:

```rust
fn cmd_remove(kind: Option<RemoveKind>) -> Result<(), String> {
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    let (config_path, _root) = resolve_project_config(&cwd)?;
    let mut config = load_project_config(&config_path)?;

    match kind {
        Some(RemoveKind::Terminal { name }) => {
            let before = config.terminals.len();
            config.terminals.retain(|t| t.name != name);
            if config.terminals.len() == before {
                return Err(format!("terminal '{}' not found", name));
            }
            println!("Removed terminal '{}'", name);
        }
        Some(RemoveKind::App { name }) => {
            let before = config.applications.len();
            config.applications.retain(|a| a.name != name);
            if config.applications.len() == before {
                return Err(format!("application '{}' not found", name));
            }
            println!("Removed application '{}'", name);
        }
        None => {
            return cmd_remove_interactive(config_path, config);
        }
    }

    save_project_config(&config_path, &config)
}

fn cmd_remove_interactive(
    config_path: PathBuf,
    mut config: ProjectConfig,
) -> Result<(), String> {
    let mut items: Vec<String> = Vec::new();

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

    if items.is_empty() {
        return Err("no terminals or applications configured".to_string());
    }

    let selected = fzf::fzf_select_multi(&items, "Select items to remove (TAB to toggle, ENTER to confirm):")?;

    let mut removed_terminals = Vec::new();
    let mut removed_apps = Vec::new();

    for line in &selected {
        if line.starts_with("[terminal] ") {
            let name = line
                .strip_prefix("[terminal] ")
                .and_then(|s| s.split(" — ").next())
                .unwrap_or("")
                .to_string();
            removed_terminals.push(name);
        } else if line.starts_with("[app] ") {
            let name = line
                .strip_prefix("[app] ")
                .and_then(|s| s.split(" — ").next())
                .unwrap_or("")
                .to_string();
            removed_apps.push(name);
        }
    }

    config
        .terminals
        .retain(|t| !removed_terminals.contains(&t.name));
    config
        .applications
        .retain(|a| !removed_apps.contains(&a.name));

    save_project_config(&config_path, &config)?;

    for name in &removed_terminals {
        println!("Removed terminal '{name}'");
    }
    for name in &removed_apps {
        println!("Removed application '{name}'");
    }

    Ok(())
}
```

- [ ] **Step 4: Add the fzf import to main.rs**

Make sure this import is present at the top of the use block (should already be there from the `mod fzf;` in Task 1, but verify `fzf` is accessible). The `cmd_remove_interactive` function uses `fzf::fzf_select_multi`, which works via the `mod fzf;` declaration.

- [ ] **Step 5: Verify it compiles**

```bash
cargo build
```

Expected: compiles with no errors.

- [ ] **Step 6: Smoke test interactive remove**

```bash
cargo run -- remove
```

Expected: fzf opens showing the configured terminals/apps. Select items and confirm removal.

- [ ] **Step 7: Smoke test direct remove still works**

```bash
cargo run -- remove terminal second
```

(Or whatever terminal name exists.) Expected: removes by name without fzf.

- [ ] **Step 8: Commit**

```bash
git add src/main.rs
git commit -m "Add interactive fzf-based remove with multi-select"
```

---

### Task 6: Improved Help Text

Add `after_help` examples to clap annotations for richer help output.

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Add `after_help` to top-level `Cli` struct**

Replace:

```rust
#[derive(Parser)]
#[command(
    name = "quickdev",
    about = "Manage and launch project terminal/app configurations"
)]
struct Cli {
```

with:

```rust
#[derive(Parser)]
#[command(
    name = "quickdev",
    about = "Manage and launch project terminal/app configurations",
    after_help = "\
Examples:
  quickdev init                                         Initialize a project
  quickdev launch                                       Launch current project
  quickdev launch my-api                                Launch a project by name
  quickdev add terminal server . --command \"npm start\"  Add a terminal tab
  quickdev add app Cursor /Applications/Cursor.app      Add an application
  quickdev remove                                       Interactive removal picker"
)]
struct Cli {
```

- [ ] **Step 2: Add `after_help` to `AddKind::Terminal`**

Replace:

```rust
    /// Add a terminal entry
    Terminal {
```

with:

```rust
    /// Add a terminal entry
    #[command(after_help = "\
Examples:
  quickdev add terminal server .                        Open shell in project root
  quickdev add terminal dev . --command \"npm run dev\"   Run a command on open
  quickdev add terminal logs ./logs                     Open shell in subdirectory")]
    Terminal {
```

- [ ] **Step 3: Add `after_help` to `AddKind::App`**

Replace:

```rust
    /// Add an application entry
    App {
```

with:

```rust
    /// Add an application entry
    #[command(after_help = "\
Examples:
  quickdev add app Cursor /Applications/Cursor.app --args \".\"
  quickdev add app Firefox /usr/bin/firefox")]
    App {
```

- [ ] **Step 4: Verify help output**

```bash
cargo run -- --help
cargo run -- add terminal --help
cargo run -- add app --help
```

Expected: examples appear at the bottom of each help output.

- [ ] **Step 5: Commit**

```bash
git add src/main.rs
git commit -m "Add examples to CLI help text"
```

---

### Task 7: Final Cleanup and Verification

Format, lint, run full test suite.

- [ ] **Step 1: Format**

```bash
cargo fmt
```

- [ ] **Step 2: Lint**

```bash
cargo clippy -- -W clippy::all
```

Fix any warnings.

- [ ] **Step 3: Full test suite**

```bash
cargo test
```

Expected: all tests pass.

- [ ] **Step 4: End-to-end smoke test**

```bash
cargo run -- --help
cargo run -- list
cargo run -- remove
cargo run -- launch
```

Verify: help shows examples, list works, remove opens fzf picker, launch opens terminals.

- [ ] **Step 5: Commit any fixes**

```bash
git add -A
git commit -m "Apply cargo fmt and clippy fixes"
```

(Skip if no changes.)
