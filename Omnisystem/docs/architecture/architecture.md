# Bonsai Universal Installer Architecture

**Production-Grade Installation Orchestrator for All Platforms**

---

## Overview

The Bonsai Installer is a **single, universal executable** that:
1. Detects the host environment (OS, virtualization, hardware)
2. Selects the optimal Omnisystem deployment mode
3. Orchestrates flawless installation across all platforms
4. Manages capabilities and resource allocation
5. Enables post-install configuration and updates

**Support Matrix**:
- Windows 10 Pro, 10 Home, 11 Pro, 11 Home, 11 Enterprise
- macOS 12.x (Intel), 13.x (ARM), 14.x (ARM), 15.x (ARM)
- Linux: Ubuntu 22.04, 24.04, Debian 12, Fedora 38+, RHEL 8+
- Android 12, 13, 14, 15 (kernel 5.10+)
- iOS 16, 17, 18 (via remote service)

---

## Design Principles

1. **Automatic Optimization** – Detect capabilities, select best mode
2. **Reversible Operations** – Every installation is fully rollbackable
3. **User-Friendly** – Simple 3-4 question wizard, no technical jargon
4. **Fast Deployment** – <5 minutes from start to running Omnisystem
5. **Secure by Default** – Capability-based, least privilege access
6. **Transparent** – Clear progress, human-readable logs
7. **Offline Capable** – Can work with cached images, fallback options

---

## Installation Phases

### Phase 1: Detection

**Host Detection**:
```
Detect:
  ├─ OS type (Windows, macOS, Linux, Android, iOS)
  ├─ OS version (e.g., Windows 11 Pro Build 22621)
  ├─ Architecture (x86-64, ARM64, ARM32)
  ├─ Virtualization support
  │   ├─ Windows: Hyper-V, VBS, HVCI
  │   ├─ macOS: Virtualization.framework, vAPIC
  │   ├─ Linux: KVM, IOMMU (Intel-VT-d / AMD-Vi)
  │   ├─ Android: pKVM, kernel version
  │   └─ iOS: N/A (remote only)
  ├─ Hardware
  │   ├─ CPU model, cores, extensions (AVX2, SSE4.1, etc.)
  │   ├─ RAM (total, available)
  │   ├─ Disk (free space, type: SSD vs HDD)
  │   ├─ GPU (NVIDIA, AMD, Intel, Apple)
  │   ├─ USB controllers (XHCI, EHCI)
  │   └─ Audio hardware
  ├─ Network
  │   ├─ Ethernet available
  │   ├─ WiFi available
  │   ├─ Firewall rules
  │   └─ Network stack version
  └─ Existing installations
      └─ Previous Omnisystem version, if any
```

**Decision Tree**:
```
if (Windows) {
    if (Pro or Enterprise) {
        if (Hyper-V_available) {
            recommend: Co-OS (Hyper-V)
        } else {
            recommend: Library OS (syscall translation)
        }
    } else {  // Windows Home
        recommend: Library OS (WSL2 fallback option)
    }
}
else if (macOS) {
    if (ARM64) {
        recommend: Co-OS (Virtualization.framework)
    } else {  // Intel
        recommend: Co-OS (VM with Virtualization.framework)
    }
}
else if (Linux) {
    if (KVM_available) {
        recommend: Co-OS (KVM)
    } else if (Docker_available) {
        recommend: Container (Docker/Podman)
    } else {
        recommend: Library OS
    }
}
else if (Android) {
    if (pKVM_available) {
        recommend: Co-OS (pKVM)
    } else {
        recommend: Container (Chroot/namespace isolation)
    }
}
else if (iOS) {
    recommend: Remote (paired Mac/PC or cloud)
}
```

**Output**: `InstallationProfile` with recommended mode and alternative options

---

### Phase 2: Decision (User Input)

**Wizard Screens** (4-5 screens max):

