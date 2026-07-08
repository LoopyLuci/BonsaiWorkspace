// HarnessStore — persistent configuration for the OmniHarness AI panel.
//
// Manages three things the user controls from the panel's settings:
//   1. Providers    — API providers (with keys in SecretStorage) + local (Ollama).
//   2. Custom agents — named presets: system prompt, model, tools, temperature.
//   3. Active state  — currently selected model + agent, per workspace.
//
// API keys live in VS Code SecretStorage (encrypted). Because the orchestrator
// reads keys from its environment / .env at startup, the store can also write a
// `.env` file into the OmniHarness root so "add a key here" actually takes effect
// after a server restart.

import * as vscode from 'vscode';
import * as fs from 'fs';
import * as path from 'path';
import { McpServerConfig } from './mcp/McpTypes';
import { HarnessMessage } from './OmniHarnessClient';

export type ProviderKind = 'api' | 'local';

export interface ProviderDef {
    id: string;            // e.g. "anthropic"
    label: string;         // e.g. "Anthropic (Claude)"
    kind: ProviderKind;
    envVar: string;        // e.g. "ANTHROPIC_API_KEY" or "OLLAMA_ENABLED"
    keyUrl?: string;       // where to obtain a key
    baseUrlEnv?: string;   // env var for a custom base URL (local providers)
    defaultBaseUrl?: string;
    example?: string;      // example model id
    /** Extra candidate base URLs to probe for this provider (local providers only). */
    probeUrls?: string[];
}

/** A locally-detected model, extracted from a real probe response — no invented fields. */
export interface LocalDetectedModel {
    id: string;             // model id/name as reported by the backend
    sizeBytes?: number;      // Ollama: models[].size
    family?: string;         // Ollama: models[].details.family
    parameterSize?: string;  // Ollama: models[].details.parameter_size
    quantization?: string;   // Ollama: models[].details.quantization_level
}

/** Result of probing one local backend (Ollama or an OpenAI-compatible server). */
export interface LocalProviderProbeResult {
    providerId: string;      // 'ollama' | 'local'
    detected: boolean;
    baseUrl?: string;        // the URL that actually responded
    models: LocalDetectedModel[];
    backendLabel?: string;   // which known backend matched by port (e.g. "LM Studio")
}

/** Common local-OpenAI-compatible backends we auto-probe by their well-known default port. */
const LOCAL_OPENAI_CANDIDATES: Array<{ url: string; label: string }> = [
    { url: 'http://localhost:8081/v1', label: 'llama.cpp (custom/8081)' },
    { url: 'http://localhost:1234/v1', label: 'LM Studio' },
    { url: 'http://localhost:8080/v1', label: 'llama.cpp server' },
];

async function fetchWithTimeout(url: string, timeoutMs: number): Promise<Response | undefined> {
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), timeoutMs);
    try {
        const resp = await fetch(url, { signal: controller.signal });
        if (!resp.ok) { return undefined; }
        return resp;
    } catch {
        return undefined;
    } finally {
        clearTimeout(timer);
    }
}

interface OllamaTagsResponse {
    models?: Array<{
        name: string;
        size?: number;
        details?: { family?: string; parameter_size?: string; quantization_level?: string };
    }>;
}

interface OpenAiModelsResponse {
    data?: Array<{ id: string; [key: string]: unknown }>;
}

/** Probe Ollama's real /api/tags endpoint. Tries `preferredBaseUrl` (a user override) first, then the default. */
async function probeOllama(preferredBaseUrl: string | undefined, timeoutMs: number): Promise<LocalProviderProbeResult> {
    const candidates = [preferredBaseUrl, 'http://localhost:11434'].filter(Boolean) as string[];
    for (const base of candidates) {
        const trimmed = base.replace(/\/+$/, '');
        const resp = await fetchWithTimeout(`${trimmed}/api/tags`, timeoutMs);
        if (!resp) { continue; }
        try {
            const json = (await resp.json()) as OllamaTagsResponse;
            const models: LocalDetectedModel[] = (json.models ?? []).map((m) => ({
                id: m.name,
                sizeBytes: m.size,
                family: m.details?.family,
                parameterSize: m.details?.parameter_size,
                quantization: m.details?.quantization_level,
            }));
            return { providerId: 'ollama', detected: true, baseUrl: trimmed, models, backendLabel: 'Ollama' };
        } catch {
            // Non-JSON or unexpected shape — treat as not detected.
        }
    }
    return { providerId: 'ollama', detected: false, models: [] };
}

