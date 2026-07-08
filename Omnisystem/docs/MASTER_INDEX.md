# Omnisystem Master Index

The single file to read first. Omnisystem is a complete operating system layer, 7-language compiler ecosystem, 152 core systems, and an AI inference harness — all implemented in the Omni-Languages (Titan, Vera, Helix, Aether, Axiom, Sylva, Nexus). 204 development phases complete. 47,300+ lines of source code.

---

## What Is Omnisystem?

Omnisystem is four things in one:

1. **OS Desktop Environment** — window manager, file manager, terminal, code editor, app launcher, system monitor, and 10 built-in applications running on Windows/Linux/macOS
2. **7-Language Compiler Ecosystem** — OmniCC compiler (10,900+ LOC) with frontends for 7 languages, a shared x86-64/ARM64 backend, a VM runtime, and native bindings to GPU/display/input
3. **152 Core Systems** — enterprise-grade infrastructure: security, cloud sync, analytics, distributed systems, mobile, desktop environment, package manager, neural network framework, and more
4. **OmniHarness AI Layer** — 5-layer polyglot AI system connecting 10+ LLM providers (Claude, GPT-4o, Gemini, Llama, etc.) to the Omni-Languages ecosystem with persistent memory, formal policy verification, and GPU acceleration

All core code is written exclusively in the Omni-Languages. OmniHarness deliberately uses Python/Rust/Clojure as its polyglot integration layer.

---

## Repository Directory Tree

```
Omnisystem/                         — Repo root
├── OmniHarness/                    — AI inference and agent orchestration subsystem
│   ├── kernel/                     — Rust gRPC kernel (event store, model router, vector store, sandbox)
│   ├── orchestrator/               — Python FastAPI REST server + CLI + model adapters + memory
│   ├── clj-orchestrator/           — Clojure orchestrator (HTN planner, patch manager, policy)
│   ├── omni-integration/           — 7 Omni-Language integration files (one per language)
│   ├── gui/                        — ClojureScript re-frame GUI (OmniBar, ThoughtPanel, ModelHub)
│   ├── proto/                      — Protobuf definitions for 7 gRPC services
│   ├── start.ps1                   — One-command startup (kernel + orchestrator)
│   └── .env.example                — Configuration template
├── src/
│   ├── compiler/                   — OmniCC compiler ecosystem
│   │   ├── OmniCC.titan            — Build orchestrator (entry point)
│   │   ├── Linker.titan            — Cross-language linker
│   │   ├── frontend/               — 7 language frontends (lexer + parser per language)
│   │   ├── backend/                — TitanBackend.titan — x86-64 + ARM64 code generation
│   │   ├── runtime/                — OmnisystemRuntime.titan — VM, allocator, GC
│   │   └── native/                 — GpuBindings.helix, InputBindings.titan, DisplayBindings.vera
│   ├── stdlib/                     — Standard libraries for all 7 languages
│   └── systems/                    — 152 core system modules
│       ├── UOSC/                   — Universal Omnisystem Core
│       └── modules/                — All 152 system modules organized by category
├── docs/                           — All documentation (this directory)
│   ├── MASTER_INDEX.md             — This file
│   ├── README.md                   — Top-level navigation
│   ├── OmniHarness/                — AI harness documentation (10 files)
│   ├── 1-Getting-Started/          — Installation, zero-to-build
│   ├── 2-Quick-Guides/             — Common tasks, tips
│   ├── 3-Architecture/             — System design, IPC, window manager
│   ├── 4-Languages/                — 7 language specifications and guides
│   ├── 5-Core-Systems/             — Compiler, runtime, apps, package manager
│   ├── 6-APIs/                     — API reference, build system
│   ├── 7-Deployment/               — Deployment modes (VS Code, standalone, container, WASM, VM, bare metal)
│   ├── 8-Project-Status/           — Completion reports, phase history
│   ├── 9-Development/              — Build milestones, quality standards, VS Code extension guide
│   └── 10-Reference/               — Catch-all technical reference
├── applications/                   — Built Omnisystem applications
├── bin/                            — Compiled CLI tools (omnicc.js, omnicc.ps1, omnicc.cmd)
└── COMPILER_ECOSYSTEM_COMPLETE.md  — Top-level status report
```

---

## The 7 Omni-Languages

Each language is purpose-designed. Every Omnisystem component is written in the appropriate language for its domain.

