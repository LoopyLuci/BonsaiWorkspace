# Omnisystem NixOS Integration Guide

## Overview

The Omnisystem flake provides seamless integration with NixOS, enabling you to deploy Omnisystem at any integration level (0%-100%) on any NixOS installation.

## Quick Start

### 1. Add Omnisystem to Your Flake

In your `flake.nix`:

```nix
{
  description = "My NixOS system with Omnisystem";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    omnisystem.url = "github:LoopyLuci/Omnisystem";
  };

  outputs = { self, nixpkgs, omnisystem }:
    let
      system = "x86_64-linux";
    in {
      nixosConfigurations.myhost = nixpkgs.lib.nixosSystem {
        inherit system;
        modules = [
          ./hardware-configuration.nix
          ./configuration.nix
          omnisystem.nixosModules.omnisystem
        ];
      };
    };
}
```

### 2. Enable Omnisystem in Configuration

In your `configuration.nix`:

```nix
services.omnisystem = {
  enable = true;
  mode = "hosted-light";  # or hosted-full, bare-metal, library-os
};
```

### 3. Deploy

```bash
nixos-rebuild switch
```

## Deployment Modes

### Hosted-Light (Fastest)

UOSC runs as systemd services within your NixOS host. All Omnisystem services become standard NixOS units.

**Configuration:**
```nix
services.omnisystem = {
  enable = true;
  mode = "hosted-light";
  services.transfer-daemon.enable = true;
  services.ums.enable = true;
  adapter.memory_mb = 512;
  adapter.cpus = 2;
};
```

**Advantages:**
- No VM overhead (native performance)
- Easy to introspect with `systemctl`
- Direct integration with NixOS networking
- Minimal memory footprint

**Use case:** Development, small servers, edge devices

---

### Hosted-Full (Most Flexible)

UOSC runs in a QEMU guest with full isolation. Seamless bidirectional integration via virtio devices.

**Configuration:**
```nix
services.omnisystem = {
  enable = true;
  mode = "hosted-full";
  adapter.memory_mb = 2048;
  adapter.cpus = 4;
  adapter.gpu_passthrough = true;
};
```

**Advantages:**
- Complete isolation (security benefit)
- GPU passthrough possible
- Can run any kernel version
- Live migration support

**Use case:** Production servers, workstations with hardware isolation

---

### Bare-Metal (Full Control)

Omnisystem boots directly from hardware. No host OS.

**Configuration:**
```nix
services.omnisystem = {
  enable = true;
  mode = "bare-metal";
  autonomic.enable = true;
};
```

**Build ISO:**
```bash
# Using Omnisystem's build CLI
build image create --profile bare-metal --output omnisystem.iso

# Or via Nix
nix build .#images.omnisystem-iso
```

**Advantages:**
- Zero overhead
- Complete control of all hardware
- Formal verification throughout
- Autonomic self-healing enabled

**Use case:** Embedded devices, private servers, full sovereignty

---

### Library-OS (Minimal Footprint)

UOSC compiles as a static library and links into your application. Unikernel-style deployment.

**Configuration:**
```nix
services.omnisystem = {
  enable = true;
  mode = "library-os";
  adapter.memory_mb = 32;
  adapter.cpus = 1;
};
```

**Link in your app:**
```rust
extern "C" {
    pub fn omnisystem_init(config: *const u8) -> i32;
}

fn main() {
    unsafe { omnisystem_init(std::ptr::null()) };
}
```

**Advantages:**
- Smallest possible footprint (32MB)
- Zero external dependencies
- Deterministic boot
- Perfect reproducibility

**Use case:** Microservices, serverless, embedded applications

---

## Service Configuration

### Enable Individual Services

```nix
services.omnisystem = {
  enable = true;

  services = {
    transfer-daemon = true;  # P2P mesh networking
    ums = true;             # Universal module system
    omnicloak = true;       # Secure browser
    vfs = true;             # Virtual filesystem
    net-stack = true;       # Network stack
    storage = true;         # Persistent storage
    device-manager = true;  # Hardware management
    display = true;         # Graphics/display
    compositor = true;      # Desktop environment
    logger = true;          # Logging service
    config = true;          # Configuration management
    service-manager = true; # Service orchestration
  };
};
```

### Adapter Configuration

```nix
services.omnisystem = {
  adapter = {
    memory_mb = 1024;       # RAM allocation
    cpus = 4;              # Virtual CPUs
    gpu_passthrough = true; # GPU support (hosted-full only)
    debug = false;         # Debug logging
  };
};
```

---

## Autonomic Management

Enable self-healing and auto-optimization:

```nix
services.omnisystem = {
  autonomic = {
    enable = true;

    healthCheck = {
      enable = true;
      interval = "10s";
    };

    performanceOptimization = {
      enable = true;
      enablePredictiveLoading = true;  # AI-powered (optional)
    };

    securityAuditing = {
      enable = true;
      scanInterval = "1h";
    };

    failureDetection = {
      enable = true;
      restartPolicy = "immediate";  # immediate | delayed | manual
    };
  };
};
```

The system will automatically:
- Restart crashed services
- Tune performance based on workload
- Detect and mitigate security threats
- Rebalance resources

---

## Federated Governance

For multi-region deployments with autonomous governance:

```nix
services.omnisystem = {
  governance = {
    councilKeys = [
      "americas-council-member-1-bls-key"
      "europe-council-member-1-bls-key"
      "apac-council-member-1-bls-key"
      # ... (7 councils × 5 members + 3 global arbiters)
    ];

    thresholdSignatures = 5;  # 5-of-7 councils required

    policyUpdateFrequency = "7d";  # Check weekly
  };
};
```

