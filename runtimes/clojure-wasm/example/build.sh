#!/usr/bin/env bash
set -euo pipefail

echo "[clojure-wasm example] Building minimal wasm via cargo"
TMPDIR=$(mktemp -d)

cat > "$TMPDIR/Cargo.toml" <<'TOML'
[package]
name = "cljwasm_example"
version = "0.1.0"
edition = "2021"
[lib]
crate-type = ["cdylib"]
TOML

mkdir -p "$TMPDIR/src"
cat > "$TMPDIR/src/lib.rs" <<'RUST'
#[no_mangle]
pub extern "C" fn _start() {
    // no-op start
}
RUST

# Ensure wasm target exists (harmless if already installed)
rustup target add wasm32-wasi || true

echo "Building temporary cargo crate to produce module.wasm"
cargo build --manifest-path "$TMPDIR/Cargo.toml" --target wasm32-wasi --release
WASM_PATH="$TMPDIR/target/wasm32-wasi/release/cljwasm_example.wasm"
if [ -f "$WASM_PATH" ]; then
  cp "$WASM_PATH" module.wasm
  echo "WASM written to module.wasm"
else
  echo "Build did not produce wasm at $WASM_PATH" >&2
  exit 1
fi

echo "Example module ready: $(pwd)/module.wasm"
