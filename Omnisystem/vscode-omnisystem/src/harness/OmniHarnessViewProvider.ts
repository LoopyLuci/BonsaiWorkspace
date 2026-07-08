// OmniHarnessViewProvider — the sidebar AI panel: a Claude-Code-style chat +
// agent surface backed by the OmniHarness orchestrator. This is the primary way
// users drive any model (local or API) to work on their project inside VS Code.

import * as vscode from 'vscode';
import * as fs from 'fs';
import * as path from 'path';
import { spawn, ChildProcess } from 'child_process';
import { OmniHarnessClient, HarnessMessage, HarnessConnectionError } from './OmniHarnessClient';
import { HarnessStore, AgentPreset, ChatSession, ChatTurn, ServerProfile } from './HarnessStore';
import { VscodeTools, ToolDiff } from './VscodeTools';
import { AgentRunner, AgentEvents, ExecOutcome, ExternalTools } from './AgentRunner';
import { McpManager } from './mcp/McpManager';
import { McpServerConfig } from './mcp/McpTypes';

function getNonce(): string {
    let text = '';
    const possible = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
    for (let i = 0; i < 32; i++) { text += possible.charAt(Math.floor(Math.random() * possible.length)); }
    return text;
}

let turnSeq = 0;
function turnId(): string { return `tn${Date.now().toString(36)}${(turnSeq++).toString(36)}`; }

/** Strip a trailing ```tool / ```json fenced block, mirroring the client's own cleanup. */
const TOOL_BLOCK_TRAILING_RE = /```(?:tool|json)[\s\S]*?```\s*$/i;
function stripToolBlock(s: string): string {
    return s.replace(TOOL_BLOCK_TRAILING_RE, '').trim();
}

interface PendingApproval {
    resolve: (approved: boolean) => void;
}

export class OmniHarnessViewProvider implements vscode.WebviewViewProvider {
    public static readonly viewType = 'omniharnessChat';

    private view?: vscode.WebviewView;
    private client: OmniHarnessClient;
    private runner?: AgentRunner;
    private session: ChatSession;
    private pendingApprovals = new Map<string, PendingApproval>();
    private serverProc?: ChildProcess;
    private mcp: McpManager;
    private healthTimer?: ReturnType<typeof setInterval>;
    private lastAlive = false;
    private autoStartAttempted = false;

    // Self-healing process management: the orchestrator should "just stay up"
    // unless the user explicitly stops it. `manualStop` distinguishes a
    // deliberate Stop Server click from an unexpected crash so we know whether
    // to auto-restart; `restartAttempts` drives exponential backoff and caps
    // the crash-loop so we don't hammer a permanently broken environment.
    private manualStop = false;
    private restartAttempts = 0;
    private restartTimer?: ReturnType<typeof setTimeout>;
    private depsReady = false;
    private static readonly MAX_AUTO_RESTARTS = 6;

    // Rolling in-memory tail of everything written to the output channel, so the
    // Settings panel can show live server logs without needing "Open Output".
    private static readonly MAX_LOG_LINES = 500;
    private logBuffer: string[] = [];

    constructor(
        private readonly ctx: vscode.ExtensionContext,
        private readonly store: HarnessStore,
        private readonly output: vscode.OutputChannel,
    ) {
        this.client = new OmniHarnessClient(this.serverUrl());
        this.mcp = new McpManager((line) => this.log(line));
        this.session = this.loadOrCreateSession();
    }

    private log(line: string): void {
        this.output.appendLine(line);
        this.logBuffer.push(line);
        if (this.logBuffer.length > OmniHarnessViewProvider.MAX_LOG_LINES) { this.logBuffer.shift(); }
    }

    private logChunk(chunk: string): void {
        this.output.append(chunk);
        for (const line of chunk.split(/\r?\n/)) {
            if (line) { this.logBuffer.push(line); }
        }
        while (this.logBuffer.length > OmniHarnessViewProvider.MAX_LOG_LINES) { this.logBuffer.shift(); }
    }

    // ── Session management ───────────────────────────────────────────────────

    private loadOrCreateSession(): ChatSession {
        const activeId = this.store.getActiveSessionId();
        const existing = activeId ? this.store.getSession(activeId) : undefined;
        if (existing) { return existing; }
        const sessions = this.store.listSessions();
        if (sessions.length) {
            void this.store.setActiveSessionId(sessions[0].id);
            return sessions[0];
        }
        // Defer creation to first use; return an unsaved in-memory placeholder.
        return {
            id: `s${Date.now().toString(36)}`, title: 'New Chat',
            model: this.store.getActiveModel(), agentId: this.store.getActiveAgentId(),
            createdAt: Date.now(), updatedAt: Date.now(), messages: [], turns: [],
        };
    }

    private postSessionLoaded(): void {
        this.post({
            type: 'sessionLoaded',
            session: { id: this.session.id, title: this.session.title, turns: this.session.turns },
        });
    }

    private postSessionsList(): void {
        this.post({
            type: 'sessionsList',
            sessions: this.store.listSessions().map((s) => ({ id: s.id, title: s.title, updatedAt: s.updatedAt, messageCount: s.turns.length })),
            activeId: this.session.id,
        });
    }

    private async persistSession(): Promise<void> {
        await this.store.saveSession(this.session);
    }

    // ── MCP integration ──────────────────────────────────────────────────────

    /** Connect/refresh MCP servers and push their status to the webview. */
    private async syncMcp(): Promise<void> {
        try {
            await this.mcp.sync(this.store.getMcpServers());
        } catch (err) {
            this.log(`[MCP] sync error: ${err instanceof Error ? err.message : err}`);
        }
        this.postMcpState();
    }

    private postMcpState(): void {
        this.post({
            type: 'mcpState',
            servers: this.store.getMcpServers(),
            status: this.mcp.status(this.store.getMcpServers()),
            tools: this.mcp.aggregatedTools().map((t) => ({ name: t.qualifiedName, server: t.serverName, description: t.tool.description ?? '' })),
        });
    }

