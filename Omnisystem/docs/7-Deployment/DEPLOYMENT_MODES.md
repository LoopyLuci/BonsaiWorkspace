# OmniOS Deployment Modes

OmniOS ships from a single codebase in six distinct deployment configurations. The same runtime, the same compiler, the same apps — different packaging.

---

## Mode 1: VS Code Extension (Primary)

### Who It's For
Developers and power users who work inside VS Code. This is the primary OmniOS experience.

### How It Works
The VS Code extension activates on startup (`onStartupFinished`). The extension host:
1. Locates the OmniCC binary (`Omnisystem/bin/omnicc.cmd` or via the `omnisystem.omniccPath` setting)
2. Spawns `omnicc runtime --ipc` as a child process
3. Establishes the JSON-RPC 2.0 IPC channel over stdin/stdout
4. Starts the Language Server (`omnicc lsp --stdio`) for all 7 Omni-Languages
5. Registers all sidebar providers, commands, and keybindings
6. Opens the OmniOS Desktop webview panel on `OmniOS: Launch Desktop`

### Setup
```
1. Install "Omnisystem" extension from the VS Code Marketplace (or from VSIX)
2. Open a workspace containing your OmniOS project (or an empty folder)
3. Press Ctrl+Shift+P → "OmniOS: Launch Desktop"
```

### Configuration
All settings live under `omnisystem.*` in VS Code settings:

| Setting | Default | Description |
|---|---|---|
| `omnisystem.omniccPath` | auto-detected | Path to the OmniCC binary |
| `omnisystem.lspEnabled` | `true` | Enable the Language Server |
| `omnisystem.formatOnSave` | `true` | Auto-format on file save |
| `omnisystem.inlayHints` | `true` | Show type inlay hints |
| `omnisystem.axiomVerify` | `false` | Background Axiom verification |
| `omnisystem.buildTarget` | `x86_64-windows` | Default build target |
| `omnisystem.runtimeIpcTimeout` | `30` | IPC request timeout (seconds) |

### File Locations
| Path | Contents |
|---|---|
| `~/.omnisystem/` | OmniOS user data root |
| `~/.omnisystem/packages/` | Installed OmniPM packages |
| `~/.omnisystem/state/` | Desktop and app state JSON files |
| `~/.omnisystem/builds.log` | Build history |
| `~/.omnisystem/config.omnisystem` | Runtime configuration |
| `~/.omnisystem/recent.json` | Recently opened files |

---

## Mode 2: Standalone Application (Tauri)

### Who It's For
Users who do not use VS Code, or who want OmniOS as a native desktop application with system integration (system tray, file associations, global keyboard shortcuts).

### How It Works
The Tauri shell embeds the OmniOS runtime directly (no VS Code, no webview sandboxing). Vera's UI renderer runs natively. The application appears in the taskbar, can start at login, and integrates with the OS notification system.

### Installation
- **Windows**: `OmniOS-2.0.0-x64-setup.exe` — NSIS installer, adds to Start Menu and taskbar, optional startup registration
- **macOS**: `OmniOS-2.0.0-universal.dmg` — drag to Applications, includes both Intel and Apple Silicon binaries
- **Linux**: `omnios_2.0.0_amd64.deb` or `omnios-2.0.0.x86_64.rpm` — installs to `/usr/local/bin/omnios`

### Architecture Differences from VS Code Mode
- Vera renderer used directly (no webview HTML layer)
- System tray icon with quick actions: Open Desktop, Build, Terminal, Quit
- File type associations: `.titan`, `.vera`, `.helix`, `.aether`, `.axiom`, `.sylva`, `.nexus` open in OmniOS Code Studio
- Native OS notifications (not VS Code information messages)
- Global keyboard shortcuts (configurable in Settings)

### Build from Source
```bash
cd apps/standalone
npm install
npm run build         # development build
npm run build:release # optimized release build
```

Requires: Rust (stable), Node.js 20+, platform-specific Tauri prerequisites (see Tauri documentation).