/**
 * Probe local OpenAI-compatible servers. Tries a user-set base URL first (if any),
 * then a short list of well-known default ports in parallel, first success wins.
 */
async function probeLocalOpenAi(preferredBaseUrl: string | undefined, timeoutMs: number): Promise<LocalProviderProbeResult> {
    const tryOne = async (base: string, label: string): Promise<LocalProviderProbeResult | undefined> => {
        const trimmed = base.replace(/\/+$/, '');
        const resp = await fetchWithTimeout(`${trimmed}/models`, timeoutMs);
        if (!resp) { return undefined; }
        try {
            const json = (await resp.json()) as OpenAiModelsResponse;
            const models: LocalDetectedModel[] = (json.data ?? []).map((m) => ({ id: m.id }));
            return { providerId: 'local', detected: true, baseUrl: trimmed, models, backendLabel: label };
        } catch {
            return undefined;
        }
    };

    if (preferredBaseUrl) {
        const preferred = await tryOne(preferredBaseUrl, 'Custom');
        if (preferred) { return preferred; }
    }

    const results = await Promise.all(
        LOCAL_OPENAI_CANDIDATES.map((c) => tryOne(c.url, c.label)),
    );
    return results.find((r) => r?.detected) ?? { providerId: 'local', detected: false, models: [] };
}

export interface AgentPreset {
    id: string;
    name: string;
    description: string;
    systemPrompt: string;
    model: string;                 // model id ("provider/model" or bare)
    temperature: number;
    maxTokens: number;
    tools: string[] | ['*'];       // allowed tool names, or ['*'] for all
    autoApprove: boolean;          // skip approval for this agent's tool calls
    builtin?: boolean;
}

/** One rendered item in a session's UI transcript (richer than the raw model messages). */
export interface ChatTurn {
    id: string;
    role: 'user' | 'assistant' | 'tool' | 'error' | 'compaction';
    text?: string;
    toolName?: string;
    toolArgs?: Record<string, unknown>;
    toolOk?: boolean;
    toolSummary?: string;
    toolContent?: string;
    diff?: { relPath: string; before: string; after: string } | null;
    checkpointId?: string;      // present + still in the checkpoint store => undoable
    ts: number;
}

export interface ChatSession {
    id: string;
    title: string;
    model: string;
    agentId: string;
    createdAt: number;
    updatedAt: number;
    messages: HarnessMessage[]; // model-facing context
    turns: ChatTurn[];          // UI-facing transcript
    // Rolling auto-compaction state: once the live context exceeds the token
    // budget, everything before `contextSummaryUpToIndex` (an index into
    // `messages`) is replaced by `contextSummary` when building model context.
    // The full, uncompacted `messages`/`turns` are never mutated, so history,
    // undo, and export are unaffected by compaction.
    contextSummary?: string;
    contextSummaryUpToIndex?: number;
}

/** Captured pre-edit file state so a mutating tool call can be undone. */
export interface Checkpoint {
    id: string;          // == the tool call id it belongs to
    sessionId: string;
    path: string;         // absolute fs path
    before: string | null; // null = the file did not exist before the edit
    ts: number;
}

export interface ServerProfile {
    id: string;
    name: string;
    url: string;
}