    /** Bridge the MCP manager to the agent runner's ExternalTools contract. */
    private buildExternalTools(): ExternalTools {
        const mgr = this.mcp;
        return {
            functionSchemas: () => mgr.aggregatedTools().map((t) => ({
                name: t.qualifiedName,
                description: `[MCP:${t.serverName}] ${t.tool.description ?? ''}`.trim(),
                parameters: (t.tool.inputSchema as Record<string, unknown>) ?? { type: 'object', properties: {} },
            })),
            describe: () => mgr.aggregatedTools()
                .map((t) => `  • ${t.qualifiedName} — [${t.serverName}] ${t.tool.description ?? ''}`)
                .join('\n'),
            has: (name) => McpManager.isMcpTool(name),
            execute: (name, args) => mgr.callTool(name, args),
        };
    }

    // ── Config helpers ───────────────────────────────────────────────────────

    private cfg() { return vscode.workspace.getConfiguration('omnisystem'); }

    /** The effective orchestrator URL: the active server profile, falling back to the plain setting. */
    private serverUrl(): string {
        const profiles = this.store.listServerProfiles();
        const active = profiles.find((p) => p.id === this.store.getActiveServerProfileId());
        return active?.url ?? this.cfg().get<string>('harness.serverUrl', 'http://localhost:8080');
    }

    private approvalMode(): string { return this.cfg().get<string>('harness.approvalMode', 'always'); }
    private toolMode(): string { return this.cfg().get<string>('harness.toolMode', 'auto'); }

    // Providers with a verified native function-calling round-trip (multi-turn
    // assistant tool_calls + tool results). Gemini/Cohere parse tool calls but do
    // not fully reconstruct structured turns, and Ollama models report no tool
    // support — those use the universal text protocol under 'auto'.
    private static readonly NATIVE_PROVIDERS = new Set([
        'anthropic', 'openai', 'groq', 'mistral', 'openrouter', 'together', 'fireworks',
    ]);

    /** modelId (both bare and provider-qualified) → provider, from /api/models. */
    private modelProvider = new Map<string, string>();
    private toolCapableModels = new Set<string>();
    private lastModels: Array<{ id: string; provider: string; context_window?: number; supports_tools?: boolean }> = [];

    private modelSupportsTools(modelId: string): boolean {
        const bare = modelId.includes('/') ? modelId.split('/').pop()! : modelId;
        return this.toolCapableModels.has(bare) || this.toolCapableModels.has(modelId);
    }

    private providerOf(modelId: string): string | undefined {
        const bare = modelId.includes('/') ? modelId.split('/').pop()! : modelId;
        return this.modelProvider.get(modelId) ?? this.modelProvider.get(bare);
    }

    /** Resolve whether to use native function calling for a given model. */
    private useNativeTools(modelId: string): boolean {
        const mode = this.toolMode();
        if (mode === 'native') { return true; }
        if (mode === 'text') { return false; }
        // 'auto': native only when the model supports tools AND its provider has a
        // verified native round-trip; otherwise fall back to the text protocol.
        const provider = this.providerOf(modelId);
        return this.modelSupportsTools(modelId)
            && !!provider
            && OmniHarnessViewProvider.NATIVE_PROVIDERS.has(provider);
    }

    private harnessRoot(): string | undefined {
        const configured = this.cfg().get<string>('harness.orchestratorPath', '').trim();
        if (configured && fs.existsSync(configured)) { return configured; }
        const roots = vscode.workspace.workspaceFolders?.map((f) => f.uri.fsPath) ?? [];
        for (const r of roots) {
            for (const candidate of [
                path.join(r, 'OmniHarness'),
                path.join(r, 'Omnisystem', 'OmniHarness'),
            ]) {
                if (fs.existsSync(path.join(candidate, 'orchestrator'))) { return candidate; }
            }
        }
        return undefined;
    }

    // ── Webview lifecycle ────────────────────────────────────────────────────

    resolveWebviewView(view: vscode.WebviewView): void {
        this.view = view;
        view.webview.options = {
            enableScripts: true,
            localResourceRoots: [this.ctx.extensionUri],
        };
        view.webview.html = this.getHtml(view.webview);
        view.webview.onDidReceiveMessage((msg) => this.handleMessage(msg));
        // Push initial state once the webview is ready (it will request it too).
        void this.refreshState();
        void this.probeServerAndMaybeAutoStart();
        void this.syncMcp();

        // Light background health poll so the status dot tracks a server that is
        // started or stopped outside the panel. On a state transition we do a full
        // probe (with model refresh); while the server stays up we also refresh the
        // model list periodically so newly-discovered local models (e.g. a fresh
        // `ollama pull`, or LM Studio started after us) appear without any manual
        // reload — "auto-updating data where possible".
        if (this.healthTimer) { clearInterval(this.healthTimer); }
        let tick = 0;
        this.healthTimer = setInterval(async () => {
            const alive = await this.client.isAlive();
            if (alive !== this.lastAlive) {
                this.lastAlive = alive;
                if (alive) { await this.probeServer(); }
                else { this.post({ type: 'serverStatus', alive: false }); }
            } else if (alive && (++tick % 4 === 0)) {
                // ~every 60s while healthy: refresh the model catalogue quietly, and
                // re-probe local runtimes (Ollama / LM Studio / llama.cpp) so a backend
                // started after us appears with zero manual reload.
                await this.sendModels();
                await this.refreshState();
            }
        }, 15000);
        view.onDidDispose(() => {
            if (this.healthTimer) { clearInterval(this.healthTimer); this.healthTimer = undefined; }
        });
    }

    public focus(): void {
        void vscode.commands.executeCommand('omniharnessChat.focus');
    }

    // ── Public command entry points (invoked from extension.ts) ──────────────

