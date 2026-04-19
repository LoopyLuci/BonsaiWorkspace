# Bonsai Assistant Audit

Date: 2026-04-17
Scope: Deep architecture and production-readiness audit of Bonsai_Assistant.md against the original prompt requirements.
Artifact audited: Bonsai_Assistant.md

## 1. Executive Summary

This plan is strong in vision and technical direction, but it is not yet production-complete as written. It is currently best described as an advanced implementation blueprint that still needs operational hardening, security boundaries, governance controls, and release engineering detail before it can be called next-gen and production-grade.

Overall assessment:
- Product ambition: Excellent
- Core architecture fit for Bonsai Workspace: Strong
- Security and safety maturity: Moderate (needs major hardening)
- Reliability and lifecycle maturity: Moderate
- Android parity and robustness: Moderate
- Backup and management completeness: Moderate to strong
- Testing and release readiness: Moderate (insufficient detail for production confidence)

Production readiness score (plan quality): 7.4/10

Key conclusion:
The plan can become production-grade with focused work in six areas:
1. Security model and permission boundaries for local tools and automation
2. Fault tolerance, process lifecycle, and recovery mechanics
3. Data governance, encryption, and backup integrity
4. Observability and diagnostics (metrics, logs, traces, health)
5. Android-specific behavior parity and fallback quality
6. CI/CD, migration strategy, and release gating

## 2. Audit Method

This audit evaluates the plan against:
- The original user prompt goals and constraints
- Production-grade standards for desktop/mobile local assistants
- Tauri application security and lifecycle best practices
- Data and backup resilience standards
- Operational readiness standards (testing, rollout, rollback)

This is a plan-level audit, not a code-level implementation verification.

## 3. Requirement Traceability to Original Prompt

### 3.1 Standalone floating GUI, works when main workspace is closed
Status: Mostly satisfied
- Positive: Second Tauri window + main close interception + tray integration is a correct direction.
- Gap: No explicit crash restart and state restoration policy if assistant window crashes while main hidden.
- Gap: No explicit startup policy matrix (boot behavior on app launch, user opt-in/out, last-state restore).

### 3.2 Can do broad local tasks (weather, files, charts, email, web scrape, shell/system)
Status: Partially satisfied
- Positive: Tool registry concept is comprehensive and aligns with task goals.
- Gap: Tool execution sandbox, policy enforcement, path restrictions, and command allowlists are underspecified.
- Gap: Web scraping legal/robots/TOS handling absent.
- Gap: Email security posture (credential handling, audit logs, accidental exfil prevention) needs stronger controls.

### 3.3 Animated avatar with TTS lip-sync
Status: Strongly satisfied (desktop), partial (Android)
- Positive: Piper + viseme timeline + SVG-based rendering is a strong, efficient architecture.
- Positive: 14-viseme mapping is thoughtfully defined.
- Gap: Android fallback via character boundary modulo mapping is low-fidelity and may degrade perceived quality.
- Gap: No jitter compensation/drift correction spec for timing mismatch between playback and RAF loop.

### 3.4 Robust management system for profiles/personas/avatars/backup
Status: Mostly satisfied
- Positive: Data model covers profiles, assets, sessions, backups.
- Positive: Backup export/import and rotation are planned.
- Gap: No backup encryption, signing, or corruption recovery workflow.
- Gap: No schema migration rollback strategy or integrity verification command.

### 3.5 Desktop and Android clean, robust GUI with fast access
Status: Partially satisfied
- Positive: Desktop layout is clear and practical.
- Positive: Android tab inclusion is planned.
- Gap: No explicit mobile performance budgets, accessibility detail, or reduced-motion mode.
- Gap: No explicit touch-first interaction quality criteria for avatar/profile management flows.

### 3.6 Next-gen and production-grade quality bar
Status: Not yet fully satisfied
- Positive: The plan is comprehensive and modular.
- Gap: Missing production essentials (security gates, observability, SLOs, chaos/failure tests, staged rollout).

