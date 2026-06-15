#!/usr/bin/env bash
set -euo pipefail

SERVE=false
PORT=3000

while [[ $# -gt 0 ]]; do
  case "$1" in
    --serve) SERVE=true; shift ;;
    --port) PORT="$2"; shift 2 ;;
    *) shift ;;
  esac
done

echo "Generating Rust docs..."
cargo doc --workspace --no-deps

if [ "$SERVE" = true ]; then
  echo "Serving docs at http://127.0.0.1:${PORT}"
  python3 -m http.server "$PORT" --directory target/doc
fi
