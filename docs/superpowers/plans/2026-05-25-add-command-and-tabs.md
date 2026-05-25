# Interactive Add & Terminal Tab Launch — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `quickdev add` interactive with fzf type selection and macOS app discovery, and fix terminal launching to use tabs where supported with inter-spawn delays.

**Architecture:** New `apps.rs` module for macOS app discovery. `launch.rs` refactored to pass `tab_index` through terminal launch functions for tab-aware spawning. `main.rs` gets optional `AddKind` with interactive `cmd_add_interactive()`. Stdin prompts for free-text input.

**Tech Stack:** Rust, clap, fzf, macOS `/Applications` scanning, platform-specific terminal tab flags

---

## File Map

| File | Responsibility |
|------|---------------|
| `src/apps.rs` (new) | macOS app discovery — scan `/Applications` and `~/Applications` |
| `src/launch.rs` (modify) | Add `tab_index` to terminal launching, inter-spawn delay, platform tab support |
| `src/main.rs` (modify) | Optional `AddKind`, interactive add flow, `prompt()` helper |
| `src/lib.rs` (modify) | Export `pub mod apps;` |
| `tests/apps_test.rs` (new) | Tests for app discovery |
| `tests/launch_test.rs` (modify) | Tests for `resolve_terminal_path` (existing, no changes needed) |

---

### Task 1: App Discovery Module

Create `src/apps.rs` for scanning installed macOS applications.

**Files:**
- Create: `src/apps.rs`
- Modify: `src/lib.rs`
- Modify: `src/main.rs` (add `mod apps;`)
- Create: `tests/apps_test.rs`

- [ ] **Step 1: Write tests**

Create `tests/apps_test.rs`:

```rust
use quickdev::apps::discover_apps;

#[test]
fn discover_apps_returns_vec() {
    let apps = discover_apps();
    // On macOS CI/dev machines, /Applications should have at least a few apps
    // On non-macOS, returns empty vec
    if cfg!(target_os = "macos") {
        assert!(!apps.is_empty(), "macOS should find at least one app");
    } else {
        assert!(apps.is_empty(), "non-macOS should return empty vec");
    }
}

#[test]
fn discover_apps_entries_have_name_and_path() {
    let apps = discover_apps();
    for (name, path) in &apps {
        assert!(!name.is_empty(), "app name should not be empty");
        assert!(!path.is_empty(), "app path should not be empty");
        if cfg!(target_os = "macos") {
            assert!(path.ends_with(".app"), "path should end with .app: {path}");
        }
    }
}

#[test]
fn discover_apps_sorted_alphabetically() {
    let apps = discover_apps();
    if apps.len() >= 2 {
        for window in apps.windows(2) {
            assert!(
                window[0].0.to_lowercase() <= window[1].0.to_lowercase(),
                "apps should be sorted: '{}' should come before '{}'",
                window[0].0,
                window[1].0
            );
        }
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test --test apps_test
```

Expected: compilation error — `quickdev::apps` doesn't exist.

- [ ] **Step 3: Write `src/apps.rs`**

```rust
pub fn discover_apps() -> Vec<(String, String)> {
    #[cfg(target_os = "macos")]
    {
        let mut apps = Vec::new();

        let dirs_to_scan = vec![
            "/Applications".to_string(),
            dirs::home_dir()
                .map(|h| format!("{}/Applications", h.display()))
                .unwrap_or_default(),
        ];

        for dir in dirs_to_scan {
            if dir.is_empty() {
                continue;
            }
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let file_name = entry.file_name().to_string_lossy().to_string();
                    if file_name.starts_with('.') {
                        continue;
                    }
                    if file_name.ends_with(".app") {
                        let name = file_name.strip_suffix(".app").unwrap_or(&file_name).to_string();
                        apps.push((name, path.to_string_lossy().to_string()));
                    }
                }
            }
        }

        apps.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
        apps.dedup_by(|a, b| a.0 == b.0);
        apps
    }

    #[cfg(not(target_os = "macos"))]
    {
        Vec::new()
    }
}
```

- [ ] **Step 4: Update `src/lib.rs`**

```rust
pub mod adapters;
pub mod apps;
pub mod config;
pub mod fzf;
pub mod launch;
pub mod models;
```

- [ ] **Step 5: Add `mod apps;` to `src/main.rs`**

Change the module declarations at the top to:

```rust
mod adapters;
mod apps;
mod config;
mod fzf;
mod launch;
mod models;
```

- [ ] **Step 6: Run tests and verify they pass**

```bash
cargo test --test apps_test
```

