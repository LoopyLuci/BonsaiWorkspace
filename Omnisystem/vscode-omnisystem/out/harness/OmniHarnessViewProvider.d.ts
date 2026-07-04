import * as vscode from 'vscode';
import { HarnessStore } from './HarnessStore';
export declare class OmniHarnessViewProvider implements vscode.WebviewViewProvider {
    private readonly ctx;
    private readonly store;
    private readonly output;
    static readonly viewType = "omniharnessChat";
    private view?;
    private client;
    private runner?;
    private session;
    private pendingApprovals;
    private serverProc?;
    private mcp;
    private healthTimer?;
    private lastAlive;
    private autoStartAttempted;
    private manualStop;
    private restartAttempts;
    private restartTimer?;
    private depsReady;
    private static readonly MAX_AUTO_RESTARTS;
    private static readonly MAX_LOG_LINES;
    private logBuffer;
    constructor(ctx: vscode.ExtensionContext, store: HarnessStore, output: vscode.OutputChannel);
    private log;
    private logChunk;
    private loadOrCreateSession;
    private postSessionLoaded;
    private postSessionsList;
    private persistSession;
    /** Connect/refresh MCP servers and push their status to the webview. */
    private syncMcp;
    private postMcpState;
    /** Bridge the MCP manager to the agent runner's ExternalTools contract. */
    private buildExternalTools;
    private cfg;
    /** The effective orchestrator URL: the active server profile, falling back to the plain setting. */
    private serverUrl;
    private approvalMode;
    private toolMode;
    private static readonly NATIVE_PROVIDERS;
    /** modelId (both bare and provider-qualified) → provider, from /api/models. */
    private modelProvider;
    private toolCapableModels;
    private lastModels;
    private modelSupportsTools;
    private providerOf;
    /** Resolve whether to use native function calling for a given model. */
    private useNativeTools;
    private harnessRoot;
    resolveWebviewView(view: vscode.WebviewView): void;
    focus(): void;
    newSession(): void;
    startServerCommand(): Promise<void>;
    stopServerCommand(): void;
    addSelectionCommand(): Promise<void>;
    exportConfigCommand(): Promise<void>;
    importConfigCommand(): Promise<void>;
    /** Undo the most recent still-undoable mutating tool call in the active session. */
    undoLastCommand(): Promise<void>;
    private post;
    private handleMessage;
    private createNewSession;
    private switchSession;
    private deleteSession;
    private refreshState;
    private postFavorites;
    private postServerProfiles;
    private switchServerProfile;
    private sendModels;
    private probeServer;
    private probeServerAndMaybeAutoStart;
    private static readonly RESERVED_OUTPUT_TOKENS;
    private static readonly DEFAULT_CONTEXT_WINDOW;
    private static readonly COMPACT_KEEP_TAIL;
    private estimateTokensForMessages;
    private modelContextWindow;
    /**
     * Builds the message list actually sent to the model for this turn,
     * compacting older history into a running summary when `force` is set
     * or the live context has crossed the configured token threshold.
     */
    private buildContextMessages;
    compactNowCommand(): Promise<void>;
    private onSend;
    private undoToolCall;
    private requestApproval;
    private resolveApproval;
    private onSaveKey;
    private onApplyEnv;
    private onSaveAgent;
    private addSelectionContext;
    private exportConfig;
    private importConfig;
    /** Runs a command in `cwd`, streaming stdout/stderr into the log. Resolves with the exit code. */
    private runCommand;
    /**
     * Ensures the orchestrator's third-party dependencies are importable,
     * installing them automatically if not — so a fresh checkout "just runs".
     *
     * Key design choices for robustness:
     *  • We do NOT `pip install -e .` the omniharness package. Running
     *    `uvicorn omniharness.server:app` from cwd=orchestrator imports the
     *    package straight from the source tree, so we only need its runtime
     *    dependencies present. This sidesteps the whole editable-build path
     *    (hatchling, README metadata, PEP 660 backend quirks) that was failing.
     *  • The import check is passed as a single JSON-quoted `-c` argument so
     *    shell:true on Windows can't split it on spaces (the old check sent
     *    Python just `import`, producing a SyntaxError).
     *  • Install strategy is layered: requirements.txt first, then a direct
     *    inline dependency list, then `-e .` as a last resort — whichever
     *    first makes the imports succeed wins.
     */
    private ensureDependencies;
    private static readonly RUNTIME_DEPS;
    private spawnServerProcess;
    /** Polls /api/health for up to ~20s; resets the restart-backoff counter once healthy. */
    private pollUntilHealthy;
    private scheduleAutoRestart;
    private startServer;
    private stopServer;
    dispose(): void;
    private getHtml;
}
//# sourceMappingURL=OmniHarnessViewProvider.d.ts.map