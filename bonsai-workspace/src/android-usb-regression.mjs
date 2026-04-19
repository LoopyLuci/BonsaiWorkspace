#!/usr/bin/env node

import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';

const ROOT = path.resolve(process.cwd(), '..', '..');
const ART_DIR = path.join(ROOT, 'tool_test', 'android-usb-regression');
const ART_FILE = path.join(ART_DIR, 'latest.json');

// Schema version for artifact backward-compatibility tracking.
const SCHEMA_VERSION = 2;

function resolveAdb() {
  const candidates = [];
  if (process.env.LOCALAPPDATA) {
    candidates.push(path.join(process.env.LOCALAPPDATA, 'Android', 'Sdk', 'platform-tools', 'adb.exe'));
  }
  if (process.env.ANDROID_HOME) {
    candidates.push(path.join(process.env.ANDROID_HOME, 'platform-tools', process.platform === 'win32' ? 'adb.exe' : 'adb'));
  }
  if (process.env.ANDROID_SDK_ROOT) {
    candidates.push(path.join(process.env.ANDROID_SDK_ROOT, 'platform-tools', process.platform === 'win32' ? 'adb.exe' : 'adb'));
  }
  for (const c of candidates) {
    if (fs.existsSync(c)) return { adb: c, candidates };
  }
  return { adb: 'adb', candidates };
}

function run(adb, args) {
  const t0 = Date.now();
  const out = spawnSync(adb, args, { encoding: 'utf8' });
  return {
    ok: out.status === 0,
    status: out.status ?? -1,
    stdout: (out.stdout || '').trim(),
    stderr: (out.stderr || '').trim(),
    duration_ms: Date.now() - t0,
    args,
  };
}

function sleep(ms) {
  const clamped = Math.max(0, Number(ms) || 0);
  if (clamped === 0) return;
  const shared = new Int32Array(new SharedArrayBuffer(4));
  Atomics.wait(shared, 0, 0, clamped);
}

function parseDevices(output) {
  const lines = output.split(/\r?\n/).slice(1).map((x) => x.trim()).filter(Boolean);
  const devices = [];
  for (const line of lines) {
    const parts = line.split(/\s+/);
    if (parts.length < 2) continue;
    devices.push({ serial: parts[0], state: parts[1], raw: line });
  }
  return devices;
}

/** Resolve APK from ANDROID_APK_PATH or known build output candidates. */
function resolveApk(explicitPath) {
  if (explicitPath) {
    if (fs.existsSync(explicitPath)) return explicitPath;
    throw new Error(`ANDROID_APK_PATH set but file not found: ${explicitPath}`);
  }
  const candidates = [
    path.join(ROOT, 'bonsai-workspace/src-tauri/gen/android/app/build/outputs/apk/universal/release/app-universal-release-unsigned.apk'),
    path.join(ROOT, 'bonsai-workspace/src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk'),
    path.join(ROOT, 'bonsai-workspace/src-tauri/gen/android/app/build/outputs/apk/arm64-v8a/release/app-arm64-v8a-release-unsigned.apk'),
    path.join(ROOT, 'bonsai-workspace/src-tauri/gen/android/app/build/outputs/apk/arm64-v8a/debug/app-arm64-v8a-debug.apk'),
  ];
  for (const c of candidates) {
    if (fs.existsSync(c)) return c;
  }
  return null; // No APK found — only fatal if requireApp is set.
}