| Language | Purpose | Source (compiler frontend) | Source (stdlib) | Docs |
|----------|---------|---------------------------|-----------------|------|
| **Titan** | Systems programming — OS kernel, compiler, runtime, performance-critical code | `src/compiler/frontend/TitanFrontend.titan` | `src/stdlib/TitanStdlib.titan` | [docs/4-Languages/TITAN.md](4-Languages/TITAN.md) |
| **Vera** | UI and component development — windows, widgets, themes, layout | `src/compiler/frontend/VeraFrontend.vera` | `src/stdlib/VeraUIStdlib.vera` | [docs/4-Languages/VERA.md](4-Languages/VERA.md) |
| **Helix** | Graphics and GPU — shaders, compute pipelines, 3D rendering | `src/compiler/frontend/HelixFrontend.helix` | `src/stdlib/HelixGraphicsRuntime.helix` | [docs/4-Languages/HELIX.md](4-Languages/HELIX.md) |
| **Aether** | Async and distributed — actors, message passing, distributed systems | `src/compiler/frontend/AetherFrontend.aether` | `src/stdlib/AetherRuntime.aether` | [docs/4-Languages/AETHER.md](4-Languages/AETHER.md) |
| **Axiom** | Formal verification — proofs, theorems, safety contracts | `src/compiler/frontend/AxiomFrontend.axiom` | `src/stdlib/AxiomFormalVerification.axiom` | [docs/4-Languages/AXIOM.md](4-Languages/AXIOM.md) |
| **Sylva** | Machine learning — neural networks, embeddings, ML pipelines | `src/compiler/frontend/SylvaFrontend.sylva` | `src/stdlib/SylvaMachineLearning.sylva` | [docs/4-Languages/SYLVA.md](4-Languages/SYLVA.md) |
| **Nexus** | Responsive design — layout systems, design tokens, breakpoints | `src/compiler/frontend/NexusFrontend.nexus` | `src/stdlib/NexusResponsiveDesign.nexus` | [docs/4-Languages/NEXUS.md](4-Languages/NEXUS.md) |

---

## Compiler Ecosystem

The OmniCC compiler processes all 7 languages through shared infrastructure.

### Compilation Pipeline

```
Source file (.titan / .vera / .helix / .aether / .axiom / .sylva / .nexus)
        │
        ▼
Language Frontend (lexer + parser → AST)
  src/compiler/frontend/{Language}Frontend.{ext}
        │
        ▼
Shared IR (intermediate representation)
        │
        ▼
src/compiler/backend/TitanBackend.titan
  (optimization + x86-64 / ARM64 code generation)
        │
        ▼
src/compiler/Linker.titan
  (cross-language symbol resolution)
        │
        ▼
Native binary / VM bytecode
        │
        ▼
src/compiler/runtime/OmnisystemRuntime.titan
  (VM execution, allocator, garbage collector)
        │
        ├──► src/compiler/native/GpuBindings.helix    → GPU hardware
        ├──► src/compiler/native/InputBindings.titan  → keyboard/mouse
        └──► src/compiler/native/DisplayBindings.vera → window surface
```

### Key Compiler Files

| Component | File | Description |
|-----------|------|-------------|
| Build orchestrator | `src/compiler/OmniCC.titan` | Entry point for all builds |
| Cross-language linker | `src/compiler/Linker.titan` | Symbol resolution across languages |
| Titan frontend | `src/compiler/frontend/TitanFrontend.titan` | Lexer + parser for Titan |
| Aether frontend | `src/compiler/frontend/AetherFrontend.aether` | Lexer + parser for Aether |
| Axiom frontend | `src/compiler/frontend/AxiomFrontend.axiom` | Lexer + parser for Axiom |
| Helix frontend | `src/compiler/frontend/HelixFrontend.helix` | Lexer + parser for Helix |
| Nexus frontend | `src/compiler/frontend/NexusFrontend.nexus` | Lexer + parser for Nexus |
| Sylva frontend | `src/compiler/frontend/SylvaFrontend.sylva` | Lexer + parser for Sylva |
| Vera frontend | `src/compiler/frontend/VeraFrontend.vera` | Lexer + parser for Vera |
| Code generator | `src/compiler/backend/TitanBackend.titan` | x86-64 + ARM64 codegen |
| VM + runtime | `src/compiler/runtime/OmnisystemRuntime.titan` | VM, allocator, GC |
| GPU bindings | `src/compiler/native/GpuBindings.helix` | GPU pipeline native bindings |
| Input bindings | `src/compiler/native/InputBindings.titan` | Keyboard/mouse/gamepad |
| Display bindings | `src/compiler/native/DisplayBindings.vera` | Window manager native surface |

### Standard Libraries

| File | Language | What It Provides |
|------|----------|-----------------|
| `src/stdlib/TitanStdlib.titan` | Titan | Collections, I/O, networking, crypto, concurrency, 3000+ functions |
| `src/stdlib/VeraUIStdlib.vera` | Vera | 40+ widgets, 6 themes, layout primitives, animation, ARIA |
| `src/stdlib/HelixGraphicsRuntime.helix` | Helix | Shader library, render passes, mesh operations, post-processing |
| `src/stdlib/AetherRuntime.aether` | Aether | Actor system, message bus, distributed consensus, event sourcing |
| `src/stdlib/AxiomFormalVerification.axiom` | Axiom | Proof tactics, SMT solver integration, refinement types |
| `src/stdlib/SylvaMachineLearning.sylva` | Sylva | Neural network layers, optimizers, data pipelines, model zoo |
| `src/stdlib/NexusResponsiveDesign.nexus` | Nexus | Grid system, flex layout, design tokens, breakpoint utilities |

