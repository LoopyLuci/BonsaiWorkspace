<script lang="ts">
  import { createEventDispatcher, onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { apiPort } from '$lib/stores/settings';
  import { DEFAULT_API_PORT } from '$lib/constants/network';

  const dispatch = createEventDispatcher<{ close: void }>();

  type AndroidUsbDevice = {
    serial: string;
    state: string;
    model?: string;
    device?: string;
    transport_id?: string;
    raw?: string;
  };

  let usbBusy = false;
  let usbError = '';
  let usbResult = '';
  let usbDevices: AndroidUsbDevice[] = [];
  let usbSelectedSerial = '';
  let usbApkPath = '';
  let usbPackageName = 'com.bonsai.workspace';
  let usbActivity = '';
  let usbWifiHost = '';
  let usbWifiPort = 5555;
  let usbShellCommand = 'getprop ro.product.model';
  let usbAdbExecutable = '';
  let usbAdbCandidates: string[] = [];

  type UsbReadiness = {
    serial: string;
    adb_executable: string;
    connected: boolean;
    authorized: boolean;
    model: string | null;
    reverse_api_active: boolean;
    api_port: number;
    status: 'disconnected' | 'unauthorized' | 'online' | 'ready';
    next_action: string;
  };
  let usbReadiness: UsbReadiness | null = null;
  let usbStrictMode = false;
  let usbEnableWifiBridge = false;
  let usbResolvedApk: { path: string; package: string | null; version_name: string | null; size_bytes: number } | null = null;
  type UsbStep = { label: string; ok: boolean; stdout: string; stderr: string; duration_ms: number; skipped?: boolean; hint?: string };
  let usbLastSteps: UsbStep[] = [];

  onMount(async () => {
    try { await refreshAndroidUsbDevices(); } catch {}
  });

  function appendUsbResult(title: string, payload: unknown) {
    const body = typeof payload === 'string' ? payload : JSON.stringify(payload);
    const line = `[${new Date().toISOString()}] ${title}: ${body}`;
    usbResult = usbResult ? `${line}\n${usbResult}` : line;
  }

  async function withInvokeTimeout<T>(promise: Promise<T>, ms: number, label: string): Promise<T> {
    let timer: ReturnType<typeof setTimeout> | null = null;
    try {
      return await Promise.race([
        promise,
        new Promise<T>((_, reject) => {
          timer = setTimeout(() => reject(new Error(`${label} timed out after ${ms}ms`)), ms);
        }),
      ]);
    } finally {
      if (timer) clearTimeout(timer);
    }
  }

  function getSelectedUsbSerial(): string {
    const serial = usbSelectedSerial.trim();
    if (!serial) {
      throw new Error('No Android device selected. Connect over USB and press Refresh USB Devices.');
    }
    return serial;
  }

  async function refreshAndroidUsbDevices() {
    usbBusy = true;
    usbError = '';
    try {
      const adbInfo = await withInvokeTimeout(
        invoke<{ adb_executable?: string; candidates?: string[] }>('android_usb_get_adb_info'),
        12000,
        'ADB info lookup',
      );
      usbAdbExecutable = String(adbInfo?.adb_executable || 'adb');
      usbAdbCandidates = Array.isArray(adbInfo?.candidates) ? adbInfo.candidates : [];

      const res = await withInvokeTimeout(
        invoke<{ devices?: AndroidUsbDevice[] }>('android_usb_list_devices'),
        20000,
        'ADB device refresh',
      );
      usbDevices = Array.isArray(res?.devices) ? res.devices : [];
      if (usbDevices.length > 0) {
        const current = usbSelectedSerial.trim();
        if (!current || !usbDevices.some((d) => d.serial === current)) {
          usbSelectedSerial = usbDevices[0].serial;
        }
      }
      appendUsbResult('ADB device refresh', { count: usbDevices.length, selected: usbSelectedSerial || null });
    } catch (e) {
      usbError = String(e);
      appendUsbResult('ADB device refresh failed', usbError);
    } finally {
      usbBusy = false;
    }
  }

  async function checkDeviceReadiness() {
    const serial = usbSelectedSerial.trim();
    usbBusy = true;
    usbError = '';
    try {
      const r = await withInvokeTimeout(
        invoke<UsbReadiness>('android_usb_get_device_readiness', {
          serial,
          apiPort: Number($apiPort || DEFAULT_API_PORT),
        }),
        20000,
        'Device readiness check',
      );
      usbReadiness = r;
      appendUsbResult('Device readiness check', r);
    } catch (e) {
      usbError = String(e);
      usbReadiness = null;
    } finally {
      usbBusy = false;
    }
  }

  async function resolveApk() {
    usbBusy = true;
    usbError = '';
    try {
      const r = await invoke<{ path: string; package: string | null; version_name: string | null; size_bytes: number }>(
        'android_usb_resolve_apk',
        { explicitPath: usbApkPath.trim() || null },
      );
      usbResolvedApk = r;
      usbApkPath = r.path;
      appendUsbResult('APK resolved', r);
    } catch (e) {
      usbError = String(e);
      usbResolvedApk = null;
    } finally {
      usbBusy = false;
    }
  }

  async function installAndLaunch() {
    const serial = getSelectedUsbSerial();
    if (!usbApkPath.trim()) {
      usbError = 'APK path is required. Use Resolve APK or enter a path manually.';
      return;
    }
    usbBusy = true;
    usbError = '';
    usbLastSteps = [];
    try {
      const r = await invoke<{ ok: boolean; steps: UsbStep[] }>('android_usb_install_and_launch', {
        serial,
        apkPath: usbApkPath.trim(),
        packageName: usbPackageName.trim() || null,
        activity: usbActivity.trim() || null,
        strictRequireApp: usbStrictMode,
      });
      usbLastSteps = (r as any).steps || [];
      appendUsbResult('Install & Launch', { ok: (r as any).ok, steps: (r as any).steps?.length });
    } catch (e) {
      usbError = String(e);
    } finally {
      usbBusy = false;
    }
  }

  async function bootstrapConnection() {
    const serial = getSelectedUsbSerial();
    usbBusy = true;
    usbError = '';
    usbLastSteps = [];
    try {
      const r = await invoke<{ ok: boolean; steps: UsbStep[] }>('android_usb_bootstrap_connection', {
        serial,
        apiPort: Number($apiPort || DEFAULT_API_PORT),
        wifiHost: usbWifiHost.trim() || null,
        wifiPort: Number(usbWifiPort || 5555),
        enableWifiBridge: usbEnableWifiBridge,
      });
      usbLastSteps = (r as any).steps || [];
      appendUsbResult('Bootstrap Connection', { ok: (r as any).ok, steps: (r as any).steps?.length });
      if ((r as any).ok) {
        await checkDeviceReadiness();
      }
    } catch (e) {
      usbError = String(e);
    } finally {
      usbBusy = false;
    }
  }

  async function runUsbRegressionSuite() {
    const serial = getSelectedUsbSerial();
    const host = (usbWifiHost || '').trim();
    try {
      const out = await invoke('android_usb_run_regression', {
        serial,
        apiPort: Number($apiPort || DEFAULT_API_PORT),
        packageName: usbPackageName.trim() || null,
        activity: usbActivity.trim() || null,
        wifiHost: host || null,
        wifiPort: Number(usbWifiPort || 5555),
        strictRequireApp: usbStrictMode,
        apkPath: usbApkPath.trim() || null,
        enableBootstrap: usbEnableWifiBridge,
      });
      usbResult = JSON.stringify(out || {});
    } catch (e) {
      usbError = String(e);
    }
  }
</script>

<!-- Simple modal wrapper for Android USB Lab -->
<div class="usb-overlay" on:click|self={() => dispatch('close')}>
  <div class="usb-panel" role="dialog" aria-modal="true" aria-label="Android USB Lab">
    <header class="usb-header">
      <h2>Android USB Lab</h2>
      <button class="close-btn" on:click={() => dispatch('close')}>✕</button>
    </header>

    <div class="usb-body">
      <p class="section-desc">Plug in an Android device and follow the guided flow: Readiness → Install & Launch → Bootstrap → Run Validation.</p>

      <div class="action-grid">
        <button class="action-btn" on:click={refreshAndroidUsbDevices} disabled={usbBusy}>{usbBusy ? 'Refreshing…' : 'Refresh Devices'}</button>
        <button class="action-btn" on:click={checkDeviceReadiness} disabled={usbBusy || !usbSelectedSerial}>Check Readiness</button>
      </div>

      <div class="form-group">
        <label>Device (adb serial)</label>
        <select bind:value={usbSelectedSerial}>
          <option value="">Select device</option>
          {#each usbDevices as d}
            <option value={d.serial}>{d.serial} ({d.state}{d.model ? ` · ${d.model}` : ''})</option>
          {/each}
        </select>
      </div>

      {#if usbReadiness}
        <div class="usb-readiness-card status-{usbReadiness.status}">
          <div class="readiness-badge">{usbReadiness.status.toUpperCase()}</div>
          <div class="readiness-detail">
            {#if usbReadiness.model}<div><strong>Model:</strong> {usbReadiness.model}</div>{/if}
            <div><strong>ADB:</strong> <code>{usbReadiness.adb_executable}</code></div>
            <div><strong>Reverse active:</strong> {usbReadiness.reverse_api_active ? '✓ yes' : '✗ no'} (port {usbReadiness.api_port})</div>
          </div>
          {#if usbReadiness.status !== 'ready'}
            <div class="readiness-next-action">→ {usbReadiness.next_action}</div>
          {/if}
        </div>
      {/if}

      <div class="form-group">
        <label>APK Path</label>
        <div class="input-with-btn">
          <input type="text" bind:value={usbApkPath} placeholder="C:/path/to/app.apk or leave blank to auto-resolve" />
          <button class="action-btn" on:click={resolveApk} disabled={usbBusy}>Resolve</button>
        </div>
      </div>

      <div class="form-group">
        <label>Package Name</label>
        <input type="text" bind:value={usbPackageName} placeholder="com.bonsai.workspace" />
      </div>

      <div class="action-grid usb-flow-grid">
        <button class="action-btn green" on:click={installAndLaunch} disabled={usbBusy || !usbSelectedSerial}>Install & Launch</button>
        <button class="action-btn blue" on:click={bootstrapConnection} disabled={usbBusy || !usbSelectedSerial}>Bootstrap Connection</button>
        <button class="action-btn green full-width" on:click={runUsbRegressionSuite} disabled={usbBusy || !usbSelectedSerial}>Run Full Validation</button>
      </div>

      {#if usbError}
        <div class="pair-error">{usbError}</div>
      {/if}
      {#if usbResult}
        <pre class="usb-result">{usbResult}</pre>
      {/if}
    </div>
  </div>
</div>

<style>
  .usb-overlay { position: fixed; inset: 0; background: rgba(0,0,0,0.6); z-index: var(--z-modal); display:flex; align-items:center; justify-content:center; }
  .usb-panel { width: 680px; max-height: 86vh; overflow:auto; background: var(--bg2); border-radius: 10px; border:1px solid var(--border); }
  .usb-header { display:flex; justify-content:space-between; align-items:center; padding:12px 16px; border-bottom:1px solid var(--border); }
  .usb-body { padding: 14px 16px; }
  .action-grid { display:grid; grid-template-columns: 1fr 1fr; gap:8px; margin-bottom:8px; }
  .action-btn { padding:8px 10px; border-radius:8px; background:var(--accent); color:#fff; border:none; }
  .action-btn.green { background:#16a34a } .action-btn.blue{ background:var(--accent) }
  .form-group { margin-top:8px; }
  .input-with-btn { display:flex; gap:8px; }
  .usb-result { margin-top:10px; max-height:220px; overflow:auto; background:var(--bg); padding:10px; border-radius:8px; }
  .close-btn { background:transparent; border:none; font-size:16px; cursor:pointer; }
  .usb-readiness-card { margin-top:8px; padding:10px; border-radius:8px; border:1px solid var(--border); background:var(--bg); }
  .readiness-badge { font-size:10px; font-weight:700; padding:2px 8px; border-radius:6px; background:rgba(128,128,128,.12); }
  .pair-error { color: var(--red); padding:8px; margin-top:8px; }
</style>