---

## Mode 3: Container (Docker / Podman)

### Who It's For
DevOps engineers, CI/CD pipelines, cloud development environments, reproducible build servers.

### How It Works
The container runs the OmniOS runtime in headless mode, exposing the IPC protocol over a TCP port. A lightweight web UI (served by the runtime) connects to it in a browser.

### Docker Usage
```bash
# Pull the image
docker pull ghcr.io/omnisystem/omnios:latest

# Run interactively, mounting a project directory
docker run -it \
  -p 7878:7878 \       # IPC port
  -p 8080:8080 \       # Web UI port
  -v /path/to/myproject:/workspace \
  ghcr.io/omnisystem/omnios:latest

# Or as a daemon
docker run -d \
  --name omnios-dev \
  -p 7878:7878 -p 8080:8080 \
  -v /path/to/myproject:/workspace \
  ghcr.io/omnisystem/omnios:latest
```

### Connecting
Open `http://localhost:8080` in a browser to access the full OmniOS Desktop UI (WASM version). The UI connects to the runtime IPC over port 7878.

Alternatively, connect a local VS Code extension to the remote runtime by setting:
```json
"omnisystem.runtimeIpcHost": "localhost:7878"
```

### Docker Compose Example
```yaml
version: '3.8'
services:
  omnios:
    image: ghcr.io/omnisystem/omnios:latest
    ports:
      - "7878:7878"
      - "8080:8080"
    volumes:
      - ./myproject:/workspace
      - omnios-state:/root/.omnisystem
    environment:
      - OMNIOS_DEFAULT_TARGET=x86_64-linux
      - OMNIOS_OPT=O2
    restart: unless-stopped

volumes:
  omnios-state:
```

### CI/CD Integration

**GitHub Actions:**
```yaml
jobs:
  build:
    runs-on: ubuntu-latest
    container:
      image: ghcr.io/omnisystem/omnios:latest
    steps:
      - uses: actions/checkout@v4
      - name: Build
        run: omnicc build --target x86_64-linux --release
      - name: Test
        run: omnicc test --verbose
```

**GitLab CI:**
```yaml
build:
  image: ghcr.io/omnisystem/omnios:latest
  script:
    - omnicc build --target x86_64-linux --release
    - omnicc test
  artifacts:
    paths:
      - target/x86_64-linux/
```

---

## Mode 4: WASM Browser App

### Who It's For
Users who want zero-install access to OmniOS. Also the primary mode for the welcome/onboarding flow at `omnisystem.dev`.

### How It Works
The OmniCC runtime is compiled to WebAssembly via Binaryen. The WASM module runs in a Web Worker so it does not block the main thread. The full OmniOS Desktop UI renders in a browser tab.

**Persistence**: Uses the Origin Private File System (OPFS) API for file system storage. Projects survive page reloads and browser restarts. Storage is per-origin (sandboxed to `omnisystem.dev`).

**Performance**: WASM+SIMD is approximately 70-80% of native speed for compute-bound workloads. Build times are slightly longer than native but still fast for typical projects.

### Access
Navigate to: `https://omnisystem.dev/desktop`

No login required. Projects are stored locally in your browser.

### Limitations vs Native
- Cannot access the host filesystem directly (only OPFS)
- Cannot spawn native PTY shells (Terminal uses a simulated shell with OmniOS builtins)
- Cannot run native binaries (only WASM output from OmniCC)
- No system tray, no file associations, no global shortcuts
- No Bonsai Launcher or Android Buddy (native process management unavailable)

### Self-Hosting the WASM Build
```bash
cd web
npm install
npm run build         # produces dist/
npm run serve         # serve locally at http://localhost:3000
```

Deploy the `dist/` directory to any static file host (Netlify, Vercel, S3, GitHub Pages).

---

## Mode 5: Virtual Machine Image (QEMU)

### Who It's For
Users who want a fully isolated OmniOS environment. Educators running OmniOS in teaching labs. Users who want to test OmniOS without affecting their host system.

