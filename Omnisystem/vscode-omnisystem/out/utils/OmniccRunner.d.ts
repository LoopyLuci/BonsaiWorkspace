import { ChildProcess } from 'child_process';
export interface RunResult {
    exitCode: number;
    stdout: string;
    stderr: string;
}
export declare class OmniccRunner {
    private _omniccPath;
    constructor();
    /**
     * Run an omnicc command to completion and collect output.
     * Optionally streams each stdout line to `onOutput` as it arrives.
     */
    run(args: string[], cwd?: string, onOutput?: (line: string) => void): Promise<RunResult>;
    /**
     * Spawn an omnicc process and return the child immediately (fire-and-forget style).
     */
    spawn(args: string[], cwd?: string): ChildProcess;
    /**
     * Try to find the omnicc binary.  Checks (in order):
     *   1. The path from extension config
     *   2. Alongside this extension's dist folder (packaged scenario)
     *   3. Relies on PATH (returns undefined if not found by the above two)
     */
    resolveOmniccPath(): Promise<string | undefined>;
    /**
     * Spawn `omnicc lsp --stdio` and return the live child process.
     * VS Code's LanguageClient expects to read/write stdio of this process.
     */
    spawnLspServer(serverPath?: string): ChildProcess;
    /**
     * Convenience: return the first workspace folder path or process.cwd().
     */
    private _workspaceRoot;
    private _exists;
}
//# sourceMappingURL=OmniccRunner.d.ts.map