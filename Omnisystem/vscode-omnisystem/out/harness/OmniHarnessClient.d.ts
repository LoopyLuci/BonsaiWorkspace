export interface ToolCall {
    id: string;
    name: string;
    arguments: Record<string, unknown>;
}
export interface FunctionSchema {
    name: string;
    description: string;
    parameters: Record<string, unknown>;
}
export interface HarnessMessage {
    role: 'system' | 'user' | 'assistant' | 'tool';
    content: string;
    tool_call_id?: string;
    tool_calls?: ToolCall[];
}
export interface ChatOptions {
    model_id: string;
    messages: HarnessMessage[];
    system?: string;
    temperature?: number;
    max_tokens?: number;
    session_id?: string;
    tools?: FunctionSchema[];
}
export interface ChatResult {
    content: string;
    model_used: string;
    finish_reason: string;
    input_tokens: number;
    output_tokens: number;
    latency_ms: number;
    tool_calls: ToolCall[];
}
export interface ModelDescriptor {
    id: string;
    provider: string;
    name?: string;
    context_window?: number;
    supports_tools?: boolean;
    supports_vision?: boolean;
    description?: string;
    [key: string]: unknown;
}
export interface HealthResult {
    status: string;
    providers: Record<string, boolean>;
    kernel: boolean;
    version: string;
}
export declare class HarnessConnectionError extends Error {
    readonly url: string;
    constructor(message: string, url: string);
}
export declare class OmniHarnessClient {
    private baseUrl;
    constructor(baseUrl: string);
    setBaseUrl(url: string): void;
    getBaseUrl(): string;
    private url;
    private request;
    /** Quick reachability + provider health probe. */
    health(): Promise<HealthResult>;
    /** True if the orchestrator responds to /api/health at all. */
    isAlive(): Promise<boolean>;
    listModels(provider?: string): Promise<ModelDescriptor[]>;
    /** Non-streaming chat completion. Supports native function calling via `tools`. */
    chat(opts: ChatOptions): Promise<ChatResult>;
    /**
     * Streaming chat completion. Invokes `onDelta` for each token chunk.
     * Resolves with the fully accumulated text. Honours `signal` for cancellation.
     */
    chatStream(opts: ChatOptions, onDelta: (delta: string) => void, signal?: AbortSignal): Promise<string>;
    memoryStore(collection: string, content: string, metadata?: Record<string, unknown>): Promise<void>;
    memorySearch(collection: string, query: string, topK?: number): Promise<Array<{
        content: string;
        score: number;
    }>>;
    swarm(body: {
        topology: string;
        task: string;
        agents: Array<Record<string, unknown>>;
        rounds?: number;
        budget?: Record<string, unknown>;
        policy?: Record<string, unknown>;
    }): Promise<{
        output: string;
        topology: string;
        steps: unknown[];
        governance: Record<string, unknown>;
    }>;
    ensemble(body: {
        prompt: string;
        models: string[];
        system?: string;
        strategy?: string;
        judge_model?: string;
        budget?: Record<string, unknown>;
        policy?: Record<string, unknown>;
    }): Promise<{
        answers: Record<string, string>;
        final: string;
        strategy: string;
        governance: Record<string, unknown>;
    }>;
    ragIngest(docId: string, text: string, metadata?: Record<string, unknown>): Promise<{
        chunks_added: number;
    }>;
    ragQuery(query: string, k?: number): Promise<{
        results: Array<{
            doc_id: string;
            text: string;
            score: number;
        }>;
    }>;
    distill(body: {
        prompts: string[];
        teachers: string[];
        judge_model?: string;
        system?: string;
        backend?: string;
        base_model?: string;
    }): Promise<{
        records: number;
        dataset_jsonl: string;
        training_config?: Record<string, unknown>;
    }>;
}
//# sourceMappingURL=OmniHarnessClient.d.ts.map