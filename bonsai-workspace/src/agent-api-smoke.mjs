#!/usr/bin/env node

const API_BASE = process.env.BONSAI_API_BASE || 'http://127.0.0.1:11369';
const UI_BASE = process.env.BONSAI_UI_BASE || 'http://localhost:1420';
const REQUEST_TIMEOUT_MS = Number(process.env.BONSAI_SMOKE_TIMEOUT_MS || 12000);
const UI_LIVE = process.env.BONSAI_UI_LIVE === '1';
const UI_HEADLESS = process.env.BONSAI_UI_HEADLESS
  ? process.env.BONSAI_UI_HEADLESS === '1'
  : !UI_LIVE;
const UI_SLOW_MO_MS = Number(process.env.BONSAI_UI_SLOW_MO_MS || (UI_LIVE ? 120 : 0));
const UI_KEEP_OPEN_MS = Number(process.env.BONSAI_UI_KEEP_OPEN_MS || (UI_LIVE ? 12000 : 0));
const UI_STEP_TIMEOUT_MS = Number(process.env.BONSAI_UI_STEP_TIMEOUT_MS || 45000);

const SKIP_API = process.env.BONSAI_SKIP_API === '1';
const SKIP_UI = process.env.BONSAI_SKIP_UI === '1';

const results = [];

function log(msg) {
  process.stdout.write(`${msg}\n`);
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function waitForBodyText(page, text, timeoutMs = UI_STEP_TIMEOUT_MS) {
  await page.waitForFunction(
    (needle) => document.body.innerText.includes(needle),
    text,
    { timeout: timeoutMs },
  );
}

async function waitForPermissionCardDismissal(page, action, timeoutMs = UI_STEP_TIMEOUT_MS) {
  const selector = action === 'approve'
    ? '.perm-card .btn-approve'
    : '.perm-card .btn-deny';

  await Promise.any([
    page.waitForSelector(selector, { state: 'hidden', timeout: timeoutMs }),
    page.waitForSelector(selector, { state: 'detached', timeout: timeoutMs }),
  ]);
}

function pass(name, detail = '') {
  results.push({ name, ok: true, detail });
  log(`PASS ${name}${detail ? ` - ${detail}` : ''}`);
}

function fail(name, detail = '') {
  results.push({ name, ok: false, detail });
  log(`FAIL ${name}${detail ? ` - ${detail}` : ''}`);
}

async function fetchWithTimeout(baseUrl, path, init = {}, timeoutMs = REQUEST_TIMEOUT_MS) {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeoutMs);
  try {
    return await fetch(`${baseUrl}${path}`, {
      ...init,
      signal: controller.signal,
      headers: {
        'content-type': 'application/json',
        ...(init.headers || {}),
      },
    });
  } finally {
    clearTimeout(timer);
  }
}

async function expectJson(baseUrl, path, name, init = {}) {
  try {
    const resp = await fetchWithTimeout(baseUrl, path, init);
    const text = await resp.text();
    let data = null;
    try {
      data = text ? JSON.parse(text) : {};
    } catch {
      fail(name, `non-JSON response (HTTP ${resp.status})`);
      return null;
    }
    if (!resp.ok) {
      fail(name, `HTTP ${resp.status} ${JSON.stringify(data).slice(0, 180)}`);
      return null;
    }
    pass(name, `HTTP ${resp.status}`);
    return data;
  } catch (err) {
    fail(name, String(err));
    return null;
  }
}

async function testSseRemoteStream(apiBase) {
  const name = 'remote stream (SSE frame event)';
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), 15000);

  try {
    const resp = await fetch(`${apiBase}/remote/stream`, {
      method: 'GET',
      signal: controller.signal,
    });

    if (!resp.ok || !resp.body) {
      fail(name, `HTTP ${resp.status}`);
      return;
    }

    const reader = resp.body.getReader();
    const decoder = new TextDecoder();
    let buffer = '';
    let gotFrame = false;

    while (!gotFrame) {
      const { value, done } = await reader.read();
      if (done) break;
      buffer += decoder.decode(value, { stream: true });

      const lines = buffer.split('\n');
      buffer = lines.pop() || '';

      for (const line of lines) {
        if (!line.startsWith('data: ')) continue;
        const payload = line.slice(6).trim();
        try {
          const parsed = JSON.parse(payload);
          if (parsed.frame && typeof parsed.frame === 'string' && parsed.frame.length > 32) {
            gotFrame = true;
            break;
          }
        } catch {
          // ignore non-json keepalive or error payloads
        }
      }
    }

    if (gotFrame) {
      pass(name, 'received frame payload');
    } else {
      fail(name, 'no frame payload before timeout/EOF');
    }
  } catch (err) {
    fail(name, String(err));
  } finally {
    clearTimeout(timer);
    controller.abort();
  }
}

