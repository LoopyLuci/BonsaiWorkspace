<script lang="ts">
  import { onMount } from 'svelte';
  import browser from '../lib/browser';
  import { BonsaiClient } from '../lib/bonsai-client';
  import type { ToolResult } from '../lib/bonsai-client';
  import type { AuditEntry } from '../lib/types';

  type SidebarTab = 'chat' | 'tools' | 'activity';
  type ToolFieldType = 'string' | 'number' | 'boolean' | 'json';

  interface ToolFieldDef {
    name: string;
    type: ToolFieldType;
    required?: boolean;
    description: string;
  }

  interface ToolDef {
    name: string;
    description: string;
    fields: ToolFieldDef[];
  }

  const tools: ToolDef[] = [
    { name: 'get_datetime', description: 'Return current local date and time.', fields: [] },
    { name: 'get_system_stats', description: 'Return live hardware and usage stats.', fields: [] },
    { name: 'get_weather', description: 'Fetch current weather for a location.', fields: [
      { name: 'location', type: 'string', required: true, description: 'City or location query' }
    ] },
    { name: 'fetch_url', description: 'Fetch and summarize web page content.', fields: [
      { name: 'url', type: 'string', required: true, description: 'Absolute HTTP(S) URL' }
    ] },
    { name: 'find_files', description: 'Find files in workspace by pattern.', fields: [
      { name: 'pattern', type: 'string', required: true, description: 'Glob or filename pattern' }
    ] },
    { name: 'read_file_assistant', description: 'Read a local file from allowed scope.', fields: [
      { name: 'path', type: 'string', required: true, description: 'Absolute file path' }
    ] },
    { name: 'write_file_assistant', description: 'Write to a local file (confirmation may be required).', fields: [
      { name: 'path', type: 'string', required: true, description: 'Absolute file path' },
      { name: 'content', type: 'string', required: true, description: 'Content to write' }
    ] },
    { name: 'search_knowledge', description: 'Search indexed workspace knowledge.', fields: [
      { name: 'query', type: 'string', required: true, description: 'Search query' }
    ] },
    { name: 'run_command', description: 'Execute a shell command (explicit confirmation required).', fields: [
      { name: 'command', type: 'string', required: true, description: 'Command string' }
    ] },
    { name: 'open_url', description: 'Open URL in browser.', fields: [
      { name: 'url', type: 'string', required: true, description: 'Absolute URL' }
    ] },
    { name: 'render_chart', description: 'Render chart from structured data.', fields: [
      { name: 'spec', type: 'json', required: true, description: 'Chart spec JSON object' }
    ] },
    { name: 'send_email', description: 'Send email via configured SMTP.', fields: [
      { name: 'to', type: 'string', required: true, description: 'Recipient email' },
      { name: 'subject', type: 'string', required: true, description: 'Email subject' },
      { name: 'body', type: 'string', required: true, description: 'Email body' }
    ] }
  ];

  let activeTab: SidebarTab = 'chat';
  let input = '';
  let streamText = '';
  let busy = false;
  let audit: AuditEntry[] = [];
  let streamId = '';
  let toolRunning: Record<string, boolean> = {};
  let toolArgs: Record<string, Record<string, string>> = {};
  let toolResults: Record<string, ToolResult> = {};
  let openResults: Record<string, boolean> = {};

  for (const tool of tools) {
    toolArgs[tool.name] = {};
    toolRunning[tool.name] = false;
    openResults[tool.name] = false;
    for (const field of tool.fields) {
      toolArgs[tool.name][field.name] = '';
    }
  }

  const runtimeListener = (msg: any) => {
    if (msg?.type !== 'EXTENSION_EVENT') return;
    const event = msg.event;
    if (event.type === 'CHAT_TOKEN' && event.streamId === streamId) {
      streamText += event.token;
    }
    if (event.type === 'CHAT_DONE' && event.streamId === streamId) {
      busy = false;
    }
    if (event.type === 'AUDIT_UPDATED') {
      void refreshAudit();
    }
  };

  async function refreshAudit() {
    const res = await browser.runtime.sendMessage({ type: 'GET_AUDIT_LOG' });
    if (res.ok) {
      audit = (res.data ?? []) as AuditEntry[];
    }
  }

  async function ask() {
    if (!input.trim()) return;
    busy = true;
    streamText = '';
    streamId = crypto.randomUUID();

    const res = await browser.runtime.sendMessage({
      type: 'CHAT_STREAM',
      streamId,
      messages: [{ role: 'user', content: input }]
    });

    if (!res.ok) {
      streamText = res.error;
      busy = false;
    }
  }

  async function clearAudit() {
    await browser.runtime.sendMessage({ type: 'CLEAR_AUDIT_LOG' });
    await refreshAudit();
  }

  function parseFieldValue(raw: string, type: ToolFieldType): unknown {
    if (type === 'number') {
      return Number(raw);
    }
    if (type === 'boolean') {
      return raw.toLowerCase() === 'true';
    }
    if (type === 'json') {
      return JSON.parse(raw);
    }
    return raw;
  }

  function collectToolArgs(tool: ToolDef): Record<string, unknown> {
    const args: Record<string, unknown> = {};
    for (const field of tool.fields) {
      const raw = (toolArgs[tool.name][field.name] ?? '').trim();
      if (!raw && !field.required) continue;
      if (!raw && field.required) {
        throw new Error(`Missing required field: ${field.name}`);
      }
      args[field.name] = parseFieldValue(raw, field.type);
    }
    return args;
  }

  async function runTool(tool: ToolDef) {
    toolRunning = { ...toolRunning, [tool.name]: true };
    openResults = { ...openResults, [tool.name]: true };

    try {
      const args = collectToolArgs(tool);
      const result = await BonsaiClient.invokeTool(tool.name, args);
      toolResults = {
        ...toolResults,
        [tool.name]: result
      };
    } catch (error) {
      toolResults = {
        ...toolResults,
        [tool.name]: {
          success: false,
          data: null,
          error: error instanceof Error ? error.message : 'Unknown tool error'
        }
      };
    } finally {
      toolRunning = { ...toolRunning, [tool.name]: false };
    }
  }

  function toggleResult(toolName: string) {
    openResults = {
      ...openResults,
      [toolName]: !openResults[toolName]
    };
  }

  onMount(() => {
    browser.runtime.onMessage.addListener(runtimeListener);
    void refreshAudit();

    return () => {
      browser.runtime.onMessage.removeListener(runtimeListener);
    };
  });
