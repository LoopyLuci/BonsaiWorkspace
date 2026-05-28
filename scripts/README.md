# Scripts

| Directory | Purpose |
|-----------|---------|
| `build/`   | Build scripts — Tauri app, watchdog binary |
| `launch/`  | Launch scripts — start Bonsai Workspace locally |
| `dev/`     | Developer utilities — desktop shortcut, autopilot, babashka tools |
| `archive/` | Deprecated one-off scripts kept for reference |

Root-level `Launch-BonsaiWorkspace.ps1` and `BonsaiExeLauncherBuilder.ps1` are thin shims that delegate to the real scripts in `scripts/launch/` and `scripts/build/` respectively.