Expected: all 3 tests pass.

- [ ] **Step 7: Commit**

```bash
git add src/apps.rs src/lib.rs src/main.rs tests/apps_test.rs
git commit -m "Add macOS app discovery module"
```

---

### Task 2: Terminal Tab-Aware Launch

Refactor `launch.rs` to support tabs where the platform allows, with inter-spawn delays.

**Files:**
- Modify: `src/launch.rs`

- [ ] **Step 1: Add `tab_index` parameter to `launch_terminal`**

Change the `launch_terminal` function signature from:

```rust
fn launch_terminal(resolved_path: &str, command: Option<&str>) -> Result<(), String> {
```

to:

```rust
fn launch_terminal(resolved_path: &str, command: Option<&str>, tab_index: usize) -> Result<(), String> {
```

And pass `tab_index` through to `try_ghostty` and `run_in_platform_terminal`:

```rust
fn launch_terminal(resolved_path: &str, command: Option<&str>, tab_index: usize) -> Result<(), String> {
    if !Path::new(resolved_path).exists() {
        return Err(format!("path not found: {resolved_path}"));
    }

    if try_ghostty(resolved_path, command).is_ok() {
        return Ok(());
    }

    run_in_platform_terminal(resolved_path, command, tab_index)
}
```

Note: `try_ghostty` does NOT get `tab_index` because Ghostty doesn't support tabs from CLI.

- [ ] **Step 2: Update `run_in_platform_terminal` signature and macOS Terminal.app tab logic**

Change the signature to accept `tab_index`:

```rust
fn run_in_platform_terminal(cwd: &str, command: Option<&str>, tab_index: usize) -> Result<(), String> {
```

Replace the macOS `#[cfg(target_os = "macos")]` block with tab-aware AppleScript:

```rust
    #[cfg(target_os = "macos")]
    {
        let cd_part = format!("cd '{}'", cwd.replace('\'', "'\\''"));
        let full = if cmd_str.is_empty() {
            cd_part
        } else {
            format!("{cd_part} && {cmd_str}")
        };
        let escaped = full.replace('"', "\\\"");
        let script = if tab_index == 0 {
            format!(
                "tell application \"Terminal\"\n    activate\n    do script \"{escaped}\"\nend tell"
            )
        } else {
            format!(
                "tell application \"Terminal\"\n    activate\n    do script \"{escaped}\" in front window\nend tell"
            )
        };
        Command::new("osascript")
            .args(["-e", &script])
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("Terminal.app launch failed: {e}"))
    }
```

- [ ] **Step 3: Update Windows Terminal tab logic**

Replace the Windows `#[cfg(target_os = "windows")]` block with tab-aware launching:

```rust
    #[cfg(target_os = "windows")]
    {
        if command_exists("wt") {
            let wt_resolved = resolve_command("wt").unwrap_or_else(|| "wt".to_string());
            if tab_index == 0 {
                let mut cmd = Command::new(&wt_resolved);
                cmd.args(["-d", cwd]);
                if !cmd_str.is_empty() {
                    cmd.args(["cmd", "/K", cmd_str]);
                }
                return cmd
                    .spawn()
                    .map(|_| ())
                    .map_err(|e| format!("wt launch failed: {e}"));
            } else {
                let mut cmd = Command::new(&wt_resolved);
                cmd.args(["-w", "0", "new-tab", "-d", cwd]);
                if !cmd_str.is_empty() {
                    cmd.args(["cmd", "/K", cmd_str]);
                }
                return cmd
                    .spawn()
                    .map(|_| ())
                    .map_err(|e| format!("wt launch failed: {e}"));
            }
        }

        if command_exists("pwsh") {
            let ps_cmd = if cmd_str.is_empty() {
                format!("Set-Location '{cwd}'")
            } else {
                format!("Set-Location '{cwd}'; {cmd_str}")
            };
            return Command::new(resolve_command("pwsh").unwrap_or_else(|| "pwsh".to_string()))
                .args(["-NoExit", "-Command", &ps_cmd])
                .spawn()
                .map(|_| ())
                .map_err(|e| format!("pwsh launch failed: {e}"));
        }

        let full = if cmd_str.is_empty() {
            format!("cd /d \"{cwd}\"")
        } else {
            format!("cd /d \"{cwd}\" && {cmd_str}")
        };
        Command::new("cmd")
            .args(["/C", "start", "cmd", "/K", &full])
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("cmd launch failed: {e}"))
    }
```

- [ ] **Step 4: Update Linux gnome-terminal tab logic**

Replace the Linux `#[cfg(all(not(...), not(...)))]` block with tab-aware launching:

