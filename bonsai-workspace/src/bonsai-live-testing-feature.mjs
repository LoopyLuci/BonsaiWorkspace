#!/usr/bin/env node

import { spawn } from 'node:child_process';
import fs from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const SRC_DIR = __dirname;
const WORKSPACE_ROOT = path.resolve(SRC_DIR, '..', '..');

const UI_BASE = process.env.BONSAI_UI_BASE || 'http://localhost:1420';
const STEP_TIMEOUT_MS = Number(process.env.BONSAI_LIVE_STEP_TIMEOUT_MS || 45000);
const SCENARIO_TIMEOUT_MS = Number(process.env.BONSAI_LIVE_SCENARIO_TIMEOUT_MS || 90000);
const LIVE_MODE = process.env.BONSAI_LIVE_MODE === '0' ? false : true;
const HEADLESS = process.env.BONSAI_LIVE_HEADLESS === '1' ? true : !LIVE_MODE;
const SLOW_MO_MS = Number(process.env.BONSAI_LIVE_SLOW_MO_MS || (LIVE_MODE ? 80 : 0));
const KEEP_OPEN_MS = Number(process.env.BONSAI_UI_KEEP_OPEN_MS || (LIVE_MODE ? 8000 : 0));
const SCENARIO_PAUSE_MS = Number(process.env.BONSAI_LIVE_SCENARIO_PAUSE_MS || (LIVE_MODE ? 1500 : 0));
const npmCmd = process.platform === 'win32' ? 'npm.cmd' : 'npm';
const LIVE_TEST_DIR_REL = 'tool_test/live-testing';
const LIVE_TEST_HELLO_REL = `${LIVE_TEST_DIR_REL}/hello.txt`;
const LIVE_TEST_DENIED_REL = `${LIVE_TEST_DIR_REL}/denied.txt`;

const suiteResults = [];

function log(msg) {
  process.stdout.write(`${msg}\n`);
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function pass(name, detail = '') {
  suiteResults.push({ name, ok: true, detail });
  log(`PASS ${name}${detail ? ` - ${detail}` : ''}`);
}

function fail(name, detail = '') {
  suiteResults.push({ name, ok: false, detail });
  log(`FAIL ${name}${detail ? ` - ${detail}` : ''}`);
}

async function cleanupArtifacts() {
  const liveTestDir = path.join(WORKSPACE_ROOT, 'tool_test', 'live-testing');
  await fs.rm(liveTestDir, { recursive: true, force: true });
}

async function waitForUrl(url, timeoutMs = 45000) {
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    try {
      const resp = await fetch(url, { method: 'GET' });
      if (resp.ok) return true;
    } catch {
      // retry until timeout
    }
    await sleep(500);
  }
  return false;
}

function startDevServer() {
  return spawn(`${npmCmd} run dev`, {
    cwd: SRC_DIR,
    stdio: 'inherit',
    shell: true,
    env: { ...process.env },
  });
}

async function waitForBodyText(page, text, timeoutMs = STEP_TIMEOUT_MS) {
  await page.waitForFunction(
    (needle) => document.body.innerText.includes(needle),
    text,
    { timeout: timeoutMs },
  );
}

async function waitForAnyBodyText(page, values, timeoutMs = STEP_TIMEOUT_MS) {
  await page.waitForFunction(
    (needles) => needles.some((needle) => document.body.innerText.includes(needle)),
    values,
    { timeout: timeoutMs },
  );
}

async function waitForAnyBodyTextSafe(page, values, timeoutMs = STEP_TIMEOUT_MS) {
  try {
    await waitForAnyBodyText(page, values, timeoutMs);
    return true;
  } catch {
    return false;
  }
}

async function waitForFileContent(filePath, expected, timeoutMs = STEP_TIMEOUT_MS) {
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    try {
      const content = await fs.readFile(filePath, 'utf8');
      if (content === expected) return true;
    } catch {
      // keep polling until timeout
    }
    await sleep(150);
  }
  return false;
}

