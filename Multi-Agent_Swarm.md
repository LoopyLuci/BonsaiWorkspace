Bonsai Workspace: Multi-Agent Swarm System
Context
The user wants a production-grade multi-agent system where multiple AI agents run simultaneously on the same task, improving speed and accuracy through parallel specialised work. Agent 1 is always the Leader/Manager that decomposes tasks, coordinates workers, and synthesises results. Users configure custom personas (name, system prompt, model preference, color, emoji) and assign them to agent slots. The system estimates RAM per agent before starting to prevent OOM. All agent messages appear in one unified chat thread with visual distinction per agent. A new dedicated Agents Settings Popup handles all configuration.

This must be fully backward-compatible: when only the Leader is configured (default), existing behavior is unchanged.

Architecture Overview
User prompt
   │
   ▼
submit_swarm_chat (Tauri command)
   │  resource check (RAM safety gate)
   │
   ▼
SwarmOrchestrator::run_swarm()          ← server-side, entirely in Rust
   │
   ├─ Leader inference (slot 0)
   │    └─ emits: agent-token-stream {agent_id, slot:0, token}
   │    └─ parses <swarm_plan> JSON tag
   │
   ├─ Worker inference × N (tokio::spawn per worker, concurrent)
   │    └─ emits: agent-token-stream {agent_id, slot:N, token}
   │    └─ emits: swarm-agent-complete when done
   │
   └─ Leader synthesis (with all worker results in context)
        └─ emits: token-stream (backward compat) + agent-token-stream
        └─ emits: swarm-complete
Single-agent path: when only Leader is enabled, run_swarm skips decomposition and calls the ReAct loop directly. No protocol overhead.

New SQLite Tables
Added via migration in AgentStore::new(), which shares the existing SqlitePool from wal.rs (bonsai.db).

