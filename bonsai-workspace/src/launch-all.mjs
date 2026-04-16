#!/usr/bin/env node

import { spawn, spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import net from 'node:net';

const SRC_DIR = process.cwd();
const ROOT_DIR = path.resolve(SRC_DIR, '..', '..');
const TAURI_DIR = path.join(ROOT_DIR, 'bonsai-workspace', 'src-tauri');
const LAUNCHER_ARTIFACT_DIR = path.join(ROOT_DIR, 'tool_test', 'launcher');
const LAUNCHER_ARTIFACT_FILE = path.join(LAUNCHER_ARTIFACT_DIR, 'latest.json');
const DEFAULT_API_PORT = 11369;

function readConfiguredApiPort() {
  try {
    const appData = process.env.APPDATA || '';
    if (!appData) return DEFAULT_API_PORT;
    const cfgPath = path.join(appData, 'com.bonsai.workspace', 'bonsai-config.json');
    if (!fs.existsSync(cfgPath)) return DEFAULT_API_PORT;
    const raw = fs.readFileSync(cfgPath, 'utf8');
    const cfg = JSON.parse(raw);
    const p = Number(cfg?.api_port);
    if (Number.isFinite(p) && p > 0 && p <= 65535) {
      return p;
    }
  } catch {
    // fall back to default
  }
  return DEFAULT_API_PORT;
}

const DEFAULTS = {
  mode: 'desktop',
  strictApp: false,
  noTests: false,
  preflightOnly: false,
  healthTimeoutMs: 180000,
  wifiPort: 5555,
  allowPortInUse: false,
  noInstall: false,
  attachExisting: true,
  reportPath: LAUNCHER_ARTIFACT_FILE,
  apiPort: readConfiguredApiPort(),
};

function log(msg) {
  process.stdout.write(`${msg}\n`);
}

function toolCmd(name) {
  if (process.platform === 'win32' && (name === 'npm' || name === 'npx')) {
    return `${name}.cmd`;
  }
  return name;
}

function parseArgs(argv) {
  const cfg = { ...DEFAULTS, raw: argv.slice() };
  for (let i = 0; i < argv.length; i += 1) {
    const a = argv[i];
    if (a === '--mode') cfg.mode = String(argv[++i] || cfg.mode);
    else if (a === '--strict-app') cfg.strictApp = true;
    else if (a === '--no-tests') cfg.noTests = true;
    else if (a === '--preflight-only') cfg.preflightOnly = true;
    else if (a === '--health-timeout-ms') cfg.healthTimeoutMs = Number(argv[++i] || cfg.healthTimeoutMs);
    else if (a === '--apk-path') cfg.apkPath = String(argv[++i] || '').trim();
    else if (a === '--serial') cfg.serial = String(argv[++i] || '').trim();
    else if (a === '--wifi-host') cfg.wifiHost = String(argv[++i] || '').trim();
    else if (a === '--wifi-port') cfg.wifiPort = Number(argv[++i] || cfg.wifiPort);
    else if (a === '--allow-port-in-use') cfg.allowPortInUse = true;
    else if (a === '--no-install') cfg.noInstall = true;
    else if (a === '--no-attach-existing') cfg.attachExisting = false;
    else if (a === '--report-path') cfg.reportPath = String(argv[++i] || '').trim() || cfg.reportPath;
    else if (a === '--api-port') cfg.apiPort = Number(argv[++i] || cfg.apiPort);
    else if (a === '--help' || a === '-h') cfg.help = true;
  }
  return cfg;
}

function printHelp() {
  log('Bonsai all-in-one launcher');
  log('');
  log('Usage: node ./launch-all.mjs [options]');
  log('');
  log('Options:');
  log('  --mode desktop|desktop+usb   Launch mode (default: desktop)');
  log('  --strict-app                 Require package/install/launch success for USB checks');
  log('  --no-tests                   Skip USB regression test phase');
  log('  --preflight-only             Run checks only; do not launch Tauri');
  log('  --health-timeout-ms <ms>     API readiness timeout (default: 180000)');
  log('  --apk-path <path>            APK path for strict USB launch/regression');
  log('  --serial <adb-serial>        Explicit Android device serial');
  log('  --wifi-host <ip>             Optional WiFi debug host');
  log('  --wifi-port <port>           WiFi debug port (default: 5555)');
  log('  --allow-port-in-use          Continue even if API port 11369 is occupied');
  log('  --no-attach-existing         Do not attach to existing API runtime when port is occupied');
  log('  --report-path <path>         Write launcher phase report JSON to this path');
  log('  --api-port <port>            API port to check/await (default: from app config or 11369)');
  log('  --no-install                 Skip npm install check/fix');
}

function nowIso() {
  return new Date().toISOString();
}

function beginPhase(report, name, details = {}) {
  const phase = {
    name,
    started_at: nowIso(),
    completed_at: null,
    duration_ms: 0,
    ok: false,
    details,
    error: null,
  };
  report.phases.push(phase);
  return phase;
}

function endPhase(phase, ok, details = {}, error = null) {
  phase.completed_at = nowIso();
  phase.duration_ms = Math.max(0, Date.parse(phase.completed_at) - Date.parse(phase.started_at));
  phase.ok = ok;
  phase.details = { ...phase.details, ...details };
  phase.error = error ? String(error) : null;
}

function writeReport(reportPath, report) {
  const outPath = path.isAbsolute(reportPath)
    ? reportPath
    : path.resolve(SRC_DIR, reportPath);
  fs.mkdirSync(path.dirname(outPath), { recursive: true });
  fs.writeFileSync(outPath, JSON.stringify(report, null, 2), 'utf8');
  return outPath;
}

function runVersionCheck(command, args = ['--version']) {
  let out;
  if (command === 'npm' && process.env.npm_execpath && fs.existsSync(process.env.npm_execpath)) {
    out = spawnSync(process.execPath, [process.env.npm_execpath, ...args], { encoding: 'utf8' });
  } else {
    out = spawnSync(toolCmd(command), args, { encoding: 'utf8' });
  }

  if (out.error) {
    throw new Error(`${command} check failed: ${out.error.message}`);
  }
  if (out.status !== 0) {
    throw new Error(`${command} check failed: ${(out.stderr || out.stdout || '').trim()}`);
  }
  return (out.stdout || out.stderr || '').trim();
}

function isPortActivelyListening(port) {
  try {
    const out = spawnSync(toolCmd('netstat'), ['-ano'], { encoding: 'utf8' });
    const dump = `${out.stdout || ''}\n${out.stderr || ''}`;
    return dump
      .split(/\r?\n/)
      .some((line) => line.includes(`:${port}`) && /LISTEN|LISTENING/i.test(line));
  } catch {
    return true;
  }
}

function ensurePathExists(absPath, label) {
  if (!fs.existsSync(absPath)) {
    throw new Error(`${label} not found: ${absPath}`);
  }
}

function ensureFrontendDeps(cfg) {
  const nodeModules = path.join(SRC_DIR, 'node_modules');
  if (fs.existsSync(nodeModules) || cfg.noInstall) {
    return;
  }
  log('[preflight] node_modules missing; running npm install...');
  const out = spawnSync(toolCmd('npm'), ['install'], { cwd: SRC_DIR, stdio: 'inherit' });
  if (out.status !== 0) {
    throw new Error('npm install failed');
  }
}

function checkPortAvailable(port) {
  return new Promise((resolve) => {
    const server = net.createServer();
    server.once('error', () => resolve(false));
    server.once('listening', () => {
      server.close(() => resolve(true));
    });
    server.listen(port, '127.0.0.1');
  });
}

async function waitForPortToBecomeAvailable(port, timeoutMs) {
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    if (await checkPortAvailable(port)) {
      return true;
    }
    await new Promise((r) => setTimeout(r, 500));
  }
  return false;
}