**How it works:**
1. Any policy change must be signed by ≥5 councils
2. Councils are elected regionally, 1-year terms
3. Global Arbiter breaks ties
4. All votes recorded in immutable audit trail
5. Rollback possible only with council approval

---

## Compatibility Layers

### POSIX Support

```nix
services.omnisystem = {
  compatibility.posix.enable = true;
};
```

Provides standard POSIX APIs for Linux binaries.

### Windows ABI Emulation

```nix
services.omnisystem = {
  compatibility.windows.enable = true;
};
```

Translates Windows API calls to Omnisystem capabilities.

### Legacy Driver Wrappers

```nix
services.omnisystem = {
  compatibility.legacyDrivers.enable = true;
};
```

Maps legacy PCI/USB device IDs to modern drivers.

---

## Networking & Mesh

Enable P2P mesh networking:

```nix
services.omnisystem = {
  networking = {
    meshEnabled = true;

    regions = [
      "Americas"
      "Europe"
      "APAC"
    ];

    peers = [
      "node1.omnisystem.local:9000"
      "node2.omnisystem.local:9000"
    ];
  };
};
```

**Mesh Features:**
- Gossip-based device registry
- <100ms policy propagation
- Multi-hop routing
- Automatic failover
- Content-addressed module distribution

---

## Capability System

Grant specific capabilities to the system:

```nix
services.omnisystem = {
  capabilities = [
    "net-access"           # Network I/O
    "storage-full"         # Full storage access
    "device-usb"           # USB device access
    "device-gpu"           # GPU access
    "policy-voting"        # Governance participation
    "service-orchestration" # Deploy services
  ];
};
```

The kernel enforces these capabilities; no ambient authority.

---

## Persistence & Recovery

Enable transaction logging and automatic recovery:

```nix
services.omnisystem = {
  persistence = {
    stateDirectory = /var/lib/omnisystem;

    survivalSystem = {
      enable = true;
      logPath = /var/log/omnisystem/survival;
    };
  };
};
```

The Survival System:
- Logs every state change
- Detects corrupted state
- Automatic rollback to last known good state
- Immutable audit trail

---

## Example Configurations

### Developer Workstation

See `nix/examples/developer-workstation.nix`:
- Full desktop integration
- All services enabled
- Autonomic management active
- GPU passthrough

### Edge IoT Node

See `nix/examples/edge-iot-node.nix`:
- 128MB minimal footprint
- Essential services only
- Mesh networking
- Basic autonomic health checks

### Federation Node

See `nix/examples/federation-node.nix`:
- Multi-region council integration
- Full governance enabled
- BLS threshold signature voting
- Production-hardened security

---

## Building Images

### ISO for Bare-Metal

```bash
nix build .#images.omnisystem-iso
# Result: ./result/iso/omnisystem-*.iso
```

### QEMU Image

```bash
nix build .#images.omnisystem-qemu
# Result: ./result/disk.qcow2
```

### Docker Image

```bash
nix build .#images.omnisystem-docker
# Result: ./result (docker tarball)
```

### Raspberry Pi SD Card

```bash
nix build .#images.omnisystem-rpi
# Result: ./result/sd-image/sd-image-aarch64-linux.img
```

---

## Troubleshooting

### Service won't start

Check logs:
```bash
systemctl status omnisystem-*
journalctl -u omnisystem-transfer-daemon -n 100
```

### Out of memory

Increase adapter.memory_mb:
```nix
services.omnisystem.adapter.memory_mb = 2048;
```

### Governance keys invalid

Verify BLS keys are properly formatted:
```bash
omnisystem-validate-governance --check-keys config
```

### Recovery from state corruption

The Survival System automatically detects and recovers:
```bash
journalctl -u omnisystem-survival-system
```

To force manual rollback (council approval required):
```bash
omnisystem-governance override --rollback-to <timestamp>
```

---

## Security Best Practices

1. **Restrict network access** – Use firewall to limit mesh peers
2. **Verify council keys** – Confirm BLS keys out-of-band before first deployment
3. **Regular audits** – Review `journalctl` for autonomic decisions
4. **Capability minimization** – Grant only necessary capabilities
5. **Rotate credentials** – Annual council re-election enforced
6. **Monitor autonomic agents** – Ensure HealthMonitor is active

---

## Performance Tuning

### For Development (hosted-light)

```nix
services.omnisystem = {
  mode = "hosted-light";
  adapter.memory_mb = 512;
  adapter.cpus = 2;
  autonomic.performanceOptimization.enablePredictiveLoading = true;
};
```

### For Production (hosted-full)

```nix
services.omnisystem = {
  mode = "hosted-full";
  adapter.memory_mb = 4096;
  adapter.cpus = 8;
  adapter.gpu_passthrough = true;
  autonomic.enable = true;
};
```

### For Embedded (bare-metal)

```nix
services.omnisystem = {
  mode = "bare-metal";
  services = {
    kernel = true;
    transfer-daemon = true;
    device-manager = true;
  };
  autonomic.performanceOptimization.enable = false;
};
```

---

## Further Reading

- **Architecture**: See `OMNISYSTEM_ARCHITECTURE_COMPLETE.md`
- **Vision**: See `OMNISYSTEM_FINAL_VISION.md`
- **Build Profiles**: See `Omnisystem/build/profiles.ti`
- **Adapter Layer**: See `Omnisystem/kernel/adapter.ti`
- **API Docs**: `nix flake show` (shows all outputs)

---

## Support

For issues, questions, or governance participation:

- **GitHub Issues**: https://github.com/LoopyLuci/Omnisystem/issues
- **Council Voting**: Governance decisions at omnisystem.governance
- **Community Forum**: (To be established)

---

**Built with Nix. Governed by consensus. Verified by Axiom. Ready for production.**
