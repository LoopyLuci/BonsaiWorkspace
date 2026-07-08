# VS Code Extension — Developer Guide

## Architecture Overview

The Omnisystem VS Code extension is the primary OmniOS user interface. It is a TypeScript application that runs in VS Code's Extension Host process (Node.js) and communicates with the OmniOS runtime via the IPC protocol documented in `IPC_PROTOCOL.md`.

The extension has three communication channels:
1. **Extension Host ↔ Webview**: `vscode.Webview.postMessage` / `onDidReceiveMessage` (the standard VS Code mechanism)
2. **Extension Host ↔ OmniOS Runtime**: JSON-RPC 2.0 over stdin/stdout (Content-Length framing)
3. **Extension Host ↔ Language Server**: JSON-RPC 2.0 over stdio (standard LSP)

---

## File Structure

```
vscode-omnisystem/
├── package.json                    Extension manifest
├── tsconfig.json
├── .vscodeignore
├── src/
│   ├── extension.ts                Activation entry point
│   ├── runtime/
│   │   ├── RuntimeProcess.ts       Process lifecycle management
│   │   ├── RuntimeClient.ts        IPC channel and message routing
│   │   └── RuntimeProtocol.ts      TypeScript types for all IPC messages
│   ├── providers/
│   │   ├── OmniTreeProvider.ts     Sidebar: project explorer
│   │   ├── BonsaiTreeProvider.ts   Sidebar: Bonsai ecosystem
│   │   ├── BuildTreeProvider.ts    Sidebar: build history
│   │   └── PackageTreeProvider.ts  Sidebar: installed packages
│   ├── webviews/
│   │   ├── OmniOSDesktop.ts        Main desktop panel
│   │   ├── BonsaiDashboard.ts      Bonsai dashboard panel
│   │   └── BuildDashboard.ts       Standalone build output panel
│   └── lsp/
│       └── LanguageClient.ts       LSP connection management
├── out/                            Compiled JavaScript (git-ignored)
├── icons/                          Extension and language icons
├── syntaxes/                       TextMate grammars for all 7 languages
├── snippets/                       Code snippets for all 7 languages
└── themes/                         Omnisystem Dark color theme
```

---

## `extension.ts` — Activation

```typescript
import * as vscode from 'vscode';
import { RuntimeProcess } from './runtime/RuntimeProcess';
import { RuntimeClient } from './runtime/RuntimeClient';
import { OmniOSDesktopPanel } from './webviews/OmniOSDesktop';
import { startLanguageClient } from './lsp/LanguageClient';
import { OmniTreeProvider } from './providers/OmniTreeProvider';
// ... other imports

let runtimeProcess: RuntimeProcess;
let runtimeClient: RuntimeClient;

export async function activate(context: vscode.ExtensionContext): Promise<void> {
    const outputChannel = vscode.window.createOutputChannel('Omnisystem');

    // Start the OmniOS runtime process
    runtimeProcess = new RuntimeProcess(outputChannel);
    runtimeClient = new RuntimeClient(runtimeProcess, outputChannel);
    await runtimeProcess.start();

    // Register tree view providers
    const omniTree = new OmniTreeProvider(runtimeClient);
    vscode.window.registerTreeDataProvider('omnisystemExplorer', omniTree);
    // ... register other providers

    // Register all commands
    context.subscriptions.push(
        vscode.commands.registerCommand('omnisystem.omniOsBoot', () => {
            OmniOSDesktopPanel.createOrShow(context.extensionUri, runtimeClient);
        }),
        vscode.commands.registerCommand('omnisystem.build', async () => {
            await runtimeClient.build.start({target: getDefaultTarget(), opt: getDefaultOpt()});
        }),
        // ... all other commands
    );

    // Start the Language Server
    await startLanguageClient(context, outputChannel);

    outputChannel.appendLine('Omnisystem extension activated');
}

export function deactivate(): void {
    runtimeProcess?.stop();
}
```

---

## `RuntimeProcess.ts` — Process Lifecycle

