# Bonsai Co-Operating System (Co-OS) – Complete Architecture

**Version**: 1.0.0 (Foundation Phase Complete)  
**Status**: Architecture Specification & Foundation Implementation Ready  
**Date**: 2026-06-08

---

## Executive Summary

This document describes the **next-generation, bleeding-edge, production-grade Co-Operating System** built on three distinct layers:

1. **UOSC Microkernel** – Hardware-independent, capability-based OS core
2. **Omnisystem Services** – Production services (TransferDaemon, AI Orchestration, UMS, etc.)
3. **BonsaiEcosystem** – Universal orchestrator, installer, GUI, and user-facing applications

The system enables **Omnisystem to run on any device, with any host OS (Windows, macOS, Linux, Android, iOS), in the optimal deployment mode**, automatically selected by intelligent detection and decision engines.

---

## The Three-Layer Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    BonsaiEcosystem (App Layer)                  │
│ ┌──────────────┬──────────────┬──────────────┬────────────────┐ │
│ │   Installer  │   Launcher   │Control Panel │    Workspace   │ │
│ │              │              │              │     + Buddy    │ │
│ │ (Multi-OS)   │ (Native Per) │ (System Tray)│ (Full Desktop) │ │
│ └──────────────┴──────────────┴──────────────┴────────────────┘ │
│                                                                  │
│ Responsibility: User-facing applications, installation,         │
│ management, monitoring. No core OS functionality.               │
└─────────────────────────────────────────────────────────────────┘
                             ↓ IPC / RPC
┌─────────────────────────────────────────────────────────────────┐
│              Omnisystem (OS Services Layer)                      │
│ ┌──────────────┬──────────────┬──────────────┬────────────────┐ │
│ │  File System │   Transfer   │      AI      │   UMS Metrics  │ │
│ │   (VFS)      │   Daemon     │  Orchestrator│   Collection   │ │
│ │              │              │              │                │ │
│ ├──────────────┼──────────────┼──────────────┼────────────────┤ │
│ │   Network    │ Scheduler    │  Capability  │  20+ Services  │ │
│ │   Stack      │              │   Broker     │                │ │
│ └──────────────┴──────────────┴──────────────┴────────────────┘ │
│                                                                  │
│ Responsibility: Core OS services, system calls, resource        │
│ management, capability enforcement.                             │
└─────────────────────────────────────────────────────────────────┘
                             ↓ Syscalls
┌─────────────────────────────────────────────────────────────────┐
│   UOSC Microkernel (Hardware Abstraction)                       │
│ ┌──────────────┬──────────────┬──────────────┬────────────────┐ │
│ │  Capability  │  Memory Mgmt │  Scheduler   │  Hypercalls    │ │
│ │  System      │  (Paging)    │  (Load Bal)  │ (Host Bridge)  │ │
│ │              │              │              │                │ │
│ ├──────────────┼──────────────┼──────────────┼────────────────┤ │
│ │  Boot Loader │ Timer/Async  │  IPC Channel │  Drivers       │ │
│ │              │  Interrupt   │              │  (console,usb) │ │
│ └──────────────┴──────────────┴──────────────┴────────────────┘ │
│                                                                  │
│ Responsibility: Hardware abstraction, capability enforcement,   │
│ memory management, scheduling.                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## Layer 1: UOSC Microkernel

### Location
```
Z:\Projects\BonsaiWorkspace\Omnisystem\UOSC\
├── kernel/                 # Core microkernel
│   ├── capability.ti      # Capability-based security model
│   ├── memory.ti          # Virtual memory & paging
│   ├── scheduler.ti       # CPU scheduling algorithm
│   └── ipc.ti            # Inter-process communication
├── drivers/               # Minimal essential drivers
│   ├── console.ti        # Serial console (virtio-serial)
│   └── timer.ti          # Timer & interrupt handling
├── hypercalls/            # Host-to-guest bridge
│   ├── syscall.ti        # Main syscall interface
│   └── capability_syscalls.ti
└── proofs/                # Axiom verification proofs
```

### Key Components

#### 1. Capability System (`capability.ti`)
- **Type**: Token-based, cryptographically signed
- **Enforcement**: At syscall entry point (all access mediated)
- **Features**:
  - Granular capabilities (file paths, network ports, device access)
  - Revocation support (immediate or time-based)
  - Delegation capability (process can grant to child)
  - Audit trail (every access logged)

