# Omnisystem Build Instructions

**All build scripts are located in**: `Omnisystem/scripts/build/`  
**All executables are built to**: `Omnisystem/launchers/`

---

## Quick Start

### Build Everything

```powershell
# Navigate to build directory
cd Omnisystem/scripts/build

# Run master build script
.\Build-All.ps1
```

Or directly from anywhere:

```powershell
PowerShell -ExecutionPolicy Bypass -File "Omnisystem/scripts/build/Build-All.ps1"
```

### Build GUI Only

```powershell
cd Omnisystem/scripts/build
.\Build-Omnisystem-GUI.ps1 -Launch
```

### Build TUI Only

```powershell
cd Omnisystem/scripts/build
.\Build-Omnisystem-TUI.ps1
```

---

## Run the Executables

After building, executables are in: `Omnisystem/launchers/`

```powershell
# GUI launcher
.\Omnisystem\launchers\Omnisystem.exe

# TUI launcher
.\Omnisystem\launchers\Omnisystem_TUI.exe

# Or use the batch launcher
PowerShell -ExecutionPolicy Bypass -File "Omnisystem/scripts/build/Omnisystem-Launch.bat"
```

---

## Directory Structure

```
Omnisystem/
├── scripts/
│   └── build/                  ← ALL BUILD SCRIPTS HERE
│       ├── Build-All.ps1       (Master orchestrator)
│       ├── Build-Omnisystem-GUI.ps1
│       ├── Build-Omnisystem-TUI.ps1
│       ├── Omnisystem-Launch.bat
│       └── README.md           (Complete build documentation)
│
└── launchers/                  ← ALL EXECUTABLES HERE
    ├── Omnisystem.exe          (Built executable)
    ├── Omnisystem_TUI.exe      (Built executable)
    └── README.md               (Launcher documentation)
```

---

## Complete Documentation

For detailed build instructions, see:
- `Omnisystem/scripts/build/README.md` - Complete build system documentation
- `Omnisystem/launchers/README.md` - Executable documentation

---

**Status**: Production Ready  
**Version**: 29.0.0

