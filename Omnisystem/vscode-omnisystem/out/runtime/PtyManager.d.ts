import { EventEmitter } from 'events';
export interface PtySession {
    id: string;
    pid: number;
    shell: string;
    cols: number;
    rows: number;
    cwd: string;
    backend: 'node-pty' | 'spawn';
    createdAt: number;
}
export type OutputCallback = (sessionId: string, data: string) => void;
export type ExitCallback = (sessionId: string, code: number) => void;
export declare class PtyManager extends EventEmitter {
    private sessions;
    private nextSessionId;
    create(cols?: number, rows?: number, shell?: string, cwd?: string, env?: Record<string, string>, onOutput?: OutputCallback, onExit?: ExitCallback): PtySession;
    private _spawnFallback;
    write(sessionId: string, data: string): boolean;
    resize(sessionId: string, cols: number, rows: number): boolean;
    kill(sessionId: string, signal?: string): boolean;
    getSession(id: string): PtySession | undefined;
    listSessions(): PtySession[];
    sessionCount(): number;
    get hasPty(): boolean;
    dispose(): void;
    private _defaultShell;
}
export declare function getPtyManager(): PtyManager;
export declare function disposePtyManager(): void;
//# sourceMappingURL=PtyManager.d.ts.map