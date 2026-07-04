import browser from '../lib/browser';
import { BonsaiClient } from '../lib/bonsai-client';
import {
  appendAuditEntry,
  clearAuditLog,
  getAuditLog,
  getSettings,
  saveSettings
} from '../lib/storage';
import type {
  BackgroundRequest,
  BackgroundResponse,
  ExtensionEvent,
  PageSnapshot,
  SelectedElementInfo
} from '../lib/types';

let connected = false;

function makeError(error: unknown): BackgroundResponse {
  return {
    ok: false,
    error: error instanceof Error ? error.message : 'Unknown error'
  };
}

async function getActiveTabId(): Promise<number> {
  const [tab] = await browser.tabs.query({ active: true, currentWindow: true });
  if (!tab?.id) {
    throw new Error('No active tab available');
  }
  return tab.id;
}

async function sendToTab<T>(tabId: number, message: unknown): Promise<T> {
  const res = await browser.tabs.sendMessage(tabId, message);
  return res as T;
}

async function emit(event: ExtensionEvent): Promise<void> {
  await browser.runtime.sendMessage({ type: 'EXTENSION_EVENT', event });
}

async function logAudit(partial: {
  action: string;
  url: string;
  selector?: string;
  payload?: Record<string, unknown>;
  result: 'allowed' | 'denied' | 'success' | 'error';
  message?: string;
}): Promise<void> {
  await appendAuditEntry({
    id: crypto.randomUUID(),
    createdAt: new Date().toISOString(),
    ...partial
  });
  await emit({ type: 'AUDIT_UPDATED' });
}

async function handleAutomationRequest(req: Extract<BackgroundRequest, { type: 'REQUEST_AUTOMATION' }>): Promise<BackgroundResponse> {
  try {
    const tab = await browser.tabs.get(req.tabId);
    const url = tab.url ?? '';

    const allowResponse = await sendToTab<{ allowed: boolean; alwaysAllow?: boolean }>(
      req.tabId,
      {
        type: 'AUTOMATION_CONFIRM',
        payload: {
          action: req.action,
          selector: req.selector,
          text: req.text,
          url: req.url
        }
      }
    );

    if (!allowResponse.allowed) {
      await logAudit({
        action: req.action,
        url,
        selector: req.selector,
        payload: { text: req.text, url: req.url },
        result: 'denied',
        message: 'User denied automated action'
      });
      return { ok: false, error: 'User denied action' };
    }

    const execute = await sendToTab<{ ok: boolean; error?: string }>(
      req.tabId,
      {
        type: 'AUTOMATION_EXECUTE',
        payload: req
      }
    );

    if (!execute.ok) {
      await logAudit({
        action: req.action,
        url,
        selector: req.selector,
        payload: { text: req.text, url: req.url },
        result: 'error',
        message: execute.error ?? 'Execution failed'
      });
      return { ok: false, error: execute.error ?? 'Execution failed' };
    }

    await logAudit({
      action: req.action,
      url,
      selector: req.selector,
      payload: { text: req.text, url: req.url },
      result: 'success'
    });

    return { ok: true, data: { success: true } };
  } catch (error) {
    return makeError(error);
  }
}

