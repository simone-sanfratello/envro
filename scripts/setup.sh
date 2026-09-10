#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
# shellcheck source=lib.sh
source "$(dirname "$0")/lib.sh"

command -v cargo >/dev/null || { echo "error: cargo not found" >&2; exit 1; }

echo "==> rustfmt"
rustup component add rustfmt

if ! require_cmd cargo-tarpaulin; then
  echo "==> install cargo-tarpaulin"
  cargo install cargo-tarpaulin --locked
fi

if ! require_cmd cargo-upgrade; then
  echo "==> install cargo-edit"
  cargo install cargo-edit --locked
fi

echo "==> pipx"
ensure_pipx

echo "==> python tooling"
ensure_cmd pre-commit pre-commit
ensure_cmd cz commitizen

echo "==> install git hooks"
pre-commit install
pre-commit install --hook-type pre-push

echo "==> setup done"
echo "tools:"
echo "  rustfmt:         $(command -v rustfmt)"
echo "  cargo-tarpaulin: $(command -v cargo-tarpaulin)"
echo "  cargo-upgrade:   $(command -v cargo-upgrade)"
echo "  pipx:            $(command -v pipx)"
echo "  pre-commit:      $(command -v pre-commit)"
echo "  cz:              $(command -v cz)"
