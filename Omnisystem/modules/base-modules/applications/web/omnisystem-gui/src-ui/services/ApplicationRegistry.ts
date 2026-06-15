/**
 * APPLICATION REGISTRY SERVICE
 * Manages application metadata, registration, and discovery
 */

import {
  ApplicationMetadata,
  ApplicationConfig,
  RegistryError,
  ApplicationEvent,
  ApplicationEventListener,
} from '../types/ApplicationTypes';

interface RegistryIndex {
  version: string;
  applications: Record<string, ApplicationMetadata>;
  categories: string[];
  lastUpdated: Date;
}

export class ApplicationRegistry {
  private registry: Map<string, ApplicationMetadata> = new Map();
  private configs: Map<string, ApplicationConfig> = new Map();
  private listeners: ApplicationEventListener[] = [];
  private registryIndex: RegistryIndex;

  constructor() {
    this.registryIndex = {
      version: '1.0.0',
      applications: {},
      categories: ['system', 'development', 'productivity', 'utility', 'other'],
      lastUpdated: new Date(),
    };
  }

  // =========================================================================
  // CORE REGISTRY OPERATIONS
  // =========================================================================

  /**
   * Register an application
   */
  register(metadata: ApplicationMetadata): void {
    try {
      // Validate metadata
      this.validateMetadata(metadata);

      // Check for duplicates
      if (this.registry.has(metadata.id)) {
        throw new RegistryError(`Application ${metadata.id} already registered`, metadata.id);
      }

      // Register the application
      this.registry.set(metadata.id, metadata);
      this.registryIndex.applications[metadata.id] = metadata;
      this.registryIndex.lastUpdated = new Date();

      // Create default config
      this.configs.set(metadata.id, {
        id: `config_${metadata.id}`,
        appId: metadata.id,
        settings: {},
        autoStart: false,
        priority: 'normal',
      });

      // Emit event
      this.emit({
        type: 'registered',
        appId: metadata.id,
        timestamp: new Date(),
        data: metadata,
      });

      console.log(`✅ Registered application: ${metadata.name} (${metadata.id})`);
    } catch (error) {
      if (error instanceof RegistryError) {
        throw error;
      }
      throw new RegistryError(`Failed to register application: ${String(error)}`, metadata.id);
    }
  }

  /**
   * Unregister an application
   */
  unregister(appId: string): void {
    try {
      if (!this.registry.has(appId)) {
        throw new RegistryError(`Application ${appId} not found`, appId);
      }

      this.registry.delete(appId);
      this.configs.delete(appId);
      delete this.registryIndex.applications[appId];
      this.registryIndex.lastUpdated = new Date();

      this.emit({
        type: 'unregistered',
        appId,
        timestamp: new Date(),
      });

      console.log(`✅ Unregistered application: ${appId}`);
    } catch (error) {
      if (error instanceof RegistryError) {
        throw error;
      }
      throw new RegistryError(`Failed to unregister application: ${String(error)}`, appId);
    }
  }

  /**
   * Get application metadata
   */
  getApplication(appId: string): ApplicationMetadata | null {
    return this.registry.get(appId) || null;
  }

  /**
   * Get all registered applications
   */
  getAllApplications(): ApplicationMetadata[] {
    return Array.from(this.registry.values());
  }

  /**
   * Get applications by category
   */
  getApplicationsByCategory(category: string): ApplicationMetadata[] {
    return Array.from(this.registry.values()).filter(
      (app) => app.category === category
    );
  }

  /**
   * Search applications
   */
  searchApplications(query: string): ApplicationMetadata[] {
    const lowerQuery = query.toLowerCase();
    return Array.from(this.registry.values()).filter(
      (app) =>
        app.name.toLowerCase().includes(lowerQuery) ||
        app.description.toLowerCase().includes(lowerQuery) ||
        app.id.toLowerCase().includes(lowerQuery)
    );
  }

  // =========================================================================
  // APPLICATION CONFIGURATION
  // =========================================================================

  /**
   * Get application configuration
   */
  getConfig(appId: string): ApplicationConfig | null {
    return this.configs.get(appId) || null;
  }

