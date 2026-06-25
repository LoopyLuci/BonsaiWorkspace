#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
JUSTFILE_PATH="$ROOT_DIR/scripts/devkit/justfile"

if ! command -v just >/dev/null 2>&1; then
  echo "DevKit launcher requires 'just'. Install with: cargo install just" >&2
  exit 1
fi

exec just --justfile "$JUSTFILE_PATH" "$@"