```rust
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        let shell_command = if cmd_str.is_empty() {
            format!("cd '{}' && exec $SHELL", cwd.replace('\'', "'\\''"))
        } else {
            format!("cd '{}' && {}", cwd.replace('\'', "'\\''"), cmd_str)
        };

        if command_exists("gnome-terminal") {
            let resolved =
                resolve_command("gnome-terminal").unwrap_or_else(|| "gnome-terminal".to_string());
            let mut cmd = Command::new(resolved);
            if tab_index > 0 {
                cmd.arg("--tab");
            }
            cmd.args(["--", "sh", "-lc", &shell_command]);
            if cmd.spawn().is_ok() {
                return Ok(());
            }
        }

        let candidates: &[(&str, &[&str])] = &[
            ("konsole", &["-e"]),
            ("alacritty", &["-e"]),
            ("xterm", &["-e"]),
        ];

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

        Err("no terminal emulator found".to_string())
    }
```

- [ ] **Step 5: Update `launch_project` to pass `tab_index` and add delay**

Add the `use std::time::Duration;` and `use std::thread;` imports at the top of the file, then replace the terminal loop in `launch_project`:

```rust
pub fn launch_project(config: &ProjectConfig, project_root: &Path) -> Vec<LaunchResult> {
    let mut results = Vec::new();

    for (i, terminal) in config.terminals.iter().enumerate() {
        if i > 0 {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        let resolved_path = resolve_terminal_path(project_root, &terminal.path);
        let result = launch_terminal(&resolved_path, terminal.command.as_deref(), i);
        results.push(LaunchResult {
            label: terminal.name.clone(),
            kind: "terminal",
            success: result.is_ok(),
            error: result.err(),
        });
    }

    for app in &config.applications {
        let resolved_args: Option<Vec<String>> =
            app.args.as_ref().map(|a| resolve_app_args(project_root, a));
        let result =
            launch_application(&app.name, &app.path, resolved_args.as_deref(), project_root);
        results.push(LaunchResult {
            label: app.name.clone(),
            kind: "app",
            success: result.is_ok(),
            error: result.err(),
        });
    }

    results
}
```

- [ ] **Step 6: Verify it compiles and existing tests pass**

```bash
cargo build && cargo test
```