async function waitForApiHealth(timeoutMs, apiPort) {
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    try {
      const resp = await fetch(`http://127.0.0.1:${apiPort}/health`);
      if (resp.ok) return true;
    } catch {
      // retry
    }
    await new Promise((r) => setTimeout(r, 1000));
  }
  return false;
}

async function checkApiHealthyOnce(apiPort) {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), 1200);
  try {
    const resp = await fetch(`http://127.0.0.1:${apiPort}/health`, { signal: controller.signal });
    return resp.ok;
  } catch {
    return false;
  } finally {
    clearTimeout(timer);
  }
}

function spawnTauriDev() {
  return spawn('cargo', ['tauri', 'dev'], {
    cwd: TAURI_DIR,
    stdio: 'inherit',
    env: { ...process.env },
  });
}

function runUsbRegression(cfg) {
  const env = { ...process.env };
  if (cfg.strictApp) env.BONSAI_REQUIRE_APP = '1';
  if (cfg.apkPath) env.ANDROID_APK_PATH = cfg.apkPath;
  if (cfg.serial) env.ANDROID_SERIAL = cfg.serial;
  if (cfg.wifiHost) env.ANDROID_WIFI_HOST = cfg.wifiHost;
  if (cfg.wifiPort) env.ANDROID_WIFI_PORT = String(cfg.wifiPort);

  log('[usb] running android USB regression...');
  const regression = spawnSync('node', ['./android-usb-regression.mjs'], {
    cwd: SRC_DIR,
    env,
    stdio: 'inherit',
  });
  if (regression.status !== 0) {
    return { ok: false };
  }

  log('[usb] appending evidence ledger...');
  const evidence = spawnSync(toolCmd('pwsh'), ['-File', './append-usb-evidence-ledger.ps1'], {
    cwd: SRC_DIR,
    env,
    stdio: 'inherit',
  });
  return { ok: evidence.status === 0 };
}

