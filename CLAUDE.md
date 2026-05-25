# CLAUDE.md

## What is QuickDev

QuickDev is a cross-platform Rust CLI that manages per-project terminal and application launch configurations. Define terminals (directory + optional command) and apps per project via TOML, then `quickdev launch` opens them all.

## Commands

```bash
cargo build                  # Build the CLI binary
cargo run -- <subcommand>    # Run a subcommand (init, launch, list, add, remove, edit)
cargo test                   # Run all tests
cargo fmt                    # Format Rust code
cargo clippy                 # Lint
```

### Other useful commands

```bash
cargo install --path .       # Install binary locally for manual testing
cargo test config_test       # Run a single test module
git tag v0.1.1 && git push --tags  # Trigger release build via GitHub Actions
```

## Architecture

```
CLI (clap) → config.rs (TOML read/write) → launch.rs (process spawning)
```

- **main.rs**: Clap CLI entry point, subcommand dispatch
- **models.rs**: Serde structs for global index and per-project config
- **config.rs**: TOML parsing, global index management, `.quickdev.toml` discovery (walks parents)
- **adapters.rs**: Tool detection (Cursor, VS Code, Zed, Ghostty), command resolution
- **launch.rs**: Cross-platform terminal and application launching via `std::process::Command`
- **fzf.rs**: fzf binary wrapper for interactive selection (check, install hint, select one/multi)
- **apps.rs**: macOS `/Applications` discovery for interactive app picker
- **lib.rs**: Public module exports

**Note:** `main.rs` (~700 lines) contains all subcommand handlers inline. It's the largest file.

### Config locations

- **Global index**: `~/Documents/quickdev/config.toml`
- **Per-project**: `.quickdev.toml` in project root

### Runtime dependencies

- **fzf** (optional): Required for interactive selection in `launch`, `add`, `remove`. CLI works without it in direct/flag mode.

### CI

- `release.yml`: Cross-platform binary builds on `v*` tag push (macOS x86/arm, Linux x86/arm, Windows x86)
- `codeql.yml`: Automated code scanning

### Platform gotchas

- Spawned processes must use `Stdio::null()` on stdout/stderr to prevent `SEL: deleteBackward:` spam on macOS
- Use `$SHELL -lc` not `sh -lc` — user's shell aliases/tools (nvm, cargo, etc.) need a login shell
- Do NOT use `-i` (interactive) flag — conflicts with Powerlevel10k instant prompt
- Ghostty has no CLI tab support — each terminal opens a separate window
- `infer_tool_id` in adapters.rs uses precise matching to avoid false positives (e.g. "Codex" must not match "code")

## Conventions

- Standard `cargo fmt` formatting
- snake_case everywhere
- Short imperative commit messages
- No async — pure synchronous Rust
- 34 tests across `tests/` — run `cargo test` before committing
