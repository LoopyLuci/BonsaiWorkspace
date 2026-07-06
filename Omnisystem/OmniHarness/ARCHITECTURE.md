# OmniHarness Architecture

## Overview

OmniHarness is a polyglot AI harness spanning Rust, Python, Clojure, TypeScript/Svelte, and (partially) the Omni-Languages. It is **not one monolithic app** — it's several independently-launchable products and services that share a common gRPC substrate (the kernel) and, where it makes sense, cross-mirror their audit trails into it. This document reflects what is actually wired together as of the 2026-07 integration pass, not an aspirational target.

```
┌───────────────────────────────────────────────────────────────────────────┐
│                              OmniHarness                                  │
│                                                                             │
│  ┌──────────────┐        ┌───────────────────┐      ┌───────────────────┐ │
│  │  Rust Kernel │◄──────►│Python Orchestrator│◄────►│Clojure Orchestrator│ │
│  │ (gRPC :50051)│  gRPC  │   (FastAPI :8080) │ HTTP │  (HTTP API :8090) │ │
│  │              │        │                   │      │                   │ │
│  │ EventStore   │        │ ModelRouter       │      │ HTNPlanner        │ │
│  │ VectorStore  │        │ ReActEngine       │      │ PolicyEngine      │ │
│  │ SessionStore │        │ Substrate (swarm/ │      │ ReactEngine       │ │
│  │ ToolRegistry │        │  ensemble/gov.)   │      │ EventStore client │ │
│  │ AuthStore    │        │ EpisodicMemory    │      │                   │ │
│  │ Sandbox      │        │ KnowledgeGraph    │      └───────────────────┘ │
│  │ MeshNode     │        └─────────┬─────────┘                           │
│  └──────┬───────┘                  │ HTTP                                │
│         │ gRPC                     ▼                                     │
│         │              ┌───────────────────┐                             │
│         │              │ vscode-omnisystem │  (VS Code extension)        │
│         │              │  OmniHarnessClient│  — chat panel, MCP client,  │
│         │              │  MCP client+server│    "Launch Workspace IDE"   │
│         │              └───────────────────┘    spawns the app below     │
│         │                                                                 │
│         ▼                                                                 │
│  ┌───────────────────────────────────┐                                   │
│  │  Workspace (Tauri + Svelte, the   │  — standalone desktop app, its    │
│  │  flagship desktop IDE/agent app)  │    own model routing/swarm/audit; │
│  │  smart_router · swarm_orchestrator│    mirrors audit + swarm events   │
│  │  kernel_bridge · orchestrator_    │    into the kernel's event store  │
│  │  bridge (cloud model visibility)  │    (module "workspace-*")         │
│  └───────────────────────────────────┘                                   │
│                                                                             │
│  ┌──────────────────────────────────────────────────────────────────┐    │
│  │  gui/ — ClojureScript (shadow-cljs + Reagent), a THIRD, separate  │    │
│  │  browser UI hitting the Python orchestrator's REST API directly. │    │
│  │  Not wired to workspace or vscode-omnisystem; genuinely builds    │    │
│  │  (verified 2026-07) but has no unique feature over the other two.│    │
│  └──────────────────────────────────────────────────────────────────┘    │
│                                                                             │
│  ┌──────────────────────────────────────────────────────────────────┐    │
│  │  omni-integration/ — 18 spec files across 7 Omni-Languages.       │    │
│  │  Only the 5 .titan files have a real execution path (parsed/     │    │
│  │  checked by Omnisystem/bootstrap's interpreter). The other 13     │    │
│  │  (.helix/.nexus/.vera/.aether/.axiom/.sylva) have NO interpreter  │    │
│  │  anywhere in this repo — they are design specs, not running code. │    │
│  └──────────────────────────────────────────────────────────────────┘    │
└───────────────────────────────────────────────────────────────────────────┘
```

---

## Components

### 1. Rust Kernel (`kernel/`)

The cross-language trust anchor: hash-chained event store + model registry, gRPC-only. A separate, independently-launched process — everything that talks to it (orchestrator, clj-orchestrator, workspace) degrades gracefully when it isn't running.

