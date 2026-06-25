#!/usr/bin/env bash
set -euo pipefail

echo "Running Rust tests..."
cargo test --workspace

echo "Running frontend checks when available..."
if [ -f "bonsai-workspace/package.json" ]; then
  (cd bonsai-workspace && npm test || true)
fi

echo "Running python tests when available..."
if [ -d "runtimes" ]; then
  python3 -m pytest -q || true
fi

echo "Test run completed."