    public newSession(): void {
        void this.createNewSession();
    }

    public async startServerCommand(): Promise<void> {
        await this.startServer();
    }

    public stopServerCommand(): void {
        this.stopServer();
    }

    public async addSelectionCommand(): Promise<void> {
        await this.addSelectionContext();
    }

    public async exportConfigCommand(): Promise<void> {
        await this.exportConfig();
    }

    public async importConfigCommand(): Promise<void> {
        await this.importConfig();
    }

    /** Undo the most recent still-undoable mutating tool call in the active session. */
    public async undoLastCommand(): Promise<void> {
        const last = [...this.session.turns].reverse().find((t) => t.role === 'tool' && t.checkpointId);
        if (!last?.checkpointId) {
            this.post({ type: 'toast', text: 'No undoable change in the active session.', error: true });
            void vscode.window.showInformationMessage('OmniHarness: no undoable change in the active session.');
            return;
        }
        await this.undoToolCall(last.checkpointId);
    }

    private post(msg: object): void {
        this.view?.webview.postMessage(msg);
    }

    // ── Message routing (webview → extension) ────────────────────────────────

    private async handleMessage(msg: any): Promise<void> {
        switch (msg?.type) {
            case 'ready':        await this.refreshState(); await this.probeServer(); this.postSessionLoaded(); this.postSessionsList(); this.postFavorites(); break;
            case 'send':         await this.onSend(String(msg.text ?? '')); break;
            case 'stop':         this.runner?.abort(); break;
            case 'newSession':   await this.createNewSession(); break;
            case 'listSessions': this.postSessionsList(); break;
            case 'switchSession':await this.switchSession(String(msg.id)); break;
            case 'renameSession':await this.store.renameSession(String(msg.id), String(msg.title ?? '')); this.postSessionsList(); break;
            case 'deleteSession':await this.deleteSession(String(msg.id)); break;
            case 'setModel':     await this.store.setActiveModel(String(msg.model)); this.session.model = String(msg.model); await this.persistSession(); break;
            case 'setAgent':     await this.store.setActiveAgentId(String(msg.agent)); this.session.agentId = String(msg.agent); await this.persistSession(); break;
            case 'refreshModels':await this.sendModels(); break;
            case 'approve':      this.resolveApproval(String(msg.id), !!msg.approved); break;
            case 'saveKey':      await this.onSaveKey(String(msg.provider), String(msg.key ?? '')); break;
            case 'saveLocalUrl': await this.store.setLocalBaseUrlOverride(String(msg.provider), String(msg.url ?? '')); await this.refreshState(); break;
            case 'applyEnv':     await this.onApplyEnv(); break;
            case 'saveAgent':    await this.onSaveAgent(msg.agent as AgentPreset); break;
            case 'deleteAgent':  await this.store.deleteAgent(String(msg.id)); await this.refreshState(); break;
            case 'startServer':  await this.startServer(); break;
            case 'stopServer':   this.stopServer(); break;
            case 'addSelection': await this.addSelectionContext(); break;
            case 'openExternal': if (msg.url) { void vscode.env.openExternal(vscode.Uri.parse(String(msg.url))); } break;
            case 'openSettings': void vscode.commands.executeCommand('workbench.action.openSettings', 'omnisystem.harness'); break;
            case 'getMcp':       this.postMcpState(); break;
            case 'syncMcp':      await this.syncMcp(); this.post({ type: 'toast', text: 'MCP servers refreshed.' }); break;
            case 'saveMcpServer':await this.store.saveMcpServer(msg.server as McpServerConfig); await this.syncMcp(); break;
            case 'deleteMcpServer': await this.store.deleteMcpServer(String(msg.id)); await this.syncMcp(); break;
            case 'toggleMcp':    await this.store.setMcpEnabled(String(msg.id), !!msg.enabled); await this.syncMcp(); break;
            case 'undoToolCall': await this.undoToolCall(String(msg.checkpointId)); break;
            case 'toggleFavoriteModel': await this.store.toggleFavoriteModel(String(msg.id)); this.postFavorites(); break;
            case 'getFavorites': this.postFavorites(); break;
            case 'getServerProfiles': this.postServerProfiles(); break;
            case 'saveServerProfile': await this.store.saveServerProfile(msg.profile as ServerProfile); this.postServerProfiles(); break;
            case 'deleteServerProfile': await this.store.deleteServerProfile(String(msg.id)); this.postServerProfiles(); break;
            case 'switchServerProfile': await this.switchServerProfile(String(msg.id)); break;
            case 'getLogs':      this.post({ type: 'logs', lines: this.logBuffer }); break;
            case 'exportConfig': await this.exportConfig(); break;
            case 'importConfig': await this.importConfig(); break;
            case 'compactNow':   await this.compactNowCommand(); break;
        }
    }

    // ── Sessions ─────────────────────────────────────────────────────────────

    private async createNewSession(): Promise<void> {
        const session = await this.store.createSession(this.store.getActiveModel(), this.store.getActiveAgentId());
        this.session = session;
        this.postSessionLoaded();
        this.postSessionsList();
    }

    private async switchSession(id: string): Promise<void> {
        const s = this.store.getSession(id);
        if (!s) { return; }
        this.session = s;
        await this.store.setActiveSessionId(id);
        if (s.model) { await this.store.setActiveModel(s.model); }
        if (s.agentId) { await this.store.setActiveAgentId(s.agentId); }
        this.postSessionLoaded();
        this.postSessionsList();
        await this.refreshState();
    }

    private async deleteSession(id: string): Promise<void> {
        const wasActive = id === this.session.id;
        await this.store.deleteSession(id);
        if (wasActive) {
            const remaining = this.store.listSessions();
            if (remaining.length) { await this.switchSession(remaining[0].id); }
            else { await this.createNewSession(); }
        } else {
            this.postSessionsList();
        }
    }