## 4. Strengths (What Is Excellent)

1. Correct architectural choice for shared resources
- Using a second webview in the same Tauri process avoids duplicate model loads and DB pools.

2. Clear modular backend decomposition
- assistant_store, assistant_tools, assistant_manager, tts_manager, assistant_backup, assistant_commands is a clean split.

3. Thoughtful TTS and avatar technical pairing
- Piper sidecar with phoneme timing and SVG viseme rendering is lightweight and portable.

4. Good data model baseline
- Profile/session/message/avatar/backup tables cover primary persistence requirements.

5. Incremental phased delivery
- Four phases with compile/build validation checkpoints is a good implementation strategy.

6. Integration awareness
- Explicit modifications to existing files and shared AppState integration show real platform alignment.

## 5. Critical Gaps and Risks (Priority Findings)

Severity key:
- Critical: Must fix before production
- High: Major quality/security/reliability risk
- Medium: Important but can be staged

### 5.1 Critical: Tool security boundary is underdefined
Risk:
- The assistant can potentially execute high-impact local actions without robust policy controls, creating privilege and abuse risks.

Missing controls:
- Per-tool allowlist/denylist with explicit argument schemas
- Path sandbox for filesystem actions (workspace-only optional mode plus explicitly approved external paths)
- Command execution policy (no unrestricted shell by default)
- Network egress policy (domain allowlist for fetch/scrape/email)
- Sensitive-data redaction before tool outputs are shown/saved

Required remediation:
- Introduce a central policy engine:
	- tool_permissions evaluated server-side before every tool call
	- strict JSON schema validation on tool arguments
	- immutable audit record for all tool attempts (allowed/denied)
- Add explicit consent flow for high-risk actions (send email, shell exec, external writes).

### 5.2 Critical: No robust secret management for SMTP and external credentials
Risk:
- Storing credentials insecurely can compromise user accounts and trust.

Required remediation:
- Use OS credential vaults:
	- Windows Credential Manager
	- Android Keystore-backed secure storage
- Never store raw SMTP passwords in SQLite or backup archives.
- Support app-password guidance and OAuth where feasible.

### 5.3 Critical: Backup integrity and confidentiality not production-safe
Risk:
- Plain ZIP backups with profile/session history and potential tool outputs can leak sensitive data.

Required remediation:
- Add optional encryption (AES-256) for backup files.
- Add integrity manifest with SHA-256 hashes of all included files.
- Add signed manifest format versioning.
- Add restore dry-run and validation mode before import commit.

### 5.4 High: Lifecycle resilience is incomplete
Risk:
- Assistant survivability claims may fail on crashes, updates, or orphaned process states.

Missing:
- Explicit crash-restart behavior
- Sidecar process watchdog and stuck-process cleanup
- Session replay behavior after forced termination

Required remediation:
- Add lifecycle state machine for assistant window and sidecars.
- Add startup recovery sequence:
	1. detect unclean shutdown
	2. restore pending session context
	3. validate DB and sidecar health
	4. resume idle-ready state

### 5.5 High: Observability and diagnostics are underspecified
Risk:
- Hard to debug field issues (audio sync, tool failures, Android quirks) without metrics and structured logs.

Required remediation:
- Add structured event telemetry (local only unless user opts in):
	- assistant_turn_latency_ms
	- tool_call_count, tool_call_error_rate
	- tts_synthesis_ms, tts_playback_ms, viseme_drift_ms
	- window_restore_time_ms
	- backup_export/import_success_rate
- Add rotating local logs with PII redaction.

### 5.6 High: Android parity quality is fragile for lip-sync and voice
Risk:
- Web Speech fallback with char-boundary viseme approximation may feel low-end versus desktop.