async function runApiSmoke() {
  const apiCandidates = process.env.BONSAI_API_BASE
    ? [process.env.BONSAI_API_BASE]
    : ['http://127.0.0.1:11369', 'http://127.0.0.1:12469'];

  let liveApiBase = apiCandidates[0];
  let health = null;
  for (const candidate of apiCandidates) {
    log(`Bonsai API smoke test base: ${candidate}`);
    try {
      const resp = await fetchWithTimeout(candidate, '/health');
      const text = await resp.text();
      const data = text ? JSON.parse(text) : {};
      if (resp.ok) {
        health = data;
        liveApiBase = candidate;
        pass('health endpoint', `HTTP ${resp.status}`);
        break;
      }
    } catch {
      // Probe failed; try next candidate.
    }
  }

  if (!health) {
    fail('health endpoint', `unreachable at candidates: ${apiCandidates.join(', ')}`);
    log('');
    log('API is unreachable. Start Bonsai app first (which starts the built-in API server), then re-run this smoke test.');
    return;
  }

  const version = await expectJson(liveApiBase, '/api/version', 'version endpoint');
  if (version && typeof version.version === 'string') {
    pass('version shape', version.version);
  } else {
    fail('version shape', 'missing version string');
  }

  const models = await expectJson(liveApiBase, '/v1/models', 'openai models endpoint');
  if (models && Array.isArray(models.data)) {
    pass('openai models payload shape', `${models.data.length} model(s)`);
  } else {
    fail('openai models payload shape', 'missing data array');
  }

  const tags = await expectJson(liveApiBase, '/api/tags', 'ollama tags endpoint');
  if (tags && Array.isArray(tags.models)) {
    pass('ollama tags payload shape', `${tags.models.length} model(s)`);
  } else {
    fail('ollama tags payload shape', 'missing models array');
  }

  const start = await expectJson(liveApiBase, '/remote/session/start', 'remote session start', { method: 'POST', body: '{}' });
  if (!start?.session_id) {
    fail('remote session id', 'session_id missing');
    return;
  }
  pass('remote session id', start.session_id);

  const offer = await expectJson(liveApiBase, '/remote/session/offer', 'remote session offer', {
    method: 'POST',
    body: JSON.stringify({ type: 'offer', sdp: 'dummy-sdp-for-smoke-test' }),
  });
  if (offer?.answer?.status === 'ready') {
    pass('remote offer answer shape', 'ready');
  } else {
    fail('remote offer answer shape', JSON.stringify(offer || {}).slice(0, 160));
  }

  try {
    const frameResp = await fetchWithTimeout(liveApiBase, '/remote/frame', { method: 'GET', headers: {} }, 20000);
    const bytes = new Uint8Array(await frameResp.arrayBuffer());
    const ctype = frameResp.headers.get('content-type') || '';
    if (frameResp.ok && ctype.includes('image/png') && bytes.length > 128) {
      pass('remote frame capture', `${bytes.length} bytes`);
    } else {
      fail('remote frame capture', `HTTP ${frameResp.status} content-type=${ctype} bytes=${bytes.length}`);
    }
  } catch (err) {
    fail('remote frame capture', String(err));
  }

  await testSseRemoteStream(liveApiBase);

  const input = await expectJson(liveApiBase, '/remote/input', 'remote input event', {
    method: 'POST',
    body: JSON.stringify({ event_type: 'mouse_move', x: 120, y: 80 }),
  });
  if (input?.status === 'accepted') {
    pass('remote input accepted shape', 'accepted');
  } else {
    fail('remote input accepted shape', JSON.stringify(input || {}).slice(0, 160));
  }

  await expectJson(liveApiBase, '/remote/session/stop', 'remote session stop', { method: 'POST', body: '{}' });

  try {
    const postStop = await fetchWithTimeout(liveApiBase, '/remote/frame', { method: 'GET', headers: {} }, 15000);
    if (!postStop.ok) {
      pass('remote frame denied after stop', `HTTP ${postStop.status}`);
    } else {
      fail('remote frame denied after stop', 'expected non-2xx after stop');
    }
  } catch (err) {
    fail('remote frame denied after stop', String(err));
  }
}

