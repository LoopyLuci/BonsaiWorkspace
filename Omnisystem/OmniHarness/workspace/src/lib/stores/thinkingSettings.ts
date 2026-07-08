import { writable, derived } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';

export interface ThinkingSettings {
  show_primary_thinking:  boolean;
  show_draft_thinking:    boolean;
  show_micro_thinking:    boolean;
  show_critic_thinking:   boolean;
  show_tool_rationale:    boolean;
  show_swarm_thinking:    boolean;
  max_thinking_tokens:    number;
}

const DEFAULTS: ThinkingSettings = {
  show_primary_thinking:  true,
  show_draft_thinking:    true,
  show_micro_thinking:    false,
  show_critic_thinking:   true,
  show_tool_rationale:    true,
  show_swarm_thinking:    false,
  max_thinking_tokens:    2048,
};

function createThinkingSettings() {
  const { subscribe, set, update } = writable<ThinkingSettings>(DEFAULTS);

  return {
    subscribe,
    async load() {
      try {
        const saved = await invoke<ThinkingSettings>('get_thinking_settings');
        set(saved);
      } catch {
        set(DEFAULTS);
      }
    },
    async save(settings: ThinkingSettings) {
      update(() => settings);
      try {
        await invoke('set_thinking_settings', { settings });
      } catch (e) {
        console.warn('Failed to save thinking settings:', e);
      }
    },
    toggle(key: keyof ThinkingSettings) {
      update(s => {
        const next = { ...s, [key]: !s[key as keyof ThinkingSettings] };
        invoke('set_thinking_settings', { settings: next }).catch(() => {});
        return next;
      });
    },
  };
}

export const thinkingSettings = createThinkingSettings();

/** Derived: which model roles are enabled */
export const enabledRoles = derived(thinkingSettings, $s => ({
  primary:  $s.show_primary_thinking,
  draft:    $s.show_draft_thinking,
  micro:    $s.show_micro_thinking,
  critic:   $s.show_critic_thinking,
  tool:     $s.show_tool_rationale,
  swarm:    $s.show_swarm_thinking,
}));