1. **Welcome Screen**
   ```
   ┌─────────────────────────────────────────┐
   │       Welcome to Bonsai Omnisystem      │
   │                                         │
   │  The sovereign operating system        │
   │                                         │
   │  Detected: Windows 11 Pro               │
   │  Recommended: Co-OS (Hyper-V)           │
   │                                         │
   │  [Continue] [Learn More] [Settings]     │
   └─────────────────────────────────────────┘
   ```

2. **Deployment Mode Selection** (if not obvious)
   ```
   How would you like to run Omnisystem?

   ○ Co-OS (Full OS alongside Windows) - Recommended
     • Hardware isolation via Hyper-V
     • Full performance
     • Shared devices (USB, GPU)
   
   ○ Library OS (Process-level isolation)
     • Lightweight, low overhead
     • Faster startup
     • Limited device access
   
   ○ Container (Namespace isolation)
     • Isolated filesystem
     • Good for testing
     • Lower resource use

   [Continue]
   ```

3. **Capability Selection**
   ```
   What should Omnisystem access?

   ☑ File access: /home/user/Documents, /home/user/Downloads
   ☑ Network: All outbound, Inbound on ports 8000-9000
   ☑ USB devices
   ☐ GPU (NVIDIA/AMD)
   ☐ Audio input/output
   ☑ Clipboard integration
   
   [Customize] [Continue]
   ```

4. **Resource Allocation**
   ```
   Resources for Omnisystem:

   CPU Cores:    [4_____] 4 / 8 available
   Memory (GB):  [8_____] 8 / 16 available
   Disk (GB):    [50____] 50 / 500 available

   Snapshots:    ☑ Enable (for pause/resume)

   [Continue]
   ```

5. **Review & Install**
   ```
   Ready to install Omnisystem:

   Mode:         Co-OS (Hyper-V)
   Capabilities: File access, Network, USB, Clipboard
   Resources:    4 CPU, 8GB RAM, 50GB disk
   Estimated:    3-5 minutes

   [Install] [Back]
   ```

**Output**: `InstallationDecision` with user selections

---

### Phase 3: Action (Actual Installation)

**Installation Steps**:

```
1. Pre-flight Checks
   ├─ Verify disk space (need 50GB + image size)
   ├─ Verify network connectivity
   ├─ Check for conflicts (port 8000-9000, Hyper-V name)
   ├─ Verify user permissions (admin/sudo)
   └─ Create rollback checkpoint

2. Download Image
   ├─ Download Omnisystem image (~2GB)
   │   └─ Verify checksum (BLAKE3)
   ├─ Download Linux kernel (if needed)
   ├─ Download initial ramdisk
   └─ Extract to temporary location

3. Set Up Hypervisor / Container
   ├─ For Windows (Hyper-V):
   │   ├─ Enable Hyper-V (if not already)
   │   ├─ Create virtual network
   │   ├─ Create VM with specified resources
   │   └─ Attach disk image
   ├─ For macOS:
   │   ├─ Create VM using Virtualization.framework
   │   ├─ Attach disk image
   │   └─ Configure console (VNC or native display)
   ├─ For Linux:
   │   ├─ Create KVM domain (libvirt)
   │   ├─ Create disk image (qcow2)
   │   └─ Configure networking
   └─ [Similar for Android, etc.]

4. First Boot
   ├─ Start VM/container
   ├─ Attach serial console
   ├─ Stream boot messages to installer UI
   ├─ Wait for "Omnisystem ready" signal
   └─ Verify kernel loaded

5. Initialize Capabilities
   ├─ Connect to Omnisystem's capability broker
   ├─ Grant selected capabilities (file access, network, USB)
   ├─ Set resource quotas (CPU, memory, I/O)
   ├─ Enable device passthrough (USB, GPU)
   └─ Save configuration

6. Install Bonsai Launcher
   ├─ Windows: Create Start Menu shortcut, add to PATH
   ├─ macOS: Create .app bundle, add to Launchpad
   ├─ Linux: Create .desktop entry, add to PATH
   ├─ Android: Create app icon
   └─ iOS: Add to home screen

7. Create First User
   ├─ Prompt for username
   ├─ Prompt for password (if desired)
   ├─ Create home directory
   ├─ Initialize shell configuration
   └─ Set default theme & language

8. Verify Installation
   ├─ Run Omnisystem startup health check
   ├─ Verify all services online
   ├─ Test file access (read/write)
   ├─ Test network connectivity
   ├─ Test GPU/USB access (if enabled)
   └─ Generate installation report

9. Clean Up
   ├─ Delete temporary files
   ├─ Compress logs
   ├─ Save recovery image
   └─ Update package manager (if Linux)

10. Launch Bonsai Workspace
    ├─ Start Omnisystem (if not running)
    ├─ Connect display (VNC, SPICE, or native)
    ├─ Wait for desktop environment
    ├─ Launch first-time setup wizard
    └─ Display success message
```