function buildMockInitScript() {
  return (config = {}) => {
    const LIVE_TEST_DIR_REL_LOCAL = 'tool_test/live-testing';
    const LIVE_TEST_HELLO_REL_LOCAL = `${LIVE_TEST_DIR_REL_LOCAL}/hello.txt`;
    const LIVE_TEST_DENIED_REL_LOCAL = `${LIVE_TEST_DIR_REL_LOCAL}/denied.txt`;

    const state = {
      apiHost: '127.0.0.1',
      apiPort: 11369,
      wsClientCount: 1,
      pairToken: 'PAIR-TEST-1234',
      localIp: '127.0.0.1',
      remoteActive: false,
      remoteSessionId: '',
      remoteFrame: btoa('mock-frame-data'),
      submitChatCalls: 0,
      chatQueue: [],
      agentSessions: [],
      agentTimeline: {},
      nextAgentSession: 1,
      eventsByName: new Map(),
      callbacks: new Map(),
      callbackId: 1,
      eventId: 1,
      tokenDelayMs: Number(config.tokenDelayMs || 0),
      liveMode: Boolean(config.liveMode),
    };

    function browserSleep(ms) {
      return new Promise((resolve) => setTimeout(resolve, ms));
    }

    function sanitizePath(p) {
      return String(p || '').replace(/\\/g, '/');
    }

    function emitTauriEvent(eventName, payload) {
      const entries = state.eventsByName.get(eventName) || [];
      for (const entry of entries) {
        const rec = state.callbacks.get(entry.handlerId);
        if (!rec) continue;
        try {
          rec.cb({ event: eventName, id: entry.id, payload });
          if (rec.once) state.callbacks.delete(entry.handlerId);
        } catch {
          // ignore callback failures in mock harness
        }
      }
    }

    async function emitStreamText(text) {
      const chunks = String(text).match(/.{1,8}/g) || [String(text)];
      for (const chunk of chunks) {
        emitTauriEvent('token-stream', chunk);
        if (state.tokenDelayMs > 0) await browserSleep(state.tokenDelayMs);
      }
      emitTauriEvent('token-speed', state.tokenDelayMs > 0 ? Math.max(8, Math.round(1000 / state.tokenDelayMs) * 8) : 70);
    }

    function makeToolApproval(action, description, rationale) {
      return {
        type: 'tool_approval',
        tool: action.tool,
        args: action.args,
        description,
        rationale,
        action,
        raw_response: `<tool_call>${JSON.stringify(action)}</tool_call>`,
        ctx_snapshot: [],
        paths_affected: action.args?.path ? [sanitizePath(action.args.path)] : [],
      };
    }

    function listMockFiles() {
      const root = String(window.__bonsaiWorkspacePath || 'z:/Projects/BonsaiWorkspace');
      return [
        { path: `${root}/Runner-Streaming_System.md`, rel: 'Runner-Streaming_System.md', name: 'Runner-Streaming_System.md', is_dir: false },
        { path: `${root}/bonsai-workspace`, rel: 'bonsai-workspace', name: 'bonsai-workspace', is_dir: true },
        { path: `${root}/bonsai-workspace/src`, rel: 'bonsai-workspace/src', name: 'src', is_dir: true },
        { path: `${root}/bonsai-workspace/src/App.svelte`, rel: 'bonsai-workspace/src/App.svelte', name: 'App.svelte', is_dir: false },
        { path: `${root}/tool_test`, rel: 'tool_test', name: 'tool_test', is_dir: true },
      ];
    }

    function ensureAgentSession(session) {
      if (!state.agentTimeline[session.id]) state.agentTimeline[session.id] = [];
    }

    function pushAgentEvent(sessionId, event_type, summary) {
      const list = state.agentTimeline[sessionId] || [];
      const seq = list.length + 1;
      const ev = {
        seq,
        session_id: sessionId,
        event_type,
        summary,
        details: {},
        ts_ms: Date.now(),
      };
      list.push(ev);
      state.agentTimeline[sessionId] = list;
      emitTauriEvent('agent-connect-event', ev);
    }

    async function runToolAction(action) {
      if (typeof window.__liveToolExec !== 'function') {
        return { message: 'tool executor unavailable' };
      }
      return await window.__liveToolExec(action);
    }

    async function handleSubmitChat(args) {
      state.submitChatCalls += 1;
      const history = Array.isArray(args?.messages) ? args.messages : [];
      const lastUser = [...history].reverse().find((m) => m?.role === 'user' && typeof m?.content === 'string')?.content || '';

      if (/list all files|list files|current directory/i.test(lastUser)) {
        const out = await runToolAction({ tool: 'list_all_files', args: { path: window.__bonsaiWorkspacePath } });
        const content = `Used list_all_files. Found ${out?.count ?? 0} entries.`;
        await emitStreamText(content);
        return {
          content,
          action_handled: false,
          tools_used: ['list_all_files'],
          stats: {
            prompt_tokens: 20,
            completion_tokens: 18,
            tokens_per_second: 30,
            time_to_first_token_ms: 20,
            total_time_ms: 300,
          },
        };
      }

      if (/create folder\s+tool_test|create\s+hello\.txt|containing\s+hello/i.test(lastUser)) {
        const queue = [
          makeToolApproval(
            { tool: 'create_dir', args: { path: `${window.__bonsaiWorkspacePath}/${LIVE_TEST_DIR_REL_LOCAL}` } },
            'Create folder tool_test/live-testing',
            'Need a writable folder for hello.txt.',
          ),
          makeToolApproval(
            { tool: 'write_file', args: { path: `${window.__bonsaiWorkspacePath}/${LIVE_TEST_HELLO_REL_LOCAL}`, content: 'Hello\n' } },
            'Write hello.txt',
            'Need to write requested file content.',
          ),
        ];
        state.chatQueue = queue;
        emitTauriEvent('permission-request', queue[0]);
        await emitStreamText('I need approval for two tool actions to complete this request.');
        return {
          content: '',
          action_handled: true,
          tools_used: [],
          stats: {
            prompt_tokens: 20,
            completion_tokens: 12,
            tokens_per_second: 24,
            time_to_first_token_ms: 25,
            total_time_ms: 250,
          },
        };
      }

      if (/deny/i.test(lastUser) && /write/i.test(lastUser)) {
        const card = makeToolApproval(
          { tool: 'write_file', args: { path: `${window.__bonsaiWorkspacePath}/${LIVE_TEST_DENIED_REL_LOCAL}`, content: 'Denied path\n' } },
          'Write denied.txt',
          'Testing deny flow for HITL.',
        );
        state.chatQueue = [card];
        emitTauriEvent('permission-request', card);
        await emitStreamText('Approval needed before writing denied.txt.');
        return {
          content: '',
          action_handled: true,
          tools_used: [],
          stats: {
            prompt_tokens: 16,
            completion_tokens: 9,
            tokens_per_second: 20,
            time_to_first_token_ms: 20,
            total_time_ms: 220,
          },
        };
      }

      const fallback = 'Live testing mock received your message and is ready for the next step.';
      await emitStreamText(fallback);
      return {
        content: fallback,
        action_handled: false,
        tools_used: [],
        stats: {
          prompt_tokens: 10,
          completion_tokens: 10,
          tokens_per_second: 20,
          time_to_first_token_ms: 15,
          total_time_ms: 140,
        },
      };
    }

    async function handleResumeToolCall(args) {
      const approved = Boolean(args?.approved);
      const queued = state.chatQueue[0] || args?.action;
      if (!queued) {
        return {
          content: 'No pending tool action.',
          action_handled: false,
          tools_used: [],
          stats: {
            prompt_tokens: 4,
            completion_tokens: 5,
            tokens_per_second: 20,
            time_to_first_token_ms: 8,
            total_time_ms: 30,
          },
        };
      }

      if (!approved) {
        state.chatQueue = [];
        const denied = 'Denied. I did not run the requested tool action.';
        await emitStreamText(denied);
        return {
          content: denied,
          action_handled: false,
          tools_used: [],
          stats: {
            prompt_tokens: 8,
            completion_tokens: 10,
            tokens_per_second: 24,
            time_to_first_token_ms: 10,
            total_time_ms: 60,
          },
        };
      }

      const current = state.chatQueue.shift() || queued;
      const out = await runToolAction(current.action || current);

      if (state.chatQueue.length > 0) {
        const next = state.chatQueue[0];
        emitTauriEvent('permission-request', next);
        const partial = `Approved ${current.tool}. Waiting on next approval.`;
        await emitStreamText(partial);
        return {
          content: partial,
          action_handled: true,
          tools_used: [current.tool],
          stats: {
            prompt_tokens: 8,
            completion_tokens: 12,
            tokens_per_second: 24,
            time_to_first_token_ms: 10,
            total_time_ms: 100,
          },
        };
      }

      const done = `Done. Created ${LIVE_TEST_HELLO_REL_LOCAL} with Hello. (${out?.message || 'ok'})`;
      await emitStreamText(done);
      return {
        content: done,
        action_handled: false,
        tools_used: [current.tool],
        stats: {
          prompt_tokens: 8,
          completion_tokens: 12,
          tokens_per_second: 24,
          time_to_first_token_ms: 10,
          total_time_ms: 100,
        },
      };
    }

    window.__TAURI_EVENT_PLUGIN_INTERNALS__ = {
      unregisterListener(eventName, id) {
        const arr = state.eventsByName.get(eventName) || [];
        state.eventsByName.set(eventName, arr.filter((x) => x.id !== id));
      },
    };

    window.__TAURI_INTERNALS__ = {
      metadata: { currentWindow: { label: 'main' } },
      transformCallback(cb, once = false) {
        const id = state.callbackId++;
        state.callbacks.set(id, { cb, once });
        return id;
      },
      unregisterCallback(id) {
        state.callbacks.delete(id);
      },
      convertFileSrc(filePath) {
        return filePath;
      },
      async invoke(cmd, args = {}) {
        if (cmd === 'plugin:event|listen') {
          const id = state.eventId++;
          const name = args.event;
          const arr = state.eventsByName.get(name) || [];
          arr.push({ id, handlerId: args.handler });
          state.eventsByName.set(name, arr);
          return id;
        }

        if (cmd === 'plugin:event|unlisten') {
          const arr = state.eventsByName.get(args.event) || [];
          state.eventsByName.set(args.event, arr.filter((x) => x.id !== args.eventId));
          return null;
        }

        if (cmd === 'plugin:barcode-scanner|scan') {
          return { content: 'bonsai://connect?ip=127.0.0.1&port=11369&token=PAIR-TEST-1234' };
        }

        if (cmd === 'get_current_session_state') return { current_session_id: null, current_session_title: null };
        if (cmd === 'set_current_session_state') return null;
        if (cmd === 'save_chat_session') return { id: 'live-session-1' };
        if (cmd === 'load_chat_session') return { id: 'live-session-1', title: 'Live Test', messages: [] };
        if (cmd === 'list_chat_sessions') return [];

        if (cmd === 'open_workspace') return window.__bonsaiWorkspacePath;
        if (cmd === 'get_git_branch') return 'main';
        if (cmd === 'list_project_files') return listMockFiles();

        if (cmd === 'list_models_registry') {
          return [
            {
              id: 'bonsai-1',
              name: 'Bonsai 1 Mock',
              path: 'mock.gguf',
              architecture: 'llama',
              parameter_count: 1700000000,
              context_length: 8192,
              quant: 'Q4_K_M',
              ram_required_mb: 1600,
              ram_label: '1.6 GB',
              valid: true,
            },
          ];
        }

        if (cmd === 'get_orchestrator_status') {
          return {
            slots: [{ index: 0, port: 15000, state: { state: 'ready', model_id: 'bonsai-1' }, requests: 2, idle_secs: 3 }],
            queue_depth: 0,
            total_ram_mb: 16384,
            free_ram_mb: 8192,
          };
        }

        if (cmd === 'load_model') return null;
        if (cmd === 'switch_model') return 'Switched model.';

        if (cmd === 'get_hardware_info') {
          return {
            ram_total_gb: 16,
            ram_available_gb: 8,
            cpu_count: 8,
            backend: 'mock',
            gpu_names: ['Mock GPU'],
          };
        }

        if (cmd === 'get_api_port') return state.apiPort;
        if (cmd === 'get_api_config') return { api_host: state.apiHost, api_port: state.apiPort };
        if (cmd === 'set_api_config') {
          state.apiHost = String(args.api_host || state.apiHost);
          state.apiPort = Number(args.api_port || state.apiPort);
          return { api_host: state.apiHost, api_port: state.apiPort };
        }

        if (cmd === 'start_remote_session') {
          state.remoteActive = true;
          state.remoteSessionId = `remote-${Date.now()}`;
          return {
            session_id: state.remoteSessionId,
            state: 'active',
            stream_url: `${window.__mockApiBase}/remote/stream`,
            frame_url: `${window.__mockApiBase}/remote/frame`,
            input_url: `${window.__mockApiBase}/remote/input`,
          };
        }

        if (cmd === 'stop_remote_session') {
          state.remoteActive = false;
          state.remoteSessionId = '';
          return null;
        }

        if (cmd === 'send_remote_input') {
          return { status: state.remoteActive ? 'accepted' : 'inactive' };
        }

        if (cmd === 'get_pair_token') return state.pairToken;
        if (cmd === 'get_local_ip') return state.localIp;
        if (cmd === 'generate_pair_qr') {
          return '<svg xmlns="http://www.w3.org/2000/svg" width="120" height="120"><rect width="120" height="120" fill="#fff"/><rect x="10" y="10" width="100" height="100" fill="#111"/></svg>';
        }
        if (cmd === 'ws_client_count') return state.wsClientCount;
        if (cmd === 'save_desktop_connection') return null;

        if (cmd === 'agent_connect_start_session') {
          const session = {
            id: `ac-${state.nextAgentSession++}`,
            goal: args.goal || null,
            workspace_path: args.workspacePath || null,
            status: 'active',
            created_at_ms: Date.now(),
            updated_at_ms: Date.now(),
            last_event_summary: 'Session started',
          };
          state.agentSessions.unshift(session);
          ensureAgentSession(session);
          pushAgentEvent(session.id, 'session.started', 'Session started');
          return session;
        }

        if (cmd === 'agent_connect_list_sessions') return state.agentSessions;
        if (cmd === 'agent_connect_get_active_session') return state.agentSessions.find((s) => s.status === 'active') || null;
        if (cmd === 'agent_connect_set_active_session') {
          const session = state.agentSessions.find((s) => s.id === args.sessionId) || null;
          if (session) {
            session.updated_at_ms = Date.now();
          }
          return session;
        }
        if (cmd === 'agent_connect_end_session') {
          const targetId = args.sessionId || (state.agentSessions.find((s) => s.status === 'active')?.id || '');
          const session = state.agentSessions.find((s) => s.id === targetId);
          if (session) {
            session.status = String(args.status || 'completed');
            session.updated_at_ms = Date.now();
            session.last_event_summary = 'Session completed';
            pushAgentEvent(session.id, 'session.completed', 'Session completed');
            return session;
          }
          throw new Error('Session not found');
        }

        if (cmd === 'agent_connect_get_timeline') {
          const list = state.agentTimeline[String(args.sessionId)] || [];
          const afterSeq = Number(args.afterSeq || 0);
          return list.filter((ev) => ev.seq > afterSeq).slice(0, Number(args.limit || 300));
        }

        if (cmd === 'spawn_pty_terminal') {
          setTimeout(() => emitTauriEvent('pty-output', '\\r\\nmock shell ready\\r\\n'), 40);
          return null;
        }
        if (cmd === 'send_to_pty') {
          const input = String(args.input || '');
          setTimeout(() => emitTauriEvent('pty-output', `executed: ${input}\\r\\n`), 20);
          return null;
        }
        if (cmd === 'resize_pty') return null;

        if (cmd === 'list_available_chat_tools') {
          return [
            { name: 'read_file', description: 'Read files', requires_approval: false, is_custom: false },
            { name: 'list_all_files', description: 'List files', requires_approval: false, is_custom: false },
            { name: 'create_dir', description: 'Create directory', requires_approval: true, is_custom: false },
            { name: 'write_file', description: 'Write file', requires_approval: true, is_custom: false },
            { name: 'run_command', description: 'Run command', requires_approval: true, is_custom: false },
          ];
        }

        if (cmd === 'submit_chat') return await handleSubmitChat(args);
        if (cmd === 'resume_tool_call') return await handleResumeToolCall(args);
        if (cmd === 'stop_chat_generation') return null;
        if (cmd === 'voice_transcribe') return '';
        if (cmd === 'stop_voice_capture') return null;

        if (cmd === 'read_file') return '';
        if (cmd === 'write_file') return null;
        if (cmd === 'execute_tool_call') return 'mock tool output';

        return null;
      },
    };

    window.__BONSAI_LIVE_TESTING_STATE = state;

    // Keep remote preview alive when a session is active.
    setInterval(() => {
      if (!state.remoteActive) return;
      emitTauriEvent('remote-frame', { frame: state.remoteFrame });
    }, 1000);
  };
}

