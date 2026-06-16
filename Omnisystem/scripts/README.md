# Omnisystem Scripts Directory
## Build and Development Scripts

**Location**: `Omnisystem/scripts/`  
**Version**: 29.0.0  
**Purpose**: Centralized build, development, and deployment scripts

---

## Directory Structure

```
Omnisystem/scripts/
├── README.md                    (This file)
│
└── build/                       (Build System - Primary)
    ├── README.md               (Build system documentation)
    ├── Build-All.ps1           (Master build orchestrator)
    ├── Build-Omnisystem-GUI.ps1    (GUI executable builder)
    ├── Build-Omnisystem-TUI.ps1    (TUI executable builder)
    ├── .build/                 (GUI build artifacts - temporary)
    └── .build-tui/             (TUI build artifacts - temporary)
```

---

## What's Here

### build/ Subdirectory

**Purpose**: All build scripts for creating Omnisystem executables

**Contains:**
- Master build orchestrator
- GUI launcher builder
- TUI launcher builder
- Build documentation
- Build artifacts (temporary)

**Outputs**: `Omnisystem/launchers/Omnisystem.exe` and `Omnisystem_TUI.exe`

---

## Quick Start

### From Project Root

```powershell
# Build both GUI and TUI
.\Build-All.ps1

# Build GUI and launch
.\Build-GUI.ps1 -Launch

# Build TUI
.\Build-TUI.ps1

# Build everything with clean
.\Build-All.ps1 -Clean
```

---

## Build System Overview

The Omnisystem build system transforms TITAN source code into Windows executables.

**Pipeline**: TITAN → C Code → Windows PE Executable

**Output**: `Omnisystem/launchers/Omnisystem.exe` and `Omnisystem_TUI.exe`

**Requirements**: Clang or MSVC compiler

---

**Status**: ✅ Production Ready  
**Version**: 29.0.0  
**Last Updated**: 2026-06-16

