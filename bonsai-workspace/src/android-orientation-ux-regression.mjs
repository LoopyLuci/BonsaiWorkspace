#!/usr/bin/env node

import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';

const ROOT = path.resolve(process.cwd(), '..', '..');
const ART_DIR = path.join(ROOT, 'Screenshots', 'android-orientation-ux');
const DEVICE_ID = (process.env.ANDROID_SERIAL || '').trim();
const PACKAGE_NAME = (process.env.ANDROID_PACKAGE || 'com.bonsai.workspace').trim();
const ACTIVITY = (process.env.ANDROID_ACTIVITY || '').trim();
const PAUSE_MS = Math.max(300, Number(process.env.ANDROID_ORIENTATION_PAUSE_MS || '2200'));
const ALLOW_LOCKED = process.env.ANDROID_ORIENTATION_ALLOW_LOCKED === '1';

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
    if (fs.existsSync(c)) return c;
  }
  return 'adb';
}

function run(adb, args, options = {}) {
  const t0 = Date.now();
  const out = spawnSync(adb, args, {
    encoding: options.binary ? null : 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  const stdout = options.binary ? out.stdout : String(out.stdout || '').trim();
  const stderr = String(out.stderr || '').trim();
  return {
    ok: out.status === 0,
    status: out.status ?? -1,
    stdout,
    stderr,
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
  const lines = String(output || '')
    .split(/\r?\n/)
    .slice(1)
    .map((x) => x.trim())
    .filter(Boolean);
  const devices = [];
  for (const line of lines) {
    const parts = line.split(/\s+/);
    if (parts.length < 2) continue;
    devices.push({ serial: parts[0], state: parts[1], raw: line });
  }
  return devices;
}

function adbArgs(serial, args) {
  return serial ? ['-s', serial, ...args] : args;
}

function launchApp(adb, serial, steps) {
  let launch;
  if (ACTIVITY) {
    const activityName = ACTIVITY.startsWith('.') ? `${PACKAGE_NAME}/${ACTIVITY}` : ACTIVITY;
    launch = run(adb, adbArgs(serial, ['shell', 'am', 'start', '-n', activityName]));
    steps.push({ label: 'launch app via am start', ...launch });
    return launch.ok;
  }

  launch = run(adb, adbArgs(serial, ['shell', 'monkey', '-p', PACKAGE_NAME, '-c', 'android.intent.category.LAUNCHER', '1']));
  steps.push({ label: 'launch app via monkey', ...launch });
  return launch.ok;
}

function screenshot(adb, serial, outPath, steps, label) {
  const shot = run(adb, adbArgs(serial, ['exec-out', 'screencap', '-p']), { binary: true });
  if (!shot.ok) {
    steps.push({ label, ...shot, hint: 'screencap failed' });
    return false;
  }
  fs.writeFileSync(outPath, shot.stdout);
  steps.push({
    label,
    ok: true,
    status: 0,
    duration_ms: shot.duration_ms,
    args: shot.args,
    file: outPath,
    bytes: fs.statSync(outPath).size,
  });
  return true;
}

function readSurfaceOrientation(adb, serial, steps, label) {
  const out = run(adb, adbArgs(serial, ['shell', 'dumpsys', 'input']));
  if (!out.ok) {
    steps.push({ label, ...out, hint: 'Unable to read SurfaceOrientation from dumpsys input' });
    return null;
  }

  const text = String(out.stdout || '');
  const m = text.match(/SurfaceOrientation:\s*(\d+)/);
  const value = m ? Number(m[1]) : null;
  steps.push({
    label,
    ok: value !== null,
    status: out.status,
    stdout: value === null ? out.stdout : `SurfaceOrientation: ${value}`,
    stderr: out.stderr,
    duration_ms: out.duration_ms,
    args: out.args,
  });
  return value;
}

function main() {
  fs.mkdirSync(ART_DIR, { recursive: true });
  const runId = new Date().toISOString().replace(/[:.]/g, '-');
  const runDir = path.join(ART_DIR, runId);
  fs.mkdirSync(runDir, { recursive: true });

  const adb = resolveAdb();
  const steps = [];

  const devicesRes = run(adb, ['devices', '-l']);
  steps.push({ label: 'adb devices -l', ...devicesRes });
  if (!devicesRes.ok) {
    throw new Error(`adb devices failed: ${devicesRes.stderr || devicesRes.stdout}`);
  }

  const devices = parseDevices(devicesRes.stdout);
  const serial = DEVICE_ID || (devices.find((d) => d.state === 'device')?.serial || '');
  if (!serial) {
    throw new Error('No connected Android device found. Set ANDROID_SERIAL or connect a USB debug device.');
  }

  steps.push({ label: 'selected serial', ok: true, serial });

  const model = run(adb, adbArgs(serial, ['shell', 'getprop', 'ro.product.model']));
  steps.push({ label: 'device model', ...model });

  const size = run(adb, adbArgs(serial, ['shell', 'wm', 'size']));
  steps.push({ label: 'wm size', ...size });

  const rotationOff = run(adb, adbArgs(serial, ['shell', 'settings', 'put', 'system', 'accelerometer_rotation', '0']));
  steps.push({ label: 'disable auto-rotate', ...rotationOff });

  const launchOk = launchApp(adb, serial, steps);
  if (!launchOk) {
    throw new Error('Failed to launch app for orientation capture.');
  }

  const portraitRotate = run(adb, adbArgs(serial, ['shell', 'settings', 'put', 'system', 'user_rotation', '0']));
  steps.push({ label: 'set portrait rotation', ...portraitRotate });
  sleep(PAUSE_MS);
  const portraitOrientation = readSurfaceOrientation(adb, serial, steps, 'read portrait SurfaceOrientation');

  const portraitPng = path.join(runDir, 'portrait.png');
  if (!screenshot(adb, serial, portraitPng, steps, 'capture portrait screenshot')) {
    throw new Error('Failed to capture portrait screenshot.');
  }

  const landscapeRotate = run(adb, adbArgs(serial, ['shell', 'settings', 'put', 'system', 'user_rotation', '1']));
  steps.push({ label: 'set landscape rotation', ...landscapeRotate });
  sleep(PAUSE_MS);
  const landscapeOrientation = readSurfaceOrientation(adb, serial, steps, 'read landscape SurfaceOrientation');

  const landscapePng = path.join(runDir, 'landscape.png');
  if (!screenshot(adb, serial, landscapePng, steps, 'capture landscape screenshot')) {
    throw new Error('Failed to capture landscape screenshot.');
  }

  const restorePortrait = run(adb, adbArgs(serial, ['shell', 'settings', 'put', 'system', 'user_rotation', '0']));
  steps.push({ label: 'restore portrait rotation', ...restorePortrait });

  const portraitOk = portraitOrientation === 0 || portraitOrientation === 2;
  const landscapeOk = landscapeOrientation === 1 || landscapeOrientation === 3;
  const orientationVerified = portraitOk && landscapeOk;

  const report = {
    schema_version: 1,
    ts: new Date().toISOString(),
    ok: orientationVerified,
    adb,
    serial,
    model: String(model.stdout || '').trim(),
    package_name: PACKAGE_NAME,
    activity: ACTIVITY || null,
    pause_ms: PAUSE_MS,
    allow_locked: ALLOW_LOCKED,
    orientation: {
      portrait_surface_orientation: portraitOrientation,
      landscape_surface_orientation: landscapeOrientation,
      verified: orientationVerified,
    },
    screenshots: {
      portrait: path.relative(ROOT, portraitPng).replace(/\\/g, '/'),
      landscape: path.relative(ROOT, landscapePng).replace(/\\/g, '/'),
    },
    run_dir: path.relative(ROOT, runDir).replace(/\\/g, '/'),
    steps,
  };

  const latestPath = path.join(ART_DIR, 'latest.json');
  fs.writeFileSync(path.join(runDir, 'report.json'), JSON.stringify(report, null, 2));
  fs.writeFileSync(latestPath, JSON.stringify(report, null, 2));

  if (!orientationVerified) {
    if (ALLOW_LOCKED) {
      console.log('ANDROID_ORIENTATION_UX_OK=0');
      console.log('ANDROID_ORIENTATION_UX_LOCKED=1');
      console.log('ANDROID_ORIENTATION_UX_NOTE=SurfaceOrientation did not switch to landscape; app/device appears orientation-locked.');
      console.log(`ANDROID_ORIENTATION_UX_ARTIFACT=${latestPath}`);
      console.log(`ANDROID_ORIENTATION_UX_RUN_DIR=${runDir}`);
      return;
    }

    console.error('ANDROID_ORIENTATION_UX_OK=0');
    console.error('ANDROID_ORIENTATION_UX_ERROR=SurfaceOrientation did not switch to landscape; app/device appears orientation-locked.');
    console.error(`ANDROID_ORIENTATION_UX_ARTIFACT=${latestPath}`);
    console.error(`ANDROID_ORIENTATION_UX_RUN_DIR=${runDir}`);
    process.exit(1);
  }

  console.log('ANDROID_ORIENTATION_UX_OK=1');
  console.log(`ANDROID_ORIENTATION_UX_ARTIFACT=${latestPath}`);
  console.log(`ANDROID_ORIENTATION_UX_RUN_DIR=${runDir}`);
}

try {
  main();
} catch (error) {
  console.error(`ANDROID_ORIENTATION_UX_OK=0`);
  console.error(`ANDROID_ORIENTATION_UX_ERROR=${String(error)}`);
  process.exit(1);
}
