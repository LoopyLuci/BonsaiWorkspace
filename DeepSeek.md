# DeepSeek Handbook

## Current State & Progress

Merged baseline now includes the recent launcher/runtime hardening and feature rollouts:

- PR #20 merged on main for launcher and model readiness reliability.
- Vite launcher flow stabilized for Tauri startup path.
- Slot readiness race reduced with health-probe fallback and assistant retry behavior.
- Model Data system integrated for richer model metadata and model UX.
- Quick Options and task queue UI/status integration landed.
- BonsaiBot multi-platform integration path is available for Discord, Telegram, Email, and Matrix.

## Key Features & User Flows

### Model Data

- Model entries include enriched metadata such as capability strengths, context window, tier, and RAM estimates.
- User flow: open Model Selector, inspect metadata, load/select a model, confirm Active state.

### Quick Options

- Chat area includes quick-launch prompts for Weather, Time, Files, Sys Stats, and Web.
- User flow: pick option, review prefilled prompt, send, and inspect response/activity log.

### Task Queue

- Runtime queue tracks pending and active workloads with visible status on the bottom bar.
- User flow: submit one or many prompts, monitor queue counts, observe queue drain as work completes.

### BonsaiBot

- Messaging adapters support Discord, Telegram, Email, and Matrix.
- User flow: configure tokens/credentials, run bot service, validate health and platform status via admin API.

## Build, Test & Deployment

### Local Build

- Primary launcher:
  - `node bonsai-workspace/src/launch-all.mjs --mode desktop`

### Desktop Artifact Builder

- Use the dedicated builder scripts from repository root:
  - `BonsaiExeLauncherBuilder.ps1`
  - `BonsaiExeLauncherBuilder.cmd`

These scripts standardize frontend plus Tauri build orchestration and artifact output handling.

### Validation Checklist

- `cargo check` in relevant Rust crates.
- Targeted runtime/feature tests for new behavior.
- Launcher smoke validation for Vite + Tauri startup.

## 19. Development Cycle Closeout - 2026-05-04

### Merged PRs (This Cycle)
| PR | Title | Status |
|----|-------|--------|
| #19 | Warmup crash fix (--no-warmup) | ✅ Merged |
| #20 | Vite launcher crash + slot-ready race fix | ✅ Merged |
| #21 | Documentation update (README, manual, handbook) | ✅ Merged |
| #22 | Clojure/Python P1 hardening (4 commits) | ✅ Merged |
| #23 | GPU crash auto-fallback to CPU | ✅ Merged |
| #24 | Inference mode system + notification UX + terminal fix | ✅ Merged |

### Build Status
- `cargo check -p bonsai-workspace` - clean
- `npm run build` - clean

### Unstaged Artifacts
- latest.json, preflight-cache.json, BonsaiWorkspace.exe, .kotlin/
