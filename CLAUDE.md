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

## Architecture

```
CLI (clap) → config.rs (TOML read/write) → launch.rs (process spawning)
```

- **main.rs**: Clap CLI entry point, subcommand dispatch
- **models.rs**: Serde structs for global index and per-project config
- **config.rs**: TOML parsing, global index management, `.quickdev.toml` discovery (walks parents)
- **adapters.rs**: Tool detection (Cursor, VS Code, Zed, Ghostty), command resolution
- **launch.rs**: Cross-platform terminal and application launching via `std::process::Command`

### Config locations

- **Global index**: `~/Documents/quickdev/config.toml`
- **Per-project**: `.quickdev.toml` in project root

## Conventions

- Standard `cargo fmt` formatting
- snake_case everywhere
- Short imperative commit messages
- No async — pure synchronous Rust
