# Bonsai Bot — Production Mail Server Plan

Status: Draft
Date: 2026-04-19

Goal
----
Provide a reliable, secure, and observable email delivery system for `bonsai-bot` used for transactional notifications (confirmations, alerts, account-related emails, admin notifications) while minimizing operational overhead and maximizing deliverability.

Requirements
------------
- High deliverability (SMTP provider with good IP reputation)
- TLS for all outgoing SMTP connections
- DKIM, SPF, DMARC configured and validated
- Bounce & complaint handling and suppression list
- Persisted outbound queue with retry/backoff and idempotency
- Secrets and credentials stored securely (OS keyring / vault)
- Per-domain rate limiting and global quotas
- Monitoring, metrics, and alerting (success, failure, queue depth, bounce rate)
- Staging/dev test harness (Mailtrap / Ethereal) and integration tests
- Support attachments (bounded size), templating, localization

Provider Options (recommendation)
---------------------------------
1. Managed Transactional Email (Recommended)
   - Amazon SES, Postmark, or Mailgun
   - Pros: high deliverability, webhooks (bounces/complaints), API & SMTP, DKIM help, scale
   - Cons: vendor lock, cost
   - Recommendation: **Amazon SES** for scale and cost control or **Postmark** for developer-friendly transactional focus.

2. Managed Relay + Inbound Handling
   - Use provider for outbound and inbound webhook handling (SES + S3 + Lambda) if receiving inbound mail is required.

3. Self-hosted (not recommended for production deliverability)
   - Postfix + OpenDKIM + Dovecot. Good for full control but high ops/upkeep and reputational risk.

Architecture & Design
---------------------
- Bonsai-bot will use a pluggable `EmailSender` interface with two concrete implementations:
  - `smtp` using the `lettre` crate with STARTTLS/SMTPS.
  - `provider_api` using provider SDKs or REST APIs (SES, Postmark).
- Message flow:
  1. Application enqueues message into a durable `email_queue` table in SQLite (idempotency key, payload, status, attempts, next_retry_at, created_at).
  2. A single or small pool of background workers drain the queue, send email via configured provider, and update status.
  3. On transient failure, worker sets exponential-backoff with jitter and increments attempts; after N failures mark permanent failure and record bounce/error.
  4. Provider webhooks (bounce/complaint) are received by an admin endpoint and mapped to queue entries via Message-ID or custom X-headers; then mark as permanent failure and add to suppression list.
- Security:
  - Provider API keys / SMTP creds stored in OS keyring (`bonsai-bot` service), not on disk.
  - DKIM private keys stored in secured config dir with tight FS permissions; ideally use KMS/vault for secrets in cloud deployments.
- Deliverability:
  - Publish SPF records (include provider or SES), DKIM public key in DNS, and a DMARC record (p=quarantine → p=reject after monitoring).
  - Enforce TLS on connections and prefer API over SMTP where supported.

Implementation Plan (Phase 1 → Phase 3)
---------------------------------------
Phase 1 — Design + quick win (1–2 days)
- Add `EmailSender` trait and `smtp` implementation using `lettre`.
- Add `email_queue` table and worker task with simple retry/backoff.
- Add `set_smtp_credentials`/`has_smtp_credentials` Tauri assistant commands (already partially present) and ensure they store creds in keyring (done for assistant already).
- Add basic telemetry: metrics for sent, failed, queue depth.
- Integrate with a test provider (Mailtrap/Ethereal) for dev.

Phase 2 — Managed provider & webhooks (2–3 days)
- Add SES/postmark provider integration (API-based) with robust error parsing.
- HTTP endpoints to receive bounce/complaint webhooks; map to messages via custom `X-Bonsai-Id` header.
- Build suppression list and admin UI to inspect bounces.
- Implement DKIM signing helper (if using SMTP and self-hosted DKIM) or configure provider-managed DKIM.

Phase 3 — Harden & rollout (2–4 days)
- Configure DNS (SPF, DKIM, DMARC) and verify with monitoring.
- Add rate limiting, per-domain throttling, and retries cap.
- Add observability and alerts (high failure rate, high queue depth, bounce spikes).
- Run staged rollout: dev → staging (Mailtrap) → canary → production.

Operational Concerns
--------------------
- Abuse controls: validate templates and rate-limit template-driven mass messages.
- Data retention: what to store in `email_queue` (avoid storing PII raw in logs); truncate or redact sensitive content.
- Privacy & compliance: archival rules for email content, opt-out & suppression handling.
- Key rotation: provide commands to rotate SMTP/API keys and rotate DKIM keys when needed.

Testing Strategy
----------------
- Unit tests for queue worker and retry/backoff logic.
- Integration tests using Mailtrap/Ethereal to validate headers, DKIM signature (if applicable), and webhook handling.
- End-to-end smoke test: enqueue a confirmation email and verify delivery + webhook bounce handling.

APIs & Integration Points
-------------------------
- Internal: function `enqueue_email(to, subject, template_id, context, msg_id?)` returns job id.
- Admin: `POST /admin/email/retry/:job_id`, `GET /admin/email/queue`, `DELETE /admin/email/:job_id`.
- Webhooks: `/admin/email/webhook/<provider>` (validate signature or secret header)

Open Questions
--------------
- Preferred provider (SES vs Postmark vs Mailgun) — select one based on cost & deliverability goals.
- Expected volume (per month) to size queue workers and choose provider plan.
- Do we need inbound mail handling (user replies) or only outbound transactional messages?

Recommended Next Steps
----------------------
1. Confirm provider choice (SES recommended) and DNS ownership for DKIM/SPF.
2. Implement Phase 1 skeleton in `bonsai-bot` with queue + `lettre` SMTP sender and keys in keyring.
3. Add dev integration with Mailtrap and run E2E tests.
4. Implement Phase 2 webhooks and provider API integration.
5. Roll out to production behind monitoring and alerting.

Contact
-------
For implementation assistance, I can implement Phase 1 and wire Tauri UI hooks for SMTP creds and small admin views for queue inspection.