```typescript
import * as child_process from 'child_process';
import * as events from 'events';

export class RuntimeProcess extends events.EventEmitter {
    private process: child_process.ChildProcess | null = null;
    private restartAttempts = 0;
    private readonly MAX_RESTARTS = 3;
    private heartbeatInterval: NodeJS.Timeout | null = null;

    constructor(
        private readonly outputChannel: vscode.OutputChannel,
        private readonly omniccPath: string = resolvedOmniccPath()
    ) { super(); }

    async start(): Promise<void> {
        this.process = child_process.spawn(
            this.omniccPath,
            ['runtime', '--ipc'],
            {
                cwd: workspaceRoot() ?? process.cwd(),
                stdio: ['pipe', 'pipe', 'pipe'],
                windowsHide: true
            }
        );

        this.process.on('exit', (code) => {
            this.outputChannel.appendLine(`Runtime exited with code ${code}`);
            this.emit('runtimeCrashed', code);
            this.scheduleRestart();
        });

        this.process.stderr!.on('data', (data: Buffer) => {
            this.outputChannel.appendLine('[runtime] ' + data.toString().trim());
        });

        this.startHeartbeat();
        this.emit('runtimeReady');
        this.restartAttempts = 0;
    }

    private scheduleRestart(): void {
        if (this.restartAttempts >= this.MAX_RESTARTS) {
            this.outputChannel.appendLine('Runtime crashed too many times. Manual restart required.');
            return;
        }
        const delay = Math.pow(2, this.restartAttempts) * 500; // 500ms, 1s, 2s
        this.restartAttempts++;
        this.outputChannel.appendLine(`Restarting runtime in ${delay}ms (attempt ${this.restartAttempts}/${this.MAX_RESTARTS})`);
        setTimeout(() => {
            this.start();
            this.emit('runtimeRestarted');
        }, delay);
    }

    private startHeartbeat(): void {
        let missedHeartbeats = 0;
        this.heartbeatInterval = setInterval(async () => {
            try {
                await this.client!.ping();
                missedHeartbeats = 0;
            } catch {
                missedHeartbeats++;
                if (missedHeartbeats >= 3) {
                    this.outputChannel.appendLine('Runtime not responding to heartbeat — killing');
                    this.process?.kill('SIGKILL');
                }
            }
        }, 5000);
    }

    get stdin(): NodeJS.WritableStream { return this.process!.stdin!; }
    get stdout(): NodeJS.ReadableStream { return this.process!.stdout!; }

    stop(): void {
        if (this.heartbeatInterval) clearInterval(this.heartbeatInterval);
        this.process?.kill('SIGTERM');
    }
}
```

---

## `RuntimeClient.ts` — IPC Channel