| Module | Responsibility |
|---|---|
| `event_store.rs` | Immutable SHA-256 Merkle chain event log, JSONL-persisted |
| `model_router.rs` | Provider inference + HTTP calls to AI providers |
| `vector_store.rs` | Cosine similarity search, hash embeddings |
| `sandbox.rs` | Wasmtime WASM execution, fuel + memory limits |
| `tool_registry.rs` | Built-in tools: file I/O, HTTP, calculator, list_dir |
| `auth.rs` | Capability-scoped API key store (SHA-256 hashed) |
| `grpc_server.rs` | tonic gRPC server, 6 services from `proto/omniharness.proto`, plus an **opt-in auth interceptor** (see Security Model) |
| `mesh.rs` | libp2p gossipsub P2P event replication, mDNS discovery |
| `main.rs` | Boot orchestration, graceful shutdown |

**Port:** gRPC on `[::1]:50051` by default (IPv6 loopback — override with `GRPC_ADDR`). Clients should dial `localhost:50051`, not a literal `127.0.0.1`, so the OS resolver picks whichever address family the kernel actually bound; this bit workspace's own bridge during development (see kernel_bridge.rs's comment).

### 2. Python Orchestrator (`orchestrator/`)

The richest-SDK-coverage integration layer and the **only thing vscode-omnisystem talks to**. Also hosts the "substrate" — swarm/ensemble/governance/RAG/evolution engines, independent of workspace's own swarm implementation.

| Module | Responsibility |
|---|---|
| `models/router.py` | ModelRouter with adapters for ~9 cloud providers + local runtime autodiscovery |
| `react/engine.py` | ReAct loop: text-format + native function calling |
| `substrate/governance.py` | Budget/CapabilityPolicy/AuditLog/KillSwitch — the Governor every substrate run executes inside |
| `substrate/swarm.py` | pipeline/parallel/orchestrator-workers/debate topologies |
| `memory/{vector,episodic,graph}.py` | Vector store, SQLite episodic history, knowledge graph |
| `grpc_client.py` | Lazy gRPC client to the kernel — supports `OMNIHARNESS_ADMIN_KEY` metadata for the kernel's opt-in auth |
| `clj_client.py` | HTTP client to clj-orchestrator's planner/policy API — same graceful-degradation contract |
| `server.py` | FastAPI REST; `/api/health` reports both `kernel` and `clj_orchestrator` connectivity; `/api/planner/*` proxies to clj-orchestrator |

**Port:** REST on `:8080`.

### 3. Clojure Orchestrator (`clj-orchestrator/`)

HTN planner, policy engine, ReAct loop — genuinely wired in as of 2026-07 (previously fully built but never started or called by anything).

| Namespace | Responsibility |
|---|---|
| `client.clj` | mount-managed gRPC channel + 6 service stubs to the kernel; attaches `x-omniharness-key` metadata when `OMNIHARNESS_ADMIN_KEY` is set |
| `events.clj` | Event append/verify/query against the kernel's Merkle chain |
| `policy.clj` | Compiled policy rules, capability-based, default-deny |
| `planner.clj` | HTN planner with sequential execution |
| `react_engine.clj` | Async ReAct loop via core.async |
| `http_server.clj` | **New** — real HTTP API (`/health`, `/plan`, `/plan/execute`, `/policy/check`, `/kernel/verify`) via http-kit/compojure/ring, started by `lein run serve` |

**Port:** HTTP on `:8090` (`CLJ_HTTP_PORT`). Started by `start.ps1` when a working `lein` is on PATH (the script verifies `lein --version` actually succeeds, not just that a `lein` shim exists — a broken/incomplete self-install shadowing a working one is a real failure mode seen in development).

### 4. Workspace — the flagship desktop app (`workspace/`)

A Tauri v2 + Svelte desktop IDE/agent app — the actual product individuals run day to day. Entirely independent of the orchestrator/clj-orchestrator (its own model routing, its own multi-agent swarm engine, its own audit log), but bridged into the kernel:

