# CLI Refinements Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add 6 refinements: selective launch with fzf, richer list, init templates, global default emulator, args prompt in interactive add, and project deregister.

**Architecture:** All changes in existing files. `GlobalConfig` gains `emulator` field. `launch_project` accepts global emulator as fallback. `main.rs` gets new commands (Deregister), modified commands (Launch with `--all`, Init with `--from`, Edit with `--global`), richer list formatting, and shared display builder extracted from remove. main.rs is 539 lines — acceptable for a CLI entry point, no split needed.

**Tech Stack:** Rust, clap, fzf, toml, serde

---

## File Map

| File | Changes |
|------|---------|
| `src/models.rs` | Add `emulator: Option<String>` to `GlobalConfig` |
| `src/config.rs` | Add comment header to `save_global_config` |
| `src/launch.rs` | Accept `global_emulator` param in `launch_project` and pass down |
| `src/main.rs` | All 6 features: selective launch, richer list, init --from, edit --global, deregister, args prompt, shared display builder |
| `tests/models_test.rs` | Update `GlobalConfig` round-trip test |
| `tests/config_test.rs` | Test global config comment header |

---

### Task 1: GlobalConfig Emulator Field + Global Config Comments

Add `emulator: Option<String>` to `GlobalConfig` and add a comment header to `save_global_config`.

**Files:**
- Modify: `src/models.rs`
- Modify: `src/config.rs`
- Modify: `tests/models_test.rs`
- Modify: `tests/config_test.rs`

- [ ] **Step 1: Add `emulator` field to `GlobalConfig` in `src/models.rs`**

Replace:
```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct GlobalConfig {
    #[serde(default)]
    pub projects: Vec<GlobalProjectEntry>,
}
```

with:

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct GlobalConfig {
    pub emulator: Option<String>,
    #[serde(default)]
    pub projects: Vec<GlobalProjectEntry>,
}
```

- [ ] **Step 2: Update `load_global_config` default in `src/config.rs`**

Change the empty-file fallback from:
```rust
        return Ok(GlobalConfig { projects: vec![] });
```

to:

```rust
        return Ok(GlobalConfig {
            emulator: None,
            projects: vec![],
        });