    // ── State push (extension → webview) ─────────────────────────────────────

    private async refreshState(): Promise<void> {
        const providers = await this.store.providerStatus();
        this.post({
            type: 'state',
            activeModel: this.store.getActiveModel(),
            activeAgent: this.store.getActiveAgentId(),
            agents: this.store.getAgents(),
            providers,
            approvalMode: this.approvalMode(),
            serverUrl: this.serverUrl(),
        });
        this.postServerProfiles();
    }

    private postFavorites(): void {
        this.post({ type: 'favorites', favorites: this.store.getFavoriteModels() });
    }

    private postServerProfiles(): void {
        this.post({
            type: 'serverProfiles',
            profiles: this.store.listServerProfiles(),
            activeId: this.store.getActiveServerProfileId(),
        });
    }

    private async switchServerProfile(id: string): Promise<void> {
        await this.store.setActiveServerProfileId(id);
        this.client.setBaseUrl(this.serverUrl());
        this.autoStartAttempted = false;
        this.postServerProfiles();
        await this.refreshState();
        await this.probeServer();
    }

    private async sendModels(): Promise<void> {
        try {
            const models = await this.client.listModels();
            this.toolCapableModels = new Set(
                models.filter((m) => m.supports_tools).map((m) => m.id),
            );
            this.modelProvider = new Map();
            for (const m of models) {
                this.modelProvider.set(m.id, m.provider);
                this.modelProvider.set(`${m.provider}/${m.id}`, m.provider);
            }
            this.lastModels = models;
            this.post({ type: 'models', models });
        } catch (err) {
            this.post({ type: 'models', models: [], error: err instanceof Error ? err.message : String(err) });
        }
    }

    private async probeServer(): Promise<void> {
        try {
            const health = await this.client.health();
            this.lastAlive = true;
            this.post({ type: 'serverStatus', alive: true, health });
            await this.sendModels();
        } catch {
            this.lastAlive = false;
            this.post({ type: 'serverStatus', alive: false });
        }
    }

    private async probeServerAndMaybeAutoStart(): Promise<void> {
        await this.probeServer();
        if (this.lastAlive || this.autoStartAttempted) { return; }
        if (this.cfg().get<boolean>('harness.autoStartServer', false)) {
            this.autoStartAttempted = true;
            await this.startServer();
        }
    }

    // ── Intelligent context compaction ───────────────────────────────────────
    //
    // The naive approach (slice the last N messages) silently throws away
    // older context with no visibility and no regard for the model's actual
    // context window. Instead: estimate live token usage against the active
    // model's real context_window, and once it crosses a configurable
    // threshold, ask the model itself to fold everything but a recent tail
    // into a dense running summary. That summary — not the raw turns — is
    // what gets prepended to future requests. The full, unsummarized
    // `session.messages`/`session.turns` are never touched, so history,
    // undo, and export always show the complete conversation; only what is
    // actually sent to the model is compacted, and every compaction is
    // recorded as a visible turn plus a toast so it is never a silent black box.

    private static readonly RESERVED_OUTPUT_TOKENS = 4096;
    private static readonly DEFAULT_CONTEXT_WINDOW = 128000;
    private static readonly COMPACT_KEEP_TAIL = 8;

    private estimateTokensForMessages(messages: HarnessMessage[]): number {
        let chars = 0;
        for (const m of messages) { chars += m.content?.length ?? 0; }
        return Math.ceil(chars / 4);
    }

    private modelContextWindow(modelId: string): number {
        const bare = modelId.includes('/') ? modelId.split('/').pop()! : modelId;
        const entry = this.lastModels.find((m) => m.id === modelId || m.id === bare);
        return entry?.context_window ?? OmniHarnessViewProvider.DEFAULT_CONTEXT_WINDOW;
    }

    /**
     * Builds the message list actually sent to the model for this turn,
     * compacting older history into a running summary when `force` is set
     * or the live context has crossed the configured token threshold.
     */
    private async buildContextMessages(model: string, force = false): Promise<HarnessMessage[]> {
        const all = this.session.messages;
        const coveredUpTo = Math.min(this.session.contextSummaryUpToIndex ?? 0, all.length);
        const tail = all.slice(coveredUpTo);
        const summaryMsg: HarnessMessage[] = this.session.contextSummary
            ? [{ role: 'system', content: `Summary of earlier conversation (compacted to save context):\n${this.session.contextSummary}` }]
            : [];

        const autoCompact = this.cfg().get<boolean>('harness.autoCompact', true);
        if (!autoCompact && !force) { return [...summaryMsg, ...tail]; }

        const contextWindow = this.modelContextWindow(model);
        const threshold = this.cfg().get<number>('harness.compactThreshold', 0.75);
        const budget = Math.max(2000, Math.floor(contextWindow * threshold) - OmniHarnessViewProvider.RESERVED_OUTPUT_TOKENS);
        const currentTokens = this.estimateTokensForMessages(summaryMsg) + this.estimateTokensForMessages(tail);

        const needsCompaction = force ? tail.length > OmniHarnessViewProvider.COMPACT_KEEP_TAIL : currentTokens > budget;
        if (!needsCompaction || tail.length <= OmniHarnessViewProvider.COMPACT_KEEP_TAIL) {
            if (force) { this.post({ type: 'toast', text: 'Nothing worth compacting yet — conversation is still short.' }); }
            return [...summaryMsg, ...tail];
        }

        const toSummarize = tail.slice(0, tail.length - OmniHarnessViewProvider.COMPACT_KEEP_TAIL);
        const keep = tail.slice(tail.length - OmniHarnessViewProvider.COMPACT_KEEP_TAIL);

        try {
            const priorSummary = this.session.contextSummary ? `Existing running summary so far:\n${this.session.contextSummary}\n\n` : '';
            const transcript = toSummarize.map((m) => `${m.role.toUpperCase()}: ${m.content}`).join('\n\n').slice(0, 60000);
            const result = await this.client.chat({
                model_id: model,
                messages: [{
                    role: 'user',
                    content: `${priorSummary}Summarize the following conversation turns into a concise but complete running summary. `
                        + 'Preserve: key facts, decisions made, file paths touched, code changes made, open TODOs, and any unresolved questions. '
                        + 'Be dense — this summary replaces the original turns as the model\'s only memory of them. Output only the summary, no commentary.'
                        + `\n\n${transcript}`,
                }],
                temperature: 0.2,
                max_tokens: 1200,
            });
            const newSummary = result.content.trim();
            if (!newSummary) { throw new Error('empty summary'); }

            const tokensBefore = this.estimateTokensForMessages(toSummarize);
            const tokensSaved = Math.max(0, tokensBefore - Math.ceil(newSummary.length / 4));
            this.session.contextSummary = newSummary;
            this.session.contextSummaryUpToIndex = coveredUpTo + toSummarize.length;
            this.session.turns.push({
                id: turnId(), role: 'compaction', ts: Date.now(),
                text: `Compacted ${toSummarize.length} earlier message(s) into a running summary (~${tokensSaved} tokens saved).`,
            });
            void this.persistSession();
            this.post({
                type: 'compaction', summary: newSummary, messagesCompacted: toSummarize.length, tokensSaved,
            });
            return [{ role: 'system', content: `Summary of earlier conversation (compacted to save context):\n${newSummary}` }, ...keep];
        } catch (err) {
            // Summarization call failed (e.g. server hiccup) — degrade gracefully
            // to bounded truncation rather than blowing the context window or
            // failing the turn outright.
            this.log(`[OmniHarness] Auto-compaction failed, falling back to truncation: ${err instanceof Error ? err.message : err}`);
            return [...summaryMsg, ...tail.slice(-20)];
        }
    }