function buildTauriMockInitScript() {
  return (config = {}) => {
    const liveMode = Boolean(config.liveMode);
    const tokenDelayMs = Number(config.tokenDelayMs || 0);

    const callbackMap = new Map();
    const listeners = new Map();
    let cbId = 1;
    let eventId = 1;

    function browserSleep(ms) {
      return new Promise((resolve) => setTimeout(resolve, ms));
    }

    async function emitStreamText(text) {
      // Emit small chunks so ChatPanel renders a true streaming transcript.
      const chunks = String(text).match(/.{1,6}/g) || [String(text)];
      for (const chunk of chunks) {
        emitTauriEvent('token-stream', chunk);
        if (tokenDelayMs > 0) await browserSleep(tokenDelayMs);
      }
      emitTauriEvent('token-speed', tokenDelayMs > 0 ? Math.max(1, Math.round(1000 / tokenDelayMs) * 6) : 60);
    }

    function emitTauriEvent(eventName, payload) {
      const eventListeners = listeners.get(eventName) || [];
      for (const entry of eventListeners) {
        const rec = callbackMap.get(entry.handlerId);
        if (!rec) continue;
        try {
          rec.cb({ event: eventName, id: entry.id, payload });
          if (rec.once) callbackMap.delete(entry.handlerId);
        } catch {
          // ignore callback errors in smoke mode
        }
      }
    }

    window.__BonsaiMockTauri = {
      emit: emitTauriEvent,
      submitChatCalls: 0,
      lastSubmitChatArgs: null,
      lastToolUsed: '',
    };

    window.__TAURI_EVENT_PLUGIN_INTERNALS__ = {
      unregisterListener(eventName, id) {
        const arr = listeners.get(eventName) || [];
        listeners.set(eventName, arr.filter((x) => x.id !== id));
      },
    };

    window.__TAURI_INTERNALS__ = {
      metadata: { currentWindow: { label: 'main' } },
      transformCallback(cb, once = false) {
        const id = cbId++;
        callbackMap.set(id, { cb, once });
        return id;
      },
      unregisterCallback(id) {
        callbackMap.delete(id);
      },
      convertFileSrc(filePath) {
        return filePath;
      },
      async invoke(cmd, args = {}) {
        if (cmd === 'plugin:event|listen') {
          const id = eventId++;
          const eventName = args.event;
          const arr = listeners.get(eventName) || [];
          arr.push({ id, handlerId: args.handler });
          listeners.set(eventName, arr);
          return id;
        }

        if (cmd === 'plugin:event|unlisten') {
          const arr = listeners.get(args.event) || [];
          listeners.set(args.event, arr.filter((x) => x.id !== args.eventId));
          return null;
        }

        if (cmd === 'get_current_session_state') {
          return { current_session_id: null, current_session_title: null };
        }
        if (cmd === 'set_current_session_state') return null;
        if (cmd === 'save_chat_session') return { id: 'mock-session-id' };
        if (cmd === 'load_chat_session') return { id: 'mock-session-id', title: 'Mock', messages: [] };
        if (cmd === 'list_chat_sessions') return [];
        if (cmd === 'list_models_registry') return [];
        if (cmd === 'get_orchestrator_status') return { slots: [], queue_depth: 0, total_ram_mb: 0, free_ram_mb: 0 };
        if (cmd === 'list_available_chat_tools') {
          return [
            { name: 'read_file', description: 'Read files', requires_approval: false, is_custom: false },
            { name: 'list_all_files', description: 'Recursively list all files', requires_approval: false, is_custom: false },
            { name: 'write_file', description: 'Write files', requires_approval: true, is_custom: false },
            { name: 'run_command', description: 'Run shell command', requires_approval: true, is_custom: false },
          ];
        }
        if (cmd === 'execute_tool_call') {
          return 'mock tool output';
        }

        if (cmd === 'submit_chat') {
          window.__BonsaiMockTauri.submitChatCalls += 1;
          window.__BonsaiMockTauri.lastSubmitChatArgs = args;
          const history = Array.isArray(args.messages) ? args.messages : [];
          const lastUserText = [...history]
            .reverse()
            .find((m) => m?.role === 'user' && typeof m?.content === 'string')?.content || '';
          const looksLikeListRequest = /(list|show|display|enumerate)\s+.*(files|directory|folder)|\b(list files|readme|read me|read file|read the file|show files)\b/i.test(lastUserText);
          const enabledTools = Array.isArray(args.enabledTools) ? args.enabledTools : [];

          if (looksLikeListRequest) {
            if (args.workspacePath && enabledTools.includes('list_all_files')) {
              window.__BonsaiMockTauri.lastToolUsed = 'list_all_files';
              const reply = 'Listed files using list_all_files.';
              await emitStreamText(reply);
              return {
                content: reply,
                action_handled: false,
                tools_used: ['list_all_files'],
                stats: {
                  prompt_tokens: 16,
                  completion_tokens: 8,
                  tokens_per_second: 14.0,
                  time_to_first_token_ms: 12,
                  total_time_ms: 110,
                },
              };
            }

            window.__BonsaiMockTauri.lastToolUsed = '';
            const reply = 'I can list files with available tools, but no workspace is open.';
            await emitStreamText(reply);
            return {
              content: reply,
              action_handled: false,
              tools_used: [],
              stats: {
                prompt_tokens: 12,
                completion_tokens: 10,
                tokens_per_second: 10.0,
                time_to_first_token_ms: 10,
                total_time_ms: 95,
              },
            };
          }

          const hasToolResult = history.some((m) => typeof m?.content === 'string' && m.content.includes('<tool_result>'));

          if (hasToolResult) {
            const reply = 'Mock tool executed successfully.';
            await emitStreamText(reply);
            return {
              content: reply,
              action_handled: false,
              tools_used: ['write_file'],
              stats: {
                prompt_tokens: 12,
                completion_tokens: 8,
                tokens_per_second: 12.0,
                time_to_first_token_ms: 15,
                total_time_ms: 90,
              },
            };
          }

          const payload = {
            type: 'tool_approval',
            tool: 'write_file',
            args: {
              path: 'C:/mock/file.txt',
              content: 'hello from mock',
            },
            description: 'Write file: C:/mock/file.txt',
            rationale: 'Mock model requested permission to write a file.',
            paths_affected: ['C:/mock/file.txt'],
            action: { tool: 'write_file', args: { path: 'C:/mock/file.txt', content: 'hello from mock' } },
            raw_response: '<tool_call>{"tool":"write_file","args":{"path":"C:/mock/file.txt","content":"hello from mock"}}</tool_call>',
            ctx_snapshot: history,
          };

          if (liveMode) {
            await emitStreamText('I need approval before I can write this file.');
            await browserSleep(Math.max(300, tokenDelayMs * 4));
          }

          emitTauriEvent('permission-request', payload);

          return {
            content: '',
            action_handled: true,
            tools_used: [],
            stats: {
              prompt_tokens: 10,
              completion_tokens: 1,
              tokens_per_second: 1.0,
              time_to_first_token_ms: 10,
              total_time_ms: 40,
            },
          };
        }

        if (cmd === 'voice_transcribe') return '';
        if (cmd === 'stop_voice_capture') return null;
        if (cmd === 'stop_chat_generation') return null;
        if (cmd === 'read_file') return '';
        if (cmd === 'write_file') return null;
        if (cmd === 'open_workspace') return 'C:/mock';
        if (cmd === 'list_project_files') {
          return [
            { path: 'C:/mock/README.md', rel: 'README.md', name: 'README.md', is_dir: false },
            { path: 'C:/mock/src', rel: 'src', name: 'src', is_dir: true },
            { path: 'C:/mock/src/main.ts', rel: 'src/main.ts', name: 'main.ts', is_dir: false },
          ];
        }

        return null;
      },
    };
  };
}

