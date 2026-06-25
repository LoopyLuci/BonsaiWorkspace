# Omnisystem Deployment Guide

**How to deploy Omnisystem in any environment: Co-OS, VM, container, library OS, or embedded.**

---

## Deployment Modes Overview

| Mode | Isolation | Startup | Use Case | Complexity |
|------|-----------|---------|----------|------------|
| **Co-OS** | Hardware (hypervisor) | 10-20s | Production, security-critical | High |
| **VM** | Hardware (hypervisor) | 15-30s | Development, testing | Medium |
| **Container** | Namespace/cgroup | 2-5s | Development, CI/CD, cloud | Low |
| **Library OS** | Process + syscall translation | <1s | Embedded, legacy systems | Medium |
| **Bare-metal** | Full hardware | 5-10s | Dedicated servers, primary OS | High |

---

## Mode 1: Co-OS Deployment

### What is Co-OS Mode?

Omnisystem runs **alongside** your existing OS (Windows, macOS, Linux) without replacing it. It shares hardware resources but is isolated via hypervisor.

**Benefits**:
- Native Omnisystem performance (95% of bare-metal)
- Host OS still available
- Can share files/network between Omnisystem and host
- Easy to uninstall (remove VM)

**Requirements**:
- Hypervisor available (Hyper-V, KVM, Virtualization.framework)
- 4GB+ RAM
- 50GB+ disk space

### Installation Steps

#### Windows 11 Pro/Enterprise

```powershell
# 1. Download Bonsai Installer
Invoke-WebRequest -Uri "https://releases.bonsai-ai.org/installer/windows/bonsai-installer.exe" `
  -OutFile "bonsai-installer.exe"

# 2. Run installer (will auto-detect Hyper-V)
.\bonsai-installer.exe

# 3. Follow wizard:
# - Select deployment mode (Co-OS recommended)
# - Allocate resources (CPU cores, RAM, disk)
# - Grant capabilities (file access, network, USB, etc.)
# - Wait for installation (~3-5 minutes)

# 4. Launch Omnisystem
# - Click Start Menu → Bonsai Omnisystem
# - Or use CLI:
omnisystem start
```

#### macOS 12+ (Intel/ARM)

```bash
# 1. Download installer
curl -L https://releases.bonsai-ai.org/installer/macos/bonsai-installer.dmg \
  -o bonsai-installer.dmg

# 2. Open and install
open bonsai-installer.dmg
# Then drag Bonsai Installer to Applications

# 3. Run installer
open /Applications/Bonsai\ Installer.app

# 4. Follow wizard (uses Virtualization.framework)

# 5. Launch
launchctl start org.bonsai.omnisystem
# Or click Launchpad → Bonsai Omnisystem
```

#### Linux (Ubuntu 22.04+, Fedora 38+)

```bash
# 1. Download installer
wget https://releases.bonsai-ai.org/installer/linux/bonsai-installer-$(lsb_release -cs).sh

# 2. Run installer
chmod +x bonsai-installer-*.sh
sudo ./bonsai-installer-*.sh

# Follow prompts to:
# - Enable KVM (if needed)
# - Set up libvirt
# - Configure network bridge
# - Allocate resources

# 3. Launch
sudo systemctl start omnisystem
# Or use CLI:
omnisystem start
```

### Co-OS Configuration

After installation, configure via **Bonsai Control Panel**:

```bash
# Open control panel
omnisystem control-panel

# Or command-line:
omnisystem config --cpu 4 --memory 8192 --disk 50
```

### Managing Co-OS

```bash
# Start Omnisystem
omnisystem start

# Check status
omnisystem status
# Output: ● Running (uptime 2h 45m, CPU 15%, RAM 6/16GB)

# Pause (snapshot and pause VM)
omnisystem pause

# Resume from snapshot
omnisystem resume

# Stop gracefully
omnisystem stop

# Force kill (if unresponsive)
omnisystem kill

# Remove (delete VM and data)
omnisystem delete
```

### Network Bridging

By default, Omnisystem gets an IP via DHCP. To configure static IP or bridging:

```bash
# View current network config
omnisystem net show

# Set static IP
omnisystem net ip set 192.168.1.100 255.255.255.0

# Bridge with physical interface (Linux)
omnisystem net bridge --interface eth0

# Forward ports (access Omnisystem service from host)
omnisystem net forward 8080:localhost:8000
```

### Accessing Omnisystem from Host

```bash
# SSH into Omnisystem
ssh omnisystem@192.168.x.x

# Or use included SSH client
omnisystem ssh

# Copy files (SCP-style)
omnisystem copy /local/file omnisystem:/remote/path
omnisystem copy omnisystem:/remote/file /local/path

# Mount Omnisystem filesystem on host (Linux/macOS)
mkdir ~/omnisystem-fs
omnisystem mount ~/omnisystem-fs
# Then access files as: ~/omnisystem-fs/...
```

---

## Mode 2: Virtual Machine Deployment

### Using QEMU/KVM

```bash
# Download disk image
wget https://releases.bonsai-ai.org/images/omnisystem.qcow2

