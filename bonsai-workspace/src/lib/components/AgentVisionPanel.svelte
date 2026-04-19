<script lang="ts">
  import { createEventDispatcher, onDestroy } from 'svelte';
  import { requestAskBonsai } from '$lib/stores/chat';
  import {
    clearVisionSnapshot,
    setVisionStreamActive,
    upsertVisionFrame,
    upsertVisionSnapshot,
    latestVisionFrame,
  } from '$lib/stores/vision';
  import { get } from 'svelte/store';

  type OpenCvApi = any;
  const OPENCV_SCRIPT_PATH = '/vendor/opencv/opencv.js';
  const dispatch = createEventDispatcher<{ close: void; openChat: void }>();

  let popupRef: HTMLDivElement | null = null;
  let sourceVideoRef: HTMLVideoElement | null = null;
  let sourceCanvasRef: HTMLCanvasElement | null = null;
  let visionCanvasRef: HTMLCanvasElement | null = null;

  let position = { x: 68, y: 84 };
  let isDragging = false;
  let dragOffset = { x: 0, y: 0 };

  let captureActive = false;
  let captureError = '';
  let cvStatus = 'OpenCV: loading...';
  let cvReady = false;
  let frameLoopId = 0;
  let mediaStream: MediaStream | null = null;
  let sourceResolution = 'n/a';
  let edgeRatio = 0;
  let lastEdgeRatio = 0;
  let lastTelemetryUpdateMs = 0;
  let lastFrameCaptureMs = 0;

  let cv: OpenCvApi = null;

  function loadScriptOnce(src: string): Promise<void> {
    return new Promise((resolve, reject) => {
      const existing = document.querySelector<HTMLScriptElement>(`script[src="${src}"]`);
      if (existing) {
        if (existing.dataset.loaded === '1') {
          resolve();
          return;
        }
        existing.addEventListener('load', () => resolve(), { once: true });
        existing.addEventListener('error', () => reject(new Error(`Failed to load ${src}`)), { once: true });
        return;
      }

      const script = document.createElement('script');
      script.src = src;
      script.async = true;
      script.addEventListener('load', () => {
        script.dataset.loaded = '1';
        resolve();
      }, { once: true });
      script.addEventListener('error', () => reject(new Error(`Failed to load ${src}`)), { once: true });
      document.head.appendChild(script);
    });
  }

  async function ensureOpenCv() {
    if (cvReady) return;
    try {
      cvStatus = 'OpenCV: loading...';
      await loadScriptOnce(OPENCV_SCRIPT_PATH);
      cv = (window as any).cv;
      if (!cv) {
        throw new Error('OpenCV global was not initialized');
      }
      if (typeof cv.onRuntimeInitialized === 'function' && !cv.Mat) {
        await new Promise<void>((resolve, reject) => {
          const timeout = window.setTimeout(() => reject(new Error('OpenCV initialization timed out')), 12000);
          const done = () => {
            window.clearTimeout(timeout);
            resolve();
          };
          if (cv.Mat) {
            done();
            return;
          }
          cv.onRuntimeInitialized = done;
        });
      }
      cvReady = true;
      cvStatus = 'OpenCV: ready';
    } catch (error) {
      cvReady = false;
      cvStatus = 'OpenCV: failed to load';
      captureError = `Could not initialize OpenCV: ${String(error)}`;
    }
  }

  async function startVision() {
    captureError = '';
    await ensureOpenCv();
    if (!cvReady) return;

    try {
      mediaStream = await navigator.mediaDevices.getDisplayMedia({
        video: {
          frameRate: { ideal: 15, max: 24 },
        },
        audio: false,
      });

      if (!sourceVideoRef) {
        throw new Error('Video element not ready');
      }

      sourceVideoRef.srcObject = mediaStream;
      await sourceVideoRef.play();

      captureActive = true;
      setVisionStreamActive(true);
      const track = mediaStream.getVideoTracks()[0];
      track.addEventListener('ended', () => {
        stopVision();
      });

      runFrameLoop();
    } catch (error) {
      captureActive = false;
      captureError = `Screen capture unavailable: ${String(error)}`;
      cleanupStream();
    }
  }

  function stopVision() {
    captureActive = false;
    setVisionStreamActive(false);
    clearVisionSnapshot();
    lastEdgeRatio = 0;
    lastTelemetryUpdateMs = 0;
    lastFrameCaptureMs = 0;
    if (frameLoopId) {
      window.cancelAnimationFrame(frameLoopId);
      frameLoopId = 0;
    }
    cleanupStream();
  }

  function cleanupStream() {
    if (mediaStream) {
      for (const track of mediaStream.getTracks()) {
        track.stop();
      }
    }
    mediaStream = null;
    if (sourceVideoRef) {
      sourceVideoRef.srcObject = null;
    }
  }

  function runFrameLoop() {
    if (!captureActive || !sourceVideoRef || !sourceCanvasRef || !visionCanvasRef || !cvReady || !cv) return;

    const width = Math.max(320, Math.floor(sourceVideoRef.videoWidth || 0));
    const height = Math.max(200, Math.floor(sourceVideoRef.videoHeight || 0));
    sourceResolution = `${width}x${height}`;

    if (sourceCanvasRef.width !== width || sourceCanvasRef.height !== height) {
      sourceCanvasRef.width = width;
      sourceCanvasRef.height = height;
    }
    if (visionCanvasRef.width !== width || visionCanvasRef.height !== height) {
      visionCanvasRef.width = width;
      visionCanvasRef.height = height;
    }

    const ctx = sourceCanvasRef.getContext('2d');
    if (!ctx) return;

    const step = () => {
      if (!captureActive || !sourceVideoRef || !sourceCanvasRef || !visionCanvasRef || !cvReady || !cv) {
        return;
      }

      ctx.drawImage(sourceVideoRef, 0, 0, width, height);

      const src = cv.imread(sourceCanvasRef);
      const gray = new cv.Mat();
      const edges = new cv.Mat();
      try {
        cv.cvtColor(src, gray, cv.COLOR_RGBA2GRAY, 0);
        cv.Canny(gray, edges, 60, 140, 3, false);
        cv.imshow(visionCanvasRef, edges);
        const nonZero = cv.countNonZero(edges);
        edgeRatio = Math.min(1, Math.max(0, nonZero / (width * height)));

        const now = Date.now();
        if (now - lastTelemetryUpdateMs >= 500) {
          const motionDelta = Math.abs(edgeRatio - lastEdgeRatio);
          const luminance = Number(cv.mean(gray)[0] ?? 0);
          upsertVisionSnapshot({
            timestampIso: new Date(now).toISOString(),
            resolution: `${width}x${height}`,
            edgeDensityPct: Math.round(edgeRatio * 1000) / 10,
            luminance: Math.round(luminance * 10) / 10,
            motionDeltaPct: Math.round(motionDelta * 1000) / 10,
          });
          lastEdgeRatio = edgeRatio;
          lastTelemetryUpdateMs = now;
        }

        if (now - lastFrameCaptureMs >= 1500) {
          const frameDataUrl = captureFramePreview(sourceCanvasRef, width, height);
          if (frameDataUrl) {
            upsertVisionFrame({
              timestampIso: new Date(now).toISOString(),
              resolution: `${width}x${height}`,
              dataUrl: frameDataUrl,
            });
          }
          lastFrameCaptureMs = now;
        }
      } finally {
        src.delete();
        gray.delete();
        edges.delete();
      }

      frameLoopId = window.requestAnimationFrame(step);
    };

    frameLoopId = window.requestAnimationFrame(step);
  }

  function captureFramePreview(sourceCanvas: HTMLCanvasElement, width: number, height: number): string | null {
    const targetW = Math.max(320, Math.min(640, width));
    const scale = targetW / Math.max(1, width);
    const targetH = Math.max(180, Math.round(height * scale));

    const previewCanvas = document.createElement('canvas');
    previewCanvas.width = targetW;
    previewCanvas.height = targetH;
    const previewCtx = previewCanvas.getContext('2d');
    if (!previewCtx) return null;
    previewCtx.drawImage(sourceCanvas, 0, 0, targetW, targetH);
    return previewCanvas.toDataURL('image/jpeg', 0.55);
  }

  function sendVisionSnapshotToChat() {
    const detail = Math.round(edgeRatio * 1000) / 10;
    const frame = get(latestVisionFrame);
    requestAskBonsai({
      action: 'explain',
      prompt: [
        'Live screen-share context from Agent Vision is active.',
        'Use this telemetry to discuss what is visible and what to do next.',
        '',
        'Agent Vision snapshot context:',
        `- Source resolution: ${sourceResolution}`,
        `- Edge density: ${detail}%`,
        `- Motion delta: ${Math.round(Math.abs(edgeRatio - lastEdgeRatio) * 1000) / 10}%`,
        `- Timestamp: ${new Date().toISOString()}`,
        `- Frame captured for attachment-ready path: ${frame ? `yes (${frame.resolution} @ ${frame.timestampIso})` : 'not yet'}`,
        '',
        'Please discuss what this likely means in the current project context and suggest the next debugging or implementation step.',
      ].join('\n'),
    });
    dispatch('openChat');
  }

  function onHeaderPointerDown(event: PointerEvent) {
    if (!popupRef) return;
    isDragging = true;
    const rect = popupRef.getBoundingClientRect();
    dragOffset = { x: event.clientX - rect.left, y: event.clientY - rect.top };
    window.addEventListener('pointermove', onDragMove);
    window.addEventListener('pointerup', onDragEnd);
  }

  function onDragMove(event: PointerEvent) {
    if (!isDragging || !popupRef) return;
    const maxX = Math.max(0, window.innerWidth - popupRef.offsetWidth);
    const maxY = Math.max(0, window.innerHeight - popupRef.offsetHeight);
    position = {
      x: Math.min(maxX, Math.max(0, event.clientX - dragOffset.x)),
      y: Math.min(maxY, Math.max(0, event.clientY - dragOffset.y)),
    };
  }

  function onDragEnd() {
    isDragging = false;
    window.removeEventListener('pointermove', onDragMove);
    window.removeEventListener('pointerup', onDragEnd);
  }

  function closePanel() {
    stopVision();
    dispatch('close');
  }

  onDestroy(() => {
    stopVision();
    onDragEnd();
  });