    public async compactNowCommand(): Promise<void> {
        const model = this.store.getActiveModel();
        if (!model) { this.post({ type: 'toast', text: 'No active model selected.', error: true }); return; }
        this.post({ type: 'toast', text: 'Compacting conversation context...' });
        await this.buildContextMessages(model, true);
    }

    // ── Chat / agent turn ────────────────────────────────────────────────────

    private async onSend(text: string): Promise<void> {
        if (!text.trim()) { return; }
        this.client.setBaseUrl(this.serverUrl());

        const model = this.store.getActiveModel();
        const agent = this.store.getAgent(this.store.getActiveAgentId()) ?? this.store.getAgents()[0];

        this.post({ type: 'userMessage', text });
        this.post({ type: 'assistantStart' });
        this.session.turns.push({ id: turnId(), role: 'user', text, ts: Date.now() });

        const pendingToolMeta = new Map<string, { tool: string; args: Record<string, unknown> }>();
        let lastAssistantTurnText: string | null = null;

        const tools = new VscodeTools((tool, args, preview, diff) => this.requestApproval(tool, args, preview, agent, diff));
        const events: AgentEvents = {
            onAssistantDelta: (d: string) => this.post({ type: 'assistantDelta', text: d }),
            onAssistantDone: (f: string) => {
                this.post({ type: 'assistantDone', text: f });
                const stripped = stripToolBlock(f);
                if (stripped) {
                    this.session.turns.push({ id: turnId(), role: 'assistant', text: stripped, ts: Date.now() });
                    lastAssistantTurnText = stripped;
                } else {
                    lastAssistantTurnText = null;
                }
            },
            onToolCall: (id: string, tool: string, args: Record<string, unknown>) => {
                pendingToolMeta.set(id, { tool, args });
                this.post({ type: 'toolCall', id, tool, args });
            },
            onToolResult: (id: string, outcome: ExecOutcome) => {
                const meta = pendingToolMeta.get(id);
                pendingToolMeta.delete(id);
                let checkpointId: string | undefined;
                if (outcome.checkpoint) {
                    checkpointId = id;
                    void this.store.saveCheckpoint({
                        id: checkpointId, sessionId: this.session.id,
                        path: outcome.checkpoint.fsPath, before: outcome.checkpoint.before, ts: Date.now(),
                    });
                }
                this.post({
                    type: 'toolResult', id, ok: outcome.ok, summary: outcome.summary, content: outcome.content,
                    diff: outcome.diff, checkpointId,
                });
                this.session.turns.push({
                    id: turnId(), role: 'tool', ts: Date.now(),
                    toolName: meta?.tool, toolArgs: meta?.args,
                    toolOk: outcome.ok, toolSummary: outcome.summary, toolContent: outcome.content,
                    diff: outcome.diff ?? null, checkpointId,
                });
            },
            onStatus: (s: string) => this.post({ type: 'status', text: s }),
            onFinal: (a: string) => {
                this.session.messages.push({ role: 'user', content: text });
                this.session.messages.push({ role: 'assistant', content: a });
                if (a && a !== lastAssistantTurnText) {
                    this.session.turns.push({ id: turnId(), role: 'assistant', text: a, ts: Date.now() });
                }
                this.post({ type: 'final', text: a });
                void this.persistSession();
                this.postSessionsList();
            },
            onError: (m: string) => {
                this.session.turns.push({ id: turnId(), role: 'error', text: m, ts: Date.now() });
                this.post({ type: 'error', message: m });
                void this.persistSession();
            },
        };
        this.runner = new AgentRunner(this.client, tools, events, this.buildExternalTools());

        // Keep model context bounded so the context window stays healthy; the full
        // transcript is still preserved in session.turns for display/undo/history.
        const boundedHistory = await this.buildContextMessages(model);
        const native = this.useNativeTools(model);
        this.post({ type: 'status', text: native ? 'Native tools' : 'Text tools' });
        try {
            if (native) {
                await this.runner.runNative(agent, model, boundedHistory, text);
            } else {
                await this.runner.run(agent, model, boundedHistory, text);
            }
        } catch (err) {
            if (err instanceof HarnessConnectionError) {
                this.post({ type: 'error', message: err.message, needsServer: true });
            } else {
                this.post({ type: 'error', message: err instanceof Error ? err.message : String(err) });
            }
        }
    }

