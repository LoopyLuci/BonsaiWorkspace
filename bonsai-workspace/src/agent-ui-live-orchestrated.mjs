#!/usr/bin/env node

import { spawn } from 'node:child_process';

const ROOT = process.cwd();
const SRC_DIR = ROOT;
const UI_BASE = process.env.BONSAI_UI_BASE || 'http://localhost:1420';
const npmCmd = process.platform === 'win32' ? 'npm' : 'npm';

function log(msg) {
  process.stdout.write(`${msg}\n`);
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function waitForUrl(url, timeoutMs = 45000) {
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    try {
      const resp = await fetch(url, { method: 'GET' });
      if (resp.ok) return true;
    } catch {
      // Retry until timeout.
    }
    await sleep(500);
  }
  return false;
}

function runCommand(commandLine, cwd, extraEnv = {}) {
  return new Promise((resolve, reject) => {
    log(`> ${commandLine} (cwd=${cwd})`);
    const child = spawn(commandLine, {
      cwd,
      stdio: 'inherit',
      shell: true,
      env: { ...process.env, ...extraEnv },
    });

    child.on('error', reject);
    child.on('exit', (code) => {
      if (code === 0) {
        resolve();
      } else {
        reject(new Error(`${commandLine} failed with code ${code}`));
      }
    });
  });
}

function startDevServer() {
  return spawn(`${npmCmd} run dev`, {
    cwd: SRC_DIR,
    stdio: 'inherit',
    shell: true,
    env: { ...process.env },
  });
}

async function main() {
  let devServer = null;
  let startedDevServer = false;

  try {
    const alreadyReady = await waitForUrl(UI_BASE, 1500);
    if (alreadyReady) {
      log(`Reusing existing dev server at ${UI_BASE}`);
    } else {
      log('Starting frontend dev server...');
      devServer = startDevServer();
      startedDevServer = true;

      const ready = await waitForUrl(UI_BASE, 60000);
      if (!ready) {
        throw new Error(`Dev server did not become ready at ${UI_BASE} within timeout`);
      }
      log(`Dev server ready at ${UI_BASE}`);
    }

    log('Launching visible real-time HITL demo...');
    await runCommand('node ./agent-api-smoke.mjs', SRC_DIR, {
      BONSAI_SKIP_API: '1',
      BONSAI_SKIP_UI: '0',
      BONSAI_UI_LIVE: '1',
      BONSAI_UI_HEADLESS: '0',
      BONSAI_UI_KEEP_OPEN_MS: process.env.BONSAI_UI_KEEP_OPEN_MS || '120000',
      BONSAI_UI_SLOW_MO_MS: process.env.BONSAI_UI_SLOW_MO_MS || '120',
    });

    log('Live demo finished successfully.');
  } finally {
    if (startedDevServer && devServer && !devServer.killed) {
      devServer.kill();
    }
  }
}

main().catch((err) => {
  log(`FATAL ${String(err)}`);
  process.exitCode = 1;
});
