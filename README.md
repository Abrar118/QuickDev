# QuickDev

A cross-platform CLI tool that saves and launches your project's terminal tabs and applications with a single command. Define your terminal layout and apps per project, then `quickdev launch` opens everything at once.

## Why QuickDev?

Every time you start working on a project, you open the same terminals, navigate to the same directories, run the same commands, and launch the same apps. QuickDev eliminates that repetition. Configure once, launch instantly.

## Features

- **Terminal tabs** with custom working directories and startup commands
- **Application launching** with automatic tool detection (VS Code, Cursor, Zed)
- **Capture running apps** (macOS) into your project config with `quickdev capture`
- **Interactive mode** with [fzf](https://github.com/junegunn/fzf) for adding, removing, and selectively launching items
- **Per-project config** stored as a human-readable, machine-local `.quickdev.toml` in your project root
- **Global project index** so you can launch any project from anywhere
- **Config templates** to clone one project's setup to another
- **Diagnostics** with `quickdev doctor`, `validate`, and `prune` to keep configs healthy
- **Cross-platform** support for macOS, Linux, and Windows

## Installation

### Option 1: Download Pre-built Binary (Recommended)

Download the latest binary for your platform from the [Releases page](https://github.com/Abrar118/QuickDev/releases).

| Platform | File |
|----------|------|
| macOS (Apple Silicon) | `quickdev-macos-aarch64.tar.gz` |
| macOS (Intel) | `quickdev-macos-x86_64.tar.gz` |
| Linux (x86_64) | `quickdev-linux-x86_64.tar.gz` |
| Linux (ARM64) | `quickdev-linux-aarch64.tar.gz` |
| Windows (x86_64) | `quickdev-windows-x86_64.zip` |

Then extract and move to your PATH:

```bash
# macOS / Linux
tar xzf quickdev-*.tar.gz
sudo mv quickdev /usr/local/bin/

# Windows (PowerShell)
Expand-Archive quickdev-windows-x86_64.zip -DestinationPath .
Move-Item quickdev.exe C:\Windows\System32\
```

### Option 2: Build from Source

Requires [Rust](https://rustup.rs/) 1.70+.

```bash
git clone https://github.com/Abrar118/QuickDev.git
cd QuickDev
cargo install --path .
```

This installs the `quickdev` binary to `~/.cargo/bin/`. Make sure `~/.cargo/bin` is in your `PATH` (it is by default with rustup).

### Optional: Install fzf

[fzf](https://github.com/junegunn/fzf) enables interactive features (selection pickers for launch, add, remove, and project switching). QuickDev works without it, but the interactive modes require it.

```bash
# macOS
brew install fzf

# Linux
sudo apt install fzf

# Windows
choco install fzf
```

### Verify installation

```bash
quickdev --help
```

## Quick Start

```bash
# 1. Navigate to your project
cd ~/Code/my-project

# 2. Initialize QuickDev
quickdev init

# 3. Add terminals
quickdev add terminal server . --command "npm run dev"
quickdev add terminal logs ./logs
quickdev add terminal tests . --command "npm test"

# 4. Add applications
quickdev add app Cursor /Applications/Cursor.app --args "."

# 5. Launch everything
quickdev launch --all
```

Or use the interactive mode:

```bash
# Interactive add — prompts for type, path, name, command
quickdev add

# Interactive launch — pick which items to open
quickdev launch

# Interactive remove — pick which items to delete
quickdev remove
```

## Commands

### `quickdev init`

Initialize a new project in the current directory. Creates `.quickdev.toml` and registers the project in the global index.

```bash
quickdev init                    # Create empty config
quickdev init --from my-api      # Clone config from another project
```

If a `.quickdev.toml` already exists but isn't registered (e.g., you cloned a repo that has one), `quickdev init` will re-register it instead of erroring.

### `quickdev launch`

Launch terminals and applications for a project.

```bash
quickdev launch                  # Interactive picker (select items with fzf)
quickdev launch my-api           # Interactive picker for a project by name
quickdev launch --all            # Launch everything without picker
quickdev launch my-api --all     # Launch everything for a named project
quickdev launch --dry-run        # Preview what would launch without opening anything
```

When run without `--all`, an fzf multi-select picker appears — whether you launch the current project or name one explicitly. Use `TAB` to toggle items and `ENTER` to launch. If only one item is configured, it launches directly.

Application `args` support placeholders like `{root}`, `{name}`, and `{cwd}` — see [Application argument placeholders](#application-argument-placeholders).

### `quickdev list`

Show all registered projects with their terminal and app names.

```bash
quickdev list
```

Output:

```
Projects:
  QuickDev    /Users/you/Code/quickdev
    Terminals: dev, tests
    Apps: Cursor

  my-api      /Users/you/Code/my-api
    Terminals: server, worker, logs
    Apps: Cursor, Docker Desktop
```

Projects whose directory no longer exists are marked `(path missing)`; a directory that exists but has lost its `.quickdev.toml` is marked `(.quickdev.toml missing)`.

Flags:

```bash
quickdev list --missing    # Only projects whose path or .quickdev.toml is missing
quickdev list --json       # Machine-readable JSON array (scriptable)
```

### `quickdev add`

Add terminals or applications to the current project.

**Interactive mode** (no arguments):

```bash
quickdev add
```

Prompts you to select Terminal or Application, then walks you through the fields. For applications on macOS, it scans `/Applications` and lets you pick from an fzf list.

**Direct mode:**

```bash
# Add a terminal
quickdev add terminal <name> <path> [--command "..."] [--emulator ghostty]

# Add an application
quickdev add app <name> <path> [--args "." "--flag"]
```

Examples:

```bash
quickdev add terminal server .                         # Shell in project root
quickdev add terminal dev . --command "npm run dev"    # Run command on open
quickdev add terminal logs ./logs                      # Shell in subdirectory
quickdev add app Cursor /Applications/Cursor.app --args "."
quickdev add app Firefox /usr/bin/firefox
```

### `quickdev remove`

Remove terminals or applications from the current project.

**Interactive mode** (no arguments):

```bash
quickdev remove
```

Opens an fzf multi-select picker showing all configured items. Use `TAB` to toggle and `ENTER` to confirm removal.

**Direct mode:**

```bash
quickdev remove terminal <name>
quickdev remove app <name>
```

### `quickdev edit`

Open the project config in your `$EDITOR`.

```bash
quickdev edit                    # Edit project .quickdev.toml
quickdev edit --global           # Edit global config
```

Falls back to `vi` if `$EDITOR` is not set.

### `quickdev deregister`

Remove the current project from the global index.

```bash
quickdev deregister              # Remove from index, keep .quickdev.toml
quickdev deregister --delete     # Remove from index AND delete .quickdev.toml
```

### `quickdev validate`

Check the current project's `.quickdev.toml` for problems. Reports errors (terminal
paths that escape the project root, unsupported emulators, an empty project name) and
warnings (application paths that don't exist, unknown `{placeholder}` tokens). Exits
non-zero if there are any errors.

```bash
quickdev validate
```

### `quickdev prune`

Remove registrations from the global index whose project directory or `.quickdev.toml`
no longer exists.

```bash
quickdev prune
```

### `quickdev doctor`

Diagnose the global config and every registered project, printing a `✓`/`✗`/`⚠` summary.
A missing `fzf` is reported as a warning; a bad global config or any unhealthy project is
an error (non-zero exit).

```bash
quickdev doctor          # Report only
quickdev doctor --fix    # Create missing global config, prune dead registrations,
                         # and normalize project configs to canonical form
```

### `quickdev capture`

Snapshot the macOS GUI apps you currently have running and add the ones you pick
into this project's `.quickdev.toml`:

```bash
quickdev capture        # interactive multi-select (requires fzf)
quickdev capture --all  # add all detected apps without prompting
```

Only running apps installed in `/Applications` or `~/Applications` are offered, and
apps already in your config are skipped. macOS only. The first run may prompt for
Automation/Accessibility permission so QuickDev can read the list of running apps via
System Events.

## Configuration

### Project Config — `.quickdev.toml`

Each project has a `.quickdev.toml` in its root directory:

```toml
[project]
name = "my-api"

[[terminals]]
name = "server"
path = "."
command = "npm run dev"

[[terminals]]
name = "logs"
path = "./logs"

[[terminals]]
name = "database"
path = "."
command = "docker compose up"
emulator = "ghostty"

[[applications]]
name = "Cursor"
path = "/Applications/Cursor.app"
args = ["."]

[[applications]]
name = "Docker Desktop"
path = "/Applications/Docker.app"
```

#### Terminal fields

| Field | Required | Description |
|-------|----------|-------------|
| `name` | yes | Label for this terminal tab |
| `path` | yes | Working directory, relative to project root (e.g., `.`, `./src`) |
| `command` | no | Startup command to run when the terminal opens |
| `emulator` | no | Terminal emulator override: `"ghostty"`, `"terminal"`. Omit for auto-detect |

#### Application fields

| Field | Required | Description |
|-------|----------|-------------|
| `name` | yes | Application display name |
| `path` | yes | Executable or `.app` bundle path |
| `args` | no | Arguments passed to the application (e.g., `["."]` to open project root) |

#### Application argument placeholders

`args` entries support placeholders, substituted at launch time:

| Placeholder | Expands to |
|-------------|------------|
| `{root}`    | project root absolute path |
| `{config}`  | absolute path to the project's `.quickdev.toml` |
| `{name}`    | project name |
| `{cwd}`     | the directory you ran `quickdev` from |

A bare `"."` still means `{root}` (backward compatible). Placeholders are substituted as substrings, so `"{root}/README.md"` works.

```toml
[[applications]]
name = "Cursor"
path = "/Applications/Cursor.app"
args = ["{root}", "--reuse-window"]
```

For editor tools (VS Code / Cursor / Zed): if `args` are provided they are used as-is; otherwise the project root is opened by default.

### Global Config — `~/Documents/quickdev/config.toml`

The global index is auto-managed. You can set a default terminal emulator here:

```toml
emulator = "ghostty"

[[projects]]
name = "my-api"
path = "/Users/you/Code/my-api"

[[projects]]
name = "my-app"
path = "/Users/you/Code/my-app"
```

Edit with `quickdev edit --global`.

### Emulator Priority

When launching a terminal, QuickDev checks:

1. **Per-terminal** `emulator` field in `.quickdev.toml` (highest priority)
2. **Global** `emulator` in `~/Documents/quickdev/config.toml`
3. **Auto-detect** (lowest priority)

Auto-detect tries Ghostty first, then falls back to the platform default.

### Managing the default emulator

Set the global default terminal emulator without hand-editing the global config:

```bash
quickdev config set emulator ghostty   # ghostty | terminal
quickdev config get emulator
quickdev config unset emulator
```

## Terminal Emulator Support

| Emulator | macOS | Linux | Windows | Tabs |
|----------|-------|-------|---------|------|
| Ghostty | yes | yes | no | no (separate windows) |
| Terminal.app | yes | - | - | yes |
| gnome-terminal | - | yes | - | yes |
| Windows Terminal | - | - | yes | yes |
| PowerShell 7 | - | - | yes | no (separate windows) |
| konsole | - | yes | - | no |
| alacritty | - | yes | - | no |
| xterm | - | yes | - | no |

**Tab behavior:** When multiple terminals are configured, emulators that support tabs will open subsequent terminals as tabs in the same window. The first terminal always opens a new window.

**Ghostty note:** Ghostty does not support opening tabs from the CLI ([ghostty-org/ghostty#12136](https://github.com/ghostty-org/ghostty/issues/12136)). Each terminal opens as a separate window.

## Tool Detection

When launching applications, QuickDev auto-detects known tools and uses their CLI commands:

| Application | Detected by | CLI used | Behavior |
|-------------|-------------|----------|----------|
| VS Code | name/path contains "vscode", "Visual Studio Code", or path ends with `code.app` | `code` | Opens project root |
| Cursor | name/path contains "cursor" | `cursor` | Opens project root |
| Zed | name/path contains "zed" | `zed` | Opens project root |

Editors automatically receive the project root as an argument. Unknown applications are launched generically (via `open -a` on macOS, direct execution elsewhere).

## Path Resolution

- **Terminal paths** are relative to the project root. `"."` means the project root, `"./src"` means the `src` subdirectory.
- **Application paths** are absolute. `~` expansion is supported (e.g., `~/bin/myapp`).
- **Application args** containing `"."` are replaced with the project root path at launch time.
- Paths that don't exist at launch time produce a warning but don't abort other items.

## Project Picker

If you run any command (`launch`, `add`, `remove`, `edit`) from a directory that doesn't contain a `.quickdev.toml` (or any parent), QuickDev shows an fzf picker with all registered projects. Select one and the command proceeds as if you were in that directory.

If fzf is not installed, the command errors with a hint to install fzf.

## Tips

- **Config templates:** Set up one project's config, then use `quickdev init --from my-api` in new projects to get the same terminal/app layout.
- **Edit directly:** The `.quickdev.toml` file is plain TOML. You can edit it by hand — `quickdev edit` opens it in your editor.
- **Keep it local:** `.quickdev.toml` describes your personal machine (absolute app paths, your preferred emulator), so it's git-ignored by default rather than shared.
- **Scripting:** All commands work non-interactively when given explicit arguments (e.g. `quickdev launch --all`, `quickdev add terminal dev .`). The `launch` picker activates whenever `--all` is omitted; `add`/`remove` pickers activate when their item arguments are omitted.

## Building from Source

```bash
git clone https://github.com/Abrar118/quickdev.git
cd quickdev
cargo build --release
```

The binary is at `target/release/quickdev`.

### Running Tests

```bash
cargo test
```

### Development

```bash
cargo run -- <subcommand>        # Run without installing
cargo fmt                        # Format code
cargo clippy                     # Lint
```

## License

Licensed under the [MIT License](LICENSE).
