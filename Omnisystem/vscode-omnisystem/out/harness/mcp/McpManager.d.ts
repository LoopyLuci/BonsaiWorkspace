import { McpServerConfig, McpAggregatedTool } from './McpTypes';
export interface McpServerStatus {
    id: string;
    name: string;
    transport: string;
    connected: boolean;
    toolCount: number;
    error?: string;
}
export interface McpToolResult {
    ok: boolean;
    summary: string;
    content: string;
}
export declare class McpManager {
    private clients;
    private log;
    constructor(log?: (line: string) => void);
    /** (Re)connect to the given set of enabled servers. Disconnects removed ones. */
    sync(configs: McpServerConfig[]): Promise<void>;
    /** All tools across all connected servers, namespaced for the model. */
    aggregatedTools(): McpAggregatedTool[];
    /** True if the given tool name is an MCP-namespaced tool. */
    static isMcpTool(name: string): boolean;
    /** Call an aggregated MCP tool by its qualified name. */
    callTool(qualifiedName: string, args: Record<string, unknown>): Promise<McpToolResult>;
    private renderResult;
    status(configs: McpServerConfig[]): McpServerStatus[];
    dispose(): void;
}
//# sourceMappingURL=McpManager.d.ts.map