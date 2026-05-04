# Clojure & Python Integration Audit Report

**Date:** 2026-05-03  
**Branch:** `audit/clojure-python-integration`  
**Auditor:** GitHub Copilot (Claude Sonnet 4.6)

---

## Python — Status Summary

| File | Feature | Completeness | Sandboxing | Issues |
|------|---------|-------------|------------|--------|
| `runtimes/python/worker.py` | HTTP health worker | Prototype only — `/health` GET, no skill dispatch | Binds `127.0.0.1` only (good); no CPU/mem cap; no venv | P1: No resource limits; no venv/requirements.txt; no version pin |
| `bonsai-runtime/src/lib.rs` → `start_python_worker()` | Spawns `python <script> <port>` | Functional — returns `ProcessController` | None — `python` on PATH executes with full user privileges | P1: No `ulimit`, no `--check`, no path validation on script; assumes `python` not `python3` |
| `bonsai-bot/src/admin_api.rs` → `/runtime/start` (kind=python) | Runtime lifecycle management | Complete — start/stop/list/timeout watchdog | Path jail present (allowed_script_paths); per-user concurrency limit; timeout kill | Good; P1: timeout enforced by watchdog but no CPU/mem cgroup |
| `bonsai-bot/src/admin_api.rs` → `/runtime/create-skill` (language=python) | Write `.py` file to skills dir | Complete — sanitized name, UUID suffix | Writes to `allowed_script_paths[0]/skills` or config dir | Good — no shell injection; P1: written script can import anything once run |
| `bonsai-bot/tests/integration_runtime.rs` | Integration test for python runtime | Functional — guarded by `which::which("python")` | N/A | **P0 FIXED**: was `crate::` instead of `bonsai_bot::` — broke CI |
| `.github/workflows/ci-bb.yml` | CI test runner | Python 3.x installed via `actions/setup-python` | N/A | P2: No version pin — uses `3.x`; tested on both linux and windows |

---

## Clojure — Status Summary

| File | Feature | Completeness | Sandboxing | Issues |
|------|---------|-------------|------------|--------|
| `runtimes/clojure/bb_runner.clj` | Babashka stdin health protocol | Prototype only — responds to "health" on stdin | None — `bb` runs with full user privileges, full filesystem access | P1: No path jail; no network restriction; no version pin |
| `bonsai-runtime/src/lib.rs` → `start_babashka_worker()` | Spawns `bb <script>` | Functional — returns `ProcessController` | None — assumes `bb` on PATH; no sandbox | P1: No resource limits; no bb version check |
| `bonsai-runtime/src/lib.rs` → `start_clojurewasm_worker()` | Runs `.wasm` module | Complete — in-process wasmtime (feature flag) or CLI fallback | Wasmtime fuel budget caps CPU; no network by default in WASI | Good; P1: fuel budget defaults to 1M ops regardless of timeout; memory unbounded in CLI fallback |
| `bonsai-bot/src/admin_api.rs` → `/runtime/start` (kind=babashka) | Runtime lifecycle management | Complete | Path jail + concurrency limit + timeout watchdog | Good; same P1 as Python: no cgroup |
| `bonsai-bot/src/admin_api.rs` → `/runtime/create-skill` (language=babashka/clojure) | Write `.clj` file to skills dir | Complete | Same as Python path | Same as Python |
| `runtimes/clojure-wasm/build-local.sh` | Prerequisites + clone ClojureWasm | CI-only prereq check + clone | N/A | P2: Still references deprecated `wasm32-wasi` in `rustup target add` (non-fatal, just confusing) |
| `runtimes/clojure-wasm/example/build.sh` | Build minimal `.wasm` module | Functional post-fix | N/A | **P0 FIXED**: used `wasm32-wasi` — fails on Rust ≥ 1.78; now tries `wasm32-wasip1` first |
| `.github/workflows/ci-clojurewasm.yml` | Prereq check on linux + windows | Passes | N/A | Was passing before fix |
| `.github/workflows/ci-clojurewasm-full.yml` | Full build + test | **FAILING** pre-fix | N/A | **P0 FIXED**: wasm32-wasi target no longer exists in Rust 1.78+ |
| `.github/workflows/ci-bb.yml` | Build + cargo test (all features) | **FAILING** pre-fix | N/A | **P0 FIXED**: `integration_runtime.rs` used `crate::` in integration test |

---

## Critical Gaps (P0 — fixed in this PR)

1. **`runtimes/clojure-wasm/example/build.sh`** — Hard-coded `wasm32-wasi` target fails on Rust ≥ 1.78 because Rust renamed it to `wasm32-wasip1`. Fixed: try `wasm32-wasip1` first, fall back to `wasm32-wasi` for compatibility with older toolchains.

2. **`bonsai-bot/tests/integration_runtime.rs`** — Used `crate::metrics`, `crate::session`, `crate::admin_api` in an integration test. Integration tests compile as separate crates; they must reference the library via its crate name (`bonsai_bot::`) not `crate::`. Fixed: replaced all `crate::` with `bonsai_bot::`.

---

## Improvements (P1 — should fix)

1. **Python process resource limits** — `start_python_worker()` spawns `python` with no CPU or memory ceiling. On Linux, wrap the `Command` with `ulimit -v 524288` (512 MB virtual) or use cgroups. On Windows, use Job Objects. A runaway skill can OOM the server.

2. **Python binary name** — `Command::new("python")` fails on systems where only `python3` exists (most modern Linux distros). Use `which::which("python3").or_else(|_| which::which("python"))` or resolve at startup.