```

- [ ] **Step 3: Add comment header to `save_global_config` in `src/config.rs`**

Add a constant above `save_global_config`:

```rust
const GLOBAL_COMMENT_HEADER: &str = "\
# QuickDev global configuration
#
# emulator = (optional) Default terminal emulator: \"ghostty\", \"terminal\"
#
# Projects are auto-managed by quickdev init / deregister
";
```

Then change `save_global_config` to:

```rust
pub fn save_global_config(path: &Path, config: &GlobalConfig) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create config directory: {e}"))?;
    }
    let serialized = toml::to_string_pretty(config)
        .map_err(|e| format!("failed to serialize global config: {e}"))?;
    let content = format!("{GLOBAL_COMMENT_HEADER}\n{serialized}");
    fs::write(path, content).map_err(|e| format!("failed to write global config: {e}"))
}
```

- [ ] **Step 4: Fix `GlobalConfig` construction in `tests/models_test.rs`**

In `global_config_round_trip`, change:
```rust
    let config = GlobalConfig {
        projects: vec![
```

to:

```rust
    let config = GlobalConfig {
        emulator: None,
        projects: vec![
```

- [ ] **Step 5: Add global config comment test in `tests/config_test.rs`**

Add this test at the end of the file:

```rust
#[test]
fn save_global_config_adds_comment_header() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config.toml");

    let config = GlobalConfig {
        emulator: Some("ghostty".to_string()),
        projects: vec![],
    };

    save_global_config(&config_path, &config).unwrap();
    let content = std::fs::read_to_string(&config_path).unwrap();

    assert!(
        content.starts_with("# QuickDev global configuration"),
        "should start with comment header"
    );
    assert!(content.contains("emulator = \"ghostty\""));
}
```

- [ ] **Step 6: Fix any other `GlobalConfig` constructions in tests**

In `save_and_load_global_config` test in `tests/config_test.rs`, change:
```rust
    let config = GlobalConfig {
        projects: vec![GlobalProjectEntry {
```

to:

```rust
    let config = GlobalConfig {
        emulator: None,
        projects: vec![GlobalProjectEntry {
```

Also in `unique_project_name_appends_suffix`:
```rust
    let config = GlobalConfig {
        projects: vec![
```

to:

```rust
    let config = GlobalConfig {
        emulator: None,
        projects: vec![
```

- [ ] **Step 7: Run tests**

```bash
cargo test
```

Expected: all tests pass.

- [ ] **Step 8: Commit**

```bash
git add src/models.rs src/config.rs tests/models_test.rs tests/config_test.rs
git commit -m "Add global emulator field and comment header to global config"
```

---

### Task 2: Launch with Global Emulator Fallback

Pass the global emulator through the launch pipeline.

**Files:**
- Modify: `src/launch.rs`
- Modify: `src/main.rs` (just the `cmd_launch` function)

- [ ] **Step 1: Change `launch_project` signature to accept global emulator**

In `src/launch.rs`, change:

```rust
pub fn launch_project(config: &ProjectConfig, project_root: &Path) -> Vec<LaunchResult> {
```

to:

```rust
pub fn launch_project(
    config: &ProjectConfig,
    project_root: &Path,
    global_emulator: Option<&str>,
) -> Vec<LaunchResult> {
```

- [ ] **Step 2: Use global emulator as fallback in the terminal loop**

In the terminal loop inside `launch_project`, change:

```rust
        let result = launch_terminal(
            &resolved_path,
            terminal.command.as_deref(),
            i,
            terminal.emulator.as_deref(),
        );
```

to:

```rust
        let effective_emulator = terminal.emulator.as_deref().or(global_emulator);
        let result = launch_terminal(
            &resolved_path,
            terminal.command.as_deref(),
            i,
            effective_emulator,
        );
```

- [ ] **Step 3: Update `cmd_launch` in `src/main.rs` to pass global emulator**

Change the launch call from:

```rust
    let results = launch::launch_project(&config, &project_root);
```

to:

```rust
    let global_path = global_config_path();
    let global = load_global_config(&global_path)?;
    let results = launch::launch_project(&config, &project_root, global.emulator.as_deref());
```

Note: the `Some(name)` branch already loads `global` — reuse it there. The `None` branch needs to load it. Restructure `cmd_launch` so `global` is loaded once at the top:

```rust
fn cmd_launch(project: Option<String>, all: bool) -> Result<(), String> {
    let global_path = global_config_path();
    let global = load_global_config(&global_path)?;

    let (config, project_root) = match project {
        Some(name) => {
            let entry = global
                .projects
                .iter()
                .find(|p| p.name == name)
                .ok_or_else(|| format!("project '{}' not found in global index", name))?;
            let root = PathBuf::from(&entry.path);
            let config_path = root.join(".quickdev.toml");
            let config = load_project_config(&config_path)?;
            (config, root)
        }
        None => {
            let cwd = std::env::current_dir()
                .map_err(|e| format!("cannot read current directory: {e}"))?;
            let (config_path, root) = resolve_project_config(&cwd)?;
            let config = load_project_config(&config_path)?;
            (config, root)
        }
    };

    if config.terminals.is_empty() && config.applications.is_empty() {
        return Err("no terminals or applications configured".to_string());
    }

    let results = launch::launch_project(&config, &project_root, global.emulator.as_deref());
    print_launch_summary(&results);

    let any_success = results.iter().any(|r| r.success);
    if !any_success {
        process::exit(1);
    }

    Ok(())
}
```

(The `all` parameter will be used in Task 3.)

- [ ] **Step 4: Update Launch command variant to add `--all` flag (needed for signature)**

Change in the `Commands` enum:
```rust
    Launch {
        /// Project name from the global index (omit to use current directory)
        project: Option<String>,
    },
```

to:

```rust
    /// Launch terminals and applications for a project
    Launch {
        /// Project name from the global index (omit to use current directory)
        project: Option<String>,
        /// Launch all items without interactive selection
        #[arg(long)]
        all: bool,
    },
```

And the main match arm from:
```rust
        Commands::Launch { project } => cmd_launch(project),
```

to:
```rust
        Commands::Launch { project, all } => cmd_launch(project, all),
```

- [ ] **Step 5: Verify build and tests**

```bash
cargo build && cargo test
```

Expected: compiles, all tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/launch.rs src/main.rs
git commit -m "Pass global emulator through launch pipeline, add --all flag"
```

---

### Task 3: Selective Launch with fzf

When `quickdev launch` is run without `--all` and without a project name, show fzf picker.

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Extract shared display builder**

Add this function before `cmd_remove_interactive`:

```rust
fn build_item_display_list(config: &ProjectConfig) -> Vec<String> {
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
```

- [ ] **Step 2: Refactor `cmd_remove_interactive` to use shared builder**

Replace the item-building code in `cmd_remove_interactive`:
```rust
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
```

with:

```rust
    let items = build_item_display_list(&config);
```

- [ ] **Step 3: Add `parse_selected_items` helper**

Add this function after `build_item_display_list`:

```rust
fn parse_selected_items(selected: &[String]) -> (Vec<String>, Vec<String>) {
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

- [ ] **Step 4: Refactor `cmd_remove_interactive` to use `parse_selected_items`**

Replace the parsing block in `cmd_remove_interactive`:
```rust
    let mut removed_terminals = Vec::new();
    let mut removed_apps = Vec::new();

    for line in &selected {
        if line.starts_with("[terminal] ") {
            ...
        } else if line.starts_with("[app] ") {
            ...
        }
    }
```

with:

```rust
    let (removed_terminals, removed_apps) = parse_selected_items(&selected);
```

- [ ] **Step 5: Add selective launch logic to `cmd_launch`**

After the `if config.terminals.is_empty() && ...` check, add the fzf selection before launching. Replace the section from `if config.terminals.is_empty()` to the end of the function with:

```rust
    if config.terminals.is_empty() && config.applications.is_empty() {
        return Err("no terminals or applications configured".to_string());
    }

    let config = if !all && project.is_none() {
        let items = build_item_display_list(&config);
        if items.len() == 1 {
            config
        } else {
            let selected = fzf::fzf_select_multi(
                &items,
                "Select items to launch (TAB to toggle, ENTER to confirm):",
            )?;
            let (terminal_names, app_names) = parse_selected_items(&selected);
            ProjectConfig {
                project: config.project,
                terminals: config
                    .terminals
                    .into_iter()
                    .filter(|t| terminal_names.contains(&t.name))
                    .collect(),
                applications: config
                    .applications
                    .into_iter()
                    .filter(|a| app_names.contains(&a.name))
                    .collect(),
            }
        }
    } else {
        config
    };

    let results = launch::launch_project(&config, &project_root, global.emulator.as_deref());
    print_launch_summary(&results);

    let any_success = results.iter().any(|r| r.success);
    if !any_success {
        process::exit(1);
    }

    Ok(())
```

- [ ] **Step 6: Verify build and smoke test**

```bash
cargo build && cargo test
cargo run -- launch --all
```

Expected: `--all` launches everything, bare `launch` shows fzf picker.

- [ ] **Step 7: Commit**

```bash
git add src/main.rs
git commit -m "Add selective launch with fzf picker, extract shared display helpers"
```

---

### Task 4: Richer List Output

Replace table format with vertical layout showing terminal/app names.

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Replace `cmd_list` function**

Replace the entire `cmd_list` function with:

```rust
fn cmd_list() -> Result<(), String> {
    let global_path = global_config_path();
    let global = load_global_config(&global_path)?;

    if global.projects.is_empty() {
        println!("No projects registered. Run 'quickdev init' in a project directory.");
        return Ok(());
    }

    println!("Projects:");

    for entry in &global.projects {
        let root = PathBuf::from(&entry.path);
        let config_path = root.join(".quickdev.toml");

        if !config_path.exists() {
            println!("  {}    {}  (missing)", entry.name, entry.path);
            println!();
            continue;
        }

        let cfg = match load_project_config(&config_path) {
            Ok(cfg) => cfg,
            Err(_) => {
                println!("  {}    {}  (config error)", entry.name, entry.path);
                println!();
                continue;
            }
        };

        println!("  {}    {}", entry.name, entry.path);

        if !cfg.terminals.is_empty() {
            let names: Vec<&str> = cfg.terminals.iter().map(|t| t.name.as_str()).collect();
            println!("    Terminals: {}", names.join(", "));
        }

        if !cfg.applications.is_empty() {
            let names: Vec<&str> = cfg.applications.iter().map(|a| a.name.as_str()).collect();
            println!("    Apps: {}", names.join(", "));
        }

        println!();
    }

    Ok(())
}
```

- [ ] **Step 2: Verify and smoke test**

```bash
cargo build && cargo run -- list
```

Expected: vertical layout with terminal/app names.

- [ ] **Step 3: Commit**

```bash
git add src/main.rs
git commit -m "Replace list table format with richer vertical layout"
```

---

### Task 5: Init Templates + Smart Re-register

Add `--from <project>` flag to `init`, and smart re-register if `.quickdev.toml` exists but isn't indexed.

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Add `--from` flag to Init command**

Change:
```rust
    /// Create .quickdev.toml in the current directory and register the project
    Init,
```

to:

```rust
    /// Create .quickdev.toml in the current directory and register the project
    Init {
        /// Clone config from another project by name
        #[arg(long)]
        from: Option<String>,
    },
```

Update the main match arm from:
```rust
        Commands::Init => cmd_init(),
```

to:
```rust
        Commands::Init { from } => cmd_init(from),
```

- [ ] **Step 2: Rewrite `cmd_init` to handle all cases**

Replace the entire `cmd_init` function with:

```rust
fn cmd_init(from: Option<String>) -> Result<(), String> {
    let cwd = std::env::current_dir().map_err(|e| format!("cannot read current directory: {e}"))?;
    let config_path = cwd.join(".quickdev.toml");

    let dir_name = cwd
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "project".to_string());

    let global_path = global_config_path();
    let mut global = load_global_config(&global_path)?;

    let cwd_str = cwd.to_string_lossy().to_string();
    let already_indexed = global.projects.iter().any(|p| p.path == cwd_str);

    if config_path.exists() && already_indexed {
        return Err(format!(
            ".quickdev.toml already exists and is registered in {}",
            cwd.display()
        ));
    }

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

    let project_name = unique_project_name(&dir_name, &global);

    let project_config = match from {
        Some(source_name) => {
            let source_entry = global
                .projects
                .iter()
                .find(|p| p.name == source_name)
                .ok_or_else(|| format!("source project '{}' not found in global index", source_name))?;
            let source_path = PathBuf::from(&source_entry.path).join(".quickdev.toml");
            let source = load_project_config(&source_path)?;
            ProjectConfig {
                project: ProjectEntry {
                    name: project_name.clone(),
                },
                terminals: source.terminals,
                applications: source.applications,
            }
        }
        None => ProjectConfig {
            project: ProjectEntry {
                name: project_name.clone(),
            },
            terminals: vec![],
            applications: vec![],
        },
    };

    save_project_config(&config_path, &project_config)?;

    global.projects.push(GlobalProjectEntry {
        name: project_name.clone(),
        path: cwd_str,
    });
    save_global_config(&global_path, &global)?;

    if from.is_some() {
        println!(
            "Initialized project '{}' from template in {}",
            project_name,
            cwd.display()
        );
    } else {
        println!(
            "Initialized project '{}' in {}",
            project_name,
            cwd.display()
        );
    }
    println!("Global index updated at {}", global_path.display());
    Ok(())
}
```

- [ ] **Step 3: Verify build and tests**

```bash
cargo build && cargo test
```

Expected: compiles, all tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/main.rs
git commit -m "Add init --from templates and smart re-register"
```

---

### Task 6: Edit --global + Deregister + Args Prompt

Three small features in one task since they're each just a few lines.

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Add `--global` flag to Edit command**

Change:
```rust
    /// Open .quickdev.toml in $EDITOR
    Edit,
```

to:

```rust
    /// Open .quickdev.toml in $EDITOR (or --global for global config)
    Edit {
        /// Edit global config instead of project config
        #[arg(long)]
        global: bool,
    },
```

Update the main match arm from:
```rust
        Commands::Edit => cmd_edit(),
```

to:
```rust
        Commands::Edit { global } => cmd_edit(global),
```

- [ ] **Step 2: Update `cmd_edit` to handle `--global`**

Replace `cmd_edit` with:

```rust
fn cmd_edit(global: bool) -> Result<(), String> {
    let config_path = if global {
        global_config_path()
    } else {
        let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
        let (path, _root) = resolve_project_config(&cwd)?;
        path
    };

    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());

    std::process::Command::new(&editor)
        .arg(&config_path)
        .status()
        .map_err(|e| format!("failed to open editor '{}': {}", editor, e))?;

    Ok(())
}
```

- [ ] **Step 3: Add Deregister command**

Add to the `Commands` enum:

```rust
    /// Remove current project from global index
    Deregister {
        /// Also delete the .quickdev.toml file
        #[arg(long)]
        delete: bool,
    },
```

Add to the main match:
```rust
        Commands::Deregister { delete } => cmd_deregister(delete),
```

- [ ] **Step 4: Implement `cmd_deregister`**

Add this function:

```rust
fn cmd_deregister(delete: bool) -> Result<(), String> {
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    let (config_path, project_root) = resolve_project_config(&cwd)?;

    let global_path = global_config_path();
    let mut global = load_global_config(&global_path)?;

    let root_str = project_root.to_string_lossy().to_string();
    let before = global.projects.len();
    let removed_name = global
        .projects
        .iter()
        .find(|p| p.path == root_str)
        .map(|p| p.name.clone());
    global.projects.retain(|p| p.path != root_str);

    if global.projects.len() == before {
        return Err("project not found in global index".to_string());
    }

    save_global_config(&global_path, &global)?;

    if delete {
        std::fs::remove_file(&config_path)
            .map_err(|e| format!("failed to delete {}: {e}", config_path.display()))?;
        println!(
            "Deregistered and deleted config for '{}'",
            removed_name.unwrap_or_default()
        );
    } else {
        println!(
            "Deregistered project '{}'",
            removed_name.unwrap_or_default()
        );
    }

    Ok(())
}
```

- [ ] **Step 5: Add args prompt to interactive add**

In `cmd_add_interactive`, in the `"Application"` match arm, change:

```rust
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
```

to:

```rust
        "Application" => {
            let (app_name, app_path) = pick_application()?;

            if config.applications.iter().any(|a| a.name == app_name) {
                return Err(format!("application '{}' already exists", app_name));
            }

            let args_input =
                prompt("Arguments (e.g., \".\" to open project root, Enter to skip): ")?;
            let args = if args_input.is_empty() {
                None
            } else {
                Some(args_input.split_whitespace().map(String::from).collect())
            };

            config.applications.push(AppEntry {
                name: app_name.clone(),
                path: app_path,
                args,
            });
```

- [ ] **Step 6: Update help examples**

Update the top-level `after_help` to include new commands. Replace:

```rust
    after_help = "\
Examples:
  quickdev init                                         Initialize a project
  quickdev launch                                       Launch current project
  quickdev launch my-api                                Launch a project by name
  quickdev add terminal server . --command \"npm start\"  Add a terminal tab
  quickdev add app Cursor /Applications/Cursor.app      Add an application
  quickdev remove                                       Interactive removal picker"
```

with:

```rust
    after_help = "\
Examples:
  quickdev init                                         Initialize a project
  quickdev init --from my-api                           Clone config from another project
  quickdev launch                                       Select items to launch
  quickdev launch --all                                 Launch everything
  quickdev launch my-api                                Launch a project by name
  quickdev add                                          Interactive add
  quickdev remove                                       Interactive removal picker
  quickdev list                                         Show all projects
  quickdev edit                                         Edit project config
  quickdev edit --global                                Edit global config
  quickdev deregister                                   Unregister project"
```

- [ ] **Step 7: Verify build and full test suite**

```bash
cargo build && cargo test
```

Expected: compiles, all tests pass.

- [ ] **Step 8: Smoke test new features**

```bash
cargo run -- --help
cargo run -- edit --global
cargo run -- deregister --help
```

Expected: help shows new commands, edit --global opens global config.

- [ ] **Step 9: Commit**

```bash
git add src/main.rs
git commit -m "Add edit --global, deregister command, and args prompt in interactive add"
```

---

### Task 7: Final Cleanup

Format, lint, verify everything.

- [ ] **Step 1: Format and lint**

```bash
cargo fmt && cargo clippy -- -W clippy::all
```

Fix any warnings.

- [ ] **Step 2: Full test suite**

```bash
cargo test
```

Expected: all tests pass.

- [ ] **Step 3: End-to-end smoke test**

```bash
cargo run -- list
cargo run -- launch
cargo run -- launch --all
cargo run -- edit --global
cargo run -- deregister --help
```

- [ ] **Step 4: Commit any fixes**

```bash
git add -A
git commit -m "Apply cargo fmt and clippy fixes"
```

(Skip if no changes.)
