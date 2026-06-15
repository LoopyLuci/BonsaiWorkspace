/**
 * OMNISYSTEM APPLICATION REGISTRY & LAUNCHER TYPES
 * Complete type definitions for application management system
 */

// ============================================================================
// APPLICATION CORE TYPES
// ============================================================================

export interface ApplicationMetadata {
  id: string;
  name: string;
  version: string;
  description: string;
  author: string;
  category: 'system' | 'development' | 'productivity' | 'utility' | 'other';
  icon: string; // Base64 or URL
  executable: string; // Path to executable or entry point
  args?: string[]; // Default arguments
  env?: Record<string, string>; // Environment variables
  minMemory?: number; // MB
  maxMemory?: number; // MB
  requiredGPU?: boolean;
  permissions?: string[]; // Required system permissions
  dependencies?: string[]; // Other app IDs that must be installed
}

export interface ApplicationInstance {
  id: string;
  appId: string;
  processId: number;
  startTime: Date;
  status: 'running' | 'paused' | 'stopped' | 'error';
  cpuUsage: number; // Percentage
  memoryUsage: number; // MB
  windowHandle?: string;
  lastError?: string;
}

export interface ApplicationRegistry {
  installed: ApplicationMetadata[];
  available: ApplicationMetadata[];
  running: ApplicationInstance[];
  lastUpdated: Date;
}

export interface ApplicationConfig {
  id: string;
  appId: string;
  settings: Record<string, any>;
  autoStart: boolean;
  priority: 'low' | 'normal' | 'high';
}

// ============================================================================
// DISCOVERY & REGISTRY TYPES
// ============================================================================

export interface DiscoveryResult {
  found: ApplicationMetadata[];
  missing: string[];
  errors: DiscoveryError[];
  timestamp: Date;
}

export interface DiscoveryError {
  path: string;
  error: string;
  severity: 'warning' | 'error';
}

export interface RegistryIndex {
  version: string;
  applications: {
    [appId: string]: ApplicationMetadata;
  };
  categories: string[];
  lastUpdated: Date;
}

// ============================================================================
// LAUNCHER & PROCESS TYPES
// ============================================================================

export interface LaunchRequest {
  appId: string;
  args?: string[];
  env?: Record<string, string>;
  workingDirectory?: string;
  detached?: boolean; // Run as separate process
}

export interface LaunchResult {
  success: boolean;
  processId?: number;
  error?: string;
  instanceId?: string;
}

export interface ProcessInfo {
  processId: number;
  appId: string;
  name: string;
  status: 'running' | 'paused' | 'zombie' | 'terminated';
  cpuUsage: number;
  memoryUsage: number;
  startTime: Date;
  uptime: number; // Seconds
  windowHandle?: string;
}

export interface ProcessMonitorData {
  processes: ProcessInfo[];
  totalCpuUsage: number;
  totalMemoryUsage: number;
  timestamp: Date;
}

// ============================================================================
// INTER-APPLICATION COMMUNICATION TYPES
// ============================================================================

export interface AppMessage {
  id: string;
  from: string; // Source app ID
  to: string; // Destination app ID (or 'broadcast')
  type: string; // Message type
  data: any; // Message payload
  timestamp: Date;
  priority: 'low' | 'normal' | 'high';
  requiresAck: boolean;
}

export interface AppMessageHandler {
  messageType: string;
  handler: (message: AppMessage) => Promise<void>;
}

export interface AppService {
  id: string;
  name: string;
  appId: string;
  methods: string[];
  description?: string;
}

export interface ServiceRequest {
  serviceId: string;
  method: string;
  params: any;
  fromApp: string;
}

export interface ServiceResponse {
  requestId: string;
  success: boolean;
  result?: any;
  error?: string;
}

// ============================================================================
// UI/DISPLAY TYPES
// ============================================================================

export interface AppMenuItem {
  app: ApplicationMetadata;
  isRunning: boolean;
  instance?: ApplicationInstance;
  lastUsed?: Date;
}

export interface AppMenuCategory {
  category: string;
  icon: string;
  apps: AppMenuItem[];
  count: number;
}

export interface AppMenuState {
  categories: AppMenuCategory[];
  running: ApplicationInstance[];
  favorites: string[]; // App IDs
  recentlyUsed: string[]; // App IDs
  searchQuery: string;
  selectedCategory?: string;
}

// ============================================================================
// EVENT TYPES
// ============================================================================

export interface ApplicationEvent {
  type: 'launched' | 'terminated' | 'crashed' | 'error' | 'message' | 'registered' | 'unregistered';
  appId: string;
  timestamp: Date;
  data?: any;
}

export interface ApplicationEventListener {
  (event: ApplicationEvent): void;
}

// ============================================================================
// ERROR TYPES
// ============================================================================

export class ApplicationError extends Error {
  constructor(
    message: string,
    public appId?: string,
    public severity: 'warning' | 'error' | 'critical' = 'error'
  ) {
    super(message);
    this.name = 'ApplicationError';
  }
}

export class RegistryError extends ApplicationError {
  constructor(message: string, appId?: string) {
    super(message, appId, 'error');
    this.name = 'RegistryError';
  }
}

export class LauncherError extends ApplicationError {
  constructor(message: string, appId?: string) {
    super(message, appId, 'critical');
    this.name = 'LauncherError';
  }
}
