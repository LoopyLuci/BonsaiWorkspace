import { EventEmitter } from 'events';
export interface RuntimeClientEvents {
    ready: [];
    crash: [exitCode: number | null];
    restart: [attempt: number];
    notification: [method: string, params: unknown];
    'term.output': [sessionId: string, data: string];
    'system.metrics': [metrics: SystemMetrics];
}
export interface SystemMetrics {
    cpu_pct: number;
    mem_mb: number;
    uptime_s: number;
    process_count: number;
}
export interface FsEntry {
    name: string;
    path: string;
    kind: 'file' | 'dir' | 'symlink';
    size: number;
    modified: number;
    extension: string;
}
export interface FsReadResult {
    path: string;
    content: string;
    encoding: string;
    size: number;
}
export interface BuildProgress {
    phase: string;
    current: number;
    total: number;
    message: string;
}
export interface BuildResult {
    success: boolean;
    output_file: string;
    binary_size: number;
    errors: string[];
    warnings: string[];
    duration_ms: number;
    phase_times: Record<string, number>;
}
export interface TermSession {
    session_id: string;
    pid: number;
    shell: string;
    cols: number;
    rows: number;
}
export declare class RuntimeClient extends EventEmitter {
    private proc;
    private pending;
    private nextId;
    private readBuffer;
    private expectedLength;
    private _ready;
    private _requestQueue;
    private _restartAttempts;
    private _maxRestarts;
    private _restartDelayMs;
    private _disposed;
    private readonly REQUEST_TIMEOUT_MS;
    private readonly READY_TIMEOUT_MS;
    private readonly omniccPath;
    get isReady(): boolean;
    get restartCount(): number;
    constructor(extensionPath: string);
    start(): Promise<void>;
    dispose(): void;
    private _spawn;
    private _scheduleRestart;
    private _flushQueue;
    private _onData;
    private _dispatch;
    private _send;
    call<T = unknown>(method: string, params?: unknown): Promise<T>;
    notify(method: string, params?: unknown): void;
    fsListDir(dirPath: string): Promise<FsEntry[]>;
    private _fsListDirFallback;
    fsReadFile(filePath: string): Promise<FsReadResult>;
    fsWriteFile(filePath: string, content: string): Promise<void>;
    fsDelete(filePath: string): Promise<void>;
    fsMkdir(dirPath: string): Promise<void>;
    fsExists(filePath: string): Promise<boolean>;
    fsStat(filePath: string): Promise<FsEntry | null>;
    buildProject(projectPath: string, target?: string, optLevel?: string, onProgress?: (p: BuildProgress) => void): Promise<BuildResult>;
    private _buildFallback;
    buildGetStatus(): Promise<{
        active: boolean;
        phase?: string;
        progress?: number;
    }>;
    buildCancel(): Promise<void>;
    termCreate(cols: number, rows: number, shell?: string): Promise<TermSession>;
    termWrite(sessionId: string, data: string): Promise<void>;
    termResize(sessionId: string, cols: number, rows: number): Promise<void>;
    termKill(sessionId: string): Promise<void>;
    pmList(): Promise<Array<{
        name: string;
        version: string;
        installed: boolean;
    }>>;
    pmInstall(packageName: string, version?: string): Promise<{
        success: boolean;
        message: string;
    }>;
    pmUninstall(packageName: string): Promise<{
        success: boolean;
    }>;
    pmSearch(query: string): Promise<Array<{
        name: string;
        version: string;
        description: string;
    }>>;
    mlInference(modelPath: string, input: unknown): Promise<{
        output: unknown;
        latency_ms: number;
    }>;
    mlGetModels(): Promise<Array<{
        name: string;
        path: string;
        framework: string;
        size_mb: number;
    }>>;
    systemGetMetrics(): Promise<SystemMetrics>;
    systemGetPlatformInfo(): Promise<{
        os: string;
        arch: string;
        hostname: string;
        total_mem_mb: number;
        free_mem_mb: number;
    }>;
    systemRunCommand(command: string, args: string[], cwd?: string): Promise<{
        stdout: string;
        stderr: string;
        exit_code: number;
    }>;
    convertAnalyze(filePath: string): Promise<{
        source_language: string;
        target_language: string;
        complexity: 'low' | 'medium' | 'high';
        estimated_lines: number;
        supported: boolean;
    }>;
    convertFile(filePath: string, targetLanguage: string): Promise<{
        converted_path: string;
        success: boolean;
        warnings: string[];
    }>;
    diagnostics(): {
        ready: boolean;
        restartCount: number;
        pendingRequests: number;
        queuedRequests: number;
    };
}
export declare function getRuntimeClient(extensionPath?: string): RuntimeClient;
export declare function disposeRuntimeClient(): void;
//# sourceMappingURL=RuntimeClient.d.ts.map