async function installPageHarness(page) {
  const mockApiBase = 'http://127.0.0.1:11369';

  await page.exposeFunction('__liveToolExec', async (action) => {
    const tool = String(action?.tool || '');
    const args = action?.args || {};

    if (tool === 'list_all_files') {
      const root = String(args.path || WORKSPACE_ROOT);
      return {
        count: 5,
        entries: [
          { rel: 'Runner-Streaming_System.md', path: path.join(root, 'Runner-Streaming_System.md').replace(/\\/g, '/') },
          { rel: 'bonsai-workspace/src/App.svelte', path: path.join(root, 'bonsai-workspace', 'src', 'App.svelte').replace(/\\/g, '/') },
        ],
      };
    }

    if (tool === 'create_dir') {
      const dirPath = String(args.path || '').replace(/\\/g, '/');
      await fs.mkdir(dirPath, { recursive: true });
      return { message: `created ${dirPath}` };
    }

    if (tool === 'write_file') {
      const filePath = String(args.path || '').replace(/\\/g, '/');
      await fs.mkdir(path.dirname(filePath), { recursive: true });
      await fs.writeFile(filePath, String(args.content ?? ''), 'utf8');
      return { message: `wrote ${filePath}` };
    }

    if (tool === 'run_command') {
      return { message: `cwd=${WORKSPACE_ROOT}` };
    }

    return { message: `noop:${tool}` };
  });

  await page.route('**/v1/models', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ data: [{ id: 'bonsai-1' }] }),
    });
  });

  await page.route('**/remote/stream', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'text/event-stream',
      body: 'data: {"frame":"bW9jaw=="}\n\n',
    });
  });

  await page.addInitScript(() => {
    window.__bonsaiWorkspacePath = 'z:/Projects/BonsaiWorkspace';
    window.__mockApiBase = 'http://127.0.0.1:11369';

    class MockEventSource {
      constructor(url) {
        this.url = url;
        this.onmessage = null;
        this.onerror = null;
        this._closed = false;

        setTimeout(() => {
          if (this._closed || typeof this.onmessage !== 'function') return;
          this.onmessage({ data: JSON.stringify({ frame: 'bW9jaw==' }) });
        }, 100);
      }

      close() {
        this._closed = true;
      }
    }

    window.EventSource = MockEventSource;
  });
  await page.addInitScript(buildMockInitScript(), {
    liveMode: LIVE_MODE,
    tokenDelayMs: LIVE_MODE ? 70 : 0,
  });
}