export const KNOWN_PROVIDERS: ProviderDef[] = [
    { id: 'anthropic',  label: 'Anthropic (Claude)',  kind: 'api', envVar: 'ANTHROPIC_API_KEY',  keyUrl: 'https://console.anthropic.com/settings/keys', example: 'anthropic/claude-sonnet-4-6' },
    { id: 'openai',     label: 'OpenAI (GPT)',        kind: 'api', envVar: 'OPENAI_API_KEY',     keyUrl: 'https://platform.openai.com/api-keys',       example: 'gpt-4o' },
    { id: 'gemini',     label: 'Google (Gemini)',     kind: 'api', envVar: 'GOOGLE_API_KEY',     keyUrl: 'https://aistudio.google.com/apikey',         example: 'gemini/gemini-2.0-flash' },
    { id: 'groq',       label: 'Groq',                kind: 'api', envVar: 'GROQ_API_KEY',       keyUrl: 'https://console.groq.com/keys',              example: 'groq/llama-3.3-70b-versatile' },
    { id: 'mistral',    label: 'Mistral AI',          kind: 'api', envVar: 'MISTRAL_API_KEY',    keyUrl: 'https://console.mistral.ai/api-keys',        example: 'mistral/mistral-large-latest' },
    { id: 'cohere',     label: 'Cohere',              kind: 'api', envVar: 'COHERE_API_KEY',     keyUrl: 'https://dashboard.cohere.com/api-keys',      example: 'cohere/command-r-plus' },
    { id: 'openrouter', label: 'OpenRouter',          kind: 'api', envVar: 'OPENROUTER_API_KEY', keyUrl: 'https://openrouter.ai/keys',                 example: 'openrouter/openai/gpt-4o' },
    { id: 'together',   label: 'Together AI',         kind: 'api', envVar: 'TOGETHER_API_KEY',   keyUrl: 'https://api.together.xyz/settings/api-keys',  example: 'together/meta-llama/Meta-Llama-3.1-70B-Instruct-Turbo' },
    { id: 'fireworks',  label: 'Fireworks AI',        kind: 'api', envVar: 'FIREWORKS_API_KEY',  keyUrl: 'https://fireworks.ai/account/api-keys',      example: 'fireworks/accounts/fireworks/models/llama-v3p1-70b-instruct' },
    { id: 'ollama',     label: 'Ollama (local)',      kind: 'local', envVar: 'OLLAMA_ENABLED',   baseUrlEnv: 'OLLAMA_BASE_URL', defaultBaseUrl: 'http://localhost:11434', example: 'ollama/llama3.2' },
    { id: 'local',      label: 'Local OpenAI-compatible (llama.cpp / LM Studio / Jan)', kind: 'local', envVar: 'LOCAL_OPENAI_ENABLED', baseUrlEnv: 'LOCAL_OPENAI_BASE_URL', defaultBaseUrl: 'http://localhost:8081/v1', example: 'local/gemma-4-31B-it-UD-Q2_K_XL' },
];

const AGENTS_KEY = 'omniharness.agents';
const ACTIVE_MODEL_KEY = 'omniharness.activeModel';
const ACTIVE_AGENT_KEY = 'omniharness.activeAgent';
const MCP_KEY = 'omniharness.mcpServers';
const SESSIONS_KEY = 'omniharness.sessions';           // workspaceState — per-project history
const ACTIVE_SESSION_KEY = 'omniharness.activeSession'; // workspaceState
const CHECKPOINTS_KEY = 'omniharness.checkpoints';      // workspaceState — undo data
const SERVER_PROFILES_KEY = 'omniharness.serverProfiles'; // globalState
const ACTIVE_PROFILE_KEY = 'omniharness.activeServerProfile'; // globalState
const FAVORITE_MODELS_KEY = 'omniharness.favoriteModels'; // globalState
const MAX_SESSIONS = 200;
const MAX_CHECKPOINTS = 500;

