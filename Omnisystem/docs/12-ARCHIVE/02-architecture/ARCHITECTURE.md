# Omnisystem Architecture

## Overview

The Omnisystem is a complete rebuilding of the Bonsai Ecosystem using four Omni-languages:

1. **Titan**: Systems language (linear types, effects, self-hosting)
2. **Sylva**: Scripting language (dynamic + gradual static, JIT)
3. **Aether**: Actor language (location-transparent, CRDT-native, supervision)
4. **Axiom**: Proof language (dependent types, code extraction)

All code in the Omnisystem follows strict naming conventions and architectural patterns to ensure consistency, verifiability, and maintainability.

---

## Naming Conventions

### Crate/Service Names
- **Pattern**: `build-{service-name}`
- **Examples**: `build-p2p`, `build-compress`, `build-ai`, `build-observability`
- **All lowercase, hyphenated**

### Module Names
- **Pattern**: `build::{service_name}` (snake_case)
- **Examples**: `build::p2p`, `build::compress`, `build::container`

### USOS Kernel Structures
- **Pattern**: `usos_{component}_{subcomponent}`
- **Examples**: 
  - `usos_process`
  - `usos_memory_region`
  - `usos_capability_token`
  - `usos_ipc_channel`
  - `usos_scheduler`

### Configuration Files
- **Pattern**: `build.toml` at workspace root
- **Language-specific**: `titan.toml`, `sylva.toml`, `aether.toml`

### Build Artifacts
- **Pattern**: `build-{name}.{ext}`
- **Examples**: `build-kernel.bin`, `build-system.img`, `build-services.tar`

---

## The Four Omni-Languages

### 1. Titan (Systems Language)

**Purpose**: Core system components, performance-critical code, standard libraries

**Key Features**:
- Linear types with ownership system (all resources: memory, file handles, sockets, GPU buffers)
- Algebraic effect system (`io`, `alloc`, `net`, `fail`, `async`)
- Self-hosting compiler
- Multiple compilation targets:
  - Native (x86-64, ARM64, RISC-V via Cranelift)
  - WebAssembly (WASI)
  - GPU (CUDA, ROCm, SPIR-V)
- Deterministic execution (no garbage collector; all cleanup is compile-time)

**Primary Uses**:
- USOS kernel
- build-p2p (crypto core)
- build-compress
- build-container
- build-compiler (core)
- build-observability
- build-ai (core inference)
- build-qa (static analysis)
- Standard library

**Standard Library Structure**:
```
titan::core              # No-std: mem, ptr, sync primitives
titan::io                # File, socket, console I/O
titan::sync              # Mutex, RwLock, Condvar, Atomic
titan::fs                # Filesystem abstraction
titan::net               # TCP, UDP, Domain sockets
titan::collections       # Vec, HashMap, BTreeMap, etc.
titan::thread            # Threading primitives
titan::simd              # SIMD operations
titan::alloc             # Memory allocator interface
```

---

### 2. Sylva (Scripting Language)

**Purpose**: Rapid development, dynamic queries, interactive tools

**Key Features**:
- Dynamic typing with optional gradual static typing (`strict` mode)
- JIT compilation to native code
- Time-travel debugging (state snapshots, rewind execution)
- REPL with advanced features (inline docs, auto-complete)
- Interop with Titan libraries (automatic type marshaling)
- JSON/YAML/TOML handling built-in

**Primary Uses**:
- build-bot scripting
- build-compiler frontend
- build-knowledge query language
- Configuration and scripting

**Key Bindings**:
```
sylva::titan              # Marshal between Sylva dynamic and Titan static
sylva::http               # HTTP client/server
sylva::json               # JSON parsing/generation
sylva::async              # Async/await backed by Aether
sylva::repl               # Interactive development
```

---

### 3. Aether (Actor Language)

**Purpose**: Distributed concurrency, message passing, large-scale coordination

**Key Features**:
- Location-transparent actors (local or remote via build-p2p)
- CRDT-native state (GCounter, PNCounter, ORSet, LWWRegister)
- Supervision trees (Erlang-style with restart strategies)
- Distributed persistence (to build-observability)
- Automatic message serialization
- Consistent hashing for actor placement (scales to 10,000+ nodes)