Required remediation:
- Prefer Piper on Android where packaging allows and resource budget permits.
- If fallback remains, add improved phoneme approximation pipeline based on tokenized text and language heuristics, not char modulo.
- Add quality mode toggle: Battery Saver vs High Fidelity.

### 5.7 Medium: Data migration and rollback strategy missing
Risk:
- Schema changes over versions can break backups/restores and user state.

Required remediation:
- Add explicit migration table and forward/backward compatibility policy.
- Add preflight migration checks and backup-before-migrate guard.

### 5.8 Medium: Multi-profile active state invariants need enforcement
Risk:
- assistant_profiles.is_active can drift into invalid states without DB constraints.

Required remediation:
- Enforce single-active-profile invariant with transaction + unique partial index.
- Add startup correction routine if data is inconsistent.

## 6. Security and Privacy Hardening Checklist

Must-have controls before production:
- Tool permission model enforced server-side
- High-risk action confirmations
- File operation sandbox defaults
- Secret vault integration for credentials and tokens
- Backup encryption and integrity hashes
- PII-aware logging with redaction
- CSP hardened for assistant webview
- Strict input validation for all Tauri commands
- Domain allowlist and HTTP timeout/retry policies for web tools
- Safe HTML rendering for scraped content and tool results

Recommended additions:
- Prompt-injection guardrails for tool-using turns
- Tool output classifiers (sensitive content warnings)
- Optional "offline strict mode" that disables network tools entirely

## 7. Reliability, Performance, and UX Readiness

### 7.1 Reliability SLO candidates
- Assistant window open-to-ready: p95 < 1.2s on desktop
- Send message to first token: p95 < 2.0s (local model dependent)
- TTS completion failure rate: < 1%
- Backup export failure rate: < 0.5%

### 7.2 Performance budgets
- Assistant renderer JS bundle target: < 700KB compressed
- Idle CPU usage target: < 2% desktop, < 4% Android
- Idle memory target: < 300MB incremental over baseline
- Avatar RAF loop should pause when hidden/minimized

### 7.3 UX quality requirements
- Full keyboard navigability in desktop assistant panel
- Touch targets >= 44px on Android
- Reduced motion and mute-by-default options
- Explicit command result provenance in chat bubbles
- Tool progress indicators with cancel support

## 8. Integration Quality with Bonsai Workspace

Strengths:
- Shared AppState and SQLite integration is correct.
- Dedicated assistant event channels reduce coupling.

Needed improvements:
- Define strict event namespace conventions and versioning.
- Add conflict policy between main workspace and assistant for shared model allocation.
- Add model scheduling fairness rules when both UIs issue requests.
- Add user-visible "resource arbitration" settings (assistant priority vs workspace priority).

## 9. Avatar System Audit

Positive:
- SVG strategy is efficient and maintainable.
- Viseme mapping is practical.

Gaps to close:
- No avatar content safety/spec validation for imported SVG.
- No schema for avatar rig completeness validation.
- No explicit fallback when missing viseme path(s).

Required controls:
- SVG sanitizer pipeline (strip scripts, external refs, unsupported tags).
- Avatar validator command:
	- checks required viseme ids 0..13
	- checks bounding box and path validity
	- generates diagnostics report

## 10. Backup and Management System Audit

What is good:
- Includes profiles, avatars, sessions, messages.
- Has registry table and rotation concept.

What is missing for production:
- Encryption at rest for backups
- Backup verification command
- Conflict resolution strategy on import (merge/replace/per-profile)
- Import transaction safety with rollback

Recommended import modes:
- Merge by IDs with conflict rename
- Replace selected profile only
- Full replace (with automatic snapshot before apply)

## 11. Testing and Release Gate Audit

Current plan includes cargo check and npm build, which is necessary but not sufficient.

Add required test suites:
- Unit tests:
	- policy engine
	- tool arg validation
	- profile/active invariant
	- viseme timeline mapping
- Integration tests:
	- assistant survives main close
	- sidecar restart after failure
	- backup export/import roundtrip
	- multi-session persistence