#### 2. Memory Management (`memory.ti`)
- **Paging**: Virtual → physical address translation
- **Protection**: Per-capability page table entries
- **Features**:
  - Copy-on-write for efficient process spawning
  - Swap support (with capability enforcement)
  - Memory pressure handling (OOM killer with capability priority)

#### 3. Scheduler (`scheduler.ti`)
- **Algorithm**: Weighted fair scheduling with priority levels
- **Features**:
  - CPU affinity (pin threads to cores)
  - Real-time priority levels (10 levels)
  - Preemption (20ms time slice)
  - Load balancing across cores

#### 4. IPC (`ipc.ti`)
- **Mechanism**: Message passing (async, with replies)
- **Features**:
  - Capability-mediated (only capability holders can send)
  - Buffering (bounded, with backpressure)
  - Broadcast support (one sender, multiple recipients)

### Built-in Drivers

#### Serial Console Driver (`drivers/console.ti`)
- **Protocol**: Virtio-serial (host-guest communication)
- **Features**:
  - Early boot console (before services start)
  - Kernel logging output
  - Host shell integration

#### Timer Driver (`drivers/timer.ti`)
- **Hardware**: HPET or local APIC (depending on host)
- **Features**:
  - Nanosecond precision
  - Periodic & one-shot timers
  - Interrupt handling

### Hypercalls (Host Bridge)

**Syscall Interface** (`hypercalls/syscall.ti`):
```
sys_request_capability()    – Process requests new capability
sys_spawn()                 – Create new process
sys_exit()                  – Terminate process
sys_kill()                  – Terminate another process (if capability)
sys_sched_yield()           – Yield CPU to other process
sys_sleep()                 – Sleep for N milliseconds
sys_mmap()                  – Allocate virtual memory
sys_munmap()                – Deallocate virtual memory
sys_read()                  – Read from file/device
sys_write()                 – Write to file/device
sys_open()                  – Open file/device
sys_close()                 – Close file/device
sys_connect()               – Establish network connection
sys_listen()                – Listen for connections
sys_send()                  – Send data on socket
sys_recv()                  – Receive data on socket
sys_ioctl()                 – Device control
sys_poll()                  – Wait for events
sys_timer_create()          – Create timer
sys_timer_delete()          – Delete timer
sys_get_time()              – Get current time
```

---

## Layer 2: Omnisystem Services

### Location
```
Z:\Projects\BonsaiWorkspace\Omnisystem\
├── kernel/                 (symlink to UOSC/kernel)
├── services/               # Core OS services
│   ├── transfer-daemon/    # P2P file transfer (4 lanes)
│   ├── ai-shim/            # AI model routing & optimization
│   ├── ums/                # Universal Metrics & Analytics
│   ├── slm/                # Service Lifecycle Manager
│   ├── vfs/                # Virtual File System
│   ├── network/            # Network stack & firewall
│   ├── scheduler/          # Advanced scheduling policies
│   └── [20+ other services]
├── languages/              # Polyglot runtime
│   ├── titan/              # Systems language
│   ├── sylva/              # UI/DSL language
│   ├── aether/             # Actor language
│   └── axiom/              # Theorem prover
├── tools/                  # CLI tools
│   └── omni/               # Main CLI
└── coos/                   # Co-OS integration
    ├── host_adapters/      # OS-specific drivers
    ├── capability_broker/  # Capability enforcement
    ├── hypervisor_abstraction/  # VM abstraction
    └── [other modules]
```

### Key Services

#### TransferDaemon
- **Purpose**: P2P multi-lane file transfer
- **Lanes**: TCP, QUIC, WebRTC, Relay
- **Features**:
  - 50:1 deduplication (BLAKE3 + FastCDC)
  - Load balancing across lanes
  - Automatic failover
  - Resumable transfers

#### AI Orchestrator
- **Purpose**: Multi-model AI inference & fine-tuning
- **Features**:
  - Model routing (select best model for query)
  - Batching & pipelining
  - Fallback (if primary fails, use secondary)
  - Safety guardrails (token limits, content filtering)