/** Example MCP servers users can enable (all disabled until configured). */
function defaultMcpServers(): McpServerConfig[] {
    return [
        {
            id: 'filesystem',
            name: 'Filesystem',
            transport: 'stdio',
            enabled: false,
            command: 'npx',
            args: ['-y', '@modelcontextprotocol/server-filesystem', '.'],
        },
        {
            id: 'git',
            name: 'Git',
            transport: 'stdio',
            enabled: false,
            command: 'npx',
            args: ['-y', '@modelcontextprotocol/server-git'],
        },
        {
            id: 'omniharness',
            name: 'OmniHarness (memory + models)',
            transport: 'stdio',
            enabled: false,
            command: 'python',
            args: ['-m', 'omniharness.mcp_server'],
        },
    ];
}
const SECRET_PREFIX = 'omniharness.key.';

function builtinAgents(): AgentPreset[] {
    return [
        {
            id: 'coder',
            name: 'Coder',
            description: 'Full-access coding agent — reads, edits, runs, and searches your project.',
            systemPrompt:
                'You are OmniHarness Coder, an expert software engineer working inside the user\'s VS Code workspace. ' +
                'Make focused, correct changes. Read files before editing them. Prefer small, verifiable edits. ' +
                'Explain what you changed and why. Match the style of the surrounding code.',
            model: '',
            temperature: 0.2,
            maxTokens: 8192,
            tools: ['*'],
            autoApprove: false,
            builtin: true,
        },
        {
            id: 'ask',
            name: 'Ask',
            description: 'Read-only assistant — answers questions about the codebase without changing anything.',
            systemPrompt:
                'You are OmniHarness Ask, a helpful assistant answering questions about the user\'s codebase. ' +
                'You may read files, list directories, and search, but you must not modify anything. ' +
                'Be concise and cite file paths and line numbers.',
            model: '',
            temperature: 0.3,
            maxTokens: 4096,
            tools: ['read_file', 'list_dir', 'search', 'get_diagnostics', 'get_selection', 'open_file'],
            autoApprove: true,
            builtin: true,
        },
        {
            id: 'architect',
            name: 'Architect',
            description: 'Planning agent — explores the codebase and proposes an implementation plan.',
            systemPrompt:
                'You are OmniHarness Architect. Explore the codebase to understand it, then produce a clear, ' +
                'step-by-step implementation plan with file paths and concrete changes. Do not write files unless asked; ' +
                'focus on a precise, actionable design.',
            model: '',
            temperature: 0.4,
            maxTokens: 8192,
            tools: ['read_file', 'list_dir', 'search', 'get_diagnostics'],
            autoApprove: true,
            builtin: true,
        },
    ];
}

export class HarnessStore {
    constructor(private readonly ctx: vscode.ExtensionContext) {}

    /** Last real probe results, kept for synchronous reads (e.g. writeEnvFile) between refreshes. */
    private localProbeCache: LocalProviderProbeResult[] = [];

    // ── Providers ────────────────────────────────────────────────────────────

    listProviders(): ProviderDef[] {
        return KNOWN_PROVIDERS;
    }

    private static localOverrideKey(id: string): string {
        return `omniharness.localBaseUrlOverride.${id}`;
    }

    /** A user-typed "advanced/custom" base URL for a local provider, if they've set one. */
    getLocalBaseUrlOverride(id: string): string | undefined {
        return this.ctx.globalState.get<string>(HarnessStore.localOverrideKey(id)) || undefined;
    }

    async setLocalBaseUrlOverride(id: string, url: string): Promise<void> {
        await this.ctx.globalState.update(HarnessStore.localOverrideKey(id), url.trim() || undefined);
    }

    /**
     * Real network auto-detection for local model runtimes: HTTP-probes Ollama's
     * `/api/tags` and a short list of common OpenAI-compatible ports (LM Studio,
     * llama.cpp) with a short timeout, in parallel. A user-set override URL (if
     * any) is tried first and preferred over the guessed defaults. No state is
     * faked — a backend only reports `detected: true` if it actually answered.
     */
    async probeLocalProviders(timeoutMs = 800): Promise<LocalProviderProbeResult[]> {
        const [ollama, local] = await Promise.all([
            probeOllama(this.getLocalBaseUrlOverride('ollama'), timeoutMs),
            probeLocalOpenAi(this.getLocalBaseUrlOverride('local'), timeoutMs),
        ]);
        this.localProbeCache = [ollama, local];
        return this.localProbeCache;
    }

