#!/usr/bin/env bash
set -euo pipefail

echo "Setting up Bonsai Ecosystem..."

if ! command -v rustc >/dev/null 2>&1; then
  echo "Rust is missing. Install via rustup before continuing."
  exit 1
fi

if ! command -v node >/dev/null 2>&1; then
  echo "Node.js is missing. Install Node 20+ before continuing."
  exit 1
fi

if ! command -v python3 >/dev/null 2>&1; then
  echo "Python 3 is missing. Install Python 3.11+ before continuing."
  exit 1
fi

mkdir -p target dist logs manifests

if [ -f "requirements.txt" ]; then
  python3 -m pip install -r requirements.txt
fi

cargo build -p bonsai-cli --release

echo "Setup complete."