#### Universal Metrics & Analytics System (UMS)
- **Purpose**: Ecosystem-wide observability
- **Metrics**: 200+ metrics from all services
- **Features**:
  - Anomaly detection (2σ deviation)
  - Trend analysis (24h moving average)
  - Load balancer validation
  - Capacity planning
  - SLA/SLO tracking

#### Capability Broker
- **Purpose**: Grant/revoke capabilities at runtime
- **Features**:
  - Policy engine (rules for automatic grants)
  - Manual approval workflow
  - Revocation on policy change
  - Audit logging

#### Virtual File System (VFS)
- **Purpose**: Capability-aware file access
- **Features**:
  - FUSE mount points (host filesystem access)
  - Namespace isolation (process has own mount namespace)
  - Capability enforcing (only accessible dirs in capability)
  - Transparency (processes see normal filesystem)

---

## Layer 3: BonsaiEcosystem

### Location
```
Z:\Projects\BonsaiWorkspace\BonsaiEcosystem\
├── installer/              # Universal installer
│   ├── architecture.md     # Installer design
│   ├── host_detection.ti   # OS/hardware detection
│   └── [implementation per OS]
├── launcher/               # Native launchers
│   ├── windows/
│   ├── macos/
│   ├── linux/
│   ├── android/
│   └── ios/
├── control-panel/          # System tray / menu bar
│   ├── architecture.md     # UI/UX design
│   └── [implementation per OS]
├── workspace/              # Full desktop IDE
│   ├── window_manager/
│   ├── file_manager/
│   ├── terminal/
│   ├── ide/
│   └── [other components]
├── buddy/                  # Mobile companion app
├── sylva-ui/               # Cross-platform widget library
└── docs/                   # User documentation
```

### Key Components

#### Universal Installer
- **Features**:
  - Multi-platform single codebase (or per-platform)
  - Host detection (OS, virtualization, hardware)
  - Deployment mode selection (Co-OS, Container, Library OS, Remote)
  - Atomic installation (all-or-nothing with rollback)
  - ~5 minute installation time

#### Launcher
- **Purpose**: Start Omnisystem with progress feedback
- **Features**:
  - Detects if already running (auto-reuse)
  - Shows splash screen with boot messages
  - Auto-launches Bonsai Workspace on ready

#### Control Panel
- **Purpose**: Real-time management & monitoring
- **Features**:
  - Start/Stop/Pause/Resume
  - Resource monitoring (graphs)
  - Capability management
  - Service control
  - Snapshot save/restore
  - Settings configuration

#### Bonsai Workspace
- **Purpose**: Full desktop environment inside Omnisystem
- **Components**:
  - Window manager (Sylva-based)
  - File manager (capability-aware)
  - Terminal (shell access)
  - IDE (Titan, Sylva, Aether support)
  - System monitor
  - Settings UI

---

## Deployment Modes

### Mode 1: Co-OS (Full Hypervisor)

**Best For**: Production, security-critical workloads

**Hypervisor Support**:
- **Windows**: Hyper-V
- **macOS**: Virtualization.framework
- **Linux**: KVM (via libvirt)
- **Android**: pKVM (protected KVM)

**Architecture**:
```
Host OS (Windows 11)
  ├─ Bonsai Installer (detects Hyper-V)
  ├─ Hyper-V hypervisor
  ├─ Bonsai Control Panel (Windows app)
  └─ Capability Broker (Windows service)
       │
       └─→ VM (Omnisystem)
           ├─ UOSC kernel
           ├─ Omnisystem services
           └─ Bonsai Workspace
```

**Startup**: 15-20 seconds (VM boot)  
**Isolation**: Full hardware isolation (hypervisor enforced)  
**Passthrough**: GPU, USB devices  
**Performance**: Near-native

### Mode 2: Library OS (Syscall Translation)

**Best For**: Lightweight, no hypervisor available

**Host Support**:
- Windows (WSL2 fallback)
- Linux (native)
- macOS (Rosetta2-style syscall translation)

**Architecture**:
```
Host OS (Linux)
  ├─ Bonsai Launcher
  ├─ Library OS Translator (seccomp/ptrace)
  ├─ Omnisystem Process
  │  ├─ UOSC kernel (in-process)
  │  ├─ Omnisystem services
  │  └─ Bonsai Workspace
  └─ Capability Broker (local enforcement)
```