### How It Works
A bootable QEMU-compatible disk image containing:
- GRUB bootloader
- Minimal Linux kernel (6.x LTS)
- OmniOS runtime and full toolchain pre-installed
- OmniPM registry cache for offline use
- OmniOS Desktop (WASM browser mode via a local Chromium instance)

### Usage
```bash
# Download
wget https://releases.omnisystem.dev/omnios-2.0.0-vm.qcow2

# Run with QEMU (4 GB RAM, KVM acceleration)
qemu-system-x86_64 \
  -m 4G \
  -enable-kvm \
  -drive file=omnios-2.0.0-vm.qcow2,format=qcow2 \
  -vga virtio \
  -display sdl \
  -net nic -net user,hostfwd=tcp::8080-:8080

# Access the desktop at http://localhost:8080 in the host browser
# Or use the VM's built-in display (boots to Openbox + Chromium in kiosk mode)
```

### VirtualBox Usage
Import `omnios-2.0.0.ova` into VirtualBox. The VM is pre-configured with 4 GB RAM, 2 CPUs, and VirtualBox Guest Additions.

### Shared Folders
Mount a host directory into the VM for file exchange:
```bash
# QEMU: use 9p filesystem
qemu-system-x86_64 \
  ... \
  -virtfs local,path=/host/myproject,mount_tag=hostprojects,security_model=passthrough
```
Inside the VM: `mount -t 9p hostprojects /mnt/projects`

---

## Mode 6: Bare Metal (Future Milestone)

### Status
Under active research. Estimated availability: 18-24 months.

### What Is Being Built

**Stage 1 — UEFI Bootloader (Titan)**
- UEFI application written in Titan
- Loads the OmniOS kernel image from the EFI partition
- Sets up memory map and passes it to the kernel
- Initializes the serial console for early boot logging

**Stage 2 — OmniOS Kernel (Titan + Helix + Aether)**
- Written entirely in Omni-Languages (no C runtime dependency)
- Memory management: physical page allocator, virtual memory manager, demand paging
- Process management: Aether-based cooperative/preemptive scheduler
- Device drivers: AHCI (SATA storage), NVMe, USB HID (keyboard/mouse), framebuffer display
- Syscall interface: compatible subset of POSIX (read, write, open, close, mmap, fork, exec)
- Network stack: TCP/IP written as a set of Aether actors

**Stage 3 — OmniOS Desktop on Bare Metal**
- Vera UI renderer running directly on the framebuffer (no X11, no Wayland, no Electron)
- OmniCC compiler running natively on the OmniOS kernel
- Full OmniOS Desktop app suite

### Trying the Early Builds
Early bare metal builds will be published as QEMU images with the OmniOS kernel replacing the Linux kernel. Subscribe to the Omnisystem newsletter for announcements.

---

## Comparison Matrix

| Feature | VS Code Extension | Standalone App | Container | WASM Browser | VM Image | Bare Metal |
|---|---|---|---|---|---|---|
| Real terminal (PTY) | ✓ | ✓ | ✓ | Simulated | ✓ | ✓ |
| Host filesystem | ✓ | ✓ | Mounted | OPFS only | Shared folder | Native |
| Real build output | ✓ | ✓ | ✓ | WASM only | ✓ | ✓ |
| LSP / IntelliSense | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| OmniPM | ✓ | ✓ | ✓ | Limited | ✓ | ✓ |
| Bonsai Launcher | ✓ | ✓ | ✗ | ✗ | ✗ | ✓ |
| System tray | ✗ | ✓ | ✗ | ✗ | ✗ | ✓ |
| Offline | Partial | ✓ | ✓ | ✓ | ✓ | ✓ |
| Zero install | ✗ | ✗ | docker pull | ✓ | ✗ | ✗ |
| VS Code integration | Native | ✗ | Via remote | ✗ | ✗ | ✗ |
| Status | Production | Beta | Beta | Beta | Alpha | Research |