</script>

<div class="vision-popup" bind:this={popupRef} style="left: {position.x}px; top: {position.y}px;">
  <div class="vision-header" on:pointerdown={onHeaderPointerDown}>
    <div>
      <div class="title">Agent Vision</div>
      <div class="subtitle">Live screen understanding via OpenCV</div>
    </div>
    <button class="close-btn" on:click={closePanel} type="button" aria-label="Close Agent Vision">x</button>
  </div>

  <div class="vision-controls">
    {#if !captureActive}
      <button class="primary" on:click={startVision} type="button">Start Screen Capture</button>
    {:else}
      <button class="warn" on:click={stopVision} type="button">Stop Capture</button>
    {/if}
    <button class="secondary" on:click={sendVisionSnapshotToChat} type="button" disabled={!captureActive}>Discuss Snapshot</button>
  </div>

  <div class="vision-meta">
    <span>{cvStatus}</span>
    <span>Resolution: {sourceResolution}</span>
    <span>Edge Density: {Math.round(edgeRatio * 1000) / 10}%</span>
  </div>

  {#if captureError}
    <p class="error">{captureError}</p>
  {/if}

  <div class="vision-grid">
    <section>
      <h4>Input</h4>
      <video bind:this={sourceVideoRef} autoplay muted playsinline></video>
      <canvas bind:this={sourceCanvasRef} class="hidden-canvas"></canvas>
    </section>
    <section>
      <h4>Agent View (Edges)</h4>
      <canvas bind:this={visionCanvasRef}></canvas>
    </section>
  </div>
</div>

<style>
  .vision-popup {
    position: fixed;
    z-index: var(--z-panel, 100);
    width: min(700px, calc(100vw - 24px));
    max-height: min(78vh, 840px);
    overflow: hidden;
    border-radius: 14px;
    border: 1px solid var(--border);
    background: color-mix(in srgb, var(--bg2) 92%, #0a0a0a 8%);
    box-shadow: 0 22px 60px rgba(0, 0, 0, 0.45);
    backdrop-filter: blur(6px);
  }

  .vision-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding: 10px 12px;
    border-bottom: 1px solid var(--border);
    cursor: move;
    user-select: none;
    background: linear-gradient(135deg, rgba(22, 163, 74, 0.18), rgba(2, 132, 199, 0.1));
  }

  .title {
    font-size: 13px;
    font-weight: 700;
    color: var(--text);
  }

  .subtitle {
    font-size: 11px;
    color: var(--text-dim);
  }

  .close-btn {
    border: 1px solid var(--border);
    background: transparent;
    color: var(--text-dim);
    border-radius: 8px;
    width: 28px;
    height: 28px;
    cursor: pointer;
  }

  .close-btn:hover {
    color: var(--text);
    background: var(--bg-hover);
  }

  .vision-controls {
    display: flex;
    gap: 8px;
    padding: 10px 12px 0;
    flex-wrap: wrap;
  }

  .vision-controls button {
    border-radius: 8px;
    padding: 8px 12px;
    border: 1px solid transparent;
    cursor: pointer;
    font-size: 12px;
    font-weight: 600;
  }

  .primary {
    color: #0d1f14;
    background: linear-gradient(135deg, #22c55e, #16a34a);
  }

  .warn {
    color: #2f1212;
    background: linear-gradient(135deg, #f59e0b, #f97316);
  }

  .secondary {
    border-color: var(--border);
    color: var(--text);
    background: transparent;
  }

  .secondary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .vision-meta {
    display: flex;
    gap: 10px;
    flex-wrap: wrap;
    padding: 8px 12px 0;
    font-size: 11px;
    color: var(--text-dim);
  }

  .error {
    margin: 8px 12px 0;
    color: #f87171;
    font-size: 12px;
  }

  .vision-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
    padding: 10px 12px 12px;
  }

  .vision-grid section {
    border: 1px solid var(--border);
    border-radius: 10px;
    overflow: hidden;
    background: rgba(0, 0, 0, 0.25);
  }

  .vision-grid h4 {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.02em;
    color: var(--text-dim);
    border-bottom: 1px solid var(--border);
    padding: 7px 8px;
    background: rgba(255, 255, 255, 0.03);
  }

  .vision-grid video,
  .vision-grid canvas {
    width: 100%;
    height: 220px;
    display: block;
    background: #050505;
    object-fit: cover;
  }

  .hidden-canvas {
    display: none;
  }

  @media (max-width: 840px) {
    .vision-grid {
      grid-template-columns: 1fr;
    }
  }
</style>