# Boot in QEMU
qemu-system-x86_64 \
  -drive file=omnisystem.qcow2,format=qcow2 \
  -m 4096 \
  -cpu host \
  -enable-kvm \
  -net user,hostfwd=tcp::2222-:22 \
  -display gtk

# Or with more networking
qemu-system-x86_64 \
  -drive file=omnisystem.qcow2 \
  -m 4096 \
  -cpu host \
  -enable-kvm \
  -net bridge,br=br0 \
  -net nic,model=virtio
```

### Using Hyper-V

```powershell
# Download VHD image
$url = "https://releases.bonsai-ai.org/images/omnisystem.vhdx"
Invoke-WebRequest -Uri $url -OutFile "omnisystem.vhdx"

# Create VM
New-VM -Name Omnisystem `
  -MemoryStartupBytes 4GB `
  -SwitchName Default `
  -VHDPath "$PWD\omnisystem.vhdx"

# Start VM
Start-VM -Name Omnisystem

# Connect to console
vmconnect localhost Omnisystem
```

### Using VirtualBox

```bash
# Download OVA (VirtualBox appliance)
wget https://releases.bonsai-ai.org/images/omnisystem.ova

# Import
VBoxManage import omnisystem.ova

# Start
VBoxManage startvm Omnisystem --type headless

# Access via RDP or VNC
# Default: localhost:5900 (VNC)
```

---

## Mode 3: Container Deployment

### Docker

```bash
# Pull image
docker pull bonsai-ai/omnisystem:latest

# Run container
docker run -it --rm bonsai-ai/omnisystem:latest /bin/sylva

# Run with persistent storage
docker run -it --rm \
  -v omnisystem-data:/data \
  bonsai-ai/omnisystem:latest

# Run with GPU support
docker run -it --rm \
  --gpus all \
  bonsai-ai/omnisystem:latest
```

### Kubernetes

```yaml
# omnisystem-pod.yaml
apiVersion: v1
kind: Pod
metadata:
  name: omnisystem
spec:
  containers:
  - name: omnisystem
    image: bonsai-ai/omnisystem:latest
    resources:
      requests:
        memory: "4Gi"
        cpu: "2"
      limits:
        memory: "8Gi"
        cpu: "4"
    volumeMounts:
    - name: data
      mountPath: /data
  volumes:
  - name: data
    emptyDir: {}
```

Deploy:

```bash
kubectl apply -f omnisystem-pod.yaml
kubectl exec -it omnisystem -- /bin/sylva
```

### Docker Compose

```yaml
version: '3.8'
services:
  omnisystem:
    image: bonsai-ai/omnisystem:latest
    container_name: omnisystem
    volumes:
      - omnisystem-data:/data
    ports:
      - "8080:8080"
    environment:
      - OMNISYSTEM_WORKERS=4
      - OMNISYSTEM_LOG_LEVEL=info
    restart: unless-stopped

volumes:
  omnisystem-data:
```

Start:

```bash
docker-compose up -d
docker-compose exec omnisystem /bin/sylva
```

---

## Mode 4: Library OS Deployment

### Embedded in Another OS

Library OS mode embeds Omnisystem into an existing kernel. Useful for:
- Embedded systems (no hypervisor)
- Real-time applications
- Legacy systems

### Building Library OS

```bash
# Build Omnisystem as static library
make library-os
# Output: libomnisystem.a

# Use in C/C++ program
cat > main.c << 'EOF'
#include "omnisystem.h"

int main() {
    omnisystem_init();
    omnisystem_run();
    return 0;
}
EOF

# Compile
gcc main.c -L. -lomnisystem -o my_system

# Run
./my_system
```

### Integration Points

Library OS provides syscalls that translate to:

```
Your Program
    ↓
[Omnisystem Library OS]
    ↓
[Host OS Syscalls]
    ↓
[Hardware]
```

---

## Mode 5: Bare-Metal Deployment

### Create Bootable USB

```bash
# Download ISO
wget https://releases.bonsai-ai.org/images/omnisystem.iso

# Write to USB (Linux)
sudo dd if=omnisystem.iso of=/dev/sdX bs=4M conv=fsync
sudo eject /dev/sdX

# Or use balena-etcher (GUI)
# - Download balena-etcher
# - Select omnisystem.iso
# - Select USB drive
# - Click Flash
```

### Boot Process

1. Insert USB stick
2. Reboot computer
3. Enter boot menu (F12, DEL, ESC, or BIOS key depending on system)
4. Select USB device
5. Wait for Omnisystem to boot (~5-10 seconds)
6. Omnisystem Workspace will appear

### First Boot Configuration

After booting, Omnisystem will show setup wizard:

1. **Timezone & Language** – Select your region
2. **Network** – Configure Ethernet or Wi-Fi
3. **User Account** – Create initial user
4. **Privacy** – Enable/disable telemetry
5. **Complete** – System is ready to use

### Dual Boot (Windows/Linux)

Omnisystem can coexist with Windows or Linux on the same disk:

```bash
# During installation:
# 1. Choose "Install alongside existing OS"
# 2. Select partition size for Omnisystem
# 3. Boot loader will auto-detect both systems
# 4. At startup, choose which OS to boot
```

---

## Mode 6: Cloud Deployment

### AWS EC2

```bash
# 1. Create KVM instance
# (AMI with KVM + libvirt support)
# - Instance type: t3.xlarge (4 vCPU, 16GB RAM)
# - Root volume: 100GB gp2

# 2. SSH into instance
ssh -i key.pem ec2-user@instance-ip

# 3. Download and run Omnisystem
wget https://releases.bonsai-ai.org/installer/linux/omnisystem-installer.sh
bash omnisystem-installer.sh

# 4. Configure as Co-OS
omnisystem config --cpu 4 --memory 8192
omnisystem start

# 5. Forward ports to host
omnisystem net forward 8080:0.0.0.0:8000
```

### Google Cloud Platform

```bash
# Create VM with nested virtualization enabled
gcloud compute instances create omnisystem \
  --image-family=ubuntu-2204-lts \
  --image-project=ubuntu-os-cloud \
  --enable-nested-virtualization \
  --machine-type=n2-standard-4

# Rest is same as AWS
```

### Azure

```bash
# Create VM with nested virtualization
az vm create \
  --resource-group myGroup \
  --name omnisystem \
  --image UbuntuLTS \
  --size Standard_D4s_v3

# Enable nested virtualization in portal:
# VM → Configuration → Enable nested virtualization
```

---

## Scaling Omnisystem

### Single Machine

One Omnisystem instance per machine, managed by Bonsai Control Panel.

### Multiple Machines

Deploy multiple Omnisystem instances across machines using **Omnisystem Cluster Manager** (separate tool):

```bash
# Create cluster
omnisystem-cluster init --nodes 3 --region us-west

# Deploy application
omnisystem-cluster deploy myapp --replicas 3 --placement round-robin

# Monitor cluster
omnisystem-cluster status
```

---

## Backup & Recovery

### Snapshotting

Omnisystem automatically snapshots state:

```bash
# Manual snapshot
omnisystem snapshot save "before-upgrade"

# List snapshots
omnisystem snapshot list

# Restore from snapshot
omnisystem snapshot restore "before-upgrade"

# Delete snapshot
omnisystem snapshot delete "before-upgrade"
```

### Backup Files

```bash
# Backup /data directory
omnisystem backup create /data backup-2026-06-08.tar.gz

# Restore
omnisystem backup restore backup-2026-06-08.tar.gz /data

# Incremental backup (only changed files)
omnisystem backup create --incremental /data backup-incremental.tar
```

---

## Upgrading Omnisystem

### In-Place Upgrade

```bash
# Check for new version
omnisystem update check
# Output: New version 1.1.0 available (you have 1.0.0)

# Download update
omnisystem update download 1.1.0

# Install update (will snapshot before updating)
omnisystem update install 1.1.0

# Verify
omnisystem --version
# Output: Omnisystem 1.1.0
```

### Rollback

If update fails:

```bash
# Restore pre-update snapshot
omnisystem snapshot restore "before-update-1.1.0"

# System will revert to previous version
omnisystem --version
# Output: Omnisystem 1.0.0
```

---

## Monitoring & Logging

### System Monitoring

```bash
# Real-time monitoring
omnisystem monitor

# CPU, memory, disk, network graphs

# Export metrics
omnisystem metrics export prometheus
omnisystem metrics export json
```

### Logs

```bash
# View logs
omnisystem logs --follow

# Filter by service
omnisystem logs --service ai-shim
omnisystem logs --service transfer-daemon

# Export logs
omnisystem logs export /backup/logs.tar.gz
```

---

## Troubleshooting

### Co-OS Won't Start

```bash
# Check hypervisor status
omnisystem check hypervisor

# Check resources
omnisystem check resources

# View detailed error log
omnisystem logs --level debug

# Rollback if recent change
omnisystem snapshot restore last
```

### Poor Performance

```bash
# Check resource allocation
omnisystem config show

# Increase resources
omnisystem config --cpu 8 --memory 16384

# Check for bottlenecks
omnisystem monitor --detailed
```

### Network Issues

```bash
# Check network config
omnisystem net show

# Test connectivity
omnisystem net ping 8.8.8.8

# Restart network
omnisystem net restart
```

---

## Next Steps

1. **Choose deployment mode** – Select the mode that fits your use case
2. **Install** – Follow the mode-specific installation guide
3. **Configure** – Set up resources, network, storage as needed
4. **Monitor** – Use Bonsai Control Panel or CLI for monitoring
5. **Scale** – Add more instances if needed

For more information:
- [Building](BUILD.md) – Build from source
- [Contributing](CONTRIBUTING.md) – Help develop Omnisystem
- [Architecture](ARCHITECTURE.md) – Understand the design

---

**Deployment Guide Version**: 1.0.0  
**Last Updated**: 2026-06-08

