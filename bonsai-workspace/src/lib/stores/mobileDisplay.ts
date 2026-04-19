import { derived, writable } from 'svelte/store';

export interface MobileDisplaySettings {
  topOffsetPx: number;
  bottomOffsetPx: number;
  leftOffsetPx: number;
  rightOffsetPx: number;
}

const STORAGE_KEY = 'bonsai.mobileDisplay.v1';
const CONFIRMED_STORAGE_KEY = 'bonsai.mobileDisplay.confirmed.v1';
const PREVIEW_TIMEOUT_MS = 30000;
const DEFAULT_SETTINGS: MobileDisplaySettings = {
  topOffsetPx: 0,
  bottomOffsetPx: 0,
  leftOffsetPx: 0,
  rightOffsetPx: 0,
};

const DEFAULT_ANDROID_SETTINGS: MobileDisplaySettings = {
  topOffsetPx: 24,
  bottomOffsetPx: 36,
  leftOffsetPx: 0,
  rightOffsetPx: 0,
};

export interface MobileDisplayPendingState {
  isPending: boolean;
  secondsLeft: number;
  source: 'manual' | 'auto-detect' | 'reset' | '';
}

function clampPx(value: number): number {
  if (!Number.isFinite(value)) return 0;
  return Math.max(-48, Math.min(96, Math.round(value)));
}

function sanitize(raw: Partial<MobileDisplaySettings> | null | undefined): MobileDisplaySettings {
  return {
    topOffsetPx: clampPx(Number(raw?.topOffsetPx ?? 0)),
    bottomOffsetPx: clampPx(Number(raw?.bottomOffsetPx ?? 0)),
    leftOffsetPx: clampPx(Number(raw?.leftOffsetPx ?? 0)),
    rightOffsetPx: clampPx(Number(raw?.rightOffsetPx ?? 0)),
  };
}

function isAndroidEnvironment(): boolean {
  if (typeof navigator === 'undefined') return false;
  return /android/i.test(navigator.userAgent ?? '');
}

function defaultSettingsForCurrentPlatform(): MobileDisplaySettings {
  return isAndroidEnvironment() ? DEFAULT_ANDROID_SETTINGS : DEFAULT_SETTINGS;
}

export const mobileDisplaySettings = writable<MobileDisplaySettings>(DEFAULT_SETTINGS);

export const mobileDisplayPending = writable<MobileDisplayPendingState>({
  isPending: false,
  secondsLeft: 0,
  source: '',
});

let confirmedSettings: MobileDisplaySettings = { ...DEFAULT_SETTINGS };
let pendingBaseline: MobileDisplaySettings | null = null;
let pendingExpiresAt = 0;
let pendingInterval: ReturnType<typeof setInterval> | null = null;

export const mobileDisplayStyle = derived(mobileDisplaySettings, ($s) => {
  return [
    `--bonsai-mobile-safe-top: calc(env(safe-area-inset-top, 0px) + ${$s.topOffsetPx}px)`,
    `--bonsai-mobile-safe-bottom: calc(env(safe-area-inset-bottom, 0px) + ${$s.bottomOffsetPx}px)`,
    `--bonsai-mobile-safe-left: calc(env(safe-area-inset-left, 0px) + ${$s.leftOffsetPx}px)`,
    `--bonsai-mobile-safe-right: calc(env(safe-area-inset-right, 0px) + ${$s.rightOffsetPx}px)`,
  ].join('; ');
});

function copySettings(settings: MobileDisplaySettings): MobileDisplaySettings {
  return {
    topOffsetPx: settings.topOffsetPx,
    bottomOffsetPx: settings.bottomOffsetPx,
    leftOffsetPx: settings.leftOffsetPx,
    rightOffsetPx: settings.rightOffsetPx,
  };
}

function persistCurrent(settings: MobileDisplaySettings) {
  if (typeof window === 'undefined') return;
  try {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(settings));
  } catch (error) {
    console.warn('[mobileDisplay] Failed to persist settings', error);
  }
}

function persistConfirmed(settings: MobileDisplaySettings) {
  if (typeof window === 'undefined') return;
  try {
    window.localStorage.setItem(CONFIRMED_STORAGE_KEY, JSON.stringify(settings));
  } catch (error) {
    console.warn('[mobileDisplay] Failed to persist confirmed settings', error);
  }
}

