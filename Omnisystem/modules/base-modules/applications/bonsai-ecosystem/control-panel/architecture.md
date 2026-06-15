# Bonsai Control Panel – System Tray / Menu Bar Manager

**Real-Time Omnisystem Management & Monitoring**

---

## Overview

The **Bonsai Control Panel** is a lightweight system tray (Windows/Linux) / menu bar (macOS) application that provides one-click access to Omnisystem management:

- **Start/Stop/Pause/Resume** – Control Omnisystem lifecycle
- **Resource Monitoring** – Real-time CPU, memory, disk, network graphs
- **Capability Management** – Grant/revoke file access, network, devices
- **Service Management** – Start/stop individual services (AI, Transfer, etc.)
- **Settings** – Adjust VM resources, enable/disable features
- **Snapshots** – Quick save/restore VM states for testing
- **Logs** – View system events and debug information
- **Updates** – Check for and apply Omnisystem updates

---

## Platform-Specific Implementation

### Windows (System Tray)

**Technology**: C# WPF (Windows Presentation Foundation) or XAML

**Appearance**:
```
System Tray:
  ├─ Icon: Bonsai leaf (green = running, gray = stopped, yellow = paused)
  └─ Tooltip: "Bonsai Omnisystem (Running, 45% CPU, 8GB/16GB RAM)"

Right-click Menu:
  ├─ ▶ Resume         (if paused)
  ├─ ⏸ Pause          (if running)
  ├─ ◼ Stop           (if running)
  ├─ ────────
  ├─ 📊 Dashboard
  ├─ ⚙ Settings
  ├─ 📝 Logs
  ├─ ────────
  ├─ 🔄 Check Updates
  ├─ ❓ Help & Support
  └─ ✕ Exit
```

**Features**:
- Double-click icon to open dashboard
- Auto-start option (registry HKLM\Run)
- Silent mode (notifications only on critical events)
- Integration with Windows Notifications

### macOS (Menu Bar)

**Technology**: Swift AppKit

**Appearance**:
```
Menu Bar (top-right):
  ├─ Icon: Bonsai leaf (black on white background)
  └─ Tooltip: "Bonsai (Running, 45% CPU)"

Menu:
  ├─ ▶ Resume         (if paused)
  ├─ ⏸ Pause          (if running)
  ├─ ◼ Stop           (if running)
  ├─ ─────────────
  ├─ 📊 Open Dashboard
  ├─ ⚙ Settings
  ├─ 📝 View Logs
  ├─ ─────────────
  ├─ 🔄 Check Updates
  ├─ ❓ Help & Support
  └─ ✕ Quit

Dock:
  └─ Bonsai.app (running indicator)
```

**Features**:
- Click icon to show menu
- Dock icon shows when running
- Notification Center integration
- macOS Spotlight search integration

### Linux (System Tray / App Indicator)

**Technology**: Python PyQt6 or Rust GTK

**Appearance** (GNOME/KDE):
```
System Tray:
  ├─ Icon: Bonsai leaf
  └─ Tooltip: "Bonsai Omnisystem (Running)"

Menu:
  ├─ ▶ Resume
  ├─ ⏸ Pause
  ├─ ◼ Stop
  ├─ ────────
  ├─ 📊 Dashboard
  ├─ ⚙ Settings
  ├─ 📝 Logs
  ├─ ────────
  ├─ 🔄 Check Updates
  ├─ ❓ Help
  └─ ✕ Quit

Desktop Entry:
  ~/.config/autostart/bonsai.desktop
  (auto-start on login)
```

**Features**:
- DBus integration (systemd notify)
- XDG Desktop Entry for autostart
- Freedesktop notifications

---

## Dashboard UI Layout

### Main Tab: Status & Control

