# Bonsai Universal Surface Runtime (USR)

## Goal
Build a custom rendering/runtime stack so Bonsai has one UX across desktop and Android, including Fire OS devices where system WebView is incomplete.

## Problem Statement
Current Android app relies on system WebView. On some Fire OS builds, required framework classes (for example WebView stack dependencies) are missing, causing blank app rendering.

## Proposed System
Replace WebView dependency with a Bonsai-owned surface pipeline:

1. Shared UI Runtime Layer
- Keep one UI feature model and state protocol.
- Render source-of-truth state from a shared scene graph (not browser DOM-only assumptions).
- Preserve desktop and mobile UX parity through one layout engine + responsive constraints.

2. Render Backends
- Desktop backend: existing Tauri desktop shell (web frontend still supported during migration).
- Android backend A (preferred): native GPU surface renderer (Skia/WGPU style pipeline).
- Android backend B (fallback): remote surface stream from desktop host for ultra-weak clients.

3. Interaction Transport
- Bi-directional event channel:
  - Pointer/touch/keyboard events from client -> runtime.
  - Frame deltas + state patches from runtime -> client.
- Transport options:
  - Local process bridge (desktop).
  - WebSocket + binary frame channel (mobile/remote).

4. Asset and Font Consistency
- Bundle identical font packs, icon assets, spacing tokens, and theme tokens across platforms.
- Runtime verifies asset hash versions on startup to avoid UI drift.

## Runtime Modes
- mode = native_webview
  - Use system WebView only if compatibility checks pass.
- mode = native_surface
  - Use custom native renderer for full local experience without WebView.
- mode = remote_surface
  - Stream uniform UI surface from paired desktop for maximum compatibility.

## New Backend Contract (Implemented Now)
The backend command layer now supports richer mobile view orchestration:

- android_mobile_get_display_info
  - Pulls device display size, density, and current orientation telemetry.
- android_mobile_set_orientation
  - Locks portrait/landscape or unlocks rotation for stable UX.
- android_mobile_launch_bonsai
  - Launches Bonsai activity on target device.
- android_mobile_prepare_uniform_runtime
  - One-shot bootstrap: wake + unlock + reverse API/WS ports + launch app.

These commands are designed to feed the future USR mode selector and session bootstrap flow.

## Integration Plan
Phase 1: Compatibility Router (1-2 sprints)
- Add runtime mode probe on Android startup.
- If WebView unsupported, route to USR session selector instead of static unsupported screen.
- Expose chosen mode to desktop pairing/session APIs.

Phase 2: Remote Surface Parity (2-4 sprints)
- Add desktop-hosted compositor endpoint with frame delta protocol.
- Android fallback activity embeds native SurfaceView client.
- Route all input through existing remote input command pipeline.

Phase 3: Native Surface Renderer (4-8 sprints)
- Implement local Android renderer for core Bonsai panels.
- Reuse same UI state protocol as desktop.
- Keep remote_surface as compatibility fallback.

Phase 4: Convergence and Hardening
- Device capability matrix and automatic mode selection.
- Deterministic snapshot tests: desktop/native_surface/remote_surface must match UI state and layout invariants.
- Perf targets per tier (low-end Fire tablet to desktop workstation).

## Security and Reliability
- Pairing tokens required for remote_surface sessions.
- Signed capability handshake before input/frame channels open.
- Backpressure-aware streaming and heartbeat-based reconnection.
- Local artifact logging for screenshots, session traces, and fallback reason codes.

## Success Criteria
- Any supported Android/Fire device can open Bonsai without blank screen.
- Same settings, panes, and assistant behaviors across desktop and Android.
- Runtime mode transitions are transparent and recoverable.
- User-visible UX is functionally equivalent regardless of rendering backend.
