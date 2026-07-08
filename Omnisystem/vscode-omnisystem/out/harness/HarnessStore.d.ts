import * as vscode from 'vscode';
import { McpServerConfig } from './mcp/McpTypes';
import { HarnessMessage } from './OmniHarnessClient';
export type ProviderKind = 'api' | 'local';
export interface ProviderDef {
    id: string;
    label: string;
    kind: ProviderKind;
    envVar: string;
    keyUrl?: string;
    baseUrlEnv?: string;
    defaultBaseUrl?: string;
    example?: string;
    /** Extra candidate base URLs to probe for this provider (local providers only). */
    probeUrls?: string[];
}
/** A locally-detected model, extracted from a real probe response — no invented fields. */
export interface LocalDetectedModel {
    id: string;
    sizeBytes?: number;
    family?: string;
    parameterSize?: string;
    quantization?: string;
}
/** Result of probing one local backend (Ollama or an OpenAI-compatible server). */
export interface LocalProviderProbeResult {
    providerId: string;
    detected: boolean;
    baseUrl?: string;
    models: LocalDetectedModel[];
    backendLabel?: string;
}
export interface AgentPreset {
    id: string;
    name: string;
    description: string;
    systemPrompt: string;
    model: string;
    temperature: number;
    maxTokens: number;
    tools: string[] | ['*'];
    autoApprove: boolean;
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
    diff?: {
        relPath: string;
        before: string;
        after: string;
    } | null;
    checkpointId?: string;
    ts: number;
}
export interface ChatSession {
    id: string;
    title: string;
    model: string;
    agentId: string;
    createdAt: number;
    updatedAt: number;
    messages: HarnessMessage[];
    turns: ChatTurn[];
    contextSummary?: string;
    contextSummaryUpToIndex?: number;
}
/** Captured pre-edit file state so a mutating tool call can be undone. */
export interface Checkpoint {
    id: string;
    sessionId: string;
    path: string;
    before: string | null;
    ts: number;
}
export interface ServerProfile {
    id: string;
    name: string;
    url: string;
}
export declare const KNOWN_PROVIDERS: ProviderDef[];
export declare class HarnessStore {
    private readonly ctx;
    constructor(ctx: vscode.ExtensionContext);
    /** Last real probe results, kept for synchronous reads (e.g. writeEnvFile) between refreshes. */
    private localProbeCache;
    listProviders(): ProviderDef[];
    private static localOverrideKey;
    /** A user-typed "advanced/custom" base URL for a local provider, if they've set one. */
    getLocalBaseUrlOverride(id: string): string | undefined;
    setLocalBaseUrlOverride(id: string, url: string): Promise<void>;
    /**
     * Real network auto-detection for local model runtimes: HTTP-probes Ollama's
     * `/api/tags` and a short list of common OpenAI-compatible ports (LM Studio,
     * llama.cpp) with a short timeout, in parallel. A user-set override URL (if
     * any) is tried first and preferred over the guessed defaults. No state is
     * faked — a backend only reports `detected: true` if it actually answered.
     */
    probeLocalProviders(timeoutMs?: number): Promise<LocalProviderProbeResult[]>;
    /** The last known probe results without re-probing the network (may be empty before the first probe). */
    getLastLocalProbe(): LocalProviderProbeResult[];
    getKey(providerId: string): Promise<string | undefined>;
    hasKey(providerId: string): Promise<boolean>;
    setKey(providerId: string, key: string): Promise<void>;
    /** Snapshot of which providers currently have a key configured. */
    providerStatus(): Promise<Array<ProviderDef & {
        configured: boolean;
        localModels?: LocalDetectedModel[];
        localBackendLabel?: string;
    }>>;
    /**
     * Write all configured keys into an `.env` file at the OmniHarness root so the
     * orchestrator picks them up on next (re)start. Returns the path written.
     */
    writeEnvFile(harnessRoot: string): Promise<string>;
    getAgents(): AgentPreset[];
    getAgent(id: string): AgentPreset | undefined;
    saveAgent(agent: AgentPreset): Promise<void>;
    deleteAgent(id: string): Promise<void>;
    getActiveModel(): string;
    setActiveModel(model: string): Promise<void>;
    getActiveAgentId(): string;
    setActiveAgentId(id: string): Promise<void>;
    getMcpServers(): McpServerConfig[];
    saveMcpServer(server: McpServerConfig): Promise<void>;
    deleteMcpServer(id: string): Promise<void>;
    setMcpEnabled(id: string, enabled: boolean): Promise<void>;
    listSessions(): ChatSession[];
    getSession(id: string): ChatSession | undefined;
    createSession(model: string, agentId: string): Promise<ChatSession>;
    /** Persist a session's transcript/messages (called after each turn). Auto-titles from the first user message. */
    saveSession(session: ChatSession): Promise<void>;
    renameSession(id: string, title: string): Promise<void>;
    deleteSession(id: string): Promise<void>;
    getActiveSessionId(): string;
    setActiveSessionId(id: string): Promise<void>;
    saveCheckpoint(cp: Checkpoint): Promise<void>;
    getCheckpoint(id: string): Checkpoint | undefined;
    deleteCheckpoint(id: string): Promise<void>;
    listServerProfiles(): ServerProfile[];
    saveServerProfile(profile: ServerProfile): Promise<void>;
    deleteServerProfile(id: string): Promise<void>;
    getActiveServerProfileId(): string;
    setActiveServerProfileId(id: string): Promise<void>;
    getFavoriteModels(): string[];
    toggleFavoriteModel(id: string): Promise<string[]>;
}
//# sourceMappingURL=HarnessStore.d.ts.map