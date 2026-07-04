#!/usr/bin/env bash
set -euo pipefail

echo "Deploy macOS placeholder script"
cargo build --release -p omnisystem-workspace || true
echo "Add notarization/signing pipeline here"