async function runUiHitlSmoke() {
  const namePrefix = 'ui hitl';
  log(`Scripted UI HITL smoke base: ${UI_BASE}`);
  if (UI_LIVE) {
    log('UI live mode enabled: browser is visible and conversation is paced for real-time viewing.');
  }

  let playwright;
  try {
    playwright = await import('playwright');
  } catch (err) {
    fail(`${namePrefix} playwright import`, String(err));
    return;
  }

  const browser = await playwright.chromium.launch({
    headless: UI_HEADLESS,
    ...(UI_SLOW_MO_MS > 0 ? { slowMo: UI_SLOW_MO_MS } : {}),
  });
  const page = await browser.newPage();
  const consoleErrors = [];

  page.on('console', (msg) => {
    if (msg.type() === 'error') {
      consoleErrors.push(msg.text());
    }
  });

  await page.addInitScript(buildTauriMockInitScript(), {
    liveMode: UI_LIVE,
    tokenDelayMs: UI_LIVE ? 85 : 0,
  });

  try {
    await page.goto(UI_BASE, { waitUntil: 'domcontentloaded', timeout: UI_STEP_TIMEOUT_MS });
    pass(`${namePrefix} load app`);

    await page.waitForSelector('textarea.chat-input', { timeout: UI_STEP_TIMEOUT_MS });
    pass(`${namePrefix} chat input visible`);

    // Regression: without a workspace, file-list requests must be blocked with guidance
    // before submit_chat is called.
    await page.fill('textarea.chat-input', 'List all files in the current directory');
    await page.click('button.btn-send');
    await waitForBodyText(page, 'No workspace folder is open yet.');
    const submitCallsWithoutWorkspace = await page.evaluate(
      () => Number(window.__BonsaiMockTauri?.submitChatCalls || 0),
    );
    if (submitCallsWithoutWorkspace === 0) {
      pass(`${namePrefix} no-workspace file request guard`);
    } else {
      fail(`${namePrefix} no-workspace file request guard`, `submit_chat called ${submitCallsWithoutWorkspace} time(s)`);
    }

    // Regression: once a workspace is open, list-file requests should route through
    // list_all_files instead of producing a capability-only answer.
    await page.click('button.btn-open');
    await page.waitForSelector('.ws-name', { timeout: UI_STEP_TIMEOUT_MS });
    await page.fill('textarea.chat-input', 'List all files in the current directory');
    await page.click('button.btn-send');
    await page.waitForFunction(
      () => window.__BonsaiMockTauri?.lastToolUsed === 'list_all_files',
      null,
      { timeout: UI_STEP_TIMEOUT_MS },
    );
    await waitForBodyText(page, 'Listed files using list_all_files.');
    pass(`${namePrefix} workspace list request uses list_all_files`);

    if (UI_LIVE) {
      await page.click('textarea.chat-input');
      await page.type('textarea.chat-input', 'Please create a file for me', { delay: 30 });
    } else {
      await page.fill('textarea.chat-input', 'Please create a file for me');
    }
    await page.click('button.btn-send');

    await page.waitForSelector('.perm-card .btn-approve', { timeout: UI_STEP_TIMEOUT_MS });
    pass(`${namePrefix} permission card rendered`);

    await page.click('.perm-card .btn-approve');
    await sleep(250);
    pass(`${namePrefix} approve flow complete`);

    if (UI_LIVE) {
      await page.click('textarea.chat-input');
      await page.type('textarea.chat-input', 'Do another write and I will deny', { delay: 30 });
    } else {
      await page.fill('textarea.chat-input', 'Do another write and I will deny');
    }
    await page.click('button.btn-send');
    await page.waitForSelector('.perm-card .btn-deny', { timeout: UI_STEP_TIMEOUT_MS });
    await page.click('.perm-card .btn-deny');
    await sleep(250);
    pass(`${namePrefix} deny flow complete`);

    if (consoleErrors.length === 0) {
      pass(`${namePrefix} console errors`, 'none');
    } else {
      fail(`${namePrefix} console errors`, consoleErrors.slice(0, 3).join(' | '));
    }

    if (UI_LIVE && UI_KEEP_OPEN_MS > 0) {
      log(`Keeping UI open for ${UI_KEEP_OPEN_MS}ms so the session can be observed.`);
      await sleep(UI_KEEP_OPEN_MS);
    }
  } catch (err) {
    fail(`${namePrefix} scripted session`, String(err));
  } finally {
    await browser.close();
  }
}

async function run() {
  if (!SKIP_API) {
    await runApiSmoke();
  } else {
    log('Skipping API smoke tests (BONSAI_SKIP_API=1).');
  }

  if (!SKIP_UI) {
    await runUiHitlSmoke();
  } else {
    log('Skipping UI HITL smoke tests (BONSAI_SKIP_UI=1).');
  }

  summarizeAndExit();
}

function summarizeAndExit() {
  const passed = results.filter((r) => r.ok).length;
  const failed = results.filter((r) => !r.ok).length;
  log('');
  log(`Summary: ${passed} passed, ${failed} failed`);

  if (failed > 0) {
    process.exitCode = 1;
  }
}

run().catch((err) => {
  log(`FATAL ${String(err)}`);
  process.exitCode = 1;
});
