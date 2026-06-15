# Launchers

Quick-launch scripts for running BonsaiWorkspace applications.

## Location

`scripts/launchers/`

## Quick Launch Commands

### Launch Bonsai Workspace IDE

**Script**: `Launch-BonsaiWorkspace.ps1`  
**Platform**: Windows (PowerShell)  
**Purpose**: Start the Bonsai Workspace IDE  
**Usage**:
```powershell
.\scripts\launchers\Launch-BonsaiWorkspace.ps1
```

**What it does**:
- Detects Tauri app location
- Starts the IDE
- Sets up environment
- Launches on default port (1420)

### Start UACS (Universal Agent Control System)

**Script**: `START_UACS.ps1` (Windows) or `START_UACS.sh` (Linux/macOS)  
**Purpose**: Launch the Universal Agent Control System  
**Usage**:

**Windows**:
```powershell
.\scripts\launchers\START_UACS.ps1
```

**Linux/macOS**:
```bash
bash scripts/launchers/START_UACS.sh
```

**What it does**:
- Initializes UACS system
- Sets up agent control environment
- Starts agent management interface
- Enables human-in-the-loop control

## All Launchers

| Script | Platform | Purpose | Command |
|--------|----------|---------|---------|
| Launch-BonsaiWorkspace.ps1 | Windows | IDE | `.\scripts\launchers\Launch-BonsaiWorkspace.ps1` |
| START_UACS.ps1 | Windows | UACS system | `.\scripts\launchers\START_UACS.ps1` |
| START_UACS.sh | Linux/macOS | UACS system | `bash scripts/launchers/START_UACS.sh` |

## Quick Access

### One-Click Startup

Add shortcuts to these commands on your desktop or taskbar for quick access:

**Windows Desktop Shortcut**:
1. Right-click desktop → New → Shortcut
2. Location: `powershell -NoProfile -ExecutionPolicy Bypass -File "Z:\Projects\BonsaiWorkspace\scripts\launchers\Launch-BonsaiWorkspace.ps1"`
3. Name: "Bonsai Workspace"

**Windows Taskbar**:
1. Pin PowerShell to taskbar
2. Right-click → Properties
3. Target: Same as above
4. Start in: `Z:\Projects\BonsaiWorkspace`

## Application Ports

| Application | Port | URL |
|------------|------|-----|
| Bonsai Workspace IDE | 1420 | `http://localhost:1420` |
| UACS System | 3000 | `http://localhost:3000` |

## Pre-Launch Checklist

Before launching:
1. ✓ System has been built (`build-complete-system.ps1`)
2. ✓ Dependencies are installed
3. ✓ Python environment configured
4. ✓ No build errors in recent build

## Troubleshooting Launches

### IDE Won't Start
1. Check if build succeeded
2. Verify port 1420 is not in use
3. Check `logs/` for error output
4. Try building again with `build-and-run.ps1`

### UACS Won't Start
1. Verify UACS system files exist
2. Check Python environment
3. Review startup logs
4. Ensure required dependencies are installed

### Port Already in Use
```powershell
# Find process on port
Get-NetTCPConnection -LocalPort 1420

# Kill process if needed
Stop-Process -Id [PID] -Force
```

## Development & Customization

### Creating New Launchers

To add a new launcher script:

1. Create script in `scripts/launchers/`
2. Add to this README
3. Follow naming convention: `[APP]-[ACTION].ps1`
4. Examples:
   - `Launch-BonsaiWorkspace.ps1`
   - `Launch-BMF-Server.ps1`
   - `Start-KDB-Indexer.ps1`

### Script Template

```powershell
<#
.SYNOPSIS
    Brief description of what this launcher does
.DESCRIPTION
    Detailed explanation
.EXAMPLE
    .\scripts\launchers\[ScriptName].ps1
#>

param(
    [switch]$Verbose
)

# Your launch logic here
Write-Host "Launching application..."

# Setup environment
# Start application
# Wait/monitor
```

