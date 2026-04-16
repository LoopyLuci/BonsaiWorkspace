# Bonsai Workspace Prompt Templates

## Prompt: Fix blank screen or startup failure
"Review `src/App.svelte` and the Tauri frontend initialization path. Help me identify why the app shows a blank screen, whether there is a missing Monaco worker, CSP issue, or runtime error in the console."

## Prompt: Debug sidecar startup
"Inspect the Rust sidecar orchestration and bootstrap flow. Help me find why `llama-server` or `whisper-server` won't start, and suggest safer runtime spawning, readiness checks, and path traversal protections."

## Prompt: Improve terminal PTY integration
"Review the PTY terminal flow from `spawn_pty_terminal` to frontend event emission. Explain why terminal output may not appear and propose how to keep `send_to_pty` and `resize_pty` working reliably."

## Prompt: Enhance inline diff UX
"Inspect the Monaco diff decorations and inline accept/reject widget implementation. Help me ensure the diff hunks render correctly and the `accept_diff_hunk` flow applies changes without breaking existing editor state."

## Prompt: Add command palette support
"Add or improve keyboard handling for the command palette while preserving existing Ctrl+K behavior. Keep the palette accessible, support action execution, and avoid introducing conflicts with other shortcuts."

## Prompt: Review model download and orchestrator logic
"Review `model_orchestrator.rs` and the model registry scanning logic. Suggest improvements for download retries, progress reporting, and safe model selection in the frontend store."

## Prompt: Upgrade theme and styling patterns
"Review the theme system and Tailwind usage. Help me make sure dark/light/high-contrast themes use CSS variables consistently and avoid hardcoded color values in `src/lib/components`."

## Prompt: Audit IPC and state flows
"Audit the IPC event names, Tauri command handlers, and Svelte stores. Confirm the code uses snake_case consistently and that frontend listeners match the backend event emissions."

## Prompt: Add newcomer documentation
"Help me write a short doc overview for the Bonsai Workspace architecture, including the Rust/Tauri backend, Svelte frontend, sidecar model flow, and how files are opened/edited in the app."