- E2E desktop tests:
	- weather, files, chart, email dry-run, web scrape summary
	- avatar swap mid-response
	- TTS stop/cancel behavior
- Android E2E tests:
	- buddy tab interaction latency
	- voice fallback path
	- lifecycle under orientation/app backgrounding

Release gates:
- Security gate: all critical findings closed
- Reliability gate: SLO smoke tests pass
- Migration gate: backup+restore compatibility verified
- Performance gate: budgets met on reference devices

## 12. Recommended Production-Grade Additions to Plan

1. Add a dedicated "Phase 0: Security and Policy Foundation"
- Implement tool policy engine first
- Implement secrets vault integration
- Implement audit logs and redaction

2. Add "Phase 2.5: Reliability and Observability"
- Sidecar watchdog, crash recovery, health endpoints
- Structured local metrics and diagnostic viewer

3. Add "Phase 4.5: Hardening and Launch Readiness"
- Load/perf tests
- Backup encryption + verification
- Migration and rollback drills

## 13. Definition of Done (Production)

The Bonsai Assistant is production-ready only when all are true:
- 100% of core prompt requirements are implemented and validated in E2E tests.
- No Critical/High security findings remain open.
- Backup restore succeeds from N-2 versions with integrity verification.
- Assistant survives main window close, app restart, and sidecar crash scenarios.
- Android and desktop both pass parity test suite for core interactions.
- All high-risk actions require explicit user confirmation unless permanently approved.
- Logs are structured, redacted, and diagnosable locally.
- Performance and reliability SLOs pass on reference hardware.

## 14. Final Verdict

The plan is very strong and clearly on the right trajectory, but it is not yet "perfect" or fully production-grade in its current form. It needs explicit hardening layers around security, secrets, backup integrity, reliability recovery, and observability.

If the remediation items in this audit are incorporated as first-class work (not deferred), Bonsai Assistant can credibly meet the next-gen, production-grade, and deeply integrated quality bar requested in the original prompt.

## 15. Immediate Action List (Top 12)

1. Implement server-side tool policy engine with strict arg schemas.
2. Introduce high-risk action approval flow and persisted consent model.
3. Add secrets vault integration for SMTP credentials.
4. Add encrypted backups with manifest hashing.
5. Add sidecar watchdog and assistant recovery state machine.
6. Add structured metrics and redacted logs.
7. Add SVG import sanitizer and avatar rig validator.
8. Enforce single active profile invariant transactionally.
9. Add migration preflight and rollback workflow.
10. Add full E2E desktop and Android test suites.
11. Add model arbitration policy between assistant and workspace.
12. Add release gates tied to security, SLO, migration, and perf thresholds.

## 16. Implementation-Ready Execution Checklist

This section converts the audit into concrete engineering work with explicit files, task outputs, and acceptance tests.

### 16.1 Phase 0: Security and Policy Foundation (Must Complete Before Broad Tooling)

Target outcome:
- No unrestricted high-risk local action path exists.

Files to create:
- src-tauri/src/assistant_policy.rs
- src-tauri/src/assistant_audit_log.rs
- src-tauri/src/secrets_store.rs

Files to update:
- src-tauri/src/assistant_tools.rs
- src-tauri/src/assistant_manager.rs
- src-tauri/src/assistant_commands.rs
- src-tauri/src/lib.rs
- src-tauri/Cargo.toml

Tasks:
1. Implement server-side tool policy evaluator in src-tauri/src/assistant_policy.rs.
2. Add strict per-tool argument schema checks in src-tauri/src/assistant_tools.rs.
3. Enforce allow/deny decisions before each tool call in src-tauri/src/assistant_manager.rs.
4. Add explicit high-risk confirmation command flow in src-tauri/src/assistant_commands.rs.
5. Add immutable local audit log writer in src-tauri/src/assistant_audit_log.rs.
6. Add secure secret vault abstraction in src-tauri/src/secrets_store.rs.
7. Wire policy and secret services into AppState in src-tauri/src/lib.rs.

