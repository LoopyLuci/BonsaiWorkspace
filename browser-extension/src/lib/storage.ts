import browser from 'webextension-polyfill';
import type { AuditEntry, ExtensionSettings } from './types';

const SETTINGS_KEY = 'bonsai.settings';
const AUDIT_KEY = 'bonsai.auditLog';

export const defaultSettings: ExtensionSettings = {
  apiHost: '127.0.0.1',
  apiPort: 11369,
  buddyHost: '127.0.0.1',
  buddyPort: 11420,
  workspaceUrl: 'http://localhost:1420',
  defaultModel: 'bonsai-buddy',
  desktopConnectionToken: '',
  observationMode: 'none',
  autoAllowActionsByHost: []
};

export async function getSettings(): Promise<ExtensionSettings> {
  const raw = await browser.storage.local.get(SETTINGS_KEY);
  const current = raw[SETTINGS_KEY] as Partial<ExtensionSettings> | undefined;
  return {
    ...defaultSettings,
    ...(current ?? {})
  };
}

export async function saveSettings(next: Partial<ExtensionSettings>): Promise<ExtensionSettings> {
  const merged = {
    ...(await getSettings()),
    ...next
  };
  await browser.storage.local.set({ [SETTINGS_KEY]: merged });
  return merged;
}

export async function getAuditLog(): Promise<AuditEntry[]> {
  const raw = await browser.storage.local.get(AUDIT_KEY);
  return (raw[AUDIT_KEY] as AuditEntry[] | undefined) ?? [];
}

export async function appendAuditEntry(entry: AuditEntry): Promise<void> {
  const current = await getAuditLog();
  const next = [entry, ...current].slice(0, 500);
  await browser.storage.local.set({ [AUDIT_KEY]: next });
}

export async function clearAuditLog(): Promise<void> {
  await browser.storage.local.set({ [AUDIT_KEY]: [] });
}
