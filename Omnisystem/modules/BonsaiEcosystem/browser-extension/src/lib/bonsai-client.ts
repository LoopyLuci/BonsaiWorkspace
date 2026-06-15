import type { ChatMessage } from './types';
import { getSettings } from './storage';

interface ChatCompletionChunk {
  choices?: Array<{
    delta?: {
      content?: string;
    };
  }>;
}

interface ToolCallResponse {
  choices?: Array<{
    finish_reason?: string;
    message?: {
      content?: string;
      tool_calls?: Array<{
        function?: {
          name?: string;
          arguments?: string;
        };
      }>;
    };
  }>;
  error?: {
    message?: string;
  };
}

export interface ToolResult {
  success: boolean;
  data: unknown;
  error?: string;
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

function schemaFromArgs(args: Record<string, unknown>): Record<string, unknown> {
  const properties: Record<string, unknown> = {};
  const required: string[] = [];

  for (const [key, value] of Object.entries(args)) {
    required.push(key);
    const valueType = Array.isArray(value) ? 'array' : typeof value;
    if (valueType === 'number') {
      properties[key] = { type: 'number' };
    } else if (valueType === 'boolean') {
      properties[key] = { type: 'boolean' };
    } else if (valueType === 'array') {
      properties[key] = { type: 'array', items: { type: 'string' } };
    } else if (valueType === 'object' && value !== null) {
      properties[key] = { type: 'object', additionalProperties: true };
    } else {
      properties[key] = { type: 'string' };
    }
  }

  return {
    type: 'object',
    properties,
    required,
    additionalProperties: true
  };
}

async function fetchWithTimeout(url: string, init: RequestInit, timeoutMs: number): Promise<Response> {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeoutMs);

  try {
    return await fetch(url, {
      ...init,
      signal: controller.signal
    });
  } finally {
    clearTimeout(timer);
  }
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

  async invokeTool(name: string, args: Record<string, unknown>): Promise<ToolResult> {
    const settings = await getSettings();
    const url = `http://${settings.buddyHost}:${settings.buddyPort}/v1/chat/completions`;

    const body = {
      model: settings.defaultModel,
      stream: false,
      messages: [
        {
          role: 'user',
          content: `Invoke tool \"${name}\" with these arguments: ${JSON.stringify(args)}`
        }
      ],
      tools: [
        {
          type: 'function',
          function: {
            name,
            description: `Execute Bonsai tool: ${name}`,
            parameters: schemaFromArgs(args)
          }
        }
      ],
      tool_choice: {
        type: 'function',
        function: { name }
      }
    };

    try {
      const response = await fetchWithTimeout(
        url,
        {
          method: 'POST',
          headers: await getAuthHeader(),
          body: JSON.stringify(body)
        },
        30_000
      );

      if (!response.ok) {
        const text = await response.text();
        return {
          success: false,
          data: null,
          error: `HTTP ${response.status}: ${text}`
        };
      }

      const payload = (await response.json()) as ToolCallResponse;
      if (payload.error?.message) {
        return {
          success: false,
          data: null,
          error: payload.error.message
        };
      }

      const firstChoice = payload.choices?.[0];
      const firstToolCall = firstChoice?.message?.tool_calls?.[0];
      const rawArgs = firstToolCall?.function?.arguments;
      if (!rawArgs) {
        return {
          success: false,
          data: payload,
          error: 'Malformed response: missing tool_calls[0].function.arguments'
        };
      }

      try {
        const parsed = JSON.parse(rawArgs) as unknown;
        return {
          success: true,
          data: parsed
        };
      } catch {
        return {
          success: true,
          data: rawArgs
        };
      }
    } catch (error) {
      if (error instanceof DOMException && error.name === 'AbortError') {
        return {
          success: false,
          data: null,
          error: 'Tool invocation timed out after 30 seconds'
        };
      }

      return {
        success: false,
        data: null,
        error: error instanceof Error ? error.message : 'Unknown tool invocation error'
      };
    }
  }
};
