import {
  DebugSession,
  InitializedEvent,
  TerminatedEvent,
  StoppedEvent,
  OutputEvent,
  Thread,
  StackFrame,
  Scope,
  Source,
  Handles,
  Breakpoint
} from '@vscode/debugadapter';
import { DebugProtocol } from '@vscode/debugprotocol';
import { spawn, ChildProcess } from 'child_process';
import * as path from 'path';
import * as readline from 'readline';

interface LaunchRequestArguments extends DebugProtocol.LaunchRequestArguments {
  program: string;
  args?: string[];
  cwd?: string;
  stopOnEntry?: boolean;
  omniccPath?: string;
  target?: string;
}

interface VariableContainer {
  frameId: number;
  scope: 'locals' | 'globals';
}

interface DapMessage {
  type: string;
  event?: string;
  command?: string;
  body?: Record<string, unknown>;
}

interface DebugFrame {
  id: number;
  name: string;
  file: string;
  line: number;
  column: number;
}

interface DebugVariable {
  name: string;
  value: string;
  type: string;
  variablesReference: number;
}

interface DebugStopEvent {
  reason: string;
  threadId: number;
  description?: string;
  text?: string;
}

class OmnisystemDebugSession extends DebugSession {
  private _debugProcess: ChildProcess | undefined;
  private _variableHandles = new Handles<VariableContainer>();
  private _configurationDone = false;
  private _pendingBreakpoints: Map<string, DebugProtocol.SourceBreakpoint[]> = new Map();
  private _frameCache: Map<number, DebugFrame[]> = new Map();
  private _variableCache: Map<string, DebugVariable[]> = new Map();
  private _responseCallbacks: Map<string, (body: Record<string, unknown>) => void> = new Map();
  private _requestSeq = 0;

  constructor() {
    super();
    this.setDebuggerLinesStartAt1(true);
    this.setDebuggerColumnsStartAt1(true);
  }

  protected initializeRequest(
    response: DebugProtocol.InitializeResponse,
    _args: DebugProtocol.InitializeRequestArguments
  ): void {
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
    this.sendEvent(new InitializedEvent());
  }

  protected configurationDoneRequest(
    response: DebugProtocol.ConfigurationDoneResponse,
    _args: DebugProtocol.ConfigurationDoneArguments
  ): void {
    this._configurationDone = true;
    this.sendResponse(response);
  }

  protected async launchRequest(
    response: DebugProtocol.LaunchResponse,
    args: LaunchRequestArguments
  ): Promise<void> {
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

    this.sendEvent(new OutputEvent(`Launching: ${omniccPath} ${spawnArgs.join(' ')}\n`, 'console'));

    this._debugProcess = spawn(omniccPath, spawnArgs, {
      cwd,
      stdio: ['pipe', 'pipe', 'pipe']
    });

    const rl = readline.createInterface({ input: this._debugProcess.stdout! });

    rl.on('line', (line: string) => {
      if (!line.trim()) return;
      try {
        const msg: DapMessage = JSON.parse(line);
        this._handleDebugMessage(msg);
      } catch {
        // Plain output line
        this.sendEvent(new OutputEvent(line + '\n', 'stdout'));
      }
    });

    this._debugProcess.stderr!.on('data', (data: Buffer) => {
      this.sendEvent(new OutputEvent(data.toString(), 'stderr'));
    });

    this._debugProcess.on('exit', (code: number | null) => {
      this.sendEvent(new OutputEvent(`Process exited with code ${code ?? 0}\n`, 'console'));
      this.sendEvent(new TerminatedEvent());
    });

    this._debugProcess.on('error', (err: Error) => {
      this.sendEvent(new OutputEvent(`Failed to start omnicc: ${err.message}\n`, 'stderr'));
      this.sendEvent(new TerminatedEvent());
    });

    // Wait for configurationDone before resuming if stopOnEntry
    if (args.stopOnEntry) {
      this._sendToDebugProcess({ command: 'stopOnEntry', body: { stop: true } });
    }

    this.sendResponse(response);
  }

