# Bonsai Messaging Bot Server Audit (v2)

Date: 2026-04-19
Audited document: BonsaiMessagingBotServer.md
Audit type: Design and implementation-readiness review

## Findings

### Critical

1. Admin token key mismatch will break authenticated Tauri-to-bot calls.
	Evidence: key account is defined as `admin_api_token` (line 194), but Tauri integration fetches `bot_admin_token` (line 566 and line 283).
	Impact: UI status checks and admin API calls can fail even when the token exists.
	Fix: standardize one canonical key name across config docs, keyring wrapper, and Tauri commands.

2. Sensitive Matrix recovery secret is intentionally exfiltratable via UI command.
	Evidence: `get_matrix_key_backup_passphrase()` explicitly returns backup passphrase to UI (line 490 and line 586).
	Impact: expands secret exposure path and increases theft risk from local compromise, logs, or screenshot leakage.
	Fix: require explicit one-time reveal flow with re-auth, redaction by default, and audit logging; avoid returning raw secret on normal command paths.

### High

1. Sanitizer implementation uses `regex::Regex` but `regex` crate is not declared in dependencies.
	Evidence: sanitizer pseudo-code uses `regex::Regex::new(...)` (line 309); dependency list does not include `regex` (lines 106-129).
	Impact: implementation will not compile as specified.
	Fix: add `regex` dependency and precompile patterns once (lazy static) to avoid per-message compile overhead.

2. Injection defense can over-block legitimate user intent and under-specify structured safety.
	Evidence: deny-list blocks phrases like "ignore previous instructions" and literal `bonsai_ext` (lines 300-308).
	Impact: false positives for benign discussions; brittle detection that can be bypassed by paraphrase.
	Fix: move from phrase deny-list to policy-layer intent classification plus strict boundary checks on protocol fields, with telemetry for false positives.

3. Email dedup strategy relies on Message-ID only, which is not guaranteed.
	Evidence: dedup source for Email is Message-ID header (line 350); duplicate test assumes Message-ID present (line 701).
	Impact: duplicate replies possible for emails missing/malformed Message-ID.
	Fix: define fallback dedup key (hash of sender+date+subject+body snippet) when Message-ID absent.

4. Confirmation replay behavior after restart is underspecified and can produce duplicate prompts/actions.
	Evidence: restart behavior says unexpired confirms are reloaded and prompt resent (line 266).
	Impact: user confusion, double-confirm race if platform also retries interaction callbacks.
	Fix: include idempotency key per confirmation prompt and explicit prompt-state transitions (`created`, `prompted`, `resolved`, `expired`).

### Medium

1. Security statement is overconfident for localhost threat model.
	Evidence: "No CSRF risk" assertion (line 281).
	Impact: may understate risk from local malware, browser extension abuse, or stolen bearer token.
	Fix: reword as reduced remote risk, and require defense-in-depth controls (short token TTL, optional local process binding checks, request origin telemetry).

2. Queue-full behavior promises a reply without guaranteeing delivery channel availability.
	Evidence: on full queue, design says reply immediately with busy message (line 355).
	Impact: in overloaded state, busy replies may also fail or amplify pressure.
	Fix: define separate low-cost control path and per-platform shed strategy with retry-after hints.

3. Circuit-breaker thresholds are fixed constants without per-platform tuning guidance.
	Evidence: opens after 5 failures, half-open probe at 30s (line 362).
	Impact: can flap in unstable networks or be too slow in recovery.
	Fix: make thresholds configurable with safe defaults and expose metrics-driven tuning guidance.

4. IMAP SEARCH expression is shown conceptually but not valid for direct implementation as written.
	Evidence: pseudo query `FROM allowed_from_addrs[0] OR FROM ...` (line 518).
	Impact: implementers may ship broken mailbox filters.
	Fix: specify exact IMAP query-building algorithm and fallback per server capability.

### Low

1. Minor terminology inconsistencies remain between document sections.
	Evidence: references to both generic loopback assumptions and explicit bearer flow can read contradictory (lines 268-283).
	Impact: small integration confusion.
	Fix: unify language in one security assumptions section.

2. Verification set is strong but lacks explicit latency/error SLO targets.
	Evidence: positive/negative checks are present (lines 675-717), but no formal thresholds for p95 latency or error budgets.
	Impact: pass/fail criteria may be subjective.
	Fix: add measurable operational SLOs (for example p95 reply latency, max error rate, recovery time objective).

## What Is Strong in v2

1. Structured confirmation protocol replaced magic-string control flow.
2. Backpressure, dedup, and circuit-breaker concerns are explicitly addressed.
3. Failure and recovery matrix is materially improved over earlier drafts.
4. Security posture is significantly stronger with keychain-based secret handling and allowlists.

## Release Readiness Verdict

Status: Conditional proceed.

Proceed to implementation after resolving all Critical and High findings, then run adversarial verification again (token mismatch, secret exposure path, sanitizer false-positive rate, and duplicate-event handling).