```typescript
import { EventEmitter } from 'events';

interface PendingRequest {
    resolve: (result: unknown) => void;
    reject: (error: Error) => void;
    timeout: NodeJS.Timeout;
}

export class RuntimeClient {
    private pendingRequests = new Map<number, PendingRequest>();
    private notificationListeners = new Map<string, Set<(params: unknown) => void>>();
    private nextId = 1;
    private buffer = Buffer.alloc(0);

    constructor(private readonly runtimeProcess: RuntimeProcess) {
        runtimeProcess.stdout.on('data', (chunk: Buffer) => this.onData(chunk));
    }

    // Send a request and await the response
    async sendRequest<T = unknown>(method: string, params: unknown = {}): Promise<T> {
        return new Promise<T>((resolve, reject) => {
            const id = this.nextId++;
            const timeout = setTimeout(() => {
                this.pendingRequests.delete(id);
                reject(new Error(`IPC timeout: ${method} (30s)`));
            }, 30000);

            this.pendingRequests.set(id, { resolve: resolve as any, reject, timeout });
            this.send({ jsonrpc: '2.0', id, method, params });
        });
    }

    // Register a listener for notification events (no id in the message)
    onNotification(method: string, handler: (params: unknown) => void): () => void {
        if (!this.notificationListeners.has(method)) {
            this.notificationListeners.set(method, new Set());
        }
        this.notificationListeners.get(method)!.add(handler);
        return () => this.notificationListeners.get(method)?.delete(handler);
    }

    private send(message: object): void {
        const json = JSON.stringify(message);
        const frame = `Content-Length: ${Buffer.byteLength(json)}\r\n\r\n${json}`;
        this.runtimeProcess.stdin.write(frame);
    }

    private onData(chunk: Buffer): void {
        this.buffer = Buffer.concat([this.buffer, chunk]);
        while (true) {
            const headerEnd = this.buffer.indexOf('\r\n\r\n');
            if (headerEnd === -1) break;
            const header = this.buffer.slice(0, headerEnd).toString();
            const match = header.match(/Content-Length: (\d+)/);
            if (!match) { this.buffer = this.buffer.slice(headerEnd + 4); continue; }
            const contentLength = parseInt(match[1]);
            if (this.buffer.length < headerEnd + 4 + contentLength) break;
            const json = this.buffer.slice(headerEnd + 4, headerEnd + 4 + contentLength).toString();
            this.buffer = this.buffer.slice(headerEnd + 4 + contentLength);
            this.dispatchMessage(JSON.parse(json));
        }
    }

    private dispatchMessage(msg: any): void {
        if (msg.id !== undefined && this.pendingRequests.has(msg.id)) {
            const pending = this.pendingRequests.get(msg.id)!;
            this.pendingRequests.delete(msg.id);
            clearTimeout(pending.timeout);
            if (msg.error) {
                pending.reject(new Error(`${msg.error.message} (code: ${msg.error.code})`));
            } else {
                pending.resolve(msg.result);
            }
        } else if (msg.method) {
            const listeners = this.notificationListeners.get(msg.method);
            if (listeners) listeners.forEach(fn => fn(msg.params));
        }
    }

    // Typed convenience methods
    readonly fs = {
        readDir: (path: string) => this.sendRequest<FsReadDirResult>('fs.readDir', {path}),
        readFile: (path: string) => this.sendRequest<FsReadFileResult>('fs.readFile', {path}),
        writeFile: (path: string, content: string) => this.sendRequest<FsWriteFileResult>('fs.writeFile', {path, content}),
        delete: (path: string, recursive = false) => this.sendRequest<FsDeleteResult>('fs.delete', {path, recursive}),
        move: (src: string, dst: string) => this.sendRequest<FsMoveResult>('fs.move', {src, dst}),
        search: (root: string, nameGlob: string, contentPattern?: string) =>
            this.sendRequest<FsSearchResult>('fs.search', {root, nameGlob, contentPattern}),
        watch: (path: string, recursive = true) => this.sendRequest<{watchId: string}>('fs.watch', {path, recursive}),
        unwatch: (watchId: string) => this.sendRequest('fs.unwatch', {watchId}),
    };

    readonly build = {
        start: (opts: BuildStartParams) => this.sendRequest<BuildResult>('build.start', opts),
        cancel: () => this.sendRequest('build.cancel'),
        watch: (opts: BuildWatchParams) => this.sendRequest<{watchId: string}>('build.watch', opts),
    };

    readonly term = {
        create: (opts: TermCreateParams) => this.sendRequest<TermCreateResult>('term.create', opts),
        kill: (id: string) => this.sendRequest('term.kill', {id}),
    };

    readonly pm = {
        list: () => this.sendRequest<PmListResult>('pm.list'),
        search: (query: string) => this.sendRequest<PmSearchResult>('pm.search', {query}),
        install: (pkg: string, version?: string) => this.sendRequest<PmInstallResult>('pm.install', {pkg, version}),
        remove: (pkg: string) => this.sendRequest<PmRemoveResult>('pm.remove', {pkg}),
        update: (pkg?: string) => this.sendRequest<PmUpdateResult>('pm.update', {pkg}),
        audit: () => this.sendRequest<PmAuditResult>('pm.audit'),
    };

    readonly system = {
        stats: () => this.sendRequest<SystemStatsResult>('system.stats'),
        health: () => this.sendRequest<SystemHealthResult>('system.health'),
    };

    async ping(): Promise<void> {
        await this.sendRequest('ping');
    }
}
```

---

## `OmniOSDesktop.ts` — Panel and Message Routing

The `_handleMessage` method is the central router between the webview and the runtime. Every message from the webview JS calls a real IPC operation:

```typescript
private async _handleMessage(msg: {command: string; [key: string]: unknown}): Promise<void> {
    switch (msg.command) {
        // File system
        case 'getFiles': {
            const path = (msg.path as string) || workspaceRoot() || '';
            const result = await this.runtimeClient.fs.readDir(path);
            this._panel.webview.postMessage({type: 'fileList', ...result});
            break;
        }

        case 'openFile': {
            const uri = vscode.Uri.file(msg.text as string);
            await vscode.window.showTextDocument(uri);
            await this.runtimeClient.fs.readFile(msg.text as string); // update recent files
            break;
        }

        // Build
        case 'runBuild': {
            const args = msg.args as string[];
            const unsubLine = this.runtimeClient.onNotification('build.line', (params: any) => {
                this._panel.webview.postMessage({type: 'buildLine', ...params});
            });
            const unsubPhase = this.runtimeClient.onNotification('build.phase', (params: any) => {
                this._panel.webview.postMessage({type: 'buildPhase', ...params});
            });
            try {
                const result = await this.runtimeClient.build.start({
                    target: args[args.indexOf('--target') + 1] ?? 'x86_64-windows',
                    opt: args[args.indexOf('--opt') + 1] ?? 'O2',
                    flags: args,
                    cwd: workspaceRoot() ?? process.cwd()
                });
                this._panel.webview.postMessage({type: 'buildDone', ...result});
            } finally {
                unsubLine();
                unsubPhase();
            }
            break;
        }

        // Terminal input
        case 'termInput': {
            this.runtimeClient.send('term.input', {id: msg.id, data: msg.data}); // notification
            break;
        }

        // ... all other cases
    }
}
```

---

## Adding a New App

1. **Add the app to `appMeta`** in the webview JS:
   ```javascript
   'my-app': {title: 'My App', icon: '🔧'}
   ```

2. **Add a desktop icon** in the HTML:
   ```html
   <div class="desktop-icon" data-app="my-app">
     <div class="di-icon" style="background:linear-gradient(...)">🔧</div>
     <div class="di-label">My App</div>
   </div>
   ```

3. **Add a start menu entry:**
   ```html
   <div class="sm-app-btn" data-app="my-app">
     <div class="sm-app-icon" style="...">🔧</div>
     <div class="sm-app-name">My App</div>
   </div>
   ```

4. **Implement `buildMyApp(container)`** in the webview JS:
   ```javascript
   function buildMyApp(c) {
     c.innerHTML = '<div class="app-container">...</div>';
     // Wire up buttons, IPC calls, etc.
   }
   ```

5. **Add the case to `buildAppContent`:**
   ```javascript
   case 'my-app': buildMyApp(container); break;
   ```

6. **Add any new IPC commands** to:
   - `bin/omnicc.js` (runtime handler)
   - `RuntimeClient.ts` (typed wrapper)
   - `OmniOSDesktop._handleMessage` (routing)
   - `IPC_PROTOCOL.md` (documentation)

---

## Building and Packaging

```bash
cd vscode-omnisystem

# Install dependencies
npm install

# Compile TypeScript
npm run compile

# Package VSIX
npx vsce package

# Install for testing
code --install-extension omnisystem-2.0.0.vsix --force

# Reload VS Code
# Ctrl+Shift+P → Developer: Reload Window
```

### Continuous Build During Development
```bash
npm run watch    # watch mode: recompiles on save
```

Then use `code --install-extension` and "Reload Window" after each meaningful change.

---

## Debugging the Extension

### Extension Host
1. Open the `vscode-omnisystem` folder in a second VS Code window
2. Press F5 to launch the Extension Development Host
3. Set breakpoints in `src/*.ts` files
4. Use the Debug Console to evaluate expressions

### Webview
1. In the Extension Development Host, run the command `OmniOS: Launch Desktop`
2. Press `Ctrl+Shift+I` to open webview DevTools
3. Console, Sources, and Network tabs are all available
4. All `console.log()` calls in the webview JS appear in this DevTools console

### Runtime Process
Add `--debug` flag to the spawn args in `RuntimeProcess.ts` to enable verbose runtime logging to `outputChannel`.

View runtime output: View → Output → select "Omnisystem" from the dropdown.
