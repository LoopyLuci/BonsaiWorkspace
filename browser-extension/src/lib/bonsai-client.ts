import type { ChatMessage } from './types';
import { getSettings } from './storage';

interface ChatCompletionChunk {
  choices?: Array<{
    delta?: {
      content?: string;
    };
  }>;
}

async function getAuthHeader(): Promise<HeadersInit> {
  const settings = await getSettings();
  if (!settings.desktopConnectionToken) {
    return { 'Content-Type': 'application/json' };
  }

  return {
    'Content-Type': 'application/json',
    Authorization: `Bearer ${settings.desktopConnectionToken}`
  };
}

async function checkedJsonFetch(url: string, init?: RequestInit): Promise<unknown> {
  const response = await fetch(url, init);
  if (!response.ok) {
    const body = await response.text();
    throw new Error(`HTTP ${response.status}: ${body}`);
  }
  return response.json();
}

export const BonsaiClient = {
  async getStatus(): Promise<unknown> {
    const settings = await getSettings();
    const url = `http://${settings.apiHost}:${settings.apiPort}/health`;
    return checkedJsonFetch(url);
  },

  async listModels(): Promise<unknown> {
    const settings = await getSettings();
    const url = `http://${settings.apiHost}:${settings.apiPort}/v1/models`;
    return checkedJsonFetch(url, {
      headers: await getAuthHeader()
    });
  },

  async getHardwareInfo(): Promise<unknown> {
    const settings = await getSettings();
    const url = `http://${settings.apiHost}:${settings.apiPort}/api/hardware`;
    return checkedJsonFetch(url, {
      headers: await getAuthHeader()
    });
  },

  async chat(messages: ChatMessage[]): Promise<unknown> {
    const settings = await getSettings();
    const url = `http://${settings.buddyHost}:${settings.buddyPort}/v1/chat/completions`;
    return checkedJsonFetch(url, {
      method: 'POST',
      headers: await getAuthHeader(),
      body: JSON.stringify({
        model: settings.defaultModel,
        messages,
        stream: false
      })
    });
  },

  async chatStream(messages: ChatMessage[], onToken: (token: string) => void): Promise<void> {
    const settings = await getSettings();
    const url = `http://${settings.buddyHost}:${settings.buddyPort}/v1/chat/completions`;
    const response = await fetch(url, {
      method: 'POST',
      headers: await getAuthHeader(),
      body: JSON.stringify({
        model: settings.defaultModel,
        messages,
        stream: true
      })
    });

    if (!response.ok || !response.body) {
      const body = await response.text();
      throw new Error(`Streaming failed (${response.status}): ${body}`);
    }

    const decoder = new TextDecoder();
    const reader = response.body.getReader();
    let buffered = '';

    while (true) {
      const { done, value } = await reader.read();
      if (done) break;

      buffered += decoder.decode(value, { stream: true });
      const lines = buffered.split('\n');
      buffered = lines.pop() ?? '';

      for (const lineRaw of lines) {
        const line = lineRaw.trim();
        if (!line.startsWith('data:')) continue;
        const payload = line.slice(5).trim();
        if (payload === '[DONE]') return;

        try {
          const parsed = JSON.parse(payload) as ChatCompletionChunk;
          const token = parsed.choices?.[0]?.delta?.content;
          if (token) onToken(token);
        } catch {
          // Ignore malformed chunks to keep the stream alive.
        }
      }
    }
  },

  async invokeTool(name: string, args: Record<string, unknown>): Promise<unknown> {
    const settings = await getSettings();
    const url = `http://${settings.apiHost}:${settings.apiPort}/api/tools/invoke`;
    return checkedJsonFetch(url, {
      method: 'POST',
      headers: await getAuthHeader(),
      body: JSON.stringify({ name, args })
    });
  }
};