Acceptance tests:
1. Tool call with invalid args is rejected with deterministic error and logged.
2. Tool call outside policy is rejected and logged as denied.
3. High-risk action requires explicit approval token before execution.
4. SMTP credentials are never persisted in SQLite or plaintext files.
5. Unit tests cover allow/deny matrix and schema validation edge cases.

### 16.2 Phase 1: Assistant Runtime Foundation (Window, State, Lifecycle)

Target outcome:
- Assistant remains usable when main window is closed and restores reliably after restart.

Files to create:
- bonsai-workspace/src/assistant.html
- bonsai-workspace/src/assistant-main.ts
- bonsai-workspace/src/AssistantApp.svelte
- bonsai-workspace/src/lib/stores/assistant.ts
- bonsai-workspace/src/lib/components/assistant/BonsaiAssistant.svelte
- bonsai-workspace/src/lib/components/assistant/AssistantMessageList.svelte
- bonsai-workspace/src/lib/components/assistant/AssistantMessage.svelte
- bonsai-workspace/src/lib/components/assistant/AssistantInputBar.svelte

Files to update:
- bonsai-workspace/src/vite.config.ts
- bonsai-workspace/src/App.svelte
- bonsai-workspace/src-tauri/tauri.conf.json
- bonsai-workspace/src-tauri/src/lib.rs
- bonsai-workspace/src-tauri/src/assistant_store.rs
- bonsai-workspace/src-tauri/src/assistant_commands.rs

Tasks:
1. Add assistant webview window config and tray wiring.
2. Implement close-intercept behavior for main window while assistant exists.
3. Add assistant MPA entry points in vite config.
4. Implement baseline assistant message/session flows.
5. Add lifecycle recovery hook for unclean shutdown state.

Acceptance tests:
1. Closing main window with assistant open hides main and keeps assistant active.
2. Tray toggle shows/hides assistant window correctly.
3. Assistant conversation persists after app restart.
4. Build output includes both main and assistant entry bundles.

### 16.3 Phase 2: Capability Engine and Safe Local Actions

Target outcome:
- Assistant can perform weather/files/chart/web/system/email with policy enforcement.

Files to update:
- src-tauri/src/assistant_tools.rs
- src-tauri/src/assistant_manager.rs
- src-tauri/src/assistant_commands.rs
- bonsai-workspace/src/lib/components/assistant/QuickActionChips.svelte
- bonsai-workspace/src/lib/components/assistant/InlineToolResult.svelte
- bonsai-workspace/src/lib/components/assistant/AssistantSettings.svelte

Tasks:
1. Implement tool adapters with timeout, retry, and deterministic error mapping.
2. Add cancellable ReAct loop and tool-step streaming.
3. Add user-facing tool progress and failure transparency in UI.
4. Add domain allowlist and path sandbox controls in settings.

Acceptance tests:
1. Weather flow returns structured card under nominal conditions.
2. File search cannot escape configured policy boundaries.
3. Chart rendering returns deterministic SVG for fixed input.
4. Email send path requires approved credentials and emits auditable events.
5. Cancelling a running turn stops pending tool execution safely.

### 16.4 Phase 3: TTS, Lip-Sync, Avatar Runtime Quality

Target outcome:
- Smooth desktop lip-sync with robust fallback behavior and import-safe avatars.

Files to create:
- src-tauri/src/avatar_validator.rs

Files to update:
- src-tauri/src/tts_manager.rs
- src-tauri/src/bootstrap.rs
- src-tauri/src/assistant_commands.rs
- src-tauri/src/lib.rs
- src-tauri/Cargo.toml
- bonsai-workspace/src/lib/components/assistant/AssistantAvatar.svelte
- bonsai-workspace/src/lib/components/assistant/AvatarPicker.svelte
- bonsai-workspace/src/lib/stores/tts.ts