CREATE TABLE IF NOT EXISTS personas (
    id            TEXT    PRIMARY KEY,
    name          TEXT    NOT NULL,
    system_prompt TEXT    NOT NULL,
    model_id      TEXT,
    color         TEXT    NOT NULL DEFAULT '#4a9eff',
    icon_emoji    TEXT    NOT NULL DEFAULT '🤖',
    created_at    INTEGER NOT NULL,
    updated_at    INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS agent_configs (
    id           TEXT    PRIMARY KEY,
    slot_index   INTEGER NOT NULL UNIQUE,
    label        TEXT    NOT NULL,
    persona_id   TEXT    REFERENCES personas(id) ON DELETE SET NULL,
    model_id     TEXT,
    color        TEXT    NOT NULL DEFAULT '#4a9eff',
    icon_emoji   TEXT    NOT NULL DEFAULT '🤖',
    enabled      INTEGER NOT NULL DEFAULT 1,
    max_tokens   INTEGER NOT NULL DEFAULT 4096,
    created_at   INTEGER NOT NULL,
    updated_at   INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_agent_configs_slot ON agent_configs(slot_index);

-- ALTER TABLE guard (run in try/catch; SQLite errors if column already exists):
-- ALTER TABLE session_messages ADD COLUMN agent_id TEXT;
-- CREATE INDEX IF NOT EXISTS idx_msgs_agent ON session_messages(agent_id);

CREATE TABLE IF NOT EXISTS swarm_runs (
    id           TEXT    PRIMARY KEY,
    session_id   TEXT    REFERENCES chat_sessions(id) ON DELETE CASCADE,
    user_prompt  TEXT    NOT NULL,
    leader_plan  TEXT,
    status       TEXT    NOT NULL DEFAULT 'running',
    started_at   INTEGER NOT NULL,
    completed_at INTEGER
);

CREATE TABLE IF NOT EXISTS swarm_agent_results (
    id           TEXT    PRIMARY KEY,
    swarm_run_id TEXT    NOT NULL REFERENCES swarm_runs(id) ON DELETE CASCADE,
    agent_id     TEXT    NOT NULL,
    agent_slot   INTEGER NOT NULL,
    subtask      TEXT    NOT NULL,
    result       TEXT,
    stats_json   TEXT,
    started_at   INTEGER NOT NULL,
    completed_at INTEGER
);
First-run seeding (when agent_configs is empty): insert one row for the Leader (slot_index=0, label="Leader", color="#f5a623", icon_emoji="👑", enabled=1).

New Rust Files
src-tauri/src/agent_store.rs (NEW)
pub struct Persona {
    pub id: String, pub name: String, pub system_prompt: String,
    pub model_id: Option<String>, pub color: String, pub icon_emoji: String,
    pub created_at: i64, pub updated_at: i64,
}

pub struct AgentConfig {
    pub id: String, pub slot_index: i64, pub label: String,
    pub persona_id: Option<String>, pub model_id: Option<String>,
    pub color: String, pub icon_emoji: String,
    pub enabled: bool, pub max_tokens: i64,
    pub created_at: i64, pub updated_at: i64,
}

pub struct ResolvedAgent {
    pub config: AgentConfig,
    pub persona: Option<Persona>,
    pub system_prompt: String,           // persona.system_prompt or built-in default
    pub effective_model_id: Option<String>, // config.model_id ?? persona.model_id ?? None
    pub ram_required_mb: u64,
}

pub struct AgentStore { pool: SqlitePool }
impl AgentStore {
    pub async fn new(pool: SqlitePool) -> Result<Self>
    pub async fn list_agents(&self) -> Result<Vec<AgentConfig>>
    pub async fn resolve_agents(&self, registry: &ModelRegistry) -> Result<Vec<ResolvedAgent>>
    pub async fn upsert_agent(&self, config: AgentConfig) -> Result<AgentConfig>
    pub async fn delete_agent(&self, id: &str) -> Result<()>
    pub async fn list_personas(&self) -> Result<Vec<Persona>>
    pub async fn upsert_persona(&self, persona: Persona) -> Result<Persona>
    pub async fn delete_persona(&self, id: &str) -> Result<()>
    pub async fn save_swarm_run(&self, run: &SwarmRun) -> Result<()>
    pub async fn save_agent_result(&self, result: &SwarmAgentResult) -> Result<()>
}
src-tauri/src/swarm_orchestrator.rs (NEW)
Key types:

pub struct SwarmRequest {
    pub run_id: String,
    pub session_id: Option<String>,
    pub user_prompt: String,
    pub workspace_path: Option<String>,
    pub enabled_tools: Option<Vec<String>>,
    pub agents: Vec<ResolvedAgent>,       // index 0 = leader
    pub cancel_flags: Vec<Arc<AtomicBool>>, // per slot
    pub global_cancel: Arc<AtomicBool>,
    pub resp_tx: oneshot::Sender<Result<SwarmResult, String>>,
}

pub struct SwarmResult {
    pub final_response: String,
    pub leader_plan: Option<serde_json::Value>,
    pub agent_results: Vec<AgentOutput>,
    pub stats: InferStats,
}

pub struct AgentOutput {
    pub agent_id: String, pub slot_index: i64,
    pub subtask: String, pub result: String, pub stats: InferStats,
}

// Leader emits this inside <swarm_plan>...</swarm_plan>:
#[derive(Deserialize)] pub struct LeaderPlan {
    pub subtasks: Vec<SubtaskSpec>,
}
#[derive(Deserialize, Serialize, Clone)] pub struct SubtaskSpec {
    pub worker_slot: usize,   // 0 = leader handles directly, 1..N = workers
    pub task: String,
    pub context: String,
}

pub struct SwarmOrchestrator { cmd_tx: mpsc::UnboundedSender<SwarmCommand> }
impl SwarmOrchestrator {
    pub fn new() -> Self
    pub fn submit(&self, req: SwarmRequest) -> Result<(), String>
}
run_swarm() logic (inside orchestrator event loop):

Build leader system prompt = tools::system_prompt() + swarm coordination extension (lists workers by persona name)
Run leader inference → parse <swarm_plan> tag
If no plan (single agent or simple query): run ReAct loop inline, return
For each SubtaskSpec with worker_slot > 0: tokio::spawn(run_worker(...))
Emit swarm-plan-ready event
futures::future::join_all(worker_handles).await
Build synthesis context with all worker results
Run leader inference again → synthesis
Emit swarm-complete, return SwarmResult
Leader system prompt extension (appended after base system_prompt):