async function runScenario(browser, name, fn) {
  log(`Running scenario: ${name}`);
  const page = await browser.newPage();
  await page.bringToFront();
  const consoleErrors = [];
  page.on('console', (msg) => {
    if (msg.type() === 'error') {
      consoleErrors.push(msg.text());
    }
  });

  try {
    await installPageHarness(page);
    await Promise.race([
      fn(page),
      new Promise((_, reject) => {
        setTimeout(() => reject(new Error(`scenario timeout after ${SCENARIO_TIMEOUT_MS}ms`)), SCENARIO_TIMEOUT_MS);
      }),
    ]);
    if (consoleErrors.length > 0) {
      throw new Error(`console errors: ${consoleErrors.slice(0, 3).join(' | ')}`);
    }
    pass(name);
    if (SCENARIO_PAUSE_MS > 0) {
      await sleep(SCENARIO_PAUSE_MS);
    }
  } catch (err) {
    fail(name, String(err));
    if (SCENARIO_PAUSE_MS > 0) {
      await sleep(SCENARIO_PAUSE_MS);
    }
  } finally {
    await Promise.race([
      page.close({ runBeforeUnload: false }),
      new Promise((resolve) => setTimeout(resolve, 5000)),
    ]);
  }
}

async function scenarioShell(page) {
  await page.goto(UI_BASE, { waitUntil: 'domcontentloaded', timeout: STEP_TIMEOUT_MS });
  await page.waitForSelector('.toolbar', { timeout: STEP_TIMEOUT_MS });
  await page.waitForSelector('footer.status-bar', { timeout: STEP_TIMEOUT_MS });

  await page.click('button[title="Toggle Terminal (Ctrl+`)"]');
  await page.waitForSelector('.term-title', { timeout: STEP_TIMEOUT_MS });
  await page.click('button[title="Toggle Terminal (Ctrl+`)"]');

  await page.click('button[title="Cycle Theme"]');
  await page.click('button[title="Cycle Theme"]');
  await page.click('button[title="Cycle Theme"]');
}

