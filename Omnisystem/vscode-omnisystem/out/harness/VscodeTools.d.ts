export interface ToolParam {
    name: string;
    type: 'string' | 'number' | 'boolean';
    description: string;
    required: boolean;
}
export interface ToolDef {
    name: string;
    description: string;
    params: ToolParam[];
    mutating: boolean;
}
/** A displayable before/after pair for a mutating tool call. */
export interface ToolDiff {
    relPath: string;
    before: string;
    after: string;
}
/** Enough to restore a file to its pre-edit state (undo). */
export interface ToolCheckpoint {
    fsPath: string;
    before: string | null;
}
export interface ToolResult {
    ok: boolean;
    summary: string;
    content?: string;
    error?: string;
    diff?: ToolDiff;
    checkpoint?: ToolCheckpoint;
}
/** Approval callback: returns true to allow, false to reject. */
export type ApprovalFn = (tool: string, args: Record<string, unknown>, preview: string, diff?: ToolDiff) => Promise<boolean>;
export declare class VscodeTools {
    private readonly approve;
    constructor(approve: ApprovalFn);
    /** Static catalog used to build the model's system prompt and the UI. */
    static catalog(): ToolDef[];
    /**
     * Convert the tool catalog to OpenAI/Anthropic-style function schemas for
     * native function calling. Optionally restrict to an allow-list of names.
     */
    static toFunctionSchemas(allowed?: string[]): Array<{
        name: string;
        description: string;
        parameters: Record<string, unknown>;
    }>;
    private root;
    private resolve;
    private rel;
    execute(name: string, args: Record<string, unknown>): Promise<ToolResult>;
    private readFile;
    private listDir;
    private search;
    private openFile;
    private getDiagnostics;
    private getSelection;
    private writeFile;
    private editFile;
    private runCommand;
}
//# sourceMappingURL=VscodeTools.d.ts.map