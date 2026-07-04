/**
 * WebSocket client that connects the VSCode extension to the Omnisystem desktop app.
 * Handles auth handshake, reconnection with exponential backoff, and message routing.
 */

import * as vscode from 'vscode';
import { DEFAULT_OMNISYSTEM_WS_URL } from './constants';

export type MessageHandler = (msg: OmnisystemMessage) => void;

export interface OmnisystemMessage {
  type: string;
  payload: unknown;
}

export class OmnisystemClient {
  private ws: WebSocket | null = null;
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private reconnectDelay = 1000;
  private readonly maxDelay = 30_000;
  private handlers: MessageHandler[] = [];
  private statusBarItem: vscode.StatusBarItem;
  private connected = false;
  private stopped = false;

  constructor(private readonly context: vscode.ExtensionContext, private readonly outputChannel: vscode.OutputChannel) {
    this.statusBarItem = vscode.window.createStatusBarItem(
      vscode.StatusBarAlignment.Right,
      100,
    );
    this.statusBarItem.command = 'omnisystem.showStatus';
    this.updateStatus('disconnected');
    this.statusBarItem.show();
  }

  get isConnected(): boolean {
    return this.connected;
  }

  onMessage(handler: MessageHandler): void {
    this.handlers.push(handler);
  }

  send(msg: OmnisystemMessage): void {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(msg));
    }
  }

  connect(): void {
    this.stopped = false;
    void this.tryConnect(); // Fire and forget
  }

  disconnect(): void {
    this.stopped = true;
    this.clearReconnect();
    this.ws?.close();
    this.ws = null;
    this.connected = false;
    this.updateStatus('disconnected');
  }

  dispose(): void {
    this.disconnect();
    this.statusBarItem.dispose();
  }

  private async tryConnect(): Promise<void> {
    if (this.stopped) return;

    const config = vscode.workspace.getConfiguration('omnisystem');
    const url    = config.get<string>('wsUrl') ?? DEFAULT_OMNISYSTEM_WS_URL;
    
    // Try to get token from SecretStorage first, fall back to workspace config
    let token = '';
    try {
      const secretToken = await this.context.secrets.get('omnisystem.pairToken');
      if (secretToken) {
        token = secretToken;
      } else {
        // Fallback to workspace config (deprecated)
        const configToken = config.get<string>('pairToken') ?? '';
        if (configToken) {
          token = configToken;
          this.log('ℹ pairToken in settings.json is deprecated — token has been migrated to SecretStorage.');
          // Migrate to SecretStorage
          await this.context.secrets.store('omnisystem.pairToken', configToken);
        }
      }
    } catch (e) {
      this.log(`Failed to retrieve token from SecretStorage: ${e}`);
      // Fall back to config
      token = config.get<string>('pairToken') ?? '';
    }

    this.updateStatus('connecting');
    this.log(`Connecting to ${url}…`);

    try {
      this.ws = new WebSocket(url);
    } catch (e) {
      this.log(`WebSocket creation failed: ${e}`);
      this.scheduleReconnect();
      return;
    }

    this.ws.onopen = () => {
      this.log('Connected — sending auth');
      this.ws!.send(JSON.stringify({ type: 'auth', payload: { token } }));
    };

    this.ws.onmessage = (ev) => {
      let msg: OmnisystemMessage;
      try {
        msg = JSON.parse(ev.data as string) as OmnisystemMessage;
      } catch {
        return;
      }

      if (msg.type === 'auth_ok') {
        this.connected = true;
        this.reconnectDelay = 1000;
        this.updateStatus('connected');
        this.log('Authenticated ✓');
        return;
      }

      if (msg.type === 'auth_fail') {
        this.log('Auth failed — check omnisystem.pairToken setting');
        this.ws?.close();
        return;
      }

      for (const h of this.handlers) {
        h(msg);
      }
    };

    this.ws.onerror = (e) => {
      this.log(`WebSocket error: ${String(e)}`);
    };

    this.ws.onclose = () => {
      this.connected = false;
      this.updateStatus('disconnected');
      this.log('Connection closed');
      this.scheduleReconnect();
    };
  }

  private scheduleReconnect(): void {
    if (this.stopped) return;
    this.clearReconnect();
    this.log(`Reconnecting in ${this.reconnectDelay / 1000}s…`);
    this.reconnectTimer = setTimeout(() => {
      this.reconnectDelay = Math.min(this.reconnectDelay * 2, this.maxDelay);
      void this.tryConnect(); // Fire and forget
    }, this.reconnectDelay);
  }

  private clearReconnect(): void {
    if (this.reconnectTimer !== null) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
  }

  private updateStatus(state: 'connected' | 'connecting' | 'disconnected'): void {
    const icons = { connected: '$(broadcast)', connecting: '$(loading~spin)', disconnected: '$(debug-disconnect)' };
    this.statusBarItem.text  = `${icons[state]} Omnisystem`;
    this.statusBarItem.tooltip = state === 'connected'
      ? 'Connected to Omnisystem Workspace'
      : state === 'connecting'
      ? 'Connecting to Omnisystem Workspace…'
      : 'Not connected to Omnisystem Workspace';
  }

  private log(msg: string): void {
    this.outputChannel.appendLine(`[omnisystem-client] ${msg}`);
  }
}
