#!/usr/bin/env node

const { spawn } = require('node:child_process');
const http = require('node:http');
const path = require('node:path');

const ROOT = path.resolve(__dirname, '..');
const LAUNCHER = path.join(ROOT, 'orchestrate-workspace-ecosystem.mjs');
const VITE_URL = process.env.VITE_DEV_URL || 'http://127.0.0.1:1420/';
const API_HOST = process.env.VITE_API_HOST || process.env.WORKSPACE_API_HOST || '127.0.0.1';
const API_PORT = process.env.VITE_API_PORT || process.env.WORKSPACE_API_PORT || '11369';
const API_HEALTH_URL = `http://${API_HOST}:${API_PORT}/health`;

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
  const args = [LAUNCHER, '--mode', 'desktop'];
  const p = spawn(node, args, { stdio: 'inherit' });
  p.on('exit', (code) => {
    console.log(`[dev-all] backend exited with ${code}`);
    process.exit(code || 0);
  });
  return p;
}

function startVite() {
  const p = spawn('npm', ['run', 'dev'], { stdio: 'inherit', cwd: ROOT });
  p.on('exit', (code) => {
    console.log(`[dev-all] vite exited with ${code}`);
    // let backend continue
  });
  return p;
}

(async () => {
  console.log('[dev-all] probing existing dev server and API health...');
  const viteRunning = await probe(VITE_URL, 700);
  const apiRunning = await probe(API_HEALTH_URL, 700);
  if (viteRunning) console.log('[dev-all] detected existing Vite server; skipping start');
  if (apiRunning) console.log('[dev-all] detected existing Workspace API; skipping backend start');

  let backend;
  if (!apiRunning) {
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
})();