**Primary Uses**:
- build-p2p (P2P mesh)
- build-media (streaming actors)
- build-ai (distributed inference)
- build-observability (log collection)
- Any distributed service

---

### 4. Axiom (Proof Language)

**Purpose**: Formal verification of critical properties, code extraction

**Key Features**:
- Full dependent type system (like Lean 4)
- Inductive types, pattern matching, tactic automation
- Code extraction to Titan and Sylva
- Small kernel (~2000 lines) for trustworthiness
- LSP server with live proof checking
- Integrated into UBVM

**Primary Uses**:
- USOS kernel verification (memory safety, scheduler correctness)
- Protocol verification (build-p2p handshake, etc.)
- Data structure proofs (no overflow, correct invariants)
- Constraint verification (build-ai safety properties)

---

## USOS Core Architecture

The USOS kernel is minimal (< 5000 lines of Titan), providing only essential primitives.

### Kernel Components

1. **Memory Manager** (`usos_memory`)
   - Physical memory allocation (frame allocator)
   - Virtual memory with paging
   - Copy-on-write
   - Capability-based regions

2. **Scheduler** (`usos_scheduler`)
   - Preemptive multitasking
   - Priority queues
   - EDF (Earliest Deadline First) for real-time
   - Deterministic scheduling order

3. **IPC** (`usos_ipc`)
   - Synchronous message passing (rendezvous)
   - Asynchronous message passing (buffered)
   - Both use capabilities for authorization

4. **Capabilities** (`usos_capability`)
   - Unforgeable tokens for all resources
   - Fine-grained: separate caps for read/write/execute
   - Revocable and delegable

5. **Boot & Service Manager** (`usos_boot`)
   - Load initial userspace image from CAS
   - Start build-service-manager
   - Initialize logging and audit

### What Is NOT in USOS Core

- Filesystems (userspace service)
- Networking stack (userspace build-p2p)
- Device drivers (userspace services)
- GUI / windowing
- Compression, encryption
- Database, blockchain
- Language runtimes

---

## Service Architecture

### Core Service Tiers

**Tier 1: Essential Infrastructure**
- build-p2p (networking)
- build-observability (logging/telemetry)
- build-container (process isolation)
- build-compress (compression library)

**Tier 2: System Services**
- build-vfs (filesystem)
- build-enclave (runtime management)
- build-compiler (compilation service)

**Tier 3: Business Logic**
- build-ai (inference)
- build-knowledge (semantic search)
- build-media (multimedia)
- build-bot (chat bridges)
- build-qa (quality assurance)

**Tier 4: Optional**
- build-blockchain (Nexus replacement)

---

## Build System (build command)

The `build` binary (written in Titan) provides:

```
build build [target]      # Build specific service or entire system
build test [suite]        # Run UBVM tests
build run [service]       # Run service or USOS kernel
build package [name]      # Create deployment package
build repl [lang]         # Start REPL (Sylva/Titan)
build verify [component]  # Run Axiom proofs
build clean               # Remove build artifacts
build status              # Show build status
```

---

## Design Principles

1. **Minimalism**: Only essential functionality in the kernel; everything else is userspace
2. **Capability-based**: All access control via unforgeable capabilities
3. **Determinism**: All I/O and randomness is tracked and reproducible
4. **Formal Verification**: Critical code has Axiom proofs
5. **Self-Hosting**: The system builds itself
6. **Content-Addressed**: All dependencies identified by BLAKE3 hash
7. **Distributed by Default**: Aether actors can span 10,000+ nodes
8. **Interop**: Seamless communication across Omni-languages via effects and IPC

---

---

## Implementation Notes

The USOS kernel (`kernel/capability.ti`, `kernel/memory.ti`, `kernel/scheduler.ti`)
and all ten platform services (`services/build-*/`) are fully implemented in Titan and
pass their test suites. The build tool is `titan-bootstrap/output/titan-compiler.exe`.

**Document Version**: 1.1  
**Last Updated**: 2026-06-04