    // ── Undo (checkpoints) ───────────────────────────────────────────────────

    private async undoToolCall(checkpointId: string): Promise<void> {
        const cp = this.store.getCheckpoint(checkpointId);
        if (!cp) { this.post({ type: 'toast', text: 'Nothing to undo (checkpoint expired or already used).', error: true }); return; }
        try {
            const uri = vscode.Uri.file(cp.path);
            if (cp.before === null) {
                await vscode.workspace.fs.delete(uri, { useTrash: true });
            } else {
                await vscode.workspace.fs.writeFile(uri, new TextEncoder().encode(cp.before));
            }
            await this.store.deleteCheckpoint(checkpointId);
            const rel = vscode.workspace.asRelativePath(uri);
            this.post({ type: 'undoDone', checkpointId, path: rel });
            this.post({ type: 'toast', text: `Reverted ${rel}.` });
            const turn = this.session.turns.find((t) => t.checkpointId === checkpointId);
            if (turn) { turn.checkpointId = undefined; }
            await this.persistSession();
        } catch (err) {
            this.post({ type: 'toast', text: `Undo failed: ${err instanceof Error ? err.message : err}`, error: true });
        }
    }

    // ── Approval flow ────────────────────────────────────────────────────────

    private async requestApproval(
        tool: string,
        args: Record<string, unknown>,
        preview: string,
        agent: AgentPreset,
        diff?: ToolDiff,
    ): Promise<boolean> {
        const mode = this.approvalMode();
        if (mode === 'yolo') { return true; }
        if (agent.autoApprove && mode !== 'always') { return true; }
        const id = `ap${Date.now().toString(36)}`;
        return new Promise<boolean>((resolve) => {
            this.pendingApprovals.set(id, { resolve });
            this.post({ type: 'approvalRequest', id, tool, args, preview, diff });
        });
    }

    private resolveApproval(id: string, approved: boolean): void {
        const p = this.pendingApprovals.get(id);
        if (p) { this.pendingApprovals.delete(id); p.resolve(approved); }
    }

    // ── Settings actions ─────────────────────────────────────────────────────

    private async onSaveKey(provider: string, key: string): Promise<void> {
        await this.store.setKey(provider, key);
        await this.refreshState();
        this.post({ type: 'toast', text: key ? `Saved key for ${provider}.` : `Cleared key for ${provider}.` });
    }

    private async onApplyEnv(): Promise<void> {
        const root = this.harnessRoot();
        if (!root) {
            this.post({ type: 'toast', text: 'OmniHarness folder not found. Set omnisystem.harness.orchestratorPath.', error: true });
            return;
        }
        try {
            const envPath = await this.store.writeEnvFile(root);
            this.log(`[OmniHarness] Wrote provider keys to ${envPath}`);
            const restart = await vscode.window.showInformationMessage(
                `Wrote keys to ${envPath}. Restart the orchestrator to apply?`, 'Restart Server', 'Later',
            );
            if (restart === 'Restart Server') { this.stopServer(); await this.startServer(); }
        } catch (err) {
            this.post({ type: 'toast', text: `Failed to write .env: ${err instanceof Error ? err.message : err}`, error: true });
        }
    }

    private async onSaveAgent(agent: AgentPreset): Promise<void> {
        if (!agent?.id || !agent?.name) { return; }
        await this.store.saveAgent(agent);
        await this.refreshState();
        this.post({ type: 'toast', text: `Saved agent "${agent.name}".` });
    }

    private async addSelectionContext(): Promise<void> {
        const editor = vscode.window.activeTextEditor;
        if (!editor) { this.post({ type: 'toast', text: 'No active editor.' }); return; }
        const sel = editor.selection;
        const file = vscode.workspace.asRelativePath(editor.document.uri);
        const snippet = editor.document.getText(sel.isEmpty ? undefined : sel);
        const where = sel.isEmpty ? file : `${file}:${sel.start.line + 1}-${sel.end.line + 1}`;
        this.post({ type: 'insertContext', label: where, text: `\n\nContext from ${where}:\n\`\`\`\n${snippet.slice(0, 4000)}\n\`\`\`\n` });
    }

    // ── Import / export config (agents, MCP servers, server profiles — no secrets) ──

    private async exportConfig(): Promise<void> {
        const bundle = {
            version: 1,
            exportedAt: new Date().toISOString(),
            agents: this.store.getAgents().filter((a) => !a.builtin),
            mcpServers: this.store.getMcpServers(),
            serverProfiles: this.store.listServerProfiles(),
        };
        const uri = await vscode.window.showSaveDialog({
            defaultUri: vscode.Uri.file('omniharness-config.json'),
            filters: { JSON: ['json'] },
            title: 'Export OmniHarness Configuration',
        });
        if (!uri) { return; }
        await vscode.workspace.fs.writeFile(uri, new TextEncoder().encode(JSON.stringify(bundle, null, 2)));
        this.post({ type: 'toast', text: `Exported configuration to ${uri.fsPath}.` });
    }

    private async importConfig(): Promise<void> {
        const uris = await vscode.window.showOpenDialog({
            canSelectMany: false, filters: { JSON: ['json'] }, title: 'Import OmniHarness Configuration',
        });
        if (!uris || !uris.length) { return; }
        try {
            const bytes = await vscode.workspace.fs.readFile(uris[0]);
            const bundle = JSON.parse(new TextDecoder().decode(bytes));
            let count = 0;
            for (const agent of bundle.agents ?? []) { await this.store.saveAgent(agent as AgentPreset); count++; }
            for (const server of bundle.mcpServers ?? []) { await this.store.saveMcpServer(server as McpServerConfig); count++; }
            for (const profile of bundle.serverProfiles ?? []) { await this.store.saveServerProfile(profile as ServerProfile); count++; }
            await this.refreshState();
            await this.syncMcp();
            this.post({ type: 'toast', text: `Imported ${count} item(s) from ${uris[0].fsPath}.` });
        } catch (err) {
            this.post({ type: 'toast', text: `Import failed: ${err instanceof Error ? err.message : err}`, error: true });
        }
    }

