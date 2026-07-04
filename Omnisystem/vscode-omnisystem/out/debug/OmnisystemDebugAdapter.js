"use strict";
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
const debugadapter_1 = require("@vscode/debugadapter");
const child_process_1 = require("child_process");
const path = __importStar(require("path"));
const readline = __importStar(require("readline"));
class OmnisystemDebugSession extends debugadapter_1.DebugSession {
    constructor() {
        super();
        this._variableHandles = new debugadapter_1.Handles();
        this._configurationDone = false;
        this._pendingBreakpoints = new Map();
        this._frameCache = new Map();
        this._variableCache = new Map();
        this._responseCallbacks = new Map();
        this._requestSeq = 0;
        this.setDebuggerLinesStartAt1(true);
        this.setDebuggerColumnsStartAt1(true);
    }
    initializeRequest(response, _args) {
        response.body = response.body || {};
        response.body.supportsConfigurationDoneRequest = true;
        response.body.supportsEvaluateForHovers = true;
        response.body.supportsStepBack = true;
        response.body.supportsRestartRequest = true;
        response.body.supportsTerminateRequest = true;
        response.body.supportsBreakpointLocationsRequest = true;
        response.body.supportsConditionalBreakpoints = true;
        response.body.supportsHitConditionalBreakpoints = true;
        response.body.supportsLogPoints = true;
        response.body.supportsSetVariable = true;
        response.body.supportsReadMemoryRequest = false;
        response.body.supportsDisassembleRequest = false;
        this.sendResponse(response);
        this.sendEvent(new debugadapter_1.InitializedEvent());
    }
    configurationDoneRequest(response, _args) {
        this._configurationDone = true;
        this.sendResponse(response);
    }
    async launchRequest(response, args) {
        const omniccPath = args.omniccPath ?? 'omnicc';
        const program = args.program;
        const cwd = args.cwd ?? path.dirname(program);
        const extraArgs = args.args ?? [];
        const target = args.target ?? '';
        const spawnArgs = ['run', '--debug', '--dap', program];
        if (target) {
            spawnArgs.push('--target', target);
        }
        spawnArgs.push(...extraArgs);
        this.sendEvent(new debugadapter_1.OutputEvent(`Launching: ${omniccPath} ${spawnArgs.join(' ')}\n`, 'console'));
        this._debugProcess = (0, child_process_1.spawn)(omniccPath, spawnArgs, {
            cwd,
            stdio: ['pipe', 'pipe', 'pipe']
        });
        const rl = readline.createInterface({ input: this._debugProcess.stdout });
        rl.on('line', (line) => {
            if (!line.trim())
                return;
            try {
                const msg = JSON.parse(line);
                this._handleDebugMessage(msg);
            }
            catch {
                // Plain output line
                this.sendEvent(new debugadapter_1.OutputEvent(line + '\n', 'stdout'));
            }
        });
        this._debugProcess.stderr.on('data', (data) => {
            this.sendEvent(new debugadapter_1.OutputEvent(data.toString(), 'stderr'));
        });
        this._debugProcess.on('exit', (code) => {
            this.sendEvent(new debugadapter_1.OutputEvent(`Process exited with code ${code ?? 0}\n`, 'console'));
            this.sendEvent(new debugadapter_1.TerminatedEvent());
        });
        this._debugProcess.on('error', (err) => {
            this.sendEvent(new debugadapter_1.OutputEvent(`Failed to start omnicc: ${err.message}\n`, 'stderr'));
            this.sendEvent(new debugadapter_1.TerminatedEvent());
        });
        // Wait for configurationDone before resuming if stopOnEntry
        if (args.stopOnEntry) {
            this._sendToDebugProcess({ command: 'stopOnEntry', body: { stop: true } });
        }
        this.sendResponse(response);
    }
    setBreakPointsRequest(response, args) {
        const sourcePath = args.source.path ?? '';
        const requestedBps = args.breakpoints ?? [];
        this._pendingBreakpoints.set(sourcePath, requestedBps);
        const breakpoints = requestedBps.map((bp, idx) => {
            const verified = this._debugProcess !== undefined;
            const b = new debugadapter_1.Breakpoint(verified, bp.line, bp.column);
            b.id = idx;
            return b;
        });
        this._sendToDebugProcess({
            command: 'setBreakpoints',
            body: {
                source: sourcePath,
                breakpoints: requestedBps.map(bp => ({
                    line: bp.line,
                    column: bp.column ?? 0,
                    condition: bp.condition ?? '',
                    hitCondition: bp.hitCondition ?? '',
                    logMessage: bp.logMessage ?? ''
                }))
            }
        });
        response.body = { breakpoints };
        this.sendResponse(response);
    }
    breakpointLocationsRequest(response, args) {
        response.body = {
            breakpoints: [
                { line: args.line, column: args.column ?? 0 }
            ]
        };
        this.sendResponse(response);
    }
    threadsRequest(response) {
        response.body = {
            threads: [new debugadapter_1.Thread(1, 'Main Thread')]
        };
        this.sendResponse(response);
    }
    stackTraceRequest(response, args) {
        const cached = this._frameCache.get(args.threadId);
        if (cached) {
            response.body = {
                stackFrames: cached.map(f => this._toStackFrame(f)),
                totalFrames: cached.length
            };
            this.sendResponse(response);
            return;
        }
        const seq = this._nextSeq();
        this._responseCallbacks.set(`stackTrace:${seq}`, (body) => {
            const frames = body.frames ?? [];
            this._frameCache.set(args.threadId, frames);
            response.body = {
                stackFrames: frames.map(f => this._toStackFrame(f)),
                totalFrames: frames.length
            };
            this.sendResponse(response);
        });
        this._sendToDebugProcess({
            seq,
            command: 'stackTrace',
            body: { threadId: args.threadId, startFrame: args.startFrame ?? 0, levels: args.levels ?? 20 }
        });
        // Fallback if no response within 2s
        setTimeout(() => {
            if (this._responseCallbacks.has(`stackTrace:${seq}`)) {
                this._responseCallbacks.delete(`stackTrace:${seq}`);
                response.body = { stackFrames: [], totalFrames: 0 };
                this.sendResponse(response);
            }
        }, 2000);
    }
    scopesRequest(response, args) {
        const localsHandle = this._variableHandles.create({ frameId: args.frameId, scope: 'locals' });
        const globalsHandle = this._variableHandles.create({ frameId: args.frameId, scope: 'globals' });
        response.body = {
            scopes: [
                new debugadapter_1.Scope('Locals', localsHandle, false),
                new debugadapter_1.Scope('Globals', globalsHandle, true)
            ]
        };
        this.sendResponse(response);
    }
    variablesRequest(response, args) {
        const container = this._variableHandles.get(args.variablesReference);
        if (!container) {
            response.body = { variables: [] };
            this.sendResponse(response);
            return;
        }
        const cacheKey = `${container.frameId}:${container.scope}`;
        const cached = this._variableCache.get(cacheKey);
        if (cached) {
            response.body = { variables: this._toProtocolVariables(cached) };
            this.sendResponse(response);
            return;
        }
        const seq = this._nextSeq();
        this._responseCallbacks.set(`variables:${seq}`, (body) => {
            const vars = body.variables ?? [];
            this._variableCache.set(cacheKey, vars);
            response.body = { variables: this._toProtocolVariables(vars) };
            this.sendResponse(response);
        });
        this._sendToDebugProcess({
            seq,
            command: 'variables',
            body: { frameId: container.frameId, scope: container.scope }
        });
        setTimeout(() => {
            if (this._responseCallbacks.has(`variables:${seq}`)) {
                this._responseCallbacks.delete(`variables:${seq}`);
                response.body = { variables: [] };
                this.sendResponse(response);
            }
        }, 2000);
    }
    continueRequest(response, args) {
        this._clearCaches();
        this._sendToDebugProcess({ command: 'continue', body: { threadId: args.threadId } });
        response.body = { allThreadsContinued: true };
        this.sendResponse(response);
    }
    nextRequest(response, args) {
        this._clearCaches();
        this._sendToDebugProcess({ command: 'next', body: { threadId: args.threadId } });
        this.sendResponse(response);
    }
    stepInRequest(response, args) {
        this._clearCaches();
        this._sendToDebugProcess({ command: 'stepIn', body: { threadId: args.threadId } });
        this.sendResponse(response);
    }
    stepOutRequest(response, args) {
        this._clearCaches();
        this._sendToDebugProcess({ command: 'stepOut', body: { threadId: args.threadId } });
        this.sendResponse(response);
    }
    stepBackRequest(response, args) {
        this._clearCaches();
        this._sendToDebugProcess({ command: 'stepBack', body: { threadId: args.threadId } });
        this.sendResponse(response);
    }
    restartRequest(response, _args) {
        this._clearCaches();
        this._sendToDebugProcess({ command: 'restart', body: {} });
        this.sendResponse(response);
    }
    evaluateRequest(response, args) {
        const seq = this._nextSeq();
        this._responseCallbacks.set(`evaluate:${seq}`, (body) => {
            response.body = {
                result: body.result ?? '<no result>',
                variablesReference: body.variablesReference ?? 0,
                type: body.type
            };
            this.sendResponse(response);
        });
        this._sendToDebugProcess({
            seq,
            command: 'evaluate',
            body: {
                expression: args.expression,
                frameId: args.frameId,
                context: args.context ?? 'repl'
            }
        });
        setTimeout(() => {
            if (this._responseCallbacks.has(`evaluate:${seq}`)) {
                this._responseCallbacks.delete(`evaluate:${seq}`);
                response.body = { result: '<timeout>', variablesReference: 0 };
                this.sendResponse(response);
            }
        }, 3000);
    }
    terminateRequest(response, _args) {
        this._killDebugProcess();
        this.sendResponse(response);
    }
    disconnectRequest(response, _args) {
        this._killDebugProcess();
        this.sendResponse(response);
    }
    // ── Private helpers ──────────────────────────────────────────────────────────
    _handleDebugMessage(msg) {
        if (msg.type === 'event') {
            switch (msg.event) {
                case 'stopped': {
                    const body = msg.body;
                    const reason = body?.reason ?? 'pause';
                    const threadId = body?.threadId ?? 1;
                    this._clearCaches();
                    const ev = new debugadapter_1.StoppedEvent(reason, threadId, body?.description);
                    ev.body.text = body?.text;
                    this.sendEvent(ev);
                    break;
                }
                case 'terminated':
                    this.sendEvent(new debugadapter_1.TerminatedEvent());
                    break;
                case 'output': {
                    const body = msg.body;
                    this.sendEvent(new debugadapter_1.OutputEvent(body?.output ?? '', (body?.category ?? 'stdout')));
                    break;
                }
            }
        }
        else if (msg.type === 'response') {
            const seq = msg.body?.seq;
            const command = msg.command ?? '';
            const key = `${command}:${seq}`;
            const cb = this._responseCallbacks.get(key);
            if (cb && msg.body) {
                this._responseCallbacks.delete(key);
                cb(msg.body);
            }
        }
    }
    _sendToDebugProcess(payload) {
        if (this._debugProcess?.stdin?.writable) {
            this._debugProcess.stdin.write(JSON.stringify(payload) + '\n');
        }
    }
    _killDebugProcess() {
        if (this._debugProcess) {
            try {
                this._debugProcess.kill('SIGTERM');
            }
            catch {
                // Already dead
            }
            this._debugProcess = undefined;
        }
    }
    _toStackFrame(f) {
        const src = new debugadapter_1.Source(path.basename(f.file), this.convertDebuggerPathToClient(f.file));
        return new debugadapter_1.StackFrame(f.id, f.name, src, f.line, f.column);
    }
    _toProtocolVariables(vars) {
        return vars.map(v => ({
            name: v.name,
            value: v.value,
            type: v.type,
            variablesReference: v.variablesReference,
            presentationHint: { kind: 'data' }
        }));
    }
    _clearCaches() {
        this._frameCache.clear();
        this._variableCache.clear();
        this._variableHandles.reset();
    }
    _nextSeq() {
        return String(++this._requestSeq);
    }
}
// Entry point when run as a process
OmnisystemDebugSession.run(OmnisystemDebugSession);
//# sourceMappingURL=OmnisystemDebugAdapter.js.map