# BonsaiBot Final Design

Date: 2026-04-19
Status: Final integrated design specification
Sources synthesized: BonsaiBot.md (product vision), BonsaiMessagingBotServer.md (production server architecture)

## 1. Product Definition

BonsaiBot is a local-first, cross-platform personal assistant system that connects the channels users already use (Discord, Telegram, Matrix, Email first) to Bonsai Buddy intelligence and tool execution.

Design goals:

1. Cross-platform operation with no public IP requirement.
2. Local control plane with strict security defaults.
3. Full assistant capability with explicit confirmation gates for high-risk actions.
4. Durable, per-user session continuity across restarts.
5. Operational reliability under network, API, and platform failures.
6. Clear path to future channel expansion without destabilizing core runtime.

## 2. Scope and Release Model

### 2.1 MVP Scope (Production Core)

Supported channels in MVP:

1. Discord
2. Telegram
3. Matrix
4. Email (IMAP + SMTP)

Out of MVP (backlog):

1. WhatsApp, Signal, iMessage, Slack, Teams, and other concept channels from BonsaiBot.md
2. Voice Wake, Talk Mode, and Live Canvas as first-class bot endpoints
3. Multi-node companion orchestration beyond existing Bonsai surfaces

### 2.2 Release Channels

1. Stable: production-grade, migration-safe releases.
2. Beta: pre-release validation with feature flags enabled selectively.
3. Dev: rapid iteration with expanded diagnostics and stricter assertions.

## 3. System Architecture

## 3.1 Topology

1. bonsai-bot: standalone Rust binary, local service endpoint on 127.0.0.1:11421.
2. bonsai-workspace (Tauri app): configuration and status UI control plane.
3. bonsai-bot to Buddy API: local HTTP integration at 127.0.0.1:11420.
4. Platform integrations: outbound connections only (NAT-friendly).

## 3.2 Core Runtime Pipeline

Inbound pipeline stages (deterministic order):

1. Platform adapter ingest.
2. Dedup check (event ID or fallback hash).
3. Allowlist and identity validation.
4. Rate limit check.
5. Input sanitizer and protocol-boundary guard.
6. Session resolution and continuity mapping.
7. Buddy request execution with timeout and circuit-breaker policy.
8. Confirmation gate handling where required.
9. Platform-aware formatting and outbound send.
10. Metrics and audit emission.

## 3.3 Concurrency and Backpressure

1. Bounded inbound queue (default 1024).
2. Separate low-cost control path for overload notices.
3. Worker pool (default 8 workers).
4. Global in-flight semaphore (default 64 Buddy requests).
5. Per-platform bounded outbound queues (default 256 each).

## 4. Component Design

### 4.1 bonsai-bot modules