    // ── Orchestrator process management ──────────────────────────────────────
    //
    // Goal: the orchestrator "just works" and stays running unless the user
    // explicitly stops it. That means: (1) auto-detect and install missing
    // Python dependencies before the first launch attempt in a session, so a
    // fresh checkout doesn't fail with "No module named uvicorn"; (2) if the
    // process crashes unexpectedly, auto-restart with exponential backoff
    // instead of silently staying dead; (3) never fight a manual Stop Server.

    /** Runs a command in `cwd`, streaming stdout/stderr into the log. Resolves with the exit code. */
    private runCommand(py: string, args: string[], cwd: string): Promise<number> {
        return new Promise((resolve) => {
            let proc: ChildProcess;
            try {
                proc = spawn(py, args, { cwd, shell: true });
            } catch (e) {
                this.log(`[OmniHarness] Failed to launch ${py}: ${e instanceof Error ? e.message : e}`);
                resolve(1);
                return;
            }
            proc.stdout?.on('data', (d: Buffer) => this.logChunk(d.toString()));
            proc.stderr?.on('data', (d: Buffer) => this.logChunk(d.toString()));
            proc.on('error', (e) => { this.log(`[OmniHarness] Command failed: ${e.message}`); resolve(1); });
            proc.on('close', (code) => resolve(code ?? 1));
        });
    }

    /**
     * Ensures the orchestrator's third-party dependencies are importable,
     * installing them automatically if not — so a fresh checkout "just runs".
     *
     * Key design choices for robustness:
     *  • We do NOT `pip install -e .` the omniharness package. Running
     *    `uvicorn omniharness.server:app` from cwd=orchestrator imports the
     *    package straight from the source tree, so we only need its runtime
     *    dependencies present. This sidesteps the whole editable-build path
     *    (hatchling, README metadata, PEP 660 backend quirks) that was failing.
     *  • The import check is passed as a single JSON-quoted `-c` argument so
     *    shell:true on Windows can't split it on spaces (the old check sent
     *    Python just `import`, producing a SyntaxError).
     *  • Install strategy is layered: requirements.txt first, then a direct
     *    inline dependency list, then `-e .` as a last resort — whichever
     *    first makes the imports succeed wins.
     */
    private async ensureDependencies(py: string, orchestrator: string): Promise<boolean> {
        if (this.depsReady) { return true; }
        if (!this.cfg().get<boolean>('harness.autoInstallDependencies', true)) { this.depsReady = true; return true; }

        // Critical modules the server imports at startup. Quoted as ONE arg.
        const importCheck = 'import uvicorn, fastapi, pydantic, httpx, dotenv, sse_starlette, aiosqlite, aiofiles';
        const check = async () => (await this.runCommand(py, ['-c', JSON.stringify(importCheck)], orchestrator)) === 0;

        if (await check()) { this.depsReady = true; return true; }

        this.log('[OmniHarness] Orchestrator dependencies missing — installing automatically...');
        this.post({ type: 'toast', text: 'Installing OmniHarness orchestrator dependencies (first run, this can take a minute)...' });

        // Make sure pip itself is available (some minimal installs lack it).
        await this.runCommand(py, ['-m', 'ensurepip', '--upgrade'], orchestrator);

        const pipBase = ['-m', 'pip', 'install', '--upgrade', '--disable-pip-version-check'];
        const reqPath = path.join(orchestrator, 'requirements.txt');
        const attempts: Array<{ label: string; args: string[] }> = [];
        if (fs.existsSync(reqPath)) {
            attempts.push({ label: 'requirements.txt', args: [...pipBase, '-r', 'requirements.txt'] });
        }
        attempts.push({ label: 'inline dependency list', args: [...pipBase, ...OmniHarnessViewProvider.RUNTIME_DEPS] });
        attempts.push({ label: 'editable package (-e .)', args: [...pipBase, '-e', '.'] });

        for (const attempt of attempts) {
            this.log(`[OmniHarness] Installing dependencies via ${attempt.label}...`);
            await this.runCommand(py, attempt.args, orchestrator);
            if (await check()) {
                this.log(`[OmniHarness] Orchestrator dependencies satisfied (${attempt.label}).`);
                this.depsReady = true;
                return true;
            }
            this.log(`[OmniHarness] ${attempt.label} did not satisfy all imports — trying next strategy...`);
        }

        this.log('[OmniHarness] All dependency-install strategies failed. See the log above for the underlying pip error.');
        this.post({
            type: 'toast',
            text: `Could not auto-install orchestrator dependencies. Open the Output log for details, or run "${py} -m pip install -r requirements.txt" manually in ${orchestrator}.`,
            error: true,
        });
        return false;
    }

    // Runtime dependencies the orchestrator needs to boot and serve. Mirrors
    // requirements.txt; used as a direct-install fallback if that file is absent
    // or a build-backend path fails. Kept unpinned-loose to match the source.
    private static readonly RUNTIME_DEPS = [
        'httpx>=0.27', 'fastapi>=0.115', 'uvicorn[standard]>=0.30', 'pydantic>=2',
        'anthropic>=0.40', 'openai>=1.55', 'google-generativeai>=0.8',
        'numpy>=1.26', 'python-dotenv>=1.0', 'aiofiles>=23', 'websockets>=12',
        'aiosqlite>=0.20', 'sse-starlette>=2.0',
    ];