async function scenarioCommandPalette(page) {
  await page.goto(UI_BASE, { waitUntil: 'domcontentloaded', timeout: STEP_TIMEOUT_MS });
  await page.waitForSelector('textarea.chat-input', { timeout: STEP_TIMEOUT_MS });

  await page.click('body');
  await page.keyboard.press('Control+K');

  let paletteVisible = true;
  try {
    await page.waitForSelector('.palette', { timeout: 2500 });
  } catch {
    paletteVisible = false;
  }

  if (!paletteVisible) {
    await page.keyboard.press('Meta+K');
    try {
      await page.waitForSelector('.palette', { timeout: 2500 });
      paletteVisible = true;
    } catch {
      paletteVisible = false;
    }
  }

  if (!paletteVisible) {
    await page.evaluate(() => {
      window.dispatchEvent(new KeyboardEvent('keydown', { key: 'k', ctrlKey: true, bubbles: true }));
    });
    await page.waitForSelector('.palette', { timeout: STEP_TIMEOUT_MS });
  }

  await page.fill('.palette-input', 'Toggle Terminal');
  await page.waitForFunction(
    () => document.body.innerText.includes('Toggle Terminal'),
    null,
    { timeout: STEP_TIMEOUT_MS },
  );
  await page.keyboard.press('Escape');
  await page.waitForSelector('.palette', { state: 'hidden', timeout: STEP_TIMEOUT_MS });
}