function main() {
  const apiPort = Number(process.env.BONSAI_API_PORT || '11369');
  const wifiHost = (process.env.ANDROID_WIFI_HOST || '').trim();
  const wifiPort = Number(process.env.ANDROID_WIFI_PORT || '5555');
  const packageName = (process.env.ANDROID_PACKAGE || 'com.bonsai.workspace').trim();
  const activity = (process.env.ANDROID_ACTIVITY || '').trim();
  const explicitSerial = (process.env.ANDROID_SERIAL || '').trim();
  const requireApp = process.env.BONSAI_REQUIRE_APP === '1';
  const processVerifyAttempts = Math.max(1, Number(process.env.ANDROID_PROCESS_VERIFY_ATTEMPTS || '5'));
  const processVerifyDelayMs = Math.max(0, Number(process.env.ANDROID_PROCESS_VERIFY_DELAY_MS || '900'));
  const enableBootstrap = process.env.ANDROID_ENABLE_BOOTSTRAP === '1';
  const explicitApkPath = (process.env.ANDROID_APK_PATH || '').trim() || null;

  const { adb, candidates } = resolveAdb();
  const steps = [];

  // ── Device discovery ──────────────────────────────────────────────────────
  const devicesRes = run(adb, ['devices', '-l']);
  steps.push({ label: 'adb devices -l', ...devicesRes });
  if (!devicesRes.ok) {
    throw new Error(`adb devices failed: ${devicesRes.stderr || devicesRes.stdout}`);
  }

  const devices = parseDevices(devicesRes.stdout);
  const serial = explicitSerial || (devices.find((d) => d.state === 'device')?.serial || '');
  if (!serial) {
    throw new Error('No connected Android device found. Set ANDROID_SERIAL or connect a USB-debug device.');
  }

  steps.push({ label: 'selected-serial', ok: true, serial });

  const model = run(adb, ['-s', serial, 'shell', 'getprop', 'ro.product.model']);
  steps.push({ label: 'adb shell getprop ro.product.model', ...model });

  // ── Bootstrap (reverse mapping) ───────────────────────────────────────────
  const reverse = run(adb, ['-s', serial, 'reverse', `tcp:${apiPort}`, `tcp:${apiPort}`]);
  steps.push({ label: 'adb reverse', ...reverse });

  const reverseList = run(adb, ['-s', serial, 'reverse', '--list']);
  const reverseVerified = reverseList.ok && reverseList.stdout.includes(`tcp:${apiPort}`);
  steps.push({
    label: 'adb reverse --list',
    ...reverseList,
    ok: reverseVerified,
    hint: reverseVerified ? null : `tcp:${apiPort} not found in reverse list`,
  });

  // ── APK install (if path provided or auto-resolved) ───────────────────────
  let resolvedApkPath = null;
  try {
    resolvedApkPath = resolveApk(explicitApkPath);
  } catch (err) {
    steps.push({ label: 'resolve apk', ok: false, stdout: '', stderr: err.message, duration_ms: 0, fatal: true });
    if (requireApp) throw err;
  }

  if (resolvedApkPath) {
    const install = run(adb, ['-s', serial, 'install', '-r', resolvedApkPath]);
    steps.push({ label: 'adb install -r', ...install, apk_path: resolvedApkPath });
    if (!install.ok && requireApp) {
      steps.push({
        label: 'install gate',
        ok: false, stdout: '', stderr: 'APK install failed in strict mode.',
        duration_ms: 0, fatal: true,
      });
    }
  }

  // ── Package check ─────────────────────────────────────────────────────────
  const packageCheck = run(adb, ['-s', serial, 'shell', 'pm', 'path', packageName]);
  const packageInstalled = packageCheck.ok && /package:/.test(packageCheck.stdout);
  steps.push({
    label: `check package ${packageName}`,
    ...packageCheck,
    ok: packageInstalled || !requireApp,
    installed: packageInstalled,
    hint: packageInstalled
      ? 'Package is installed.'
      : requireApp
        ? 'Package not found — strict mode enabled.'
        : 'Package not installed; continuing (BONSAI_REQUIRE_APP not set).',
  });

  // ── App launch ────────────────────────────────────────────────────────────
  if (packageInstalled || requireApp) {
    let launch;
    if (activity) {
      launch = run(adb, ['-s', serial, 'shell', 'am', 'start', '-n', `${packageName}/${activity}`]);
    } else {
      launch = run(adb, ['-s', serial, 'shell', 'monkey', '-p', packageName, '-c', 'android.intent.category.LAUNCHER', '1']);
    }
    steps.push({ label: 'launch app', ...launch });

    // Verify process started.
    let pidRes = run(adb, ['-s', serial, 'shell', 'pidof', packageName]);
    let pidOk = pidRes.ok && pidRes.stdout.trim().length > 0;
    let pidAttempt = 1;
    while (!pidOk && pidAttempt < processVerifyAttempts) {
      sleep(processVerifyDelayMs);
      pidAttempt += 1;
      pidRes = run(adb, ['-s', serial, 'shell', 'pidof', packageName]);
      pidOk = pidRes.ok && pidRes.stdout.trim().length > 0;
    }
    steps.push({
      label: 'verify process running',
      ...pidRes,
      ok: pidOk || !requireApp,
      attempts: pidAttempt,
      retry_delay_ms: processVerifyDelayMs,
      hint: pidOk
        ? `pid: ${pidRes.stdout.trim()}`
        : `Process not found after launch (attempts=${pidAttempt}/${processVerifyAttempts}).`,
    });
  } else {
    steps.push({
      label: 'launch app',
      ok: true,
      skipped: true,
      stdout: '', stderr: '', duration_ms: 0,
      hint: `Package ${packageName} is not installed; set BONSAI_REQUIRE_APP=1 to make this mandatory.`,
    });
  }

  // ── WiFi bridge (optional) ────────────────────────────────────────────────
  if (wifiHost || enableBootstrap) {
    const bridgeHost = wifiHost;
    if (bridgeHost) {
      const tcpip = run(adb, ['-s', serial, 'tcpip', String(wifiPort)]);
      steps.push({ label: 'adb tcpip', ...tcpip });

      const connect = run(adb, ['connect', `${bridgeHost}:${wifiPort}`]);
      steps.push({ label: 'adb connect', ...connect });
    }
  }

  // ── Verdict ───────────────────────────────────────────────────────────────
  const ok = steps.every((s) => s.ok !== false);

  const record = {
    schema_version: SCHEMA_VERSION,
    ts: new Date().toISOString(),
    ok,
    adb,
    adbCandidates: candidates,
    serial,
    apiPort,
    wifiHost: wifiHost || null,
    wifiPort,
    packageName,
    activity: activity || null,
    strict_require_app: requireApp,
    resolved_apk_path: resolvedApkPath,
    steps,
  };

  fs.mkdirSync(ART_DIR, { recursive: true });
  fs.writeFileSync(ART_FILE, JSON.stringify(record, null, 2), 'utf8');

  console.log(`USB_REGRESSION_OK=${ok ? '1' : '0'}`);
  console.log(`USB_REGRESSION_ARTIFACT=${ART_FILE}`);
  console.log(`USB_REGRESSION_SERIAL=${serial}`);
  console.log(`USB_REGRESSION_STRICT=${requireApp ? '1' : '0'}`);
  if (!ok) process.exitCode = 1;
}

main();
