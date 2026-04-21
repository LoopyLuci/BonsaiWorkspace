## ClojureWasm Integration Plan — Bonsai Workspace

Goal: provide a robust, secure path to run Clojure code compiled to WebAssembly (via ClojureWasm) as a managed runtime in `bonsai-runtime`, with good developer ergonomics and CI support.

Overview:
- Evaluate ClojureWasm output (WASM modules targeting WASI) and its required host bindings.
- Provide a `clojurewasm` runtime kind in `bonsai-runtime` that can host compiled modules using a Wasm runtime (Wasmtime/Wasmer) with WASI sandboxing.
- Add build tooling to compile Clojure sources to Wasm (CI + local dev scripts).

Phased plan:
1) Research & prototype (1-2 weeks)
  - Clone https://github.com/clojurewasm/ClojureWasm and build a minimal "hello" Wasm module using their toolchain.
  - Confirm the produced Wasm is WASI-compatible and determine ABI (how to call into the module, expected exported functions, memory model).
  - Prototype hosting the module with a small Rust binary using the `wasmtime` crate: instantiate, call init function, and call a `health` or `handle` entry point.
  - Measure module size/perf/memory usage.

2) Design host API + module contract (3-5 days)
  - Define the runtime contract: how modules expose health checks, log output, handle requests, and do I/O. Prefer a small WASI-compatible convention:
    - Exports: `run()` or `handle_request(ptr, len)` (use WASI fd/io for streaming if needed)
    - Environment: pass config via env vars or WASI args
  - Decide on integration points for existing Bonsai flows: make modules respond to stdin protocol (like `bb_runner.clj`) or HTTP on an ephemeral port.

3) Implement `clojurewasm` runtime in `bonsai-runtime` (2-3 weeks)
  - Add `start_clojurewasm_worker(module_path, options)` to `RuntimeManager`.
  - Use `wasmtime` crate to run modules in-process (preferred for performance) or spawn a small host process that links Wasmtime and exposes the same lifecycle protocol as other runtimes.
  - Implement resource limits: memory cap, CPU/fuel limits (Wasmtime supports fuel), execution timeout (enforced by monitor), and WASI sandboxing (no file/network access unless explicitly allowed).
  - Expose same admin endpoints (`/runtime/start`, `/runtime/stop`, `/runtime/list`) and reuse audit & whitelist controls.

4) Tooling & CI (1-2 weeks)
  - Add a `runtimes/clojure-wasm/` directory with example Clojure sources and build scripts.
  - Add CI job that builds a sample module using ClojureWasm builder and validates the module with the Rust host (unit/integration tests).
  - Provide fallback: when module build or host isn't available, run Babashka-based flows for server-side scripting.

5) Developer UX & docs (ongoing)
  - Document how to build, test, and deploy ClojureWasm modules locally.
  - Provide example modules that implement the expected health/IPC contract.
  - Offer a `make runtimes/clojure-wasm` command and a Docker-based builder to avoid host-toolchain issues.

6) Security review & hardening (1 week)
  - Audit the allowed host functions and WASI capabilities.
  - Add flame/timeout/fuel budgets per module and per-user quotas.
  - Ensure audit logging for module lifecycle events and disallowed actions.

Key implementation choices & rationale:
- Host runtime: use `wasmtime` crate (mature, good Rust integration, fuel API for CPU budgeting, WASI support).
- Execution model: prefer in-process isolates (faster startup) with strict resource controls; fallback to proxy helper process if isolation or tooling requires it.
- Build model: keep module build out-of-band (CI or local builder) and only accept signed or vetted modules in production; during dev allow local path modules under `runtimes/`.

Risks & mitigation:
- ClojureWasm maturity: maintain a Babashka fallback for server scripting while the Wasm path matures.
- Module size/perf: measure and consider caching compiled modules or prewarming.
- Host ABI drift: define and pin a small stable contract for modules and include versioning in module metadata.

Deliverables:
- `bonsai-runtime` support for `clojurewasm` start/stop/list
- Example ClojureWasm modules in `runtimes/clojure-wasm/`
- CI pipeline that builds and validates modules
- Documentation and developer scripts
