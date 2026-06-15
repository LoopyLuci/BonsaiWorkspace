# Security notes

- **Server auth token**: The mobile automation server supports an optional bearer token (`AUTH_TOKEN` / `MOBILE_AUTOMATION_AUTH_TOKEN`). See `bonsai-workspace/tools/mobile-testing-automation/self-hosted-runner/README.md` for usage, examples, and how the token is propagated to Tauri commands and start scripts.
- **Content Security Policy (CSP)**: During development the Tauri CSP was expanded to allow local dev endpoints (http(s)://localhost:*, ws(s)://localhost:* and ipc.localhost variants). See `bonsai-workspace/src-tauri/tauri.conf.json`.
- **Database schema**: The canonical DB schema is in `DB_SCHEMA.md`.
- **Runtime security spec**: Additional security design notes are in `docs/runtime-security-spec.md`.

Responsible disclosure or security questions: open an issue or contact the repository maintainers.
