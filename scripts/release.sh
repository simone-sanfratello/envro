#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
# shellcheck source=lib.sh
source "$(dirname "$0")/lib.sh"

echo "==> checkout and pull main"
git checkout main
git pull origin main

if [[ -n "$(git status --porcelain)" ]]; then
  echo "error: working tree is not clean; commit or stash changes before release" >&2
  git status --short >&2
  exit 1
fi

echo "==> format check"
cargo fmt --check

echo "==> tests"
cargo test --tests -- --test-threads=1

echo "==> coverage"
if ! require_cmd cargo-tarpaulin; then
  echo "error: cargo-tarpaulin not found; run: just setup" >&2
  exit 1
fi
cargo tarpaulin --tests --fail-under 100

echo "==> bump version (commitizen)"
ensure_cmd cz commitizen
cz bump

echo "==> push main and tags"
# cz creates lightweight tags; --follow-tags only pushes annotated tags
git push origin main
git push origin "refs/tags/$(git describe --tags --exact-match HEAD)"

echo "==> publish crate"
cargo publish

echo "==> release done"

