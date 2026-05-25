# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| 0.1.x   | Yes       |

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
- **Path traversal** in terminal `path` fields. Paths are resolved relative to the project root. Absolute paths are used as-is.
- **Config file trust.** QuickDev executes commands defined in `.quickdev.toml`. Only use configs from repositories you trust. Review `.quickdev.toml` before running `quickdev launch` on cloned projects.

## Dependencies

QuickDev has minimal dependencies:

- `clap` — CLI argument parsing
- `serde` + `toml` — configuration serialization
- `dirs` — home directory resolution

No network access, no remote code execution, no telemetry.
