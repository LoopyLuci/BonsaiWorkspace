// McpClient — a single connection to one MCP server (stdio or Streamable HTTP).
// Implements the JSON-RPC 2.0 handshake: initialize → notifications/initialized →
// tools/list → tools/call. Kept dependency-free (Node stdio + global fetch).

import { spawn, ChildProcess } from 'child_process';
import {
    MCP_PROTOCOL_VERSION,
    McpServerConfig,
    McpToolSchema,
    McpCallToolResult,
    JsonRpcResponse,
} from './McpTypes';

type MessageHandler = (msg: JsonRpcResponse & { method?: string }) => void;

interface Transport {
    start(): Promise<void>;
    send(payload: object): Promise<void>;
    onMessage(cb: MessageHandler): void;
    onClose(cb: (reason: string) => void): void;
    close(): void;
}

// ── stdio transport (newline-delimited JSON) ────────────────────────────────

class StdioTransport implements Transport {
    private proc?: ChildProcess;
    private buffer = '';
    private msgCb: MessageHandler = () => {};
    private closeCb: (reason: string) => void = () => {};

    constructor(private readonly cfg: McpServerConfig) {}

    async start(): Promise<void> {
        if (!this.cfg.command) { throw new Error('stdio MCP server requires a "command".'); }
        this.proc = spawn(this.cfg.command, this.cfg.args ?? [], {
            cwd: this.cfg.cwd,
            env: { ...process.env, ...(this.cfg.env ?? {}) },
            shell: true,
            stdio: ['pipe', 'pipe', 'pipe'],
        });
        this.proc.stdout?.on('data', (d: Buffer) => this.onData(d.toString()));
        this.proc.stderr?.on('data', () => { /* server logs — ignored */ });
        this.proc.on('error', (e) => this.closeCb(`spawn error: ${e.message}`));
        this.proc.on('close', (code) => this.closeCb(`process exited (${code})`));
    }

    private onData(chunk: string): void {
        this.buffer += chunk;
        let nl: number;
        while ((nl = this.buffer.indexOf('\n')) !== -1) {
            const line = this.buffer.slice(0, nl).trim();
            this.buffer = this.buffer.slice(nl + 1);
            if (!line) { continue; }
            try { this.msgCb(JSON.parse(line)); } catch { /* ignore non-JSON log line */ }
        }
    }

    async send(payload: object): Promise<void> {
        if (!this.proc?.stdin) { throw new Error('stdio transport not started'); }
        this.proc.stdin.write(JSON.stringify(payload) + '\n');
    }

    onMessage(cb: MessageHandler): void { this.msgCb = cb; }
    onClose(cb: (reason: string) => void): void { this.closeCb = cb; }
    close(): void { if (this.proc && !this.proc.killed) { this.proc.kill(); } }
}

// ── Streamable HTTP transport ───────────────────────────────────────────────

class HttpTransport implements Transport {
    private msgCb: MessageHandler = () => {};
    private closeCb: (reason: string) => void = () => {};
    private sessionId?: string;

    constructor(private readonly cfg: McpServerConfig) {}

    async start(): Promise<void> {
        if (!this.cfg.url) { throw new Error('http MCP server requires a "url".'); }
    }

    async send(payload: object): Promise<void> {
        const headers: Record<string, string> = {
            'Content-Type': 'application/json',
            'Accept': 'application/json, text/event-stream',
            ...(this.cfg.headers ?? {}),
        };
        if (this.sessionId) { headers['Mcp-Session-Id'] = this.sessionId; }

        const resp = await fetch(this.cfg.url!, { method: 'POST', headers, body: JSON.stringify(payload) });
        const sid = resp.headers.get('mcp-session-id');
        if (sid) { this.sessionId = sid; }

        // Notifications (no id) may return 202 with no body.
        if (resp.status === 202) { return; }
        if (!resp.ok) { throw new Error(`HTTP ${resp.status} ${resp.statusText}`); }

        const ctype = resp.headers.get('content-type') ?? '';
        const text = await resp.text();
        if (ctype.includes('text/event-stream')) {
            for (const frame of text.split('\n\n')) {
                for (const line of frame.split('\n')) {
                    const t = line.trimStart();
                    if (!t.startsWith('data:')) { continue; }
                    const data = t.slice(5).trim();
                    if (!data || data === '[DONE]') { continue; }
                    try { this.msgCb(JSON.parse(data)); } catch { /* ignore */ }
                }
            }
        } else if (text.trim()) {
            try { this.msgCb(JSON.parse(text)); } catch { /* ignore */ }
        }
    }

