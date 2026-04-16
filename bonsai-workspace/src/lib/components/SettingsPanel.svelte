<script lang="ts">
  import { createEventDispatcher, onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { scan } from '@tauri-apps/plugin-barcode-scanner';
  import { addAssistantMessage } from '$lib/stores/chat';
  import { DEFAULT_API_PORT } from '$lib/constants/network';
  import { availableModels, activeModel, activeModelId, orchestratorStatus, refreshStatus, refreshModels, modelSwitchStatus } from '$lib/stores/models';
  import { apiHost, apiPort, apiBaseUrl, loadApiSettings, saveApiSettings } from '$lib/stores/settings';
  import {
    BONSAI_CATALOG, findRegistryModel,
    downloadCatalogModel, downloadingId, downloadPct, downloadError,
  } from '$lib/stores/catalog';

  const dispatch = createEventDispatcher<{ close: void }>();

  let hwInfo:          Record<string, unknown> = {};
  let loadingOp        = '';
  let errorMsg         = '';
  let switchDetails    = '';
  let apiTestResult    = '';
  let apiTestLoading   = false;
  let saveApiLoading   = false;

  let remoteSessionId  = '';
  let remoteState      = 'inactive';
  let remoteStatus     = '';
  let remoteStreamUrl  = '';
  let remoteFrameUrl   = '';
  let remoteInputUrl   = '';
  let remoteLoading    = false;
  let remotePreviewSrc = '';
  let remotePreviewErr = '';
  let remoteInputResult = '';
  let remoteEventSource: EventSource | null = null;

  onMount(async () => {
    await refreshModels();
    await refreshStatus();
    modelSwitchStatus.set('');
    try { hwInfo = await invoke<Record<string,unknown>>('get_hardware_info'); } catch {}
    try { await loadApiSettings(); } catch (e) { console.warn('Failed to load API settings', e); }
    try { await refreshAndroidUsbDevices(); } catch {}
  });

  async function switchModel(modelId: string, name: string) {
    activeModelId.set(modelId);
    loadingOp = `Switching to ${name}…`;
    switchDetails = `Switching to ${name}…`;
    modelSwitchStatus.set(switchDetails);
    errorMsg = '';
    try {
      const msg = await invoke<string>('switch_model', { modelId });
      await refreshModels();
      await refreshStatus();
      switchDetails = `${msg} Orchestrator refreshed.`;
      modelSwitchStatus.set(switchDetails);
      addAssistantMessage(msg);
    } catch (e) {
      errorMsg = String(e);
      switchDetails = `Model switch failed: ${errorMsg}`;
      modelSwitchStatus.set(switchDetails);
    } finally { loadingOp = ''; }
  }

  async function copyApiEndpoint() {
    try {
      await navigator.clipboard.writeText($apiBaseUrl);
      apiTestResult = `Copied API endpoint: ${$apiBaseUrl}`;
    } catch {
      apiTestResult = 'Unable to copy API endpoint. Please copy manually.';
    }
  }

  async function testApiEndpoint() {
    apiTestLoading = true;
    apiTestResult = '';
    try {
      const resp = await fetch(`${$apiBaseUrl}/v1/models`);
      const json = await resp.json();
      if (resp.ok) {
        apiTestResult = `API reachable: ${json.data?.length ?? 'unknown'} model(s) available.`;
      } else {
        apiTestResult = `API error: ${json.error?.message ?? resp.statusText}`;
      }
    } catch (e) {
      apiTestResult = `API test failed: ${String(e)}`;
    } finally {
      apiTestLoading = false;
    }
  }

  async function applyApiSettings() {
    saveApiLoading = true;
    apiTestResult = '';
    try {
      const config = await saveApiSettings($apiHost, $apiPort);
      apiTestResult = `API settings saved: ${config.api_host}:${config.api_port}`;
    } catch (e) {
      apiTestResult = `Save failed: ${String(e)}`;
    } finally {
      saveApiLoading = false;
    }
  }

  function disconnectRemotePreview() {
    remotePreviewSrc = '';
    remotePreviewErr = '';
    if (remoteEventSource) {
      remoteEventSource.close();
      remoteEventSource = null;
    }
  }

  function connectRemotePreview(url: string) {
    disconnectRemotePreview();
    try {
      remoteEventSource = new EventSource(url);
      remoteEventSource.onmessage = event => {
        try {
          const data = JSON.parse(event.data);
          if (data.frame) {
            remotePreviewSrc = `data:image/png;base64,${data.frame}`;
            remotePreviewErr = '';
          }
        } catch (err) {
          remotePreviewErr = `Preview parse error: ${String(err)}`;
        }
      };
      remoteEventSource.onerror = () => {
        remotePreviewErr = 'Remote preview connection lost.';
      };
    } catch (err) {
      remotePreviewErr = `Failed to connect preview: ${String(err)}`;
    }
  }

  async function startRemoteSession() {
    remoteLoading = true;
    remoteStatus = 'Starting remote session…';
    try {
      const result = await invoke<{
        session_id: string;
        state: string;
        stream_url: string;
        frame_url: string;
        input_url: string;
      }>('start_remote_session');
      remoteSessionId = result.session_id;
      remoteState = result.state;
      remoteStreamUrl = result.stream_url;
      remoteFrameUrl = result.frame_url;
      remoteInputUrl = result.input_url;
      remoteStatus = `Remote session started. Stream URL is available.`;
      connectRemotePreview(remoteStreamUrl);
    } catch (e) {
      remoteStatus = `Failed to start remote session: ${String(e)}`;
    } finally {
      remoteLoading = false;
    }
  }

  async function stopRemoteSession() {
    remoteLoading = true;
    remoteStatus = 'Stopping remote session…';
    try {
      await invoke('stop_remote_session');
      remoteSessionId = '';
      remoteState = 'inactive';
      remoteStreamUrl = '';
      remoteFrameUrl = '';
      remoteInputUrl = '';
      remoteStatus = 'Remote session stopped.';
      disconnectRemotePreview();
    } catch (e) {
      remoteStatus = `Failed to stop remote session: ${String(e)}`;
    } finally {
      remoteLoading = false;
    }
  }

  async function copyRemoteStreamUrl() {
    if (!remoteStreamUrl) return;
    try {
      await navigator.clipboard.writeText(remoteStreamUrl);
      remoteStatus = 'Remote stream URL copied to clipboard.';
    } catch {
      remoteStatus = 'Unable to copy remote stream URL. Copy manually.';
    }
  }

  async function sendRemoteInputTest(eventType: string) {
    if (!remoteSessionId) {
      remoteInputResult = 'Start a session before sending remote input.';
      return;
    }
    remoteInputResult = 'Sending test input…';
    const payload = {
      event_type: eventType,
      x: 100,
      y: 100,
      button: 'left',
      key: eventType === 'key' ? 'Enter' : undefined,
      modifiers: eventType === 'key' ? ['control'] : undefined,
    };

    try {
      const result = await invoke<{ status: string }>('send_remote_input', { event: payload });
      remoteInputResult = `Remote input accepted: ${result.status}`;
    } catch (e) {
      remoteInputResult = `Remote input failed: ${String(e)}`;
    }
  }

  onDestroy(() => {
    disconnectRemotePreview();
  });

  async function downloadWhisper() {
    loadingOp = 'Downloading Whisper model…';
    try {
      const path = await invoke<string>('download_whisper_model');
      addAssistantMessage(`✅ Whisper model saved to \`${path}\``);
    } catch (e) { errorMsg = String(e); }
    finally { loadingOp = ''; }
  }

  async function importGguf() {
    try {
      const path = await invoke<string>('prompt_gguf_import');
      if (path) { addAssistantMessage(`📦 Model imported from \`${path}\``); await refreshModels(); }
    } catch (e) { errorMsg = String(e); }
  }

  async function handleDownload(entry: typeof BONSAI_CATALOG[number]) {
    await downloadCatalogModel(entry);
    if ($downloadError) errorMsg = $downloadError;
  }

  // ── Connection / pairing ──────────────────────────────────────────────────
  let pairToken    = '';
  let pairQrSvg   = '';
  let localIp     = '';
  let wsClientCount = 0;
  let pairLoading = false;
  let pairScanLoading = false;
  let pairScanResult = '';
  let pairVerifyLoading = false;
  let pairVerifyResult = '';
  let pairEvidencePath = '';
  let pairLastEvidence: Record<string, unknown> | null = null;
  let pairError   = '';

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
  let usbRegressionEvidencePath = '';
  let usbRegressionLast: Record<string, unknown> | null = null;

  // USB Lab Runtime System — new state.
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

  function extractScannedValue(payload: unknown): string {
    if (typeof payload === 'string') return payload;
    if (Array.isArray(payload)) {
      for (const item of payload) {
        const value = extractScannedValue(item);
        if (value) return value;
      }
      return '';
    }
    if (payload && typeof payload === 'object') {
      const obj = payload as Record<string, unknown>;
      if (typeof obj.content === 'string') return obj.content;
      if (typeof obj.rawValue === 'string') return obj.rawValue;
      if (typeof obj.displayValue === 'string') return obj.displayValue;
    }
    return '';
  }

  function parseBonsaiConnectUrl(scanned: string): { ip: string; token: string } {
    let parsed: URL;
    try {
      parsed = new URL(scanned);
    } catch {
      throw new Error('Scanned QR is not a valid URL.');
    }

    if (parsed.protocol !== 'bonsai:') {
      throw new Error(`Unsupported QR scheme: ${parsed.protocol}`);
    }

    const ipParam = parsed.searchParams.get('ip')?.trim() || '';
    const token = parsed.searchParams.get('token')?.trim() || '';
    const port = parsed.searchParams.get('port')?.trim() || '';
    if (!ipParam) throw new Error('QR code is missing ip parameter.');
    if (!token) throw new Error('QR code is missing token parameter.');

    const ip = port && !ipParam.includes(':') ? `${ipParam}:${port}` : ipParam;
    return { ip, token };
  }

  function buildPairingWsUrl(ip: string): string {
    const trimmed = ip.trim();
    if (!trimmed) throw new Error('Desktop connection IP is empty.');

    const hasScheme = /^wss?:\/\//i.test(trimmed);
    const base = hasScheme ? trimmed : `ws://${trimmed}`;
    const url = new URL(base);
    if (!url.port) {
      url.port = String(DEFAULT_API_PORT);
    }
    url.pathname = '/ws';
    url.search = '';
    url.hash = '';
    return url.toString();
  }

  function tokenHint(token: string): string {
    const t = token.trim();
    if (!t) return '';
    if (t.length <= 4) return t;
    return `${t.slice(0, 2)}***${t.slice(-2)}`;
  }

  async function verifyMobilePairingOverWs(connection: { ip: string; token: string }): Promise<{ ok: boolean; detail: string; wsUrl: string; elapsedMs: number }> {
    const wsUrl = buildPairingWsUrl(connection.ip);
    const start = Date.now();

    return await new Promise((resolve) => {
      let settled = false;
      let timer: ReturnType<typeof setTimeout> | null = null;
      let socket: WebSocket | null = null;

      const finish = (ok: boolean, detail: string) => {
        if (settled) return;
        settled = true;
        const elapsedMs = Date.now() - start;
        if (timer) clearTimeout(timer);
        if (socket && socket.readyState === WebSocket.OPEN) {
          socket.close();
        }
        resolve({ ok, detail, wsUrl, elapsedMs });
      };

      try {
        socket = new WebSocket(wsUrl);
      } catch (err) {
        finish(false, `WebSocket init failed: ${String(err)}`);
        return;
      }

      timer = setTimeout(() => {
        finish(false, 'Timed out waiting for auth_ok/auth_fail response.');
      }, 8000);

      socket.onopen = () => {
        try {
          socket?.send(JSON.stringify({
            type: 'auth',
            payload: { token: connection.token },
          }));
        } catch (err) {
          finish(false, `Failed to send auth payload: ${String(err)}`);
        }
      };

      socket.onmessage = (event) => {
        let msg: Record<string, unknown>;
        try {
          msg = JSON.parse(String(event.data));
        } catch {
          finish(false, 'Received non-JSON response from desktop websocket.');
          return;
        }

        const type = String(msg.type || '');
        if (type === 'auth_ok') {
          finish(true, 'Received auth_ok from desktop websocket.');
          return;
        }
        if (type === 'auth_fail') {
          const payload = msg.payload as Record<string, unknown> | undefined;
          const reason = payload && typeof payload.reason === 'string' ? payload.reason : 'unknown reason';
          finish(false, `Received auth_fail: ${reason}`);
        }
      };

      socket.onerror = () => {
        finish(false, 'WebSocket connection error while verifying pairing.');
      };
    });
  }

  async function captureMobilePairingEvidence(params: {
    source: string;
    connection: { ip: string; token: string };
    scannedPayload?: string;
    verification: { ok: boolean; detail: string; wsUrl: string; elapsedMs: number };
  }) {
    try {
      const res = await invoke<{ path?: string; record?: Record<string, unknown> }>('record_mobile_pairing_evidence', {
        source: params.source,
        ip: params.connection.ip,
        verified: params.verification.ok,
        detail: params.verification.detail,
        wsUrl: params.verification.wsUrl,
        elapsedMs: params.verification.elapsedMs,
        scannedPayload: params.scannedPayload ?? null,
        tokenHint: tokenHint(params.connection.token),
      });
      pairEvidencePath = String(res?.path || '');
      pairLastEvidence = (res?.record || null) as Record<string, unknown> | null;
    } catch (e) {
      pairError = `Evidence capture failed: ${String(e)}`;
    }
  }

  async function loadPairInfo() {
    pairLoading = true;
    pairError   = '';
    try {
      [pairToken, localIp, pairQrSvg] = await Promise.all([
        invoke<string>('get_pair_token'),
        invoke<string>('get_local_ip'),
        invoke<string>('generate_pair_qr'),
      ]);
      wsClientCount = await invoke<number>('ws_client_count');
      const evidence = await invoke<{ path?: string; items?: Record<string, unknown>[] }>('get_mobile_pairing_evidence', { limit: 1 });
      pairEvidencePath = String(evidence?.path || '');
      pairLastEvidence = Array.isArray(evidence?.items) && evidence.items.length > 0
        ? evidence.items[evidence.items.length - 1]
        : null;
    } catch (e) {
      pairError = String(e);
    } finally {
      pairLoading = false;
    }
  }

  async function refreshWsCount() {
    try { wsClientCount = await invoke<number>('ws_client_count'); } catch {}
  }

  async function scanMobilePairQr() {
    pairScanLoading = true;
    pairScanResult = '';
    pairError = '';
    try {
      const raw = await scan({ windowed: true });
      const scanned = extractScannedValue(raw);
      if (!scanned) {
        throw new Error('Scanner returned an empty QR payload.');
      }

      const connection = parseBonsaiConnectUrl(scanned);
      await invoke('save_desktop_connection', connection);
      pairScanResult = `Saved desktop connection: ${connection.ip}`;

      pairVerifyLoading = true;
      const verification = await verifyMobilePairingOverWs(connection);
      pairVerifyResult = verification.ok
        ? `Pairing verified (${verification.elapsedMs}ms): ${verification.detail}`
        : `Pairing verification failed (${verification.elapsedMs}ms): ${verification.detail}`;

      await captureMobilePairingEvidence({
        source: 'qr_scan',
        connection,
        scannedPayload: scanned,
        verification,
      });

      addAssistantMessage(`Mobile pairing target saved: ${connection.ip}. ${pairVerifyResult}`);
    } catch (e) {
      pairError = `Scan failed: ${String(e)}`;
    } finally {
      pairVerifyLoading = false;
      pairScanLoading = false;
    }
  }

  async function verifySavedMobilePairing() {
    pairVerifyLoading = true;
    pairVerifyResult = '';
    pairError = '';
    try {
      const saved = await invoke<{ ip: string; token: string } | null>('load_desktop_connection');
      if (!saved) {
        throw new Error('No saved desktop connection found. Scan QR first.');
      }

      const connection = {
        ip: String(saved.ip || '').trim(),
        token: String(saved.token || '').trim(),
      };
      if (!connection.ip || !connection.token) {
        throw new Error('Saved desktop connection is incomplete.');
      }

      const verification = await verifyMobilePairingOverWs(connection);
      pairVerifyResult = verification.ok
        ? `Saved pairing verified (${verification.elapsedMs}ms): ${verification.detail}`
        : `Saved pairing verification failed (${verification.elapsedMs}ms): ${verification.detail}`;

      await captureMobilePairingEvidence({
        source: 'saved_connection',
        connection,
        verification,
      });
    } catch (e) {
      pairError = `Saved pairing verification failed: ${String(e)}`;
    } finally {
      pairVerifyLoading = false;
    }
  }

  function appendUsbResult(title: string, payload: unknown) {
    const body = typeof payload === 'string' ? payload : JSON.stringify(payload);
    const line = `[${new Date().toISOString()}] ${title}: ${body}`;
    usbResult = usbResult ? `${line}\n${usbResult}` : line;
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
      const adbInfo = await invoke<{ adb_executable?: string; candidates?: string[] }>('android_usb_get_adb_info');
      usbAdbExecutable = String(adbInfo?.adb_executable || 'adb');
      usbAdbCandidates = Array.isArray(adbInfo?.candidates) ? adbInfo.candidates : [];

      const res = await invoke<{ devices?: AndroidUsbDevice[] }>('android_usb_list_devices');
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
      const r = await invoke<UsbReadiness>('android_usb_get_device_readiness', {
        serial,
        apiPort: Number($apiPort || DEFAULT_API_PORT),
      });
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
      usbLastSteps = r.steps || [];
      appendUsbResult('Install & Launch', { ok: r.ok, steps: r.steps?.length });
      if (!r.ok) {
        const failed = r.steps?.find((s) => !s.ok && !s.skipped);
        usbError = failed ? `Step failed: ${failed.label}. ${failed.stderr || failed.hint || ''}` : 'Install & Launch failed.';
      }
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
      usbLastSteps = r.steps || [];
      appendUsbResult('Bootstrap Connection', { ok: r.ok, steps: r.steps?.length });
      if (r.ok) {
        // Refresh readiness so the badge updates.
        await checkDeviceReadiness();
      } else {
        const failed = r.steps?.find((s) => !s.ok);
        usbError = failed ? `Step failed: ${failed.label}. ${failed.stderr || ''}` : 'Bootstrap failed.';
      }
    } catch (e) {
      usbError = String(e);
    } finally {
      usbBusy = false;
    }
  }

  async function runUsbAction(title: string, runner: () => Promise<unknown>) {
    usbBusy = true;
    usbError = '';
    try {
      const out = await runner();
      appendUsbResult(title, out);
      return out;
    } catch (e) {
      usbError = String(e);
      appendUsbResult(`${title} failed`, usbError);
      throw e;
    } finally {
      usbBusy = false;
    }
  }

  async function clearUsbReverse() {
    const serial = getSelectedUsbSerial();
    await runUsbAction('Clear adb reverse mappings', () => invoke('android_usb_reverse_clear', { serial }));
  }

  async function enableUsbWifiDebug() {
    const serial = getSelectedUsbSerial();
    await runUsbAction('Enable WiFi debugging (adb tcpip)', () =>
      invoke('android_usb_enable_wifi_debug', {
        serial,
        port: Number(usbWifiPort || 5555),
      }),
    );
  }

  async function connectUsbWifiDebug() {
    const host = (usbWifiHost || localIp || '').trim();
    if (!host) {
      usbError = 'WiFi host is required. Use device IP on the same network.';
      return;
    }
    await runUsbAction('Connect WiFi debugging', () =>
      invoke('android_usb_connect_wifi', {
        host,
        port: Number(usbWifiPort || 5555),
      }),
    );
    await refreshAndroidUsbDevices();
  }

  async function disconnectUsbWifiDebug() {
    await runUsbAction('Disconnect WiFi debugging', () => invoke('android_usb_disconnect_wifi', { host: null }));
    await refreshAndroidUsbDevices();
  }

  async function runUsbShellCommand() {
    const serial = getSelectedUsbSerial();
    if (!usbShellCommand.trim()) {
      usbError = 'Shell command is required.';
      return;
    }
    await runUsbAction('ADB shell command', () =>
      invoke('android_usb_shell', {
        serial,
        shellCommand: usbShellCommand.trim(),
      }),
    );
  }

  async function runUsbRegressionSuite() {
    const serial = getSelectedUsbSerial();
    const host = (usbWifiHost || '').trim();
    try {
      const out = await runUsbAction('Run Android USB regression suite', () =>
        invoke<{ path?: string; record?: Record<string, unknown> }>('android_usb_run_regression', {
          serial,
          apiPort: Number($apiPort || DEFAULT_API_PORT),
          packageName: usbPackageName.trim() || null,
          activity: usbActivity.trim() || null,
          wifiHost: host || null,
          wifiPort: Number(usbWifiPort || 5555),
          strictRequireApp: usbStrictMode,
          apkPath: usbApkPath.trim() || null,
          enableBootstrap: usbEnableWifiBridge,
        }),
      ) as { path?: string; record?: Record<string, unknown> };

      usbRegressionEvidencePath = String(out?.path || '');
      usbRegressionLast = (out?.record || null) as Record<string, unknown> | null;
    } catch {
      // runUsbAction already captures details
    }
  }
