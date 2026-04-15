/**
 * Bonsai Workspace Runner — VSCode Extension entry point.
 *
 * On activation:
 *   1. Creates the BonsaiClient WebSocket connection.
 *   2. Starts StateStreamer to capture and forward VSCode state.
 *   3. Registers CommandHandler to execute Bonsai→VSCode commands.
 *   4. Registers the connect / disconnect / status VSCode commands.
 */

import * as vscode from 'vscode';
import { BonsaiClient } from './bonsai-client';
import { StateStreamer } from './state-streamer';
import { handleCommand } from './command-handler';

let client:  BonsaiClient  | null = null;
let streamer: StateStreamer | null = null;

export function activate(context: vscode.ExtensionContext): void {
  const output = vscode.window.createOutputChannel('Bonsai Workspace Runner');
  context.subscriptions.push(output);

  client  = new BonsaiClient(output);
  streamer = new StateStreamer(client);

  // Route inbound commands from Bonsai.
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
    vscode.commands.registerCommand('bonsai.connect', () => {
      client?.connect();
      vscode.window.showInformationMessage('Connecting to Bonsai Workspace…');
    }),

    vscode.commands.registerCommand('bonsai.disconnect', () => {
      streamer?.stop();
      client?.disconnect();
      vscode.window.showInformationMessage('Disconnected from Bonsai Workspace.');
    }),

    vscode.commands.registerCommand('bonsai.showStatus', () => {
      const connected = client?.isConnected ?? false;
      vscode.window.showInformationMessage(
        connected
          ? 'Bonsai Workspace: Connected'
          : 'Bonsai Workspace: Not connected — use "Bonsai: Connect" to connect.',
      );
    }),
  );

  // Auto-connect if setting is enabled.
  const config = vscode.workspace.getConfiguration('bonsai');
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
