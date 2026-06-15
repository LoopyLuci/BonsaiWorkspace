# ClojureWasm — Local (No-Docker) Build Instructions

This directory contains helper scripts and guidance to build ClojureWasm modules locally without Docker. The goal is to support development and CI that does not rely on container images.

Prerequisites (install on your host):
- `git`
- OpenJDK 17+ (`java` on PATH)
- Rust toolchain (`rustup`, `cargo`, `rustc`) and the `wasm32-wasi` target
- `curl` / `unzip` (for convenience)

Quick local steps (Linux/macOS):

1. Ensure prerequisites are installed.
2. From the repository root run:

```bash
./runtimes/clojure-wasm/build-local.sh
```

On Windows PowerShell:

```powershell
.\runtimes\clojure-wasm\build-local.ps1
```

Scripts attempt to clone `https://github.com/clojurewasm/ClojureWasm` into `build/clojurewasm` and will either run the repository's build hooks (if present) or print precise next steps. In CI use the `--ci` (bash) or `-CI` (PowerShell) flags to run prereq checks and cloning without starting potentially long builds.

If you need help running the build on your platform, open an issue in the repo and include the output of the script.