    /** The last known probe results without re-probing the network (may be empty before the first probe). */
    getLastLocalProbe(): LocalProviderProbeResult[] {
        return this.localProbeCache;
    }

    async getKey(providerId: string): Promise<string | undefined> {
        return this.ctx.secrets.get(SECRET_PREFIX + providerId);
    }

    async hasKey(providerId: string): Promise<boolean> {
        return !!(await this.getKey(providerId));
    }

    async setKey(providerId: string, key: string): Promise<void> {
        if (key) {
            await this.ctx.secrets.store(SECRET_PREFIX + providerId, key);
        } else {
            await this.ctx.secrets.delete(SECRET_PREFIX + providerId);
        }
    }

    /** Snapshot of which providers currently have a key configured. */
    async providerStatus(): Promise<Array<ProviderDef & { configured: boolean; localModels?: LocalDetectedModel[]; localBackendLabel?: string }>> {
        const probes = await this.probeLocalProviders();
        const out: Array<ProviderDef & { configured: boolean; localModels?: LocalDetectedModel[]; localBackendLabel?: string }> = [];
        for (const p of KNOWN_PROVIDERS) {
            if (p.kind === 'local') {
                const probe = probes.find((r) => r.providerId === p.id);
                out.push({ ...p, configured: !!probe?.detected, localModels: probe?.models ?? [], localBackendLabel: probe?.backendLabel });
            } else {
                out.push({ ...p, configured: await this.hasKey(p.id) });
            }
        }
        return out;
    }

    /**
     * Write all configured keys into an `.env` file at the OmniHarness root so the
     * orchestrator picks them up on next (re)start. Returns the path written.
     */
    async writeEnvFile(harnessRoot: string): Promise<string> {
        const lines: string[] = [
            '# Generated by the Omnisystem VS Code extension — OmniHarness panel settings.',
            '# Keys are mirrored here from VS Code SecretStorage so the orchestrator can load them.',
            '',
        ];
        for (const p of KNOWN_PROVIDERS) {
            if (p.kind === 'local') {
                lines.push(`${p.envVar}=1`);
                if (p.baseUrlEnv && p.defaultBaseUrl) {
                    lines.push(`${p.baseUrlEnv}=${p.defaultBaseUrl}`);
                }
                continue;
            }
            const key = await this.getKey(p.id);
            if (key) {
                lines.push(`${p.envVar}=${key}`);
            }
        }
        lines.push('');
        const envPath = path.join(harnessRoot, '.env');
        fs.writeFileSync(envPath, lines.join('\n'), 'utf8');
        return envPath;
    }

    // ── Agents ───────────────────────────────────────────────────────────────

    getAgents(): AgentPreset[] {
        const custom = this.ctx.globalState.get<AgentPreset[]>(AGENTS_KEY, []);
        // Built-ins first, then user agents; user agents may override built-ins by id.
        const byId = new Map<string, AgentPreset>();
        for (const a of builtinAgents()) { byId.set(a.id, a); }
        for (const a of custom) { byId.set(a.id, a); }
        return [...byId.values()];
    }

    getAgent(id: string): AgentPreset | undefined {
        return this.getAgents().find((a) => a.id === id);
    }

    async saveAgent(agent: AgentPreset): Promise<void> {
        const custom = this.ctx.globalState.get<AgentPreset[]>(AGENTS_KEY, []);
        const idx = custom.findIndex((a) => a.id === agent.id);
        const clean: AgentPreset = { ...agent, builtin: false };
        if (idx >= 0) { custom[idx] = clean; } else { custom.push(clean); }
        await this.ctx.globalState.update(AGENTS_KEY, custom);
    }

    async deleteAgent(id: string): Promise<void> {
        const custom = this.ctx.globalState.get<AgentPreset[]>(AGENTS_KEY, []);
        await this.ctx.globalState.update(AGENTS_KEY, custom.filter((a) => a.id !== id));
    }