## Swarm coordination

You are the Leader in a multi-agent swarm. Available workers:
{for each enabled worker: "  Worker {slot} ({persona_name}): {persona_description_first_line}"}

When the request benefits from parallel work, output a plan FIRST:
<swarm_plan>
{"subtasks":[{"worker_slot":1,"task":"...","context":"..."},...]}
</swarm_plan>
Then optionally add a brief note. If no decomposition needed, skip the tag and reply directly.
Worker system prompt = persona's system_prompt + their tool list (same tools as leader). Workers are full ReAct agents, not just "answer-only" — they can use tools.

Modified Rust Files
src-tauri/src/lib.rs
Add to AppState:

pub agent_store: Arc<agent_store::AgentStore>,
pub swarm_orchestrator: Arc<swarm_orchestrator::SwarmOrchestrator>,
pub swarm_cancels: Arc<StdMutex<HashMap<String, Vec<Arc<AtomicBool>>>>>,
pub swarm_global_cancel: Arc<AtomicBool>,
Add to setup():

let agent_store = Arc::new(
    tauri::async_runtime::block_on(agent_store::AgentStore::new(wal.pool()))
        .expect("Agent store init failed"),
);
let swarm_orchestrator = Arc::new(swarm_orchestrator::SwarmOrchestrator::new());
Add new commands to invoke_handler!.

