<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';

  interface McpServer {
    id: string;
    name: string;
    command: string;
    args: string[];
    namespace: string;
    enabled: boolean;
    status?: 'connected' | 'disconnected' | 'error';
    statusMessage?: string;
  }

  let servers: McpServer[] = [];
  let showAdd = false;
  let connecting = false;
  let deleteConfirm: string | null = null;

  // Add-server form state
  let form = {
    name: '',
    command: 'npx',
    argsText: '',
    namespace: '',
  };

  async function load() {
    try {
      const raw = await invoke<McpServer[]>('list_mcp_servers');
      servers = raw.map(s => ({ ...s, status: 'disconnected' }));
    } catch (e) {
      console.error('list_mcp_servers:', e);
    }
  }

  async function save() {
    if (!form.name || !form.command || !form.namespace) return;
    const args = form.argsText.split('\n').map(s => s.trim()).filter(Boolean);
    const id = crypto.randomUUID();
    try {
      await invoke('upsert_mcp_server', {
        config: { id, name: form.name, command: form.command, args, namespace: form.namespace, enabled: true },
      });
      form = { name: '', command: 'npx', argsText: '', namespace: '' };
      showAdd = false;
      await load();
    } catch (e) {
      console.error('upsert_mcp_server:', e);
    }
  }

  async function toggleEnabled(server: McpServer) {
    try {
      await invoke('upsert_mcp_server', {
        config: { ...server, enabled: !server.enabled, args: server.args },
      });
      await load();
    } catch (e) {
      console.error('toggle:', e);
    }
  }

  async function connectAll() {
    connecting = true;
    try {
      const connected = await invoke<string[]>('reconnect_mcp_servers');
      servers = servers.map(s => ({
        ...s,
        status: connected.includes(s.name) ? 'connected' : (s.enabled ? 'error' : 'disconnected'),
      }));
    } catch (e) {
      console.error('reconnect_mcp_servers:', e);
    } finally {
      connecting = false;
    }
  }

  async function remove(id: string) {
    try {
      await invoke('delete_mcp_server', { id });
      deleteConfirm = null;
      await load();
    } catch (e) {
      console.error('delete_mcp_server:', e);
    }
  }

  onMount(load);
</script>