</script>

<!-- svelte-ignore a11y-click-events-have-key-events a11y-no-static-element-interactions -->
<div class="settings-overlay" on:click|self={() => dispatch('close')} role="presentation">
  <div class="settings-panel" role="dialog" aria-modal="true" aria-label="Settings">
    <header class="settings-header">
      <h2>Settings</h2>
      <button class="close-btn" on:click={() => dispatch('close')} aria-label="Close settings">✕</button>
    </header>

    {#if errorMsg}
      <div class="error-bar" role="alert">
        {errorMsg}
        <button on:click={() => (errorMsg = '')}>✕</button>
      </div>
    {/if}

    {#if loadingOp}
      <div class="loading-bar">{loadingOp}</div>
    {/if}

    <!-- Hardware info -->
    {#if Object.keys(hwInfo).length}
      <section class="section">
        <h3 class="section-title">Hardware</h3>
        <div class="hw-grid">
          <div class="hw-item"><span class="hw-label">RAM</span><span class="hw-val">{hwInfo.ram_total_gb} GB</span></div>
          <div class="hw-item"><span class="hw-label">Available</span><span class="hw-val">{hwInfo.ram_available_gb} GB</span></div>
          <div class="hw-item"><span class="hw-label">CPUs</span><span class="hw-val">{hwInfo.cpu_count}</span></div>
          <div class="hw-item"><span class="hw-label">Backend</span><span class="hw-val">{hwInfo.backend}</span></div>
          {#if Array.isArray(hwInfo.gpu_names) && hwInfo.gpu_names.length > 0}
            <div class="hw-item hw-item--wide"><span class="hw-label">GPU(s)</span><span class="hw-val hw-val--sm">{hwInfo.gpu_names.join(', ')}</span></div>
          {/if}
        </div>
      </section>
    {/if}

    {#if $orchestratorStatus}
      <section class="section status-summary">
        <h3 class="section-title">Orchestrator Status</h3>
        <div class="status-grid">
          <div class="status-item"><span class="status-label">Slots</span><span class="status-value">{$orchestratorStatus.slots.length}</span></div>
          <div class="status-item"><span class="status-label">Queue</span><span class="status-value">{$orchestratorStatus.queue_depth}</span></div>
          <div class="status-item"><span class="status-label">RAM Free</span><span class="status-value">{Math.round($orchestratorStatus.free_ram_mb / 1024)} GB</span></div>
          <div class="status-item"><span class="status-label">Total RAM</span><span class="status-value">{Math.round($orchestratorStatus.total_ram_mb / 1024)} GB</span></div>
        </div>
        <div class="slot-list">
          {#each $orchestratorStatus.slots as slot}
            <div class="slot-row">
              <span class="slot-label">Slot {slot.index}</span>
              <span class="slot-state">{slot.state.state}{slot.state.model_id ? ` (${slot.state.model_id})` : ''}</span>
              <span class="slot-info">{slot.requests} req · {slot.idle_secs}s idle</span>
            </div>
          {/each}
        </div>
      </section>
    {/if}

    <section class="section api-settings">
      <h3 class="section-title">API Settings</h3>
      <div class="form-group">
        <label for="api-host">API Host</label>
        <input id="api-host" type="text" bind:value={$apiHost} />
      </div>
      <div class="form-group">
        <label for="api-port">API Port</label>
        <input id="api-port" type="number" min="1" max="65535" bind:value={$apiPort} />
      </div>
      <div class="form-note">
        External agents can connect to the OpenAI-compatible endpoint shown here.
      </div>
      <div class="action-grid">
        <button class="action-btn blue" type="button" on:click={copyApiEndpoint} disabled={apiTestLoading || saveApiLoading || remoteLoading}>
          📋 Copy endpoint
        </button>
        <button class="action-btn green" type="button" on:click={testApiEndpoint} disabled={apiTestLoading || saveApiLoading || remoteLoading}>
          {apiTestLoading ? 'Testing…' : 'Test API'}
        </button>
        <button class="action-btn blue" type="button" on:click={applyApiSettings} disabled={apiTestLoading || saveApiLoading || remoteLoading}>
          {saveApiLoading ? 'Saving…' : 'Save API settings'}
        </button>
      </div>
      {#if apiTestResult}
        <div class="api-test-result">{apiTestResult}</div>
      {/if}
      <div class="api-endpoint">Current endpoint: <code>{$apiBaseUrl}</code></div>
    </section>

    <section class="section remote-control">
      <h3 class="section-title">Remote Control</h3>
      <div class="form-note">
        Start a local remote session to stream screen frames and negotiate input.
      </div>
      <div class="action-grid">
        <button class="action-btn green" type="button" on:click={startRemoteSession} disabled={remoteLoading || apiTestLoading || saveApiLoading}>
          {remoteLoading ? 'Starting…' : 'Start Remote Session'}
        </button>
        <button class="action-btn red" type="button" on:click={stopRemoteSession} disabled={remoteLoading || !remoteSessionId}>
          {remoteLoading ? 'Stopping…' : 'Stop Remote Session'}
        </button>
      </div>
      {#if remoteStatus}
        <div class="api-test-result">{remoteStatus}</div>
      {/if}
      {#if remoteSessionId}
        <div class="remote-info">
          <div><strong>Session ID:</strong> <code>{remoteSessionId}</code></div>
          <div><strong>Stream URL:</strong> <code>{remoteStreamUrl}</code> <button class="copy-link" type="button" on:click={copyRemoteStreamUrl}>Copy</button></div>
          <div><strong>Frame URL:</strong> <code>{remoteFrameUrl}</code></div>
          <div><strong>Input URL:</strong> <code>{remoteInputUrl}</code></div>
        </div>
        <div class="remote-action-grid">
          <button class="action-btn blue" type="button" on:click={() => sendRemoteInputTest('click')} disabled={!remoteSessionId || remoteLoading}>
            Send Test Click
          </button>
          <button class="action-btn blue" type="button" on:click={() => sendRemoteInputTest('key')} disabled={!remoteSessionId || remoteLoading}>
            Send Test Key
          </button>
        </div>
        {#if remoteInputResult}
          <div class="api-test-result">{remoteInputResult}</div>
        {/if}
        <div class="remote-preview">
          {#if remotePreviewErr}
            <div class="remote-error">{remotePreviewErr}</div>
          {/if}
          {#if remotePreviewSrc}
            <img class="remote-preview-img" src={remotePreviewSrc} alt="Remote preview" />
          {:else}
            <div class="remote-preview-placeholder">Waiting for first frame…</div>
          {/if}
        </div>
      {/if}
    </section>

    {#if switchDetails}
      <section class="section switch-details">
        <h3 class="section-title">Switch details</h3>
        <div class="switch-log">{switchDetails}</div>
      </section>
    {/if}

    <!-- Models -->
    <section class="section">
      <h3 class="section-title">Language Models</h3>
      <div class="model-list">
        {#each BONSAI_CATALOG as entry (entry.catalogId)}
          {@const reg = findRegistryModel(entry, $availableModels)}
          {@const isActive = !!reg && $activeModel?.id === reg.id}
          {@const isDling = $downloadingId === entry.catalogId}
          <div class="model-row" class:active-model={isActive}>
            <div class="model-info">
              <div class="model-name">
                {entry.name}
                {#if entry.isDefault}<span class="badge-default">default</span>{/if}
              </div>
              <div class="model-meta">
                {entry.quant} · ~{entry.ramGb} GB RAM
                {#if !reg}<span class="badge-notlocal">not downloaded</span>{/if}
              </div>
            </div>
            <div class="model-actions">
              {#if isActive}
                <span class="badge-active">Active</span>
              {:else if reg && !isDling}
                <button class="btn-sm" on:click={() => switchModel(reg.id, entry.name)} disabled={!!loadingOp}>
                  Use
                </button>
              {:else if isDling}
                <span class="badge-active" style="background: var(--accent)">{$downloadPct}%</span>
              {:else}
                <button class="btn-sm btn-dl" on:click={() => handleDownload(entry)} disabled={!!loadingOp}>
                  ⬇ Download
                </button>
              {/if}
            </div>
          </div>
        {/each}

        <!-- Any extra local models not in the catalog -->
        {#each $availableModels.filter(m => !BONSAI_CATALOG.some(e => findRegistryModel(e, $availableModels)?.id === m.id)) as model (model.id)}
          <div class="model-row" class:active-model={model.id === $activeModel?.id}>
            <div class="model-info">
              <div class="model-name">{model.name}</div>
              <div class="model-meta">{model.quant} · ~{Math.round(model.ram_required_mb / 1024 * 10) / 10} GB RAM</div>
            </div>
            <div class="model-actions">
              {#if model.id === $activeModel?.id}
                <span class="badge-active">Active</span>
              {:else}
                <button class="btn-sm" on:click={() => switchModel(model.id, model.name)} disabled={!!loadingOp}>Use</button>
              {/if}
            </div>
          </div>
        {/each}
      </div>

      <div class="action-grid">
        <button class="action-btn blue" on:click={importGguf} disabled={!!loadingOp}>
          📂 Import Local GGUF
        </button>
        <button class="action-btn green" on:click={downloadWhisper} disabled={!!loadingOp}>
          ⬇ Download Whisper
        </button>
      </div>
    </section>

    <!-- ── Connection / Pairing ──────────────────────────────────────────── -->
    <section class="section connection-section">
      <h3 class="section-title">Mobile & VSCode Connection</h3>
      <p class="section-desc">
        Scan the QR code with the Bonsai Android app, or paste the token into
        the <strong>Bonsai Workspace Runner</strong> VSCode extension settings.
      </p>
      <div class="pair-row">
        <div class="qr-area">
          {#if pairQrSvg}
            {@html pairQrSvg}
          {:else}
            <button class="action-btn" on:click={loadPairInfo} disabled={pairLoading}>
              {pairLoading ? 'Loading…' : 'Show QR Code'}
            </button>
          {/if}
        </div>
        <div class="pair-info">
          {#if pairToken}
            <div class="pair-field">
              <span class="pair-label">Pair token</span>
              <code class="pair-token">{pairToken}</code>
            </div>
            <div class="pair-field">
              <span class="pair-label">LAN IP</span>
              <code class="pair-token">{localIp || '…'}</code>
            </div>
            <div class="pair-field">
              <span class="pair-label">WebSocket</span>
              <code class="pair-token">ws://{localIp || '…'}:{DEFAULT_API_PORT}/ws</code>
            </div>
            <div class="pair-field">
              <span class="pair-label">WS clients</span>
              <code class="pair-token">{wsClientCount}</code>
            </div>
            <div class="action-grid pair-actions">
              <button class="action-btn" on:click={refreshWsCount}>↺ Refresh</button>
              <button class="action-btn blue" on:click={scanMobilePairQr} disabled={pairScanLoading}>
                {pairScanLoading ? 'Scanning…' : 'Scan Mobile QR'}
              </button>
              <button class="action-btn" on:click={verifySavedMobilePairing} disabled={pairVerifyLoading}>
                {pairVerifyLoading ? 'Verifying…' : 'Verify Saved Pairing'}
              </button>
            </div>
            {#if pairScanResult}
              <div class="api-test-result">{pairScanResult}</div>
            {/if}
            {#if pairVerifyResult}
              <div class="api-test-result">{pairVerifyResult}</div>
            {/if}
            {#if pairEvidencePath}
              <div class="pair-field">
                <span class="pair-label">Evidence log</span>
                <code class="pair-token">{pairEvidencePath}</code>
              </div>
            {/if}
            {#if pairLastEvidence}
              <div class="pair-field">
                <span class="pair-label">Last evidence</span>
                <code class="pair-token">{JSON.stringify(pairLastEvidence)}</code>
              </div>
            {/if}
          {/if}
          {#if pairError}
            <div class="pair-error">{pairError}</div>
          {/if}
        </div>
      </div>
    </section>

    <section class="section usb-lab-section">
      <h3 class="section-title">Android USB Lab</h3>
      <p class="section-desc">
        Plug in an Android tablet, click <strong>Refresh Devices</strong>, then follow the guided flow:
        Readiness → Install &amp; Launch → Bootstrap Connection → Run Full Validation.
      </p>

      <!-- ── Device picker + readiness card ── -->
      <div class="action-grid">
        <button class="action-btn" on:click={refreshAndroidUsbDevices} disabled={usbBusy}>
          {usbBusy ? 'Refreshing…' : 'Refresh Devices'}
        </button>
        <button class="action-btn" on:click={checkDeviceReadiness} disabled={usbBusy || !usbSelectedSerial}>
          Check Readiness
        </button>
      </div>

      <div class="form-group usb-form-group">
        <label for="usb-device">Device (adb serial)</label>
        <select id="usb-device" bind:value={usbSelectedSerial}>
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

      <!-- ── APK + package config ── -->
      <div class="form-group usb-form-group">
        <label for="apk-path">APK Path</label>
        <div class="input-with-btn">
          <input id="apk-path" type="text" bind:value={usbApkPath} placeholder="C:/path/to/app.apk or leave blank to auto-resolve" />
          <button class="action-btn" on:click={resolveApk} disabled={usbBusy}>Resolve</button>
        </div>
        {#if usbResolvedApk}
          <div class="apk-meta">
            {#if usbResolvedApk.package}<span>pkg: <code>{usbResolvedApk.package}</code></span>{/if}
            {#if usbResolvedApk.version_name}<span>v{usbResolvedApk.version_name}</span>{/if}
            <span>{Math.round(usbResolvedApk.size_bytes / 1024)} KB</span>
          </div>
        {/if}
      </div>

      <div class="form-group usb-form-group">
        <label for="pkg-name">Package Name</label>
        <input id="pkg-name" type="text" bind:value={usbPackageName} placeholder="com.bonsai.workspace" />
      </div>

      <div class="form-group usb-form-group">
        <label for="activity-name">Launch Activity (optional)</label>
        <input id="activity-name" type="text" bind:value={usbActivity} placeholder=".MainActivity" />
      </div>

      <div class="usb-toggle-row">
        <label class="toggle-label">
          <input type="checkbox" bind:checked={usbStrictMode} />
          Strict mode <span class="toggle-hint">(fail if app not installed or launch fails)</span>
        </label>
      </div>

      <!-- ── One-click flow buttons ── -->
      <div class="action-grid usb-flow-grid">
        <button class="action-btn green" on:click={installAndLaunch} disabled={usbBusy || !usbSelectedSerial}>
          Install &amp; Launch
        </button>
        <button class="action-btn blue" on:click={bootstrapConnection} disabled={usbBusy || !usbSelectedSerial}>
          Bootstrap Connection
        </button>
        <button class="action-btn green full-width" on:click={runUsbRegressionSuite} disabled={usbBusy || !usbSelectedSerial}>
          Run Full Validation
        </button>
      </div>

      <!-- ── Per-step result table ── -->
      {#if usbLastSteps.length > 0}
        <table class="usb-steps-table">
          <thead><tr><th>Step</th><th>Result</th><th>ms</th><th>Detail</th></tr></thead>
          <tbody>
            {#each usbLastSteps as s}
              <tr class={s.skipped ? 'step-skip' : s.ok ? 'step-ok' : 'step-fail'}>
                <td>{s.label}</td>
                <td>{s.skipped ? 'SKIP' : s.ok ? 'PASS' : 'FAIL'}</td>
                <td>{s.duration_ms ?? '—'}</td>
                <td class="step-detail">{s.hint || s.stderr || s.stdout || ''}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      {/if}

      <!-- ── WiFi bridge config ── -->
      <details class="usb-advanced">
        <summary>WiFi Bridge &amp; Advanced</summary>

        <div class="usb-toggle-row">
          <label class="toggle-label">
            <input type="checkbox" bind:checked={usbEnableWifiBridge} />
            Enable WiFi bridge in Bootstrap Connection
          </label>
        </div>

        <div class="usb-wifi-grid">
          <div class="form-group usb-form-group">
            <label for="wifi-host">Device WiFi Host/IP</label>
            <input id="wifi-host" type="text" bind:value={usbWifiHost} placeholder="192.168.1.120" />
          </div>
          <div class="form-group usb-form-group">
            <label for="wifi-port">WiFi Debug Port</label>
            <input id="wifi-port" type="number" min="1" max="65535" bind:value={usbWifiPort} />
          </div>
        </div>

        <div class="action-grid">
          <button class="action-btn" on:click={enableUsbWifiDebug} disabled={usbBusy}>Enable tcpip over USB</button>
          <button class="action-btn" on:click={connectUsbWifiDebug} disabled={usbBusy}>Connect WiFi Debug</button>
        </div>
        <div class="action-grid">
          <button class="action-btn" on:click={disconnectUsbWifiDebug} disabled={usbBusy}>Disconnect WiFi Debug</button>
          <button class="action-btn" on:click={clearUsbReverse} disabled={usbBusy}>Clear adb reverse</button>
        </div>

        <div class="form-group usb-form-group">
          <label for="usb-shell">ADB Shell Command</label>
          <input id="usb-shell" type="text" bind:value={usbShellCommand} placeholder="getprop ro.product.model" />
        </div>
        <div class="action-grid">
          <button class="action-btn blue" on:click={runUsbShellCommand} disabled={usbBusy}>Run Shell Command</button>
        </div>

        <div class="pair-field">
          <span class="pair-label">ADB executable</span>
          <code class="pair-token">{usbAdbExecutable || 'adb'}</code>
        </div>
        {#if usbAdbCandidates.length > 0}
          <div class="pair-field">
            <span class="pair-label">ADB candidates</span>
            <code class="pair-token">{usbAdbCandidates.join(' | ')}</code>
          </div>
        {/if}
      </details>

      {#if usbError}
        <div class="pair-error">{usbError}</div>
      {/if}
      {#if usbRegressionEvidencePath}
        <div class="pair-field">
          <span class="pair-label">Evidence file</span>
          <code class="pair-token">{usbRegressionEvidencePath}</code>
        </div>
      {/if}
      {#if usbRegressionLast}
        <div class="pair-field">
          <span class="pair-label">Last regression: {usbRegressionLast.ok ? '✓ PASS' : '✗ FAIL'}</span>
          <code class="pair-token">serial: {usbRegressionLast.serial} · strict: {String(usbRegressionLast.strict_require_app)}</code>
        </div>
      {/if}
      {#if usbResult}
        <pre class="usb-result">{usbResult}</pre>
      {/if}
    </section>

  </div>
</div>

<style>
  .settings-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0,0,0,0.6);
    z-index: 400;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .settings-panel {
    width: 520px;
    max-height: 80vh;
    overflow-y: auto;
    background: var(--bg2);
    border: 1px solid var(--border);
    border-radius: 12px;
    box-shadow: 0 20px 60px rgba(0,0,0,0.5);
    display: flex;
    flex-direction: column;
  }

  .settings-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 16px 20px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .settings-header h2 { font-size: 15px; font-weight: 600; }

  .close-btn {
    background: transparent;
    border: none;
    color: var(--text-dim);
    font-size: 16px;
    cursor: pointer;
    padding: 2px 6px;
    border-radius: 4px;
  }
  .close-btn:hover { background: var(--bg-hover); color: var(--text); }

  .section { padding: 16px 20px; border-bottom: 1px solid var(--border); }
  .section-title {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-dim);
    margin-bottom: 10px;
  }

  .hw-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 8px; }
  .hw-item--wide { grid-column: 1 / -1; }
  .hw-val--sm { font-size: 11px; font-weight: 400; color: var(--text-dim); }
  .hw-item {
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 8px 10px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .status-summary .status-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 8px;
  }
  .status-item {
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 10px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 10px;
    font-size: 12px;
  }
  .status-label { color: var(--text-dim); text-transform: uppercase; letter-spacing: 0.05em; font-size: 10px; }
  .status-value { font-weight: 600; }
  .slot-list {
    margin-top: 12px;
    display: grid;
    gap: 6px;
  }
  .slot-row {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 10px;
    padding: 10px 12px;
    border-radius: 8px;
    background: var(--bg);
    border: 1px solid var(--border);
    font-size: 12px;
  }
  .slot-state { font-weight: 600; color: var(--text); }
  .slot-info { color: var(--text-dim); }
  .switch-details .switch-log {
    padding: 10px 12px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 8px;
    color: var(--text);
    font-size: 12px;
    white-space: pre-wrap;
  }
  .hw-label { font-size: 10px; color: var(--text-dim); text-transform: uppercase; letter-spacing: 0.05em; }
  .hw-val   { font-size: 14px; font-weight: 600; }

  .model-list { display: flex; flex-direction: column; gap: 6px; margin-bottom: 12px; }
  .model-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 12px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 8px;
    transition: border-color 0.15s;
  }
  .model-row.active-model { border-color: var(--accent); }

  .remote-info {
    display: grid;
    gap: 8px;
    margin-top: 12px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 12px;
    font-size: 12px;
  }

  .remote-preview {
    margin-top: 12px;
    background: var(--bg);
    border: 1px dashed var(--border);
    border-radius: 10px;
    padding: 12px;
    display: grid;
    gap: 10px;
  }

  .remote-action-grid {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    margin-top: 12px;
  }

  .remote-preview-placeholder,
  .remote-error {
    color: var(--text-dim);
    font-size: 12px;
  }

  .remote-preview-img {
    width: 100%;
    border-radius: 10px;
    max-height: 320px;
    object-fit: contain;
    background: var(--bg2);
  }

  .copy-link {
    margin-left: 8px;
    border: 1px solid var(--border);
    background: transparent;
    color: var(--text);
    padding: 4px 8px;
    border-radius: 6px;
    cursor: pointer;
  }

  .copy-link:hover { background: var(--bg-hover); }

  .model-name { font-size: 13px; font-weight: 500; }
  .model-meta { font-size: 11px; color: var(--text-dim); margin-top: 2px; }

  .badge-default {
    font-size: 9px;
    background: rgba(251,191,36,0.15);
    color: #fbbf24;
    border: 1px solid rgba(251,191,36,0.3);
    border-radius: 4px;
    padding: 1px 5px;
    margin-left: 5px;
    vertical-align: middle;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .badge-notlocal {
    font-size: 9px;
    background: rgba(239,68,68,0.12);
    color: var(--red);
    border: 1px solid rgba(239,68,68,0.25);
    border-radius: 4px;
    padding: 1px 5px;
    margin-left: 5px;
    vertical-align: middle;
  }

  .btn-dl {
    background: rgba(251,191,36,0.1) !important;
    border-color: rgba(251,191,36,0.4) !important;
    color: #fbbf24 !important;
  }
  .btn-dl:hover { background: rgba(251,191,36,0.2) !important; }

  .badge-active {
    font-size: 11px;
    background: var(--accent);
    color: #fff;
    padding: 2px 8px;
    border-radius: 10px;
  }

  .btn-sm {
    font-size: 12px;
    background: var(--bg2);
    border: 1px solid var(--border);
    color: var(--text);
    padding: 3px 10px;
    border-radius: 5px;
    cursor: pointer;
  }
  .btn-sm:hover { background: var(--bg-hover); }
  .btn-sm:disabled { opacity: 0.5; cursor: not-allowed; }

  .action-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 6px; }
  .pair-actions { margin-top: 6px; }
  .action-btn {
    padding: 8px 12px;
    border: none;
    border-radius: 7px;
    font-size: 12px;
    cursor: pointer;
    transition: opacity 0.15s;
    color: #fff;
  }
  .action-btn.green { background: #16a34a; }
  .action-btn.blue  { background: var(--accent); }
  .action-btn:hover { opacity: 0.85; }
  .action-btn:disabled { opacity: 0.5; cursor: not-allowed; }

  .error-bar {
    background: var(--red);
    color: #fff;
    font-size: 12px;
    padding: 6px 16px;
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .error-bar button { background: transparent; border: none; color: #fff; cursor: pointer; }

  .loading-bar {
    background: var(--accent);
    color: #fff;
    font-size: 12px;
    padding: 6px 16px;
    animation: pulse 1.2s infinite;
  }

  .api-settings .form-group {
    display: grid;
    gap: 6px;
    margin-bottom: 12px;
  }
  .api-settings label {
    font-size: 11px;
    color: var(--text-dim);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .api-settings input {
    width: 100%;
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 10px 12px;
    background: var(--bg);
    color: var(--text);
  }
  .api-settings .form-note {
    color: var(--text-dim);
    font-size: 11px;
    margin-top: -6px;
    margin-bottom: 12px;
  }
  .api-test-result {
    margin-top: 10px;
    padding: 10px 12px;
    border-radius: 8px;
    background: rgba(59,130,246,0.08);
    border: 1px solid rgba(59,130,246,0.2);
    color: #fff;
    font-size: 12px;
  }
  .api-endpoint {
    margin-top: 12px;
    color: var(--text-dim);
    font-size: 11px;
    word-break: break-all;
  }

  @keyframes pulse { 0%,100%{opacity:1} 50%{opacity:0.7} }

  .usb-lab-section .section-desc {
    font-size: 12px;
    color: var(--text-dim);
    margin-bottom: 12px;
    line-height: 1.5;
  }

  .usb-form-group {
    margin-top: 10px;
  }

  .usb-form-group label {
    font-size: 11px;
    color: var(--text-dim);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    margin-bottom: 6px;
    display: block;
  }

  .usb-form-group input,
  .usb-form-group select {
    width: 100%;
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 10px 12px;
    background: var(--bg);
    color: var(--text);
  }

  .usb-wifi-grid {
    display: grid;
    grid-template-columns: 1fr 140px;
    gap: 8px;
  }

  .usb-result {
    margin-top: 10px;
    max-height: 180px;
    overflow: auto;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 10px;
    font-size: 11px;
    color: var(--text);
    white-space: pre-wrap;
    word-break: break-word;
  }

  /* ── USB Lab Runtime System ── */
  .usb-readiness-card {
    margin: 10px 0;
    padding: 10px 14px;
    border-radius: 8px;
    border: 1px solid var(--border);
    background: var(--bg);
    font-size: 12px;
  }
  .usb-readiness-card.status-ready   { border-color: #3c8; background: rgba(51,204,136,.08); }
  .usb-readiness-card.status-online  { border-color: #fa0; background: rgba(255,170,0,.08); }
  .usb-readiness-card.status-unauthorized { border-color: #f66; background: rgba(255,80,80,.08); }
  .usb-readiness-card.status-disconnected { border-color: var(--border); }

  .readiness-badge {
    display: inline-block;
    font-size: 10px;
    font-weight: 700;
    letter-spacing: .06em;
    padding: 2px 7px;
    border-radius: 4px;
    margin-bottom: 6px;
    background: rgba(128,128,128,.15);
  }
  .status-ready   .readiness-badge { background: rgba(51,204,136,.25); color: #2a9; }
  .status-online  .readiness-badge { background: rgba(255,170,0,.25);  color: #c80; }
  .status-unauthorized .readiness-badge { background: rgba(255,80,80,.2); color: #d44; }

  .readiness-detail { line-height: 1.7; color: var(--text-dim); }
  .readiness-detail code { font-size: 11px; }
  .readiness-next-action {
    margin-top: 6px;
    font-size: 11px;
    color: var(--text-dim);
    font-style: italic;
  }

  .input-with-btn {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .input-with-btn input { flex: 1; }
  .input-with-btn .action-btn { white-space: nowrap; flex-shrink: 0; }

  .apk-meta {
    display: flex;
    gap: 10px;
    flex-wrap: wrap;
    font-size: 11px;
    color: var(--text-dim);
    margin-top: 4px;
  }
  .apk-meta code { font-size: 11px; }

  .usb-toggle-row {
    margin: 8px 0;
  }
  .toggle-label {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    cursor: pointer;
  }
  .toggle-hint { color: var(--text-dim); font-size: 11px; }

  .usb-flow-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
    margin: 10px 0;
  }
  .usb-flow-grid .full-width { grid-column: 1 / -1; }

  .usb-steps-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 11px;
    margin: 10px 0;
  }
  .usb-steps-table th {
    text-align: left;
    padding: 4px 6px;
    border-bottom: 1px solid var(--border);
    color: var(--text-dim);
    font-weight: 600;
  }
  .usb-steps-table td {
    padding: 3px 6px;
    border-bottom: 1px solid rgba(128,128,128,.1);
    vertical-align: top;
  }
  .usb-steps-table tr.step-ok  td:first-child { color: #3c8; }
  .usb-steps-table tr.step-fail td:first-child { color: #f66; }
  .usb-steps-table tr.step-skip td:first-child { color: var(--text-dim); }
  .step-detail { max-width: 180px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--text-dim); }

  .usb-advanced {
    margin: 12px 0 4px;
    font-size: 12px;
  }
  .usb-advanced summary {
    cursor: pointer;
    color: var(--text-dim);
    font-size: 11px;
    padding: 4px 0;
    user-select: none;
  }
  .usb-advanced summary:hover { color: var(--text); }

  /* ── Connection / pairing ── */
  .connection-section .section-desc {
    font-size: 12px;
    color: var(--text-dim);
    margin-bottom: 14px;
    line-height: 1.5;
  }

  .pair-row {
    display: flex;
    gap: 20px;
    align-items: flex-start;
    flex-wrap: wrap;
  }

  .qr-area {
    flex-shrink: 0;
    width: 160px;
    height: 160px;
    background: #fff;
    border-radius: 10px;
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
  }

  .qr-area :global(svg) {
    width: 100%;
    height: 100%;
  }

  .pair-info {
    display: flex;
    flex-direction: column;
    gap: 10px;
    flex: 1;
    min-width: 180px;
  }

  .pair-field {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .pair-label {
    font-size: 11px;
    color: var(--text-dim);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .pair-token {
    font-family: 'Cascadia Code', 'Fira Code', monospace;
    font-size: 13px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 4px 8px;
    word-break: break-all;
  }

  .pair-error {
    color: var(--red);
    font-size: 12px;
  }
</style>