src-tauri/src/commands.rs
New commands to add (all pub async fn, #[tauri::command]):

// Persona CRUD
list_personas(state) -> Result<Vec<Persona>, String>
upsert_persona(state, persona: Persona) -> Result<Persona, String>
delete_persona(state, id: String) -> Result<(), String>

// Agent config CRUD
list_agent_configs(state) -> Result<Vec<ResolvedAgent>, String>
upsert_agent_config(state, config: AgentConfig) -> Result<AgentConfig, String>
delete_agent_config(state, id: String) -> Result<(), String>

// Resource estimation
estimate_swarm_resources(state, agents: Vec<AgentConfig>) -> Result<SwarmResourceEstimate, String>
// SwarmResourceEstimate { total_ram_required_mb, free_ram_mb, fits, per_agent: Vec<AgentResourceCost> }

// Swarm execution
submit_swarm_chat(app_handle, state, messages, workspace_path, enabled_tools)
    -> Result<SwarmChatResponse, String>
// SwarmChatResponse { run_id, final_content, leader_plan, agent_results, stats, action_handled, tools_used }

// Cancellation
cancel_agent(state, run_id: String, slot: usize) -> Result<(), String>
cancel_swarm(state, run_id: String) -> Result<(), String>

// Status
get_swarm_run_status(state, run_id: String) -> Result<SwarmRunStatus, String>
estimate_swarm_resources logic:

// unique model RAM: each distinct model_id counted once (shared slot reuse)
// + 256 MB KV-cache overhead per concurrent agent
// Safety gate: fits = total <= free_ram * 0.85
submit_swarm_chat logic:

agent_store.resolve_agents(&orchestrator.registry) → Vec<ResolvedAgent>
Filter to enabled agents
If 1 agent: call existing submit_chat logic directly
If >1: estimate_swarm_resources → if !fits return Err
Create SwarmRequest, submit to swarm_orchestrator
Await resp_rx
Persist final message to session with agent_id = leader.config.id
Resource Estimation Algorithm
unique_model_ram = sum of ram_required_mb for each DISTINCT model_id among enabled agents
kv_overhead = 256 MB × number_of_enabled_agents
total_required = unique_model_ram + kv_overhead
fits = total_required ≤ free_ram_mb × 0.85
Model ram_required_mb comes from the existing ModelInfo in ModelOrchestrator's registry (already tracked). sysinfo already imported and used in model_orchestrator.rs.

New Frontend Files
src/lib/stores/agents.ts (NEW)
export interface Persona { id, name, system_prompt, model_id, color, icon_emoji }
export interface AgentConfig { id, slot_index, label, persona_id, model_id, color, icon_emoji, enabled, max_tokens }
export interface ResolvedAgent { config: AgentConfig, persona: Persona|null, system_prompt, effective_model_id, ram_required_mb }
export interface SwarmResourceEstimate { total_ram_required_mb, free_ram_mb, fits, per_agent: AgentResourceCost[] }

export const agentConfigs     = writable<ResolvedAgent[]>([]);
export const personas         = writable<Persona[]>([]);
export const swarmEnabled     = writable<boolean>(false);   // true when ≥2 agents enabled
export const resourceEstimate = writable<SwarmResourceEstimate|null>(null);
export const activeSwarmRunId = writable<string|null>(null);
export const agentStreams      = writable<Map<string,string>>(new Map()); // agent_id → accumulated tokens

export async function loadAgentConfigs() { /* invoke list_agent_configs */ }
export async function loadPersonas()     { /* invoke list_personas */ }
export async function refreshResourceEstimate() { /* invoke estimate_swarm_resources */ }
src/lib/components/AgentsPanel.svelte (NEW)
Full-featured modal popup (same visual style as SettingsPanel: fixed overlay, centered card, header + close button).

Three tabs:

Tab: Agents

Sorted list of agentConfigs by slot_index
Each row: colored circle (swatch), emoji, label input, persona <select>, model <select>, enabled toggle, delete button (disabled for slot 0 / leader)
"Add Worker" button → calls upsert_agent_config with next slot_index
Resource budget bar: total_ram_required_mb / free_ram_mb as fill bar. Color: green ≤70%, amber ≤85%, red >85%
Warning banner when !fits: "Not enough RAM — disable an agent or choose a smaller model"
Tab: Personas

Grid of persona cards (emoji, name, color dot, first 80 chars of system_prompt)
"New Persona" form: name, emoji picker (text input), color <input type=color>, model <select>, system_prompt <textarea> (min 200px height)
Edit card expands inline; Save/Cancel
Delete with confirmation
Tab: About

Plain-language explanation of Leader/Worker model
Dispatches 'close' event. Mounted in App.svelte like SettingsPanel.

Modified Frontend Files
src/lib/stores/chat.ts
Extend ChatMessage:

export interface ChatMessage {
  // ...existing fields...
  agent_id?:    string;
  agent_label?: string;
  agent_color?: string;
  agent_slot?:  number;   // 0 = leader
}
src/lib/components/ChatPanel.svelte
Import: import { agentConfigs, swarmEnabled, agentStreams, activeSwarmRunId } from '$lib/stores/agents'

Event listeners (in onMount):

listen<{agent_id:string; slot:number; token:string}>('agent-token-stream', e => {
  agentStreams.update(m => {
    const next = new Map(m);
    next.set(e.payload.agent_id, (next.get(e.payload.agent_id) ?? '') + e.payload.token);
    return next;
  });
});
listen('swarm-agent-complete', e => { /* finalize that agent's message row */ });
listen('swarm-plan-ready', e => { /* show plan preview in chat */ });
listen('swarm-complete', e => { activeSwarmRunId.set(null); });
Message render — add agent badge above message bubble when msg.agent_slot !== undefined:
{#if msg.agent_slot !== undefined}
  <div class="agent-badge"
       class:is-leader={msg.agent_slot === 0}
       style:--agent-color={msg.agent_color ?? '#4a9eff'}>
    <span class="agent-emoji">{agentEmoji(msg)}</span>
    <span class="agent-label">{msg.agent_label}</span>
  </div>
{/if}
Leader badge: gold border + crown emoji. Workers: color tint background.

Send routing:
async function sendMessage() {
  if ($swarmEnabled) {
    result = await invoke('submit_swarm_chat', {...});
    // result.final_content added as leader message
  } else {
    result = await invoke('submit_chat', {...}); // unchanged
  }
}
Live in-progress rows: During a swarm run, render one pending .msg-row per active agent showing their $agentStreams buffer, with a pulsing dot animation. Finalized when swarm-agent-complete arrives.

Toolbar: Add "⚡ Agents" button that dispatches openAgents to App.svelte.

src/App.svelte
import AgentsPanel from '$lib/components/AgentsPanel.svelte';
let showAgents = false;
// In toolbar: <button on:click={() => showAgents = true}>⚡ Agents</button>
// Near other modal mounts:
{#if showAgents}
  <AgentsPanel on:close={() => showAgents = false} />
{/if}
Also call loadAgentConfigs() and loadPersonas() in onMount (alongside existing store loads).

Tauri Events Reference
Event	Payload	When
agent-token-stream	{ agent_id, slot, token }	Each token from any agent
agent-thinking-start	{ agent_id, slot, label }	Agent begins inference
agent-thinking-end	{ agent_id, slot, stats }	Agent inference done
swarm-plan-ready	{ run_id, leader_plan }	Leader finished decomposition
swarm-agent-complete	{ run_id, agent_id, slot, result, stats }	Worker done
swarm-complete	{ run_id, final_content, stats }	All done
swarm-error	{ run_id, agent_id?, error }	Failure
token-stream	string	Leader synthesis (backward compat)
token-speed	number	Leader synthesis (backward compat)
Implementation Order
Step	File	Action
1	src-tauri/src/agent_store.rs	Create — CRUD for personas/agents/swarm_runs
2	src-tauri/src/wal.rs	Add migration: new tables + ALTER TABLE session_messages
3	src-tauri/src/lib.rs	Add agent_store, swarm_orchestrator, swarm_cancels, swarm_global_cancel to AppState; init block
4	src-tauri/src/swarm_orchestrator.rs	Create — leader-worker coordination loop
5	src-tauri/src/commands.rs	Add all new commands (persona CRUD, agent CRUD, estimate_swarm_resources, submit_swarm_chat, cancel_*, get_swarm_run_status)
6	src-tauri/src/lib.rs	Register new commands in invoke_handler!
7	cargo check	Validate Rust compilation
8	src/lib/stores/agents.ts	Create frontend store
9	src/lib/stores/chat.ts	Extend ChatMessage with agent fields
10	src/lib/components/AgentsPanel.svelte	Create 3-tab settings popup
11	src/lib/components/ChatPanel.svelte	Wire new events, agent badges, send routing
12	src/App.svelte	Mount AgentsPanel, toolbar button, onMount loads
13	npm run build	Validate frontend compilation
Verification
cargo check with no errors after step 7
npm run build with no errors after step 13
Single-agent behavior unchanged: open app, send a message, verify response works
Add a Worker agent in Agents panel, send a message, verify Leader badge appears, worker badge appears, swarm-plan shows in chat
Resource gate: configure agents exceeding RAM, verify submit_swarm_chat returns error and chat shows warning
Persona create/edit/delete: verify round-trips through SQLite
Agent color: set custom color, verify message bubble tint matches
Cancel: during a swarm run, verify cancel_swarm stops all agents
CSS additions (AgentsPanel + ChatPanel)
/* Agent badge in chat */
.agent-badge { display:flex; align-items:center; gap:6px; font-size:11px; margin-bottom:4px; }
.agent-badge.is-leader { color: var(--accent-hl); font-weight:700; }
.agent-emoji { font-size:14px; }
.agent-label { color: var(--text-dim); }

/* Message bubble tint for agents */
.msg-row.agent-msg .msg-bubble {
  background: color-mix(in srgb, var(--agent-color) 12%, var(--bg2));
  border-color: color-mix(in srgb, var(--agent-color) 40%, var(--border));
}
.msg-row.agent-msg.is-leader .msg-bubble { border-color: var(--accent); }

/* Resource budget bar */
.resource-bar { height:6px; border-radius:3px; background:var(--bg); overflow:hidden; }
.resource-bar-fill { height:100%; border-radius:3px; transition:width .3s; }
.resource-bar-fill.safe   { background: var(--green); }
.resource-bar-fill.warn   { background: var(--amber); }
.resource-bar-fill.danger { background: var(--red); }