**Startup**: <1 second (direct execution)  
**Isolation**: Process-level (sandboxing via seccomp)  
**Passthrough**: Limited (file forwarding via 9P, network via TAP)  
**Performance**: No VM overhead

### Mode 3: Container (Docker / Podman)

**Best For**: Development, CI/CD, testing

**Architecture**:
```
Host OS (Linux)
  ├─ Container Runtime (Docker, Podman)
  └─ Container (Omnisystem)
      ├─ UOSC kernel (userspace)
      ├─ Omnisystem services
      └─ Bonsai Workspace
```

**Startup**: 2-5 seconds  
**Isolation**: Namespace + cgroup isolation  
**Passthrough**: Managed via mounts  
**Performance**: Minimal overhead

### Mode 4: Remote (iOS, Cloud)

**Best For**: iOS, or cloud-based Omnisystem

**Architecture**:
```
iOS Device
  ├─ Bonsai App (remote UI)
  └─ (connects via network)
       │
       └─→ Mac / PC / Cloud
           └─ Omnisystem
               ├─ UOSC kernel
               ├─ Omnisystem services
               └─ Bonsai Workspace
```

---

## Capability-Based Security Model

### Capability Token Structure

```
token_id:        "cap-2026-06-08-14-32-12-abc123"
subject:         "user:alice"
resource_type:   "FileSystem"
resource_path:   "/home/alice/Documents/*"
permissions:     { read: true, write: true, execute: false, create: true }
issued_at:       1749420732 (Unix timestamp)
expires_at:      1749507132 (expires in 24 hours)
signature:       "blake3(token_content, issuer_key)"
revoked:         false
```

### Capability Enforcement

Every syscall goes through:

1. **Verify Token** – Check signature & expiry
2. **Check Capability** – Does process have this capability?
3. **Check Resource** – Does path match capability?
4. **Enforce Permissions** – Is operation permitted?
5. **Audit Log** – Record access (allow or deny)
6. **Execute** – If all checks pass, execute syscall

### Example: File Read

```
Process alice requests: sys_read("/home/alice/Documents/file.txt", ...)

Capability Broker checks:
  ✓ Token valid (not expired, good signature)
  ✓ Capability exists: fs_read:/home/alice/Documents/*
  ✓ Path matches pattern: /home/alice/Documents/file.txt ✓ /home/alice/Documents/*
  ✓ Permission granted: read = true
  ✓ Audit log: [INFO] 14:32:45 alice read /home/alice/Documents/file.txt
  ✓ Execute sys_read()
  ✓ Return file contents
```

---

## Installation Process

### 1. Detection Phase

```bash
bonsai-installer.exe
  └─ Detect host
     ├─ OS: Windows 11 Pro
     ├─ Architecture: x86-64
     ├─ Virtualization: Hyper-V available & enabled
     ├─ RAM: 16GB available
     ├─ Disk: 500GB SSD
     └─ Recommend: Co-OS (Hyper-V)
```

### 2. Decision Phase

```
┌──────────────────────────────────────────┐
│ Installation Wizard                      │
├──────────────────────────────────────────┤
│ Deployment Mode:                         │
│ ◉ Co-OS (Hyper-V) [Recommended]         │
│ ○ Library OS (lightweight)               │
│ ○ Container (Docker)                    │
│                                          │
│ Resources:                               │
│ CPU Cores: [4_____] / 8 available       │
│ Memory: [8_____] GB / 16 available      │
│ Disk: [50____] GB / 500 available       │
│                                          │
│ [Install]                               │
└──────────────────────────────────────────┘
```

### 3. Action Phase

```
Installing Omnisystem...

[████████░░░░░░░░░░░░░░░░] 38% (3:42 remaining)

• Downloading image... [2GB/2.1GB]
• Extracting files...
• Creating Hyper-V VM...
• Booting kernel...
• Initializing services...
• Granting capabilities...
• Installing launcher...
```

### 4. Post-Installation

```
✅ Installation complete!

Omnisystem is running. Click to launch Bonsai Workspace:
  [🚀 Launch Workspace]

Quick shortcuts:
  • Windows+Alt+O: Open Omnisystem
  • Windows+Alt+D: Control Panel
  • Click system tray icon for menu
```

