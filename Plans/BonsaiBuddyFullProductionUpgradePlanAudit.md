# Bonsai Buddy Full Production Upgrade Plan - Audit

## Scope Audited

- Source: `BonsaiBuddyFullProductionUpgradePlan.md`
- Audit date: 2026-04-18
- Goal: Validate plan completeness, execution order, risk coverage, and testability for production rollout.

## Executive Assessment

The plan is strong and implementation-ready overall. It is phased correctly, separates hard blockers from polish work, and includes a practical acceptance matrix. The biggest remaining delivery risk is not plan quality, but **runtime consistency during verification** (stale processes and missing one-command recycle flow), plus a few **cross-file consistency checks** that must be enforced while landing patches.

## Findings (Ordered By Severity)

### High

1. **Operational recycle path was underspecified for repeatable validation**
	- Impact: Without a stable API recycle endpoint, "forced recycle" checks become manual and less repeatable.
	- Risk: False negatives/positives during regression loops.
	- Recommendation: Add and standardize `/v1/admin/recycle` with deterministic response payload (recycled slots + per-slot errors).

2. **Window/session UX persistence requirements were not explicitly codified as acceptance criteria**
	- Impact: User-visible regression risk after restart (window geometry reset, Buddy visibility mismatch).
	- Risk: Plan can be marked complete while startup UX still feels unstable.
	- Recommendation: Add acceptance rows for persisted main window bounds and Buddy open/closed restoration.

### Medium

1. **Event-target migration needs strict static grep gates**
	- Impact: Any leftover `emit_to("assistant")` usage can silently reintroduce split-stream UI bugs.
	- Risk: Intermittent behavior under mixed command/update flows.
	- Recommendation: Add CI grep check for forbidden assistant-target stream emits in runtime paths.

2. **Tool-call ID migration read-compat strategy is correct but should include rollback note**
	- Impact: If malformed historical records exist, migration logic may partially hydrate UI state.
	- Risk: Non-fatal but confusing tool-card rendering.
	- Recommendation: Document rollback behavior and fallback UI treatment for invalid IDs.

3. **Proxy resilience section is solid but should define explicit timeout budget by branch**
	- Impact: Distinguishing retryable 502 from hard failures may drift over time.
	- Risk: Latency spikes or premature user-facing errors.
	- Recommendation: Pin concrete retry count/backoff timing in the plan.

### Low

1. **Observability section should include one canonical "golden trace" sample**
	- Impact: Harder to audit expected logs during incident review.
	- Recommendation: Add one example session trace (request id -> model selection -> stream -> completion/error).

2. **Z-index and hotkey UX checks should include one screenshot baseline per platform**
	- Impact: Visual regressions may pass functional checks.
	- Recommendation: Store screenshot refs in acceptance artifacts.

## Coverage Matrix (Plan Quality)

- Architecture sequencing: **Pass**
- API/runtime reliability: **Pass with operational hardening needed**
- Data migration safety: **Pass**
- UI safety and rendering policy: **Pass**
- Observability and diagnosability: **Pass with minor gaps**
- Regression testability/repeatability: **Pass with recycle endpoint requirement**

## Readiness Verdict

**Conditionally Approved** for production implementation.

The plan can proceed as written if the following guardrails are enforced during execution:

1. Add a deterministic recycle endpoint and use it for all recycle validation.
2. Add explicit UX persistence acceptance criteria (window geometry + Buddy visibility).
3. Add grep/CI checks for stream emit scope and tool_call_id schema consistency.

## Suggested Immediate Addendum

Add a short "Phase 0.5 - Runtime Determinism" section:

- Deliver `/v1/admin/recycle`.
- Add one-command recycle + layers distribution script.
- Record expected non-empty `LAYERS` distribution output format.

This keeps the rest of the plan unchanged while making verification repeatable and auditable.