async function scenarioSettingsRemotePairing(page) {
  await page.goto(UI_BASE, { waitUntil: 'domcontentloaded', timeout: STEP_TIMEOUT_MS });
  await page.click('button[title="Settings"]');
  await page.waitForSelector('.settings-panel', { timeout: STEP_TIMEOUT_MS });

  await page.locator('button:has-text("Test API")').scrollIntoViewIfNeeded();
  await page.click('button:has-text("Test API")');
  await waitForAnyBodyTextSafe(page, ['API reachable', 'API error', 'API test failed']);

  await page.locator('button:has-text("Save API settings")').scrollIntoViewIfNeeded();
  await page.click('button:has-text("Save API settings")');
  await waitForAnyBodyTextSafe(page, ['API settings saved', 'Save failed']);

  await page.click('button:has-text("Start Remote Session")');
  await page.waitForSelector('.remote-info', { timeout: STEP_TIMEOUT_MS });
  await page.click('button:has-text("Send Test Click")');
  await waitForAnyBodyTextSafe(page, ['Remote input accepted', 'Remote input failed']);
  await page.click('button:has-text("Stop Remote Session")');
  await waitForAnyBodyTextSafe(page, ['Remote session stopped', 'Failed to stop remote session']);

  await page.locator('button:has-text("Show QR Code")').scrollIntoViewIfNeeded();
  await page.click('button:has-text("Show QR Code")');
  await page.waitForSelector('.pair-token', { timeout: STEP_TIMEOUT_MS });
  await page.locator('button:has-text("Scan Mobile QR")').scrollIntoViewIfNeeded();
  await page.click('button:has-text("Scan Mobile QR")');
  await waitForAnyBodyTextSafe(page, ['Saved desktop connection', 'Scan failed']);

  await page.click('button[aria-label="Close settings"]');
}

