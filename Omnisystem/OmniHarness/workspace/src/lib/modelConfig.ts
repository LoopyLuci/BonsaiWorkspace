// Model configuration for Workspace model selector
export interface Model {
  id: string;
  name: string;
  description: string;
  size: string;
  type: 'cpu' | 'gpu' | 'hybrid';
  quantization: string;
  capabilities: string[];
  status: 'available' | 'loading' | 'error';
}

export const DEFAULT_MODELS: Model[] = [
  {
    id: 'octopus-v1',
    name: 'Octopus AI v1',
    description: 'Intelligent server management and computer science assistant. Expert in Linux, Docker, NixOS, networking, security, and programming.',
    size: '3-8GB (depending on quantization)',
    type: 'cpu',
    quantization: 'q4_k_m',
    capabilities: [
      'Server monitoring',
      'Container orchestration',
      'NixOS configuration',
      'Security analysis',
      'Performance optimization',
      'Incident response',
      'Programming assistance'
    ],
    status: 'available'
  },
  {
    id: 'llama-3-8b',
    name: 'Llama 3 8B (Base)',
    description: 'Base large language model. General-purpose reasoning and code generation.',
    size: '3.3GB (Q4_K_M)',
    type: 'cpu',
    quantization: 'q4_k_m',
    capabilities: [
      'General conversation',
      'Code generation',
      'Writing',
      'Analysis'
    ],
    status: 'available'
  },
  {
    id: 'mistral-7b',
    name: 'Mistral 7B',
    description: 'Fast, efficient open-source model. Good for coding and technical tasks.',
    size: '3.8GB (Q4_K_M)',
    type: 'cpu',
    quantization: 'q4_k_m',
    capabilities: [
      'Code generation',
      'Technical writing',
      'Problem solving'
    ],
    status: 'available'
  },
  {
    id: 'neural-chat-7b',
    name: 'Neural Chat 7B',
    description: 'Instruction-following model optimized for chat interactions.',
    size: '3.8GB (Q4_K_M)',
    type: 'cpu',
    quantization: 'q4_k_m',
    capabilities: [
      'Chat',
      'Instructions',
      'Dialogue'
    ],
    status: 'available'
  }
];

export interface ModelSelectorConfig {
  showAllModels: boolean;
  scanDirectories: string[];
  defaultModel: string;
  showDescriptions: boolean;
  enableHotSwap: boolean;
  autoLoadLatest: boolean;
  showSystemPrompt: boolean;
  allowCustomSystemPrompt: boolean;
}

export const MODEL_SELECTOR_CONFIG: ModelSelectorConfig = {
  // Show all registered models in the selector
  showAllModels: true,
  // Directories to scan for additional .bkp or .gguf models
  scanDirectories: [
    '~/.workspace/models',
    './models',
    '../models'
  ],
  // Default model when workspace starts
  defaultModel: 'octopus-v1',
  // Show detailed model descriptions in selector
  showDescriptions: true,
  // Enable hot-swapping between models without restart
  enableHotSwap: true,
  // Automatically load latest version of selected model
  autoLoadLatest: true,
  // Show system prompt in UI
  showSystemPrompt: true,
  // Allow users to customize system prompt
  allowCustomSystemPrompt: true
};

// Server configuration for model endpoints
export const SERVER_CONFIG = {
  apiHost: 'http://127.0.0.1',
  apiPort: 11425,
  inferencePort: 4000,
  mcpServerPort: 7780,
  getApiUrl: () => `${SERVER_CONFIG.apiHost}:${SERVER_CONFIG.apiPort}`,
  getInferenceUrl: () => `${SERVER_CONFIG.apiHost}:${SERVER_CONFIG.inferencePort}`,
  getMcpUrl: () => `${SERVER_CONFIG.apiHost}:${SERVER_CONFIG.mcpServerPort}`
};

// Model loading strategy
export const MODEL_LOADING_STRATEGY = {
  // Preload model on startup
  preloadOnStartup: true,
  // Timeout for model loading (ms)
  loadingTimeout: 30000,
  // Show loading progress
  showLoadingProgress: true,
  // Cache loaded models in memory
  enableModelCache: true,
  // Maximum cached models
  maxCachedModels: 3,
  // Unload unused models after (ms)
  unloadIdleModels: 600000  // 10 minutes
};