    onMessage(cb: MessageHandler): void { this.msgCb = cb; }
    onClose(cb: (reason: string) => void): void { this.closeCb = cb; }
    close(): void { this.closeCb('closed'); }
}

// ── Client ──────────────────────────────────────────────────────────────────

interface Pending {
    resolve: (value: unknown) => void;
    reject: (err: Error) => void;
    timer: ReturnType<typeof setTimeout>;
}

export class McpClient {
    private transport: Transport;
    private pending = new Map<number, Pending>();
    private nextId = 1;
    private started = false;
    public serverInfo: { name?: string; version?: string } = {};
    public tools: McpToolSchema[] = [];
    public lastError?: string;

    constructor(public readonly cfg: McpServerConfig) {
        this.transport = cfg.transport === 'http' ? new HttpTransport(cfg) : new StdioTransport(cfg);
        this.transport.onMessage((msg) => this.handleMessage(msg));
        this.transport.onClose((reason) => this.onClose(reason));
    }

    private handleMessage(msg: JsonRpcResponse & { method?: string; id?: number | string }): void {
        // Response to one of our requests.
        if (msg.id !== undefined && (msg.result !== undefined || msg.error !== undefined)) {
            const p = this.pending.get(Number(msg.id));
            if (p) {
                clearTimeout(p.timer);
                this.pending.delete(Number(msg.id));
                if (msg.error) { p.reject(new Error(msg.error.message || 'MCP error')); }
                else { p.resolve(msg.result); }
            }
            return;
        }
        // A server→client request (e.g. sampling). We don't support these — nack
        // so the server doesn't hang waiting.
        if (msg.method && msg.id !== undefined) {
            void this.transport.send({
                jsonrpc: '2.0', id: msg.id,
                error: { code: -32601, message: 'Method not supported by OmniHarness MCP client' },
            });
        }
        // Notifications from the server are ignored.
    }

    private onClose(reason: string): void {
        this.lastError = reason;
        for (const [, p] of this.pending) { clearTimeout(p.timer); p.reject(new Error(`MCP server closed: ${reason}`)); }
        this.pending.clear();
        this.started = false;
    }

    private request(method: string, params: unknown, timeoutMs = 30000): Promise<unknown> {
        const id = this.nextId++;
        const payload = { jsonrpc: '2.0', id, method, params };
        return new Promise<unknown>((resolve, reject) => {
            const timer = setTimeout(() => {
                this.pending.delete(id);
                reject(new Error(`MCP request "${method}" timed out after ${timeoutMs}ms`));
            }, timeoutMs);
            this.pending.set(id, { resolve, reject, timer });
            this.transport.send(payload).catch((e) => {
                clearTimeout(timer); this.pending.delete(id); reject(e);
            });
        });
    }

    private notify(method: string, params?: unknown): Promise<void> {
        return this.transport.send({ jsonrpc: '2.0', method, params });
    }

    /** Connect, perform the handshake, and discover tools. */
    async connect(): Promise<void> {
        await this.transport.start();
        const init = await this.request('initialize', {
            protocolVersion: MCP_PROTOCOL_VERSION,
            capabilities: { tools: {} },
            clientInfo: { name: 'OmniHarness', version: '1.0.0' },
        }, 20000) as { serverInfo?: { name?: string; version?: string } };
        this.serverInfo = init?.serverInfo ?? {};
        await this.notify('notifications/initialized');
        this.started = true;
        await this.refreshTools();
    }

    async refreshTools(): Promise<McpToolSchema[]> {
        const res = await this.request('tools/list', {}) as { tools?: McpToolSchema[] };
        this.tools = res?.tools ?? [];
        return this.tools;
    }

    async callTool(name: string, args: Record<string, unknown>): Promise<McpCallToolResult> {
        const res = await this.request('tools/call', { name, arguments: args ?? {} }, 120000);
        return (res ?? {}) as McpCallToolResult;
    }

    isConnected(): boolean { return this.started; }

    close(): void {
        this.transport.close();
        this.onClose('client closed');
    }
}
