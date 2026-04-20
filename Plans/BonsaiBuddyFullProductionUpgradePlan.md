Bonsai Buddy — Full Production Upgrade Plan
Context
The Bonsai Buddy assistant backend and frontend are ~98% structurally complete but have critical integration bugs preventing the basic chat from working reliably. This plan fixes them in order of severity, adds the dual-port API (workspace 11369 / buddy 11420), and lifts quality to production grade.

Domain Boundary
Two distinct domains share the SQLite pool and model orchestrator. This plan touches Buddy domain only unless explicitly noted.

Domain	Key Files	Scope of This Plan
Buddy	assistant_*.rs, buddy_api_server.rs, BonsaiAssistant.svelte, assistant.ts	ALL phases
Workspace Chat	commands.rs submit_chat, ChatPanel.svelte, chat.ts	Phase 5 only (context policy mirrors the proven pattern)
No workspace-chat session tables, UI, or commands are modified in phases 1–4 or 6–7.

Critical Files
File	Change
src-tauri/src/config.rs	Add BUDDY_API_PORT = 11420, buddy_api_port: u16 to AppConfig
src-tauri/src/buddy_api_server.rs	NEW — HTTP server on 11420
src-tauri/src/lib.rs	Start buddy server; add get_buddy_api_port command
src-tauri/src/assistant_manager.rs	Fix global event emit; inject default system prompt
src-tauri/src/assistant_commands.rs	Fix tool_call_id hardcode; fix backup stub; add auto_title_session
src-tauri/src/assistant_store.rs	Add delete_backup_entry, set_session_title; schema migration v7
src/lib/components/AssistantMessage.svelte	Markdown rendering + DOMPurify allowlist
src/lib/stores/assistant.ts	Error store; token-budget context truncation
src/lib/components/BonsaiAssistant.svelte	Error banner; auto-title wiring
src-tauri/src/api_server.rs	Add 502-retry branch + emit proxy-recovery-attempted
src-tauri/src/commands.rs	Emit permission-resolved after approval/denial
src/lib/components/TerminalPanel.svelte	3 new event subscriptions (bootstrap-progress, proxy-recovery, permission-resolved)
src/app.css	Global z-index CSS variable ladder
src/lib/components/CommandPalette.svelte	Add Ctrl+K · F1 hint to palette footer
src/lib/components/MonacoEditor.svelte	Persistent shortcut hint in editor header
src/App.svelte	Capture-phase keyboard listener
Phase 1 — Port Configuration
src-tauri/src/config.rs:

pub const DEFAULT_API_HOST: &str = "127.0.0.1";
pub const DEFAULT_API_PORT: u16  = 11369;   // Bonsai Workspace
pub const BUDDY_API_PORT:   u16  = 11420;   // Bonsai Buddy

pub struct AppConfig {
    pub api_host:       String,   // 11369 server bind host
    pub api_port:       u16,      // 11369 default
    pub buddy_api_port: u16,      // 11420 default — NEW
    // existing fields unchanged ...
}
Rollback: Config file is read/written with serde_json. If buddy_api_port key is absent from an existing config file, serde(default) falls back to BUDDY_API_PORT. No migration needed.

Phase 2 — Buddy API Server (port 11420)
src-tauri/src/buddy_api_server.rs (new, ~220 lines):

Security Policy
Mode	Condition	Auth
Loopback-only (default)	Bind 127.0.0.1	None required
LAN mode (opt-in)	buddy_api_host = "0.0.0.0" in config	Authorization: Bearer <BUDDY_API_KEY> required; key stored in SecretsStore
Default is loopback with no key. LAN mode never activates unless user explicitly changes the bind host in Settings.

Endpoints
POST /v1/chat/completions   → run_assistant_turn (stream=true/false)
GET  /v1/models             → [{"id":"bonsai-buddy","object":"model","owned_by":"bonsai"}]
GET  /health                → {"status":"ok","port":11420,"buddy":true}
Stream/non-stream contract
stream=false (complete response):

{
  "id": "buddy-<uuid>",
  "object": "chat.completion",
  "choices": [{"index":0,"message":{"role":"assistant","content":"..."},"finish_reason":"stop"}],
  "usage": {"prompt_tokens":0,"completion_tokens":0}
}
stream=true (SSE):

data: {"id":"buddy-<uuid>","choices":[{"delta":{"content":"tok"},"index":0}]}
...
data: [DONE]
Error envelope (both modes):