async function run() {
  const cfg = parseArgs(process.argv.slice(2));
  if (cfg.help) {
    printHelp();
    return;
  }

  if (!['desktop', 'desktop+usb'].includes(cfg.mode)) {
    throw new Error(`Unsupported mode: ${cfg.mode}`);
  }

  const report = {
    schema_version: 1,
    ts: nowIso(),
    finished_at: null,
    ok: false,
    mode: cfg.mode,
    preflight_only: cfg.preflightOnly,
    strict_app: cfg.strictApp,
    attached_existing_runtime: false,
    spawned_tauri: false,
    api_healthy: false,
    usb_validation_ran: false,
    usb_validation_ok: null,
    api_port: cfg.apiPort,
    report_path: cfg.reportPath,
    phases: [],
    error: null,
  };

  try {
    const preflight = beginPhase(report, 'preflight');
    log('[launcher] preflight checks starting...');
    ensurePathExists(path.join(SRC_DIR, 'package.json'), 'Frontend package.json');
    ensurePathExists(path.join(TAURI_DIR, 'Cargo.toml'), 'Tauri Cargo.toml');
    ensurePathExists(path.join(TAURI_DIR, 'tauri.conf.json'), 'Tauri config');

    const versions = {
      node: runVersionCheck('node'),
      npm: runVersionCheck('npm'),
      cargo: runVersionCheck('cargo'),
      tauri: runVersionCheck('cargo', ['tauri', '--version']),
    };
    log(`[preflight] node: ${versions.node}`);
    log(`[preflight] npm: ${versions.npm}`);
    log(`[preflight] cargo: ${versions.cargo}`);
    log(`[preflight] tauri: ${versions.tauri}`);

    ensureFrontendDeps(cfg);

    let apiFree = await checkPortAvailable(cfg.apiPort);
    report.api_healthy = await checkApiHealthyOnce(cfg.apiPort);

    if (!apiFree && !report.api_healthy && !isPortActivelyListening(cfg.apiPort)) {
      log(`[preflight] detected stale non-listening socket state on port ${cfg.apiPort}; treating port as available.`);
      apiFree = true;
    }

    // In desktop mode, transient listeners can briefly hold the port while exiting.
    if (!apiFree && !report.api_healthy && !cfg.allowPortInUse && !cfg.preflightOnly) {
      log(`[preflight] port ${cfg.apiPort} is occupied by a non-healthy listener; waiting briefly for release...`);
      const released = await waitForPortToBecomeAvailable(cfg.apiPort, 8000);
      if (released) {
        apiFree = true;
        log(`[preflight] port ${cfg.apiPort} became available; continuing launch.`);
      }
    }

    if (!apiFree && !cfg.allowPortInUse && !cfg.preflightOnly && !(cfg.attachExisting && report.api_healthy)) {
      throw new Error(`Port ${cfg.apiPort} is already in use. Close existing Bonsai/API process, use --allow-port-in-use, or keep attach-to-existing enabled.`);
    }
    if (!apiFree && cfg.preflightOnly) {
      log(`[preflight] port ${cfg.apiPort} already in use; reporting warning only in preflight mode.`);
    } else if (!apiFree && cfg.allowPortInUse) {
      log(`[preflight] port ${cfg.apiPort} already in use; continuing because --allow-port-in-use was provided.`);
    } else if (!apiFree && cfg.attachExisting && report.api_healthy) {
      log(`[preflight] port ${cfg.apiPort} occupied by healthy API; launcher will attach to existing runtime.`);
      report.attached_existing_runtime = true;
    }

    endPhase(preflight, true, { versions, api_port_free: apiFree, api_healthy: report.api_healthy });

    if (cfg.preflightOnly) {
      report.ok = true;
      report.finished_at = nowIso();
      const outPath = writeReport(cfg.reportPath, report);
      log(`[launcher] preflight completed successfully. Report: ${outPath}`);
      return;
    }

    let tauri = null;
    let shuttingDown = false;

    if (!report.attached_existing_runtime) {
      const launchPhase = beginPhase(report, 'spawn_tauri');
      log('[launcher] starting cargo tauri dev...');
      tauri = spawnTauriDev();
      report.spawned_tauri = true;
      endPhase(launchPhase, true);
    }

    const shutdown = () => {
      if (shuttingDown) return;
      shuttingDown = true;
      if (!tauri) return;
      try {
        tauri.kill('SIGINT');
      } catch {
        // ignore
      }
    };

    process.on('SIGINT', shutdown);
    process.on('SIGTERM', shutdown);

    const healthPhase = beginPhase(report, 'wait_for_api_health', { timeout_ms: cfg.healthTimeoutMs });
    const healthy = report.api_healthy || (await waitForApiHealth(cfg.healthTimeoutMs, cfg.apiPort));
    if (!healthy) {
      endPhase(healthPhase, false, {}, `API did not become healthy within ${cfg.healthTimeoutMs}ms.`);
      shutdown();
      throw new Error(`API did not become healthy within ${cfg.healthTimeoutMs}ms.`);
    }
    report.api_healthy = true;
    endPhase(healthPhase, true);

    log(`[launcher] API is healthy on http://127.0.0.1:${cfg.apiPort}/health`);

    if (cfg.mode === 'desktop+usb' && !cfg.noTests) {
      const usbPhase = beginPhase(report, 'usb_validation');
      report.usb_validation_ran = true;
      const usb = runUsbRegression(cfg);
      report.usb_validation_ok = usb.ok;
      if (!usb.ok) {
        endPhase(usbPhase, false, {}, 'USB regression failed');
        log('[launcher] USB regression failed. Tauri app is still running for investigation.');
        process.exitCode = 1;
      } else {
        endPhase(usbPhase, true);
        log('[launcher] USB regression and evidence append completed successfully.');
      }
    }

    report.ok = process.exitCode !== 1;
    report.finished_at = nowIso();
    const outPath = writeReport(cfg.reportPath, report);
    log(`[launcher] report written to: ${outPath}`);

    if (report.attached_existing_runtime) {
      log('[launcher] launch sequence complete (attached to existing runtime).');
      return;
    }

    log('[launcher] launch sequence complete. Press Ctrl+C to stop.');

    await new Promise((resolve) => {
      tauri.on('exit', (code) => {
        if (typeof code === 'number' && code !== 0 && process.exitCode !== 1) {
          process.exitCode = code;
        }
        resolve();
      });
    });
  } catch (err) {
    report.ok = false;
    report.error = err.message;
    report.finished_at = nowIso();
    const outPath = writeReport(cfg.reportPath, report);
    process.stderr.write(`[launcher] ERROR: ${err.message}\n`);
    process.stderr.write(`[launcher] report written to: ${outPath}\n`);
    process.exitCode = 1;
  }
}

run();
