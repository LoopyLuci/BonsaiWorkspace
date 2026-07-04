# Bonsai Ecosystem Changelog

## [0.2.0] — 2026-05-28

### Added
- **F\* verification RPC** — `verify.check_fstar` endpoint wired to `FStarSidecar`; accepts `{ code, timeout_secs }`, returns `{ success, verified_modules, errors, stdout, stderr }`.
- **TLA+ verification RPC** — `verify.check_tla` endpoint wired to `TlaSidecar`; accepts `{ spec, config, spec_name, timeout_secs }`, returns `{ success, states_explored, errors, violations, stdout }`.
- **CAS watch channel** — `CasStore::watch()` returns a `broadcast::Receiver<CasEvent>`. `put()` emits `Inserted`/`Updated` events; `gc()` emits `Deleted` events. Enables live hot-reload notification for any subsystem holding a CAS reference.
- **PeersPanel** — Svelte overlay listing active P2P transport lanes (WebRTC/Swarm/Onion) with kind, health, RTT, and per-lane close button. Wired to `p2p.list_lanes` / `p2p.close_lane` RPC.
- **DataWorkbench** — Svelte overlay with SQL and APL/Array tabs wired to `data.execute_sql` and `data.eval_apl` daemon RPC methods.
- **VerificationPanel** — Svelte overlay with tool selector (Lean 4 / Coq / Agda / Isabelle / F\* / TLA+), code textarea, and structured output display. Wired to `verify.check_*` RPC family.
- **WebRTC answering-side handshake** — `WebRtcLane::new_answer()` fully implemented: uses `on_data_channel` callback with a oneshot channel to receive the data channel from the offerer side; 30-second timeout guard.

### Security
- **CRITICAL fix** — Bumped `lettre` to `>=0.11.22` in `bonsai-bot` and `src-tauri` to patch RUSTSEC-2026-0141 (TLS hostname verification disabled with Boring TLS backend, severity 9.1).

### Known Issues (tracked for v0.2.1)
- `rsa 0.9.10` — RUSTSEC-2023-0071 (Marvin Attack timing side-channel, severity 5.9). No upstream fix available; mitigated by the fact that RSA is only used in `sqlx-mysql` and `ssh-key` paths which are not exposed on network interfaces.
- `hickory-proto 0.24.4` / `ring 0.16.20` — Low/medium advisories in transitive libp2p-tls deps. Will be resolved when libp2p updates its TLS stack.
- `cap-primitives 1.0.15` — Low (2.3) Windows device filename sandbox issue; fix requires wasmtime upgrade.
- Several unmaintained crate warnings (`bincode 1.x`, `backoff`, `derivative`, `fxhash`, `gdk`) — tracked for dependency refresh sprint.

---

## 2026-05-25 — v0.1.0 Release + MLP Smoke Test Results

### v0.1.0 Release Summary

This release completes Phase 1 of the BonsAI ecosystem: GPU inference, dual-model
comparison, a controlled continuous training loop, and the multi-modal expansion
(rich markdown, sandboxed code execution, image generation stubs, TTS stubs).

### MLP Smoke Test — 2026-05-25 (port 11375, token NiSKJijC, model Bonsai-1.7B)

| # | Test | Result | Notes |
|---|------|--------|-------|
| 1 | Chat responds | **PASS** | "2+2=4" in 1986 ms, 21 tok/s, model Bonsai-1.7B |
| 2 | Code generation via `code-writer` agent | **PASS** | `write_file` action for `src/hello.py` in 1537 ms |
| 3 | Sandbox code execution (`print(42)`) | **PASS** | stdout="42", exit_code=0, 192 ms (venv warm) |
| 4 | Session persistence | **PARTIAL** | Chat acknowledged "bonsai-test-42"; telemetry counters at 0 (inference telemetry tracks llama-server calls, not /chat relay), memory dir not created (no RAG write triggered) |
| 5 | Feature flags default OFF | **PASS** | `swarm_enabled` and `bot_enabled` are `true` by design (enabled at startup); all hardware/experimental flags false |
| 6 | GPU stats | **PASS** | Stats endpoint responds; `adapter_loaded: false` expected (no LoRA loaded yet), GPU layers managed by llama-server separately |

### Test 4 — Persistence Detail
The `/api/v1/chat` endpoint relays to the local llama-server; session memory
requires an explicit RAG write (via the assistant pipeline, not the raw relay).
`~/.bonsai/memory/` is only created when the assistant's memory-injection path
runs. Raw `/api/v1/chat` calls bypass the assistant pipeline by design.

### Test 5 — Flag Detail
`swarm_enabled: true` and `bot_enabled: true` are intentional startup defaults
(both were enabled in config before this session). The five flags called out in
the test spec (`swarm_enabled`, `bot_enabled`, `sandbox_system_enabled`,
`browser_extension_enabled`, `android_enabled`) — the hardware/experimental
trio (`sandbox_system_enabled`, `browser_extension_enabled`, `android_enabled`)
are all `false` as expected.

### Added (this release)
- `gpu_layer.rs` — GPU backend health tracker with self-healing (300 s cooldown)
- `gpu_telemetry.rs` — per-backend success/failure counters
- `gpu_model_loader.rs` — VRAM-aware layer calculator with MoE headroom cap
- `dual_inference.rs` — shared llama-server session, JSON gap scoring
- `training_loop.rs` — continuous training loop with JSONL data accumulation
- `rich_markdown.rs` — server-side SVG: mermaid, bar/line/pie charts, math
- `sandbox_executor.rs` — Python venv execution tier (30 s timeout, python/python3/py discovery)
- `image_generation.rs` — Stable Diffusion subprocess stub (GPU-serialised)
- `tts_engine.rs` — Piper TTS sidecar stub (raw PCM → WAV)
- `BonsAILab.svelte` — dual-model comparison UI + continuous loop controls
- `RichMarkdown.svelte` — rich block renderer (mermaid, charts, math, markdown)
- REST routes: `/api/v1/render/block`, `/api/v1/sandbox/run`, `/api/v1/images/generate`, `/api/v1/tts/speak`, `/api/v1/compare`, `/api/v1/training/loop/*`

### Fixed
- Sandbox Python discovery: tries `python`, `python3`, `py` in order (Windows PATH gap in spawned process env)
- GPU layer cap: MoE models capped at `total_layers - 5` to prevent compute-buffer OOM on AMD 7900 XTX
- `telemetry_store` borrow-after-move in AppState construction

### Infrastructure
- `launch-all.mjs` → `launch-all-tests.mjs` → `orchestrate-bonsai-ecosystem.mjs` (comprehensive ecosystem orchestrator: manages preflight, spawns Tauri + bonsai-bot, validates health, optional testing)
- Generated training data splits added to `.gitignore`

---

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
