#!/usr/bin/env bash
# Configure GitHub Actions secrets needed by .github/workflows/release.yml
set -euo pipefail

REPO="${REPO:-simone-sanfratello/envro}"

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "error: $1 not found" >&2
    exit 1
  }
}

require_cmd gh
require_cmd cargo

if ! gh auth status >/dev/null 2>&1; then
  echo "error: gh is not authenticated; run: gh auth login" >&2
  exit 1
fi

echo "==> repo: ${REPO}"
echo
echo "Secrets used by Release workflow:"
echo "  CARGO_REGISTRY_TOKEN   required — crates.io API token (cargo publish)"
echo "  RELEASE_GITHUB_TOKEN   recommended — classic PAT with 'repo' scope"
echo "                         (needed to push bump commits/tags to main;"
echo "                         falls back to GITHUB_TOKEN if unset)"
echo

# --- CARGO_REGISTRY_TOKEN ---------------------------------------------------
if [[ -n "${CARGO_REGISTRY_TOKEN:-}" ]]; then
  echo "==> setting CARGO_REGISTRY_TOKEN from env"
  printf '%s' "$CARGO_REGISTRY_TOKEN" | gh secret set CARGO_REGISTRY_TOKEN --repo "$REPO"
else
  echo "==> setting CARGO_REGISTRY_TOKEN (paste crates.io token, then Enter)"
  echo "    create at: https://crates.io/settings/tokens"
  echo "    scope: publish-update for crate 'envro'"
  gh secret set CARGO_REGISTRY_TOKEN --repo "$REPO"
fi

# --- RELEASE_GITHUB_TOKEN ---------------------------------------------------
if [[ "${SKIP_RELEASE_GITHUB_TOKEN:-0}" == "1" ]]; then
  echo "==> skipping RELEASE_GITHUB_TOKEN (SKIP_RELEASE_GITHUB_TOKEN=1)"
elif [[ -n "${RELEASE_GITHUB_TOKEN:-}" ]]; then
  echo "==> setting RELEASE_GITHUB_TOKEN from env"
  printf '%s' "$RELEASE_GITHUB_TOKEN" | gh secret set RELEASE_GITHUB_TOKEN --repo "$REPO"
else
  echo "==> setting RELEASE_GITHUB_TOKEN (paste classic PAT with 'repo' scope)"
  echo "    create at: https://github.com/settings/tokens"
  echo "    skip with: SKIP_RELEASE_GITHUB_TOKEN=1 $0"
  gh secret set RELEASE_GITHUB_TOKEN --repo "$REPO"
fi

echo
echo "==> current secrets:"
gh secret list --repo "$REPO"
echo
echo "==> done"
