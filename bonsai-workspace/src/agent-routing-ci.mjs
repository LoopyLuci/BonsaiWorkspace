#!/usr/bin/env node

import { spawn } from 'node:child_process';

const ROOT = process.cwd();
const SRC_DIR = ROOT;
const TAURI_DIR = `${ROOT}/../src-tauri`;
const UI_BASE = process.env.BONSAI_UI_BASE || 'http://localhost:1420';
const npmCmd = process.platform === 'win32' ? 'npm' : 'npm';
const cargoCmd = process.platform === 'win32' ? 'cargo' : 'cargo';
const CARGO_TARGET_DIR = `${TAURI_DIR}/target-ci-local`;

const RUST_TEST_MATRIX = [
  'file_inventory_requests_detected',
  'non_inventory_requests_ignored',
  'tool_name_from_action_defaults_when_missing',
  'resume_payloads_exclude_system_and_append_tool_result',
  'parse_tool_calls_accepts_normalized_tag_variants',
  'parse_tool_calls_counts_malformed_empty_and_non_json_payloads',
  'parse_tool_calls_handles_fenced_payload',
];

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
      // keep retrying until timeout
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

function rustDeterministicEnv() {
  return {
    CARGO_TARGET_DIR,
    CARGO_BUILD_JOBS: '1',
    CARGO_INCREMENTAL: '1',
    // Reduce memory pressure and improve repeatability on this Windows machine.
    RUSTFLAGS: process.env.RUSTFLAGS || '-Cdebuginfo=0',
  };
}

async function warmRustCache() {
  await runCommand(
    `${cargoCmd} test --manifest-path "${TAURI_DIR}/Cargo.toml" --profile ci-local --no-run`,
    TAURI_DIR,
    rustDeterministicEnv(),
  );
}

async function runRustMatrix() {
  const results = [];
  for (const filter of RUST_TEST_MATRIX) {
    const startedAt = Date.now();
    try {
      await runCommand(
        `${cargoCmd} test ${filter} --manifest-path "${TAURI_DIR}/Cargo.toml" --profile ci-local -- --test-threads=1`,
        TAURI_DIR,
        rustDeterministicEnv(),
      );
      results.push({ filter, ok: true, ms: Date.now() - startedAt });
    } catch (err) {
      results.push({ filter, ok: false, ms: Date.now() - startedAt, err: String(err) });
      break;
    }
  }

  log('Rust test matrix summary:');
  for (const item of results) {
    const sec = (item.ms / 1000).toFixed(1);
    log(` - ${item.ok ? 'PASS' : 'FAIL'} ${item.filter} (${sec}s)`);
  }

  const failed = results.find((r) => !r.ok);
  if (failed) {
    throw new Error(`Rust test matrix failed at ${failed.filter}: ${failed.err || 'unknown error'}`);
  }
}

function startDevServer() {
  const child = spawn(`${npmCmd} run dev`, {
    cwd: SRC_DIR,
    stdio: 'inherit',
    shell: true,
    env: { ...process.env },
  });
  return child;
}

async function main() {
  let devServer = null;
  let startedDevServer = false;
  try {
    log('Warming deterministic Rust test cache/profile...');
    await warmRustCache();

    log('Running deterministic Rust per-test matrix...');
    await runRustMatrix();

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

    await runCommand('node ./agent-api-smoke.mjs', SRC_DIR, {
      BONSAI_SKIP_API: '1',
      BONSAI_SKIP_UI: '0',
    });

    log('All deterministic routing regressions passed.');
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