{"error":{"type":"buddy_error","message":"No model slot ready.","code":503}}
Port conflict handling
Try buddy_api_port, then buddy_api_port + 1 … +4. If all fail, log warning and emit buddy-api-unavailable event; app continues without the API server.

Rollback: If buddy_api_server::start() fails, AppState.buddy_api_port is set to 0 and get_buddy_api_port returns 0 — Settings UI shows "Unavailable".

Phase 3 — Fix Streaming for Embedded (MobileLayout) View
Root cause: accumulate_stream() and run_plain_turn emit to webview_window("assistant"). The MobileLayout Buddy tab lives in "main" and never receives these events.

Fix in assistant_manager.rs — replace targeted emits with global broadcasts:

// Before:
app.emit_to(tauri::EventTarget::webview_window("assistant"), "token-stream-assistant", tok)
// After:
app.emit("token-stream-assistant", tok)
Apply to all assistant events:

token-stream-assistant (accumulate_stream line 712, run_plain_turn line 814)
assistant-tool-start, assistant-tool-done, assistant-tool-error
assistant-confirm-required
tts-visemes, tts-started, tts-done, tts-error
The "assistant" window will still receive them (Tauri global emit reaches all windows).

Rollback: This is a backwards-compatible change. The event names are unchanged.

Phase 4 — Fix Tool Call ID Replay + Data Migration
Problem
assistant_commands.rs rebuilds history with "tool_call_id": "call_0" hardcoded. This breaks:

Multi-tool turns (tool 2 always gets ID "call_0" same as tool 1)
Session replay on models that validate ID referential integrity
Schema Migration (migration v7)
-- Add tool_call_id column to assistant_messages (NULL for non-tool rows)
ALTER TABLE assistant_messages ADD COLUMN tool_call_id TEXT;
Migration runs at startup in AssistantStore::new() after existing migrations. Existing rows get tool_call_id = NULL.

Read-time compatibility (backward compat for old rows)
In assistant_commands.rs history rebuild:

"tool" => {
    // Prefer stored tool_call_id; fall back to synthetic stable ID
    let call_id = m.tool_call_id
        .clone()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| format!("call_{}", i));  // stable per-index fallback
    history.push(json!({
        "role": "tool",
        "content": m.content,
        "tool_call_id": call_id,
    }));
}
Old sessions with tool_call_id = NULL get synthetic IDs call_0, call_1 … per position — not correct for multi-tool turns but stable and never worse than current behavior.

New sessions store the real tool_call_id from the model response in ToolCallOutcome.tool_call_id → persisted by store.append_message().

Fix delete_assistant_backup_entry
Add to AssistantStore:

pub async fn delete_backup_entry(&self, id: &str) -> Result<(), String> {
    sqlx::query!("DELETE FROM backup_registry WHERE id = ?", id)
        .execute(&self.pool).await.map_err(|e| e.to_string())?;
    Ok(())
}
Remove the stub comment and call state.assistant_store.delete_backup_entry(&id).await?.

Phase 5 — Context Window Management (Token-Budget Policy)
Strategy: Keep a fixed window of 30 messages, but pin critical messages so they are never dropped. This prevents losing early constraints or tool outcomes.

Pinned message categories (never truncated):

The most-recent system message (index 0 always)
Any role: "tool" message from the current turn
The latest user message
Implementation in assistant.ts sendAssistantMessage():

const CONTEXT_LIMIT = 30;
const msgs = $assistantMessages;

if (msgs.length > CONTEXT_LIMIT) {
  const pinned = msgs.filter(m =>
    m.role === 'tool' &&
    // only keep tool messages from the last 3 turns
    msgs.indexOf(m) >= msgs.length - 6
  );
  const recentWindow = msgs.slice(-CONTEXT_LIMIT);
  // Ensure pinned items are present (they likely already are within the window)
  const history = recentWindow;
  // Inject a context-gap notice as first user message after system
  history.splice(1, 0, {
    role: 'user',
    content: '[Note: earlier messages were trimmed for context. The conversation continues below.]',
    // ... minimal AssistantMessage shape
  });
}
Summary refresh (triggered every 20 turns): after run_assistant_turn succeeds on turn 20, 40, 60 …, call a background invoke('summarize_assistant_context', { sessionId }) that asks the model for a 3-sentence summary and stores it as a pinned system message at position [1]. Implement summarize_assistant_context command in Phase 7 cleanup (non-blocking for MVP).

This matches the pattern the workspace chat already uses for context slicing (CONTEXT_LIMIT constant in ChatPanel.svelte).