```
┌──────────────────────────────────────────────────────────────┐
│ Bonsai Omnisystem Control Panel                        [_][□][×] │
├──────────────────────────────────────────────────────────────┤
│ Status│Resources│Capabilities│Services│Snapshots│Settings│Logs │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  Status: ● RUNNING                                          │
│  Uptime: 12 hours 45 minutes                                │
│  Version: 1.0.0 (build 2026.06.08)                          │
│                                                              │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │ Quick Controls:                                         │ │
│  │ [⏸ Pause] [◼ Stop] [📸 Snapshot] [🔄 Update]           │ │
│  └─────────────────────────────────────────────────────────┘ │
│                                                              │
│  Next scheduled update: 2026-06-15 14:00                    │
│  Security: All systems nominal                              │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

### Resources Tab: Performance Monitoring

```
┌──────────────────────────────────────────────────────────────┐
│ Resources Tab                                                │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  CPU: 45% ████████░░░░░░░░░░░░ (4/8 cores)                 │
│    Graph: [████████░░░░░░░░░░░░░░░] (last 24h)              │
│                                                              │
│  Memory: 8.2GB ████████░░░░░░░░░░ (16GB total)             │
│    Graph: [░░░████████░░░░░░░░░░░░] (last 24h)              │
│    Swap: 512MB ░░░░░░░░░░░░░░░░░░░░                         │
│                                                              │
│  Disk: 32GB ████████░░░░░░░░░░░░░░░░░░░░ (50GB total)      │
│    Graph: [░░░░░░░░░░░░░░░░░░░░░░░░░░░░░] (7 days)         │
│    Cache: 2.1GB                                              │
│                                                              │
│  Network:                                                    │
│    ↓ In:  12.5 Mbps  ├─ TCP: 8.2 Mbps                      │
│    ↑ Out: 3.2 Mbps   ├─ QUIC: 2.1 Mbps                     │
│                      ├─ WebRTC: 1.8 Mbps                   │
│                      └─ Relay: 0.4 Mbps                    │
│    Graph: [████░░░░░░░░░░░░░░░░░░░░░░░░░] (last 6h)        │
│                                                              │
│  GPU: NVIDIA RTX 4090 (Used: 6.2GB / 24GB)                │
│    Temperature: 62°C                                         │
│    Clock: 2.4GHz                                             │
│                                                              │
│  ⚙ Auto-scale resources ☑  [Configure]                     │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

### Capabilities Tab: Access Control

```
┌──────────────────────────────────────────────────────────────┐
│ Capabilities Tab                                             │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  File System:                                                │
│    ☑ /home/user/Documents       ← Hover: [✕ Revoke]       │
│    ☑ /home/user/Downloads       ← Hover: [✕ Revoke]       │
│    ☑ /home/user/Pictures        ← Hover: [✕ Revoke]       │
│    ☐ /home/user/Private         ← Hover: [+ Grant]        │
│                                                              │
│  Network:                                                    │
│    ☑ Outbound (all)             ← Hover: [✕ Revoke]       │
│    ☑ Inbound 0.0.0.0:8000-9000  ← Hover: [✕ Revoke]       │
│    ☐ DNS resolving              ← Hover: [+ Grant]        │
│                                                              │
│  Hardware:                                                   │
│    ☑ USB devices (all)          ← Hover: [✕ Revoke]       │
│    ☐ GPU compute                ← Hover: [+ Grant]        │
│    ☑ Audio output               ← Hover: [✕ Revoke]       │
│    ☐ Audio input                ← Hover: [+ Grant]        │
│    ☑ Clipboard                  ← Hover: [✕ Revoke]       │
│    ☐ Camera                     ← Hover: [+ Grant]        │
│                                                              │
│  System:                                                     │
│    ☑ Time/Date access           ← Hover: [✕ Revoke]       │
│    ☑ Random number generator    ← Hover: [✕ Revoke]       │
│                                                              │
│  [+ Add Custom Capability] [Import Policy]                  │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

### Services Tab: Individual Service Control

```
┌──────────────────────────────────────────────────────────────┐
│ Services Tab                                                 │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  Service Name           Status    CPU    Memory  Uptime     │
│  ─────────────────────────────────────────────────────────  │
│  ● TransferDaemon       Running   8.2%   512MB   12h 45m   │
│  ● AI Orchestrator      Running   22.1%  2.4GB   12h 45m   │
│  ● UMS Collector        Running   2.1%   256MB   12h 45m   │
│  ● File Vault Service   Running   1.2%   128MB   12h 45m   │
│  ● Network Stack        Running   3.4%   384MB   12h 45m   │
│  ● Capability Broker    Running   0.8%   64MB    12h 45m   │
│  ● Bonsai Workspace     Running   7.2%   1.2GB   12h 30m   │
│  ○ Backup Service       Stopped   —      —       —         │
│                                                              │
│  Right-click service for [Restart], [Stop], [View Logs]    │
│                                                              │
│  [Start All] [Stop All] [Restart All]                      │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

### Snapshots Tab: VM State Management

