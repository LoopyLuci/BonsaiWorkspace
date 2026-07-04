export declare const MCP_PROTOCOL_VERSION = "2024-11-05";
export type McpTransportKind = 'stdio' | 'http';
export interface McpServerConfig {
    id: string;
    name: string;
    transport: McpTransportKind;
    enabled: boolean;
    command?: string;
    args?: string[];
    env?: Record<string, string>;
    cwd?: string;
    url?: string;
    headers?: Record<string, string>;
}
export interface McpToolSchema {
    name: string;
    description?: string;
    inputSchema?: Record<string, unknown>;
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
    error?: {
        code: number;
        message: string;
        data?: unknown;
    };
}
export interface McpCallToolResult {
    content?: Array<{
        type: string;
        text?: string;
        [k: string]: unknown;
    }>;
    isError?: boolean;
    [k: string]: unknown;
}
/** The MCP tool-call namespace prefix used to route to an MCP server. */
export declare const MCP_PREFIX = "mcp__";
export declare function qualifyToolName(serverId: string, tool: string): string;
/** Parse `mcp__<serverId>__<tool>` → { serverId, tool } (tool may contain `__`? no). */
export declare function parseQualifiedName(qualified: string): {
    serverId: string;
    tool: string;
} | undefined;
//# sourceMappingURL=McpTypes.d.ts.map