    private spawnServerProcess(py: string, orchestrator: string, port: string): void {
        this.log(`[OmniHarness] Starting orchestrator: ${py} -m uvicorn omniharness.server:app --port ${port}`);
        this.log(`[OmniHarness] cwd: ${orchestrator}`);

        this.serverProc = spawn(py, ['-m', 'uvicorn', 'omniharness.server:app', '--host', '0.0.0.0', '--port', port], {
            cwd: orchestrator, shell: true,
        });
        this.serverProc.stdout?.on('data', (d: Buffer) => this.logChunk(d.toString()));
        this.serverProc.stderr?.on('data', (d: Buffer) => this.logChunk(d.toString()));
        this.serverProc.on('error', (e) => this.log(`[OmniHarness] Failed to start: ${e.message}`));
        this.serverProc.on('close', (code) => {
            this.log(`[OmniHarness] Orchestrator exited (${code}).`);
            this.post({ type: 'serverStatus', alive: false });
            this.serverProc = undefined;
            if (this.manualStop) { return; }
            if (!this.cfg().get<boolean>('harness.autoRestartServer', true)) { return; }
            this.scheduleAutoRestart(py, orchestrator, port);
        });
    }

    /** Polls /api/health for up to ~20s; resets the restart-backoff counter once healthy. */
    private async pollUntilHealthy(): Promise<boolean> {
        for (let i = 0; i < 20; i++) {
            await new Promise((r) => setTimeout(r, 1000));
            if (this.manualStop) { return false; }
            if (await this.client.isAlive()) {
                this.restartAttempts = 0;
                await this.probeServer();
                return true;
            }
        }
        return false;
    }

    private scheduleAutoRestart(py: string, orchestrator: string, port: string): void {
        if (this.restartAttempts >= OmniHarnessViewProvider.MAX_AUTO_RESTARTS) {
            this.post({
                type: 'toast',
                text: `Orchestrator crashed ${OmniHarnessViewProvider.MAX_AUTO_RESTARTS} times in a row and was not restarted again. `
                    + 'Check the Output log for the underlying error, then use "Start Server" once it is fixed.',
                error: true,
            });
            return;
        }
        this.restartAttempts++;
        const delayMs = Math.min(1000 * 2 ** (this.restartAttempts - 1), 30000);
        this.log(`[OmniHarness] Orchestrator crashed unexpectedly — auto-restarting in ${Math.round(delayMs / 1000)}s `
            + `(attempt ${this.restartAttempts}/${OmniHarnessViewProvider.MAX_AUTO_RESTARTS})...`);
        if (this.restartTimer) { clearTimeout(this.restartTimer); }
        this.restartTimer = setTimeout(() => {
            if (this.manualStop) { return; }
            this.spawnServerProcess(py, orchestrator, port);
            void this.pollUntilHealthy();
        }, delayMs);
    }

    private async startServer(): Promise<void> {
        this.manualStop = false;
        if (await this.client.isAlive()) { this.post({ type: 'toast', text: 'Server already running.' }); await this.probeServer(); return; }
        const root = this.harnessRoot();
        if (!root) {
            this.post({ type: 'toast', text: 'OmniHarness folder not found. Set omnisystem.harness.orchestratorPath.', error: true });
            return;
        }
        const py = this.cfg().get<string>('harness.pythonPath', 'python');
        const orchestrator = path.join(root, 'orchestrator');
        const port = new URL(this.serverUrl()).port || '8080';

        this.output.show(true);

        const depsOk = await this.ensureDependencies(py, orchestrator);
        if (!depsOk) { return; }
        if (this.manualStop) { return; }

        this.restartAttempts = 0;
        this.spawnServerProcess(py, orchestrator, port);
        const healthy = await this.pollUntilHealthy();
        if (healthy) {
            this.post({ type: 'toast', text: 'Orchestrator is up.' });
        } else if (!this.manualStop) {
            this.post({ type: 'toast', text: 'Server did not become healthy in time. Check the Output panel.', error: true });
        }
    }

    private stopServer(): void {
        this.manualStop = true;
        if (this.restartTimer) { clearTimeout(this.restartTimer); this.restartTimer = undefined; }
        this.restartAttempts = 0;
        if (this.serverProc && !this.serverProc.killed) {
            this.serverProc.kill();
            this.log('[OmniHarness] Stopped orchestrator.');
        }
        this.serverProc = undefined;
        this.post({ type: 'serverStatus', alive: false });
    }

    public dispose(): void {
        if (this.healthTimer) { clearInterval(this.healthTimer); this.healthTimer = undefined; }
        if (this.restartTimer) { clearTimeout(this.restartTimer); this.restartTimer = undefined; }
        this.stopServer();
        this.mcp.dispose();
    }

    // ── HTML ─────────────────────────────────────────────────────────────────

    private getHtml(webview: vscode.Webview): string {
        const nonce = getNonce();
        const uri = (...p: string[]) => webview.asWebviewUri(vscode.Uri.joinPath(this.ctx.extensionUri, ...p)).toString();
        const cssUri = uri('media', 'harness', 'harness.css');
        const jsUri = uri('media', 'harness', 'harness.js');
        const widgetCss = uri('media', 'omni-widgets.css');

        return /* html */`<!DOCTYPE html>
<html lang="en" data-theme="omni-dark">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <meta http-equiv="Content-Security-Policy" content="default-src 'none'; img-src ${webview.cspSource} https: data:; style-src ${webview.cspSource} 'unsafe-inline'; script-src 'nonce-${nonce}'; connect-src 'none';">
  <link rel="stylesheet" href="${widgetCss}">
  <link rel="stylesheet" href="${cssUri}">
  <title>OmniHarness AI</title>
</head>
<body>
  <div id="app"></div>
  <script nonce="${nonce}" src="${jsUri}"></script>
</body>
</html>`;
    }
}
