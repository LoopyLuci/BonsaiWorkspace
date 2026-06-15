#!/usr/bin/env bash
set -euo pipefail

echo "Building BonsAI V2 ecosystem..."
cargo build --release -p bonsai-bat
cargo build --release -p bonsai-moe
cargo build --release -p bonsai-kef
cargo build --release -p bonsai-tdl
cargo build --release -p bonsai-safety
cargo build --release -p bonsai-package
echo "Running tests..."
cargo test --workspace --release
echo "BonsAI V2 ecosystem ready"
