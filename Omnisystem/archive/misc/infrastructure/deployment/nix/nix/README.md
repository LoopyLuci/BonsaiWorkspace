# Omnisystem NixOS Integration

Complete NixOS flake for seamless Omnisystem deployment at any integration level.

## Directory Structure

```
nix/
├── flake.nix                    # Main flake with outputs, modules, configurations
├── packages.nix                 # Omnisystem component packages and derivations
├── modules/
│   ├── omnisystem.nix          # Main NixOS module (services, adaptation, configuration)
│   ├── autonomic.nix           # Autonomous agents and self-management
│   ├── compatibility.nix       # POSIX, Windows ABI, legacy driver support
│   └── governance.nix          # Federated council integration and voting
├── examples/
│   ├── developer-workstation.nix    # Full desktop with all services
│   ├── edge-iot-node.nix           # Minimal 128MB edge device
│   └── federation-node.nix         # Multi-region governance setup
├── INTEGRATION_GUIDE.md         # Complete deployment guide
└── README.md                    # This file
```

## Quick Start

### 1. Use Pre-configured System

The flake includes 6 ready-to-use configurations:

```bash
# Developer workstation (full features)
nix build .#nixosConfigurations.omnisystem-developer.config.system.build.toplevel

# IoT edge node (128MB minimal)
nix build .#nixosConfigurations.omnisystem-iot.config.system.build.toplevel

# Hosted-light (systemd services)
nix build .#nixosConfigurations.omnisystem-hosted-light.config.system.build.toplevel

# Hosted-full (QEMU VM)
nix build .#nixosConfigurations.omnisystem-hosted-full.config.system.build.toplevel

# Bare-metal (standalone)
nix build .#nixosConfigurations.omnisystem-baremetal.config.system.build.toplevel

# Library OS (minimal linked library)
nix build .#nixosConfigurations.omnisystem-library.config.system.build.toplevel
```

## Modules

### omnisystem.nix (Main Module)

Core module enabling Omnisystem integration. Provides:

- Services: Toggle individual services (transfer-daemon, ums, omnicloak, etc.)
- Modes: hosted-light, hosted-full, bare-metal, library-os
- Adapter: Memory, CPU, GPU allocation
- Capabilities: Grant specific system permissions
- Autonomic: Self-healing agents configuration
- Compatibility: POSIX, Windows, legacy driver support
- Networking: Mesh configuration, peers, regions
- Persistence: State storage and Survival System

### autonomic.nix (Optional)

Autonomous management agents:
- Health monitoring agent (10s heartbeat)
- Performance optimizer (workload-aware tuning)
- Security auditor (threat detection)
- Failure detector (auto-recovery)
- Policy enforcer (governance compliance)

### compatibility.nix (Optional)

Compatibility layers:
- POSIX layer (Linux binary support)
- Windows ABI emulation
- Legacy device driver wrappers

### governance.nix (Optional)

Federated governance:
- BLS threshold signature verification
- Regional council integration
- Policy voting and enforcement
- Immutable audit trail

## Deployment Modes

| Mode | Integration | Use Case |
|------|-------------|----------|
| **hosted-light** | 40% | Development, edge |
| **hosted-full** | 80% | Production servers |
| **bare-metal** | 0% | Full sovereignty |
| **library-os** | 100% | Microservices |

## Image Building

```bash
nix build .#images.omnisystem-iso      # Bootable ISO
nix build .#images.omnisystem-qemu     # QEMU disk image
nix build .#images.omnisystem-docker   # Docker image
nix build .#images.omnisystem-rpi      # Raspberry Pi image
```

## Examples

See `examples/` directory:
- **developer-workstation.nix** – Full desktop integration
- **edge-iot-node.nix** – Minimal 128MB edge device
- **federation-node.nix** – Multi-region governance setup

## Governance

- 7 Regional Councils (Americas, Europe, APAC, Africa, Middle East, Oceania, Global)
- BLS Threshold Signatures (5-of-7 required for policy)
- Immutable Audit Trail
- Annual Re-election

## Security Features

- Capability-based security (no ambient authority)
- Formal verification (Axiom proofs)
- Hardware isolation (CHERI/TDX)
- Continuous verification (UVM every 5 minutes)
- Federated governance (no single point of failure)

## Quick Integration

In your `flake.nix`:

```nix
inputs.omnisystem.url = "github:LoopyLuci/Omnisystem";
modules = [omnisystem.nixosModules.omnisystem];
```

In `configuration.nix`:

```nix
services.omnisystem = {
  enable = true;
  mode = "hosted-light";
};
```

Deploy:

```bash
nixos-rebuild switch
```

## Next Steps

1. Read INTEGRATION_GUIDE.md for complete documentation
2. Choose a deployment mode
3. Customize for your needs
4. Deploy with nixos-rebuild

---

**Production-ready NixOS integration for Omnisystem.**