</script>

<main class="sidebar-root">
  <header class="tab-bar">
    <button class:active={activeTab === 'chat'} on:click={() => (activeTab = 'chat')}>Chat</button>
    <button class:active={activeTab === 'tools'} on:click={() => (activeTab = 'tools')}>Tools</button>
    <button class:active={activeTab === 'activity'} on:click={() => (activeTab = 'activity')}>Activity</button>
  </header>

  {#if activeTab === 'chat'}
    <section class="panel">
      <h2>Bonsai Buddy</h2>
      <textarea bind:value={input} rows="3" placeholder="Ask Buddy anything"></textarea>
      <button on:click={ask} disabled={busy}>Send</button>
      <div class="stream-box">
        {#if streamText}
          {streamText}
        {:else}
          Streaming response appears here...
        {/if}
      </div>
    </section>
  {/if}

  {#if activeTab === 'tools'}
    <section class="panel tool-grid">
      <h2>Bonsai Tools</h2>
      {#each tools as tool}
        <article class="tool-card">
          <div class="tool-header">
            <strong>{tool.name}</strong>
            <button on:click={() => runTool(tool)} disabled={toolRunning[tool.name]}>
              {toolRunning[tool.name] ? 'Running...' : 'Run'}
            </button>
          </div>
          <p>{tool.description}</p>

          {#if tool.fields.length > 0}
            <div class="tool-fields">
              {#each tool.fields as field}
                <label>
                  <span>{field.name} {field.required ? '*' : ''}</span>
                  {#if field.type === 'json'}
                    <textarea
                      rows="3"
                      bind:value={toolArgs[tool.name][field.name]}
                      placeholder={field.description}
                    ></textarea>
                  {:else if field.type === 'number'}
                    <input
                      type="number"
                      bind:value={toolArgs[tool.name][field.name]}
                      placeholder={field.description}
                    />
                  {:else}
                    <input
                      type="text"
                      bind:value={toolArgs[tool.name][field.name]}
                      placeholder={field.description}
                    />
                  {/if}
                </label>
              {/each}
            </div>
          {/if}

          {#if toolResults[tool.name]}
            <div class="result-wrap">
              <button class="toggle-result" on:click={() => toggleResult(tool.name)}>
                {openResults[tool.name] ? 'Hide result' : 'Show result'}
              </button>
              {#if openResults[tool.name]}
                <pre class="result-box">{JSON.stringify(toolResults[tool.name], null, 2)}</pre>
              {/if}
            </div>
          {/if}
        </article>
      {/each}
    </section>
  {/if}

  {#if activeTab === 'activity'}
    <section class="panel">
      <div class="activity-header">
        <h2>Action History</h2>
        <button class="secondary" on:click={clearAudit}>Clear</button>
      </div>
      <div class="activity-list">
        {#if audit.length === 0}
          <div>No actions logged yet.</div>
        {:else}
          {#each audit as item}
            <article class="activity-item">
              <div class="activity-title">{item.action} · {item.result}</div>
              <div class="activity-url">{item.url}</div>
              {#if item.message}
                <div>{item.message}</div>
              {/if}
            </article>
          {/each}
        {/if}
      </div>
    </section>
  {/if}
</main>

<style>
  .sidebar-root {
    min-height: 100vh;
    padding: 14px;
    display: grid;
    gap: 12px;
    background: radial-gradient(circle at top right, #1e293b 5%, #0f172a 58%);
    color: #f8fafc;
  }

  .tab-bar {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 8px;
  }

  .tab-bar button {
    background: #1f2937;
    color: #e5e7eb;
    border: 1px solid #374151;
  }

  .tab-bar button.active {
    background: #b45309;
    border-color: #f59e0b;
    color: #fff7ed;
  }

  .panel {
    border: 1px solid #334155;
    border-radius: 14px;
    padding: 12px;
    background: rgba(15, 23, 42, 0.8);
    display: grid;
    gap: 10px;
  }

  .panel h2 {
    margin: 0;
    color: #fbbf24;
  }

  .stream-box,
  .result-box {
    white-space: pre-wrap;
    background: #0b1220;
    border: 1px solid #374151;
    border-radius: 10px;
    padding: 10px;
    max-height: 260px;
    overflow: auto;
  }

  .tool-grid {
    align-content: start;
  }

  .tool-card {
    border: 1px solid #3f3f46;
    border-radius: 12px;
    padding: 10px;
    background: #111827;
    display: grid;
    gap: 8px;
  }

  .tool-card p {
    margin: 0;
    color: #cbd5e1;
    font-size: 13px;
  }

  .tool-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 10px;
  }

  .tool-fields {
    display: grid;
    gap: 8px;
  }

  .tool-fields label {
    display: grid;
    gap: 4px;
    font-size: 12px;
    color: #d1d5db;
  }

  .toggle-result {
    background: #374151;
    color: #f9fafb;
  }

  .activity-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .activity-header h2 {
    margin: 0;
  }

  .activity-list {
    max-height: 70vh;
    overflow: auto;
    display: grid;
    gap: 8px;
  }

  .activity-item {
    border: 1px solid #374151;
    border-radius: 10px;
    padding: 8px;
    background: #111827;
  }

  .activity-title {
    font-weight: 700;
    color: #fcd34d;
  }

  .activity-url {
    font-size: 12px;
    color: #94a3b8;
  }
</style>