function clearPendingTimer() {
  if (pendingInterval) {
    clearInterval(pendingInterval);
    pendingInterval = null;
  }
  pendingExpiresAt = 0;
}

function updatePendingCountdown() {
  if (!pendingExpiresAt) return;
  const remainingMs = Math.max(0, pendingExpiresAt - Date.now());
  const secondsLeft = Math.ceil(remainingMs / 1000);
  mobileDisplayPending.update((prev) => ({ ...prev, secondsLeft }));
}

function startPendingTimer(source: MobileDisplayPendingState['source'], timeoutMs: number) {
  clearPendingTimer();
  pendingExpiresAt = Date.now() + timeoutMs;
  mobileDisplayPending.set({
    isPending: true,
    secondsLeft: Math.ceil(timeoutMs / 1000),
    source,
  });

  pendingInterval = setInterval(() => {
    updatePendingCountdown();
    if (Date.now() >= pendingExpiresAt) {
      revertUnconfirmedMobileDisplaySettings();
    }
  }, 250);
}

export function updateMobileDisplaySettings(patch: Partial<MobileDisplaySettings>) {
  const next = sanitize({ ...confirmedSettings, ...patch });
  clearPendingTimer();
  pendingBaseline = null;
  mobileDisplayPending.set({ isPending: false, secondsLeft: 0, source: '' });
  confirmedSettings = copySettings(next);
  mobileDisplaySettings.set(next);
  persistCurrent(next);
  persistConfirmed(next);
}

export function resetMobileDisplaySettings() {
  updateMobileDisplaySettings(defaultSettingsForCurrentPlatform());
}

export function applyMobileDisplayPreview(
  patch: Partial<MobileDisplaySettings>,
  options?: {
    source?: MobileDisplayPendingState['source'];
    replace?: boolean;
    timeoutMs?: number;
  },
) {
  const source = options?.source ?? 'manual';
  const timeoutMs = Math.max(1000, options?.timeoutMs ?? PREVIEW_TIMEOUT_MS);

  if (!pendingBaseline) {
    pendingBaseline = copySettings(confirmedSettings);
  }

  mobileDisplaySettings.update((prev) => {
    const base = options?.replace ? DEFAULT_SETTINGS : prev;
    const next = sanitize({ ...base, ...patch });
    persistCurrent(next);
    return next;
  });

  startPendingTimer(source, timeoutMs);
}

export function confirmMobileDisplaySettings() {
  let current = confirmedSettings;
  const unsub = mobileDisplaySettings.subscribe((value) => {
    current = value;
  });
  unsub();

  confirmedSettings = copySettings(current);
  pendingBaseline = null;
  clearPendingTimer();
  mobileDisplayPending.set({ isPending: false, secondsLeft: 0, source: '' });
  persistCurrent(current);
  persistConfirmed(current);
}

export function revertUnconfirmedMobileDisplaySettings() {
  if (!pendingBaseline) {
    clearPendingTimer();
    mobileDisplayPending.set({ isPending: false, secondsLeft: 0, source: '' });
    return;
  }

  const rollback = copySettings(pendingBaseline);
  confirmedSettings = copySettings(rollback);
  pendingBaseline = null;
  clearPendingTimer();
  mobileDisplayPending.set({ isPending: false, secondsLeft: 0, source: '' });
  mobileDisplaySettings.set(rollback);
  persistCurrent(rollback);
  persistConfirmed(rollback);
}

