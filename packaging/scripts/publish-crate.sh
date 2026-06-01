#!/usr/bin/env bash
set -euo pipefail

# Publish the quickdev crate to crates.io.
# One-time prerequisite: cargo login <token>   (get a token at https://crates.io/me)
# Run from anywhere inside the repo. The crate version published is whatever
# Cargo.toml currently declares — bump it (Task 1) before running.

cd "$(git rev-parse --show-toplevel)"

echo "==> cargo package --list"
cargo package --list

# Dry run tolerates an uncommitted tree (it never uploads); the real publish
# below stays strict and will refuse to upload a dirty tree.
echo "==> cargo publish --dry-run"
cargo publish --dry-run --allow-dirty

echo
read -r -p "Publish $(cargo pkgid 2>/dev/null || echo quickdev) to crates.io now? [y/N] " reply
if [[ "$reply" == "y" || "$reply" == "Y" ]]; then
  cargo publish
  echo "==> published"
else
  echo "==> aborted (nothing published)"
fi
