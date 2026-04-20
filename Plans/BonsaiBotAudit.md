# BonsaiBot Concept Audit

Date: 2026-04-19
Source: BonsaiBot.md
Audit Type: Concept and implementation-readiness audit

## Executive Assessment

Overall status: Partial (vision is strong, implementation contract is incomplete).

The document is effective as a concept reference but not yet executable as a build or delivery specification. It combines product vision, platform claims, and operational commands without defining concrete integration boundaries for Bonsai Workspace and Bonsai Buddy.

## Strengths

1. Clear local-first positioning and user value proposition.
2. Strong multi-channel ambition with practical security themes (pairing, sandboxing).
3. Good operator surface ideas (doctor command, channel policies, update channels).
4. Useful feature grouping by voice, canvas, tools, and companion nodes.

## Critical Findings

1. Scope explosion without prioritization:
	The platform list is very broad and mixes production and aspirational channels, creating high delivery risk and unclear MVP.

2. No Bonsai-specific integration contract:
	The document states this must be customized for Bonsai Workspace and Bonsai Buddy, but does not define API boundaries, event contracts, session mapping, or error semantics.

3. Security model is conceptual but not enforceable:
	DM policies and sandbox defaults are described, but there is no policy evaluation order, trust-tier model, or explicit deny-by-default matrix for each tool family.

4. Missing persistence model:
	No explicit schema for sessions, sender allowlists, pairing state, confirmations, or audit logs.

5. Inconsistent platform assumptions:
	The doc spans macOS launchd, Linux systemd, and Windows via WSL2 recommendation, but does not define first-class Windows runtime support despite current workspace context being Windows-heavy.

## High Findings

1. Ambiguous tool boundary:
	"First-class tools" are named, but request/response schemas, side-effect classes, and confirmation requirements are not defined.

2. Missing observability requirements:
	There are no required logs, trace IDs, metrics, or health endpoints for production operation.

3. No rollout and rollback strategy:
	Update channels are listed, but there is no controlled rollout plan, feature flags, or rollback policy.

4. Missing threat model:
	No explicit abuse cases for untrusted inbound content, prompt injection, replay, impersonation, or token theft.

## Medium Findings

1. Documentation quality issues:
	Minor formatting and path inconsistencies (for example malformed config path quotes) reduce implementation confidence.

2. Command surface not tied to authorization levels:
	Chat commands and operator commands are listed without privilege tiers.

3. Build instructions are upstream-oriented:
	The source build section appears to target an external project flow rather than Bonsai Workspace integration flow.

## Readiness Score

1. Product vision readiness: 8/10
2. Security design readiness: 4/10
3. Integration readiness: 3/10
4. Delivery readiness: 3/10
5. Overall implementation readiness: 4/10

## Required Remediation Before Build Execution

1. Define MVP scope:
	Choose initial supported channels and defer the rest.

2. Define Bonsai integration contract:
	Specify APIs, metadata, session continuity, confirmation signaling, and failure handling.

3. Define enforceable policy model:
	Tool risk classes, trust tiers, gating rules, allowlist semantics, and escalation paths.

4. Define persistence and audit schema:
	Sessions, pending confirmations, pairing approvals, security events, and retention.

5. Define observability baseline:
	Health endpoints, structured logs, trace correlation IDs, and minimum metrics set.

6. Define platform support matrix:
	Explicitly mark supported, experimental, and backlog platforms for Windows/macOS/Linux.

## Recommended Reframe

Treat this document as a product vision artifact, then generate a separate implementation spec that is constrained, testable, and directly mapped to Bonsai code surfaces.

## Suggested Completion Criteria for This Doc

Mark BonsaiBot.md complete only when:

1. MVP channels are explicitly declared.
2. API and policy contracts are concrete.
3. Persistence and security models are specified.
4. Verification steps are deterministic and reproducible.
5. Rollout and rollback controls are documented.