**Progress Visualization**:
```
Installing Omnisystem...

[████████░░░░░░░░░░░░░░░░░░░] 38% (3:42 remaining)

Current step: Setting up Hyper-V virtual machine
  • Creating VM...
  • Attaching disk...
  • Configuring network...
```

**Error Handling**:
- If step fails, show error with options: [Retry], [Skip], [Rollback & Cancel]
- Log all errors to `/var/log/bonsai-installer.log`
- On rollback, restore from checkpoint (reverse all steps)

---

### Phase 4: Post-Installation

**Bonsai Launcher** (system entry point):
- Detects Omnisystem not running → starts it
- Shows splash screen with progress
- Displays console output during boot
- Auto-launches Bonsai Workspace on completion

**First-Time Setup Wizard** (inside Omnisystem):
- Timezone & language selection
- Online accounts (if desired)
- Theme selection
- Default applications
- Privacy settings

**Bonsai Control Panel** (system tray app):
- Start/Stop Omnisystem
- Monitor resources
- Manage capabilities
- View logs
- Configure settings

---

## Deployment Modes in Detail

### Co-OS Mode (Hyper-V / KVM / Virtualization.framework)

**Best For**: Production, security-critical workloads, maximum isolation

**Startup**: 10-20 seconds (VM boot time)

**Features**:
- Full hardware isolation
- Can run multiple Omnisystem instances
- Hot snapshots for pause/resume
- GPU passthrough support
- USB passthrough support
- Full network isolation with bridging

**Architecture**:
```
Host OS
  ├─ Bonsai Launcher (start Omnisystem)
  ├─ Bonsai Control Panel (manage Omnisystem)
  ├─ Hypervisor (KVM, Hyper-V, etc.)
  └─ Capability Broker
         │
         └─→ [VM: Omnisystem]
              ├─ Kernel
              ├─ Services
              ├─ Bonsai Workspace
              └─ User applications
```

### Library OS Mode (Syscall Translation)

**Best For**: Lightweight, no hypervisor available, WSL2 fallback

**Startup**: <1 second (direct process execution)

**Features**:
- Direct process execution
- Syscall translation layer
- Minimal overhead
- Limited device access
- File system forwarding via 9P

**Architecture**:
```
Host OS
  ├─ Bonsai Launcher (start Omnisystem)
  ├─ Library OS Translator (syscall interception)
  ├─ Omnisystem Process
  │   ├─ Kernel (in-process)
  │   ├─ Services
  │   ├─ Bonsai Workspace
  │   └─ User applications
  └─ Capability Broker (local enforcement)
```

### Container Mode (Docker / Podman / LXC)

**Best For**: Development, testing, CI/CD pipelines

**Startup**: 2-5 seconds (container launch)

**Features**:
- Namespace isolation
- Filesystem isolation
- Network isolation
- Easy rollback (remove container)
- Compatible with orchestration (Kubernetes, etc.)

**Architecture**:
```
Host OS
  ├─ Container Runtime (Docker, Podman)
  ├─ Bonsai Launcher
  └─ [Container: Omnisystem]
      ├─ Kernel (user-space in container)
      ├─ Services
      ├─ Bonsai Workspace
      └─ User applications
```

---

## Multi-Platform Installer Implementation

