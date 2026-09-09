#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

echo "==> checkout and pull main"
git checkout main
git pull origin main

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
