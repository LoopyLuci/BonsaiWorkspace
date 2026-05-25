<script lang="ts">
  import { createEventDispatcher, onMount, onDestroy } from 'svelte';
  import { get } from 'svelte/store';
  import { invoke } from '@tauri-apps/api/core';
  import { scan } from '@tauri-apps/plugin-barcode-scanner';
  import { addAssistantMessage } from '$lib/stores/chat';
  import DOMPurify from 'dompurify';
  import ClusterControlPanel from '$lib/components/ClusterControlPanel.svelte';
  import { DEFAULT_API_PORT } from '$lib/constants/network';
  import {
    availableModels,
    activeModel,
    activeModelId,
    orchestratorStatus,
    refreshStatus,
    refreshModels,
    modelSwitchStatus,
    setModelSwitchStatus,
    defaultInferenceMode,
    refreshDefaultInferenceMode,
    setDefaultInferenceMode,
    applyInferenceModeToAll,
  } from '$lib/stores/models';
  import type { InferenceMode } from '$lib/types/inference_mode';
  import { inferenceModeLabel, toInferenceMode } from '$lib/types/inference_mode';
  import { apiHost, apiPort, apiBaseUrl, loadApiSettings, saveApiSettings } from '$lib/stores/settings';
  import {
    applyAutoDetectedMobileDisplaySettings,
    applyMobileDisplayPreview,
    confirmMobileDisplaySettings,
    mobileDisplayPending,
    mobileDisplaySettings,
    resetMobileDisplaySettings,
    revertUnconfirmedMobileDisplaySettings,
  } from '$lib/stores/mobileDisplay';
  import {
    BONSAI_CATALOG, findRegistryModel,
    downloadCatalogModel, downloadingId, downloadPct, downloadError,
  } from '$lib/stores/catalog';
  import { featureFlags, loadFeatureFlags } from '$lib/stores/features';
  import TrainingDashboard from '$lib/components/TrainingDashboard.svelte';

  const dispatch = createEventDispatcher<{ close: void }>();

  let hwInfo:          Record<string, unknown> = {};
  let loadingOp        = '';
  let errorMsg         = '';
  let switchDetails    = '';
  let apiTestResult    = '';
  let displaySettingsStatus = '';
  let apiTestLoading   = false;
  let saveApiLoading   = false;
  let defaultInferenceModeKey: 'auto' | 'cpu_only' | 'gpu_only' | 'hybrid' = 'hybrid';
  let defaultHybridLayers = 20;
  let inferenceDefaultsMsg = '';

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
    try {
      await refreshDefaultInferenceMode();
      defaultInferenceModeKey = $defaultInferenceMode.mode;
      if ($defaultInferenceMode.mode === 'hybrid') {
        defaultHybridLayers = $defaultInferenceMode.gpu_layers;
      }
    } catch {}
    try { await refreshAndroidUsbDevices(); } catch {}
    refreshBotStatus();
    botStatusInterval = setInterval(refreshBotStatus, 30_000);
    try { await loadFeatureFlags(); } catch {}
  });

  function sliderValue(event: Event): number {
    const target = event.currentTarget as HTMLInputElement | null;
    return Number(target?.value ?? 0);
  }

  function previewDisplayPatch(patch: {
    topOffsetPx?: number;
    bottomOffsetPx?: number;
    leftOffsetPx?: number;
    rightOffsetPx?: number;
  }) {
    applyMobileDisplayPreview(patch, { source: 'manual' });
    displaySettingsStatus = 'Screen adjustment preview started. Confirm within 30 seconds to keep.';
  }

  function autoDetectScreenSize() {
    const detected = applyAutoDetectedMobileDisplaySettings();
    displaySettingsStatus = `Auto-detected insets: top ${detected.topOffsetPx}px, bottom ${detected.bottomOffsetPx}px, left ${detected.leftOffsetPx}px, right ${detected.rightOffsetPx}px.`;
  }

  function keepScreenChanges() {
    confirmMobileDisplaySettings();
    displaySettingsStatus = 'Screen adjustment confirmed and saved.';
  }

  function revertScreenChanges() {
    revertUnconfirmedMobileDisplaySettings();
    displaySettingsStatus = 'Unconfirmed screen changes reverted to the last confirmed layout.';
  }

  function resetDisplayDefaults() {
    resetMobileDisplaySettings();
    displaySettingsStatus = 'Display offsets reset to defaults.';
  }

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
      setModelSwitchStatus(switchDetails, 5000);
      addAssistantMessage(msg);
    } catch (e) {
      errorMsg = String(e);
      if (/model load timeout/i.test(errorMsg)) {
        switchDetails = `Switch timed out. Waiting for ${name} to become ready...`;
        modelSwitchStatus.set(switchDetails);
        const deadline = Date.now() + 180000;
        while (Date.now() < deadline) {
          await refreshStatus();
          const status = get(orchestratorStatus);
          const isReady = status?.slots.some((slot) =>
            slot.state.state === 'ready' && slot.state.model_id === modelId,
          );
          if (isReady) {
            await refreshModels();
            switchDetails = `${name} became ready after timeout grace window.`;
            setModelSwitchStatus(switchDetails, 5000);
            errorMsg = '';
            return;
          }
          await new Promise((resolve) => setTimeout(resolve, 1500));
        }
      }
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

  async function saveInferenceDefaults() {
    const mode: InferenceMode = toInferenceMode(defaultInferenceModeKey, defaultHybridLayers);
    const saved = await setDefaultInferenceMode(mode);
    inferenceDefaultsMsg = saved ? `Default set to ${inferenceModeLabel(saved)}` : 'Failed to save default inference mode';
  }

  async function applyInferenceDefaultsToAllModels() {
    const mode: InferenceMode = toInferenceMode(defaultInferenceModeKey, defaultHybridLayers);
    const updated = await applyInferenceModeToAll(mode);
    inferenceDefaultsMsg = updated > 0
      ? `Applied ${inferenceModeLabel(mode)} to ${updated} model(s)`
      : 'No models were updated';
    await refreshModels();
    await refreshStatus();
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

  let showAdvanced = false;
  let showTrainingDashboard = false;

  // ── Training: local model selection ──────────────────────────────────────
  let selectedGgufPath = '';

  async function browseGguf(): Promise<void> {
    try {
      const { open } = await import('@tauri-apps/plugin-dialog');
      const selected = await open({
        title: 'Select local GGUF model',
        filters: [{ name: 'GGUF model', extensions: ['gguf'] }],
        multiple: false,
        directory: false,
      });
      if (typeof selected === 'string') selectedGgufPath = selected;
    } catch {
      // plugin-dialog unavailable in some build configs — let user type path
    }
  }

  // ── Training Monitor state ────────────────────────────────────────────────
  type TrainingStatus = 'idle' | 'running' | 'completed' | 'failed';
  interface TrainProgress {
    status: TrainingStatus;
    epoch: string;
    step: number;
    totalSteps: number;
    loss: string;
    pct: number;
    elapsed: string;
    eta: string;
    device: string;
    dtype: string;
    examples: number;
    curatedMerged: number;
  }

  let trainingStatus: TrainingStatus = 'idle';
  let trainingLog = '';
  let trainProgress: TrainProgress = {
    status: 'idle', epoch: '0', step: 0, totalSteps: 0,
    loss: '—', pct: 0, elapsed: '0s', eta: '—',
    device: '—', dtype: '—', examples: 0, curatedMerged: 0,
  };
  let pollInterval: ReturnType<typeof setInterval> | null = null;
  let coreStats: Record<string, unknown> = {};

  type HistoryRun = {
    id: string; started_at: number; finished_at: number | null;
    base_model: string; status: string; metrics: string | null;
    total_examples: number | null; curated_examples: number | null;
  };
  let runHistory: HistoryRun[] = [];
  let showHistory = false;

  function parseTrainLine(line: string): void {
    // Parse structured [tag] key=val lines emitted by finetune.py
    const m = line.match(/^\[(\w+)\]\s+(.+)$/);
    if (!m) return;
    const [, tag, rest] = m;
    const kv: Record<string, string> = {};
    for (const part of rest.matchAll(/(\w+)=(\S+)/g)) kv[part[1]] = part[2];

    if (tag === 'device') {
      trainProgress = { ...trainProgress, device: kv.using ?? '—', dtype: kv.dtype ?? '—' };
    } else if (tag === 'train' && kv.status === 'starting') {
      trainProgress = { ...trainProgress, examples: Number(kv.examples ?? 0),
        curatedMerged: 0, step: 0, pct: 0, loss: '—' };
    } else if (tag === 'progress') {
      trainProgress = {
        ...trainProgress,
        step: Number(kv.step ?? 0),
        totalSteps: Number(kv.total ?? trainProgress.totalSteps),
        epoch: kv.epoch ?? trainProgress.epoch,
        loss: kv.loss ?? trainProgress.loss,
        pct: parseFloat(kv.pct ?? '0'),
        elapsed: kv.elapsed ?? trainProgress.elapsed,
        eta: kv.eta ?? trainProgress.eta,
      };
    } else if (tag === 'save') {
      trainProgress = { ...trainProgress, pct: 100, eta: '0s', status: 'completed' };
      trainingStatus = 'completed';
    } else if (tag === 'data') {
      if (kv.curated) trainProgress = { ...trainProgress, curatedMerged: Number(kv.curated) };
    }
  }

  async function startTrainingMonitor(): Promise<void> {
    if (!selectedGgufPath) {
      trainingLog = '[error] Select a local .gguf model file first.';
      return;
    }
    trainingStatus = 'running';
    trainingLog = `[offline] model=${selectedGgufPath}\n`;
    trainProgress = { ...trainProgress, status: 'running', step: 0, pct: 0, loss: '—', eta: '—' };
    startStatsPoll();
    try {
      const adapterPath = await invoke<string>('start_training_cycle', {
        modelPath: selectedGgufPath,
        dataPath: 'data/bonsai_core/bonsai_core_train_v2.jsonl',
        outputPath: null,
      });
      trainingLog += `\n[save] path=${adapterPath}`;
      parseTrainLine(`[save] path=${adapterPath}`);
      trainingStatus = 'completed';
    } catch (e: unknown) {
      trainingLog += `\n[error] ${String(e)}`;
      trainingStatus = 'failed';
      trainProgress = { ...trainProgress, status: 'failed' };
    } finally {
      stopStatsPoll();
    }
  }

  function startStatsPoll(): void {
    if (pollInterval) return;
    pollInterval = setInterval(async () => {
      try {
        const cfg = await invoke<{ api_port: number; pair_token: string }>('get_api_config');
        const res = await fetch(`http://127.0.0.1:${cfg.api_port}/api/v1/core/stats`, {
          headers: { Authorization: `Bearer ${cfg.pair_token}` },
        });
        if (res.ok) coreStats = await res.json();
      } catch { /* stats are best-effort */ }
    }, 2000);
  }

  function stopStatsPoll(): void {
    if (pollInterval) { clearInterval(pollInterval); pollInterval = null; }
  }

  async function loadRunHistory(): Promise<void> {
    try {
      const cfg = await invoke<{ api_port: number; pair_token: string }>('get_api_config');
      const res = await fetch(`http://127.0.0.1:${cfg.api_port}/api/v1/telemetry/training`, {
        headers: { Authorization: `Bearer ${cfg.pair_token}` },
      });
      if (res.ok) runHistory = await res.json();
    } catch { /* offline */ }
  }

  function fmtTs(ms: number): string {
    return ms ? new Date(ms).toLocaleString() : '—';
  }
  function statusColor(s: string): string {
    return s === 'completed' ? '#4caf50' : s === 'failed' ? '#e57373' : s === 'running' ? '#64b5f6' : '#888';
  }

  function getFlagValue(key: string): boolean {
    return !!($featureFlags as unknown as Record<string, boolean>)[key];
  }

  function onFlagChange(key: string, e: Event): void {
    toggleFlag(key, (e.currentTarget as HTMLInputElement).checked);
  }

  async function toggleFlag(key: string, value: boolean) {
    const flags = $featureFlags as unknown as Record<string, boolean>;
    flags[key] = value;
    featureFlags.set($featureFlags);
    await invoke('set_feature_flags', { flags: $featureFlags });
  }

  onDestroy(() => {
    disconnectRemotePreview();
    if (botStatusInterval) clearInterval(botStatusInterval);
    stopStatsPoll();
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
      const out = await withInvokeTimeout(runner(), 25000, title);
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

  // ── Messaging Bots ──────────────────────────────────────────────────────────

  type BotPlatformStatus = { connected: boolean; error?: string };
  type BotStatus = { running: boolean; platforms: Record<string, BotPlatformStatus> };

  let botStatus: BotStatus = { running: false, platforms: {} };
  let botStatusInterval: ReturnType<typeof setInterval> | null = null;

  // Discord form state
  let discordToken = '';
  let discordGuildIds = '';
  let discordChannelIds = '';
  let discordUserIds = '';

  // Telegram form state
  let telegramToken = '';
  let telegramChatIds = '';

  // Email form state
  let emailImapPassword = '';
  let emailSmtpPassword = '';
  let emailImapHost = '';
  let emailImapPort = 993;
  let emailImapUsername = '';
  let emailSmtpHost = '';
  let emailSmtpUsername = '';
  let emailSmtpFrom = '';
  let emailSubjectPrefix = '[BONSAI]';
  let emailAllowedFrom = '';

  let botSaveMsg = '';

  async function refreshBotStatus() {
    try {
      const s = await invoke<Record<string, unknown>>('get_bot_server_status');
      const raw = (s['platforms'] as Record<string, unknown>) || {};
      const platforms: Record<string, BotPlatformStatus> = {};
      for (const [k, v] of Object.entries(raw)) {
        const vs = typeof v === 'string' ? v : String(v);
        platforms[k] = { connected: vs === 'connected', error: vs === 'connected' ? undefined : vs };
      }
      botStatus = { running: true, platforms };
    } catch {
      botStatus = { running: false, platforms: {} };
    }
  }

  function parseCsvList(s: string): string[] {
    return s.split(',').map(x => x.trim()).filter(Boolean);
  }

  async function saveDiscordConfig() {
    botSaveMsg = '';
    try {
      await invoke('save_discord_bot_config', {
        token: discordToken,
        allowedGuildIds:   parseCsvList(discordGuildIds),
        allowedChannelIds: parseCsvList(discordChannelIds),
        allowedUserIds:    parseCsvList(discordUserIds),
      });
      botSaveMsg = 'Discord settings saved.';
      await refreshBotStatus();
    } catch (e) {
      botSaveMsg = `Error: ${e}`;
    }
  }

  async function saveTelegramConfig() {
    botSaveMsg = '';
    try {
      await invoke('save_telegram_bot_config', {
        token: telegramToken,
        allowedChatIds: parseCsvList(telegramChatIds).map(Number),
      });
      botSaveMsg = 'Telegram settings saved.';
      await refreshBotStatus();
    } catch (e) {
      botSaveMsg = `Error: ${e}`;
    }
  }

  async function saveEmailConfig() {
    botSaveMsg = '';
    try {
      await invoke('save_email_bot_config', {
        imapPassword:      emailImapPassword,
        smtpPassword:      emailSmtpPassword,
        imapHost:          emailImapHost,
        imapPort:          Number(emailImapPort),
        imapUsername:      emailImapUsername,
        smtpHost:          emailSmtpHost,
        smtpUsername:      emailSmtpUsername,
        smtpFrom:          emailSmtpFrom,
        subjectPrefix:     emailSubjectPrefix,
        allowedFromAddrs:  parseCsvList(emailAllowedFrom),
      });
      botSaveMsg = 'Email settings saved.';
      await refreshBotStatus();
    } catch (e) {
      botSaveMsg = `Error: ${e}`;
    }
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

    <section class="section">
      <h3 class="section-title">Cluster Control</h3>
      <ClusterControlPanel />
    </section>

    <section class="section api-settings">
      <h3 class="section-title">API Settings</h3>
      <div class="form-group">
        <label for="api-host">API Host</label>
        <input id="api-host" type="text" data-bonsai-action="Settings:ApiHost" style="-webkit-app-region: no-drag;" bind:value={$apiHost} />
      </div>
      <div class="form-group">
        <label for="api-port">API Port</label>
        <input id="api-port" type="number" min="1" max="65535" data-bonsai-action="Settings:ApiPort" style="-webkit-app-region: no-drag;" bind:value={$apiPort} />
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

    <section class="section mobile-display-settings">
      <h3 class="section-title">Mobile Screen Adjustment</h3>
      <div class="form-note">
        Auto-detect viewport insets, then fine-tune offsets safely with a 30-second confirmation rollback.
      </div>

      <div class="action-grid">
        <button class="action-btn" type="button" on:click={autoDetectScreenSize}>Auto detect screen size</button>
        <button class="action-btn green" type="button" on:click={keepScreenChanges} disabled={!$mobileDisplayPending.isPending}>Keep screen changes</button>
        <button class="action-btn red" type="button" on:click={revertScreenChanges} disabled={!$mobileDisplayPending.isPending}>Revert now</button>
        <button class="action-btn" type="button" on:click={resetDisplayDefaults}>Reset display offsets</button>
      </div>

      {#if $mobileDisplayPending.isPending}
        <div class="display-warning">
          Preview active ({$mobileDisplayPending.source}). Reverting in {$mobileDisplayPending.secondsLeft}s unless confirmed.
        </div>
      {/if}

      {#if displaySettingsStatus}
        <div class="display-status">{displaySettingsStatus}</div>
      {/if}

      <div class="range-grid">
        <label>
          Top offset ({$mobileDisplaySettings.topOffsetPx}px)
          <input
            type="range"
            min="-24"
            max="72"
            step="1"
            value={$mobileDisplaySettings.topOffsetPx}
            on:input={(e) => previewDisplayPatch({ topOffsetPx: sliderValue(e) })}
          />
        </label>
        <label>
          Bottom offset ({$mobileDisplaySettings.bottomOffsetPx}px)
          <input
            type="range"
            min="-24"
            max="96"
            step="1"
            value={$mobileDisplaySettings.bottomOffsetPx}
            on:input={(e) => previewDisplayPatch({ bottomOffsetPx: sliderValue(e) })}
          />
        </label>
        <label>
          Left offset ({$mobileDisplaySettings.leftOffsetPx}px)
          <input
            type="range"
            min="-24"
            max="48"
            step="1"
            value={$mobileDisplaySettings.leftOffsetPx}
            on:input={(e) => previewDisplayPatch({ leftOffsetPx: sliderValue(e) })}
          />
        </label>
        <label>
          Right offset ({$mobileDisplaySettings.rightOffsetPx}px)
          <input
            type="range"
            min="-24"
            max="48"
            step="1"
            value={$mobileDisplaySettings.rightOffsetPx}
            on:input={(e) => previewDisplayPatch({ rightOffsetPx: sliderValue(e) })}
          />
        </label>
      </div>
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

    <section class="section">
      <h3 class="section-title">Inference Defaults</h3>
      <p class="section-desc">Choose the default mode for newly discovered local models, then optionally apply it to all existing models.</p>
      <div class="api-config-grid">
        <label>
          Default mode
          <select bind:value={defaultInferenceModeKey}>
            <option value="auto">Auto</option>
            <option value="hybrid">Hybrid</option>
            <option value="gpu_only">GPU Only</option>
            <option value="cpu_only">CPU Only</option>
          </select>
        </label>
        {#if defaultInferenceModeKey === 'hybrid'}
          <label>
            GPU layers
            <input type="number" min="1" max="200" bind:value={defaultHybridLayers} />
          </label>
        {/if}
      </div>
      <div class="action-grid">
        <button class="action-btn blue" type="button" on:click={saveInferenceDefaults}>
          Save Default
        </button>
        <button class="action-btn" type="button" on:click={applyInferenceDefaultsToAllModels}>
          Apply To All Models
        </button>
      </div>
      {#if inferenceDefaultsMsg}
        <div class="api-test-result">{inferenceDefaultsMsg}</div>
      {/if}
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
            {@html DOMPurify.sanitize(pairQrSvg, { USE_PROFILES: { svg: true, svgFilters: true } })}
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
      <p class="section-desc">Android USB Lab is now a standalone window. Open the lab to run device readiness, install, bootstrap, and validation flows.</p>
      <div class="action-grid">
        <button class="action-btn" on:click={() => dispatch('openAndroidUsbLab')}>
          Open Android USB Lab
        </button>
        <button class="action-btn" on:click={() => dispatch('openAndroidUsbLab')}>Open Lab (alt)</button>
      </div>
    </section>

    <!-- ── Messaging Bots ─────────────────────────────────────────────────── -->
    <section class="section bots-section">
      <h3 class="section-title">
        Messaging Bots
        <span class="bot-status-badge" class:connected={botStatus.running}>
          {botStatus.running ? '● running' : '○ not running'}
        </span>
        <button class="refresh-bot" on:click={refreshBotStatus} title="Refresh bot status">↺</button>
      </h3>
      <p class="section-desc">Connect Bonsai to Discord, Telegram, and Email. Tokens are stored in the OS keychain.</p>

      {#if botSaveMsg}
        <div class="bot-save-msg">{botSaveMsg}</div>
      {/if}

      <!-- Discord -->
      <details class="bot-platform-details">
        <summary>
          Discord
          {#if botStatus.platforms['discord']?.connected}
            <span class="platform-badge ok">● Connected</span>
          {:else if botStatus.running}
            <span class="platform-badge err">○ Disconnected</span>
          {/if}
        </summary>
        <div class="bot-form">
          <label class="bot-label">Bot Token <span class="secret-hint">(write-only)</span>
            <input class="bot-input" type="password" data-bonsai-action="Settings:DiscordToken" style="-webkit-app-region: no-drag;" placeholder="Paste new token to update" bind:value={discordToken} autocomplete="off"/>
          </label>
          <label class="bot-label">Guild IDs (comma-separated, empty = any)
            <input class="bot-input" type="text" data-bonsai-action="Settings:DiscordGuildIds" style="-webkit-app-region: no-drag;" placeholder="123456789012345678, ..." bind:value={discordGuildIds}/>
          </label>
          <label class="bot-label">Channel IDs (comma-separated, empty = any)
            <input class="bot-input" type="text" data-bonsai-action="Settings:DiscordChannelIds" style="-webkit-app-region: no-drag;" placeholder="optional" bind:value={discordChannelIds}/>
          </label>
          <label class="bot-label">User IDs (comma-separated, empty = any)
            <input class="bot-input" type="text" data-bonsai-action="Settings:DiscordUserIds" style="-webkit-app-region: no-drag;" placeholder="optional" bind:value={discordUserIds}/>
          </label>
          <button class="save-btn" on:click={saveDiscordConfig}>Save Discord</button>
        </div>
      </details>

      <!-- Telegram -->
      <details class="bot-platform-details">
        <summary>
          Telegram
          {#if botStatus.platforms['telegram']?.connected}
            <span class="platform-badge ok">● Connected</span>
          {:else if botStatus.running}
            <span class="platform-badge err">○ Disconnected</span>
          {/if}
        </summary>
        <div class="bot-form">
          <label class="bot-label">Bot Token <span class="secret-hint">(write-only)</span>
            <input class="bot-input" type="password" data-bonsai-action="Settings:TelegramToken" style="-webkit-app-region: no-drag;" placeholder="Paste new token to update" bind:value={telegramToken} autocomplete="off"/>
          </label>
          <label class="bot-label">Allowed Chat IDs (comma-separated)
            <input class="bot-input" type="text" data-bonsai-action="Settings:TelegramChatIds" style="-webkit-app-region: no-drag;" placeholder="-100123456789, ..." bind:value={telegramChatIds}/>
          </label>
          <button class="save-btn" on:click={saveTelegramConfig}>Save Telegram</button>
        </div>
      </details>

      <!-- Email -->
      <details class="bot-platform-details">
        <summary>
          Email (IMAP + SMTP)
          {#if botStatus.platforms['email']?.connected}
            <span class="platform-badge ok">● Connected</span>
          {:else if botStatus.running}
            <span class="platform-badge err">○ Disconnected</span>
          {/if}
        </summary>
        <div class="bot-form">
          <label class="bot-label">IMAP Host
            <input class="bot-input" type="text" data-bonsai-action="Settings:EmailImapHost" style="-webkit-app-region: no-drag;" placeholder="imap.gmail.com" bind:value={emailImapHost}/>
          </label>
          <label class="bot-label">IMAP Port
            <input class="bot-input" type="number" data-bonsai-action="Settings:EmailImapPort" style="-webkit-app-region: no-drag;" placeholder="993" bind:value={emailImapPort}/>
          </label>
          <label class="bot-label">IMAP Username
            <input class="bot-input" type="text" data-bonsai-action="Settings:EmailImapUsername" style="-webkit-app-region: no-drag;" placeholder="you@gmail.com" bind:value={emailImapUsername}/>
          </label>
          <label class="bot-label">IMAP Password <span class="secret-hint">(write-only)</span>
            <input class="bot-input" type="password" data-bonsai-action="Settings:EmailImapPassword" style="-webkit-app-region: no-drag;" placeholder="Paste new password to update" bind:value={emailImapPassword} autocomplete="off"/>
          </label>
          <label class="bot-label">SMTP Host
            <input class="bot-input" type="text" data-bonsai-action="Settings:EmailSmtpHost" style="-webkit-app-region: no-drag;" placeholder="smtp.gmail.com" bind:value={emailSmtpHost}/>
          </label>
          <label class="bot-label">SMTP Username
            <input class="bot-input" type="text" data-bonsai-action="Settings:EmailSmtpUsername" style="-webkit-app-region: no-drag;" placeholder="you@gmail.com" bind:value={emailSmtpUsername}/>
          </label>
          <label class="bot-label">SMTP From
            <input class="bot-input" type="text" data-bonsai-action="Settings:EmailSmtpFrom" style="-webkit-app-region: no-drag;" placeholder="Bonsai &lt;you@gmail.com&gt;" bind:value={emailSmtpFrom}/>
          </label>
          <label class="bot-label">SMTP Password <span class="secret-hint">(write-only)</span>
            <input class="bot-input" type="password" data-bonsai-action="Settings:EmailSmtpPassword" style="-webkit-app-region: no-drag;" placeholder="Paste new password to update" bind:value={emailSmtpPassword} autocomplete="off"/>
          </label>
          <label class="bot-label">Subject Prefix
            <input class="bot-input" type="text" data-bonsai-action="Settings:EmailSubjectPrefix" style="-webkit-app-region: no-drag;" placeholder="[BONSAI]" bind:value={emailSubjectPrefix}/>
          </label>
          <label class="bot-label">Allowed From Addresses (comma-separated)
            <input class="bot-input" type="text" data-bonsai-action="Settings:EmailAllowedFrom" style="-webkit-app-region: no-drag;" placeholder="alice@example.com, bob@example.com" bind:value={emailAllowedFrom}/>
          </label>
          <button class="save-btn" on:click={saveEmailConfig}>Save Email</button>
        </div>
      </details>
    </section>

    <section class="section feature-flags-section">
      <button class="section-title feature-flags-toggle" type="button" on:click={() => (showAdvanced = !showAdvanced)}>
        Advanced {showAdvanced ? '▲' : '▼'}
      </button>
      {#if showAdvanced}
        <h4 class="flags-heading">Feature Flags</h4>
        <div class="flags-grid">
          {#each Object.keys($featureFlags) as key}
            <label class="flag-row">
              <span class="flag-key">{key.replace(/_/g, ' ')}</span>
              <input
                type="checkbox"
                checked={getFlagValue(key)}
                on:change={(e) => onFlagChange(key, e)}
              />
            </label>
          {/each}
        </div>

        <div class="training-section">
          <h4 class="flags-heading">BonsAI-Core Training Monitor</h4>

          <!-- Status bar -->
          <div class="tm-status-bar">
            <span class="tm-status-dot" style="background:{statusColor(trainingStatus)}"></span>
            <span class="tm-status-label">{trainingStatus.toUpperCase()}</span>
            {#if trainingStatus === 'running'}
              <span class="tm-device-badge">{trainProgress.device}</span>
              <span class="tm-device-badge">{trainProgress.dtype}</span>
            {/if}
          </div>

          <!-- Progress bar -->
          {#if trainingStatus === 'running' || trainingStatus === 'completed'}
            <div class="tm-progress-track">
              <div class="tm-progress-fill" style="width:{trainProgress.pct}%"></div>
            </div>
            <div class="tm-metrics-row">
              <span>Epoch {trainProgress.epoch}</span>
              <span>Step {trainProgress.step}{trainProgress.totalSteps ? `/${trainProgress.totalSteps}` : ''}</span>
              <span class="tm-loss">Loss {trainProgress.loss}</span>
              <span>{trainProgress.pct.toFixed(1)}%</span>
              <span>⏱ {trainProgress.elapsed}</span>
              {#if trainProgress.eta !== '0s' && trainProgress.eta !== '—'}
                <span>ETA {trainProgress.eta}</span>
              {/if}
            </div>
            {#if trainProgress.examples > 0}
              <div class="tm-data-row">
                <span>{trainProgress.examples} examples</span>
                {#if trainProgress.curatedMerged > 0}
                  <span class="tm-curated-badge">+{trainProgress.curatedMerged} curated</span>
                {/if}
              </div>
            {/if}
          {/if}

          <!-- Live stats from /core/stats poll -->
          {#if trainingStatus === 'running' && Object.keys(coreStats).length > 0}
            <div class="tm-stats-grid">
              {#if coreStats.queue_depth !== undefined}
                <div class="tm-stat"><span class="tm-stat-label">Queue</span><span>{coreStats.queue_depth}</span></div>
              {/if}
              {#if coreStats.curator_buffered !== undefined}
                <div class="tm-stat"><span class="tm-stat-label">Curated buf</span><span>{coreStats.curator_buffered}</span></div>
              {/if}
              {#if coreStats.adapter_loaded !== undefined}
                <div class="tm-stat"><span class="tm-stat-label">Adapter</span><span>{coreStats.adapter_loaded ? '✓' : '–'}</span></div>
              {/if}
            </div>
          {/if}

          <!-- Log tail -->
          {#if trainingLog}
            <pre class="training-log tm-log">{trainingLog.split('\n').slice(-20).join('\n')}</pre>
          {/if}

          <!-- Model picker -->
          <div class="tm-model-row">
            <button class="btn-training tm-browse-btn" on:click={browseGguf}
              title="Browse for a local .gguf model file">
              📁 Model
            </button>
            <input class="tm-model-input"
              type="text"
              placeholder="D:\Models\...\Bonsai-1.7B-Q2_K.gguf"
              bind:value={selectedGgufPath}
              title="Local GGUF path — no downloads" />
          </div>
          {#if selectedGgufPath}
            <div class="tm-model-hint">
              🔒 Offline — local model only, no network calls
            </div>
          {:else}
            <div class="tm-model-hint tm-model-warn">
              ⚠ Select a local .gguf file to enable training
            </div>
          {/if}

          <!-- Action buttons -->
          <div class="training-actions" style="margin-top:10px">
            <button class="btn-training btn-primary-training"
              on:click={startTrainingMonitor}
              disabled={trainingStatus === 'running' || !selectedGgufPath}>
              {trainingStatus === 'running' ? '⏳ Training…' : '▶ Train New Adapter'}
            </button>
            <button class="btn-training" on:click={() => { showTrainingDashboard = true; }}>
              Dashboard
            </button>
            <button class="btn-training" on:click={() => { showHistory = !showHistory; loadRunHistory(); }}>
              History {showHistory ? '▲' : '▼'}
            </button>
          </div>

          <!-- Run history table -->
          {#if showHistory}
            <div class="tm-history">
              {#if runHistory.length === 0}
                <div class="tm-history-empty">No training runs recorded yet.</div>
              {:else}
                <table class="tm-history-table">
                  <thead>
                    <tr><th>Run</th><th>Started</th><th>Status</th><th>Examples</th></tr>
                  </thead>
                  <tbody>
                    {#each runHistory as run}
                      <tr>
                        <td class="tm-run-id">{run.id}</td>
                        <td>{fmtTs(run.started_at)}</td>
                        <td style="color:{statusColor(run.status)}">{run.status}</td>
                        <td>{run.total_examples ?? '—'}</td>
                      </tr>
                    {/each}
                  </tbody>
                </table>
              {/if}
            </div>
          {/if}
        </div>
      {/if}
    </section>

  </div>
</div>

{#if showTrainingDashboard}
  <div class="dashboard-overlay" role="dialog" aria-modal="true">
    <TrainingDashboard onClose={() => (showTrainingDashboard = false)} />
  </div>
{/if}

<style>
  .settings-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0,0,0,0.6);
    z-index: var(--z-overlay, 500);
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

  .mobile-display-settings .form-note {
    color: var(--text-dim);
    font-size: 11px;
    margin-bottom: 10px;
  }

  .mobile-display-settings .range-grid {
    display: grid;
    gap: 10px;
    margin-bottom: 10px;
  }

  .mobile-display-settings label {
    display: grid;
    gap: 6px;
    font-size: 12px;
    color: var(--text-dim);
  }

  .mobile-display-settings input[type='range'] {
    width: 100%;
  }

  .display-warning {
    margin-top: 8px;
    border: 1px solid var(--yellow);
    color: var(--yellow);
    border-radius: 10px;
    padding: 8px 10px;
    font-size: 12px;
    background: color-mix(in srgb, var(--yellow) 10%, transparent);
  }

  .display-status {
    margin-top: 8px;
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 8px 10px;
    font-size: 12px;
    color: var(--text-dim);
    background: var(--bg2);
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

  /* ── Messaging bots ─────────────────────────────────────────────────────── */
  .bots-section { padding: 12px 16px; }

  .bot-status-badge {
    margin-left: 8px;
    font-size: 11px;
    font-weight: 400;
    color: var(--text-muted, #888);
  }
  .bot-status-badge.connected { color: var(--green, #4caf50); }

  .refresh-bot {
    margin-left: 8px;
    background: transparent;
    border: 1px solid var(--border);
    color: var(--text-dim);
    padding: 2px 8px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 11px;
  }
  .refresh-bot:hover { background: var(--bg-hover); color: var(--text); }

  .bot-platform-details {
    border: 1px solid var(--border, #333);
    border-radius: 6px;
    margin-top: 8px;
    padding: 0 12px;
  }
  .bot-platform-details > summary {
    cursor: pointer;
    padding: 8px 0;
    font-size: 13px;
    font-weight: 600;
    list-style: none;
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .platform-badge {
    font-size: 11px;
    font-weight: 400;
  }
  .platform-badge.ok  { color: var(--green, #4caf50); }
  .platform-badge.err { color: var(--text-muted, #888); }

  .bot-form {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 8px 0 12px;
  }
  .bot-label {
    display: flex;
    flex-direction: column;
    gap: 3px;
    font-size: 12px;
    color: var(--text-muted, #888);
  }
  .bot-input {
    background: var(--bg-input, #1a1a1a);
    border: 1px solid var(--border, #333);
    border-radius: 4px;
    color: var(--text, #eee);
    font-size: 13px;
    padding: 5px 8px;
  }
  .secret-hint { font-size: 10px; color: var(--text-muted, #888); }

  .bot-save-msg {
    font-size: 12px;
    color: var(--green, #4caf50);
    padding: 4px 0;
  }

  .feature-flags-toggle {
    background: transparent;
    border: none;
    cursor: pointer;
    color: var(--text-dim);
    padding: 0;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    width: 100%;
    text-align: left;
  }
  .feature-flags-toggle:hover { color: var(--text); }

  .flags-heading {
    font-size: 11px;
    color: var(--text-dim);
    margin: 10px 0 6px;
    font-weight: 600;
  }

  .flags-grid {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .flag-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px 10px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 6px;
    cursor: pointer;
    font-size: 12px;
  }

  .flag-key {
    color: var(--text);
    text-transform: capitalize;
  }

  .training-section {
    margin-top: 16px;
    padding-top: 12px;
    border-top: 1px solid var(--border);
  }

  .training-actions {
    display: flex;
    gap: 8px;
    margin-top: 8px;
  }

  .btn-training {
    background: var(--bg3, #25253a);
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--text);
    cursor: pointer;
    font-size: 0.82rem;
    padding: 6px 14px;
  }
  .btn-training:hover:not(:disabled) { background: var(--bg4, #30304a); }
  .btn-training:disabled { opacity: 0.45; cursor: default; }

  .training-log {
    background: var(--bg3, #25253a);
    border-radius: 4px;
    font-family: var(--font-mono, monospace);
    font-size: 0.75rem;
    margin-top: 8px;
    padding: 8px;
    white-space: pre-wrap;
    word-break: break-all;
  }

  /* ── Training Monitor ───────────────────────────────────────────────────── */
  .tm-status-bar {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 8px 0 6px;
  }
  .tm-status-dot {
    width: 8px; height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
  }
  .tm-status-label {
    font-size: 0.72rem;
    font-weight: 600;
    letter-spacing: 0.05em;
    color: var(--text-dim, #888);
  }
  .tm-device-badge {
    background: var(--bg4, #30304a);
    border-radius: 4px;
    font-size: 0.68rem;
    padding: 1px 6px;
    color: var(--text-dim, #aaa);
  }
  .tm-progress-track {
    background: var(--bg3, #25253a);
    border-radius: 4px;
    height: 6px;
    margin: 6px 0 4px;
    overflow: hidden;
  }
  .tm-progress-fill {
    background: linear-gradient(90deg, #6c63ff, #a78bfa);
    border-radius: 4px;
    height: 100%;
    transition: width 0.4s ease;
  }
  .tm-metrics-row {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    font-size: 0.73rem;
    color: var(--text-dim, #aaa);
    margin-bottom: 4px;
  }
  .tm-loss { color: #a78bfa; font-weight: 600; }
  .tm-data-row {
    display: flex;
    gap: 8px;
    font-size: 0.71rem;
    color: var(--text-dim, #888);
    margin-bottom: 4px;
  }
  .tm-curated-badge {
    background: #2a3a2a;
    border: 1px solid #4caf5066;
    border-radius: 4px;
    color: #81c784;
    padding: 0 5px;
  }
  .tm-stats-grid {
    display: flex;
    gap: 12px;
    margin: 6px 0;
  }
  .tm-stat {
    background: var(--bg3, #25253a);
    border-radius: 4px;
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 4px 10px;
    gap: 2px;
    font-size: 0.72rem;
  }
  .tm-stat-label { color: var(--text-dim, #888); font-size: 0.66rem; }
  .tm-log { max-height: 120px; overflow-y: auto; margin-top: 6px; }
  .btn-primary-training {
    background: var(--accent, #6c63ff) !important;
    border-color: var(--accent, #6c63ff) !important;
    color: #fff !important;
  }
  .btn-primary-training:hover:not(:disabled) {
    background: #7c74ff !important;
  }
  .tm-history { margin-top: 10px; }
  .tm-history-empty {
    color: var(--text-dim, #888);
    font-size: 0.75rem;
    padding: 6px 0;
  }
  .tm-history-table {
    border-collapse: collapse;
    font-size: 0.72rem;
    width: 100%;
  }
  .tm-history-table th {
    color: var(--text-dim, #888);
    font-weight: 600;
    padding: 4px 6px;
    text-align: left;
    border-bottom: 1px solid var(--border);
  }
  .tm-history-table td {
    padding: 3px 6px;
    border-bottom: 1px solid var(--border, #2a2a3a);
    color: var(--text);
  }
  .tm-run-id {
    font-family: var(--font-mono, monospace);
    font-size: 0.68rem;
    color: var(--text-dim, #888);
  }
  .tm-model-row {
    display: flex;
    gap: 6px;
    align-items: center;
    margin-top: 10px;
  }
  .tm-browse-btn {
    flex-shrink: 0;
    white-space: nowrap;
  }
  .tm-model-input {
    background: var(--bg3, #25253a);
    border: 1px solid var(--border);
    border-radius: 5px;
    color: var(--text);
    flex: 1;
    font-family: var(--font-mono, monospace);
    font-size: 0.72rem;
    padding: 5px 8px;
    min-width: 0;
  }
  .tm-model-input:focus { outline: 1px solid var(--accent, #6c63ff); }
  .tm-model-hint {
    font-size: 0.68rem;
    color: #4caf50;
    margin-top: 3px;
  }
  .tm-model-warn { color: #ffa726; }

  .dashboard-overlay {
    position: fixed;
    inset: 0;
    z-index: 600;
    display: flex;
    align-items: stretch;
    justify-content: center;
    background: rgba(0, 0, 0, 0.7);
    padding: 32px;
  }
</style>