Expected: compiles, all existing tests pass (launch_test tests don't call `launch_terminal` directly).

- [ ] **Step 7: Commit**

```bash
git add src/launch.rs
git commit -m "Add tab-aware terminal launching with inter-spawn delay"
```

---

### Task 3: Interactive Add Command

Make `quickdev add` (no subcommand) interactive with fzf type selection, stdin prompts for terminal fields, and fzf app picker on macOS.

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Make `AddKind` optional**

Change the `Add` variant in the `Commands` enum from:

```rust
    /// Add a terminal or application entry
    Add {
        #[command(subcommand)]
        kind: AddKind,
    },
```

to:

```rust
    /// Add a terminal or application entry (interactive if no subcommand given)
    Add {
        #[command(subcommand)]
        kind: Option<AddKind>,
    },
```

- [ ] **Step 2: Update the main match arm**

Change:

```rust
        Commands::Add { kind } => cmd_add(kind),
```

to:

```rust
        Commands::Add { kind } => cmd_add(kind),
```

(No change needed — the match arm passes `kind` which is now `Option<AddKind>`.)

- [ ] **Step 3: Add the `prompt` helper function**

Add this function somewhere before `cmd_add` in `main.rs`:

```rust
fn prompt(message: &str) -> Result<String, String> {
    eprint!("{message}");
    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .map_err(|e| format!("failed to read input: {e}"))?;
    Ok(input.trim().to_string())
}
```

- [ ] **Step 4: Rewrite `cmd_add` to handle both modes**

Replace the entire `cmd_add` function with:

```rust
fn cmd_add(kind: Option<AddKind>) -> Result<(), String> {
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    let (config_path, _root) = resolve_project_config(&cwd)?;
    let mut config = load_project_config(&config_path)?;

    match kind {
        Some(AddKind::Terminal {
            name,
            path,
            command,
        }) => {
            if config.terminals.iter().any(|t| t.name == name) {
                return Err(format!("terminal '{}' already exists", name));
            }
            config.terminals.push(TerminalEntry {
                name: name.clone(),
                path,
                command,
            });
            println!("Added terminal '{}'", name);
        }
        Some(AddKind::App { name, path, args }) => {
            if config.applications.iter().any(|a| a.name == name) {
                return Err(format!("application '{}' already exists", name));
            }
            config.applications.push(AppEntry {
                name: name.clone(),
                path,
                args,
            });
            println!("Added application '{}'", name);
        }
        None => {
            return cmd_add_interactive(config_path, config);
        }
    }

    save_project_config(&config_path, &config)
}
```

- [ ] **Step 5: Add `cmd_add_interactive` function**

Add this function after `cmd_add`:

```rust
fn cmd_add_interactive(config_path: PathBuf, mut config: ProjectConfig) -> Result<(), String> {
    let types = vec!["Terminal".to_string(), "Application".to_string()];
    let selected = fzf::fzf_select_one(&types, "Select what to add:")?;

    match selected.as_str() {
        "Terminal" => {
            let path = prompt("Path (. for current directory): ")?;
            let path = if path.is_empty() { ".".to_string() } else { path };

            let name = prompt("Name for this tab: ")?;
            if name.is_empty() {
                return Err("name cannot be empty".to_string());
            }
            if config.terminals.iter().any(|t| t.name == name) {
                return Err(format!("terminal '{}' already exists", name));
            }

            let command_input = prompt("Startup command (optional, press Enter to skip): ")?;
            let command = if command_input.is_empty() {
                None
            } else {
                Some(command_input)
            };

            config.terminals.push(TerminalEntry {
                name: name.clone(),
                path,
                command,
            });
            save_project_config(&config_path, &config)?;
            println!("Added terminal '{name}'");
        }
        "Application" => {
            let (app_name, app_path) = pick_application()?;

            if config.applications.iter().any(|a| a.name == app_name) {
                return Err(format!("application '{}' already exists", app_name));
            }

            config.applications.push(AppEntry {
                name: app_name.clone(),
                path: app_path,
                args: None,
            });
            save_project_config(&config_path, &config)?;
            println!("Added application '{app_name}'");
        }
        _ => return Err("invalid selection".to_string()),
    }

    Ok(())
}
```

- [ ] **Step 6: Add `pick_application` function**

Add this function after `cmd_add_interactive`:

```rust
fn pick_application() -> Result<(String, String), String> {
    let discovered = apps::discover_apps();

    if discovered.is_empty() {
        let app_path = prompt("Application path: ")?;
        if app_path.is_empty() {
            return Err("path cannot be empty".to_string());
        }
        let app_name = prompt("Application name: ")?;
        if app_name.is_empty() {
            return Err("name cannot be empty".to_string());
        }
        return Ok((app_name, app_path));
    }

    let mut items: Vec<String> = vec!["[Enter path manually]".to_string()];
    for (name, path) in &discovered {
        items.push(format!("{name}  ({path})"));
    }

    let selected = fzf::fzf_select_one(&items, "Select an application:")?;

    if selected == "[Enter path manually]" {
        let app_path = prompt("Application path: ")?;
        if app_path.is_empty() {
            return Err("path cannot be empty".to_string());
        }
        let app_name = prompt("Application name: ")?;
        if app_name.is_empty() {
            return Err("name cannot be empty".to_string());
        }
        return Ok((app_name, app_path));
    }

    let app_name = selected
        .split("  (")
        .next()
        .unwrap_or(&selected)
        .to_string();

    let entry = discovered
        .iter()
        .find(|(name, _)| *name == app_name)
        .ok_or_else(|| format!("app '{}' not found in discovered list", app_name))?;

    Ok((entry.0.clone(), entry.1.clone()))
}
```

- [ ] **Step 7: Verify it compiles**

```bash
cargo build
```

Expected: compiles with no errors.

- [ ] **Step 8: Run all tests**

```bash
cargo test
```

Expected: all tests pass.

- [ ] **Step 9: Smoke test interactive add**

```bash
cargo run -- add
```

Expected: fzf prompts for Terminal/Application selection, then follows the interactive flow.

- [ ] **Step 10: Smoke test direct add still works**

```bash
cargo run -- add terminal smoketest . --command "echo hello"
cargo run -- remove terminal smoketest
```

Expected: adds and removes without fzf interaction.

- [ ] **Step 11: Commit**

```bash
git add src/main.rs
git commit -m "Add interactive add command with fzf type selection and app picker"
```

---

### Task 4: Final Cleanup

Format, lint, test, smoke test everything together.

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
cargo run -- add
cargo run -- launch
cargo run -- remove
```

Verify: interactive add works, launch opens terminals (with tabs where supported), interactive remove works.

- [ ] **Step 5: Commit any fixes**

```bash
git add -A
git commit -m "Apply cargo fmt and clippy fixes"
```

(Skip if no changes.)