---

## OmniHarness — AI Layer

OmniHarness is a 5-layer polyglot AI system. It lives at `OmniHarness/` in the repo root.

### 5 Layers and Where They Live

| Layer | Technology | Directory |
|-------|-----------|-----------|
| 1 — Rust Kernel | Rust + Wasmtime + libp2p | `OmniHarness/kernel/src/` |
| 2 — Python Orchestrator | Python + FastAPI | `OmniHarness/orchestrator/omniharness/` |
| 3 — Clojure Orchestrator | Clojure | `OmniHarness/clj-orchestrator/src/omniharness/` |
| 4 — Omni-Languages Integration | Titan/Aether/Sylva/Axiom/Vera/Nexus/Helix | `OmniHarness/omni-integration/` |
| 5 — ClojureScript GUI | ClojureScript + re-frame | `OmniHarness/gui/src/omniharness/` |

### Omni-Integration Files

| File | Language | Role |
|------|----------|------|
| `OmniHarness/omni-integration/HarnessCore.titan` | Titan | REST bridge to HTTP API |
| `OmniHarness/omni-integration/ModelBridge.aether` | Aether | Async actor-based model routing |
| `OmniHarness/omni-integration/MemoryLayer.sylva` | Sylva | ML semantic memory (5 layers) |
| `OmniHarness/omni-integration/PolicyVerifier.axiom` | Axiom | 9 formal safety theorems |
| `OmniHarness/omni-integration/HarnessUI.vera` | Vera | Chat UI components (ChatPanel, ThoughtPanel, etc.) |
| `OmniHarness/omni-integration/HarnessLayout.nexus` | Nexus | Responsive chat layout design tokens |
| `OmniHarness/omni-integration/GPUAcceleration.helix` | Helix | 5 GPU compute pipelines for embeddings + inference |

### How to Start OmniHarness

```powershell
# One command — starts Rust kernel (gRPC :50051) + Python orchestrator (HTTP :8000)
OmniHarness/start.ps1

# First message
omniharness chat "hello"

# Use a specific model
omniharness chat --model anthropic/claude-opus-4-5 "Explain quantum computing"
```

### Supported AI Providers

Anthropic (Claude), OpenAI (GPT-4o, o1, o3), Google (Gemini 1.5/2.0), Groq, Mistral, Cohere, OpenRouter, Together AI, Fireworks AI, Ollama (local)

### OmniHarness Documentation

| Doc | Path | What It Covers |
|-----|------|---------------|
| Overview | [docs/OmniHarness/README.md](OmniHarness/README.md) | What OmniHarness is, 5-layer architecture |
| Quick Start | [docs/OmniHarness/QUICKSTART.md](OmniHarness/QUICKSTART.md) | Zero to first AI response |
| Architecture | [docs/OmniHarness/ARCHITECTURE.md](OmniHarness/ARCHITECTURE.md) | Merkle chain, gRPC, ReAct, vector store |
| Models | [docs/OmniHarness/MODELS.md](OmniHarness/MODELS.md) | All providers, context windows, capabilities |
| REST API | [docs/OmniHarness/API.md](OmniHarness/API.md) | All endpoints with curl examples |
| CLI | [docs/OmniHarness/CLI.md](OmniHarness/CLI.md) | All commands and flags |
| Integration | [docs/OmniHarness/INTEGRATION.md](OmniHarness/INTEGRATION.md) | Using OmniHarness from Titan/Aether/Sylva code |
| Configuration | [docs/OmniHarness/CONFIGURATION.md](OmniHarness/CONFIGURATION.md) | All environment variables + .env.example |
| Kernel | [docs/OmniHarness/KERNEL.md](OmniHarness/KERNEL.md) | Rust kernel internals, all source files |
| Memory | [docs/OmniHarness/MEMORY.md](OmniHarness/MEMORY.md) | Episodic, vector, knowledge graph |

---

## The 152 Core Systems

Core systems live in `src/systems/modules/`. Major phase groups:

