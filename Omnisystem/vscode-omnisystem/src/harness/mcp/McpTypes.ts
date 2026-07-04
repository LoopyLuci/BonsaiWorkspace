// MCP (Model Context Protocol) shared types.
// Protocol: JSON-RPC 2.0. Spec: https://modelcontextprotocol.io
// Transports supported: stdio (spawned subprocess) and Streamable HTTP.

export const MCP_PROTOCOL_VERSION = '2024-11-05';

export type McpTransportKind = 'stdio' | 'http';

export interface McpServerConfig {
    id: string;
    name: string;
    transport: McpTransportKind;
    enabled: boolean;
    // stdio
    command?: string;
    args?: string[];
    env?: Record<string, string>;
    cwd?: string;
    // http
    url?: string;
    headers?: Record<string, string>;
}

export interface McpToolSchema {
    name: string;
    description?: string;
    inputSchema?: Record<string, unknown>;   // JSON Schema
}

/** A tool discovered from an MCP server, namespaced for the agent's toolset. */
export interface McpAggregatedTool {
    /** Namespaced id exposed to the model: `mcp__<serverId>__<tool>`. */
    qualifiedName: string;
    serverId: string;
    serverName: string;
    tool: McpToolSchema;
}

export interface JsonRpcRequest {
    jsonrpc: '2.0';
    id: number | string;
    method: string;
    params?: unknown;
}

export interface JsonRpcNotification {
    jsonrpc: '2.0';
    method: string;
    params?: unknown;
}

export interface JsonRpcResponse {
    jsonrpc: '2.0';
    id: number | string;
    result?: unknown;
    error?: { code: number; message: string; data?: unknown };
}

export interface McpCallToolResult {
    content?: Array<{ type: string; text?: string; [k: string]: unknown }>;
    isError?: boolean;
    [k: string]: unknown;
}

/** The MCP tool-call namespace prefix used to route to an MCP server. */
export const MCP_PREFIX = 'mcp__';

export function qualifyToolName(serverId: string, tool: string): string {
    return `${MCP_PREFIX}${serverId}__${tool}`;
}

/** Parse `mcp__<serverId>__<tool>` → { serverId, tool } (tool may contain `__`? no). */
export function parseQualifiedName(qualified: string): { serverId: string; tool: string } | undefined {
    if (!qualified.startsWith(MCP_PREFIX)) { return undefined; }
    const rest = qualified.slice(MCP_PREFIX.length);
    const sep = rest.indexOf('__');
    if (sep === -1) { return undefined; }
    return { serverId: rest.slice(0, sep), tool: rest.slice(sep + 2) };
}
