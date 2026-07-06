export type InferenceMode =
  | { mode: 'auto' }
  | { mode: 'cpu_only' }
  | { mode: 'gpu_only' }
  | { mode: 'hybrid'; gpu_layers: number };

export const DEFAULT_INFERENCE_MODE: InferenceMode = { mode: 'hybrid', gpu_layers: 20 };

export function inferenceModeLabel(mode: InferenceMode): string {
  switch (mode.mode) {
    case 'auto':
      return 'Auto';
    case 'cpu_only':
      return 'CPU Only';
    case 'gpu_only':
      return 'GPU Only';
    case 'hybrid':
      return `Hybrid (${mode.gpu_layers} layers)`;
  }
}

export function inferenceModeKey(mode: InferenceMode): 'auto' | 'cpu_only' | 'gpu_only' | 'hybrid' {
  return mode.mode;
}

export function toInferenceMode(kind: string, hybridLayers = 20): InferenceMode {
  switch (kind) {
    case 'auto':
      return { mode: 'auto' };
    case 'cpu_only':
      return { mode: 'cpu_only' };
    case 'gpu_only':
      return { mode: 'gpu_only' };
    default:
      return { mode: 'hybrid', gpu_layers: hybridLayers };
  }
}