async function scenarioAgentConnect(page) {
  await page.goto(UI_BASE, { waitUntil: 'domcontentloaded', timeout: STEP_TIMEOUT_MS });
  await page.click('button[title="Agent Connect"]');
  await page.waitForSelector('.agent-connect-panel', { timeout: STEP_TIMEOUT_MS });

  await page.fill('input[placeholder="Session goal (optional)"]', 'Live feature test session');
  await page.click('button:has-text("Start Session")');
  await waitForBodyText(page, 'Session started');
  await waitForBodyText(page, 'ac-1');

  await page.click('button:has-text("End Session")');
  await waitForBodyText(page, 'Session completed');
  await page.click('button[aria-label="Close Agent Connect"]');
}

async function scenarioChatHitlTooling(page) {
  await page.goto(UI_BASE, { waitUntil: 'domcontentloaded', timeout: STEP_TIMEOUT_MS });
  await page.waitForSelector('textarea.chat-input', { timeout: STEP_TIMEOUT_MS });

  await page.click('button.btn-open');
  await page.waitForSelector('.ws-name', { timeout: STEP_TIMEOUT_MS });

  await page.fill('textarea.chat-input', 'List all files in the current directory');
  await page.click('button.btn-send');
  await waitForAnyBodyTextSafe(page, ['Used list_all_files', 'Listed files']);

  await page.fill('textarea.chat-input', 'Create folder tool_test/live-testing and then create hello.txt containing Hello.');
  await page.click('button.btn-send');

  await page.waitForSelector('.perm-card .btn-approve', { timeout: STEP_TIMEOUT_MS });
  await page.click('.perm-card .btn-approve');
  await page.waitForSelector('.perm-card .btn-approve', { timeout: STEP_TIMEOUT_MS });
  await page.click('.perm-card .btn-approve');

  const helloPath = path.join(WORKSPACE_ROOT, 'tool_test', 'live-testing', 'hello.txt');
  const wroteExpected = await waitForFileContent(helloPath, 'Hello\n', STEP_TIMEOUT_MS);
  if (!wroteExpected) {
    throw new Error('hello.txt was not created with expected content before timeout');
  }

  await page.fill('textarea.chat-input', 'Try writing again and I will deny');
  await page.click('button.btn-send');
  await page.waitForSelector('.perm-card .btn-deny', { timeout: STEP_TIMEOUT_MS });
  await page.click('.perm-card .btn-deny');
  await waitForAnyBodyTextSafe(page, ['Denied. I did not run the requested tool action.', 'Denied']);
}

