// OmniHarnessClient — typed REST/SSE client for the OmniHarness orchestrator.
//
// The orchestrator (FastAPI, default http://localhost:8080) exposes:
//   GET  /api/health
//   GET  /api/models
//   POST /api/chat
//   POST /api/chat/stream      (Server-Sent Events)
//   POST /api/agent/run
//   POST /api/memory/store | /api/memory/search
// This client wraps those endpoints so the extension can drive any model —
// local (Ollama) or API (Anthropic, OpenAI, Google, Groq, Mistral, Cohere,
// OpenRouter, Together, Fireworks) — through a single gateway.

export interface ToolCall {
    id: string;
    name: string;
    arguments: Record<string, unknown>;
}

export interface FunctionSchema {
    name: string;
    description: string;
    parameters: Record<string, unknown>;   // JSON Schema
}

export interface HarnessMessage {
    role: 'system' | 'user' | 'assistant' | 'tool';
    content: string;
    tool_call_id?: string;                  // for role:'tool'
    tool_calls?: ToolCall[];                // for role:'assistant' that requested tools
}

export interface ChatOptions {
    model_id: string;
    messages: HarnessMessage[];
    system?: string;
    temperature?: number;
    max_tokens?: number;
    session_id?: string;
    tools?: FunctionSchema[];               // native function calling
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

export class HarnessConnectionError extends Error {
    constructor(message: string, public readonly url: string) {
        super(message);
        this.name = 'HarnessConnectionError';
    }
}

export class OmniHarnessClient {
    constructor(private baseUrl: string) {
        this.baseUrl = baseUrl.replace(/\/+$/, '');
    }

    setBaseUrl(url: string): void {
        this.baseUrl = url.replace(/\/+$/, '');
    }

    getBaseUrl(): string {
        return this.baseUrl;
    }

    private url(path: string): string {
        return `${this.baseUrl}${path}`;
    }

    private async request<T>(
        path: string,
        init?: RequestInit,
        timeoutMs = 20000,
    ): Promise<T> {
        const controller = new AbortController();
        const timer = setTimeout(() => controller.abort(), timeoutMs);
        try {
            const resp = await fetch(this.url(path), {
                ...init,
                signal: controller.signal,
                headers: { 'Content-Type': 'application/json', ...(init?.headers ?? {}) },
            });
            if (!resp.ok) {
                const body = await resp.text().catch(() => '');
                throw new Error(`HTTP ${resp.status} ${resp.statusText} — ${body.slice(0, 400)}`);
            }
            return (await resp.json()) as T;
        } catch (err) {
            if (err instanceof Error && (err.name === 'AbortError' || /fetch failed|ECONNREFUSED|ENOTFOUND/i.test(err.message))) {
                throw new HarnessConnectionError(
                    `Cannot reach OmniHarness orchestrator at ${this.baseUrl}. Is the server running?`,
                    this.baseUrl,
                );
            }
            throw err;
        } finally {
            clearTimeout(timer);
        }
    }

    /** Quick reachability + provider health probe. */
    async health(): Promise<HealthResult> {
        return this.request<HealthResult>('/api/health', { method: 'GET' }, 6000);
    }

    /** True if the orchestrator responds to /api/health at all. */
    async isAlive(): Promise<boolean> {
        try {
            await this.health();
            return true;
        } catch {
            return false;
        }
    }

    async listModels(provider?: string): Promise<ModelDescriptor[]> {
        const q = provider ? `?provider=${encodeURIComponent(provider)}` : '';
        const data = await this.request<{ models: ModelDescriptor[] }>(`/api/models${q}`, { method: 'GET' }, 10000);
        return data.models ?? [];
    }

    /** Non-streaming chat completion. Supports native function calling via `tools`. */
    async chat(opts: ChatOptions): Promise<ChatResult> {
        const res = await this.request<Partial<ChatResult>>('/api/chat', {
            method: 'POST',
            body: JSON.stringify({
                model_id: opts.model_id,
                messages: opts.messages,
                system: opts.system,
                temperature: opts.temperature ?? 0.7,
                max_tokens: opts.max_tokens ?? 4096,
                session_id: opts.session_id,
                tools: opts.tools,
                stream: false,
            }),
        }, 180000);
        return {
            content: res.content ?? '',
            model_used: res.model_used ?? opts.model_id,
            finish_reason: res.finish_reason ?? 'stop',
            input_tokens: res.input_tokens ?? 0,
            output_tokens: res.output_tokens ?? 0,
            latency_ms: res.latency_ms ?? 0,
            tool_calls: res.tool_calls ?? [],
        };
    }

