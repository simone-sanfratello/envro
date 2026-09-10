#!/usr/bin/env bash
# shared helpers for setup / release / maintenance

export PATH="${HOME}/.local/bin:${HOME}/.cargo/bin:${PATH}"

require_cmd() {
  command -v "$1" >/dev/null 2>&1
}

hash_refresh() {
  hash -r 2>/dev/null || true
}

ensure_pipx() {
  if ! require_cmd pipx; then
    echo "==> install pipx"
    sudo apt install -y pipx
  fi

  echo "==> pipx ensurepath"
  pipx ensurepath
  # apply for current process (ensurepath usually only updates shell rc files)
  export PATH="${HOME}/.local/bin:${PATH}"
  hash_refresh

  if ! require_cmd pipx; then
    echo "error: pipx still not found after install" >&2
    return 1
  fi
}

ensure_python_installer() {
  if require_cmd uv || require_cmd pipx || require_cmd pip3 || require_cmd pip; then
    if require_cmd pipx; then
      pipx ensurepath >/dev/null 2>&1 || true
      export PATH="${HOME}/.local/bin:${PATH}"
    fi
    return 0
  fi

  ensure_pipx
}

install_python_tool() {
  local pkg="$1"
  ensure_python_installer

  if require_cmd uv; then
    uv tool install "$pkg"
  elif require_cmd pipx; then
    pipx install "$pkg"
  elif require_cmd pip3; then
    pip3 install --user "$pkg"
  else
    pip install --user "$pkg"
  fi

  hash_refresh
}

ensure_cmd() {
  local cmd="$1"
  local pkg="${2:-$1}"

  if require_cmd "$cmd"; then
    return 0
  fi

  echo "==> install ${pkg} (${cmd} not found)"
  install_python_tool "$pkg"

  if ! require_cmd "$cmd"; then
    echo "error: ${cmd} still not found after installing ${pkg}" >&2
    echo "hint: ensure ${HOME}/.local/bin is on PATH (restart the shell or: export PATH=\"\$HOME/.local/bin:\$PATH\")" >&2
    return 1
  fi
}
