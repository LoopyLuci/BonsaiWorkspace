import { McpServerConfig, McpToolSchema, McpCallToolResult } from './McpTypes';
export declare class McpClient {
    readonly cfg: McpServerConfig;
    private transport;
    private pending;
    private nextId;
    private started;
    serverInfo: {
        name?: string;
        version?: string;
    };
    tools: McpToolSchema[];
    lastError?: string;
    constructor(cfg: McpServerConfig);
    private handleMessage;
    private onClose;
    private request;
    private notify;
    /** Connect, perform the handshake, and discover tools. */
    connect(): Promise<void>;
    refreshTools(): Promise<McpToolSchema[]>;
    callTool(name: string, args: Record<string, unknown>): Promise<McpCallToolResult>;
    isConnected(): boolean;
    close(): void;
}
//# sourceMappingURL=McpClient.d.ts.map