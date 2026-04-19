#!/usr/bin/env node

import { spawn, spawnSync } from 'node:child_process';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import os from 'node:os';
import fs from 'node:fs';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const TAURI_DIR = resolve(__dirname, '../src-tauri');
const APP_WINDOW_API_PORT = Number(process.env.BONSAI_APP_WINDOW_API_PORT || 11369);

function log(msg) {
  process.stdout.write(`${msg}\n`);
}

function startTauri() {
  const command = process.platform === 'win32' ? 'cargo tauri dev' : 'cargo tauri dev';
  const child = spawn(command, {
    cwd: TAURI_DIR,
    shell: true,
    stdio: 'inherit',
    env: { ...process.env },
  });

  child.on('exit', (code) => {
    process.exitCode = code ?? 0;
  });

  return child;
}

function appConfigPath() {
  const appData = process.env.APPDATA || resolve(os.homedir(), 'AppData', 'Roaming');
  return resolve(appData, 'com.bonsai.workspace', 'bonsai-config.json');
}

function enforceApiPort(port) {
  const cfgPath = appConfigPath();
  fs.mkdirSync(dirname(cfgPath), { recursive: true });

  let cfg = {
    api_host: '127.0.0.1',
    api_port: port,
    current_session_id: null,
    current_session_title: null,
  };

  if (fs.existsSync(cfgPath)) {
    try {
      const parsed = JSON.parse(fs.readFileSync(cfgPath, 'utf8'));
      cfg = {
        ...cfg,
        ...parsed,
        api_host: parsed.api_host || '127.0.0.1',
        api_port: port,
      };
    } catch {
      // keep defaults if config is malformed
    }
  }

  fs.writeFileSync(cfgPath, JSON.stringify(cfg, null, 2));
  return `http://${cfg.api_host}:${port}`;
}

function pidsListeningOnPort(port) {
  const result = spawnSync('netstat', ['-ano'], {
    encoding: 'utf8',
    windowsHide: true,
  });
  const text = `${result.stdout || ''}`;
  const lines = text.split(/\r?\n/);
  const suffix = `:${port}`;
  const pids = new Set();

  for (const line of lines) {
    const trimmed = line.trim();
    if (!trimmed.startsWith('TCP')) continue;
    if (!trimmed.includes(suffix)) continue;
    if (!trimmed.includes('LISTENING')) continue;

    const cols = trimmed.split(/\s+/);
    const pid = cols[cols.length - 1];
    if (pid && /^\d+$/.test(pid)) pids.add(pid);
  }

  return [...pids];
}

function killPid(pid) {
  spawnSync('taskkill', ['/F', '/PID', String(pid)], {
    encoding: 'utf8',
    windowsHide: true,
  });
}

async function freeDevPortIfNeeded() {
  if (process.platform !== 'win32') return;
  // Best-effort cleanup for stale listeners that break Tauri beforeDevCommand/API bind.
  for (const pid of pidsListeningOnPort(1420)) killPid(pid);
  for (const pid of pidsListeningOnPort(11369)) killPid(pid);
  for (const pid of pidsListeningOnPort(APP_WINDOW_API_PORT)) killPid(pid);

  // Kill stale app process instances if still running detached.
  spawnSync('taskkill', ['/IM', 'bonsai-workspace.exe', '/F'], {
    encoding: 'utf8',
    windowsHide: true,
  });
}

log('Launching Bonsai Workspace in an independent desktop app window...');
log('This is the app-window live mode (not browser Playwright mode).');
log('Once the app opens, use Agent Connect / Chat to run a live streaming conversation and HITL approvals.');
log(`Dedicated app-window API port: ${APP_WINDOW_API_PORT} (override with BONSAI_APP_WINDOW_API_PORT).`);
log('Press Ctrl+C in this terminal to stop the app.');

await freeDevPortIfNeeded();
const apiBase = enforceApiPort(APP_WINDOW_API_PORT);
log(`Configured API base for app-window mode: ${apiBase}`);
startTauri();