1. main.rs: lifecycle orchestration, task supervision, graceful shutdown.
2. config.rs: schema-versioned config and keyring integration.
3. session.rs: SQLite session mapping and pending confirmation persistence.
4. buddy_client.rs: resilient Buddy API client with retries and breaker integration.
5. router.rs: security, routing, confirmation flow, and policy coordination.
6. sanitizer.rs: structural input safety and protocol-injection boundary protection.
7. dedup.rs: TTL dedup cache with platform-specific keys.
8. formatter.rs: platform rendering and chunking.
9. admin_api.rs: loopback admin service with bearer auth.
10. health.rs: health watcher and breaker state transitions.
11. metrics.rs: counters, latency histograms, and error classifications.
12. platforms/*: channel adapters implementing a unified MessagingPlatform trait.

### 4.2 bonsai-workspace integration

1. Settings tab for Bots with per-platform config and live status.
2. Tauri commands for status, config save, platform tests, and secure maintenance operations.
3. Launcher integration for auto-start and process supervision hints.

## 5. Protocol Contracts

### 5.1 Buddy completion contract

1. Non-streaming chat completions for bot channel response consistency.
2. Structured extension field for bot-specific control metadata.
3. Strongly typed confirmation contract, no magic string transport.

### 5.2 Confirmation contract

1. Buddy emits typed confirm_required metadata with token, risk, tool, prompt, and expiry.
2. Bot persists pending confirmation state in SQLite.
3. User confirms via channel-native UX.
4. Bot emits typed confirm_response metadata.
5. Buddy validates token and executes or denies.

### 5.3 Idempotency contract

1. Primary event IDs per platform.
2. Email fallback hash key when Message-ID is absent.
3. TTL dedup to block replay and duplicate execution.

## 6. Security Model

### 6.1 Trust boundaries

1. All inbound user content is untrusted.
2. Admin API is local-only and token-gated.
3. Secret values are keychain-only.
4. High-risk tool operations require explicit confirmation.

### 6.2 Controls

1. Allowlists per platform.
2. Per-user token-bucket rate limits.
3. Sanitization for structural safety and protocol boundary enforcement.
4. Confirmation TTL and nonce/state protections.
5. Audit events for sensitive operations, including secret-reveal workflows.

### 6.3 Secret lifecycle

1. Generate bot_admin_token on first start.
2. Rotation endpoint with immediate invalidation of prior token.
3. Revocation path per platform token.
4. Invalid token detection and user-visible remediation signal.

## 7. Data Model

### 7.1 bot_sessions

Maps (platform, user, chat) to buddy_session_id with soft-archive lifecycle.

### 7.2 pending_confirms

Stores token, user context, prompt payload, expiry, and prompt state machine.

State machine:

1. created
2. prompted
3. resolved
4. expired

## 8. Reliability and Recovery

### 8.1 Circuit breaker

Configurable thresholds:

1. open_after_failures
2. half_open_probe_secs
3. close_on_successes

### 8.2 Failure handling

1. Platform reconnect loops are adapter-owned.
2. Buddy outages degrade gracefully with explicit user messaging.
3. SQLite lock and corruption paths include retry and recovery flow.
4. Queue saturation sheds load without process crash.

## 9. Platform Adapter Requirements

### 9.1 Discord

1. Message + slash-command support.
2. Component-based approval buttons.
3. Embed-safe formatting and chunking.

### 9.2 Telegram

1. Command and free-text handling.
2. Inline keyboard confirmation.
3. MarkdownV2 escaping and split logic.

### 9.3 Matrix

1. Sync loop with E2E support.
2. Key backup and recovery flow.
3. Prefix commands plus direct-room conversational path.

### 9.4 Email

1. IMAP IDLE with polling fallback.
2. Safe SEARCH strategy with code-side allowlist filtering.
3. Multipart SMTP replies with HTML and plain-text fallback.

## 10. Performance and SLOs

Target SLOs:

1. Discord and Telegram p95 response <= 5s.
2. Matrix p95 response <= 8s.
3. Email p95 response <= 35s.
4. Circuit recovery <= 35s after Buddy restoration.
5. Queue-drop rate <= 1 percent under saturation events.
6. Sanitizer false-positive rate < 0.5 percent.

## 11. Verification Strategy

### 11.1 Positive validation

1. Health and status endpoint correctness.
2. End-to-end messaging per channel.
3. Session persistence and restart continuity.
4. Confirmation approve and deny flows.

### 11.2 Adversarial validation

1. Unauthorized sender rejection before Buddy call.
2. Rate-limit enforcement.
3. Duplicate event suppression.
4. Expired confirmation behavior.
5. Admin API unauthorized access rejection.
6. Injection and malformed metadata resilience.

## 12. Rollout Plan

1. Build bonsai-bot core and one channel adapter (Discord or Telegram) first.
2. Validate full security and reliability baseline.
3. Add remaining MVP adapters incrementally with contract tests.
4. Enable Bots tab and launcher integration.
5. Run full positive and adversarial test suite.
6. Promote from beta to stable after SLO and security gates pass.

## 13. Governance and Observability

1. Structured logs with correlation IDs per inbound event and buddy turn.
2. Metrics endpoints and dashboard-ready counters.
3. Auditable security events (token rotation, secret reveal, confirmation resolution, policy denials).
4. Runbooks for outage, token invalidation, and DB recovery scenarios.

## 14. Final Acceptance Gates

BonsaiBot is release-ready when all conditions below are met:

1. All MVP channels pass end-to-end tests.
2. Security controls pass adversarial suite.
3. No unresolved critical findings remain.
4. SLOs pass under repeat runs.
5. Launcher and settings integration are stable.
6. Documentation and runbooks are complete and accurate.

## 15. Non-Goals for This Release

1. Expanding to every concept channel from the reference document.
2. Full voice and companion-node orchestration parity.
3. Deep multi-agent workspace routing beyond existing Buddy architecture.

This final design intentionally prioritizes secure, reliable, and extensible delivery over breadth-first channel expansion.