    // ── Active selection ─────────────────────────────────────────────────────

    getActiveModel(): string {
        const cfg = vscode.workspace.getConfiguration('omnisystem').get<string>('harness.defaultModel', '');
        return this.ctx.workspaceState.get<string>(ACTIVE_MODEL_KEY, '') || cfg || 'anthropic/claude-sonnet-4-6';
    }

    async setActiveModel(model: string): Promise<void> {
        await this.ctx.workspaceState.update(ACTIVE_MODEL_KEY, model);
    }

    getActiveAgentId(): string {
        return this.ctx.workspaceState.get<string>(ACTIVE_AGENT_KEY, 'coder');
    }

    async setActiveAgentId(id: string): Promise<void> {
        await this.ctx.workspaceState.update(ACTIVE_AGENT_KEY, id);
    }

    // ── MCP servers ──────────────────────────────────────────────────────────

    getMcpServers(): McpServerConfig[] {
        const stored = this.ctx.globalState.get<McpServerConfig[]>(MCP_KEY);
        if (!stored) { return defaultMcpServers(); }
        return stored;
    }

    async saveMcpServer(server: McpServerConfig): Promise<void> {
        const servers = this.getMcpServers();
        const idx = servers.findIndex((s) => s.id === server.id);
        if (idx >= 0) { servers[idx] = server; } else { servers.push(server); }
        await this.ctx.globalState.update(MCP_KEY, servers);
    }

    async deleteMcpServer(id: string): Promise<void> {
        const servers = this.getMcpServers().filter((s) => s.id !== id);
        await this.ctx.globalState.update(MCP_KEY, servers);
    }

    async setMcpEnabled(id: string, enabled: boolean): Promise<void> {
        const servers = this.getMcpServers();
        const s = servers.find((x) => x.id === id);
        if (s) { s.enabled = enabled; await this.ctx.globalState.update(MCP_KEY, servers); }
    }

    // ── Chat sessions (per-workspace history) ────────────────────────────────

    listSessions(): ChatSession[] {
        const sessions = this.ctx.workspaceState.get<ChatSession[]>(SESSIONS_KEY, []);
        return [...sessions].sort((a, b) => b.updatedAt - a.updatedAt);
    }

    getSession(id: string): ChatSession | undefined {
        return this.ctx.workspaceState.get<ChatSession[]>(SESSIONS_KEY, []).find((s) => s.id === id);
    }

    async createSession(model: string, agentId: string): Promise<ChatSession> {
        const session: ChatSession = {
            id: `s${Date.now().toString(36)}${Math.random().toString(36).slice(2, 6)}`,
            title: 'New Chat',
            model, agentId,
            createdAt: Date.now(), updatedAt: Date.now(),
            messages: [], turns: [],
        };
        const sessions = this.ctx.workspaceState.get<ChatSession[]>(SESSIONS_KEY, []);
        sessions.unshift(session);
        if (sessions.length > MAX_SESSIONS) { sessions.length = MAX_SESSIONS; }
        await this.ctx.workspaceState.update(SESSIONS_KEY, sessions);
        await this.setActiveSessionId(session.id);
        return session;
    }

    /** Persist a session's transcript/messages (called after each turn). Auto-titles from the first user message. */
    async saveSession(session: ChatSession): Promise<void> {
        const sessions = this.ctx.workspaceState.get<ChatSession[]>(SESSIONS_KEY, []);
        if (session.title === 'New Chat') {
            const firstUser = session.turns.find((t) => t.role === 'user' && t.text);
            if (firstUser?.text) { session.title = firstUser.text.trim().slice(0, 60); }
        }
        session.updatedAt = Date.now();
        const idx = sessions.findIndex((s) => s.id === session.id);
        if (idx >= 0) { sessions[idx] = session; } else { sessions.unshift(session); }
        await this.ctx.workspaceState.update(SESSIONS_KEY, sessions);
    }