async function handleRequest(req: BackgroundRequest): Promise<BackgroundResponse> {
  try {
    switch (req.type) {
      case 'PING':
        return { ok: true, data: { pong: true } };

      case 'CONNECT': {
        try {
          // Try the configured port first; if that fails probe 11369-11374 to
          // handle cases where the workspace bound to a non-default port.
          let didConnect = false;
          try {
            await BonsaiClient.getStatus();
            didConnect = true;
          } catch {
            const settings = await getSettings();
            const host = settings.apiHost ?? '127.0.0.1';
            const PROBE_PORTS = [11369, 11370, 11371, 11372, 11373, 11374];
            for (const port of PROBE_PORTS) {
              if (port === settings.apiPort) continue; // already tried
              try {
                const r = await fetch(`http://${host}:${port}/health`, { signal: AbortSignal.timeout(1000) });
                if (r.ok) {
                  await saveSettings({ apiPort: port });
                  didConnect = true;
                  break;
                }
              } catch { /* keep probing */ }
            }
            if (!didConnect) throw new Error(`Bonsai not found on ports ${PROBE_PORTS.join(', ')}`);
          }
          connected = true;
          await emit({ type: 'CONNECTION_STATUS', connected: true });
          return { ok: true, data: { connected: true } };
        } catch (error) {
          connected = false;
          await emit({
            type: 'CONNECTION_STATUS',
            connected: false,
            detail: error instanceof Error ? error.message : 'Unable to connect'
          });
          return makeError(error);
        }
      }

      case 'GET_STATUS':
        return { ok: true, data: { connected } };

      case 'LIST_MODELS':
        return { ok: true, data: await BonsaiClient.listModels() };

      case 'GET_SETTINGS':
        return { ok: true, data: await getSettings() };

      case 'SAVE_SETTINGS':
        return { ok: true, data: await saveSettings(req.settings) };

      case 'OPEN_WORKSPACE': {
        const settings = await getSettings();
        await browser.tabs.create({ url: settings.workspaceUrl });
        return { ok: true, data: { opened: true } };
      }

      case 'GET_PAGE_SNAPSHOT': {
        const tabId = req.tabId ?? (await getActiveTabId());
        const data = await sendToTab<PageSnapshot>(tabId, {
          type: 'GET_PAGE_SNAPSHOT',
          payload: { includeHtml: req.includeHtml ?? false }
        });
        return { ok: true, data };
      }

      case 'SUMMARIZE_CURRENT_PAGE': {
        const tabId = await getActiveTabId();
        const snapshot = await sendToTab<PageSnapshot>(tabId, {
          type: 'GET_PAGE_SNAPSHOT',
          payload: { includeHtml: false }
        });
        const response = await BonsaiClient.chat([
          {
            role: 'user',
            content: `Summarize this page in concise bullets.\\nTitle: ${snapshot.title}\\nURL: ${snapshot.url}\\n\\nContent:\\n${snapshot.visibleText.slice(0, 14000)}`
          }
        ]);
        return { ok: true, data: response };
      }

      case 'CHAT':
        return { ok: true, data: await BonsaiClient.chat(req.messages) };

      case 'CHAT_STREAM': {
        await BonsaiClient.chatStream(req.messages, async (token) => {
          await emit({ type: 'CHAT_TOKEN', streamId: req.streamId, token });
        });
        await emit({ type: 'CHAT_DONE', streamId: req.streamId });
        return { ok: true, data: { streamId: req.streamId } };
      }

      case 'REQUEST_AUTOMATION':
        return handleAutomationRequest(req);

      case 'GET_AUDIT_LOG':
        return { ok: true, data: await getAuditLog() };

      case 'CLEAR_AUDIT_LOG':
        await clearAuditLog();
        await emit({ type: 'AUDIT_UPDATED' });
        return { ok: true, data: { cleared: true } };

      default:
        return { ok: false, error: 'Unknown request type' };
    }
  } catch (error) {
    return makeError(error);
  }
}

browser.runtime.onInstalled.addListener(async () => {
  browser.contextMenus.create({
    id: 'ask-bonsai-selection',
    title: 'Ask Bonsai Buddy about selection',
    contexts: ['selection']
  });

  browser.contextMenus.create({
    id: 'edit-in-workspace',
    title: 'Edit in Bonsai Workspace',
    contexts: ['selection', 'page']
  });

  try {
    await BonsaiClient.getStatus();
    connected = true;
  } catch {
    connected = false;
  }
});

browser.runtime.onMessage.addListener((message: unknown) => {
  return handleRequest(message as BackgroundRequest);
});

browser.contextMenus.onClicked.addListener(async (info: any, tab: any) => {
  if (!tab?.id) return;

  if (info.menuItemId === 'ask-bonsai-selection' && info.selectionText) {
    const streamId = crypto.randomUUID();
    await handleRequest({
      type: 'CHAT_STREAM',
      streamId,
      messages: [
        {
          role: 'user',
          content: `Answer this selected text request:\\n${info.selectionText}`
        }
      ]
    });
  }

  if (info.menuItemId === 'edit-in-workspace') {
    const settings = await getSettings();
    const text = encodeURIComponent(info.selectionText ?? '');
    await browser.tabs.create({ url: `${settings.workspaceUrl}#import=${text}` });
  }
});

browser.omnibox.onInputChanged.addListener(async (text: string, suggest: (results: any[]) => void) => {
  try {
    const streamId = crypto.randomUUID();
    let responseText = '';
    await BonsaiClient.chatStream([{ role: 'user', content: text }], (token) => {
      responseText += token;
    });
    suggest([
      {
        content: text,
        description: responseText.slice(0, 140) || 'Bonsai Buddy is thinking...'
      }
    ]);
    await emit({ type: 'CHAT_DONE', streamId });
  } catch {
    suggest([
      {
        content: text,
        description: 'Bonsai is unavailable. Is the desktop app running?'
      }
    ]);
  }
});

browser.omnibox.onInputEntered.addListener(async (text: string) => {
  const settings = await getSettings();
  await browser.tabs.create({
    url: `${settings.workspaceUrl}#prompt=${encodeURIComponent(text)}`
  });
});

browser.commands.onCommand.addListener(async (command: string) => {
  if (command === 'open-workspace') {
    const settings = await getSettings();
    await browser.tabs.create({ url: settings.workspaceUrl });
  }
});

async function openSidebarForTab(tabId: number): Promise<void> {
  const chromeApi = globalThis.chrome;
  if (chromeApi?.sidePanel?.open) {
    chromeApi.sidePanel.open({ tabId });
    return;
  }

  if ((browser as unknown as { sidebarAction?: { open: () => Promise<void> } }).sidebarAction?.open) {
    await (browser as unknown as { sidebarAction: { open: () => Promise<void> } }).sidebarAction.open();
  }
}

browser.action.onClicked.addListener(async (tab: any) => {
  if (!tab.id) return;
  await openSidebarForTab(tab.id);
});
