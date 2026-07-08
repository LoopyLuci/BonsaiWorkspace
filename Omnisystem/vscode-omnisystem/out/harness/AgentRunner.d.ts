import { OmniHarnessClient, HarnessMessage, FunctionSchema } from './OmniHarnessClient';
import { VscodeTools, ToolDiff, ToolCheckpoint } from './VscodeTools';
import { AgentPreset } from './HarnessStore';
/** What a tool execution reports back to the agent loop's UI events. */
export interface ExecOutcome {
    ok: boolean;
    summary: string;
    content: string;
    diff?: ToolDiff;
    checkpoint?: ToolCheckpoint;
}
/**
 * External tools contributed to the agent from outside VS Code — currently MCP
 * servers. Their tools are merged into both the native and text tool loops.
 */
export interface ExternalTools {
    functionSchemas(): FunctionSchema[];
    describe(): string;
    has(name: string): boolean;
    execute(name: string, args: Record<string, unknown>): Promise<{
        ok: boolean;
        summary: string;
        content: string;
    }>;
}
export interface AgentEvents {
    onAssistantDelta: (text: string) => void;
    onAssistantDone: (fullText: string) => void;
    onToolCall: (id: string, tool: string, args: Record<string, unknown>) => void;
    onToolResult: (id: string, outcome: ExecOutcome) => void;
    onStatus: (text: string) => void;
    onFinal: (answer: string) => void;
    onError: (message: string) => void;
}
export declare class AgentRunner {
    private readonly client;
    private readonly tools;
    private readonly events;
    private readonly external?;
    private aborted;
    constructor(client: OmniHarnessClient, tools: VscodeTools, events: AgentEvents, external?: ExternalTools | undefined);
    abort(): void;
    /** MCP tools are offered when the agent allows all tools (tools = ['*']). */
    private externalEnabled;
    /** Dispatch a tool call to the built-in VS Code tools or an external (MCP) tool. */
    private execTool;
    private buildSystemPrompt;
    private allowedTools;
    private parseAction;
    /**
     * Run the agent loop. `history` is the prior conversation; `userText` is the
     * new user message. Streams assistant output and executes tool calls until the
     * model emits a final answer or `maxSteps` is reached.
     */
    run(agent: AgentPreset, model: string, history: HarnessMessage[], userText: string, maxSteps?: number): Promise<void>;
    /**
     * Native function-calling loop. Sends tool schemas to the provider and acts on
     * the structured tool_calls it returns. Non-streaming per step (the model
     * returns tool_calls as data, not tokens), but shares the same UI events.
     */
    runNative(agent: AgentPreset, model: string, history: HarnessMessage[], userText: string, maxSteps?: number): Promise<void>;
    private buildNativeSystemPrompt;
    /** Simple non-agentic single-turn streaming chat (for the "Ask" quick path). */
    chatOnce(model: string, system: string, history: HarnessMessage[], userText: string, temperature: number, maxTokens: number): Promise<void>;
}
//# sourceMappingURL=AgentRunner.d.ts.map