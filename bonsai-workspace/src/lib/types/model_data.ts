/**
 * TypeScript mirror of the Rust `model_data` types.
 * Keep in sync with bonsai-workspace/src-tauri/src/model_data.rs.
 */

import type { InferenceMode } from '$lib/types/inference_mode';

// ── Source / Provider ─────────────────────────────────────────────────────────

export type ModelSource =
  | {
      type: 'local_gguf';
      path: string;
      registry_id?: string;
    }
  | {
      type: 'openai_compatible';
      base_url: string;
      model_id: string;
      provider_name: string;
      api_key_secret_name?: string;
    };

// ── Capabilities ──────────────────────────────────────────────────────────────

export type ToolCallingSupport = 'none' | 'basic' | 'parallel' | 'native';

export type ModelStrength =
  | 'coding'
  | 'math'
  | 'reasoning'
  | 'writing'
  | 'instruction'
  | 'multilingual'
  | 'long_context'
  | 'speed'
  | 'vision'
  | 'research'
  | 'data_analysis';

export type ModelTier = 'frontier' | 'capable' | 'fast' | 'specialized' | 'embedded';

export type PromptFormat =
  | 'open_a_i_messages'
  | 'chat_m_l'
  | 'llama3'
  | 'mistral'
  | 'alpaca'
  | 'gemma'
  | 'phi3'
  | 'deep_seek'
  | 'qwen2'
  | 'command_r'
  | { custom: string };

export interface ModelCapabilities {
  context_window:     number;
  max_output_tokens:  number;
  accepts_images:     boolean;
  accepts_audio:      boolean;
  accepts_documents:  boolean;
  tool_calling:       ToolCallingSupport;
  json_mode:          boolean;
  structured_output:  boolean;
  streaming:          boolean;
  extended_thinking:  boolean;
  strengths:          ModelStrength[];
  tier:               ModelTier;
}

// ── Inference Profile ─────────────────────────────────────────────────────────

export interface InferenceProfile {
  temperature:           number;
  top_p?:                number;
  top_k?:                number;
  min_p?:                number;
  repeat_penalty?:       number;
  presence_penalty?:     number;
  frequency_penalty?:    number;
  max_tokens:            number;
  stop_sequences:        string[];
  system_prompt_prefix?: string;
  system_prompt_suffix?: string;
}

// ── Skill Affinity ────────────────────────────────────────────────────────────

export type AffinityLevel = 'excellent' | 'good' | 'fair' | 'poor' | 'incompatible';

export interface SkillAffinity {
  skill_id: string;
  level:    AffinityLevel;
  note?:    string;
}

// ── Hardware Info ─────────────────────────────────────────────────────────────

export interface LocalFileInfo {
  path:              string;
  file_size_bytes:   number;
  ram_required_mb:   number;
  quant_label:       string;
  gpu_layers?:       number;
}

// ── Core ModelData ────────────────────────────────────────────────────────────

export interface ModelData {
  id:              string;
  name:            string;
  family?:         string;
  version?:        string;
  description:     string;
  source:          ModelSource;
  capabilities:    ModelCapabilities;
  inference:       InferenceProfile;
  inference_mode:  InferenceMode;
  prompt_format:   PromptFormat;
  skill_affinities: SkillAffinity[];
  authors:         string[];
  organization?:   string;
  license?:        string;
  homepage_url?:   string;
  training_cutoff?: string;
  parameter_count?: number;
  architecture?:   string;
  tags:            string[];
  notes:           string;
  local_file?:     LocalFileInfo;
  created_at:      number;
  updated_at:      number;
}

// ── Summary (list view) ───────────────────────────────────────────────────────

export interface ModelDataSummary {
  id:             string;
  name:           string;
  family?:        string;
  description:    string;
  source_type:    'local_gguf' | 'openai_compatible';
  tier:           ModelTier;
  strengths:      ModelStrength[];
  context_window: number;
  tool_calling:   ToolCallingSupport;
  inference_mode: InferenceMode;
  tags:           string[];
  updated_at:     number;
}

// ── Generator input ───────────────────────────────────────────────────────────

export type GenerateModelDataInput =
  | { kind: 'from_registry'; registry_id: string }
  | { kind: 'from_provider'; provider: string; model_id: string; base_url?: string };

// ── UI helpers ────────────────────────────────────────────────────────────────

/** Human-readable label for a tier. */
export function tierLabel(tier: ModelTier): string {
  switch (tier) {
    case 'frontier':    return 'Frontier';
    case 'capable':     return 'Capable';
    case 'fast':        return 'Fast';
    case 'specialized': return 'Specialized';
    case 'embedded':    return 'Embedded';
  }
}

/** Emoji badge for a tier. */
export function tierBadge(tier: ModelTier): string {
  switch (tier) {
    case 'frontier':    return '★';
    case 'capable':     return '◆';
    case 'fast':        return '⚡';
    case 'specialized': return '◎';
    case 'embedded':    return '●';
  }
}

/** Human-readable label for a tool-calling level. */
export function toolCallingLabel(tc: ToolCallingSupport): string {
  switch (tc) {
    case 'none':     return 'No tool calling';
    case 'basic':    return 'Basic (1 tool/turn)';
    case 'parallel': return 'Parallel tools';
    case 'native':   return 'Native';
  }
}

/** Human-readable label for a strength. */
export function strengthLabel(s: ModelStrength): string {
  switch (s) {
    case 'coding':        return 'Coding';
    case 'math':          return 'Math';
    case 'reasoning':     return 'Reasoning';
    case 'writing':       return 'Writing';
    case 'instruction':   return 'Instruction';
    case 'multilingual':  return 'Multilingual';
    case 'long_context':  return 'Long Context';
    case 'speed':         return 'Speed';
    case 'vision':        return 'Vision';
    case 'research':      return 'Research';
    case 'data_analysis': return 'Data Analysis';
  }
}

/** Format context window as a human-readable string. */
export function contextWindowLabel(tokens: number): string {
  if (tokens >= 1_000_000) return `${(tokens / 1_000_000).toFixed(0)}M ctx`;
  if (tokens >= 1_000)     return `${(tokens / 1_000).toFixed(0)}K ctx`;
  return `${tokens} ctx`;
}

/** Provider display name from a ModelSource. */
export function providerLabel(source: ModelSource): string {
  if (source.type === 'local_gguf') return 'Local';
  return source.provider_name;
}
