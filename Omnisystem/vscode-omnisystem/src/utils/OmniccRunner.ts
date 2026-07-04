import { spawn, ChildProcess } from 'child_process';
import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';
import * as readline from 'readline';

export interface RunResult {
  exitCode: number;
  stdout: string;
  stderr: string;
}

export class OmniccRunner {
  private _omniccPath: string;

  constructor() {
    const config = vscode.workspace.getConfiguration('omnisystem');
    this._omniccPath = config.get<string>('omniccPath', 'omnicc');
  }

  /**
   * Run an omnicc command to completion and collect output.
   * Optionally streams each stdout line to `onOutput` as it arrives.
   */
  async run(
    args: string[],
    cwd?: string,
    onOutput?: (line: string) => void
  ): Promise<RunResult> {
    const resolved = await this.resolveOmniccPath();
    const binary = resolved ?? this._omniccPath;
    const workDir = cwd ?? this._workspaceRoot();

    return new Promise<RunResult>((resolve) => {
      const stdoutChunks: string[] = [];
      const stderrChunks: string[] = [];

      const proc = spawn(binary, args, {
        cwd: workDir,
        stdio: ['ignore', 'pipe', 'pipe'],
        windowsHide: true
      });

      const rl = readline.createInterface({ input: proc.stdout! });
      rl.on('line', (line: string) => {
        stdoutChunks.push(line);
        onOutput?.(line);
      });

      proc.stderr!.on('data', (chunk: Buffer) => {
        stderrChunks.push(chunk.toString());
      });

      proc.on('error', (err: NodeJS.ErrnoException) => {
        const msg = `Failed to spawn '${binary}': ${err.message}`;
        resolve({ exitCode: -1, stdout: stdoutChunks.join('\n'), stderr: msg });
      });

      proc.on('close', (code: number | null) => {
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
  spawn(args: string[], cwd?: string): ChildProcess {
    const workDir = cwd ?? this._workspaceRoot();
    return spawn(this._omniccPath, args, {
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
  async resolveOmniccPath(): Promise<string | undefined> {
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
  spawnLspServer(serverPath?: string): ChildProcess {
    const args = serverPath
      ? ['lsp', '--stdio', '--server', serverPath]
      : ['lsp', '--stdio'];

    const workDir = this._workspaceRoot();
    return spawn(this._omniccPath, args, {
      cwd: workDir,
      stdio: ['pipe', 'pipe', 'pipe'],
      windowsHide: true,
      env: {
        ...process.env,
        OMNICC_LSP_LOG: vscode.workspace
          .getConfiguration('omnisystem')
          .get<string>('lspLogLevel', 'warn'),
        OMNICC_LSP_LOG_FILE: path.join(workDir, 'omnisystem-lsp.log')
      }
    });
  }

  /**
   * Convenience: return the first workspace folder path or process.cwd().
   */
  private _workspaceRoot(): string {
    return vscode.workspace.workspaceFolders?.[0]?.uri.fsPath ?? process.cwd();
  }

  private _exists(p: string): boolean {
    try {
      fs.accessSync(p, fs.constants.X_OK);
      return true;
    } catch {
      return false;
    }
  }
}