| Module | Responsibility |
|---|---|
| `smart_router.rs` | Hardware-aware local model routing (independent of `orchestrator/models/router.py` — a deliberate, not-yet-unified duplication; see `orchestrator_bridge.rs`) |
| `swarm_orchestrator.rs` | Real multi-agent LLM dispatch (leader/worker, retries, delegation) |
| `swarm_commander_bridge.rs` | Mirrors real swarm lifecycle events into `crates/swarm`'s hierarchy/ledger/DAG (that crate's own execution engine is a stub — this makes it an observability layer over the real engine instead of a second, fake one) and into the kernel's event store (module `workspace-swarm`) |
| `kernel_bridge.rs` | gRPC client to the kernel — mirrors `assistant_audit_log.rs` into the kernel's event store (module `workspace-assistant`); sends `OMNIHARNESS_ADMIN_KEY` as metadata when set |
| `orchestrator_bridge.rs` | **New** — optional HTTP call to the Python orchestrator's `/api/models`, surfaced in the Agents panel's About tab as read-only visibility into cloud models the orchestrator has configured |
| `kernel_commands.rs` | `kernel_status`/`kernel_list_models` Tauri commands, surfaced live in the UI |

Launched standalone (`cargo tauri dev` / built installer), or from `vscode-omnisystem` via the "Launch Workspace IDE" command (`cmdOpenWorkspaceIde` in `extension.ts`), which spawns it as an external process with a duplicate-launch guard.

### 5. gui/ — ClojureScript browser UI

A third, independent frontend (shadow-cljs + Reagent + re-frame) hitting the Python orchestrator's REST API directly — alongside workspace (Svelte) and vscode-omnisystem (VS Code webviews), which do the same thing for different surfaces. **Was silently broken** (missing `react`/`react-dom` npm peer deps that Reagent requires) until 2026-07; now verified to compile (`npx shadow-cljs compile app`). No unique capability over the other two UIs — kept as a lightweight, editor-independent option (e.g. for a headless/remote box where neither Tauri nor VS Code apply) rather than deleted, since it's real working code, but not a priority for new features.

### 6. Omni-Languages Integration (`omni-integration/`)

18 spec files across 7 Omni-Languages. **Only 5 have any real execution path**: `SubstrateCore.titan`, `HarnessCore.titan`, `ModelLoaderBridge.titan`, `ModuleSystemBridge.titan`, `SmartRouterBridge.titan` — these parse and type-check cleanly against `Omnisystem/bootstrap`'s real Titan interpreter (`npm run check:omni-integration` in that directory; raw-string literals, `extern "C"` blocks, and string line-continuation support were added to the bootstrap lexer/parser specifically so these would). The other 13 files (`GPUAcceleration.helix`, `HarnessLayout.nexus`, `HarnessUI.vera`, `ModelBridge.aether`, `PolicyVerifier.axiom`, `DistillationEngine.sylva`, `MemoryLayer.sylva`, `SubstrateCompute.helix`, `SubstrateGovernance.axiom`, `SubstrateLayout.nexus`, `SubstratePanel.vera`, `SwarmActors.aether`, `TrainingLoopBridge.sylva`) have **no interpreter anywhere in this repo** — they are design specs describing intended behavior, not code that runs. Treat them accordingly; building interpreters for Helix/Nexus/Vera/Aether/Axiom/Sylva is real, substantial, unstarted work, not a documentation gap.

---

## Data Flow

### Chat request path (via orchestrator, e.g. from vscode-omnisystem):
```
VS Code chat panel → OmniHarnessClient → POST /api/chat
  → ModelRouter.route() → provider adapter (Anthropic/OpenAI/Ollama/…)
    → response → grpc.append_event("orchestrator", "ChatComplete", ...) if kernel reachable
```

### Swarm run path (two independent engines, cross-visible via the kernel):
```
Workspace: user selects "Custom Swarm" → swarm_orchestrator.rs dispatches real
  leader/worker LLM calls → swarm_commander_bridge mirrors into crates/swarm's
  hierarchy/ledger/DAG (UI-visible in the Swarm Commander panel) AND into the
  kernel's event store under module "workspace-swarm"

Orchestrator: POST /api/swarm/run → SwarmCoordinator (pipeline/parallel/
  orchestrator/debate) → Governor.audit mirrors into the kernel's event store
  under module "orchestrator-substrate", tagged with a per-run UUID as
  session_id so every event from one run correlates together
```

### Planner path (orchestrator delegating to clj-orchestrator):
```
POST /api/planner/plan → clj_client.py → clj-orchestrator's /plan
  → planner.clj (HTN forward-chaining search) → JSON plan back through both hops
  (503 from the orchestrator if clj-orchestrator isn't running — not a required dependency)
```

