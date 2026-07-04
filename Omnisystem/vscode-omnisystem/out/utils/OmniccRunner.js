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
exports.OmniccRunner = void 0;
const child_process_1 = require("child_process");
const vscode = __importStar(require("vscode"));
const path = __importStar(require("path"));
const fs = __importStar(require("fs"));
const readline = __importStar(require("readline"));
class OmniccRunner {
    constructor() {
        const config = vscode.workspace.getConfiguration('omnisystem');
        this._omniccPath = config.get('omniccPath', 'omnicc');
    }
    /**
     * Run an omnicc command to completion and collect output.
     * Optionally streams each stdout line to `onOutput` as it arrives.
     */
    async run(args, cwd, onOutput) {
        const resolved = await this.resolveOmniccPath();
        const binary = resolved ?? this._omniccPath;
        const workDir = cwd ?? this._workspaceRoot();
        return new Promise((resolve) => {
            const stdoutChunks = [];
            const stderrChunks = [];
            const proc = (0, child_process_1.spawn)(binary, args, {
                cwd: workDir,
                stdio: ['ignore', 'pipe', 'pipe'],
                windowsHide: true
            });
            const rl = readline.createInterface({ input: proc.stdout });
            rl.on('line', (line) => {
                stdoutChunks.push(line);
                onOutput?.(line);
            });
            proc.stderr.on('data', (chunk) => {
                stderrChunks.push(chunk.toString());
            });
            proc.on('error', (err) => {
                const msg = `Failed to spawn '${binary}': ${err.message}`;
                resolve({ exitCode: -1, stdout: stdoutChunks.join('\n'), stderr: msg });
            });
            proc.on('close', (code) => {
                resolve({
                    exitCode: code ?? 0,
                    stdout: stdoutChunks.join('\n'),
                    stderr: stderrChunks.join('')
                });
            });
        });
    }
    /**
     * Spawn an omnicc process and return the child immediately (fire-and-forget style).
     */
    spawn(args, cwd) {
        const workDir = cwd ?? this._workspaceRoot();
        return (0, child_process_1.spawn)(this._omniccPath, args, {
            cwd: workDir,
            stdio: ['pipe', 'pipe', 'pipe'],
            windowsHide: true
        });
    }
    /**
     * Try to find the omnicc binary.  Checks (in order):
     *   1. The path from extension config
     *   2. Alongside this extension's dist folder (packaged scenario)
     *   3. Relies on PATH (returns undefined if not found by the above two)
     */
    async resolveOmniccPath() {
        // 1. Config-provided path
        const configPath = this._omniccPath;
        if (configPath !== 'omnicc' && this._exists(configPath)) {
            return configPath;
        }
        // 2. Next to the extension's out/ directory
        const extensionDir = path.resolve(__dirname, '..', '..');
        const candidates = [
            path.join(extensionDir, 'bin', 'omnicc'),
            path.join(extensionDir, 'bin', 'omnicc.exe'),
            path.join(extensionDir, 'omnicc'),
            path.join(extensionDir, 'omnicc.exe')
        ];
        for (const candidate of candidates) {
            if (this._exists(candidate)) {
                return candidate;
            }
        }
        // 3. Let the OS resolve it via PATH
        return undefined;
    }
    /**
     * Spawn `omnicc lsp --stdio` and return the live child process.
     * VS Code's LanguageClient expects to read/write stdio of this process.
     */
    spawnLspServer(serverPath) {
        const args = serverPath
            ? ['lsp', '--stdio', '--server', serverPath]
            : ['lsp', '--stdio'];
        const workDir = this._workspaceRoot();
        return (0, child_process_1.spawn)(this._omniccPath, args, {
            cwd: workDir,
            stdio: ['pipe', 'pipe', 'pipe'],
            windowsHide: true,
            env: {
                ...process.env,
                OMNICC_LSP_LOG: vscode.workspace
                    .getConfiguration('omnisystem')
                    .get('lspLogLevel', 'warn'),
                OMNICC_LSP_LOG_FILE: path.join(workDir, 'omnisystem-lsp.log')
            }
        });
    }
    /**
     * Convenience: return the first workspace folder path or process.cwd().
     */
    _workspaceRoot() {
        return vscode.workspace.workspaceFolders?.[0]?.uri.fsPath ?? process.cwd();
    }
    _exists(p) {
        try {
            fs.accessSync(p, fs.constants.X_OK);
            return true;
        }
        catch {
            return false;
        }
    }
}
exports.OmniccRunner = OmniccRunner;
//# sourceMappingURL=OmniccRunner.js.map