  protected setBreakPointsRequest(
    response: DebugProtocol.SetBreakpointsResponse,
    args: DebugProtocol.SetBreakpointsArguments
  ): void {
    const sourcePath = args.source.path ?? '';
    const requestedBps = args.breakpoints ?? [];

    this._pendingBreakpoints.set(sourcePath, requestedBps);

    const breakpoints: Breakpoint[] = requestedBps.map((bp, idx) => {
      const verified = this._debugProcess !== undefined;
      const b = new Breakpoint(verified, bp.line, bp.column);
      (b as DebugProtocol.Breakpoint).id = idx;
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

  protected breakpointLocationsRequest(
    response: DebugProtocol.BreakpointLocationsResponse,
    args: DebugProtocol.BreakpointLocationsArguments
  ): void {
    response.body = {
      breakpoints: [
        { line: args.line, column: args.column ?? 0 }
      ]
    };
    this.sendResponse(response);
  }

  protected threadsRequest(response: DebugProtocol.ThreadsResponse): void {
    response.body = {
      threads: [new Thread(1, 'Main Thread')]
    };
    this.sendResponse(response);
  }

  protected stackTraceRequest(
    response: DebugProtocol.StackTraceResponse,
    args: DebugProtocol.StackTraceArguments
  ): void {
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
      const frames = (body.frames as DebugFrame[] | undefined) ?? [];
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

  protected scopesRequest(
    response: DebugProtocol.ScopesResponse,
    args: DebugProtocol.ScopesArguments
  ): void {
    const localsHandle = this._variableHandles.create({ frameId: args.frameId, scope: 'locals' });
    const globalsHandle = this._variableHandles.create({ frameId: args.frameId, scope: 'globals' });

    response.body = {
      scopes: [
        new Scope('Locals', localsHandle, false),
        new Scope('Globals', globalsHandle, true)
      ]
    };
    this.sendResponse(response);
  }

  protected variablesRequest(
    response: DebugProtocol.VariablesResponse,
    args: DebugProtocol.VariablesArguments
  ): void {
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
      const vars = (body.variables as DebugVariable[] | undefined) ?? [];
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

  protected continueRequest(
    response: DebugProtocol.ContinueResponse,
    args: DebugProtocol.ContinueArguments
  ): void {
    this._clearCaches();
    this._sendToDebugProcess({ command: 'continue', body: { threadId: args.threadId } });
    response.body = { allThreadsContinued: true };
    this.sendResponse(response);
  }

  protected nextRequest(
    response: DebugProtocol.NextResponse,
    args: DebugProtocol.NextArguments
  ): void {
    this._clearCaches();
    this._sendToDebugProcess({ command: 'next', body: { threadId: args.threadId } });
    this.sendResponse(response);
  }

  protected stepInRequest(
    response: DebugProtocol.StepInResponse,
    args: DebugProtocol.StepInArguments
  ): void {
    this._clearCaches();
    this._sendToDebugProcess({ command: 'stepIn', body: { threadId: args.threadId } });
    this.sendResponse(response);
  }

  protected stepOutRequest(
    response: DebugProtocol.StepOutResponse,
    args: DebugProtocol.StepOutArguments
  ): void {
    this._clearCaches();
    this._sendToDebugProcess({ command: 'stepOut', body: { threadId: args.threadId } });
    this.sendResponse(response);
  }

  protected stepBackRequest(
    response: DebugProtocol.StepBackResponse,
    args: DebugProtocol.StepBackArguments
  ): void {
    this._clearCaches();
    this._sendToDebugProcess({ command: 'stepBack', body: { threadId: args.threadId } });
    this.sendResponse(response);
  }

  protected restartRequest(
    response: DebugProtocol.RestartResponse,
    _args: DebugProtocol.RestartArguments
  ): void {
    this._clearCaches();
    this._sendToDebugProcess({ command: 'restart', body: {} });
    this.sendResponse(response);
  }

  protected evaluateRequest(
    response: DebugProtocol.EvaluateResponse,
    args: DebugProtocol.EvaluateArguments
  ): void {
    const seq = this._nextSeq();
    this._responseCallbacks.set(`evaluate:${seq}`, (body) => {
      response.body = {
        result: (body.result as string | undefined) ?? '<no result>',
        variablesReference: (body.variablesReference as number | undefined) ?? 0,
        type: body.type as string | undefined
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

  protected terminateRequest(
    response: DebugProtocol.TerminateResponse,
    _args: DebugProtocol.TerminateArguments
  ): void {
    this._killDebugProcess();
    this.sendResponse(response);
  }

  protected disconnectRequest(
    response: DebugProtocol.DisconnectResponse,
    _args: DebugProtocol.DisconnectArguments
  ): void {
    this._killDebugProcess();
    this.sendResponse(response);
  }

  // ── Private helpers ──────────────────────────────────────────────────────────

  private _handleDebugMessage(msg: DapMessage): void {
    if (msg.type === 'event') {
      switch (msg.event) {
        case 'stopped': {
          const body = msg.body as unknown as DebugStopEvent | undefined;
          const reason = body?.reason ?? 'pause';
          const threadId = body?.threadId ?? 1;
          this._clearCaches();
          const ev = new StoppedEvent(reason, threadId, body?.description);
          (ev as DebugProtocol.StoppedEvent).body.text = body?.text;
          this.sendEvent(ev);
          break;
        }
        case 'terminated':
          this.sendEvent(new TerminatedEvent());
          break;
        case 'output': {
          const body = msg.body as { output?: string; category?: string } | undefined;
          this.sendEvent(new OutputEvent(body?.output ?? '', (body?.category ?? 'stdout') as string));
          break;
        }
      }
    } else if (msg.type === 'response') {
      const seq = (msg.body as { seq?: number } | undefined)?.seq;
      const command = msg.command ?? '';
      const key = `${command}:${seq}`;
      const cb = this._responseCallbacks.get(key);
      if (cb && msg.body) {
        this._responseCallbacks.delete(key);
        cb(msg.body);
      }
    }
  }

  private _sendToDebugProcess(payload: Record<string, unknown>): void {
    if (this._debugProcess?.stdin?.writable) {
      this._debugProcess.stdin.write(JSON.stringify(payload) + '\n');
    }
  }

  private _killDebugProcess(): void {
    if (this._debugProcess) {
      try {
        this._debugProcess.kill('SIGTERM');
      } catch {
        // Already dead
      }
      this._debugProcess = undefined;
    }
  }

  private _toStackFrame(f: DebugFrame): StackFrame {
    const src = new Source(
      path.basename(f.file),
      this.convertDebuggerPathToClient(f.file)
    );
    return new StackFrame(f.id, f.name, src, f.line, f.column);
  }

  private _toProtocolVariables(vars: DebugVariable[]): DebugProtocol.Variable[] {
    return vars.map(v => ({
      name: v.name,
      value: v.value,
      type: v.type,
      variablesReference: v.variablesReference,
      presentationHint: { kind: 'data' }
    }));
  }

  private _clearCaches(): void {
    this._frameCache.clear();
    this._variableCache.clear();
    this._variableHandles.reset();
  }

  private _nextSeq(): string {
    return String(++this._requestSeq);
  }
}

// Entry point when run as a process
OmnisystemDebugSession.run(OmnisystemDebugSession);
