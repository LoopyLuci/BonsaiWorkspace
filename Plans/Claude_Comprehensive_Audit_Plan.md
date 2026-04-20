# Claude Comprehensive Project Audit Plan

Owner: Claude
Date: 2026-04-18
Scope: Full quality and completion audit for Bonsai Workspace + Bonsai Buddy + backend + tooling + VS Code extension

## 1. Audit Mission

Deliver a verifiable, evidence-backed determination of:

1. Feature completeness against intended behavior.
2. Code quality and maintainability.
3. Reliability and runtime safety.
4. Security and permission correctness.
5. Test and release readiness.

This plan is execution-oriented. Every section includes what to inspect, how to validate, what evidence to capture, and pass/fail gates.

## 2. Audit Principles

1. Evidence over assumptions: every claim links to code, logs, test output, or runtime traces.
2. Deterministic validation first: validate core behavior with reproducible scripts before manual UI checks.
3. Risk-prioritized depth: security, data integrity, and core chat/tool runtime are highest risk.
4. No blind spots: include desktop, mobile, assistant window, extension, launchers, and CI/test harnesses.
5. Completion grading: classify each feature as Complete, Partial, Missing, or Regressed.

## 3. Scope Boundaries

### In scope

1. Frontend app in bonsai-workspace/src (Workspace and Buddy UI flows).
2. Rust/Tauri backend in bonsai-workspace/src-tauri/src.
3. VS Code extension in vscode-extension/src.
4. Launchers, scripts, and test harnesses.
5. Documentation fidelity against implementation.

### Out of scope for deep code-quality review (but still sanity-check)

1. Generated build artifacts (dist, target, target-ci-local, maps, object files).
2. Third-party vendor blobs (for example bundled opencv.js) except integration and update risk.

## 4. Completion Rubric

Use this rubric for each feature/module:

1. Complete: behavior implemented, tested, documented, and stable.
2. Partial: implemented but missing edge cases/tests/docs or has known instability.
3. Missing: planned/expected behavior absent.
4. Regressed: previously working behavior currently broken or degraded.

Severity for findings:

1. Critical: security/data-loss/crash/core-runtime failure.
2. High: major feature broken or unsafe behavior likely.
3. Medium: correctness or UX quality issues with workarounds.
4. Low: maintainability, readability, minor UX gaps.

## 5. Repository Surface Map

### 5.1 Core product surfaces

