#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

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
cargo tarpaulin --tests --fail-under 100

echo "==> bump version (commitizen)"
cz bump

echo "==> push main and tags"
git push origin main --follow-tags

echo "==> publish crate"
cargo publish

echo "==> release done"
