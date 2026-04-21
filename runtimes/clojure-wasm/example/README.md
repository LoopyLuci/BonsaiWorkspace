# Example ClojureWasm Module

This directory contains a minimal example Wasm module and a `build.sh`
helper that compiles a tiny Rust crate to `module.wasm` (WASI target).

Usage:

- Locally (if you have Rust + `wasm32-wasi` target):

  ```bash
  cd runtimes/clojure-wasm/example
  ./build.sh
  ```

- In CI this script will be invoked by the repository CI steps if present.

The built `module.wasm` exports a `_start` function suitable for testing
the in-process Wasmtime host in `bonsai-runtime`.
