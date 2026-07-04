"use strict";
// McpClient — a single connection to one MCP server (stdio or Streamable HTTP).
// Implements the JSON-RPC 2.0 handshake: initialize → notifications/initialized →
// tools/list → tools/call. Kept dependency-free (Node stdio + global fetch).
Object.defineProperty(exports, "__esModule", { value: true });
exports.McpClient = void 0;
const child_process_1 = require("child_process");
const McpTypes_1 = require("./McpTypes");
// ── stdio transport (newline-delimited JSON) ────────────────────────────────
class StdioTransport {
    constructor(cfg) {
        this.cfg = cfg;
        this.buffer = '';
        this.msgCb = () => { };
        this.closeCb = () => { };
    }
    async start() {
        if (!this.cfg.command) {
            throw new Error('stdio MCP server requires a "command".');
        }
        this.proc = (0, child_process_1.spawn)(this.cfg.command, this.cfg.args ?? [], {
            cwd: this.cfg.cwd,
            env: { ...process.env, ...(this.cfg.env ?? {}) },
            shell: true,
            stdio: ['pipe', 'pipe', 'pipe'],
        });
        this.proc.stdout?.on('data', (d) => this.onData(d.toString()));
        this.proc.stderr?.on('data', () => { });
        this.proc.on('error', (e) => this.closeCb(`spawn error: ${e.message}`));
        this.proc.on('close', (code) => this.closeCb(`process exited (${code})`));
    }
    onData(chunk) {
        this.buffer += chunk;
        let nl;
        while ((nl = this.buffer.indexOf('\n')) !== -1) {
            const line = this.buffer.slice(0, nl).trim();
            this.buffer = this.buffer.slice(nl + 1);
            if (!line) {
                continue;
            }
            try {
                this.msgCb(JSON.parse(line));
            }
            catch { /* ignore non-JSON log line */ }
        }
    }
    async send(payload) {
        if (!this.proc?.stdin) {
            throw new Error('stdio transport not started');
        }
        this.proc.stdin.write(JSON.stringify(payload) + '\n');
    }
    onMessage(cb) { this.msgCb = cb; }
    onClose(cb) { this.closeCb = cb; }
    close() { if (this.proc && !this.proc.killed) {
        this.proc.kill();
    } }
}
// ── Streamable HTTP transport ───────────────────────────────────────────────
class HttpTransport {
    constructor(cfg) {
        this.cfg = cfg;
        this.msgCb = () => { };
        this.closeCb = () => { };
    }
    async start() {
        if (!this.cfg.url) {
            throw new Error('http MCP server requires a "url".');
        }
    }
    async send(payload) {
        const headers = {
            'Content-Type': 'application/json',
            'Accept': 'application/json, text/event-stream',
            ...(this.cfg.headers ?? {}),
        };
        if (this.sessionId) {
            headers['Mcp-Session-Id'] = this.sessionId;
        }
        const resp = await fetch(this.cfg.url, { method: 'POST', headers, body: JSON.stringify(payload) });
        const sid = resp.headers.get('mcp-session-id');
        if (sid) {
            this.sessionId = sid;
        }
        // Notifications (no id) may return 202 with no body.
        if (resp.status === 202) {
            return;
        }
        if (!resp.ok) {
            throw new Error(`HTTP ${resp.status} ${resp.statusText}`);
        }
        const ctype = resp.headers.get('content-type') ?? '';
        const text = await resp.text();
        if (ctype.includes('text/event-stream')) {
            for (const frame of text.split('\n\n')) {
                for (const line of frame.split('\n')) {
                    const t = line.trimStart();
                    if (!t.startsWith('data:')) {
                        continue;
                    }
                    const data = t.slice(5).trim();
                    if (!data || data === '[DONE]') {
                        continue;
                    }
                    try {
                        this.msgCb(JSON.parse(data));
                    }
                    catch { /* ignore */ }
                }
            }
        }
        else if (text.trim()) {
            try {
                this.msgCb(JSON.parse(text));
            }
            catch { /* ignore */ }
        }
    }
    onMessage(cb) { this.msgCb = cb; }
    onClose(cb) { this.closeCb = cb; }
    close() { this.closeCb('closed'); }
}
class McpClient {
    constructor(cfg) {
        this.cfg = cfg;
        this.pending = new Map();
        this.nextId = 1;
        this.started = false;
        this.serverInfo = {};
        this.tools = [];
        this.transport = cfg.transport === 'http' ? new HttpTransport(cfg) : new StdioTransport(cfg);
        this.transport.onMessage((msg) => this.handleMessage(msg));
        this.transport.onClose((reason) => this.onClose(reason));
    }
    handleMessage(msg) {
        // Response to one of our requests.
        if (msg.id !== undefined && (msg.result !== undefined || msg.error !== undefined)) {
            const p = this.pending.get(Number(msg.id));
            if (p) {
                clearTimeout(p.timer);
                this.pending.delete(Number(msg.id));
                if (msg.error) {
                    p.reject(new Error(msg.error.message || 'MCP error'));
                }
                else {
                    p.resolve(msg.result);
                }
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
    onClose(reason) {
        this.lastError = reason;
        for (const [, p] of this.pending) {
            clearTimeout(p.timer);
            p.reject(new Error(`MCP server closed: ${reason}`));
        }
        this.pending.clear();
        this.started = false;
    }
    request(method, params, timeoutMs = 30000) {
        const id = this.nextId++;
        const payload = { jsonrpc: '2.0', id, method, params };
        return new Promise((resolve, reject) => {
            const timer = setTimeout(() => {
                this.pending.delete(id);
                reject(new Error(`MCP request "${method}" timed out after ${timeoutMs}ms`));
            }, timeoutMs);
            this.pending.set(id, { resolve, reject, timer });
            this.transport.send(payload).catch((e) => {
                clearTimeout(timer);
                this.pending.delete(id);
                reject(e);
            });
        });
    }
    notify(method, params) {
        return this.transport.send({ jsonrpc: '2.0', method, params });
    }
    /** Connect, perform the handshake, and discover tools. */
    async connect() {
        await this.transport.start();
        const init = await this.request('initialize', {
            protocolVersion: McpTypes_1.MCP_PROTOCOL_VERSION,
            capabilities: { tools: {} },
            clientInfo: { name: 'OmniHarness', version: '1.0.0' },
        }, 20000);
        this.serverInfo = init?.serverInfo ?? {};
        await this.notify('notifications/initialized');
        this.started = true;
        await this.refreshTools();
    }
    async refreshTools() {
        const res = await this.request('tools/list', {});
        this.tools = res?.tools ?? [];
        return this.tools;
    }
    async callTool(name, args) {
        const res = await this.request('tools/call', { name, arguments: args ?? {} }, 120000);
        return (res ?? {});
    }
    isConnected() { return this.started; }
    close() {
        this.transport.close();
        this.onClose('client closed');
    }
}
exports.McpClient = McpClient;
//# sourceMappingURL=McpClient.js.map