3. **Python venv / dependency isolation** — `worker.py` uses stdlib only (fine for the prototype), but the `create-skill` endpoint lets arbitrary `.py` files be written and run. There is no venv, no `requirements.txt`, and no pip install step. User-written skills can `import requests` and it will either work or silently fail. Recommend: create a venv per skill or per user.

4. **Babashka version pinning** — CI installs "latest" bb dynamically. `bb_runner.clj` may break on a future bb version. Pin a minimum version check in `start_babashka_worker()` or in the CI manifest.

5. **Babashka filesystem access** — `bb` runs with the spawning process's full filesystem privileges. The path jail in `admin_api.rs` validates the **script location** but not what the script can **access at runtime**. A user-written `.clj` file can `(slurp "/etc/passwd")`. Recommend: spawn bb with a restricted working directory and/or use `bb --config` with a restricted classpath.

6. **Wasmtime fuel budget** — Default fuel is `1_000_000` ops when no timeout is specified. This may be too low for real Clojure workloads or too high for a sandbox. Expose this as a configurable value in `RuntimeLimits`.

7. **Error propagation from Python/bb workers** — `start_python_worker` and `start_babashka_worker` return `Ok(controller)` as soon as `spawn()` succeeds. If the script exits immediately with an error (bad syntax, import error), the caller sees a PID but the process is already dead. Recommend: add a startup health check (e.g., HTTP GET `/health` within 5s for Python, read first line of stdout for bb).

8. **`build-local.sh` still references deprecated target in `rustup target add`** — The `rustup target add wasm32-wasi` at line 32 of `build-local.sh` is not an error (rustup will warn but not fail), but it is confusing. Should be updated to `wasm32-wasip1`.

---

## Nice-to-Have (P2 — future)

1. **Python version pin in CI** — `python-version: '3.x'` is unpinned. Pin to `'3.12'` for reproducible builds.
2. **Babashka binary bundled** — Consider bundling a pinned bb binary in `tools/` or as a Cargo `include` to eliminate the CI download step and guarantee the dev environment matches CI.
3. **ClojureWasm runtime module** — `runtimes/clojure-wasm/` only has a prerequisite checker and example; there is no actual Clojure-compiled `.wasm` skill module. The `start_clojurewasm_worker` path in production is untested end-to-end.
4. **Skill hot-reload** — Currently, a Python or Clojure skill spawned via `create-skill` + `runtime/start` requires a stop/start cycle to pick up edits. A file-watcher that triggers a soft restart would improve developer UX.
5. **Network policy for skills** — Python skills can make arbitrary outbound HTTP calls. No egress filtering is applied. Consider adding an opt-in `"internet"` capability in `SkillManifest.requires` and enforcing it at the process level (e.g., via an HTTP proxy that checks capability grants).

---

## CI Status

| Workflow | Status | Root Cause |
|---|---|---|
| `ClojureWasm CI (no Docker)` — linux | **PASSING** | Prereq-only check, no actual build |
| `ClojureWasm CI (no Docker)` — windows | **PASSING** | Prereq-only check, no actual build |
| `ClojureWasm Full CI` — linux | **FAILING → FIXED** | `example/build.sh` used deprecated `wasm32-wasi` target (removed in Rust 1.78) |
| `CI` (bb/cargo test) — linux | **FAILING → FIXED** | `integration_runtime.rs` used `crate::` instead of `bonsai_bot::` |
| `CI` (bb/cargo test) — windows | **FAILING → FIXED** | Same root cause as linux |

---

## Python Dependency Map

| Component | Depends on Python? | How | Required at runtime? | Fallback if missing |
|---|---|---|---|---|
| `bonsai-runtime` | Yes (optional) | Spawns `python <script>` subprocess | Only if a Python skill is started | `start_python_worker` returns `Err` — admin API returns 500 |
| `bonsai-bot/admin_api` | Yes (optional) | Via bonsai-runtime | Only on `/runtime/start` with `kind=python` | Returns 500 to caller |
| CI (`ci-bb.yml`) | Yes (build-time) | `actions/setup-python@v4` for test tooling | No — test runner only | Build would skip python-dependent steps |
| `runtimes/python/worker.py` | Yes | Is the Python skill itself | Only when explicitly started | N/A |

Python is **not** required at startup. The server boots without it. It is only required when a Python skill is explicitly launched via the admin API.

---

## Clojure Dependency Map

| Component | Depends on Clojure/bb? | How | Required at runtime? | Fallback if missing |
|---|---|---|---|---|
| `bonsai-runtime` | Yes (optional) | Spawns `bb <script>` subprocess | Only if a Babashka skill is started | `start_babashka_worker` returns `Err` — admin API returns 500 |
| `bonsai-runtime` (wasmtime path) | No direct Clojure dep | Runs precompiled `.wasm` via wasmtime or CLI | Only if a clojurewasm skill is started | Falls back to wasmtime CLI if feature disabled |
| CI (`ci-bb.yml`) | Yes | Installs bb from GitHub releases (latest) | Build-time only | Skips bb-dependent tests |
| CI (`ci-clojurewasm.yml`) | Indirect | Prereq check (git clone ClojureWasm repo) | CI only | N/A |
| `runtimes/clojure/bb_runner.clj` | Yes — requires `bb` shebang | Is the Babashka skill prototype | Only when explicitly started | N/A |

`bb` is **not** expected to be bundled — it is assumed to be present on `PATH` at runtime for Babashka skills. `clojurewasm` is **CI-only** for now; the runtime module itself is a compiled `.wasm` artifact, not a live Clojure dependency. A specific bb version is **not enforced** anywhere — this is a P1 gap.
