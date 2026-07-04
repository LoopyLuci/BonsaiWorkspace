"use strict";
// McpManager — manages all configured MCP servers for the OmniHarness agent.
// Connects them, aggregates their tools under a `mcp__<server>__<tool>` namespace,
// and dispatches tool calls to the right server. The aggregated tools are merged
// into the agent's toolset alongside the built-in VS Code tools.
Object.defineProperty(exports, "__esModule", { value: true });
exports.McpManager = void 0;
const McpClient_1 = require("./McpClient");
const McpTypes_1 = require("./McpTypes");
class McpManager {
    constructor(log) {
        this.clients = new Map();
        this.log = log ?? (() => { });
    }
    /** (Re)connect to the given set of enabled servers. Disconnects removed ones. */
    async sync(configs) {
        const enabled = configs.filter((c) => c.enabled);
        const wanted = new Set(enabled.map((c) => c.id));
        // Disconnect servers no longer wanted.
        for (const [id, client] of [...this.clients]) {
            if (!wanted.has(id)) {
                client.close();
                this.clients.delete(id);
                this.log(`[MCP] Disconnected ${id}`);
            }
        }
        // Connect new / not-yet-connected servers.
        await Promise.all(enabled.map(async (cfg) => {
            const existing = this.clients.get(cfg.id);
            if (existing && existing.isConnected()) {
                return;
            }
            const client = new McpClient_1.McpClient(cfg);
            this.clients.set(cfg.id, client);
            try {
                await client.connect();
                this.log(`[MCP] Connected ${cfg.name} (${cfg.id}) — ${client.tools.length} tool(s)`);
            }
            catch (err) {
                client.lastError = err instanceof Error ? err.message : String(err);
                this.log(`[MCP] Failed to connect ${cfg.name}: ${client.lastError}`);
            }
        }));
    }
    /** All tools across all connected servers, namespaced for the model. */
    aggregatedTools() {
        const out = [];
        for (const [id, client] of this.clients) {
            if (!client.isConnected()) {
                continue;
            }
            for (const tool of client.tools) {
                out.push({
                    qualifiedName: (0, McpTypes_1.qualifyToolName)(id, tool.name),
                    serverId: id,
                    serverName: client.cfg.name,
                    tool,
                });
            }
        }
        return out;
    }
    /** True if the given tool name is an MCP-namespaced tool. */
    static isMcpTool(name) {
        return (0, McpTypes_1.parseQualifiedName)(name) !== undefined;
    }
    /** Call an aggregated MCP tool by its qualified name. */
    async callTool(qualifiedName, args) {
        const parsed = (0, McpTypes_1.parseQualifiedName)(qualifiedName);
        if (!parsed) {
            return { ok: false, summary: 'not an MCP tool', content: `"${qualifiedName}" is not an MCP tool.` };
        }
        const client = this.clients.get(parsed.serverId);
        if (!client || !client.isConnected()) {
            return { ok: false, summary: `MCP server ${parsed.serverId} not connected`, content: `MCP server "${parsed.serverId}" is not connected.` };
        }
        try {
            const res = await client.callTool(parsed.tool, args);
            const text = this.renderResult(res);
            const ok = !res.isError;
            return { ok, summary: `${parsed.serverId}:${parsed.tool} → ${ok ? 'ok' : 'error'}`, content: text };
        }
        catch (err) {
            const msg = err instanceof Error ? err.message : String(err);
            return { ok: false, summary: `${parsed.serverId}:${parsed.tool} failed`, content: msg };
        }
    }
    renderResult(res) {
        if (!res.content || res.content.length === 0) {
            return res.isError ? '(tool reported an error)' : '(no content)';
        }
        const parts = [];
        for (const block of res.content) {
            if (block.type === 'text' && typeof block.text === 'string') {
                parts.push(block.text);
            }
            else {
                parts.push(JSON.stringify(block));
            }
        }
        return parts.join('\n');
    }
    status(configs) {
        return configs.map((cfg) => {
            const client = this.clients.get(cfg.id);
            return {
                id: cfg.id,
                name: cfg.name,
                transport: cfg.transport,
                connected: !!client?.isConnected(),
                toolCount: client?.isConnected() ? client.tools.length : 0,
                error: client?.lastError,
            };
        });
    }
    dispose() {
        for (const client of this.clients.values()) {
            client.close();
        }
        this.clients.clear();
    }
}
exports.McpManager = McpManager;
//# sourceMappingURL=McpManager.js.map