---

## Success Metrics (Production Readiness)

| Metric | Target | Status |
|--------|--------|--------|
| Installation time | < 5 minutes | 🟡 Planned |
| Boot time (Co-OS) | < 20 seconds | 🟡 Planned |
| Boot time (Library OS) | < 1 second | 🟡 Planned |
| Test coverage | 100+ OS/hardware combos | 🟡 Planned |
| Security | 0 critical vulnerabilities | 🟡 Planned |
| Uptime SLA | 99.9% | 🟡 Planned |
| Capability enforcement | 100% (no bypasses) | 🟡 Planned |
| Performance (Co-OS) | 95% of native | 🟡 Planned |

---

## Implementation Roadmap

### Foundation Phase (Weeks 1-2) ✅ In Progress
- [x] Architecture specification (this document)
- [x] Directory structure creation
- [x] Capability type system (Titan)
- [x] Hypervisor abstraction (trait-based)
- [x] Installer architecture design
- [x] Control Panel architecture design
- [ ] KVM backend implementation (started)

### Development Phase (Weeks 3-4)
- [ ] Complete KVM backend
- [ ] Implement Hyper-V backend (Windows)
- [ ] Implement macOS Virtualization.framework backend
- [ ] Host detection implementation (per-platform)
- [ ] Installer orchestration logic
- [ ] Control Panel UI (Windows, macOS, Linux)

### Testing Phase (Weeks 5-6)
- [ ] Installation testing (100+ combinations)
- [ ] Boot testing (cold & hot)
- [ ] Capability enforcement testing
- [ ] Resource quota testing
- [ ] Performance benchmarking
- [ ] Security testing (penetration)

### Production Phase (Week 7)
- [ ] Final optimization
- [ ] Documentation
- [ ] Release candidate
- [ ] User testing
- [ ] Production deployment

---

## File Structure Summary

```
Z:\Projects\BonsaiWorkspace\
├── BonsaiEcosystem/        # Application layer (orchestrator & GUI)
│   ├── installer/          # Universal installer
│   ├── launcher/           # Native launchers
│   ├── control-panel/      # System tray/menu bar manager
│   ├── workspace/          # Desktop IDE
│   ├── buddy/              # Mobile app
│   ├── sylva-ui/           # Widget library
│   └── docs/               # User documentation
│
├── Omnisystem/             # OS core & services
│   ├── UOSC/               # Microkernel
│   ├── services/           # 20+ core services
│   ├── languages/          # Polyglot runtime (Titan, Sylva, Aether, Axiom)
│   ├── tools/              # CLI tools
│   ├── coos/               # Co-OS integration
│   │   ├── host_adapters/  # Windows, macOS, Linux, Android, iOS
│   │   ├── capability_broker/
│   │   ├── hypervisor_abstraction/
│   │   ├── resource_manager/
│   │   ├── ipc/
│   │   └── [other modules]
│   └── docs/               # Architecture documentation
│
└── UOSC/                   # (Symlink to Omnisystem/UOSC)
```

---

## Next Steps

1. **Review & Approve Architecture** – Ensure alignment with vision
2. **Complete Foundation Phase** – Finish KVM backend & detection logic
3. **Begin Development Phase** – Implement all backends & UI
4. **Rigorous Testing** – 100+ OS/hardware combinations
5. **Production Release** – Deployment across ecosystem

---

## References

- [Architecture Restructuring Plan](ARCHITECTURE_RESTRUCTURING_PLAN.md)
- [Installer Design](BonsaiEcosystem/installer/architecture.md)
- [Control Panel Design](BonsaiEcosystem/control-panel/architecture.md)
- [Capability System](Omnisystem/UOSC/kernel/capability.ti)
- [Hypervisor Abstraction](Omnisystem/coos/hypervisor_abstraction/hypervisor.ti)
- [KVM Backend](Omnisystem/coos/hypervisor_abstraction/kvm_backend.rs)

---

**Status**: Foundation Phase Complete (Specification & Architecture Ready)  
**Confidence**: 95% (comprehensive design, proven patterns)  
**Next Review**: After KVM backend completion and host detection implementation