### Windows Installer (Bonsai.exe)

**Built With**: WiX Toolset or NSIS

**Detects**:
- Windows version (10, 11, version number, build)
- Hyper-V available and enabled
- RAM, disk space
- Administrator privileges

**Installs**:
- Omnisystem image to `C:\Program Files\Bonsai\omnisystem`
- Bonsai Launcher to system PATH
- Windows service (optional auto-start)
- Capability broker as Windows service

**Creates Shortcuts**:
- Start Menu: "Bonsai Omnisystem"
- Desktop: "Bonsai Workspace"
- System Tray: Bonsai Control Panel

### macOS Installer (Bonsai.dmg)

**Built With**: macOS Installer framework

**Detects**:
- macOS version, architecture (Intel/ARM)
- Virtualization.framework available

**Installs**:
- Omnisystem image to `~/Library/Application Support/Bonsai`
- Bonsai Launcher to `/Applications/Bonsai.app`
- Capability broker as launchd daemon
- System extension (if needed for device passthrough)

**Creates Shortcuts**:
- Launchpad: "Bonsai Omnisystem"
- Menu Bar: Bonsai Control Panel
- Finder: `~/Library/Application Support/Bonsai`

### Linux Installer (bonsai-*.sh or .deb/.rpm)

**Shell Installer**:
```bash
$ curl -fsSL https://installer.bonsai.eco/install.sh | bash
```

**Detects**:
- Distro (Ubuntu, Debian, Fedora, etc.)
- KVM available
- Permissions (sudo)

**Installs**:
- Omnisystem image to `/opt/bonsai/omnisystem`
- Bonsai Launcher to `/usr/local/bin/bonsai`
- systemd service: `bonsaid.service`
- Capability broker via systemd

**DEB Package** (.deb for Ubuntu/Debian):
```bash
$ sudo apt install ./bonsai-latest.deb
```

**RPM Package** (.rpm for Fedora/RHEL):
```bash
$ sudo rpm -i bonsai-latest.rpm
```

### Android Installer (Bonsai.apk)

**Detects**:
- Android version, pKVM available
- RAM, disk space

**Installs**:
- Omnisystem VM/container in app data directory
- System app (if available)
- Capability broker as Android service

**Creates Shortcuts**:
- App icon in launcher
- Home screen widget (optional)
- Notification channel

### iOS Installer (App Store)

**No local installation** – works via remote connection:
- Pairs with Mac or cloud service
- Provides remote UI layer
- Lightweight (~50MB app)

---

## Error Handling & Rollback

**Common Issues & Recovery**:

| Issue | Cause | Recovery |
|-------|-------|----------|
| Hyper-V not available | Windows Home or disabled | Offer Library OS alternative |
| Disk space insufficient | < 50GB free | Show error, request cleanup |
| Port conflict (8000-9000) | Another service using ports | Offer to use different ports |
| Network unreachable | Can't download image | Offer offline install (pre-downloaded image) |
| Permission denied | Not admin/sudo | Offer to re-run with elevated privileges |
| VM won't boot | Corrupt image or config | Rollback, re-download image |

**Rollback Mechanism**:

Every installation creates a **rollback checkpoint** before starting:
```
.bonsai/rollback/2026-06-08_14-32-15/
├── prev_state.json       (snapshot of previous state)
├── installation_log.txt  (what was installed)
└── image_backup/         (backup of old image, if upgrade)
```

If installation fails:
1. Detect failure point
2. Reverse all operations in reverse order
3. Restore from checkpoint
4. Report what went wrong + offer support contact

---

## Success Metrics

✅ Installation time: < 5 minutes (including downloads)  
✅ Post-install boot time: < 20 seconds (Co-OS) or <1 sec (Library OS)  
✅ Failure recovery: 100% rollback success  
✅ User-friendly: No technical jargon, clear progress  
✅ Test coverage: 100+ OS/hardware combinations  

---

**Version**: 1.0.0  
**Status**: Architecture Ready (Implementation Follows)  
**Next**: Implement per-platform installers and detection logic
