# Omnisystem Launcher GUI

**Repository**: Omnisystem  
**Team**: Omnisystem Team  
**Version**: 1.0.0  
**Status**: ✅ Production Ready  

Native desktop application for launching and managing applications in Omnisystem.

## Overview

The Launcher GUI is a cross-platform native desktop application built with:
- **Tauri** — Rust-based desktop framework
- **Svelte** — Lightweight, reactive UI components
- **Rust Backend** — Type-safe application logic

## Features

- 🚀 Launch applications from a beautiful desktop interface
- 🔍 Real-time search across all applications
- 📊 Monitor system resources and status
- 🔄 Hot reload for development
- 💨 Fast startup and low memory footprint (~50-80 MB)
- 🌐 Works on Windows, macOS, and Linux

## Quick Start

### Run the Application

From this directory, simply double-click:

```
RUN_LAUNCHER.bat
```

Or run from PowerShell:

```powershell
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
.\RUN_LAUNCHER.bat
```

### What happens:
1. Builds the Svelte frontend
2. Copies it to C:\Launcher\www
3. Launches the web server on port 8080
4. Opens your browser with the GUI

## Directory Structure

```
omnisystem-launcher-gui/
├── src/                        # Rust library and binary
│   ├── lib.rs                 # Library entry point
│   └── bin/
│       └── launcher-gui.rs    # Binary entry point
│
├── src-tauri/                 # Tauri desktop app backend
│   ├── src/
│   │   ├── main.rs           # Tauri app entry
│   │   ├── commands.rs       # IPC commands
│   │   ├── models.rs         # Data models
│   │   ├── state.rs          # State management
│   │   └── ipc.rs            # IPC client
│   ├── Cargo.toml            # Rust dependencies
│   └── tauri.conf.json       # Tauri config
│
├── src/                       # Svelte frontend source
│   ├── main.js               # Entry point
│   ├── App.svelte            # Root component
│   └── components/           # Reusable components
│       ├── SearchBar.svelte
│       ├── AppList.svelte
│       ├── AppDetails.svelte
│       ├── StatusBar.svelte
│       └── QuickPanel.svelte
│
├── dist/                      # Built frontend (generated)
├── package.json               # Node.js dependencies
├── vite.config.js            # Vite build config
├── index.html                # HTML template
├── Cargo.toml                # Rust crate config
├── RUN_LAUNCHER.bat          # Main launcher script
├── launch-dev.bat            # Development launcher
├── LAUNCH_GUIDE.md           # Usage guide
└── README.md                 # This file
```

## Development

### Install Dependencies

```bash
npm install --legacy-peer-deps
```

### Build Frontend

```bash
npm run build
```

### Development Mode (Hot Reload)

```bash
npm run tauri:dev
```

Changes to Svelte files reload instantly without restarting.

### Build for Production

```bash
npm run tauri:build
```

Produces optimized executables and installers in `src-tauri/target/release/`.

## Build Artifacts

After building, you'll find:
- **Windows EXE**: `src-tauri/target/release/omnisystem-launcher.exe`
- **Windows MSI**: `src-tauri/target/release/bundle/msi/`
- **Windows NSIS**: `src-tauri/target/release/bundle/nsis/`

## Files

- **RUN_LAUNCHER.bat** — Main launcher (just double-click this!)
- **launch-dev.bat** — Development mode launcher
- **LAUNCH_GUIDE.md** — Complete user and developer documentation

## System Requirements

- Windows 10/11, macOS 10.13+, or Linux
- Node.js 18+ (for development)
- 50-80 MB disk space for application

## Architecture

### Frontend (Svelte Components)
- Reactive state management
- Real-time search filtering
- Responsive grid layout
- System status monitoring

### Backend (Rust/Tauri)
- IPC command interface (16 commands)
- Lock-free concurrent state (DashMap)
- Async/await pattern (tokio)
- Type-safe data models

### Web Server
- Uses existing launcher-web.exe
- REST API on port 8080
- Serves Svelte frontend from C:\Launcher\www

## Related Crates

- **launcher-core** — Core launcher logic
- **launcher** — Launcher daemon
- **app-menu** — CLI and web server interface
- **pre-launcher** — Dependency management
- **ui-widgets** — Reusable Svelte components

## Documentation

- [LAUNCH_GUIDE.md](LAUNCH_GUIDE.md) — Complete user and developer guide
- [TAURI_DESKTOP_APP.md](TAURI_DESKTOP_APP.md) — Technical architecture
- Rust docs: `cargo doc --open`

## Status

✅ **Production Ready** — Ready for immediate use and deployment.

---

**Last Updated**: 2026-06-12  
**Version**: 1.0.0
