# Bonsai Ecosystem Changelog

## 2026-05-04 - Inference Mode System & Stability Fixes

### Added
- GPU/CPU inference mode toggle (Auto, CPU Only, GPU Only, Hybrid)
- Inference mode chip selector in ChatPanel
- Inference Defaults settings with Apply to All
- Auto-dismiss model loaded notification (5 seconds)
- BonsaiExeLauncherBuilder.ps1 + .cmd for building .exe

### Fixed
- Flashing terminal window on Windows (CREATE_NO_WINDOW on all spawns)
- GPU crash auto-recovery with CPU fallback (0xc0000409, 0xc0000005)
- Vite launcher crash (4294967295 exit code)
- Slot-ready race condition (transient "No model slot is ready")
- Bonsai Buddy no longer pinned by default
- llama-server warmup crash (--no-warmup flag)

### Changed
- Quick Options moved to dropdown menu
- Queue indicator moved to bottom green status bar
- Model loading shows real-time progress bar
- Last-used model auto-loaded on next startup

### Security
- Python worker resource limits (30s CPU, 512MB RAM)
- Babashka filesystem path jail
- Babashka version pinning (1.3.191 in CI)
- Python binary preference (python3 over python on Unix)

### Documentation
- README updated with What's New, Quick Start, Building from Source
- User manual expanded with Model Selector, Quick Options, Task Queue
- DeepSeek.md handbook created as single source of truth
