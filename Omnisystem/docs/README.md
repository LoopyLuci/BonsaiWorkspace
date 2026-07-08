# Omnisystem Documentation

> **New to the repo?** Read **[MASTER_INDEX.md](MASTER_INDEX.md)** first — it maps the entire codebase in one file.

Welcome to the complete Omnisystem documentation. The platform includes a 7-language compiler ecosystem, 204 completed development phases, 47,300+ lines of Omni-Language code, and **OmniHarness** — a polyglot AI harness that connects Omnisystem to every major AI model.

---

## Navigation

### [MASTER_INDEX.md](MASTER_INDEX.md) — Start Here
Single file that orients any user or agent to the entire repository: directory layout, all 7 languages, compiler pipeline, OmniHarness, 152 systems, quick answers to common questions.

---

### 0. [OmniHarness](OmniHarness/) — AI Integration Layer ✨
Use any AI model from Omnisystem — locally or via API.

- **[Quick Start](OmniHarness/QUICKSTART.md)** — First AI response in under 5 minutes
- **[Architecture](OmniHarness/ARCHITECTURE.md)** — 5-layer polyglot design: Rust kernel + Python + Clojure + Omni-Languages + ClojureScript GUI
- **[All Models](OmniHarness/MODELS.md)** — Every supported provider and model with context windows and capabilities
- **[REST API](OmniHarness/API.md)** — Complete endpoint reference with curl examples
- **[CLI Reference](OmniHarness/CLI.md)** — `omniharness chat`, `agent`, `models`, `health`, `serve`, `remember`, `recall`
- **[Configuration](OmniHarness/CONFIGURATION.md)** — Every environment variable, .env setup
- **[Omni-Languages Integration](OmniHarness/INTEGRATION.md)** — Using OmniHarness from Titan, Aether, Sylva, Axiom, Vera, Nexus, Helix
- **[Rust Kernel](OmniHarness/KERNEL.md)** — gRPC server, Merkle chain, vector store, WASM sandbox, P2P mesh
- **[Memory Systems](OmniHarness/MEMORY.md)** — Episodic memory, vector search, knowledge graph

---

### 1. [Getting Started](1-Getting-Started/) — First Steps
- Installation and setup
- Quick start guide
- First project walkthrough
- **[Zero to Building](1-Getting-Started/OMNIOS_ZERO_TO_BUILD.md)** — Complete beginner guide: welcome screen, first build, all 10 apps explained

### 2. [Quick Guides](2-Quick-Guides/) — Essential How-Tos
- Common tasks and best practices
- Tips & tricks

### 3. [Architecture](3-Architecture/) — System Design
- **[OmniOS Desktop Architecture](3-Architecture/OMNIOS_DESKTOP_ARCHITECTURE.md)** — 5-layer system design, deployment modes, data flow
- **[IPC Protocol](3-Architecture/IPC_PROTOCOL.md)** — JSON-RPC 2.0: every command, field, error code
- **[Window Manager](3-Architecture/WINDOW_MANAGER.md)** — Z-order, snap zones, persistence, keyboard navigation

### 4. [Languages](4-Languages/) — Programming Languages
All 7 Omni-Languages — every program in Omnisystem's core is written in one of these:

- **[TITAN](4-Languages/TITAN/)** — Systems programming, I/O, hardware
- **[VERA](4-Languages/VERA/)** — UI components, window management
- **[HELIX](4-Languages/HELIX/)** — GPU shaders, graphics pipelines
- **[AETHER](4-Languages/AETHER/)** — Actor model, async, distributed systems
- **[AXIOM](4-Languages/AXIOM/)** — Formal verification, theorem proving
- **[SYLVA](4-Languages/SYLVA/)** — Machine learning, neural networks
- **[NEXUS](4-Languages/NEXUS/)** — Responsive design, layout systems

### 5. [Core Systems](5-Core-Systems/) — Engine & Runtime
- **[Compiler](5-Core-Systems/Compiler/)** — OmniCC pipeline: frontends, IR, backend, linker
- **[Runtime](5-Core-Systems/Runtime/)** — VM, allocators, GC, event loop, thread scheduler
- **[OmniOS Apps](5-Core-Systems/OMNIOS_APPS.md)** — All 10 built-in apps with full specifications
- **[OmniPM Package Manager](5-Core-Systems/OMNIPM.md)** — Package format, registry API, security audit

### 6. [APIs](6-APIs/) — Reference Documentation
- **[Build System Reference](6-APIs/BUILD_SYSTEM.md)** — OmniCC commands, BUILD.omnisystem format, build phases
- Core API reference, function documentation, type definitions

### 7. [Deployment](7-Deployment/) — Operations & Infrastructure
- **[Deployment Modes](7-Deployment/DEPLOYMENT_MODES.md)** — 6 modes: VS Code Extension, Standalone App, Container, WASM Browser, VM Image, Bare Metal
- CI/CD pipelines, infrastructure setup

### 8. [Project Status](8-Project-Status/) — Reports & History
- Phase completion reports (204 phases)
- Development history and build progress

### 9. [Development](9-Development/) — For Contributors
- **[Build Milestones](9-Development/BUILD_MILESTONES.md)** — 9 sequenced milestones
- **[Quality Standards](9-Development/QUALITY_STANDARDS.md)** — Performance, reliability, security, coverage targets
- **[VS Code Extension Guide](9-Development/VSCODE_EXTENSION_GUIDE.md)** — Architecture, message routing, adding new apps

### 10. [Reference](10-Reference/) — Technical Reference
- Widget catalog, component reference, asset systems
- API reference, configuration options, data formats

---

## Reading Order

**If you want to use AI models → AI agent:**
1. [OmniHarness/QUICKSTART.md](OmniHarness/QUICKSTART.md)
2. [OmniHarness/CLI.md](OmniHarness/CLI.md)
3. [OmniHarness/MODELS.md](OmniHarness/MODELS.md)

**If you're new to the project:**
1. [MASTER_INDEX.md](MASTER_INDEX.md)
2. [1-Getting-Started/](1-Getting-Started/)
3. [4-Languages/TITAN/](4-Languages/TITAN/)

**If you're building with Omnisystem:**
1. [3-Architecture/](3-Architecture/)
2. [5-Core-Systems/Compiler/](5-Core-Systems/Compiler/)
3. [6-APIs/BUILD_SYSTEM.md](6-APIs/BUILD_SYSTEM.md)

**If you're deploying:**
1. [7-Deployment/DEPLOYMENT_MODES.md](7-Deployment/DEPLOYMENT_MODES.md)
2. [OmniHarness/CONFIGURATION.md](OmniHarness/CONFIGURATION.md)

---

## Project Statistics

| Metric | Value |
|---|---|
| Core LOC (Omni-Languages) | 47,300+ |
| OmniHarness LOC | 8,000+ |
| Development phases | 204 complete |
| Omni-Languages | 7 |
| AI providers supported | 10 |
| Core system domains | 178 |
| Documentation files | 700+ |
| Target platforms | Windows, Linux, macOS |
| Target architectures | x86-64, ARM64 |

---

## Status: PRODUCTION READY ✅

- ✅ 7-language compiler ecosystem complete
- ✅ Runtime VM, allocator, GC, event loop operational
- ✅ 204 development phases complete
- ✅ OmniHarness AI layer — 10 providers, local + API
- ✅ Cross-platform deployment (6 modes)
- ✅ Enterprise-grade security (Axiom-verified policies)
- ✅ GPU acceleration (Helix compute pipelines)

---

**Last Updated:** July 2026 | **Version:** 2.0 | **Status:** COMPLETE + AI-ENABLED
