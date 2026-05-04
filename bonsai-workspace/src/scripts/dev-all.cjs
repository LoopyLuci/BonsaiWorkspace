#!/usr/bin/env node

const { spawn } = require('child_process');
const http = require('http');
const path = require('path');

const ROOT = path.resolve(__dirname, '..');
const LAUNCHER = path.join(ROOT, 'launch-all.mjs');
const VITE_URLS = [process.env.VITE_DEV_URL || 'http://127.0.0.1:1420/', process.env.VITE_DEV_URL_ALT || 'http://localhost:1420/'];
const API_HOST = process.env.VITE_API_HOST || process.env.BONSAI_API_HOST || '127.0.0.1';
const API_PORT = process.env.VITE_API_PORT || process.env.BONSAI_API_PORT || '11369';
const API_HEALTH_URL = `http://${API_HOST}:${API_PORT}/health`;
const TAURI_CONTEXT = Object.keys(process.env).some((k) => k.startsWith('TAURI_'));
const SKIP_BACKEND = process.env.BONSAI_DEV_ALL_NO_BACKEND === '1' || TAURI_CONTEXT;

function probe(url, timeout = 500) {
  return new Promise((resolve) => {
    try {
      const req = http.get(url, (res) => {
        res.resume();
        resolve(true);
      });
      req.on('error', () => resolve(false));
      req.setTimeout(timeout, () => {
        req.destroy();
        resolve(false);
      });
    } catch {
      resolve(false);
    }
  });
}

function startBackend() {
  const node = process.execPath;
  const args = [LAUNCHER, '--mode', 'desktop', '--no-vite-check'];
  const p = spawn(node, args, { stdio: 'inherit' });
  p.on('exit', (code) => {
    console.log(`[dev-all] backend exited with ${code}`);
    process.exit(code || 0);
  });
  return p;
}

function startVite() {
  let p;
  const viteStdio = ['ignore', 'inherit', 'inherit'];
  if (process.platform === 'win32') {
    p = spawn('cmd.exe', ['/d', '/s', '/c', 'npm run dev'], { stdio: viteStdio, cwd: ROOT });
  } else {
    p = spawn('npm', ['run', 'dev'], { stdio: viteStdio, cwd: ROOT });
  }
  p.on('exit', (code) => {
    console.log(`[dev-all] vite exited with ${code}`);
    // In Tauri beforeDevCommand mode, Vite is the primary child. If it exits,
    // fail fast so Tauri can surface the startup failure instead of hanging.
    if (SKIP_BACKEND) {
      process.exit(code || 1);
    }
    // In launcher mode, backend may continue serving API.
  });
  return p;
}

(async () => {
  console.log('[dev-all] probing existing dev server and API health...');
  let viteRunning = false;
  for (const url of VITE_URLS) {
    // try a slightly longer timeout for localhost/127 probes
    // stop at first successful probe
    // eslint-disable-next-line no-await-in-loop
    if (await probe(url, 1200)) {
      viteRunning = true;
      break;
    }
  }
  const apiRunning = SKIP_BACKEND ? false : await probe(API_HEALTH_URL, 700);
  if (viteRunning) console.log('[dev-all] detected existing Vite server; skipping start');
  if (SKIP_BACKEND) {
    console.log('[dev-all] tauri/beforeDevCommand context detected; skipping backend launcher');
  } else if (apiRunning) {
    console.log('[dev-all] detected existing Bonsai API; skipping backend start');
  }

  let backend;
  if (!SKIP_BACKEND && !apiRunning) {
    console.log('[dev-all] starting backend...');
    backend = startBackend();
  }

  let vite;
  if (!viteRunning) {
    console.log('[dev-all] starting vite dev server...');
    vite = startVite();
  }

  process.on('SIGINT', () => {
    console.log('[dev-all] SIGINT — shutting down children');
    try { if (backend) backend.kill('SIGINT'); } catch {};
    try { if (vite) vite.kill('SIGINT'); } catch {};
    process.exit(0);
  });

  // Keep this supervisor process alive; otherwise npm may exit early and tear
  // down child processes (notably Vite) in beforeDevCommand mode on Windows.
  await new Promise(() => {});
})();
