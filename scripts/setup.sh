#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

command -v cargo >/dev/null || { echo "error: cargo not found" >&2; exit 1; }

echo "==> rustfmt"
rustup component add rustfmt

if ! command -v cargo-tarpaulin >/dev/null; then
  echo "==> install cargo-tarpaulin"
  cargo install cargo-tarpaulin --locked
fi

if ! command -v cargo-upgrade >/dev/null; then
  echo "==> install cargo-edit"
  cargo install cargo-edit --locked
fi

if ! command -v pre-commit >/dev/null; then
  echo "==> install pre-commit"
  if command -v pipx >/dev/null; then
    pipx install pre-commit
  elif command -v pip >/dev/null; then
    pip install --user pre-commit
  else
    echo "error: install pipx or pip to get pre-commit" >&2
    exit 1
  fi
fi

if ! command -v cz >/dev/null; then
  echo "==> install commitizen"
  if command -v pipx >/dev/null; then
    pipx install commitizen
  elif command -v pip >/dev/null; then
    pip install --user commitizen
  else
    echo "error: install pipx or pip to get commitizen" >&2
    exit 1
  fi
fi

echo "==> install git hooks"
pre-commit install
pre-commit install --hook-type pre-push

echo "==> setup done"
