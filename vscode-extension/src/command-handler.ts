/**
 * Executes commands received from Bonsai Workspace in VSCode.
 *
 * Supported commands:
 *   open_file      – open a file in the editor
 *   cursor_set     – move the caret
 *   text_edit      – apply a ranged text replacement
 *   execute_command – run any VSCode command by id
 *   show_diff      – open VSCode diff editor
 */

import * as vscode from 'vscode';
import type { BonsaiMessage } from './bonsai-client';

interface OpenFileArgs      { path: string }
interface CursorSetArgs     { path: string; line: number; col: number }
interface TextEditArgs      { path: string; startLine: number; startCol: number; endLine: number; endCol: number; newText: string }
interface ExecuteCommandArgs { id: string; args?: unknown[] }
interface ShowDiffArgs      { originalPath: string; modifiedPath: string; title?: string }

export async function handleCommand(msg: BonsaiMessage): Promise<void> {
  const payload = msg.payload as Record<string, unknown>;
  const cmd     = payload['cmd'] as string | undefined;
  const args    = (payload['args'] ?? {}) as Record<string, unknown>;

  switch (cmd) {
    case 'open_file':
      await openFile(args as unknown as OpenFileArgs);
      break;

    case 'cursor_set':
      await setCursor(args as unknown as CursorSetArgs);
      break;

    case 'text_edit':
      await applyTextEdit(args as unknown as TextEditArgs);
      break;

    case 'execute_command': {
      const a = args as unknown as ExecuteCommandArgs;
      await vscode.commands.executeCommand(a.id, ...(a.args ?? []));
      break;
    }

    case 'show_diff': {
      const a = args as unknown as ShowDiffArgs;
      const orig = vscode.Uri.file(a.originalPath);
      const mod  = vscode.Uri.file(a.modifiedPath);
      await vscode.commands.executeCommand('vscode.diff', orig, mod, a.title ?? 'Bonsai Diff');
      break;
    }

    default:
      // Unknown command — ignore silently.
      break;
  }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

async function openFile({ path }: OpenFileArgs): Promise<void> {
  const uri = vscode.Uri.file(path);
  await vscode.window.showTextDocument(uri, { preview: false });
}

async function setCursor({ path, line, col }: CursorSetArgs): Promise<void> {
  const uri    = vscode.Uri.file(path);
  const doc    = await vscode.workspace.openTextDocument(uri);
  const editor = await vscode.window.showTextDocument(doc);
  const pos    = new vscode.Position(line, col);
  editor.selection = new vscode.Selection(pos, pos);
  editor.revealRange(new vscode.Range(pos, pos));
}

async function applyTextEdit({ path, startLine, startCol, endLine, endCol, newText }: TextEditArgs): Promise<void> {
  const uri    = vscode.Uri.file(path);
  const doc    = await vscode.workspace.openTextDocument(uri);
  const editor = await vscode.window.showTextDocument(doc);
  const range  = new vscode.Range(
    new vscode.Position(startLine, startCol),
    new vscode.Position(endLine,   endCol),
  );
  await editor.edit((b) => b.replace(range, newText));
}