    async renameSession(id: string, title: string): Promise<void> {
        const sessions = this.ctx.workspaceState.get<ChatSession[]>(SESSIONS_KEY, []);
        const s = sessions.find((x) => x.id === id);
        if (s) { s.title = title.trim().slice(0, 120) || s.title; await this.ctx.workspaceState.update(SESSIONS_KEY, sessions); }
    }

    async deleteSession(id: string): Promise<void> {
        const sessions = this.ctx.workspaceState.get<ChatSession[]>(SESSIONS_KEY, []).filter((s) => s.id !== id);
        await this.ctx.workspaceState.update(SESSIONS_KEY, sessions);
        const checkpoints = this.ctx.workspaceState.get<Checkpoint[]>(CHECKPOINTS_KEY, []).filter((c) => c.sessionId !== id);
        await this.ctx.workspaceState.update(CHECKPOINTS_KEY, checkpoints);
    }

    getActiveSessionId(): string {
        return this.ctx.workspaceState.get<string>(ACTIVE_SESSION_KEY, '');
    }

    async setActiveSessionId(id: string): Promise<void> {
        await this.ctx.workspaceState.update(ACTIVE_SESSION_KEY, id);
    }

    // ── Checkpoints (undo for mutating tool calls) ───────────────────────────

    async saveCheckpoint(cp: Checkpoint): Promise<void> {
        const checkpoints = this.ctx.workspaceState.get<Checkpoint[]>(CHECKPOINTS_KEY, []);
        checkpoints.push(cp);
        if (checkpoints.length > MAX_CHECKPOINTS) { checkpoints.splice(0, checkpoints.length - MAX_CHECKPOINTS); }
        await this.ctx.workspaceState.update(CHECKPOINTS_KEY, checkpoints);
    }

    getCheckpoint(id: string): Checkpoint | undefined {
        return this.ctx.workspaceState.get<Checkpoint[]>(CHECKPOINTS_KEY, []).find((c) => c.id === id);
    }

    async deleteCheckpoint(id: string): Promise<void> {
        const checkpoints = this.ctx.workspaceState.get<Checkpoint[]>(CHECKPOINTS_KEY, []).filter((c) => c.id !== id);
        await this.ctx.workspaceState.update(CHECKPOINTS_KEY, checkpoints);
    }

    // ── Server profiles ──────────────────────────────────────────────────────

    listServerProfiles(): ServerProfile[] {
        const stored = this.ctx.globalState.get<ServerProfile[]>(SERVER_PROFILES_KEY);
        if (stored && stored.length) { return stored; }
        return [{ id: 'default', name: 'Local', url: 'http://localhost:8080' }];
    }

    async saveServerProfile(profile: ServerProfile): Promise<void> {
        const profiles = this.listServerProfiles();
        const idx = profiles.findIndex((p) => p.id === profile.id);
        if (idx >= 0) { profiles[idx] = profile; } else { profiles.push(profile); }
        await this.ctx.globalState.update(SERVER_PROFILES_KEY, profiles);
    }

    async deleteServerProfile(id: string): Promise<void> {
        const profiles = this.listServerProfiles().filter((p) => p.id !== id);
        await this.ctx.globalState.update(SERVER_PROFILES_KEY, profiles.length ? profiles : undefined);
    }

    getActiveServerProfileId(): string {
        return this.ctx.globalState.get<string>(ACTIVE_PROFILE_KEY, 'default');
    }

    async setActiveServerProfileId(id: string): Promise<void> {
        await this.ctx.globalState.update(ACTIVE_PROFILE_KEY, id);
    }

    // ── Favorite / pinned models ──────────────────────────────────────────────

    getFavoriteModels(): string[] {
        return this.ctx.globalState.get<string[]>(FAVORITE_MODELS_KEY, []);
    }

    async toggleFavoriteModel(id: string): Promise<string[]> {
        const favs = this.getFavoriteModels();
        const idx = favs.indexOf(id);
        if (idx >= 0) { favs.splice(idx, 1); } else { favs.unshift(id); }
        await this.ctx.globalState.update(FAVORITE_MODELS_KEY, favs);
        return favs;
    }
}
