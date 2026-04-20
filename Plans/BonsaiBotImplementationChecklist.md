# BonsaiBot Implementation Checklist

Source of truth: BonsaiBotFinalDesign.md
Purpose: Convert final design into an execution checklist with clear done criteria.

## Phase 1: Foundation and Repo Scaffolding

- [ ] Create bonsai-bot workspace with Cargo.toml and feature flags.
- [ ] Add platform-gated dependencies and compile with default features.
- [ ] Add CI checks for default and all-feature build matrices.
- [ ] Add cargo audit to CI pipeline.
- [ ] Add baseline lint and formatting configs.

Done when:
- [ ] `cargo check` passes for default features.
- [ ] `cargo check --features all` passes.
- [ ] `cargo audit` reports no high/critical vulnerabilities.

## Phase 2: Core Runtime and Configuration

- [ ] Implement main runtime orchestration in main.rs.
- [ ] Implement schema-versioned BotConfig in config.rs.
- [ ] Implement keyring abstraction for all secrets.
- [ ] Implement startup validation for required platform secrets.
- [ ] Implement runtime tuning config for queues, workers, and breaker thresholds.

Done when:
- [ ] Bot starts with valid config and keyring entries.
- [ ] Missing secret disables only affected platform and emits error telemetry.
- [ ] Config reload path is stable and non-destructive.

## Phase 3: Data Persistence and Session Continuity

- [ ] Implement SQLite migrations for bot_sessions and pending_confirms.
- [ ] Implement ensure_session() fast-path and recreation path.
- [ ] Implement soft-archive and cleanup policy for stale sessions.
- [ ] Implement pending confirm state machine and prompt_nonce rules.
- [ ] Implement restart recovery for unexpired pending confirmations.

Done when:
- [ ] Session mapping persists across restarts.
- [ ] Deleted/orphaned Buddy session IDs are recreated automatically.
- [ ] Confirmation state transitions are deterministic and auditable.

## Phase 4: Buddy Integration and Protocol Contracts

- [ ] Implement Buddy client with retries and timeout policies.
- [ ] Implement circuit-breaker states (Closed/Open/HalfOpen).
- [ ] Implement typed confirm_required handling from Buddy response.
- [ ] Implement typed confirm_response injection to Buddy.
- [ ] Add backend contract updates for confirm_response handling.

Done when:
- [ ] High-risk tool request triggers structured confirmation flow.
- [ ] Approve and deny paths both execute correctly.
- [ ] Expired token path returns confirm_expired cleanly.

## Phase 5: Security Pipeline

- [ ] Implement allowlist checks per platform.
- [ ] Implement per-user rate limiting.
- [ ] Implement sanitizer with protocol boundary guards.
- [ ] Implement non-logging of raw rejected input.
- [ ] Add audit event emission for sensitive actions.
- [ ] Implement admin token generation and rotation.

Done when:
- [ ] Unauthorized sender is rejected before Buddy call.
- [ ] Rate-limit threshold rejects over-quota requests.
- [ ] Protocol-injection attempts are blocked pre-Buddy.
- [ ] Admin API requires valid bearer token (except health).

## Phase 6: Idempotency and Backpressure

- [ ] Implement dedup cache with TTL and LRU.
- [ ] Implement platform event ID mapping and fallback logic.
- [ ] Implement bounded inbound queue and overload shedding path.
- [ ] Implement low-cost control channel for busy replies.
- [ ] Implement global in-flight semaphore cap.

Done when:
- [ ] Duplicate inbound events do not trigger duplicate Buddy calls.
- [ ] Queue saturation does not crash runtime.
- [ ] Saturation response path remains responsive.

## Phase 7: Platform Adapters

### Discord
- [ ] Implement gateway handler and command routing.
- [ ] Implement slash commands and message-based interactions.
- [ ] Implement button confirmation callbacks.
- [ ] Implement embed and chunk formatting.

### Telegram
- [ ] Implement dispatcher and command routing.
- [ ] Implement inline-keyboard confirmations.
- [ ] Implement MarkdownV2-safe rendering and chunking.

### Matrix
- [ ] Implement sync loop and room/user allowlist filtering.
- [ ] Implement E2E crypto store persistence.
- [ ] Implement cross-signing and key backup flow.
- [ ] Implement confirmation response handling in-room.

### Email
- [ ] Implement IMAP IDLE/poll fallback.
- [ ] Implement robust SEARCH + code-side sender filtering.
- [ ] Implement SMTP multipart replies.
- [ ] Implement Message-ID and fallback hash dedup support.

Done when:
- [ ] All four adapters send and receive successfully in local tests.
- [ ] Confirmation behavior is consistent across adapters.

## Phase 8: Admin API and Metrics

- [ ] Implement /health route (unauthenticated).
- [ ] Implement authenticated routes: /status, /sessions, /broadcast, /config/reload, /config/rotate-admin-token.
- [ ] Implement metrics collection and exposure route.
- [ ] Add correlation IDs in logs and event traces.

Done when:
- [ ] Status reflects platform state accurately.
- [ ] Auth enforcement is consistent and tested.
- [ ] Metrics cover throughput, errors, sanitizer rejects, and queue pressure.

## Phase 9: Workspace Integration

- [ ] Add bot_server_port to workspace config.
- [ ] Add Tauri commands for bot status and platform config save/test.
- [ ] Add secure matrix key-backup reveal flow with proof + audit log.
- [ ] Add Bots tab to settings with live status polling.
- [ ] Implement launcher integration in PowerShell and launch-all.mjs.

Done when:
- [ ] Bots tab reflects live server and platform health.
- [ ] Platform token save/reload flow works without app restart.
- [ ] Bot auto-start behavior works when binary exists.

## Phase 10: Validation and Hardening

### Positive tests
- [ ] End-to-end per platform response tests.
- [ ] Session persistence test across bot restart.
- [ ] Confirmation approve/deny tests.
- [ ] Config reload and token rotation tests.

### Adversarial tests
- [ ] Unauthorized sender rejection test.
- [ ] Rate-limit threshold test.
- [ ] Duplicate event replay suppression test.
- [ ] Queue saturation resilience test.
- [ ] Circuit-breaker open/close behavior test.
- [ ] Malformed protocol payload handling test.
- [ ] Admin API unauthorized request test.

Done when:
- [ ] All acceptance criteria in BonsaiBotFinalDesign.md pass.
- [ ] No unresolved critical/high issues remain.

## Phase 11: Release and Operations

- [ ] Write operational runbook for outages and recovery.
- [ ] Write token lifecycle and secret management runbook.
- [ ] Write DB corruption and session recovery procedure.
- [ ] Define stable/beta/dev rollout policy.
- [ ] Define rollback procedure and criteria.

Done when:
- [ ] Operator documentation is complete and validated.
- [ ] Release gates are documented and reproducible.

## Exit Gate

Release is approved only when:

- [ ] MVP channels are fully operational.
- [ ] Security controls pass adversarial suite.
- [ ] SLO targets are met on repeated runs.
- [ ] Observability and runbooks are complete.
- [ ] Final sign-off artifacts are stored in repo.
