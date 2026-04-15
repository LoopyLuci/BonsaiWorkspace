#!/usr/bin/env node

import { spawn } from 'node:child_process';
import os from 'node:os';
import path from 'node:path';
import fs from 'node:fs';

const ROOT = process.cwd();
const TAURI_DIR = path.resolve(ROOT, '../src-tauri');
const UI_BASE = process.env.BONSAI_UI_BASE || 'http://localhost:1420';
const ORCHESTRATED_API_PORT = Number(process.env.BONSAI_ORCHESTRATED_API_PORT || 11369);

function getConfiguredApiBase() {
  const fallback = 'http://127.0.0.1:11369';
  try {
    if (process.env.BONSAI_API_BASE) return process.env.BONSAI_API_BASE;

    const appData = process.env.APPDATA || path.join(os.homedir(), 'AppData', 'Roaming');
    const cfgPath = path.join(appData, 'com.bonsai.workspace', 'bonsai-config.json');
    if (!fs.existsSync(cfgPath)) return fallback;

    const cfg = JSON.parse(fs.readFileSync(cfgPath, 'utf8'));
    const host = cfg.api_host || '127.0.0.1';
    const port = Number(cfg.api_port || 11369);
    return `http://${host}:${port}`;
  } catch {
    return fallback;
  }
}

function appDataConfigPath() {
  const appData = process.env.APPDATA || path.join(os.homedir(), 'AppData', 'Roaming');
  return path.join(appData, 'com.bonsai.workspace', 'bonsai-config.json');
}

function enforceOrchestratedApiPort(port) {
  const cfgPath = appDataConfigPath();
  fs.mkdirSync(path.dirname(cfgPath), { recursive: true });

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
      // Keep defaults if existing config is malformed.
    }
  }

  fs.writeFileSync(cfgPath, JSON.stringify(cfg, null, 2));
  return `http://${cfg.api_host}:${port}`;
}

let API_BASE = getConfiguredApiBase();

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function waitHttp(url, timeoutMs) {
  const start = Date.now();
  let lastErr = 'unknown';

  while (Date.now() - start < timeoutMs) {
    try {
      const resp = await fetch(url);
      if (resp.ok) return;
      lastErr = `HTTP ${resp.status}`;
    } catch (err) {
      lastErr = String(err);
    }
    await sleep(500);
  }

  throw new Error(`Timed out waiting for ${url}: ${lastErr}`);
}

function streamPrefix(prefix, stream) {
  stream.on('data', (chunk) => {
    process.stdout.write(`[${prefix}] ${chunk.toString()}`);
  });
}

function killTree(proc) {
  if (!proc || proc.killed) return;
  if (process.platform === 'win32') {
    spawn('taskkill', ['/PID', String(proc.pid), '/T', '/F'], { stdio: 'ignore' });
  } else {
    proc.kill('SIGTERM');
  }
}

async function run() {
  API_BASE = enforceOrchestratedApiPort(ORCHESTRATED_API_PORT);

  console.log('Starting Bonsai stack...');
  console.log(`UI base: ${UI_BASE}`);
  console.log(`API base: ${API_BASE}`);

  const tauriDev = spawn('cargo', ['tauri', 'dev'], {
    cwd: TAURI_DIR,
    shell: true,
    env: { ...process.env },
    stdio: ['ignore', 'pipe', 'pipe'],
  });

  let boundApiBase = '';
  let bindFailed = false;

  tauriDev.stdout.on('data', (chunk) => {
    const text = chunk.toString();
    process.stdout.write(`[tauri] ${text}`);

    const m = text.match(/\[api\]\s+Bonsai API server listening on\s+(http:\/\/[^\s]+)/i);
    if (m && m[1]) {
      boundApiBase = m[1].trim();
    }

    if (text.includes('Failed to bind')) {
      bindFailed = true;
    }
  });
  streamPrefix('tauri', tauriDev.stderr);

  const exitGuard = new Promise((_, reject) => {
    tauriDev.on('exit', (code) => {
      reject(new Error(`cargo tauri dev exited early with code ${code}`));
    });
  });

  try {
    await Promise.race([
      (async () => {
        await waitHttp(`${UI_BASE}`, 120000);
        const healthBase = boundApiBase || API_BASE;
        if (bindFailed) {
          throw new Error(`API bind failed (requested base: ${healthBase})`);
        }
        await waitHttp(`${healthBase}/health`, 120000);
        API_BASE = healthBase;
      })(),
      exitGuard,
    ]);

    console.log('Stack ready. Running full smoke suite...');

    const smoke = spawn(process.platform === 'win32' ? 'npm.cmd' : 'npm', ['run', 'test:agent-all'], {
      cwd: ROOT,
      shell: true,
      env: {
        ...process.env,
        BONSAI_API_BASE: API_BASE,
        BONSAI_UI_BASE: UI_BASE,
      },
      stdio: 'inherit',
    });

    const smokeCode = await new Promise((resolve) => {
      smoke.on('exit', (code) => resolve(code ?? 1));
    });

    if (smokeCode !== 0) {
      throw new Error(`Smoke suite failed with code ${smokeCode}`);
    }

    console.log('All orchestrated tests passed.');
  } finally {
    console.log('Tearing down Bonsai stack...');
    killTree(tauriDev);
  }
}

run().catch((err) => {
  console.error(`Orchestration failed: ${String(err)}`);
  process.exitCode = 1;
});