| Phases | Category | Key Systems |
|--------|----------|-------------|
| 22–23 | Cloud & Sync / Mobile | Cloud storage, sync engine, mobile runtimes |
| 26 | Security | Auth, encryption, audit, intrusion detection |
| 27 | Cloud Infrastructure | CDN, multi-region, load balancing |
| 28 | Analytics | Metrics, dashboards, alerting, reporting |
| 29 | Language Expansion | Additional language runtimes, web frameworks |
| 32–35 | Desktop Environment | Window manager, file manager, app launcher, system config |
| 57–76 | Enterprise Infrastructure | 20 systems for stability, traffic, data, network, operations |
| 77–152 | Complete Coverage | All remaining systems, all production-ready |
| 201–204 | Final Capstone | Deployment, testing, documentation, support |

See [docs/8-Project-Status/](8-Project-Status/) for detailed per-phase reports.

---

## How Everything Connects

```
Developer writes Omni-Language source
        │
        ▼
OmniCC.titan (build orchestrator)
  src/compiler/OmniCC.titan
        │
        ├── Language Frontend (per .{ext} file)
        │   src/compiler/frontend/{Lang}Frontend.{ext}
        │         │ AST
        │         ▼
        ├── TitanBackend.titan (codegen)
        │   src/compiler/backend/TitanBackend.titan
        │         │ object files
        │         ▼
        ├── Linker.titan
        │   src/compiler/Linker.titan
        │         │ binary
        │         ▼
        └── OmnisystemRuntime.titan (execution)
            src/compiler/runtime/OmnisystemRuntime.titan
                  │
                  ├── GpuBindings.helix → GPU
                  ├── InputBindings.titan → input devices
                  ├── DisplayBindings.vera → display/window
                  │
                  └── HarnessCore.titan → OmniHarness HTTP :8000
                                                │
                                         Python Orchestrator
                                                │
                                         Rust Kernel gRPC :50051
                                                │
                                      ┌─────────┴─────────┐
                                  Anthropic    OpenAI    Ollama ...
```

---

## Quick Answers

### How do I build?

```powershell
# Build a Titan file
bin/omnicc.ps1 build src/myfile.titan

# Full system build
bin/omnicc.ps1 build --all

# See all build commands
bin/omnicc.ps1 --help
```

See [docs/5-Core-Systems/Compiler/](5-Core-Systems/Compiler/) and [docs/6-APIs/BUILD_SYSTEM.md](6-APIs/BUILD_SYSTEM.md).

### How do I use an AI model?

```powershell
# From the CLI
omniharness chat --model anthropic/claude-sonnet-4-5 "hello"

# From Titan code
import omniharness.HarnessCore;
let harness = HarnessCore.connect();
println!(harness.chat("hello").text);
```

See [docs/OmniHarness/QUICKSTART.md](OmniHarness/QUICKSTART.md).

### Where is X?

| What | Where |
|------|-------|
| Language specs | [docs/4-Languages/](4-Languages/) |
| Compiler pipeline | [docs/5-Core-Systems/Compiler/](5-Core-Systems/Compiler/) |
| Build commands | [docs/6-APIs/BUILD_SYSTEM.md](6-APIs/BUILD_SYSTEM.md) |
| OmniHarness overview | [docs/OmniHarness/README.md](OmniHarness/README.md) |
| AI model list | [docs/OmniHarness/MODELS.md](OmniHarness/MODELS.md) |
| REST API | [docs/OmniHarness/API.md](OmniHarness/API.md) |
| CLI reference | [docs/OmniHarness/CLI.md](OmniHarness/CLI.md) |
| OmniCC source | `src/compiler/OmniCC.titan` |
| Titan stdlib | `src/stdlib/TitanStdlib.titan` |
| 10 desktop apps | [docs/5-Core-Systems/OMNIOS_APPS.md](5-Core-Systems/OMNIOS_APPS.md) |
| Package manager | [docs/5-Core-Systems/OMNIPM.md](5-Core-Systems/OMNIPM.md) |
| Deployment modes | [docs/7-Deployment/DEPLOYMENT_MODES.md](7-Deployment/DEPLOYMENT_MODES.md) |
| IPC protocol | [docs/3-Architecture/IPC_PROTOCOL.md](3-Architecture/IPC_PROTOCOL.md) |
| Window manager | [docs/3-Architecture/WINDOW_MANAGER.md](3-Architecture/WINDOW_MANAGER.md) |
| Project statistics | [docs/README.md](README.md) |

---

## Project Statistics

| Metric | Value |
|--------|-------|
| Total source LOC | 47,300+ |
| Compiler ecosystem LOC | 10,900+ |
| Documentation files | 695+ |
| Languages | 7 (Titan, Vera, Helix, Aether, Axiom, Sylva, Nexus) |
| Core systems | 152 |
| Development phases completed | 204 |
| AI providers supported | 10 |
| Platforms | Windows, Linux, macOS |
| Architectures | x86-64, ARM64 |
| Status | Production ready |

---

**Last Updated:** July 2026 | **Version:** 2.0 | **Status:** ALL 204 PHASES COMPLETE
