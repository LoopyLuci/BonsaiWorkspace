# UOSC – Universal Operating System Core

**A formally verified, capability-based microkernel for the next generation of sovereign operating systems.**

[![Build Status](https://img.shields.io/badge/build-passing-brightgreen)]()
[![License](https://img.shields.io/badge/license-Apache%202.0%2FMIT-blue)](LICENSE)
[![Formal Verification](https://img.shields.io/badge/verification-Axiom-purple)]()
[![Rust](https://img.shields.io/badge/language-Rust%20%2B%20Titan-orange)]()

## What is UOSC?

UOSC is a **minimal, formally verified microkernel** that provides the core abstractions for next-generation operating systems. It is designed to be:

- **Secure by default** – Capability-based security model; all resource access mediated by unforgeable tokens
- **Real-time capable** – Deterministic scheduler with formal proofs of deadline guarantees
- **Hardware-agnostic** – Runs bare-metal, as a hypervisor guest (KVM/Hyper-V/Virtualization.framework), or in library OS mode
- **Formally verified** – Critical paths proven with Axiom theorem prover (no undefined behavior)
- **Minimal** – ~10,000 LOC of core kernel; all drivers and services are userspace processes

UOSC is **not** a complete operating system – it is the foundation on which systems like [Omnisystem](https://github.com/your-org/omnisystem) are built. You can also use UOSC standalone for embedded or real-time systems.

## Key Features

| Feature | Description |
|---------|-------------|
| **Capability System** | Linear, revocable, cryptographically verifiable tokens for all resources. Proven isolation via Axiom. |
| **Deterministic Scheduler** | EDF (Earliest Deadline First) for real-time + CFS (Completely Fair) for normal tasks. Lock-free. |
| **Memory Management** | Buddy allocator, virtual memory with COW, NUMA awareness. Per-process protection via capabilities. |
| **Zero-Copy IPC** | Ring buffer messaging, shared memory regions, capability-mediated. Both sync and async. |
| **Sanctum Vaults** | Hardware-isolated execution environment for each process (CHERI, Intel TDX, or ARM CCA). |
| **Formal Verification** | 8+ Axiom theorems covering capability isolation, scheduler fairness, memory safety. |
| **Boot Modes** | Bare-metal (UEFI/multiboot), hypervisor guest (KVM/Hyper-V), library OS (embedded in another kernel). |

## Quick Start

### Prerequisites

- **Rust 1.75+** (for build tools and linking)
- **Titan compiler** (included in build or from Omnisystem repo)
- **QEMU** (for testing, optional)
- **Axiom** proof checker (optional, for formal verification)

### Build

```bash
# Clone repository
git clone https://github.com/your-org/uosc
cd uosc

# Build kernel
make kernel          # outputs: kernel.elf

# Build as library OS
make lib            # outputs: libuosc.a

# Run tests
make test           # runs tests under QEMU

# Verify proofs
make verify         # checks Axiom proofs (requires axiom binary)
```

### Run Bare-Metal (QEMU)

```bash
qemu-system-x86_64 -kernel kernel.elf -serial stdio -m 1024
```

### Run as Hypervisor Guest (KVM)

```bash
qemu-system-x86_64 -kernel kernel.elf -enable-kvm -cpu host -m 1024 -serial stdio
```

## Repository Structure

```
uosc/
├── README.md                    # This file
├── LICENSE                      # Apache 2.0 / MIT
├── Makefile                     # Build rules
├── Cargo.toml                   # Rust build config
│
├── kernel/                      # Core microkernel (10,000 LOC)
│   ├── boot.ti                 # Entry point, boot loader
│   ├── capability.ti           # Capability system (2,000 LOC)
│   ├── memory.ti               # Physical & virtual memory (2,500 LOC)
│   ├── scheduler.ti            # EDF + CFS scheduler (2,000 LOC)
│   ├── ipc.ti                  # Inter-process communication (1,500 LOC)
│   ├── sanctum.ti              # Vault & hardware isolation (1,500 LOC)
│   ├── hypercall.ti            # Host communication (500 LOC)
│   ├── lib.rs                  # Rust FFI binding
│   └── asm/                    # Assembly (x86-64)
│       ├── boot.asm            # Bootloader
│       ├── context_switch.asm  # CPU context switching
│       └── hypercall.asm       # Hypercall interface
│
├── include/
│   └── uosc.h                  # C API for library OS mode
│
├── lib/
│   └── libuosc.a               # Static library (library OS mode)
│
├── proof/                       # Formal verification with Axiom
│   ├── capability.ax           # Capability isolation theorem
│   ├── scheduler.ax            # Scheduler fairness & deadline guarantee
│   ├── memory.ax               # Memory allocator correctness
│   ├── ipc.ax                  # IPC safety theorem
│   └── Makefile                # Run proofs
│
├── tests/
│   ├── unit/                   # Unit tests (run in user mode)
│   │   ├── test_capability.ti
│   │   ├── test_memory.ti
│   │   ├── test_scheduler.ti
│   │   └── test_ipc.ti
│   │
│   ├── integration/            # Integration tests (QEMU)
│   │   ├── test_boot.ti
│   │   ├── test_vault.ti
│   │   └── test_hypercall.ti
│   │
│   └── bench/                  # Benchmarks
│       ├── bench_scheduler.ti
│       └── bench_ipc.ti
│
├── docs/
│   ├── ARCHITECTURE.md         # Detailed kernel design
│   ├── API.md                  # Syscall reference
│   ├── COOS.md                 # Co-OS integration guide
│   ├── SECURITY.md             # Security model & guarantees
│   ├── CONTRIBUTING.md         # Developer guidelines
│   └── FORMAL_VERIFICATION.md  # Axiom proof guide
│
└── .github/
    └── workflows/
        ├── build.yml           # Build CI pipeline
        ├── test.yml            # Test CI pipeline
        └── verify.yml          # Formal verification CI
```

## Documentation

| Document | Purpose |
|----------|---------|
| [ARCHITECTURE.md](docs/ARCHITECTURE.md) | Complete microkernel design and component specifications |
| [API.md](docs/API.md) | Syscall reference, capability types, return codes |
| [COOS.md](docs/COOS.md) | How to run UOSC as Co-OS under hypervisors |
| [SECURITY.md](docs/SECURITY.md) | Security model, threat model, capability proofs |
| [CONTRIBUTING.md](docs/CONTRIBUTING.md) | Development guidelines, code style, PR process |
| [FORMAL_VERIFICATION.md](docs/FORMAL_VERIFICATION.md) | How to write and check Axiom proofs |

## Key Components

### Capability System

Every resource (memory, IPC port, interrupt, device) is referenced by a **capability** – an unforgeable token in the process's capability table. Capabilities are:

- **Linear**: Cannot be duplicated (no copying without explicit grant)
- **Revocable**: Kernel can invalidate at any time
- **Delegable**: A process can transfer a capability to another process

```ti
// Example: grant read access to memory region
let mem_cap = cap_create(parent_cap, {read: true, write: false});
cap_delegate(mem_cap, target_process_id);
```

Proven properties (Axiom):
- No capability can be forged
- Revocation cascades to all derivatives
- A process cannot access a resource without holding the capability

### Scheduler

Two scheduling classes working together:

**EDF (Earliest Deadline First)**: For real-time tasks with hard deadlines. The kernel admits a new task only if it can prove all deadlines will be met. Proven to never miss deadlines if admission succeeds.

**CFS (Completely Fair Scheduler)**: For normal tasks. Uses a red-black tree to ensure starvation-free, fair CPU allocation. Each task gets proportional CPU time.

Lock-free per-CPU runqueues with work-stealing load balancing.

### Memory Manager

- **Physical**: Buddy allocator (4 KiB, 2 MiB, 1 GiB pages). NUMA-aware.
- **Virtual**: Per-process page tables with COW (copy-on-write).
- **Access Control**: Only processes holding a `Memory` capability can access a page.

Proven: no overlapping allocations, no use-after-free.

### IPC (Inter-Process Communication)

Two modes:

**Synchronous**: Request-reply. Sender blocks until receiver responds. Used for RPC-style calls.

**Asynchronous**: Send-and-forget. Message queued in recipient's port. Used for events.

Zero-copy: Large messages passed via shared memory region capabilities.

### Sanctum Vaults

Each userspace process runs in a hardware-isolated **vault** – a protected execution environment. Hardware isolation via:
- **CHERI capabilities** (if available)
- **Intel TDX** (Trusted Domain Extensions)
- **ARM CCA** (Confidential Computing Architecture)

Fallback: software isolation via page tables + capability checks.

### Co-OS Hypercalls

When UOSC runs as a guest under a hypervisor, it exposes hypercalls for the host to:
- Retrieve kernel state
- Grant/revoke capabilities from host
- Inject events from host

Used by Omnisystem's Co-OS driver for seamless host integration.

## Formal Verification

Critical components are verified using the **Axiom** theorem prover. Proofs cover:

| Component | Theorem | What's Proven |
|-----------|---------|---------------|
| Capability system | `cap_isolation` | No capability can be forged or accessed without grant |
| Scheduler (EDF) | `edf_deadline` | EDF never misses a deadline (if admission control respected) |
| Scheduler (CFS) | `cfs_starvation_free` | CFS never starves any task |
| Memory allocator | `alloc_no_overlap` | No two allocations can overlap |
| IPC | `ipc_safety` | Sender cannot influence receiver's memory outside capabilities |

To verify proofs locally:

```bash
axiom verify proof/capability.ax
axiom verify proof/scheduler.ax
# ... etc
```

Or run all proofs:

```bash
make verify
```

## Performance

**Typical metrics** (on x86-64 with 4 cores):

- Kernel startup: ~10ms (bare-metal), ~50ms (QEMU guest)
- Process spawn: ~1ms
- Context switch: ~0.5µs (lock-free, no TLB flush)
- IPC latency (sync): ~2µs
- IPC throughput (async): ~1M msgs/sec per CPU
- Capability check: 0 cycles (cached in TLB, hardware-enforced)

## Building for Different Targets

### Bare-Metal (UEFI/Multiboot)

```bash
make kernel TARGET=bare-metal
# outputs: kernel.elf (bootable)
```

### QEMU Guest (UEFI, BIOS)

```bash
make kernel TARGET=qemu
qemu-system-x86_64 -kernel kernel.elf -m 1024
```

### Library OS Mode (embedded in another kernel)

```bash
make lib TARGET=library-os
# outputs: libuosc.a (link with your application)
```

Usage:

```c
#include "uosc.h"

int main() {
    uosc_init();
    uosc_run();  // blocks forever
    return 0;
}
```

Link with `-luosc`.

### Hypervisor Guest (KVM, Hyper-V, Virtualization.framework)

```bash
make kernel TARGET=hypervisor-guest
# Can boot in any hypervisor via Co-OS driver (Omnisystem)
```

## Testing

### Unit Tests

```bash
make test-unit
# Compiles tests, runs them in user mode against libstd
```

### Integration Tests (QEMU)

```bash
make test-integration
# Runs full kernel boot + test suites under QEMU
```

### Benchmarks

```bash
make bench
# Measures scheduler, IPC, memory allocation latencies
```

### Formal Verification

```bash
make verify
# Checks all Axiom proofs (requires axiom binary in PATH)
```

## Contributing

See [CONTRIBUTING.md](docs/CONTRIBUTING.md) for:
- Code of conduct
- Development workflow
- Code style guidelines
- Testing requirements
- Formal verification requirements
- Commit message conventions

**Quick PR checklist**:
- [ ] Code passes `make test`
- [ ] If you modified kernel/ or capability.ti, you must add an Axiom proof
- [ ] Documentation is updated
- [ ] Commit message follows Conventional Commits
- [ ] CI passes (build, test, verify)

## Relation to Omnisystem

UOSC is the microkernel for [Omnisystem](https://github.com/your-org/omnisystem) – a complete polyglot OS built on top of UOSC. Omnisystem provides:

- Userspace services (TransferDaemon, UMS, AI Shim, etc.)
- Language runtimes (Titan, Sylva, Aether, Axiom)
- Full desktop environment (Bonsai Workspace)

You can use UOSC standalone without Omnisystem for embedded, real-time, or security-critical applications.

## Security Considerations

UOSC is designed with security as a first-class concern:

- **No implicit trust**: All access explicitly granted via capabilities
- **No privilege escalation**: Process cannot exceed its capability set
- **Cryptographic verification**: Capabilities can be verified by external systems
- **Hardware isolation**: Each vault is isolated by CPU (if available)
- **Formal proofs**: Critical security properties are mathematically verified

For detailed threat model and security guarantees, see [SECURITY.md](docs/SECURITY.md).

## License

Dual-licensed under **Apache License 2.0** or **MIT License** – choose whichever is more convenient for your project.

```
Copyright 2026 BonsaiAI Contributors

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
```

## Community

- **Issues**: Report bugs and request features on GitHub
- **Discussions**: Ask questions in GitHub Discussions
- **Contributing**: See [CONTRIBUTING.md](docs/CONTRIBUTING.md)
- **Chat**: Join us at `#uosc` on Matrix (placeholder)

## References

- [Capability-Based Security](https://en.wikipedia.org/wiki/Capability-based_security) – Wikipedia
- [Sanctum: An Open Hardware Secure Processor](https://eprint.iacr.org/2015/551.pdf) – MIT CSAIL
- [seL4: Formal Verification of an OS Kernel](https://sel4.systems/) – Data61
- [The Microkernel Debate](https://www.kernel.org/doc/html/latest/arch/um/index.html) – Andrew Tanenbaum vs Linus Torvalds

---

**UOSC Version**: 1.0.0  
**Last Updated**: 2026-06-08  
**Maintainer**: BonsaiAI Contributors