```
┌──────────────────────────────────────────────────────────────┐
│ Snapshots Tab                                                │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  Create Snapshot: [📸 Snapshot Now] [Description: ________] │
│                                                              │
│  Snapshot ID           Created           Size   Auto-Delete │
│  ─────────────────────────────────────────────────────────  │
│  snap-2026-06-08-14:32 2026-06-08 14:32  3.2GB  ☐ (7 days)│
│  snap-2026-06-07-09:15 2026-06-07 09:15  3.2GB  ☑ (today) │
│  snap-2026-06-06-18:00 2026-06-06 18:00  3.2GB  ☑ (del)   │
│                                                              │
│  Right-click for:                                            │
│  [Restore] [Delete] [Export] [Properties]                  │
│                                                              │
│  Snapshot frequency: ◯ Manual ◉ Auto (daily) ◯ Never       │
│  Auto-delete older than: [30_] days                        │
│  Max snapshots to keep: [10_]                              │
│                                                              │
│  [Save Configuration]                                        │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

### Settings Tab: Configuration

```
┌──────────────────────────────────────────────────────────────┐
│ Settings Tab                                                 │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  General:                                                    │
│    ☑ Auto-start on login                                   │
│    ☑ Show system tray icon                                 │
│    ☑ Minimize to tray on close                             │
│    ☑ Check for updates automatically                       │
│                                                              │
│  Resources:                                                  │
│    CPU Cores: [4___] (2-8 available)                       │
│    Memory (GB): [8____] (2-16 available)                   │
│    Disk (GB): [50_______] (10-500 available)              │
│    ☑ Auto-scale under load                                 │
│                                                              │
│  Network:                                                    │
│    Bridge mode: ◉ Automatic ◯ Manual                       │
│    Network adapter: [Ethernet v]                           │
│    Bandwidth limit: [None___] (Mbps)                       │
│    ☑ Monitor network activity                              │
│                                                              │
│  Display:                                                    │
│    Theme: ◉ System default ◯ Light ◯ Dark                 │
│    Language: [English v]                                   │
│    Resolution: [1920x1080] ◯ Fullscreen                   │
│                                                              │
│  Advanced:                                                   │
│    Log level: [Debug___] ← Hover: shows explanation        │
│    ☑ Enable performance profiling                          │
│    ☑ Enable telemetry (help improve Bonsai)               │
│    ☑ Enable security alerts                                │
│                                                              │
│  [Reset to Defaults] [Apply] [Cancel]                      │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

### Logs Tab: System Events

```
┌──────────────────────────────────────────────────────────────┐
│ Logs Tab                                                     │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  Level: [All___] Time: [Today__] Service: [All_____] Search │
│                                                              │
│  [INFO]  12:45:32  TransferDaemon  Peer discovered: alice  │
│  [INFO]  12:45:28  UMS Collector   Metrics collected      │
│  [DEBUG] 12:45:20  Kernel          Page fault resolved     │
│  [WARN]  12:45:10  AI Orchestrator GPU memory low (18GB)  │
│  [INFO]  12:45:00  Workspace       User logged in: user1  │
│  [ERROR] 12:44:50  Network Stack   DNS timeout (retry)    │
│  [INFO]  12:44:45  Backup Service  Backup completed (2GB) │
│                                                              │
│  [Double-click for full details]                           │
│                                                              │
│  [Export Logs] [Clear Logs] [Open Log File]                │
│                                                              │
│  Auto-scroll: ☑   Buffer: Last 1000 events                │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

---

## Communication Protocol

**Inter-process Communication** (Control Panel ↔ Omnisystem):

**Windows**:
- Named pipes: `\\.\pipe\bonsai-control`
- Protocol: JSON-RPC 2.0

**macOS**:
- XPC (inter-process communication)
- Protocol: Property lists (plist)

**Linux**:
- D-Bus system service
- Interface: `org.bonsai.ControlPanel`
- Protocol: DBus method calls

**Example JSON-RPC Request** (Windows):
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "vm.pause",
  "params": {}
}

Response:
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "success": true,
    "status": "paused",
    "message": "VM paused successfully"
  }
}
```

---

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl+P` (Win/Linux), `⌘+P` (Mac) | Pause/Resume |
| `Ctrl+S` (Win/Linux), `⌘+S` (Mac) | Stop |
| `Ctrl+D` | Open Dashboard |
| `Ctrl+,` (Win/Linux), `⌘+,` (Mac) | Settings |
| `Ctrl+L` | Show Logs |
| `Ctrl+U` | Check Updates |
| `Ctrl+Q` (Win/Linux), `⌘+Q` (Mac) | Exit |

---

## Notifications

**Critical Events** (always notify):
- Omnisystem crashed → "Omnisystem encountered an error. [Restart] [Report]"
- Security issue → "Security alert detected. [Review] [Dismiss]"
- Update available → "New version available. [Update Now] [Later]"

**Warning Events** (if enabled in settings):
- High CPU usage (>80%) → "CPU usage is high"
- Low disk space (<10GB) → "Disk space running low"
- Memory pressure → "System memory is under pressure"

**Info Events** (quiet, only in notification center):
- Service started/stopped
- Backup completed
- Update installed

---

## Performance & Memory

**Target Specifications**:
- Memory footprint: <50MB (idle), <100MB (with dashboard open)
- CPU usage: <1% (idle), <3% (monitoring)
- Startup time: <1 second
- Dashboard response time: <500ms
- Update check latency: background (no UI blocking)

---

## Success Criteria

✅ Single-click access to all Omnisystem management functions  
✅ Real-time monitoring of all metrics  
✅ Smooth animations and responsive UI  
✅ Works on Windows, macOS, Linux (platform-native feel)  
✅ Zero configuration required (works out-of-box)  
✅ Clear, actionable error messages  

---

**Version**: 1.0.0  
**Status**: Architecture Ready (Implementation Follows)  
**Next**: Implement platform-specific UIs (WPF, AppKit, PyQt6)
