"use strict";
// ─────────────────────────────────────────────────────────────────────────────
// PtyManager.ts — Real PTY sessions for the OmniOS Desktop Terminal app
//
// Uses node-pty when available (real PTY with colors, arrow keys, tab completion).
// Falls back to child_process.spawn when node-pty is not installed.
// Streams output to the webview via postMessage.
// ─────────────────────────────────────────────────────────────────────────────
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
exports.PtyManager = void 0;
exports.getPtyManager = getPtyManager;
exports.disposePtyManager = disposePtyManager;
const events_1 = require("events");
const child_process_1 = require("child_process");
const os = __importStar(require("os"));
// ─── node-pty optional import ─────────────────────────────────────────────────
let pty = null;
try {
    // eslint-disable-next-line @typescript-eslint/no-var-requires
    pty = require('node-pty');
}
catch {
    // node-pty not installed — will use spawn fallback
}
// ─── PtyManager ───────────────────────────────────────────────────────────────
class PtyManager extends events_1.EventEmitter {
    constructor() {
        super(...arguments);
        // Maps session ID → live process handle (either node-pty IPty or ChildProcess)
        this.sessions = new Map();
        this.nextSessionId = 1;
    }
    // ── Session creation ────────────────────────────────────────────────────────
    create(cols = 120, rows = 30, shell, cwd, env, onOutput, onExit) {
        const id = `pty-${this.nextSessionId++}`;
        const resolvedShell = shell ?? this._defaultShell();
        const resolvedCwd = cwd ?? os.homedir();
        const resolvedEnv = { ...process.env, TERM: 'xterm-256color', ...env };
        let handle;
        let pid = -1;
        let backend = 'spawn';
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
                ptyProc.onData((data) => {
                    this.emit('output', id, data);
                    onOutput?.(id, data);
                });
                ptyProc.onExit(({ exitCode }) => {
                    this.sessions.delete(id);
                    this.emit('exit', id, exitCode ?? 0);
                    onExit?.(id, exitCode ?? 0);
                });
            }
            catch {
                // node-pty spawn failed — fall through to spawn backend
                handle = this._spawnFallback(id, resolvedShell, resolvedCwd, resolvedEnv, cols, rows, onOutput, onExit);
                pid = handle.pid ?? -1;
            }
        }
        else {
            // spawn fallback — no PTY but functional shell
            handle = this._spawnFallback(id, resolvedShell, resolvedCwd, resolvedEnv, cols, rows, onOutput, onExit);
            pid = handle.pid ?? -1;
        }
        const meta = {
            id, pid, shell: resolvedShell, cols, rows, cwd: resolvedCwd, backend,
            createdAt: Date.now(),
        };
        this.sessions.set(id, { meta, handle });
        return meta;
    }
    _spawnFallback(id, shell, cwd, env, _cols, _rows, onOutput, onExit) {
        const isWin = process.platform === 'win32';
        const proc = (0, child_process_1.spawn)(shell, isWin ? [] : ['--login'], {
            cwd,
            env,
            stdio: ['pipe', 'pipe', 'pipe'],
            shell: false,
            windowsHide: true,
        });
        proc.stdout?.setEncoding('utf8');
        proc.stdout?.on('data', (data) => {
            this.emit('output', id, data);
            onOutput?.(id, data);
        });
        proc.stderr?.setEncoding('utf8');
        proc.stderr?.on('data', (data) => {
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
    write(sessionId, data) {
        const session = this.sessions.get(sessionId);
        if (!session)
            return false;
        const { meta, handle } = session;
        if (meta.backend === 'node-pty') {
            // eslint-disable-next-line @typescript-eslint/no-explicit-any
            handle.write(data);
        }
        else {
            const proc = handle;
            if (proc.stdin?.writable)
                proc.stdin.write(data);
        }
        return true;
    }
    resize(sessionId, cols, rows) {
        const session = this.sessions.get(sessionId);
        if (!session)
            return false;
        const { meta, handle } = session;
        meta.cols = cols;
        meta.rows = rows;
        if (meta.backend === 'node-pty') {
            // eslint-disable-next-line @typescript-eslint/no-explicit-any
            handle.resize(cols, rows);
        }
        return true;
    }
    kill(sessionId, signal = 'SIGTERM') {
        const session = this.sessions.get(sessionId);
        if (!session)
            return false;
        const { meta, handle } = session;
        if (meta.backend === 'node-pty') {
            // eslint-disable-next-line @typescript-eslint/no-explicit-any
            handle.kill(signal);
        }
        else {
            handle.kill(signal);
        }
        this.sessions.delete(sessionId);
        return true;
    }
    // ── Queries ─────────────────────────────────────────────────────────────────
    getSession(id) {
        return this.sessions.get(id)?.meta;
    }
    listSessions() {
        return Array.from(this.sessions.values()).map(s => s.meta);
    }
    sessionCount() {
        return this.sessions.size;
    }
    get hasPty() {
        return pty !== null;
    }
    // ── Dispose ─────────────────────────────────────────────────────────────────
    dispose() {
        for (const [id] of this.sessions) {
            this.kill(id, 'SIGTERM');
        }
        this.sessions.clear();
        this.removeAllListeners();
    }
    // ── Helpers ──────────────────────────────────────────────────────────────────
    _defaultShell() {
        if (process.platform === 'win32') {
            return process.env.COMSPEC ?? 'C:\\Windows\\System32\\WindowsPowerShell\\v1.0\\powershell.exe';
        }
        return process.env.SHELL ?? '/bin/bash';
    }
}
exports.PtyManager = PtyManager;
// ─── Singleton ────────────────────────────────────────────────────────────────
let _ptyManager = null;
function getPtyManager() {
    if (!_ptyManager)
        _ptyManager = new PtyManager();
    return _ptyManager;
}
function disposePtyManager() {
    if (_ptyManager) {
        _ptyManager.dispose();
        _ptyManager = null;
    }
}
//# sourceMappingURL=PtyManager.js.map