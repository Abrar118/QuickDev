#!/usr/bin/env bash
set -euo pipefail

# Publish the quickdev npm wrapper package.
# One-time prerequisite: npm login
# Usage: packaging/scripts/publish-npm.sh <version>   (e.g. 0.2.0)
# <version> must match an existing GitHub release tag vX.Y.Z, because the
# postinstall downloads from releases/download/v<version>/.

if [[ $# -ne 1 ]]; then
  echo "usage: $0 <version>   (e.g. 0.2.0)" >&2
  exit 1
fi
version="$1"

cd "$(git rev-parse --show-toplevel)/packaging/npm"

echo "==> setting package version to $version"
npm version "$version" --no-git-tag-version --allow-same-version

echo "==> npm pack --dry-run (inspect tarball contents)"
npm pack --dry-run

echo
read -r -p "Publish quickdev@$version to npm? [y/N] " reply
if [[ "$reply" == "y" || "$reply" == "Y" ]]; then
  # Scoped package (@panda-orion/quickdev) — --access public is required to
  # publish it publicly (scoped packages default to restricted).
  npm publish --access public
  echo "==> published quickdev@$version"
else
  echo "==> aborted (nothing published)"
fi
