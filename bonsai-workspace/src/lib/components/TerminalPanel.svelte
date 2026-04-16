<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { Terminal } from 'xterm';
  import { FitAddon } from 'xterm-addon-fit';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import 'xterm/css/xterm.css';

  type TerminalEventPayload = string | { session_id?: string; text?: string };

  type LogLevel = 'debug' | 'info' | 'warn' | 'error';
  type LogCategory = 'tool' | 'swarm' | 'chat' | 'system' | 'terminal';

  type ActivityLogEntry = {
    id: string;
    ts: number;
    level: LogLevel;
    category: LogCategory;
    source: string;
    summary: string;
    details?: unknown;
  };

  type ActivityLogSettings = {
    autoScroll: boolean;
    dedupe: boolean;
    maxEntries: number;
    showDebug: boolean;
    showInfo: boolean;
    showWarn: boolean;
    showError: boolean;
    showTool: boolean;
    showSwarm: boolean;
    showChat: boolean;
    showSystem: boolean;
    showTerminal: boolean;
    compactRows: boolean;
  };

  type TerminalTab = {
    id: string;
    title: string;
    input: string;
    history: string[];
    historyIndex: number;
    buffer: string;
    ready: boolean;
    error: string;
  };

  const HISTORY_KEY = 'bonsai-terminal-history-v1';

  let container: HTMLDivElement;
  let activityScrollEl: HTMLDivElement;
  let term: Terminal;
  let fit: FitAddon;
  let unlistenPty: (() => void) | null = null;
  let unlistenToolTerminal: (() => void) | null = null;
  let unlistenToolUsed: (() => void) | null = null;
  let unlistenPermission: (() => void) | null = null;
  let unlistenSwarmError: (() => void) | null = null;
  let unlistenSwarmPlan: (() => void) | null = null;
  let unlistenSwarmComplete: (() => void) | null = null;
  let unlistenAgentConnect: (() => void) | null = null;
  let resizer: ReturnType<typeof setTimeout> | null = null;
  let showLogSettings = false;
  let logSearch = '';
  let activitySettings: ActivityLogSettings = {
    autoScroll: true,
    dedupe: true,
    maxEntries: 800,
    showDebug: true,
    showInfo: true,
    showWarn: true,
    showError: true,
    showTool: true,
    showSwarm: true,
    showChat: true,
    showSystem: true,
    showTerminal: true,
    compactRows: false,
  };
  let activityLog: ActivityLogEntry[] = [];

  let tabs: TerminalTab[] = [];
  let activeTabId = 'activity';
  let tabCounter = 1;
  const LOG_SETTINGS_KEY = 'bonsai-terminal-activity-settings-v1';
  const LOG_DATA_KEY = 'bonsai-terminal-activity-log-v1';

  function nextTabId(): string {
    tabCounter += 1;
    return `tab-${tabCounter}`;
  }

  function restoreHistoryMap(): Record<string, string[]> {
    try {
      const raw = localStorage.getItem(HISTORY_KEY);
      if (!raw) return {};
      const parsed = JSON.parse(raw) as Record<string, string[]>;
      return parsed && typeof parsed === 'object' ? parsed : {};
    } catch {
      return {};
    }
  }

  function persistHistoryMap() {
    const map: Record<string, string[]> = {};
    for (const tab of tabs) map[tab.id] = tab.history.slice(-150);
    localStorage.setItem(HISTORY_KEY, JSON.stringify(map));
  }

  function createTab(id: string, title: string): TerminalTab {
    const historyMap = restoreHistoryMap();
    const history = historyMap[id] ?? [];
    return {
      id,
      title,
      input: '',
      history,
      historyIndex: history.length,
      buffer: '',
      ready: false,
      error: '',
    };
  }

  function getActiveTab(): TerminalTab | undefined {
    return tabs.find((t) => t.id === activeTabId);
  }

  function updateTab(id: string, updater: (tab: TerminalTab) => TerminalTab) {
    tabs = tabs.map((tab) => (tab.id === id ? updater(tab) : tab));
  }

  function parseTerminalPayload(payload: TerminalEventPayload): { sessionId: string; text: string } {
    if (typeof payload === 'string') {
      return { sessionId: 'default', text: payload };
    }
    const sessionId = payload?.session_id && payload.session_id.length > 0
      ? payload.session_id
      : 'agent-tool';
    return { sessionId, text: payload?.text ?? '' };
  }

  function resolveWritableSession(sessionId: string): string {
    if (tabs.some((t) => t.id === sessionId)) return sessionId;
    if (tabs.some((t) => t.id === activeTabId)) return activeTabId;
    return tabs[0]?.id ?? 'default';
  }

  function renderActiveBuffer() {
    if (activeTabId === 'activity') return;
    const active = getActiveTab();
    if (!term || !active) return;
    term.reset();
    if (active.buffer) term.write(active.buffer);
  }

  function restoreActivitySettings() {
    try {
      const raw = localStorage.getItem(LOG_SETTINGS_KEY);
      if (!raw) return;
      const parsed = JSON.parse(raw) as Partial<ActivityLogSettings>;
      activitySettings = { ...activitySettings, ...parsed };
    } catch {
      // Ignore corrupt local storage.
    }
  }

  function persistActivitySettings() {
    localStorage.setItem(LOG_SETTINGS_KEY, JSON.stringify(activitySettings));
  }

  function restoreActivityLog() {
    try {
      const raw = localStorage.getItem(LOG_DATA_KEY);
      if (!raw) return;
      const parsed = JSON.parse(raw) as ActivityLogEntry[];
      if (Array.isArray(parsed)) {
        activityLog = parsed.slice(-activitySettings.maxEntries);
      }
    } catch {
      // Ignore malformed cache.
    }
  }

  function persistActivityLog() {
    localStorage.setItem(LOG_DATA_KEY, JSON.stringify(activityLog.slice(-activitySettings.maxEntries)));
  }

  function logSignature(e: ActivityLogEntry): string {
    const details = e.details ? JSON.stringify(e.details) : '';
    return `${e.level}|${e.category}|${e.source}|${e.summary}|${details}`;
  }

  function pushLog(level: LogLevel, category: LogCategory, source: string, summary: string, details?: unknown) {
    const entry: ActivityLogEntry = {
      id: `${Date.now()}-${Math.random().toString(16).slice(2, 8)}`,
      ts: Date.now(),
      level,
      category,
      source,
      summary,
      details,
    };

    if (activitySettings.dedupe && activityLog.length > 0) {
      const prev = activityLog[activityLog.length - 1];
      if (prev && logSignature(prev) === logSignature(entry)) {
        return;
      }
    }

    activityLog = [...activityLog, entry].slice(-activitySettings.maxEntries);
    persistActivityLog();

    if (activitySettings.autoScroll) {
      requestAnimationFrame(() => {
        activityScrollEl?.scrollTo({ top: activityScrollEl.scrollHeight });
      });
    }
  }

  function copyActivityLog() {
    const lines = filteredActivityLog.map((l) => `[${new Date(l.ts).toISOString()}] [${l.level.toUpperCase()}] [${l.category}] ${l.source}: ${l.summary}${l.details ? `\n${JSON.stringify(l.details, null, 2)}` : ''}`);
    const text = lines.join('\n\n');
    navigator.clipboard.writeText(text).then(() => {
      pushLog('info', 'system', 'activity-log', 'Copied activity log to clipboard');
    }).catch((e) => {
      pushLog('error', 'system', 'activity-log', 'Failed to copy activity log', String(e));
    });
  }

  function saveActivityLog() {
    const blob = new Blob([
      JSON.stringify({
        exported_at: new Date().toISOString(),
        settings: activitySettings,
        entries: filteredActivityLog,
      }, null, 2),
    ], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `bonsai-activity-log-${new Date().toISOString().replace(/[:.]/g, '-')}.json`;
    document.body.appendChild(a);
    a.click();
    a.remove();
    URL.revokeObjectURL(url);
    pushLog('info', 'system', 'activity-log', 'Saved activity log snapshot to file');
  }

  function clearActivityLog() {
    activityLog = [];
    persistActivityLog();
    pushLog('info', 'system', 'activity-log', 'Activity log cleared');
  }

  function updateActivitySetting<K extends keyof ActivityLogSettings>(key: K, value: ActivityLogSettings[K]) {
    activitySettings = { ...activitySettings, [key]: value };
    persistActivitySettings();
    if (key === 'maxEntries') {
      activityLog = activityLog.slice(-activitySettings.maxEntries);
      persistActivityLog();
    }
  }

  $: filteredActivityLog = activityLog.filter((entry) => {
    if (!activitySettings.showDebug && entry.level === 'debug') return false;
    if (!activitySettings.showInfo && entry.level === 'info') return false;
    if (!activitySettings.showWarn && entry.level === 'warn') return false;
    if (!activitySettings.showError && entry.level === 'error') return false;

    if (!activitySettings.showTool && entry.category === 'tool') return false;
    if (!activitySettings.showSwarm && entry.category === 'swarm') return false;
    if (!activitySettings.showChat && entry.category === 'chat') return false;
    if (!activitySettings.showSystem && entry.category === 'system') return false;
    if (!activitySettings.showTerminal && entry.category === 'terminal') return false;

    if (!logSearch.trim()) return true;
    const needle = logSearch.trim().toLowerCase();
    const blob = `${entry.source} ${entry.summary} ${JSON.stringify(entry.details ?? '').toLowerCase()}`.toLowerCase();
    return blob.includes(needle);
  });

  async function spawnTabSession(tabId: string) {
    try {
      await invoke('spawn_pty_terminal', { sessionId: tabId });
      updateTab(tabId, (tab) => ({ ...tab, ready: true, error: '' }));
      updateTab(tabId, (tab) => ({ ...tab, buffer: `${tab.buffer}\r\n\x1b[1;32m✓ ${tab.title} ready\x1b[0m\r\n` }));
      if (activeTabId === tabId) renderActiveBuffer();
    } catch (e) {
      updateTab(tabId, (tab) => ({ ...tab, error: String(e), ready: false }));
      updateTab(tabId, (tab) => ({ ...tab, buffer: `${tab.buffer}\r\n\x1b[1;31m✗ PTY error: ${e}\x1b[0m\r\n` }));
      if (activeTabId === tabId) renderActiveBuffer();
    }
  }

  async function addTab() {
    const tabId = nextTabId();
    const tab = createTab(tabId, `Shell ${tabs.length + 1}`);
    tabs = [...tabs, tab];
    activeTabId = tab.id;
    renderActiveBuffer();
    await spawnTabSession(tab.id);
  }

  async function closeTab(tabId: string) {
    if (tabs.length <= 1) return;
    tabs = tabs.filter((t) => t.id !== tabId);
    const historyMap = restoreHistoryMap();
    delete historyMap[tabId];
    localStorage.setItem(HISTORY_KEY, JSON.stringify(historyMap));

    try {
      await invoke('close_pty_session', { sessionId: tabId });
    } catch {
      // Ignore close failures for already-closed sessions.
    }

    if (activeTabId === tabId) {
      activeTabId = tabs[0]?.id ?? '';
      renderActiveBuffer();
    }
  }

  function activateTab(tabId: string) {
    activeTabId = tabId;
    renderActiveBuffer();
  }

  onMount(async () => {
    restoreActivitySettings();
    restoreActivityLog();

    const defaultTab = createTab('default', 'Shell 1');
    tabs = [defaultTab];
    activeTabId = 'activity';

    term = new Terminal({
      theme: {
        background: '#18181b',
        foreground: '#e4e4e7',
        cursor: '#60a5fa',
        selectionBackground: 'rgba(59,130,246,0.3)',
      },
      fontSize: 13,
      fontFamily: "'JetBrains Mono', 'Fira Code', Menlo, monospace",
      cursorBlink: true,
      scrollback: 5000,
      convertEol: true,
    });
    fit = new FitAddon();
    term.loadAddon(fit);
    term.open(container);
    fit.fit();
    renderActiveBuffer();

    unlistenPty = await listen<TerminalEventPayload>('pty-output', (e) => {
      const { sessionId, text } = parseTerminalPayload(e.payload);
      if (!tabs.some((t) => t.id === sessionId)) return;
      updateTab(sessionId, (tab) => ({ ...tab, buffer: `${tab.buffer}${text}` }));
      if (activeTabId === sessionId) term.write(text);
      if (text.toLowerCase().includes('error') || text.toLowerCase().includes('exception')) {
        pushLog('warn', 'terminal', 'pty-output', 'Terminal emitted error-like output', { sessionId, text: text.slice(0, 600) });
      }
    });

    unlistenToolTerminal = await listen<TerminalEventPayload>('terminal-output', (e) => {
      const { sessionId, text } = parseTerminalPayload(e.payload);
      const id = resolveWritableSession(sessionId);
      const merged = `\r\n${text}`;
      updateTab(id, (tab) => ({ ...tab, buffer: `${tab.buffer}${merged}` }));
      if (activeTabId === id) term.write(merged);
      pushLog('info', 'tool', 'terminal-output', 'Tool command output streamed', { sessionId, preview: text.slice(0, 500) });
    });

    unlistenToolUsed = await listen<{ tool: string; output: string }>('tool-used', (e) => {
      pushLog('info', 'tool', e.payload.tool, 'Tool executed', {
        tool: e.payload.tool,
        outputPreview: e.payload.output?.slice(0, 500),
      });
    });

    unlistenPermission = await listen<any>('permission-request', (e) => {
      pushLog('warn', 'tool', 'permission-request', `Approval needed for ${e.payload?.tool ?? 'unknown tool'}`, e.payload);
    });

    unlistenSwarmError = await listen<any>('swarm-error', (e) => {
      pushLog('error', 'swarm', 'swarm-error', 'Swarm worker error', e.payload);
    });

    unlistenSwarmPlan = await listen<any>('swarm-plan-ready', (e) => {
      pushLog('debug', 'swarm', 'swarm-plan-ready', 'Leader produced swarm plan', e.payload);
    });

    unlistenSwarmComplete = await listen<any>('swarm-complete', (e) => {
      pushLog('info', 'swarm', 'swarm-complete', 'Swarm run completed', e.payload);
    });

    unlistenAgentConnect = await listen<any>('agent-connect-event', (e) => {
      pushLog('debug', 'chat', 'agent-connect-event', e.payload?.summary ?? 'Agent event', e.payload);
    });

    window.addEventListener('error', onWindowError);
    window.addEventListener('unhandledrejection', onUnhandledRejection);

    await spawnTabSession('default');

    const ro = new ResizeObserver(() => {
      if (resizer) clearTimeout(resizer);
      resizer = setTimeout(() => {
        fit.fit();
        const active = getActiveTab();
        if (active?.ready) {
          invoke('resize_pty_session', {
            sessionId: active.id,
            rows: term.rows,
            cols: term.cols,
          }).catch(() => {
            invoke('resize_pty', { rows: term.rows, cols: term.cols }).catch(() => {});
          });
        }
      }, 100);
    });
    ro.observe(container);
  });

  onDestroy(() => {
    unlistenPty?.();
    unlistenToolTerminal?.();
    unlistenToolUsed?.();
    unlistenPermission?.();
    unlistenSwarmError?.();
    unlistenSwarmPlan?.();
    unlistenSwarmComplete?.();
    unlistenAgentConnect?.();
    window.removeEventListener('error', onWindowError);
    window.removeEventListener('unhandledrejection', onUnhandledRejection);
    for (const tab of tabs) {
      if (tab.id !== 'default') {
        invoke('close_pty_session', { sessionId: tab.id }).catch(() => {});
      }
    }
    term?.dispose();
    if (resizer) clearTimeout(resizer);
  });

  function onWindowError(event: ErrorEvent) {
    pushLog('error', 'system', 'window.error', event.message || 'Unhandled window error', {
      filename: event.filename,
      lineno: event.lineno,
      colno: event.colno,
    });
  }

  function onUnhandledRejection(event: PromiseRejectionEvent) {
    pushLog('error', 'system', 'unhandledrejection', 'Unhandled Promise rejection', {
      reason: String(event.reason),
    });
  }

  async function sendCommand() {
    if (activeTabId === 'activity') return;
    const active = getActiveTab();
    if (!active) return;

    const cmd = active.input.trim();
    if (!cmd) return;

    let nextHistory = active.history;
    if (nextHistory[nextHistory.length - 1] !== cmd) {
      nextHistory = [...nextHistory, cmd].slice(-150);
    }

    updateTab(active.id, (tab) => ({
      ...tab,
      input: '',
      history: nextHistory,
      historyIndex: nextHistory.length,
    }));
    persistHistoryMap();

    try {
      await invoke('send_to_pty_session', {
        sessionId: active.id,
        input: cmd,
      });
    } catch {
      await invoke('send_to_pty', { input: cmd });
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (activeTabId === 'activity') return;
    const active = getActiveTab();
    if (!active) return;

    if (e.key === 'Enter') {
      e.preventDefault();
      sendCommand();
      return;
    }

    if (e.key === 'ArrowUp') {
      e.preventDefault();
      const nextIndex = Math.max(0, active.historyIndex - 1);
      const recalled = active.history[nextIndex] ?? active.input;
      updateTab(active.id, (tab) => ({ ...tab, input: recalled, historyIndex: nextIndex }));
      return;
    }

    if (e.key === 'ArrowDown') {
      e.preventDefault();
      const maxIndex = active.history.length;
      const nextIndex = Math.min(maxIndex, active.historyIndex + 1);
      const recalled = nextIndex === maxIndex ? '' : (active.history[nextIndex] ?? '');
      updateTab(active.id, (tab) => ({ ...tab, input: recalled, historyIndex: nextIndex }));
    }
  }

  function handleInput(e: Event) {
    if (activeTabId === 'activity') return;
    const active = getActiveTab();
    if (!active) return;
    const value = (e.target as HTMLInputElement).value;
    updateTab(active.id, (tab) => ({ ...tab, input: value }));
  }

  function formatLogTime(ts: number): string {
    return new Date(ts).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' });
  }

  function checked(e: Event): boolean {
    return (e.currentTarget as HTMLInputElement).checked;
  }

  function value(e: Event): string {
    return (e.currentTarget as HTMLInputElement).value;
  }
</script>

<div class="terminal-panel">
  <div class="term-header">
    <span class="term-title">Terminal</span>
    <div class="tabs">
      <button
        class="tab-chip {activeTabId === 'activity' ? 'active' : ''}"
        on:click={() => activateTab('activity')}
        type="button"
      >
        Activity Log
        <span class="tab-state {filteredActivityLog.some((x) => x.level === 'error') ? 'tab-state-error' : ''}">
          {filteredActivityLog.length}
        </span>
      </button>
      {#each tabs as tab}
        <div class="tab-chip {tab.id === activeTabId ? 'active' : ''}">
          <button class="tab-open" on:click={() => activateTab(tab.id)} type="button">{tab.title}</button>
          {#if tab.error}
            <span class="tab-state tab-state-error">!</span>
          {:else if tab.ready}
            <span class="tab-state">●</span>
          {:else}
            <span class="tab-state tab-state-wait">○</span>
          {/if}
          {#if tabs.length > 1}
            <button class="tab-close" type="button" on:click|stopPropagation={() => closeTab(tab.id)}>x</button>
          {/if}
        </div>
      {/each}
      <button class="tab-add" type="button" on:click={addTab}>+</button>
    </div>
  </div>

  {#if activeTabId === 'activity'}
    <div class="activity-toolbar">
      <input
        class="activity-search"
        placeholder="Search activity..."
        bind:value={logSearch}
        aria-label="Search activity log"
      />
      <button class="tool-btn" on:click={() => (showLogSettings = !showLogSettings)}>{showLogSettings ? 'Hide Settings' : 'Settings'}</button>
      <button class="tool-btn" on:click={copyActivityLog}>Copy</button>
      <button class="tool-btn" on:click={saveActivityLog}>Save</button>
      <button class="tool-btn danger" on:click={clearActivityLog}>Clear</button>
    </div>

    {#if showLogSettings}
      <div class="activity-settings">
        <label><input type="checkbox" checked={activitySettings.autoScroll} on:change={(e) => updateActivitySetting('autoScroll', checked(e))} /> Auto-scroll</label>
        <label><input type="checkbox" checked={activitySettings.dedupe} on:change={(e) => updateActivitySetting('dedupe', checked(e))} /> De-duplicate adjacent entries</label>
        <label><input type="checkbox" checked={activitySettings.compactRows} on:change={(e) => updateActivitySetting('compactRows', checked(e))} /> Compact rows</label>
        <label>
          Max entries
          <input type="number" min="100" max="5000" step="50" value={activitySettings.maxEntries} on:change={(e) => updateActivitySetting('maxEntries', Math.min(5000, Math.max(100, Number(value(e)) || activitySettings.maxEntries)))} />
        </label>
        <label><input type="checkbox" checked={activitySettings.showDebug} on:change={(e) => updateActivitySetting('showDebug', checked(e))} /> Debug</label>
        <label><input type="checkbox" checked={activitySettings.showInfo} on:change={(e) => updateActivitySetting('showInfo', checked(e))} /> Info</label>
        <label><input type="checkbox" checked={activitySettings.showWarn} on:change={(e) => updateActivitySetting('showWarn', checked(e))} /> Warn</label>
        <label><input type="checkbox" checked={activitySettings.showError} on:change={(e) => updateActivitySetting('showError', checked(e))} /> Error</label>
        <label><input type="checkbox" checked={activitySettings.showTool} on:change={(e) => updateActivitySetting('showTool', checked(e))} /> Tool</label>
        <label><input type="checkbox" checked={activitySettings.showSwarm} on:change={(e) => updateActivitySetting('showSwarm', checked(e))} /> Swarm</label>
        <label><input type="checkbox" checked={activitySettings.showChat} on:change={(e) => updateActivitySetting('showChat', checked(e))} /> Chat</label>
        <label><input type="checkbox" checked={activitySettings.showSystem} on:change={(e) => updateActivitySetting('showSystem', checked(e))} /> System</label>
        <label><input type="checkbox" checked={activitySettings.showTerminal} on:change={(e) => updateActivitySetting('showTerminal', checked(e))} /> Terminal</label>
      </div>
    {/if}

    <div class="activity-log" bind:this={activityScrollEl}>
      {#if filteredActivityLog.length === 0}
        <div class="activity-empty">No activity matches current filters.</div>
      {:else}
        {#each filteredActivityLog as item (item.id)}
          <div class="activity-row {item.level} {activitySettings.compactRows ? 'compact' : ''}">
            <div class="activity-head">
              <span class="activity-time">{formatLogTime(item.ts)}</span>
              <span class="activity-level">{item.level.toUpperCase()}</span>
              <span class="activity-cat">{item.category}</span>
              <span class="activity-src">{item.source}</span>
            </div>
            <div class="activity-summary">{item.summary}</div>
            {#if item.details && !activitySettings.compactRows}
              <pre class="activity-details">{JSON.stringify(item.details, null, 2)}</pre>
            {/if}
          </div>
        {/each}
      {/if}
    </div>
  {:else}
    <div bind:this={container} class="xterm-host"></div>
  {/if}

  <div class="term-input-row">
    <span class="prompt">$</span>
    <input
      value={activeTabId === 'activity' ? '' : (getActiveTab()?.input ?? '')}
      on:input={handleInput}
      on:keydown={handleKeydown}
      class="term-input"
      placeholder={activeTabId === 'activity' ? 'Activity tab is read-only' : 'Enter command...'}
      autocomplete="off"
      spellcheck="false"
      aria-label="Terminal input"
      disabled={activeTabId === 'activity' || !getActiveTab()}
    />
    <button class="term-send" on:click={sendCommand} disabled={activeTabId === 'activity'}>Send</button>
  </div>
</div>

<style>
  .terminal-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: #18181b;
  }

  .term-header {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 4px 12px;
    background: #0f0f12;
    border-bottom: 1px solid #3f3f46;
    font-size: 11px;
    flex-shrink: 0;
  }

  .term-title {
    color: #e4e4e7;
    font-weight: 600;
  }

  .tabs {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    overflow-x: auto;
    padding-bottom: 2px;
  }

  .tab-chip {
    display: flex;
    align-items: center;
    gap: 6px;
    border: 1px solid #3f3f46;
    background: #1f1f24;
    color: #cbd5e1;
    border-radius: 999px;
    padding: 3px 10px;
    cursor: pointer;
    font-size: 11px;
    white-space: nowrap;
  }

  .tab-open {
    background: transparent;
    border: none;
    color: inherit;
    cursor: pointer;
    font-size: 11px;
    padding: 0;
  }

  .tab-chip.active {
    background: #111827;
    border-color: #60a5fa;
    color: #f8fafc;
  }

  .tab-state {
    color: #22c55e;
    font-size: 10px;
  }

  .tab-state-wait {
    color: #f59e0b;
  }

  .tab-state-error {
    color: #ef4444;
  }

  .tab-close {
    background: transparent;
    border: none;
    color: #94a3b8;
    font-weight: 700;
    margin-left: 2px;
    cursor: pointer;
  }

  .activity-toolbar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 10px;
    border-bottom: 1px solid #2f2f36;
    background: #111318;
  }

  .activity-search {
    flex: 1;
    min-width: 200px;
    border: 1px solid #3f3f46;
    background: #0b0f13;
    color: #e4e4e7;
    border-radius: 8px;
    padding: 6px 9px;
    font-size: 12px;
  }

  .tool-btn {
    border: 1px solid #3f3f46;
    background: #1f1f24;
    color: #dbe4f0;
    border-radius: 8px;
    padding: 5px 9px;
    font-size: 11px;
    cursor: pointer;
  }

  .tool-btn.danger {
    border-color: #7f1d1d;
    color: #fca5a5;
  }

  .activity-settings {
    display: flex;
    flex-wrap: wrap;
    gap: 10px 14px;
    padding: 8px 10px;
    border-bottom: 1px solid #2f2f36;
    background: #0f1217;
    font-size: 11px;
    color: #cbd5e1;
  }

  .activity-settings label {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .activity-settings input[type="number"] {
    width: 88px;
    border: 1px solid #3f3f46;
    background: #0b0f13;
    color: #e4e4e7;
    border-radius: 6px;
    padding: 3px 6px;
    font-size: 11px;
  }

  .activity-log {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 8px 10px;
    background: #0d1116;
  }

  .activity-empty {
    color: #94a3b8;
    font-size: 12px;
    padding: 14px;
    text-align: center;
  }

  .activity-row {
    border: 1px solid #2f3641;
    border-left-width: 3px;
    border-radius: 8px;
    padding: 7px 9px;
    margin-bottom: 7px;
    background: #10151c;
  }

  .activity-row.debug { border-left-color: #64748b; }
  .activity-row.info { border-left-color: #22c55e; }
  .activity-row.warn { border-left-color: #f59e0b; }
  .activity-row.error { border-left-color: #ef4444; }

  .activity-row.compact {
    padding: 5px 8px;
  }

  .activity-head {
    display: flex;
    align-items: center;
    gap: 7px;
    font-size: 10px;
    color: #94a3b8;
    margin-bottom: 3px;
  }

  .activity-level {
    font-weight: 700;
    letter-spacing: 0.02em;
  }

  .activity-summary {
    color: #e2e8f0;
    font-size: 12px;
    line-height: 1.35;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .activity-details {
    margin: 6px 0 0;
    padding: 6px;
    border-radius: 6px;
    border: 1px solid #2f3641;
    background: #0a0f14;
    color: #94a3b8;
    font-size: 11px;
    white-space: pre-wrap;
    max-height: 140px;
    overflow: auto;
  }

  .tab-close:hover {
    color: #f8fafc;
  }

  .tab-add {
    border: 1px solid #3f3f46;
    background: #1f1f24;
    color: #e2e8f0;
    border-radius: 999px;
    width: 24px;
    height: 24px;
    line-height: 1;
    cursor: pointer;
  }

  .xterm-host {
    flex: 1;
    min-height: 0;
    padding: 4px;
    overflow: hidden;
  }

  .xterm-host :global(.xterm) {
    height: 100%;
  }

  .term-input-row {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 10px;
    border-top: 1px solid #3f3f46;
    background: #0f0f12;
    flex-shrink: 0;
  }

  .prompt {
    color: #22c55e;
    font-family: monospace;
    font-size: 13px;
  }

  .term-input {
    flex: 1;
    background: transparent;
    border: none;
    outline: none;
    font-family: 'JetBrains Mono', 'Fira Code', Menlo, monospace;
    font-size: 13px;
    color: #e4e4e7;
    caret-color: #60a5fa;
  }

  .term-send {
    background: #3b82f6;
    color: #fff;
    border: none;
    border-radius: 5px;
    padding: 3px 10px;
    font-size: 12px;
    cursor: pointer;
  }

  .term-send:hover {
    opacity: 0.85;
  }

  .term-send:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
</style>
