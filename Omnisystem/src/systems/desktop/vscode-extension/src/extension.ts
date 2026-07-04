/**
 * Omnisystem Workspace Runner — VSCode Extension entry point.
 *
 * On activation:
 *   1. Creates the OmnisystemClient WebSocket connection.
 *   2. Starts StateStreamer to capture and forward VSCode state.
 *   3. Registers CommandHandler to execute Omnisystem→VSCode commands.
 *   4. Registers the connect / disconnect / status VSCode commands.
 */

import * as vscode from 'vscode';
import { OmnisystemClient } from './omnisystem-client';
import { StateStreamer } from './state-streamer';
import { handleCommand } from './command-handler';

let client:  OmnisystemClient  | null = null;
let streamer: StateStreamer | null = null;

export function activate(context: vscode.ExtensionContext): void {
  const output = vscode.window.createOutputChannel('Omnisystem Workspace Runner');
  context.subscriptions.push(output);

  client  = new OmnisystemClient(context, output);
  streamer = new StateStreamer(client);

  // Route inbound commands from Omnisystem.
  client.onMessage(async (msg) => {
    if (msg.type === 'vscode_cmd') {
      await handleCommand(msg);
    }
    // Other message types (chat_token, etc.) are ignored by the extension.
  });

  // Start streamer once the client is authenticated (first auth_ok triggers
  // the streamer via the connection lifecycle — we start it here so it's ready).
  client.onMessage((msg) => {
    if (msg.type === 'auth_ok') {
      streamer?.start();
    }
  });

  // Register VSCode commands.
  context.subscriptions.push(
    vscode.commands.registerCommand('omnisystem.connect', () => {
      client?.connect();
      vscode.window.showInformationMessage('Connecting to Omnisystem Workspace…');
    }),

    vscode.commands.registerCommand('omnisystem.disconnect', () => {
      streamer?.stop();
      client?.disconnect();
      vscode.window.showInformationMessage('Disconnected from Omnisystem Workspace.');
    }),

    vscode.commands.registerCommand('omnisystem.showStatus', () => {
      const connected = client?.isConnected ?? false;
      vscode.window.showInformationMessage(
        connected
          ? 'Omnisystem Workspace: Connected'
          : 'Omnisystem Workspace: Not connected — use "Omnisystem: Connect" to connect.',
      );
    }),
  );

  // Auto-connect if setting is enabled.
  const config = vscode.workspace.getConfiguration('omnisystem');
  if (config.get<boolean>('autoConnect', true)) {
    client.connect();
  }

  // Dispose everything on deactivation.
  context.subscriptions.push({ dispose: () => deactivateAll() });
}

export function deactivate(): void {
  deactivateAll();
}

function deactivateAll(): void {
  streamer?.stop();
  client?.dispose();
  streamer = null;
  client   = null;
}