---

## Security Model

- API keys SHA-256 hashed at rest (kernel's `auth.json`)
- **gRPC auth is opt-in**: set `OMNIHARNESS_REQUIRE_AUTH=1` on the kernel to enforce the `x-omniharness-key` metadata header on every call (checked in `grpc_server.rs`'s `AuthInterceptor`); default is unenforced, matching a single-user local-first setup. All three clients (workspace's `kernel_bridge.rs`, the Python `grpc_client.py`, and Clojure's `client.clj`) send `OMNIHARNESS_ADMIN_KEY` as that header when the env var is set, whether or not enforcement is on.
- WASM sandbox: fuel + memory limits, WASI allow-list
- Policy engines: default-deny, compiled rules (both `orchestrator/substrate/governance.py` and `clj-orchestrator/policy.clj` independently implement this — not unified)
- Chain integrity: every mirrored event cryptographically linked in the kernel's event store; tampering detectable via `EventStoreService.VerifyChain` / clj-orchestrator's `/kernel/verify`

---

## CI/CD

GitHub Actions workflows at the repo root `.github/workflows/` (previously the only two workflows here pointed at a legacy root `Cargo.toml` referencing ~2500 nonexistent `src/crates/*` members and could never have passed — replaced entirely in 2026-07):

| Workflow | Covers |
|---|---|
| `rust-omniharness.yml` | kernel + workspace/src-tauri: check, test, fmt/clippy (informational) |
| `orchestrator-python.yml` | pip install, compile, import smoke test, ruff (informational) |
| `clj-orchestrator.yml` | proto codegen + `lein check` |
| `vscode-extension.yml` | typecheck, lint (informational), `.vsix` package artifact |
| `workspace-frontend.yml` | svelte-check, vite build, bundle-size budget (informational — currently over budget, a known pre-existing gap) |
| `titan-bootstrap.yml` | interpreter test suite (informational — 13/16 pass, known gaps) + `check:omni-integration` (hard gate, 5/5 pass) |
| `integration.yml` | **Boots the real kernel + orchestrator + clj-orchestrator together** and asserts the bridges actually connect — automates the manual live verification done during development. Does not drive the Tauri GUI (needs a display + UI driver, a separate investment). |
| `security.yml` | cargo-audit (kernel + workspace), npm audit ×2, pip-audit, CodeQL (JS/TS + Python) — all informational pending a triage pass |
| `release.yml` | On `v*` tag: Windows Tauri installer + VS Code `.vsix`, attached to a draft GitHub Release |

`.github/dependabot.yml` covers cargo (kernel, workspace), npm (vscode-omnisystem, workspace frontend), pip (orchestrator), and github-actions itself, weekly.

**Deliberately Windows-only** for the Rust/Tauri build and release jobs: every native dependency in `workspace/src-tauri` (audio/PTY/GPU/X11-adjacent crates) has only ever been built and verified on Windows — publishing untested macOS/Linux artifacts would be a false promise.

---

## One-Click Boot (`start.ps1`)

Starts orchestrator + kernel + clj-orchestrator + gui together. As of 2026-07: auto-builds the kernel binary and auto-installs orchestrator Python deps if missing (no manual pre-build step needed for a first run), verifies `lein` actually works rather than just existing on PATH, and ends with a real health-check summary (polls each service's actual HTTP endpoint) instead of assuming a fixed sleep was long enough.

```powershell
.\start.ps1                 # everything, auto-building what's missing
.\start.ps1 -NoKernel -NoClj -NoGui   # orchestrator only
```

---

## Supported AI Providers (orchestrator)

| Provider | Models | Auth |
|---|---|---|
| Anthropic | claude-* | x-api-key |
| OpenAI | gpt-*, o1, o3 | Bearer |
| Google | gemini-* | ?key= |
| Cohere | command-* | Bearer |
| Mistral | mistral-* | Bearer |
| Groq | llama-*, mixtral-* | Bearer |
| OpenRouter | user/model | Bearer |
| Together | * | Bearer |
| Fireworks | * | Bearer |
| Ollama / LM Studio / llama.cpp | * | None (local, autodiscovered) |

Provider is inferred from `model_id` prefix. Workspace has an entirely separate local-model routing stack (`smart_router.rs`) that doesn't go through this table at all.
