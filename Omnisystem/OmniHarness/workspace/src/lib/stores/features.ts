import { writable } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';

export interface FeatureFlags {
    swarm_enabled: boolean;
    bot_enabled: boolean;
    browser_extension_enabled: boolean;
    android_enabled: boolean;
    sandbox_system_enabled: boolean;
    mobile_automation_enabled: boolean;
    mcp_bridge_enabled: boolean;
    cluster_orchestrator_enabled: boolean;
    tts_enabled: boolean;
    hybrid_engine_enabled: boolean;
    workspace_md_enabled: boolean;
    undercover_mode: boolean;
    plan_gate_enabled: boolean;
    web_router_enabled: boolean;
    eternal_workshop_enabled: boolean;
    model_trainer_enabled: boolean;
}

const defaults: FeatureFlags = {
    swarm_enabled: false,
    bot_enabled: false,
    browser_extension_enabled: false,
    android_enabled: false,
    sandbox_system_enabled: false,
    mobile_automation_enabled: false,
    mcp_bridge_enabled: false,
    cluster_orchestrator_enabled: false,
    tts_enabled: false,
    hybrid_engine_enabled: false,
    workspace_md_enabled: true,
    undercover_mode: false,
    plan_gate_enabled: false,
    web_router_enabled: true,
    eternal_workshop_enabled: true,
    model_trainer_enabled: true,
};

export const featureFlags = writable<FeatureFlags>(defaults);

export async function loadFeatureFlags(): Promise<void> {
    try {
        const flags = await invoke<FeatureFlags>('get_feature_flags');
        featureFlags.set(flags);
    } catch (e) {
        console.error('Failed to load feature flags:', e);
    }
}

export async function saveFeatureFlags(flags: FeatureFlags): Promise<void> {
    await invoke('set_feature_flags', { flags });
    featureFlags.set(flags);
}
