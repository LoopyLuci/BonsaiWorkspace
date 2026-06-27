# Omnisystem App Menu — Complete User Guide

**Version:** 28.0.0  
**Status:** Production Ready  
**Last Updated:** 2026-06-16

## Overview

The Omnisystem App Menu is the main interface that users see when launching `Omnisystem.exe`. It provides:

- **One-click access** to all 11 BonsaiEcosystem applications
- **Visual status indicators** for all services
- **System health information** at a glance
- **Quick launcher** for common tools
- **Professional dashboard** interface

## What Happens When You Launch Omnisystem.exe

```
1. Omnisystem.exe starts
   ↓
2. BonsaiEcosystem initializes (5 phases, ~3-5 seconds)
   - Service registry registration
   - Infrastructure initialization
   - Application services launch
   - OS-level integration
   - Health checks and verification
   ↓
3. Omnisystem App Menu displays
   - Window: 1800x1000 pixels
   - Dark theme with accent colors
   - All 11 apps visible and ready
   ↓
4. User can launch any app by clicking
```

## The Omnisystem App Menu Interface

### Header Section (90px)
- **Title:** "OMNISYSTEM v28.0.0"
- **Subtitle:** "Enterprise Operating System | BonsaiEcosystem Launcher"
- **Status Display:** All services operational
- **Visual:** Dark header with orange accent bar

### Main Content (5 Sections)

#### Section 1: 🌿 BONSAI ECOSYSTEM (5 Applications)

**Workspace IDE**
- Icon: 💻
- Status: ✓ READY
- Description: Multi-Language IDE (TITAN/SYLVA/AETHER/AXIOM)
- Action: Click card to launch

**Buddy AI**
- Icon: 🤖
- Status: ✓ READY
- Description: AI Assistant (6 providers ready)
- Action: Click card to launch

**App Launcher**
- Icon: 📱
- Status: ✓ READY
- Description: Application Manager (11 apps indexed)
- Action: Click card to launch

**Browser Extension**
- Icon: 🌐
- Status: ✓ READY
- Description: Web Integration (4 platforms)
- Action: Click card to launch

**Control Panel**
- Icon: ⚙️
- Status: ✓ READY
- Description: System Monitor (port 12345)
- Action: Click card to launch or visit http://localhost:12345

#### Section 2: ⚡ OMNISYSTEM CORE (4 Tools)

**TITAN Compiler**
- Icon: 🔷
- Status: ✓ READY
- Description: Language Compiler (All 7 languages)
- Action: Click card to launch

**Debugger**
- Icon: 🐛
- Status: ✓ READY
- Description: Debug Tools (Breakpoints & trace)
- Action: Click card to launch

**Profiler**
- Icon: 📊
- Status: ✓ READY
- Description: Performance Analysis (CPU/memory/network)
- Action: Click card to launch

**Documentation**
- Icon: 📚
- Status: ✓ READY
- Description: Complete API Docs (3,500+ functions)
- Action: Click card to open

#### Section 3: 🔧 SYSTEM SERVICES (5 Services)

**Notification System**
- Icon: 📬
- Status: ✓ Running
- Description: SQLite persistence | Cross-platform delivery

**System Tray**
- Icon: 📌
- Status: ✓ Running
- Description: OS-level integration | 11-item menu

**File Associations**
- Icon: 📄
- Status: ✓ Running
- Description: 7 file types | Context menus

**Theme System**
- Icon: 🎨
- Status: ✓ Running
- Description: 10 themes | Custom colors & fonts

**Installer**
- Icon: 📦
- Status: ✓ Running
- Description: Cross-platform | Dependency management

### Footer Section (70px)
- **System Status:** All services running
- **Last Initialized:** Timestamp
- **Version Info:** 28.0.0 | PRODUCTION | READY
- **Keyboard Shortcuts:** F1 (help), Ctrl+, (settings), Alt+F4 (close)
- **Quick Status:** 11 apps ready to launch

## Application Cards

Each application is displayed as a clickable card with:

### Visual Design
- **Background:** Dark panel color with accent border (top)
- **Width:** 250px
- **Height:** 160px
- **Icon:** Large emoji icon in accent color
- **Title:** Application name in white
- **Subtitle:** Application category in gray
- **Status Badge:** "✓ READY" in green
- **Description:** Key features in gray
- **Call-to-Action:** "[CLICK TO LAUNCH]" in accent color

### Interactive Features
- Cards are clickable for launching apps
- Hover effect (visual feedback)
- Status color changes on error (would show red)
- Quick access to settings

## Launching Applications

### From the App Menu

1. **Locate the application** in the menu
2. **Click the application card**
3. **Application launches** in ~1-2 seconds

### Keyboard Shortcuts

