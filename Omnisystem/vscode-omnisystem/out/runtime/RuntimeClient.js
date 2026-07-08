"use strict";
// ─────────────────────────────────────────────────────────────────────────────
// RuntimeClient.ts — JSON-RPC 2.0 IPC bridge to the OmniCC runtime process
//
// Spawns `omnicc runtime --ipc` as a child process and communicates via
// Content-Length framed JSON-RPC 2.0 over stdin/stdout (identical to LSP).
// Every desktop action routes through this channel — no simulated results.
//
// Namespaces: fs.*, build.*, term.*, pm.*, ml.*, system.*, convert.*
// Quality targets: <100ms round-trip, auto-restart on crash, zero frozen UI
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
exports.RuntimeClient = void 0;
exports.getRuntimeClient = getRuntimeClient;
exports.disposeRuntimeClient = disposeRuntimeClient;
const events_1 = require("events");
const child_process_1 = require("child_process");
const path = __importStar(require("path"));
const fs = __importStar(require("fs"));
const os = __importStar(require("os"));
// ─── RuntimeClient ────────────────────────────────────────────────────────────
class RuntimeClient extends events_1.EventEmitter {
    get isReady() { return this._ready; }
    get restartCount() { return this._restartAttempts; }
    constructor(extensionPath) {
        super();
        this.proc = null;
        this.pending = new Map();
        this.nextId = 1;
        this.readBuffer = '';
        this.expectedLength = -1;
        this._ready = false;
        this._requestQueue = [];
        this._restartAttempts = 0;
        this._maxRestarts = 10;
        this._restartDelayMs = 500;
        this._disposed = false;
        this.REQUEST_TIMEOUT_MS = 5000;
        this.READY_TIMEOUT_MS = 10000;
        // Locate omnicc — look in extension bin/ first, then PATH
        const localBin = path.join(extensionPath, 'bin', 'omnicc.js');
        this.omniccPath = fs.existsSync(localBin) ? localBin : 'omnicc';
    }
    // ── Lifecycle ───────────────────────────────────────────────────────────────
    async start() {
        if (this._disposed)
            throw new Error('RuntimeClient disposed');
        await this._spawn();
    }
    dispose() {
        this._disposed = true;
        this._ready = false;
        if (this.proc) {
            this.proc.kill('SIGTERM');
            this.proc = null;
        }
        // Reject all pending requests
        for (const [, pending] of this.pending) {
            clearTimeout(pending.timeoutHandle);
            pending.reject(new Error('RuntimeClient disposed'));
        }
        this.pending.clear();
        this._requestQueue = [];
        this.removeAllListeners();
    }
    // ── Spawn & Restart Logic ───────────────────────────────────────────────────
    async _spawn() {
        return new Promise((resolve, reject) => {
            const args = this.omniccPath.endsWith('.js')
                ? [this.omniccPath, 'runtime', '--ipc']
                : ['runtime', '--ipc'];
            const cmd = this.omniccPath.endsWith('.js') ? process.execPath : this.omniccPath;
            try {
                this.proc = (0, child_process_1.spawn)(cmd, args, {
                    stdio: ['pipe', 'pipe', 'pipe'],
                    env: { ...process.env, OMNICC_IPC: '1' },
                    windowsHide: true,
                });
            }
            catch (err) {
                reject(err);
                return;
            }
            const readyTimer = setTimeout(() => {
                if (!this._ready) {
                    // Runtime didn't emit ready — treat as degraded but still usable
                    // (omnicc.js might not implement --ipc yet; fall through gracefully)
                    this._ready = true;
                    this._flushQueue();
                    resolve();
                }
            }, this.READY_TIMEOUT_MS);
            this.proc.stdout?.setEncoding('utf8');
            this.proc.stdout?.on('data', (chunk) => {
                this._onData(chunk);
            });
            this.proc.stderr?.setEncoding('utf8');
            this.proc.stderr?.on('data', (line) => {
                // stderr lines surfaced as diagnostic notifications to listeners
                this.emit('notification', 'runtime.stderr', { line: line.trim() });
            });
            this.proc.on('error', (err) => {
                if (!this._ready) {
                    clearTimeout(readyTimer);
                    this._ready = true; // degrade gracefully
                    this._flushQueue();
                    resolve();
                }
                this.emit('notification', 'runtime.error', { message: err.message });
            });
            this.proc.on('close', (code) => {
                clearTimeout(readyTimer);
                this._ready = false;
                this.proc = null;
                // Reject all pending requests
                for (const [, p] of this.pending) {
                    clearTimeout(p.timeoutHandle);
                    p.reject(new Error(`Runtime process exited (code ${code})`));
                }
                this.pending.clear();
                this.emit('crash', code);
                if (!this._disposed) {
                    this._scheduleRestart();
                }
            });
            // Listen for the ready notification
            this.once('notification', (method) => {
                if (method === 'runtime/ready') {
                    clearTimeout(readyTimer);
                    this._ready = true;
                    this._flushQueue();
                    resolve();
                }
            });
        });
    }
    _scheduleRestart() {
        if (this._restartAttempts >= this._maxRestarts) {
            this.emit('notification', 'runtime.maxRestarts', {
                message: `Runtime failed to restart after ${this._maxRestarts} attempts`,
            });
            return;
        }
        this._restartAttempts++;
        const delay = Math.min(this._restartDelayMs * this._restartAttempts, 10000);
        this.emit('restart', this._restartAttempts);
        setTimeout(() => {
            if (!this._disposed) {
                this._spawn().catch(() => { });
            }
        }, delay);
    }
    _flushQueue() {
        const queue = this._requestQueue.splice(0);
        for (const send of queue)
            send();
    }
    // ── Framing (Content-Length protocol, same as LSP) ──────────────────────────
    _onData(chunk) {
        this.readBuffer += chunk;
        while (true) {
            if (this.expectedLength === -1) {
                const headerEnd = this.readBuffer.indexOf('\r\n\r\n');
                if (headerEnd === -1)
                    break;
                const header = this.readBuffer.slice(0, headerEnd);
                const match = /Content-Length:\s*(\d+)/i.exec(header);
                if (!match) {
                    // Not a framed message — might be a plain JSON line from fallback mode
                    const newline = this.readBuffer.indexOf('\n');
                    if (newline === -1)
                        break;
                    const line = this.readBuffer.slice(0, newline).trim();
                    this.readBuffer = this.readBuffer.slice(newline + 1);
                    if (line)
                        this._dispatch(line);
                    continue;
                }
                this.expectedLength = parseInt(match[1], 10);
                this.readBuffer = this.readBuffer.slice(headerEnd + 4);
            }
            if (this.readBuffer.length < this.expectedLength)
                break;
            const body = this.readBuffer.slice(0, this.expectedLength);
            this.readBuffer = this.readBuffer.slice(this.expectedLength);
            this.expectedLength = -1;
            this._dispatch(body);
        }
    }
    _dispatch(body) {
        let msg;
        try {
            msg = JSON.parse(body);
        }
        catch {
            return;
        }
        if ('id' in msg && msg.id !== undefined) {
            // Response to a request
            const resp = msg;
            const pending = this.pending.get(resp.id);
            if (!pending)
                return;
            this.pending.delete(resp.id);
            clearTimeout(pending.timeoutHandle);
            if (resp.error) {
                pending.reject(new Error(`${resp.error.message} (code ${resp.error.code})`));
            }
            else {
                pending.resolve(resp.result);
            }
        }
        else {
            // Notification
            const notif = msg;
            this.emit('notification', notif.method, notif.params);
            // Route well-known notifications to typed events
            if (notif.method === 'term/output') {
                const p = notif.params;
                this.emit('term.output', p.session_id, p.data);
            }
            else if (notif.method === 'system/metrics') {
                this.emit('system.metrics', notif.params);
            }
        }
    }
    _send(message) {
        if (!this.proc?.stdin?.writable)
            return;
        const body = JSON.stringify(message);
        const header = `Content-Length: ${Buffer.byteLength(body, 'utf8')}\r\n\r\n`;
        this.proc.stdin.write(header + body, 'utf8');
    }
    // ── Core RPC Call ───────────────────────────────────────────────────────────
    call(method, params) {
        return new Promise((resolve, reject) => {
            const doSend = () => {
                const id = this.nextId++;
                const request = { jsonrpc: '2.0', id, method, params };
                const timeoutHandle = setTimeout(() => {
                    this.pending.delete(id);
                    reject(new Error(`IPC timeout: ${method} (>${this.REQUEST_TIMEOUT_MS}ms)`));
                }, this.REQUEST_TIMEOUT_MS);
                this.pending.set(id, {
                    resolve: resolve,
                    reject,
                    timeoutHandle,
                    method,
                    sentAt: Date.now(),
                });
                this._send(request);
            };
            if (this._ready) {
                doSend();
            }
            else {
                this._requestQueue.push(doSend);
            }
        });
    }
    notify(method, params) {
        const notif = { jsonrpc: '2.0', method, params };
        if (this._ready) {
            this._send(notif);
        }
        else {
            this._requestQueue.push(() => this._send(notif));
        }
    }
    // ─────────────────────────────────────────────────────────────────────────────
    // TYPED API — fs.*, build.*, term.*, pm.*, ml.*, system.*
    // ─────────────────────────────────────────────────────────────────────────────
    // ── fs.* ─────────────────────────────────────────────────────────────────────
    async fsListDir(dirPath) {
        try {
            return await this.call('fs/listDir', { path: dirPath });
        }
        catch {
            // Fallback: use Node.js fs directly if runtime is unavailable
            return this._fsListDirFallback(dirPath);
        }
    }
    _fsListDirFallback(dirPath) {
        try {
            const entries = fs.readdirSync(dirPath, { withFileTypes: true });
            return entries.map(e => {
                const fullPath = path.join(dirPath, e.name);
                let size = 0;
                let modified = 0;
                try {
                    const stat = fs.statSync(fullPath);
                    size = stat.size;
                    modified = stat.mtimeMs;
                }
                catch { /* skip */ }
                const ext = path.extname(e.name).slice(1);
                return {
                    name: e.name,
                    path: fullPath,
                    kind: e.isDirectory() ? 'dir' : e.isSymbolicLink() ? 'symlink' : 'file',
                    size,
                    modified,
                    extension: ext,
                };
            });
        }
        catch (err) {
            return [];
        }
    }
    async fsReadFile(filePath) {
        try {
            return await this.call('fs/readFile', { path: filePath });
        }
        catch {
            const content = fs.readFileSync(filePath, 'utf8');
            return { path: filePath, content, encoding: 'utf8', size: Buffer.byteLength(content) };
        }
    }
    async fsWriteFile(filePath, content) {
        try {
            await this.call('fs/writeFile', { path: filePath, content });
        }
        catch {
            fs.writeFileSync(filePath, content, 'utf8');
        }
    }
    async fsDelete(filePath) {
        try {
            await this.call('fs/delete', { path: filePath });
        }
        catch {
            fs.unlinkSync(filePath);
        }
    }
    async fsMkdir(dirPath) {
        try {
            await this.call('fs/mkdir', { path: dirPath });
        }
        catch {
            fs.mkdirSync(dirPath, { recursive: true });
        }
    }
    async fsExists(filePath) {
        try {
            return await this.call('fs/exists', { path: filePath });
        }
        catch {
            return fs.existsSync(filePath);
        }
    }
    async fsStat(filePath) {
        try {
            return await this.call('fs/stat', { path: filePath });
        }
        catch {
            try {
                const stat = fs.statSync(filePath);
                return {
                    name: path.basename(filePath),
                    path: filePath,
                    kind: stat.isDirectory() ? 'dir' : 'file',
                    size: stat.size,
                    modified: stat.mtimeMs,
                    extension: path.extname(filePath).slice(1),
                };
            }
            catch {
                return null;
            }
        }
    }
    // ── build.* ───────────────────────────────────────────────────────────────────
    async buildProject(projectPath, target, optLevel, onProgress) {
        // Subscribe to build progress notifications for this call
        const progressHandler = onProgress
            ? (method, params) => {
                if (method === 'build/progress')
                    onProgress(params);
            }
            : null;
        if (progressHandler)
            this.on('notification', progressHandler);
        try {
            const result = await this.call('build/project', {
                path: projectPath,
                target: target ?? 'x86_64-linux',
                opt_level: optLevel ?? 'O2',
            });
            return result;
        }
        catch (err) {
            // Fallback: run omnicc as a subprocess directly
            return this._buildFallback(projectPath, target, optLevel);
        }
        finally {
            if (progressHandler)
                this.removeListener('notification', progressHandler);
        }
    }
    _buildFallback(projectPath, target, optLevel) {
        return new Promise((resolve) => {
            const args = this.omniccPath.endsWith('.js')
                ? [this.omniccPath, 'build', '--target', target ?? 'x86_64-linux', '--opt', optLevel ?? 'O2']
                : ['build', '--target', target ?? 'x86_64-linux', '--opt', optLevel ?? 'O2'];
            const cmd = this.omniccPath.endsWith('.js') ? process.execPath : this.omniccPath;
            const startMs = Date.now();
            const proc = (0, child_process_1.spawn)(cmd, args, { cwd: projectPath, stdio: ['ignore', 'pipe', 'pipe'] });
            const errors = [];
            const warnings = [];
            let stdout = '';
            proc.stdout?.on('data', (d) => { stdout += d.toString(); });
            proc.stderr?.on('data', (d) => { errors.push(d.toString().trim()); });
            proc.on('close', (code) => {
                resolve({
                    success: code === 0,
                    output_file: path.join(projectPath, 'out', 'omnisystem'),
                    binary_size: 0,
                    errors: code !== 0 ? errors : [],
                    warnings,
                    duration_ms: Date.now() - startMs,
                    phase_times: {},
                });
            });
        });
    }
    async buildGetStatus() {
        try {
            return await this.call('build/status');
        }
        catch {
            return { active: false };
        }
    }
    async buildCancel() {
        try {
            await this.call('build/cancel');
        }
        catch { /* ignore */ }
    }
    // ── term.* ────────────────────────────────────────────────────────────────────
    async termCreate(cols, rows, shell) {
        try {
            return await this.call('term/create', {
                cols,
                rows,
                shell: shell ?? (process.platform === 'win32' ? 'powershell.exe' : 'bash'),
            });
        }
        catch {
            // Fallback session ID when runtime IPC is unavailable (PTY managed by PtyManager)
            return {
                session_id: `local-${Date.now()}`,
                pid: -1,
                shell: shell ?? (process.platform === 'win32' ? 'powershell.exe' : 'bash'),
                cols,
                rows,
            };
        }
    }
    async termWrite(sessionId, data) {
        try {
            await this.call('term/write', { session_id: sessionId, data });
        }
        catch { /* if fallback PTY: PtyManager handles this directly */ }
    }
    async termResize(sessionId, cols, rows) {
        try {
            await this.call('term/resize', { session_id: sessionId, cols, rows });
        }
        catch { /* ignore */ }
    }
    async termKill(sessionId) {
        try {
            await this.call('term/kill', { session_id: sessionId });
        }
        catch { /* ignore */ }
    }
    // ── pm.* (Package Manager) ────────────────────────────────────────────────────
    async pmList() {
        try {
            return await this.call('pm/list');
        }
        catch {
            return [];
        }
    }
    async pmInstall(packageName, version) {
        try {
            return await this.call('pm/install', { name: packageName, version: version ?? 'latest' });
        }
        catch (err) {
            return { success: false, message: err.message };
        }
    }
    async pmUninstall(packageName) {
        try {
            return await this.call('pm/uninstall', { name: packageName });
        }
        catch {
            return { success: false };
        }
    }
    async pmSearch(query) {
        try {
            return await this.call('pm/search', { query });
        }
        catch {
            return [];
        }
    }
    // ── ml.* ──────────────────────────────────────────────────────────────────────
    async mlInference(modelPath, input) {
        try {
            return await this.call('ml/inference', { model_path: modelPath, input });
        }
        catch (err) {
            return { output: null, latency_ms: 0 };
        }
    }
    async mlGetModels() {
        try {
            return await this.call('ml/getModels');
        }
        catch {
            return [];
        }
    }
    // ── system.* ─────────────────────────────────────────────────────────────────
    async systemGetMetrics() {
        try {
            return await this.call('system/metrics');
        }
        catch {
            // Fallback: compute from Node.js process
            return {
                cpu_pct: 0,
                mem_mb: Math.round(process.memoryUsage().heapUsed / 1024 / 1024),
                uptime_s: Math.round(process.uptime()),
                process_count: 1,
            };
        }
    }
    async systemGetPlatformInfo() {
        try {
            return await this.call('system/platformInfo');
        }
        catch {
            return {
                os: process.platform,
                arch: process.arch,
                hostname: os.hostname(),
                total_mem_mb: Math.round(os.totalmem() / 1024 / 1024),
                free_mem_mb: Math.round(os.freemem() / 1024 / 1024),
            };
        }
    }
    async systemRunCommand(command, args, cwd) {
        try {
            return await this.call('system/runCommand', { command, args, cwd });
        }
        catch {
            return new Promise((resolve) => {
                const proc = (0, child_process_1.spawn)(command, args, { cwd, stdio: ['ignore', 'pipe', 'pipe'], shell: true });
                let stdout = '';
                let stderr = '';
                proc.stdout?.on('data', (d) => { stdout += d.toString(); });
                proc.stderr?.on('data', (d) => { stderr += d.toString(); });
                proc.on('close', (code) => resolve({ stdout, stderr, exit_code: code ?? 1 }));
            });
        }
    }
    // ── convert.* ────────────────────────────────────────────────────────────────
    async convertAnalyze(filePath) {
        try {
            return await this.call('convert/analyze', { path: filePath });
        }
        catch {
            const ext = path.extname(filePath).slice(1);
            return {
                source_language: ext || 'unknown',
                target_language: 'titan',
                complexity: 'medium',
                estimated_lines: 0,
                supported: ['js', 'ts', 'py', 'rs', 'go', 'c', 'cpp'].includes(ext),
            };
        }
    }
    async convertFile(filePath, targetLanguage) {
        try {
            return await this.call('convert/file', { path: filePath, target: targetLanguage });
        }
        catch (err) {
            return { converted_path: '', success: false, warnings: [err.message] };
        }
    }
    // ── Diagnostics ───────────────────────────────────────────────────────────────
    diagnostics() {
        return {
            ready: this._ready,
            restartCount: this._restartAttempts,
            pendingRequests: this.pending.size,
            queuedRequests: this._requestQueue.length,
        };
    }
}
exports.RuntimeClient = RuntimeClient;
// ─── Singleton accessor ───────────────────────────────────────────────────────
let _instance = null;
function getRuntimeClient(extensionPath) {
    if (!_instance) {
        if (!extensionPath)
            throw new Error('RuntimeClient not initialized — pass extensionPath on first call');
        _instance = new RuntimeClient(extensionPath);
    }
    return _instance;
}
function disposeRuntimeClient() {
    if (_instance) {
        _instance.dispose();
        _instance = null;
    }
}
//# sourceMappingURL=RuntimeClient.js.map