Phase 6 — Markdown Rendering + Safe Link Handling
src/lib/components/AssistantMessage.svelte:

<script lang="ts">
  import { marked } from 'marked';
  import DOMPurify from 'dompurify';
  
  // Strict DOMPurify allowlist
  const PURIFY_CONFIG = {
    ALLOWED_TAGS: ['p','br','strong','em','code','pre','ul','ol','li',
                   'blockquote','table','thead','tbody','tr','th','td','a','h1','h2','h3','h4'],
    ALLOWED_ATTR: ['href','class','target','rel'],
    FORCE_BODY: true,
  };
  
  // Force safe link behavior on all <a> tags
  DOMPurify.addHook('afterSanitizeAttributes', (node) => {
    if (node.tagName === 'A') {
      node.setAttribute('target', '_blank');
      node.setAttribute('rel', 'noopener noreferrer');
    }
  });
  
  export let message: AssistantMessage;
  $: html = message.role === 'assistant'
    ? DOMPurify.sanitize(marked.parse(message.content ?? '') as string, PURIFY_CONFIG)
    : null;
</script>
Code block copy button — injected via marked renderer override:

const renderer = new marked.Renderer();
renderer.code = (code, lang) =>
  `<div class="code-block"><button class="copy-btn" data-code="${encodeURIComponent(code)}">Copy</button><pre><code class="language-${lang ?? ''}">${code}</code></pre></div>`;
marked.use({ renderer });
Handle click on .copy-btn in the component with navigator.clipboard.writeText(decodeURIComponent(...)).

Phase 7 — Error Display + Session Auto-Title
src/lib/stores/assistant.ts:

export const assistantError = writable<string>('');

// In sendAssistantMessage catch block:
} catch (e) {
  const msg = e instanceof Error ? e.message : String(e);
  assistantError.set(msg);
  setTimeout(() => assistantError.set(''), 8000);
}
src/lib/components/BonsaiAssistant.svelte — error banner above input bar:

{#if $assistantError}
  <div class="error-banner" role="alert">
    ⚠ {$assistantError}
    <button on:click={() => assistantError.set('')}>✕</button>
  </div>
{/if}
Session auto-title: after first successful reply when currentSession.title === 'New conversation':

New Tauri command auto_title_session(sessionId, firstUserMsg, firstReply) in assistant_commands.rs
Builds a 2-message history [{role:user, content:firstUserMsg}, {role:assistant, content:firstReply}]
Sends to model with system prompt "Summarize this exchange as a 4–6 word session title. Return only the title, no punctuation."
Calls store.set_session_title(id, title).await
Emits assistant-session-titled event with {sessionId, title} so frontend can update without refetching
src/lib/stores/assistant.ts — listen for assistant-session-titled event and update currentSession store reactively.

Phase 8 — Proxy 502 Retry (Critical)
Context: api_server.rs::proxy_to_llama() already has a pre-proxy readiness gate (ensure_active_slot_url(), line 830) and a one-shot retry on connection error (Err), but passes 502 responses through without retry. A recycling sidecar can accept the TCP connection, return 502 once, then recover — and that 502 currently reaches the caller unretried.

Fix in src-tauri/src/api_server.rs — in proxy_to_llama() after the successful initial send, add a 502-specific retry branch:

// After initial response succeeds:
let response = match initial_response.status() {
    StatusCode::BAD_GATEWAY => {
        // Slot returned 502 — it may be recycling. Trigger recovery and retry once.
        if let Some(ref model_id) = model_hint {
            let _ = timeout(Duration::from_secs(45),
                s.orchestrator.load(model_id.clone())).await;
        }
        if let Some(recovered_url) = 
            wait_for_active_slot_url(&s, model_hint.as_deref(), 80, 
                Duration::from_millis(200)).await {
            send(&recovered_url, request_body.clone()).await
                .unwrap_or(initial_response)
        } else {
            initial_response
        }
    }
    _ => initial_response,
};
Emit proxy-recovery-attempted Tauri event before the retry so activity log can capture it.

Rollback: If retry also returns 502, the second response passes through — same behavior as before. No infinite loop: single retry only.

Phase 9 — Activity Observability (High)
Context: The activity collector in TerminalPanel.svelte is already always-on and subscribes to 19 event types. Key gaps identified:

bootstrap-progress is emitted by backend (bootstrap.rs line 547) but never subscribed to
Proxy recovery attempts (proxy-recovery-attempted, added in Phase 8) are unsubscribed
Orchestrator evictions and permission grant/deny have no events at all
Individual agent failures only surface via swarm-error if the swarm emits one
Fixes:

src/lib/components/TerminalPanel.svelte — add 3 new subscriptions in onMount():

// bootstrap substeps
const unlistenBootstrapProgress = await listen('bootstrap-progress', (ev) => {
  pushEntry({ level: 'debug', category: 'system', source: 'bootstrap-progress',
    summary: `Bootstrap: ${(ev.payload as any).step}`, details: ev.payload });
});

// proxy recovery
const unlistenProxyRecovery = await listen('proxy-recovery-attempted', (ev) => {
  pushEntry({ level: 'warn', category: 'system', source: 'proxy-recovery',
    summary: 'Proxy 502 — recovery retry in progress', details: ev.payload });
});

// permission resolution (approved/denied)
const unlistenPermission = await listen('permission-resolved', (ev) => {
  const p = ev.payload as any;
  pushEntry({ level: p.granted ? 'info' : 'warn', category: 'tool',
    source: 'permission-resolved',
    summary: `Permission ${p.granted ? 'granted' : 'denied'}: ${p.tool}`, details: p });
});
Add corresponding unlisten calls to the destroy cleanup block.

src-tauri/src/commands.rs — after permission prompt resolved, emit permission-resolved:

app.emit("permission-resolved", json!({ "tool": tool_name, "granted": approved }))
    .ok();
Verification: run a full session with swarm task + model recycle + bootstrap + terminal command + permission prompt. Check localStorage bonsai-terminal-activity-log-v1 — should have entries with categories: system, swarm, tool, terminal.

Phase 10 — Overlay Z-Index Ladder (Medium)
Context: 22 overlay components use ad-hoc inline z-index values with no global scale. Key collision risks:

AgentsPanel (200) can be hidden behind SettingsPanel (400) or CommandPalette (500)
FileTree context menu at 3000 is an outlier above SkillBuilder (1000) and SessionPanel (1600)
No CSS variables — changing one layer requires auditing all 22 files
Fix in src/app.css (or new src/lib/styles/z-index.css imported from app.css):

:root {
  --z-canvas:     10;   /* canvas nodes, toolbars */
  --z-inline:     20;   /* inline popovers, chat popups */
  --z-panel:      100;  /* slide-in side panels */
  --z-dropdown:   300;  /* dropdowns that must clear panels */
  --z-overlay:    500;  /* primary modals (CommandPalette, SettingsPanel) */
  --z-modal:      800;  /* blocking modals (SessionPanel, SkillBuilder) */
  --z-context:    1000; /* context menus (FileTree) */
  --z-toast:      2000; /* toast notifications */
  --z-critical:   9999; /* bootstrap screen */
}
Apply to key overlays (replacing hard-coded values):

Component	Old	New var
AgentsPanel.svelte	200	--z-overlay
AgentVisionPanel.svelte	160	--z-panel
SettingsPanel.svelte	400	--z-overlay
CommandPalette.svelte	500	--z-overlay
SkillBuilder.svelte	1000	--z-modal
SessionPanel.svelte	1600	--z-modal
FileTree.svelte context menu	3000	--z-context
Toast.svelte	2000	--z-toast
BootstrapScreen.svelte	9999	--z-critical
Modal mutual exclusion: In App.svelte, add a closeAllModals() helper and call it before opening any new primary modal — prevents two --z-overlay modals rendering simultaneously.

Verification: Open Sessions modal on desktop — verify it fully occludes lower controls. Open from editor focus — verify CommandPalette appears above editor. Check mobile layout.

Phase 11 — Command Palette Dual Activation + Hints (Medium)
Context: Both Ctrl+K and F1 already work (verified). CommandPalette.svelte uses the capture phase listener so Monaco cannot block it. Two remaining gaps:

Hint text only appears in the empty-editor state — disappears once a file is open
App.svelte registers its own palette listener in bubble phase (potential race)
Fix 1 — Persistent hint in palette footer (src/lib/components/CommandPalette.svelte): Add Ctrl+K / F1 to the existing palette footer alongside the navigation hints:

<div class="palette-footer">
  <span>↑↓ navigate</span>
  <span>⏎ execute</span>
  <span>Esc close</span>
  <span class="palette-hint">Ctrl+K · F1</span>  <!-- NEW -->
</div>
Fix 2 — Add hint to editor toolbar (src/lib/components/MonacoEditor.svelte): Show a subtle Ctrl+K badge in the editor header bar (visible even when a file is open), not just in the empty placeholder.

Fix 3 — Capture phase in App.svelte (src/App.svelte line 203):

// Before:
window.addEventListener('keydown', globalKey);
// After:
window.addEventListener('keydown', globalKey, true);  // capture phase
This ensures Ctrl+Shift+S and Ctrl+Shift+B are not consumed by Monaco in editor-focused state.

Rollback: Capture phase is safe — globalKey in App.svelte calls event.preventDefault() only on matched shortcuts, so other keystrokes are unaffected.

Regression / Negative Test Matrix
Scenario	Expected
llama-server offline at send time	Error banner: "No model slot ready." within 3s
Model returns HTTP 400 on tools request	Falls back to run_plain_turn; streaming continues normally
tool_call_id missing in old DB row	Synthetic call_N ID used; no crash; session still loads
Confirmation token expired (>60s)	cancel_tool_action no-ops cleanly; frontend card auto-removes
Partial stream interrupted (cancel)	isAssistantThinking set to false; no orphaned streaming state
Malformed tool JSON from model	parse_tool_calls malformed_count > 0; tool skipped; prose answer returned
Buddy API on 11420 port already in use	Try +1 … +4; if all fail, warn and continue; get_buddy_api_port returns 0
LAN mode enabled without API key set	Buddy server refuses to bind on 0.0.0.0; remains loopback-only
History longer than 30 messages	Context trimmed; gap notice injected; model receives valid history
Sidecar dies → next request hits dead slot → 502 returned	Proxy retries once after recovery; proxy-recovery-attempted event in activity log
Two modals triggered in rapid succession	Second modal fully occludes first; no lower-layer controls visible
Ctrl+K pressed while Monaco editor focused	Command palette opens (capture-phase listener fires first)
F1 pressed while terminal focused	Command palette opens
bootstrap-progress event fired during model download	Activity log shows progress entry with step detail
Permission prompt approved	permission-resolved event with granted: true in activity log
Acceptance Criteria (Go / No-Go)
MobileLayout Buddy tab receives streaming tokens and tool lifecycle events.
Multi-tool turns replay with correct tool_call_id across session reloads (verified with 2-tool turn).
Markdown renders safely: code blocks, lists, links open in new tab with noopener noreferrer.
Error banner appears within 3s for failed sends; auto-dismisses after 8s.
curl http://127.0.0.1:11369/health → workspace OK; curl http://127.0.0.1:11420/health → buddy OK.
POST http://127.0.0.1:11420/v1/chat/completions with stream=false returns valid JSON; with stream=true returns SSE stream.
Session title changes from "New conversation" after first exchange.
cargo check — 0 errors.
npm run build — ✓ no new warnings beyond pre-existing Monaco chunk size.
Recycle → inference smoke: kill sidecar process, send a chat message → response completes successfully (200 with generated output) within 60s.
Activity log coverage: single run produces log entries with categories: system (bootstrap + model), swarm (if swarm used), tool, terminal.
Sessions modal occlusion: Sessions panel fully occludes lower controls on both desktop and mobile layouts.
Command palette dual activation: Ctrl+K opens palette from editor focus; F1 opens palette from terminal focus.
Execution Order
#	Task	Phase	Est.
1	config.rs — add BUDDY_API_PORT, buddy_api_port	1	5 min
2	assistant_store.rs — migration v7, delete_backup_entry, set_session_title	4/7	15 min
3	assistant_manager.rs — global event emit	3	10 min
4	assistant_commands.rs — tool_call_id fix + backup stub + auto_title_session	4/7	30 min
5	buddy_api_server.rs — new file	2	45 min
6	lib.rs — mod + startup + commands	2	15 min
7	AssistantMessage.svelte — markdown + copy	6	20 min
8	assistant.ts — error store + context truncation	5/7	15 min
9	BonsaiAssistant.svelte — error banner + auto-title	7	15 min
10	api_server.rs — 502 retry + proxy-recovery-attempted emit	8	15 min
11	TerminalPanel.svelte — 3 new event subscriptions	9	10 min
12	commands.rs — emit permission-resolved	9	5 min
13	app.css — z-index CSS variable ladder; apply to 9 key components	10	20 min
14	CommandPalette.svelte + MonacoEditor.svelte — shortcut hints	11	10 min
15	App.svelte — capture phase listener	11	5 min
16	cargo check + npm run build	—	15 min
17	Regression matrix smoke run (all 13 acceptance criteria)	—	30 **min**