- **F1** — Help and keyboard shortcuts
- **Ctrl+,** (comma) — Settings
- **Ctrl+1-5** — Quick launch Bonsai apps (1=Workspace, 2=Buddy, etc)
- **Alt+F4** — Close menu and shutdown gracefully

## System Status Indicators

### Green Status (✓ READY)
- Application is initialized and ready
- All dependencies satisfied
- No errors detected

### Yellow Status (⚠ WARNING)
- Application is running but with warnings
- Some optional features unavailable
- User should investigate

### Red Status (✗ ERROR)
- Application failed to initialize
- Not available for launch
- Check logs for details

## Menu Navigation

### Mouse Navigation
- Click cards to launch apps
- Scroll down to see all sections
- Hover for tooltips

### Keyboard Navigation
- Tab: Move between app cards
- Enter: Launch selected app
- Arrow keys: Navigate between cards

## First-Time Setup

When you first launch Omnisystem:

1. **BonsaiEcosystem initializes** (3-5 seconds)
   - You'll see initialization logs
   - All services starting up
   - Health checks running

2. **App Menu appears** (automatically)
   - All 11 apps visible
   - System status green
   - Ready to use

3. **Optional: Configure**
   - Press Ctrl+, to open settings
   - Choose theme (10 options)
   - Configure AI providers
   - Set file associations

## Troubleshooting

### An app shows red status (ERROR)
1. Click the app card for error details
2. Check logs: Omnisystem/logs/
3. Run repair: Open Control Panel → Diagnostics
4. Restart the app

### All apps show yellow (WARNING)
1. Check internet connection
2. Verify all dependencies installed
3. Run system diagnostics (Ctrl+Shift+D)
4. Restart Omnisystem

### App Menu won't load
1. Check TITAN compiler is installed
2. Verify Omnisystem path is correct
3. Check logs for initialization errors
4. Try restarting Windows/macOS/Linux

## Advanced Usage

### Launch from Command Line
```bash
# Launch specific app from terminal
omnisystem-cli launch workspace
omnisystem-cli launch buddy
omnisystem-cli launch launcher

# Get app status
omnisystem-cli status all

# View health check results
omnisystem-cli health
```

### Access Control Panel Directly
```
http://localhost:12345
```
- System statistics
- Service management
- Capability browser
- Settings

### View System Logs
- Windows: `%APPDATA%\Omnisystem\logs\`
- macOS: `~/Library/Logs/Omnisystem/`
- Linux: `~/.local/share/Omnisystem/logs/`

## Keyboard Shortcut Reference

| Shortcut | Action |
|----------|--------|
| F1 | Help & Keyboard Shortcuts |
| Ctrl+, | Settings |
| Ctrl+1 | Quick launch Workspace IDE |
| Ctrl+2 | Quick launch Buddy AI |
| Ctrl+3 | Quick launch App Launcher |
| Ctrl+4 | Quick launch Browser Ext |
| Ctrl+5 | Quick launch Control Panel |
| Ctrl+Shift+D | Run Diagnostics |
| Alt+F4 | Close Menu & Shutdown |
| Tab | Move between cards |
| Enter | Launch selected app |

## System Requirements

### Minimum
- 4GB RAM
- 2GB disk space
- Windows 10+, macOS 10.14+, Ubuntu 18.04+
- TITAN compiler installed

### Recommended
- 8GB+ RAM
- SSD with 5GB+ space
- Latest OS version
- GPU for graphics acceleration

## Performance

- **App Menu load time:** < 500ms
- **App launch time:** 1-2 seconds
- **Memory usage:** ~150MB for all services
- **CPU idle:** < 1%

## What's Running

After BonsaiEcosystem initialization, these services are always running:

1. **Control Panel** — Port 12345
2. **Notification Daemon** — Background
3. **System Tray** — OS taskbar/menu bar
4. **File Association Handler** — Background
5. **Theme Engine** — Background

These consume minimal resources and can be toggled in settings.

## Next Steps

1. **Explore the Dashboard**
   - Click around and see each section
   - Check Control Panel for stats
   - Review system health

2. **Launch Your First App**
   - Start with Workspace IDE
   - Try Buddy AI
   - Explore the Compiler

3. **Configure Your System**
   - Choose your theme
   - Set up AI providers
   - Configure file associations

4. **Read Documentation**
   - Click "Documentation" card
   - Visit https://omnisystem.dev
   - Read API reference

## Support

### Getting Help
- **F1** in App Menu for help
- Control Panel diagnostics
- Check logs in Omnisystem/logs/
- Report issues at GitHub

### Community
- GitHub discussions
- Discord community
- Documentation wiki
- Email support

---

**Version 28.0.0** — Production Ready  
**Last Updated:** 2026-06-16  
**Status:** All systems operational ✓
