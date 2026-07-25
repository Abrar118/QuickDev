# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| 0.4.x   | Yes       |

Only the latest release on the `master` branch receives security updates.

## Branch Protection

This repository is public and open to contributions, with the following rules:

- **Direct pushes to `master` are restricted to the repository owner only.** All other contributors must submit changes via pull requests.
- **Pull requests require review and approval** from the repository owner before merging. No self-merging by contributors.
- **Force pushes to `master` are not allowed** (except by the owner for administrative purposes).
- **Branch deletion of `master` is not allowed.**

### For Contributors

1. Fork the repository or create a feature branch.
2. Make your changes and push to your fork/branch.
3. Open a pull request against `master`.
4. Wait for review and approval from the maintainer.
5. The maintainer will merge approved pull requests.

### For the Maintainer

The repository owner (@Abrar118) is the only person who can:

- Push directly to `master`
- Merge pull requests
- Manage releases

## Reporting a Vulnerability

If you discover a security vulnerability in QuickDev, please report it responsibly:

1. **Do not open a public issue.** Security vulnerabilities should not be disclosed publicly until a fix is available.
2. **Email the maintainer** at abrarme118@gmail.com with:
   - A description of the vulnerability
   - Steps to reproduce
   - The potential impact
   - Any suggested fix (optional)
3. **Expected response time:** You will receive an acknowledgment within 48 hours. A fix or mitigation plan will be communicated within 7 days.
4. **Disclosure timeline:** Once a fix is released, the vulnerability will be publicly disclosed in the release notes. Credit will be given to the reporter unless they prefer to remain anonymous.

## Scope

QuickDev is a local CLI tool that spawns processes on the user's machine. Security concerns include:

- **Command injection** via `.quickdev.toml` fields (path, command, args). QuickDev passes these values to `std::process::Command` which does not invoke a shell for application launches, mitigating injection risks. Terminal commands are executed via shell (`sh -lc` / `zsh -lc`) by design, since they are user-authored.
- **Path traversal** in terminal `path` fields. Paths are resolved relative to the project root, and must name an existing directory. Absolute paths and `..` components are rejected. Note that this containment is *lexical*: a symlink inside the project that points elsewhere is followed, so a terminal can open outside the project root. This is not a sandbox — terminal `command` values are arbitrary shell by design (see below) — and should not be relied on as one.
- **Config file trust.** QuickDev executes commands defined in `.quickdev.toml`. Only use configs from repositories you trust. Review `.quickdev.toml` before running `quickdev launch` on cloned projects.
- **Temporary session files.** Grouped (tabbed) launches on kitty and gnome-terminal write per-tab wrapper scripts to a randomly-named, owner-only (`0700`) temporary directory. These files contain your terminal `command` values, outlive the process because the terminal reads them asynchronously, and are removed on a later launch.

## Dependencies

QuickDev has minimal dependencies:

- `clap` — CLI argument parsing
- `serde` + `toml` + `toml_edit` — configuration parsing and comment-preserving writes
- `dirs` — home directory resolution
- `shell-words` — POSIX-style splitting of user-supplied argument strings
- `tempfile` — atomic config writes and owner-only session directories
- `lnk` (Windows only) — reading Start Menu shortcuts during app discovery

The CLI itself performs no network access and has no telemetry. The optional
npm distribution (`@panda-orion/quickdev`) does: its `postinstall` step
downloads the matching release binary from GitHub over HTTPS and verifies it
against the published SHA-256 checksum before use.
