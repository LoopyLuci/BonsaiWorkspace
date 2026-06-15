/**
 * Captures VSCode editor state and streams it to Bonsai via the WebSocket client.
 *
 * Events sent to Bonsai:
 *   vscode_file_tree   – workspace folder structure (on change or connect)
 *   vscode_editor_open – full file content when active editor changes
 *   vscode_editor_delta – ranged text changes (VSCode's native contentChanges)
 *   vscode_cursor      – caret position
 *   vscode_diagnostics – errors / warnings for the active file
 */

import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';
import type { BonsaiClient } from './bonsai-client';

export class StateStreamer {
  private disposables: vscode.Disposable[] = [];

  constructor(private readonly client: BonsaiClient) {}

  start(): void {
    // Initial snapshot when first connected.
    this.sendFileTree();
    this.sendActiveEditor();

    // File tree: refresh when workspace folders change or files are created/deleted.
    this.disposables.push(
      vscode.workspace.onDidChangeWorkspaceFolders(() => this.sendFileTree()),
      vscode.workspace.onDidCreateFiles(() => this.sendFileTree()),
      vscode.workspace.onDidDeleteFiles(() => this.sendFileTree()),
      vscode.workspace.onDidRenameFiles(() => this.sendFileTree()),
    );

    // Active editor: send full content on switch.
    this.disposables.push(
      vscode.window.onDidChangeActiveTextEditor(() => this.sendActiveEditor()),
    );

    // Editor deltas: send VSCode's native ranged diffs.
    this.disposables.push(
      vscode.workspace.onDidChangeTextDocument((e) => {
        if (e.contentChanges.length === 0) return;
        const editor = vscode.window.activeTextEditor;
        if (!editor || editor.document !== e.document) return;

        this.client.send({
          type: 'vscode_editor_delta',
          payload: {
            path: e.document.uri.fsPath,
            ops: e.contentChanges.map((c) => ({
              range: {
                startLine: c.range.start.line,
                startCol:  c.range.start.character,
                endLine:   c.range.end.line,
                endCol:    c.range.end.character,
              },
              newText: c.text,
            })),
          },
        });
      }),
    );

    // Cursor position.
    this.disposables.push(
      vscode.window.onDidChangeTextEditorSelection((e) => {
        const pos = e.selections[0]?.active;
        if (!pos) return;
        this.client.send({
          type: 'vscode_cursor',
          payload: {
            path: e.textEditor.document.uri.fsPath,
            line: pos.line,
            col:  pos.character,
          },
        });
      }),
    );

    // Diagnostics.
    this.disposables.push(
      vscode.languages.onDidChangeDiagnostics((e) => {
        const editor = vscode.window.activeTextEditor;
        if (!editor) return;
        const uri = editor.document.uri;
        if (!e.uris.some((u) => u.toString() === uri.toString())) return;
        this.sendDiagnostics(uri);
      }),
    );
  }

  stop(): void {
    this.disposables.forEach((d) => d.dispose());
    this.disposables = [];
  }

  // ── Private helpers ────────────────────────────────────────────────────────

  private sendFileTree(): void {
    const folders = vscode.workspace.workspaceFolders;
    if (!folders || folders.length === 0) {
      this.client.send({ type: 'vscode_file_tree', payload: { root: null, entries: [] } });
      return;
    }

    const root = folders[0].uri.fsPath;
    const entries = readDirTree(root, 2);
    this.client.send({ type: 'vscode_file_tree', payload: { root, entries } });
  }

  private sendActiveEditor(): void {
    const editor = vscode.window.activeTextEditor;
    if (!editor) {
      this.client.send({ type: 'vscode_editor_open', payload: null });
      return;
    }
    const doc     = editor.document;
    const pos     = editor.selection.active;
    this.client.send({
      type: 'vscode_editor_open',
      payload: {
        path:     doc.uri.fsPath,
        language: doc.languageId,
        content:  doc.getText(),
        cursor: { line: pos.line, col: pos.character },
      },
    });

    // Also send current diagnostics for this file.
    this.sendDiagnostics(doc.uri);
  }

  private sendDiagnostics(uri: vscode.Uri): void {
    const config = vscode.workspace.getConfiguration('bonsai');
    if (!config.get<boolean>('streamDiagnostics', true)) return;

    const diags = vscode.languages.getDiagnostics(uri);
    this.client.send({
      type: 'vscode_diagnostics',
      payload: {
        path: uri.fsPath,
        items: diags.map((d) => ({
          line:     d.range.start.line,
          col:      d.range.start.character,
          severity: severityName(d.severity),
          message:  d.message,
          source:   d.source ?? '',
        })),
      },
    });
  }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

interface FileEntry {
  name: string;
  path: string;
  kind: 'file' | 'dir';
  children?: FileEntry[];
}

function readDirTree(dirPath: string, depth: number): FileEntry[] {
  if (depth < 0) return [];
  let entries: FileEntry[] = [];
  try {
    const items = fs.readdirSync(dirPath, { withFileTypes: true });
    for (const item of items) {
      if (item.name.startsWith('.')) continue;
      const fullPath = path.join(dirPath, item.name);
      if (item.isDirectory()) {
        const SKIP = new Set(['node_modules', 'target', 'dist', 'build', '.git', 'out']);
        if (SKIP.has(item.name)) continue;
        entries.push({
          name: item.name,
          path: fullPath,
          kind: 'dir',
          children: readDirTree(fullPath, depth - 1),
        });
      } else {
        entries.push({ name: item.name, path: fullPath, kind: 'file' });
      }
    }
  } catch {
    // permission errors etc. — just skip
  }
  return entries;
}

function severityName(s: vscode.DiagnosticSeverity): string {
  switch (s) {
    case vscode.DiagnosticSeverity.Error:       return 'error';
    case vscode.DiagnosticSeverity.Warning:     return 'warning';
    case vscode.DiagnosticSeverity.Information: return 'info';
    default:                                    return 'hint';
  }
}