## Management

### Updating Launcher Scripts

1. Edit the script in `scripts/launchers/`
2. Test thoroughly
3. Commit changes
4. Update this README if behavior changes

### Adding New Applications

When adding new launchable components:
1. Create launcher script in `scripts/launchers/`
2. Document in this README
3. Add to "All Launchers" table
4. Update port mapping section
5. Add troubleshooting tips if needed

## Advanced Usage

### Launching with Parameters

```powershell
# Launch with debug output
.\scripts\launchers\Launch-BonsaiWorkspace.ps1 -Verbose

# Launch and wait for completion
$process = Start-Process -FilePath powershell -ArgumentList @(
    "-NoProfile",
    "-ExecutionPolicy Bypass",
    "-File",
    "Z:\Projects\BonsaiWorkspace\scripts\launchers\Launch-BonsaiWorkspace.ps1"
) -PassThru

$process.WaitForExit()
```

### Scheduled Launches

Create Windows Task Scheduler job:
```powershell
$action = New-ScheduledTaskAction -Execute "powershell.exe" `
    -Argument "-NoProfile -ExecutionPolicy Bypass -File Z:\Projects\BonsaiWorkspace\scripts\launchers\Launch-BonsaiWorkspace.ps1"

$trigger = New-ScheduledTaskTrigger -AtStartup

Register-ScheduledTask -Action $action -Trigger $trigger -TaskName "Launch Bonsai Workspace"
```

## Monitoring Launched Applications

### Check Running Processes
```powershell
Get-Process | Where-Object {$_.Name -match "tauri|node|python"}
```

### View Application Logs
```powershell
Get-Content logs/build-launch.log -Tail 50
```

### Stop Applications
```powershell
# Stop by name
Stop-Process -Name "bonsai-workspace" -Force

# Stop by port
$pid = (Get-NetTCPConnection -LocalPort 1420).OwningProcess
Stop-Process -Id $pid -Force
```

## Performance Tips

1. **Pre-warm**: Run `setup-compilation-cache.ps1` once for faster restarts
2. **Dedicated terminal**: Keep launcher terminal open for debugging
3. **Monitor resources**: Watch CPU/memory during startup
4. **Check logs**: Always check `logs/` if something seems slow

## Integration with Build Scripts

**Recommended Workflow**:
1. Build system: `.\scripts\build-scripts\build-complete-system.ps1`
2. Launch IDE: `.\scripts\launchers\Launch-BonsaiWorkspace.ps1`

**Or in one command**:
```powershell
.\scripts\build-scripts\build-and-run.ps1
```

## Directory Structure

```
scripts/
├── build-scripts/          # Compilation and setup
│   ├── build-complete-system.ps1
│   ├── windows-full-setup.ps1
│   └── [13+ build scripts]
│
├── launchers/              # Quick application launch
│   ├── Launch-BonsaiWorkspace.ps1
│   ├── START_UACS.ps1
│   ├── START_UACS.sh
│   └── LAUNCHERS_README.md (this file)
│
├── shell/                  # Legacy shell scripts
│   └── [shell scripts]
│
├── BUILD_SCRIPTS_README.md
└── LAUNCHERS_README.md
```

## Quick Reference Card

```
┌─────────────────────────────────────────────┐
│      BONSAI WORKSPACE QUICK LAUNCH          │
├─────────────────────────────────────────────┤
│ IDE Launch:                                 │
│ .\scripts\launchers\Launch-BonsaiWorkspace  │
│                                             │
│ UACS System:                                │
│ .\scripts\launchers\START_UACS.ps1          │
│                                             │
│ Full Build + Launch:                        │
│ .\scripts\build-scripts\build-and-run.ps1   │
└─────────────────────────────────────────────┘
```

---

**Last Updated**: June 3, 2026  
**Status**: ✅ Complete and Organized  
**Total Launchers**: 3 quick-start scripts
