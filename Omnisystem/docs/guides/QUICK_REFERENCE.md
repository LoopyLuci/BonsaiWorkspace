# ⚡ Omnisystem Build - Quick Reference

## 🚀 Most Used Commands

### Build Everything (5 seconds)
```powershell
.\Quick-Build.ps1
```

### Build Desktop Only (2.5 seconds)
```powershell
.\Quick-Build.ps1 -Target desktop
```

### Release Build (Optimized)
```powershell
.\Quick-Build.ps1 -Release
```

### Verify Setup Before Building
```powershell
.\Verify-Build-Setup.ps1
```

---

## 📁 Output Location

```
Z:\Projects\Omnisystem\build\output\
  ├── Omnisystem.exe          ✅ Ready
  ├── OmnisystemGUI.exe       ⏳ Building
  └── BonsaiLauncher.exe      ⏳ Building
```

---

## 🎮 Run the Application

```powershell
.\build\output\Omnisystem.exe
```

## 📋 Build Options

| Option | Purpose | Example |
|--------|---------|---------|
| `-Target` | Which to build | `.\Quick-Build.ps1 -Target desktop` |
| `-Release` | Optimize | `.\Quick-Build.ps1 -Release` |

**Valid Targets:** `all`, `desktop`, `gui`, `launcher`

---

## 🔍 Check Build Log

```powershell
Get-Content .\build\build.log -Tail 20
```

---

## ⚙️ Advanced Usage

### Build with Advanced Options
```powershell
.\Build-Launchers.ps1 -Target all -Release -Clean
```

### Clean Old Artifacts
```powershell
.\Build-Launchers.ps1 -Target desktop -Clean
```

---

## ❓ Common Issues

### Rust Not Found?
Install from: https://rustup.rs/

### Build Failed?
Check: `Get-Content .\build\build.log`

### Permission Denied?
Run: `Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser`

---

## 📊 Current Status

| Component | Status | Binary |
|-----------|--------|--------|
| Desktop Environment | ✅ Working | Omnisystem.exe |
| GUI Launcher | ⏳ Building | OmnisystemGUI.exe |
| Ecosystem Launcher | ⏳ Building | BonsaiLauncher.exe |

---

## 📚 Full Docs

- **BUILD_GUIDE.md** - Complete documentation
- **BUILD_SYSTEM_SUMMARY.md** - System overview

---

**Last Updated:** June 16, 2026