Tasks:
1. Add Piper bootstrap and health checks.
2. Add viseme event drift correction (audio clock vs RAF clock).
3. Add avatar import sanitization and 14-viseme rig validation.
4. Add runtime fallback behavior for missing viseme paths.
5. Add Android quality-mode switch (high-fidelity vs battery saver).

Acceptance tests:
1. TTS synthesis emits viseme timeline for supported voices.
2. Avatar mouth updates remain synchronized within acceptable drift budget.
3. Invalid SVG avatar import is rejected with actionable error.
4. Missing viseme paths degrade gracefully to silence/default shape.
5. Android fallback path remains functional under no-Piper condition.

### 16.5 Phase 4: Backup, Restore, Management, and Integrity

Target outcome:
- User can safely export/import assistants with verification, rollback, and conflict handling.

Files to update:
- src-tauri/src/assistant_backup.rs
- src-tauri/src/assistant_store.rs
- src-tauri/src/assistant_commands.rs
- bonsai-workspace/src/lib/components/assistant/BackupManager.svelte
- bonsai-workspace/src/lib/components/assistant/ProfileManager.svelte
- bonsai-workspace/src/lib/components/assistant/AssistantSessionHistory.svelte

Tasks:
1. Add encrypted backup option and manifest checksums.
2. Add import modes: merge, replace-profile, full-replace.
3. Add pre-import dry-run validation and rollback-on-failure.
4. Add auto-backup rotation and restore point creation before destructive import.

Acceptance tests:
1. Export generates valid manifest and checksums.
2. Corrupt backup fails import with no state mutation.
3. Full replace creates restore point and can rollback on failure.
4. Merge import resolves conflicts without orphaned references.

### 16.6 Phase 5: Observability, SLOs, and Release Hardening

Target outcome:
- Assistant behavior is diagnosable and releasable under production gates.

Files to create:
- src-tauri/src/assistant_metrics.rs
- bonsai-workspace/src/lib/components/assistant/AssistantDiagnostics.svelte

Files to update:
- src-tauri/src/lib.rs
- src-tauri/src/assistant_manager.rs
- src-tauri/src/assistant_tools.rs
- src-tauri/src/tts_manager.rs
- bonsai-workspace/src/lib/stores/assistant.ts

Tasks:
1. Add structured local metrics and redacted log events.
2. Add diagnostics panel for last errors, tool failures, and sidecar status.
3. Add SLO assertions in CI for latency, reliability, and restart behavior.
4. Add migration compatibility tests for backup and schema versions.

Acceptance tests:
1. Metrics populate for message turns, tools, and TTS runs.
2. Diagnostics view shows actionable recent failures.
3. CI gate fails when SLO thresholds regress.
4. N-2 version backup restore compatibility passes.

### 16.7 Cross-Cutting QA Matrix (Required Before Production Signoff)

Security tests:
1. Prompt-injection simulations cannot bypass tool policy.
2. Secrets never appear in logs, backups, or message history.

Reliability tests:
1. Sidecar crash triggers graceful recovery path.
2. Assistant survives main window close and app restart scenarios.

Performance tests:
1. Idle CPU and memory stay under target thresholds.
2. Assistant open-to-ready and first-token latency meet p95 targets.

Android tests:
1. Buddy tab remains responsive after orientation/background cycles.
2. Voice/avatar fallback behavior is stable under constrained resources.

## 17. Delivery Governance and Release Control

Branching and rollout:
1. Implement each phase behind feature flags where applicable.
2. Use staged rollout in internal channels before general release.

Required release artifacts:
1. Security checklist signoff
2. SLO benchmark report
3. Backup restore compatibility report
4. Android parity report
5. Known limitations and mitigations note

Go/No-Go gate:
Release only when Sections 13, 16.7, and 17 all pass without open Critical or High findings.

