import { writable, derived } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { DEFAULT_API_HOST, DEFAULT_API_PORT } from '$lib/constants/network';

export const apiHost = writable(DEFAULT_API_HOST);
export const apiPort = writable(DEFAULT_API_PORT);
export const apiBaseUrl = derived([apiHost, apiPort], ([$apiHost, $apiPort]) => `http://${$apiHost}:${$apiPort}`);

export async function loadApiSettings() {
  try {
    const config = await invoke<{ api_host: string; api_port: number }>('get_api_config');
    apiHost.set(config.api_host);
    apiPort.set(config.api_port);
    return config;
  } catch (e) {
    console.error('Failed to load API settings:', e);
    return { api_host: DEFAULT_API_HOST, api_port: DEFAULT_API_PORT };
  }
}

export async function saveApiSettings(host: string, port: number) {
  try {
    const normalizedHost = String(host ?? '').trim();
    const normalizedPort = Number(port);

    if (!normalizedHost) {
      throw new Error('API host is required.');
    }
    if (!Number.isFinite(normalizedPort) || normalizedPort < 1 || normalizedPort > 65535) {
      throw new Error('API port must be between 1 and 65535.');
    }

    const config = await invoke<{ api_host: string; api_port: number }>('set_api_config', {
      // Tauri command args are camelCase on the JS side.
      apiHost: normalizedHost,
      apiPort: normalizedPort,
    });
    apiHost.set(config.api_host);
    apiPort.set(config.api_port);
    return config;
  } catch (e) {
    console.error('Failed to save API settings:', e);
    throw e;
  }
}
