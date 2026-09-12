#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

echo "==> upgrade Cargo.toml deps"
if command -v cargo-upgrade >/dev/null || cargo upgrade -h >/dev/null 2>&1; then
  cargo upgrade -i allow
else
  echo "warning: cargo-upgrade not found (cargo install cargo-edit); skipping Cargo.toml bumps" >&2
fi

echo "==> update Cargo.lock"
cargo update

echo "==> format"
cargo fmt

echo "==> test"
cargo test --all-features --tests -- --test-threads=1

echo "==> maintenance done"
