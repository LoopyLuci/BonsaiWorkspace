// ─────────────────────────────────────────────────────────────────────────────
// PtyManager.ts — Real PTY sessions for the OmniOS Desktop Terminal app
//
// Uses node-pty when available (real PTY with colors, arrow keys, tab completion).
// Falls back to child_process.spawn when node-pty is not installed.
// Streams output to the webview via postMessage.
// ─────────────────────────────────────────────────────────────────────────────

import { EventEmitter } from 'events';
import { spawn, ChildProcess } from 'child_process';
import * as os from 'os';
import * as path from 'path';

// ─── node-pty optional import ─────────────────────────────────────────────────

let pty: typeof import('node-pty') | null = null;
try {
  // eslint-disable-next-line @typescript-eslint/no-var-requires
  pty = require('node-pty');
} catch {
  // node-pty not installed — will use spawn fallback
}

// ─── Types ────────────────────────────────────────────────────────────────────

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

// ─── PtyManager ───────────────────────────────────────────────────────────────

export class PtyManager extends EventEmitter {
  // Maps session ID → live process handle (either node-pty IPty or ChildProcess)
  private sessions = new Map<string, {
    meta: PtySession;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    handle: any; // IPty | ChildProcess
  }>();
  private nextSessionId = 1;

  // ── Session creation ────────────────────────────────────────────────────────

  public create(
    cols = 120,
    rows = 30,
    shell?: string,
    cwd?: string,
    env?: Record<string, string>,
    onOutput?: OutputCallback,
    onExit?: ExitCallback,
  ): PtySession {
    const id = `pty-${this.nextSessionId++}`;
    const resolvedShell = shell ?? this._defaultShell();
    const resolvedCwd = cwd ?? os.homedir();
    const resolvedEnv = { ...process.env, TERM: 'xterm-256color', ...env } as Record<string, string>;

    let handle: unknown;
    let pid = -1;
    let backend: 'node-pty' | 'spawn' = 'spawn';

    if (pty) {
      // node-pty path — real PTY with full ANSI support
      try {
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        const ptyProc = pty.spawn(resolvedShell, [], {
          name: 'xterm-256color',
          cols,
          rows,
          cwd: resolvedCwd,
          env: resolvedEnv,
        });
        pid = ptyProc.pid;
        backend = 'node-pty';
        handle = ptyProc;

        ptyProc.onData((data: string) => {
          this.emit('output', id, data);
          onOutput?.(id, data);
        });

        ptyProc.onExit(({ exitCode }) => {
          this.sessions.delete(id);
          this.emit('exit', id, exitCode ?? 0);
          onExit?.(id, exitCode ?? 0);
        });
      } catch {
        // node-pty spawn failed — fall through to spawn backend
        handle = this._spawnFallback(id, resolvedShell, resolvedCwd, resolvedEnv, cols, rows, onOutput, onExit);
        pid = (handle as ChildProcess).pid ?? -1;
      }
    } else {
      // spawn fallback — no PTY but functional shell
      handle = this._spawnFallback(id, resolvedShell, resolvedCwd, resolvedEnv, cols, rows, onOutput, onExit);
      pid = (handle as ChildProcess).pid ?? -1;
    }

    const meta: PtySession = {
      id, pid, shell: resolvedShell, cols, rows, cwd: resolvedCwd, backend,
      createdAt: Date.now(),
    };
    this.sessions.set(id, { meta, handle });
    return meta;
  }

  private _spawnFallback(
    id: string,
    shell: string,
    cwd: string,
    env: Record<string, string>,
    _cols: number,
    _rows: number,
    onOutput?: OutputCallback,
    onExit?: ExitCallback,
  ): ChildProcess {
    const isWin = process.platform === 'win32';
    const proc = spawn(shell, isWin ? [] : ['--login'], {
      cwd,
      env,
      stdio: ['pipe', 'pipe', 'pipe'],
      shell: false,
      windowsHide: true,
    });

    proc.stdout?.setEncoding('utf8');
    proc.stdout?.on('data', (data: string) => {
      this.emit('output', id, data);
      onOutput?.(id, data);
    });
    proc.stderr?.setEncoding('utf8');
    proc.stderr?.on('data', (data: string) => {
      this.emit('output', id, data);
      onOutput?.(id, data);
    });

    proc.on('close', (code) => {
      this.sessions.delete(id);
      const exitCode = code ?? 0;
      this.emit('exit', id, exitCode);
      onExit?.(id, exitCode);
    });

    return proc;
  }

  // ── Session interaction ─────────────────────────────────────────────────────

  public write(sessionId: string, data: string): boolean {
    const session = this.sessions.get(sessionId);
    if (!session) return false;

    const { meta, handle } = session;
    if (meta.backend === 'node-pty') {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      (handle as any).write(data);
    } else {
      const proc = handle as ChildProcess;
      if (proc.stdin?.writable) proc.stdin.write(data);
    }
    return true;
  }

  public resize(sessionId: string, cols: number, rows: number): boolean {
    const session = this.sessions.get(sessionId);
    if (!session) return false;

    const { meta, handle } = session;
    meta.cols = cols;
    meta.rows = rows;

    if (meta.backend === 'node-pty') {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      (handle as any).resize(cols, rows);
    }
    return true;
  }

  public kill(sessionId: string, signal: string = 'SIGTERM'): boolean {
    const session = this.sessions.get(sessionId);
    if (!session) return false;

    const { meta, handle } = session;
    if (meta.backend === 'node-pty') {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      (handle as any).kill(signal);
    } else {
      (handle as ChildProcess).kill(signal as NodeJS.Signals);
    }
    this.sessions.delete(sessionId);
    return true;
  }

  // ── Queries ─────────────────────────────────────────────────────────────────

  public getSession(id: string): PtySession | undefined {
    return this.sessions.get(id)?.meta;
  }

  public listSessions(): PtySession[] {
    return Array.from(this.sessions.values()).map(s => s.meta);
  }

  public sessionCount(): number {
    return this.sessions.size;
  }

  public get hasPty(): boolean {
    return pty !== null;
  }

  // ── Dispose ─────────────────────────────────────────────────────────────────

  public dispose(): void {
    for (const [id] of this.sessions) {
      this.kill(id, 'SIGTERM');
    }
    this.sessions.clear();
    this.removeAllListeners();
  }

  // ── Helpers ──────────────────────────────────────────────────────────────────

  private _defaultShell(): string {
    if (process.platform === 'win32') {
      return process.env.COMSPEC ?? 'C:\\Windows\\System32\\WindowsPowerShell\\v1.0\\powershell.exe';
    }
    return process.env.SHELL ?? '/bin/bash';
  }
}

// ─── Singleton ────────────────────────────────────────────────────────────────

let _ptyManager: PtyManager | null = null;

export function getPtyManager(): PtyManager {
  if (!_ptyManager) _ptyManager = new PtyManager();
  return _ptyManager;
}

export function disposePtyManager(): void {
  if (_ptyManager) {
    _ptyManager.dispose();
    _ptyManager = null;
  }
}