<div class="mcp-settings">
  <div class="section-header">
    <h3>MCP Servers</h3>
    <p class="hint">
      Connect any MCP-compatible server to give Bonsai access to its tools.
      Each server's tools appear in the assistant with the configured namespace prefix.
    </p>
  </div>

  {#if servers.length > 0}
    <ul class="server-list">
      {#each servers as server (server.id)}
        <li class="server-item">
          <div class="server-info">
            <span class="server-name">{server.name}</span>
            <span class="server-meta">{server.command} · ns: {server.namespace}</span>
          </div>
          <div class="server-controls">
            <span
              class="status-dot"
              class:connected={server.status === 'connected'}
              class:error={server.status === 'error'}
              title={server.status ?? 'disconnected'}
            ></span>
            <label class="toggle" title="Enable / disable">
              <input
                type="checkbox"
                checked={server.enabled}
                on:change={() => toggleEnabled(server)}
              />
              <span></span>
            </label>
            {#if deleteConfirm === server.id}
              <button class="btn danger-sm" on:click={() => remove(server.id)}>Confirm delete</button>
              <button class="btn ghost-sm" on:click={() => deleteConfirm = null}>Cancel</button>
            {:else}
              <button class="btn ghost-sm" on:click={() => deleteConfirm = server.id}>Delete</button>
            {/if}
          </div>
        </li>
      {/each}
    </ul>
  {:else}
    <p class="empty">No MCP servers configured.</p>
  {/if}

  <div class="actions-row">
    <button class="btn primary" on:click={() => showAdd = !showAdd}>
      {showAdd ? 'Cancel' : '+ Add server'}
    </button>
    <button class="btn secondary" on:click={connectAll} disabled={connecting}>
      {connecting ? 'Connecting…' : 'Connect all'}
    </button>
  </div>

  {#if showAdd}
    <div class="add-form">
      <label>
        Display name
        <input type="text" bind:value={form.name} placeholder="My Filesystem Server" />
      </label>
      <label>
        Command
        <input type="text" bind:value={form.command} placeholder="npx" />
      </label>
      <label>
        Arguments (one per line)
        <textarea
          bind:value={form.argsText}
          rows="3"
          placeholder="-y&#10;@modelcontextprotocol/server-filesystem&#10;/tmp"
        ></textarea>
      </label>
      <label>
        Namespace prefix
        <input type="text" bind:value={form.namespace} placeholder="fs" />
        <span class="field-hint">Tools will be named <code>{form.namespace || 'ns'}__tool_name</code></span>
      </label>
      <button class="btn primary" on:click={save} disabled={!form.name || !form.command || !form.namespace}>
        Save server
      </button>
    </div>
  {/if}
</div>

<style>
  .mcp-settings { display: flex; flex-direction: column; gap: 12px; }
  .section-header h3 { margin: 0 0 4px; font-size: 15px; }
  .hint { margin: 0; font-size: 12px; opacity: 0.65; }

  .server-list { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 6px; }
  .server-item {
    display: flex; align-items: center; justify-content: space-between;
    padding: 8px 10px; border-radius: 6px; background: var(--surface2, rgba(255,255,255,0.05));
  }
  .server-info { display: flex; flex-direction: column; gap: 2px; }
  .server-name { font-size: 13px; font-weight: 600; }
  .server-meta { font-size: 11px; opacity: 0.55; }
  .server-controls { display: flex; align-items: center; gap: 8px; }

  .status-dot {
    width: 8px; height: 8px; border-radius: 50%;
    background: var(--text-muted, #888); flex-shrink: 0;
  }
  .status-dot.connected { background: #4caf50; }
  .status-dot.error     { background: #f44336; }

  .toggle { display: flex; align-items: center; cursor: pointer; }
  .toggle input { display: none; }
  .toggle span {
    width: 30px; height: 16px; border-radius: 8px;
    background: var(--border, #555); position: relative; transition: background 0.2s;
  }
  .toggle input:checked + span { background: var(--accent, #6c63ff); }
  .toggle span::after {
    content: ''; position: absolute; top: 2px; left: 2px;
    width: 12px; height: 12px; border-radius: 50%; background: #fff; transition: transform 0.2s;
  }
  .toggle input:checked + span::after { transform: translateX(14px); }

  .empty { opacity: 0.5; font-size: 13px; margin: 4px 0; }
  .actions-row { display: flex; gap: 8px; }

  .add-form {
    display: flex; flex-direction: column; gap: 10px;
    padding: 12px; border-radius: 8px;
    background: var(--surface2, rgba(255,255,255,0.04));
    border: 1px solid var(--border, rgba(255,255,255,0.1));
  }
  .add-form label { display: flex; flex-direction: column; gap: 4px; font-size: 12px; opacity: 0.8; }
  .add-form input, .add-form textarea {
    font-size: 13px; padding: 6px 8px; border-radius: 5px;
    border: 1px solid var(--border, rgba(255,255,255,0.15));
    background: var(--surface, rgba(0,0,0,0.2)); color: inherit;
  }
  .add-form textarea { resize: vertical; font-family: monospace; }
  .field-hint { font-size: 11px; opacity: 0.55; }
  .field-hint code { background: rgba(255,255,255,0.08); padding: 1px 4px; border-radius: 3px; }

  .btn {
    padding: 6px 12px; border-radius: 5px; border: none; cursor: pointer; font-size: 12px;
    transition: opacity 0.15s;
  }
  .btn:disabled { opacity: 0.4; cursor: not-allowed; }
  .btn.primary   { background: var(--accent, #6c63ff); color: #fff; }
  .btn.secondary { background: var(--surface2, rgba(255,255,255,0.1)); color: inherit; }
  .btn.ghost-sm  { background: transparent; color: inherit; opacity: 0.6; padding: 4px 8px; }
  .btn.danger-sm { background: rgba(244,67,54,0.15); color: #f44336; padding: 4px 8px; }
</style>