function printSummaryAndExit() {
  const passed = suiteResults.filter((x) => x.ok).length;
  const failed = suiteResults.filter((x) => !x.ok).length;

  log('');
  log('Bonsai Workspace Live-Testing Feature Summary');
  for (const r of suiteResults) {
    log(` - ${r.ok ? 'PASS' : 'FAIL'} ${r.name}${r.detail ? ` :: ${r.detail}` : ''}`);
  }
  log(`Total: ${passed} passed, ${failed} failed`);

  if (failed > 0) {
    process.exitCode = 1;
  }
}

async function main() {
  log(`Bonsai Workspace Live-Testing Feature`);
  log(`UI base: ${UI_BASE}`);
  log(`Mode: ${HEADLESS ? 'headless' : 'visible'} slowMo=${SLOW_MO_MS}ms`);
  log(`Scenario pause: ${SCENARIO_PAUSE_MS}ms`);

  let devServer = null;
  let startedDevServer = false;

  try {
    await cleanupArtifacts();

    const alreadyReady = await waitForUrl(UI_BASE, 1500);
    if (alreadyReady) {
      log(`Reusing existing dev server at ${UI_BASE}`);
    } else {
      log('Starting frontend dev server...');
      devServer = startDevServer();
      startedDevServer = true;
      const ready = await waitForUrl(UI_BASE, 90000);
      if (!ready) throw new Error(`Dev server did not become ready at ${UI_BASE}`);
      log(`Dev server ready at ${UI_BASE}`);
    }

    const playwright = await import('playwright');
    const browser = await playwright.chromium.launch({
      headless: HEADLESS,
      ...(SLOW_MO_MS > 0 ? { slowMo: SLOW_MO_MS } : {}),
    });

    try {
      await runScenario(browser, 'Shell Layout and Toggles', scenarioShell);
      await runScenario(browser, 'Command Palette Session', scenarioCommandPalette);
      await runScenario(browser, 'Settings Remote Pairing Session', scenarioSettingsRemotePairing);
      await runScenario(browser, 'Agent Connect Timeline Session', scenarioAgentConnect);
      await runScenario(browser, 'Chat HITL Tooling Session', scenarioChatHitlTooling);

      if (LIVE_MODE && KEEP_OPEN_MS > 0 && suiteResults.every((r) => r.ok)) {
        log(`Keeping browser open for ${KEEP_OPEN_MS}ms for visual confirmation...`);
        await sleep(KEEP_OPEN_MS);
      }
    } finally {
      await browser.close();
    }
  } finally {
    await cleanupArtifacts();

    if (startedDevServer && devServer && !devServer.killed) {
      devServer.kill();
    }
  }

  printSummaryAndExit();
}

main().catch((err) => {
  log(`FATAL ${String(err)}`);
  process.exitCode = 1;
});