  /**
   * Update application configuration
   */
  updateConfig(appId: string, config: Partial<ApplicationConfig>): void {
    try {
      const existing = this.configs.get(appId);
      if (!existing) {
        throw new RegistryError(`Application ${appId} config not found`, appId);
      }

      const updated = { ...existing, ...config, appId };
      this.configs.set(appId, updated);
      console.log(`✅ Updated configuration for ${appId}`);
    } catch (error) {
      throw new RegistryError(
        `Failed to update configuration: ${String(error)}`,
        appId
      );
    }
  }

  // =========================================================================
  // REGISTRY INDEX & PERSISTENCE
  // =========================================================================

  /**
   * Get registry index
   */
  getIndex(): RegistryIndex {
    return {
      ...this.registryIndex,
      lastUpdated: new Date(),
    };
  }

  /**
   * Save registry to storage
   */
  async save(): Promise<void> {
    try {
      const index = this.getIndex();
      const data = {
        index,
        applications: Array.from(this.registry.values()),
        configs: Array.from(this.configs.values()),
      };

      // Save to localStorage (or use invoke for file system)
      localStorage.setItem('omnisystem_registry', JSON.stringify(data));
      console.log('✅ Registry saved to storage');
    } catch (error) {
      console.error('Failed to save registry:', error);
      throw new RegistryError(`Failed to save registry: ${String(error)}`);
    }
  }

  /**
   * Load registry from storage
   */
  async load(): Promise<void> {
    try {
      const data = localStorage.getItem('omnisystem_registry');
      if (!data) {
        console.log('No saved registry found, starting fresh');
        return;
      }

      const parsed = JSON.parse(data);

      // Load applications
      for (const app of parsed.applications) {
        this.registry.set(app.id, app);
      }

      // Load configurations
      for (const config of parsed.configs) {
        this.configs.set(config.appId, config);
      }

      this.registryIndex = parsed.index;
      console.log(`✅ Loaded ${this.registry.size} applications from registry`);
    } catch (error) {
      console.error('Failed to load registry:', error);
      throw new RegistryError(`Failed to load registry: ${String(error)}`);
    }
  }

  // =========================================================================
  // VALIDATION
  // =========================================================================

  /**
   * Validate application metadata
   */
  private validateMetadata(metadata: ApplicationMetadata): void {
    const errors: string[] = [];

    if (!metadata.id) errors.push('Application ID is required');
    if (!metadata.name) errors.push('Application name is required');
    if (!metadata.version) errors.push('Application version is required');
    if (!metadata.executable) errors.push('Executable path is required');
    if (!['system', 'development', 'productivity', 'utility', 'other'].includes(metadata.category)) {
      errors.push('Invalid category');
    }

    if (errors.length > 0) {
      throw new RegistryError(`Validation failed: ${errors.join(', ')}`, metadata.id);
    }
  }

  // =========================================================================
  // EVENT MANAGEMENT
  // =========================================================================

  /**
   * Register event listener
   */
  addEventListener(listener: ApplicationEventListener): () => void {
    this.listeners.push(listener);

    // Return unsubscribe function
    return () => {
      this.listeners = this.listeners.filter((l) => l !== listener);
    };
  }

  /**
   * Emit event to all listeners
   */
  private emit(event: ApplicationEvent): void {
    for (const listener of this.listeners) {
      try {
        listener(event);
      } catch (error) {
        console.error('Event listener error:', error);
      }
    }
  }

  // =========================================================================
  // STATISTICS
  // =========================================================================

  /**
   * Get registry statistics
   */
  getStatistics() {
    const apps = Array.from(this.registry.values());
    const categories: Record<string, number> = {};

    for (const app of apps) {
      categories[app.category] = (categories[app.category] || 0) + 1;
    }

    return {
      totalApplications: this.registry.size,
      categories,
      totalConfigurations: this.configs.size,
      autoStartEnabled: Array.from(this.configs.values()).filter(
        (c) => c.autoStart
      ).length,
    };
  }

  /**
   * Clear all applications (for testing)
   */
  clear(): void {
    this.registry.clear();
    this.configs.clear();
    this.registryIndex.applications = {};
    this.registryIndex.lastUpdated = new Date();
    console.log('✅ Registry cleared');
  }
}

// Create singleton instance
export const applicationRegistry = new ApplicationRegistry();