export function detectMobileDisplaySettings(): MobileDisplaySettings {
  if (typeof window === 'undefined') {
    return copySettings(defaultSettingsForCurrentPlatform());
  }

  const vv = window.visualViewport;
  const layoutWidth = Math.max(1, Math.round(window.innerWidth));
  const layoutHeight = Math.max(1, Math.round(window.innerHeight));
  const viewportWidth = Math.max(1, Math.round(vv?.width ?? layoutWidth));
  const viewportHeight = Math.max(1, Math.round(vv?.height ?? layoutHeight));
  const offsetLeft = Math.max(0, Math.round(vv?.offsetLeft ?? 0));
  const offsetTop = Math.max(0, Math.round(vv?.offsetTop ?? 0));
  const offsetRight = Math.max(0, layoutWidth - (offsetLeft + viewportWidth));
  const offsetBottom = Math.max(0, layoutHeight - (offsetTop + viewportHeight));

  const screenWidth = Math.max(layoutWidth, Math.round(window.screen?.width ?? layoutWidth));
  const screenHeight = Math.max(layoutHeight, Math.round(window.screen?.height ?? layoutHeight));
  const widthDelta = Math.max(0, screenWidth - viewportWidth);
  const heightDelta = Math.max(0, screenHeight - viewportHeight);

  let inferredTop = Math.max(offsetTop, Math.round(heightDelta * 0.35));
  let inferredBottom = Math.max(offsetBottom, heightDelta - inferredTop);
  let inferredLeft = Math.max(offsetLeft, Math.round(widthDelta * 0.5));
  let inferredRight = Math.max(offsetRight, widthDelta - inferredLeft);

  if (isAndroidEnvironment()) {
    inferredTop = Math.max(DEFAULT_ANDROID_SETTINGS.topOffsetPx, inferredTop);
    inferredBottom = Math.max(DEFAULT_ANDROID_SETTINGS.bottomOffsetPx, inferredBottom);
    inferredLeft = Math.max(DEFAULT_ANDROID_SETTINGS.leftOffsetPx, inferredLeft);
    inferredRight = Math.max(DEFAULT_ANDROID_SETTINGS.rightOffsetPx, inferredRight);
  }

  return sanitize({
    topOffsetPx: inferredTop,
    bottomOffsetPx: inferredBottom,
    leftOffsetPx: inferredLeft,
    rightOffsetPx: inferredRight,
  });
}

export function applyAutoDetectedMobileDisplaySettings() {
  const detected = detectMobileDisplaySettings();
  applyMobileDisplayPreview(detected, {
    source: 'auto-detect',
    replace: true,
  });
  return detected;
}

export function initMobileDisplaySettings(): () => void {
  if (typeof window === 'undefined') {
    return () => {};
  }

  let hasCurrentSettings = false;
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (raw) {
      const parsed = JSON.parse(raw) as Partial<MobileDisplaySettings>;
      mobileDisplaySettings.set(sanitize(parsed));
      hasCurrentSettings = true;
    }
  } catch (error) {
    console.warn('[mobileDisplay] Failed to restore settings', error);
  }

  try {
    const rawConfirmed = window.localStorage.getItem(CONFIRMED_STORAGE_KEY);
    if (rawConfirmed) {
      confirmedSettings = sanitize(JSON.parse(rawConfirmed) as Partial<MobileDisplaySettings>);
    } else {
      confirmedSettings = hasCurrentSettings
        ? (() => {
            let restoredCurrent = defaultSettingsForCurrentPlatform();
            const unsub = mobileDisplaySettings.subscribe((settings) => {
              restoredCurrent = settings;
            });
            unsub();
            return copySettings(restoredCurrent);
          })()
        : detectMobileDisplaySettings();
      mobileDisplaySettings.set(copySettings(confirmedSettings));
      persistConfirmed(confirmedSettings);
      persistCurrent(confirmedSettings);
    }
  } catch (error) {
    console.warn('[mobileDisplay] Failed to restore confirmed settings', error);
    let restoredCurrent = defaultSettingsForCurrentPlatform();
    const unsub = mobileDisplaySettings.subscribe((settings) => {
      restoredCurrent = settings;
    });
    unsub();
    confirmedSettings = copySettings(restoredCurrent);
  }

  const unsub = mobileDisplaySettings.subscribe((settings) => {
    persistCurrent(settings);
  });

  const syncViewportHeight = () => {
    const h = window.visualViewport?.height ?? window.innerHeight;
    document.documentElement.style.setProperty('--bonsai-mobile-vh', `${Math.round(h)}px`);
  };

  syncViewportHeight();
  window.addEventListener('resize', syncViewportHeight);
  window.addEventListener('orientationchange', syncViewportHeight);
  window.visualViewport?.addEventListener('resize', syncViewportHeight);

  return () => {
    clearPendingTimer();
    unsub();
    window.removeEventListener('resize', syncViewportHeight);
    window.removeEventListener('orientationchange', syncViewportHeight);
    window.visualViewport?.removeEventListener('resize', syncViewportHeight);
  };
}