    /**
     * Streaming chat completion. Invokes `onDelta` for each token chunk.
     * Resolves with the fully accumulated text. Honours `signal` for cancellation.
     */
    async chatStream(
        opts: ChatOptions,
        onDelta: (delta: string) => void,
        signal?: AbortSignal,
    ): Promise<string> {
        let resp: Response;
        try {
            resp = await fetch(this.url('/api/chat/stream'), {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    model_id: opts.model_id,
                    messages: opts.messages,
                    system: opts.system,
                    temperature: opts.temperature ?? 0.7,
                    max_tokens: opts.max_tokens ?? 4096,
                    session_id: opts.session_id,
                    stream: true,
                }),
                signal,
            });
        } catch (err) {
            throw new HarnessConnectionError(
                `Cannot reach OmniHarness orchestrator at ${this.baseUrl}. Is the server running?`,
                this.baseUrl,
            );
        }

        if (!resp.ok) {
            const body = await resp.text().catch(() => '');
            throw new Error(`HTTP ${resp.status} ${resp.statusText} — ${body.slice(0, 400)}`);
        }
        if (!resp.body) {
            throw new Error('Streaming response had no body.');
        }

        const reader = resp.body.getReader();
        const decoder = new TextDecoder();
        let buffer = '';
        let full = '';

        // SSE frames are separated by a blank line. Each frame has `data:` lines.
        for (;;) {
            const { done, value } = await reader.read();
            if (done) { break; }
            buffer += decoder.decode(value, { stream: true });

            let sep: number;
            while ((sep = buffer.indexOf('\n\n')) !== -1) {
                const frame = buffer.slice(0, sep);
                buffer = buffer.slice(sep + 2);
                for (const line of frame.split('\n')) {
                    const trimmed = line.trimStart();
                    if (!trimmed.startsWith('data:')) { continue; }
                    const payload = trimmed.slice(5).trim();
                    if (payload === '[DONE]') { return full; }
                    if (!payload) { continue; }
                    try {
                        const obj = JSON.parse(payload) as { delta?: string; error?: string };
                        if (obj.error) { throw new Error(obj.error); }
                        if (obj.delta) {
                            full += obj.delta;
                            onDelta(obj.delta);
                        }
                    } catch (e) {
                        // Non-JSON keepalive/comment line — ignore.
                        if (e instanceof Error && e.message !== payload) { /* swallow parse noise */ }
                    }
                }
            }
        }
        return full;
    }

    async memoryStore(collection: string, content: string, metadata: Record<string, unknown> = {}): Promise<void> {
        await this.request('/api/memory/store', {
            method: 'POST',
            body: JSON.stringify({ collection, content, metadata }),
        });
    }

    async memorySearch(collection: string, query: string, topK = 5): Promise<Array<{ content: string; score: number }>> {
        const data = await this.request<{ results: Array<{ content: string; score: number }> }>(
            '/api/memory/search',
            { method: 'POST', body: JSON.stringify({ collection, query, top_k: topK }) },
        );
        return data.results ?? [];
    }

    // ── Substrate: swarm / ensemble / RAG / distillation ─────────────────────

    async swarm(body: {
        topology: string; task: string; agents: Array<Record<string, unknown>>;
        rounds?: number; budget?: Record<string, unknown>; policy?: Record<string, unknown>;
    }): Promise<{ output: string; topology: string; steps: unknown[]; governance: Record<string, unknown> }> {
        return this.request('/api/swarm/run', { method: 'POST', body: JSON.stringify(body) }, 600000);
    }

    async ensemble(body: {
        prompt: string; models: string[]; system?: string; strategy?: string; judge_model?: string;
        budget?: Record<string, unknown>; policy?: Record<string, unknown>;
    }): Promise<{ answers: Record<string, string>; final: string; strategy: string; governance: Record<string, unknown> }> {
        return this.request('/api/ensemble/run', { method: 'POST', body: JSON.stringify(body) }, 600000);
    }

    async ragIngest(docId: string, text: string, metadata: Record<string, unknown> = {}): Promise<{ chunks_added: number }> {
        return this.request('/api/rag/ingest', { method: 'POST', body: JSON.stringify({ doc_id: docId, text, metadata }) });
    }

    async ragQuery(query: string, k = 5): Promise<{ results: Array<{ doc_id: string; text: string; score: number }> }> {
        return this.request('/api/rag/query', { method: 'POST', body: JSON.stringify({ query, k }) });
    }

    async distill(body: {
        prompts: string[]; teachers: string[]; judge_model?: string; system?: string;
        backend?: string; base_model?: string;
    }): Promise<{ records: number; dataset_jsonl: string; training_config?: Record<string, unknown> }> {
        return this.request('/api/distill/build', { method: 'POST', body: JSON.stringify(body) }, 600000);
    }
}