1. Workspace desktop shell and multi-pane IDE: bonsai-workspace/src/App.svelte.
2. Buddy standalone app root: bonsai-workspace/src/AssistantApp.svelte and bonsai-workspace/src/lib/components/BonsaiAssistant.svelte.
3. Shared model picker and model orchestration UI: bonsai-workspace/src/lib/components/ModelSelector.svelte.
4. Backend orchestration/runtime/tooling: bonsai-workspace/src-tauri/src/*.rs.
5. Extension integration surface: vscode-extension/src/*.ts.

### 5.2 Frontend feature components to audit

Audit all files under bonsai-workspace/src/lib/components, with emphasis on:

1. Chat and assistant runtime: ChatPanel.svelte, AssistantInputBar.svelte, AssistantMessageList.svelte, AssistantMessage.svelte, ToolConfirmCard.svelte, InlineToolResult.svelte.
2. Model and bootstrap: ModelSelector.svelte, DownloadProgress.svelte, BootstrapScreen.svelte.
3. Project shell: FileTree.svelte, MonacoEditor.svelte, TerminalPanel.svelte, CommandPalette.svelte, StatusBar.svelte, SettingsPanel.svelte, SessionPanel.svelte.
4. Agent systems: AgentConnectPanel.svelte, AgentsPanel.svelte, AgentVisionPanel.svelte, ClusterControlPanel.svelte, ResourcesPanel.svelte.
5. Canvas systems: CodeCanvas.svelte and all files in components/canvas.
6. Buddy management: AssistantToolbar.svelte, ProfileManager.svelte, AvatarPicker.svelte, BackupManager.svelte, AssistantSessionHistory.svelte, AssistantDiagnostics.svelte.
7. Mobile surfaces: MobileLayout.svelte, MobileHome.svelte, MobileSettingsPanel.svelte, MobileViewPanel.svelte, AssistantMobile.svelte.
8. VS Code viewer surface: VscodeViewer.svelte, VscodeFileEntry.svelte.

### 5.3 Frontend state and utility modules to audit

1. Stores under bonsai-workspace/src/lib/stores:
   - agents.ts, assistant.ts, assistantSessions.ts, chat.ts, models.ts, settings.ts, terminal.ts, vision.ts, canvas.ts, cluster.ts, catalog.ts, diff.ts, editorTooling.ts, openFile.ts, mobileDisplay.ts, toast.ts, activeEditorFile.ts, vscodeState.ts.
2. Utilities under bonsai-workspace/src/lib/utils:
   - filetypes.ts, monaco.ts, visionContext.ts, wsClient.ts.
3. Constants under bonsai-workspace/src/lib/constants:
   - network.ts.

### 5.4 Backend Rust modules to audit

Audit every file in bonsai-workspace/src-tauri/src, with priority order:

1. Assistant/tool runtime: assistant_manager.rs, assistant_tools.rs, assistant_policy.rs, assistant_commands.rs, assistant_store.rs, assistant_backup.rs, assistant_metrics.rs, assistant_audit_log.rs.
2. API/runtime servers: api_server.rs, buddy_api_server.rs.
3. Model orchestration: model_orchestrator.rs, model_registry.rs, sidecar_manager.rs, bootstrap.rs.
4. Agent/swarm/cluster: swarm_orchestrator.rs, cluster_orchestrator.rs, agent_connect.rs, agent_store.rs.
5. Tool routing/cache/selectors: tools.rs, tool_core.rs, tool_selector.rs, tool_cache.rs.
6. Platform/system integration: commands.rs, remote.rs, remote_input.rs, tts_manager.rs, secrets_store.rs, user_skills.rs, wal.rs, chat_sessions.rs, mcp_bridge.rs, config.rs, lib.rs, main.rs, action_parser.rs.

### 5.5 VS Code extension modules to audit

1. extension.ts, command-handler.ts, bonsai-client.ts, state-streamer.ts, constants.ts.
2. Tests in vscode-extension/src/test/state-streamer.test.ts.

### 5.6 Scripts and automation to audit

1. Frontend scripts and orchestration files in bonsai-workspace/src:
   - launch-all.mjs, agent-api-smoke.mjs, agent-routing-ci.mjs, agent-ui-live-orchestrated.mjs, agent-orchestrate-all.mjs, bonsai-live-testing-feature.mjs, android-usb-regression.mjs, android-orientation-ux-regression.mjs, android-remote-surface-e2e-smoke.ps1, append-usb-evidence-ledger.ps1.
2. Launcher and root scripts:
   - Launch-BonsaiWorkspace.cmd, Launch-BonsaiWorkspace.ps1, Generate-BonsaiDesktopShortcut.cmd, Generate-BonsaiDesktopShortcut.ps1, bb-launch-preflight.clj, bb.edn.
3. Audit artifacts and regression evidence in tool_test and bonsai-workspace/src/tool_test.

## 6. Feature-Level Audit Matrix

For each feature area below, produce a status, findings, and evidence links.

### 6.1 Workspace Shell and IDE Flow

Checks:

1. Pane toggles, resizers, persisted behavior, keyboard shortcuts.
2. Editor load fallback behavior and error messaging.
3. File tree operations and context actions.
4. Terminal log routing and event visibility.

Evidence:

1. UI screenshots/video.
2. Console logs and app event logs.
3. Repro script for regressions.

### 6.2 Chat and Tool-Use Runtime

Checks:

1. Streaming, cancellation, token status, state reset behavior.
2. HITL permission flow end-to-end (allow/deny/timeouts/resume).
3. Tool result rendering and sanitization.
4. Failure handling when model unavailable or loading.

Evidence:

1. Request/response logs for chat completions.
2. tool-used and permission-resolved events.
3. Regression scripts and replay transcripts.

### 6.3 Model Management and Selection

Checks:

1. Model list freshness, active model state, switch robustness.
2. Download workflow, progress state, failure recovery.
3. Auto mode and custom swarm mode interactions.
4. Popup/selector responsiveness and accessibility.

Evidence:

1. Switch latency and failure metrics.
2. UI behavior on small and large viewport.
3. Process-level validation for model server launches.

### 6.4 Agent/Swarm/Cluster Features

Checks:

1. Persona CRUD, role assignment, model assignment, policy toggles.
2. Parallelization behavior and ordering guarantees.
3. Resource estimation correctness and safety gates.
4. Cluster control paths and fallback behavior.

Evidence:

1. Deterministic swarm test results.
2. Memory gate acceptance/rejection traces.
3. Ordered output proof for worker synthesis.

### 6.5 Buddy App and Management Features

Checks:

1. Buddy window lifecycle, pin/hide, toolbar actions.
2. Session history create/load/delete/undo integrity.
3. Profile edits and persistence consistency.
4. Backup export/import/verify behavior and rollback safety.
5. Avatar and TTS/lip-sync event handling including mobile fallback.

Evidence:

1. End-to-end flow recordings.
2. DB state before/after operations.
3. Backup archive contents and integrity checks.

### 6.6 Mobile and Android Paths

Checks:

1. Mobile layout routing and component behavior.
2. Android remote surface entry, lifecycle, unsupported-webview fallback.
3. USB regression workflows and evidence ledger integration.
4. Orientation and touch interaction regressions.

Evidence:

1. Android log captures.
2. Script outputs from remote surface and USB tests.
3. Device matrix notes (OS/webview variations).

### 6.7 API and Backend Correctness

Checks:

1. Health endpoints and server readiness semantics.
2. OpenAI-compatible routes correctness (/v1/chat/completions, /v1/models).
3. Buddy and workspace endpoint separation and consistency.
4. Tool execution gating and policy enforcement.
5. Recycle/restart behavior and recovery guarantees.

Evidence:

1. HTTP contract tests.
2. Structured logs from assistant runtime.
3. Recovery tests after forced recycle/crash.

### 6.8 Security and Secrets

Checks:

1. Command/tool permission boundaries.
2. Path and URL validation for file/web tools.
3. Secret handling and storage boundaries (secrets_store.rs).
4. Injection and unsafe rendering vectors in chat/tool outputs.

Evidence:

1. Negative tests for blocked operations.
2. Code references to validation paths.
3. Sanitization test cases and outputs.

### 6.9 Performance and Stability

Checks:

1. Startup and first-response latency.
2. Memory/CPU behavior with idle and heavy workflows.
3. Bundle size budgets and chunk warnings.
4. Long-session stability and leak indicators.

Evidence:

1. Timing captures and trend table.
2. Resource snapshots before/after stress runs.
3. Build/bundle reports and threshold checks.

### 6.10 VS Code Extension Integration

Checks:

1. Extension activation and command wiring.
2. State streaming and error handling.
3. Contract compatibility with backend endpoints.
4. Unit test coverage and missing cases.

Evidence:

1. Extension host logs.
2. Unit test outputs.
3. Manual command invocation checks.

## 7. Module-by-Module Code Quality Audit Template

Apply this template to every source module:

1. Intent and responsibilities.
2. Public interfaces and contract clarity.
3. Error handling and edge-case behavior.
4. Concurrency/asynchrony safety.
5. Security validation and trust boundaries.
6. Test coverage quality and missing tests.
7. Dead code/duplication/technical debt.
8. Refactor recommendations with estimated risk.

## 8. Required Test Execution Matrix

Run and capture results for:

1. Frontend build and type checks.
2. Rust compile checks and backend tests.
3. Core smoke scripts:
   - npm run test:agent-api
   - npm run test:agent-routing-ci
   - npm run test:agent-ui-live-orchestrated
   - npm run test:bonsai-live-testing-feature
4. Android workflows where available:
   - npm run test:android-usb-regression
   - android-remote-surface-e2e-smoke.ps1
5. Extension tests in vscode-extension.
6. Launcher preflight checks and generated reports.

For each test run capture:

1. Command.
2. Environment (ports, model state, device info).
3. Pass/fail.
4. Failure signature.
5. Linked logs/artifacts.

## 9. Documentation and Spec Alignment Audit

Validate that docs match implementation:

1. README.md.
2. Bonsai_Assistant.md.
3. Bonsai_Assistant_Audit.md.
4. Multi-Agent_Swarm.md.
5. Runner-Streaming_System.md.
6. Spatial_Code_Canvas.md and suggestions.
7. bonsai-workspace/user_manual.md and launcher_manual.md.

Output per document:

1. Accurate.
2. Outdated.
3. Missing sections.
4. Contradictions.

## 10. Required Deliverables for Claude

Claude should produce these artifacts:

1. Executive summary with readiness score.
2. Feature completion matrix (Complete/Partial/Missing/Regressed).
3. Detailed findings list ordered by severity.
4. Module-by-module audit appendix.
5. Test evidence index with command logs.
6. Risk register with mitigation plan.
7. Final release recommendation:
   - Ship
   - Ship with conditions
   - No-ship

## 11. Reporting Format

### 11.1 Findings format

1. ID
2. Severity
3. Area/module
4. Problem statement
5. Reproduction steps
6. Evidence
7. Recommended fix
8. Validation method

### 11.2 Completion matrix format

Columns:

1. Feature area
2. Expected behavior
3. Current behavior
4. Status
5. Evidence links
6. Blocking issues

## 12. Execution Order (Recommended)

1. Baseline environment validation and endpoint map.
2. Backend/API contract and core chat runtime.
3. Workspace UI core shell and editor/terminal.
4. Model management and tool-call gating.
5. Agents/swarm/cluster behavior.
6. Buddy window and management workflows.
7. Mobile/Android and USB flows.
8. Extension integration and tests.
9. Documentation consistency pass.
10. Consolidated risk scoring and release recommendation.

## 13. Minimum Acceptance Gates

All must pass before calling the project production-ready:

1. No open Critical findings.
2. No unresolved High findings in core runtime, security, or data integrity.
3. Core feature matrix at 95 percent or higher Complete for claimed production features.
4. Stable test matrix with deterministic pass on repeat runs.
5. Documentation updated for all user-visible behavior and operational commands.

## 14. Known Audit Hotspots to Prioritize

1. assistant_manager.rs intent routing and deterministic tool paths.
2. assistant_policy.rs and permission flow boundaries.
3. api_server.rs and buddy_api_server.rs route consistency and model-recycle behavior.
4. ModelSelector.svelte cross-surface UI behavior in Workspace and Buddy.
5. ChatPanel.svelte HITL and streaming state transitions.
6. BackupManager.svelte and assistant_backup.rs import/export integrity and safety.
7. AgentVisionPanel.svelte and mobile remote surface lifecycle edge cases.
8. Launcher/preflight scripts and port/runtime attach behavior.

## 15. Final Instruction to Claude

Audit this codebase as if preparing a production launch sign-off. Do not stop at surface checks. Confirm behavior end-to-end, tie every judgment to evidence, and clearly separate implementation defects from documentation or test gaps. Provide explicit remediation steps and a prioritized closure plan.
