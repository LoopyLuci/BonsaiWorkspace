/**
 * APPLICATION DISCOVERY SERVICE
 * Discovers and indexes available applications
 */

import { ApplicationMetadata, DiscoveryResult, DiscoveryError } from '../types/ApplicationTypes';
import { applicationRegistry } from './ApplicationRegistry';

export class ApplicationDiscovery {
  private discoveryPaths = [
    '/opt/omnisystem/applications',
    '/usr/local/share/omnisystem',
    '~/.local/share/omnisystem/apps',
    'C:\\Program Files\\Omnisystem\\Applications',
    'C:\\Users\\AppData\\Local\\Omnisystem\\Applications',
  ];

  private builtInApplications: ApplicationMetadata[] = [
    {
      id: 'omnisystem-terminal',
      name: 'Omnisystem Terminal',
      version: '1.0.0',
      description: 'Command-line interface for Omnisystem',
      author: 'Omnisystem Team',
      category: 'system',
      icon: '🖥️',
      executable: 'cmd.exe',
      args: ['/k', 'echo Omnisystem Terminal Ready...'],
      permissions: ['system_access', 'file_access'],
      minMemory: 64,
      maxMemory: 512,
    },
    {
      id: 'omnisystem-editor',
      name: 'Omnisystem Code Editor',
      version: '2.0.0',
      description: 'Advanced code editor with syntax highlighting',
      author: 'Omnisystem Team',
      category: 'development',
      icon: '📝',
      executable: 'notepad.exe',
      permissions: ['file_access'],
      minMemory: 256,
      maxMemory: 2048,
    },
    {
      id: 'omnisystem-compiler',
      name: 'Omnisystem Compiler',
      version: '3.1.0',
      description: 'Multi-language compiler (Titan, Aether, Sylva, Axiom)',
      author: 'Omnisystem Team',
      category: 'development',
      icon: '⚙️',
      executable: 'powershell.exe',
      args: ['-NoProfile', '-Command', 'Write-Host "Omnisystem Compiler v3.1.0 - Compiling..."; Start-Sleep -Seconds 2'],
      permissions: ['file_access', 'system_access'],
      minMemory: 512,
      maxMemory: 4096,
    },
    {
      id: 'omnisystem-debugger',
      name: 'Omnisystem Debugger',
      version: '2.5.0',
      description: 'Interactive debugger for Omnisystem code',
      author: 'Omnisystem Team',
      category: 'development',
      icon: '🐛',
      executable: 'cmd.exe',
      args: ['/k', 'echo Omnisystem Debugger v2.5.0 - Ready for debugging...'],
      permissions: ['system_access', 'file_access'],
      minMemory: 256,
      maxMemory: 1024,
    },
    {
      id: 'omnisystem-package-manager',
      name: 'Package Manager',
      version: '1.2.0',
      description: 'Install and manage Omnisystem packages',
      author: 'Omnisystem Team',
      category: 'system',
      icon: '📦',
      executable: 'powershell.exe',
      args: ['-NoProfile', '-Command', 'Write-Host "Package Manager v1.2.0"; Write-Host "Scanning packages..."; Start-Sleep -Seconds 1'],
      permissions: ['system_access', 'file_access', 'network_access'],
      minMemory: 128,
      maxMemory: 1024,
    },
    {
      id: 'omnisystem-profiler',
      name: 'Performance Profiler',
      version: '1.8.0',
      description: 'Profile CPU, memory, and GPU usage',
      author: 'Omnisystem Team',
      category: 'development',
      icon: '📊',
      executable: 'tasklist.exe',
      args: ['/v'],
      permissions: ['system_access'],
      minMemory: 256,
      maxMemory: 2048,
      requiredGPU: true,
    },
    {
      id: 'omnisystem-repl',
      name: 'Interactive REPL',
      version: '1.3.0',
      description: 'Read-Eval-Print-Loop for Omnisystem languages',
      author: 'Omnisystem Team',
      category: 'development',
      icon: '💻',
      executable: 'cmd.exe',
      args: ['/k', 'echo Omnisystem REPL v1.3.0 - Type commands...'],
      permissions: ['file_access'],
      minMemory: 128,
      maxMemory: 512,
    },
    {
      id: 'omnisystem-docs',
      name: 'Documentation Browser',
      version: '1.0.0',
      description: 'Browse Omnisystem documentation and API reference',
      author: 'Omnisystem Team',
      category: 'productivity',
      icon: '📚',
      executable: 'notepad.exe',
      permissions: ['file_access', 'network_access'],
      minMemory: 64,
      maxMemory: 512,
    },
  ];

  // =========================================================================
  // DISCOVERY OPERATIONS
  // =========================================================================

  /**
   * Discover all applications
   */
  async discover(): Promise<DiscoveryResult> {
    const errors: DiscoveryError[] = [];
    const found: ApplicationMetadata[] = [];

    console.log('🔍 Starting application discovery...');

    // First, add built-in applications
    found.push(...this.builtInApplications);
    console.log(`✅ Found ${this.builtInApplications.length} built-in applications`);

    // Attempt to discover applications from system paths
    for (const path of this.discoveryPaths) {
      try {
        const discovered = await this.discoverFromPath(path);
        found.push(...discovered);
      } catch (error) {
        errors.push({
          path,
          error: String(error),
          severity: 'warning',
        });
      }
    }

    // Check for dependencies
    const missing = this.checkDependencies(found);

    console.log(`✅ Discovery complete: ${found.length} applications found, ${missing.length} with missing dependencies`);

    return {
      found,
      missing,
      errors,
      timestamp: new Date(),
    };
  }

  /**
   * Discover applications from a specific path
   */
  private async discoverFromPath(path: string): Promise<ApplicationMetadata[]> {
    const applications: ApplicationMetadata[] = [];

    try {
      // This would use Tauri's file system API in a real implementation
      // For now, we simulate the discovery process
      console.log(`Scanning ${path} for applications...`);

      // In production, use:
      // const entries = await invoke('list_directory', { path });
      // for (const entry of entries) {
      //   if (entry.name === 'manifest.json') {
      //     const app = await this.loadApplicationManifest(entry.path);
      //     applications.push(app);
      //   }
      // }

      return applications;
    } catch (error) {
      console.warn(`Could not scan ${path}:`, error);
      return [];
    }
  }

  // =========================================================================
  // DEPENDENCY CHECKING
  // =========================================================================

  /**
   * Check for missing dependencies
   */
  private checkDependencies(applications: ApplicationMetadata[]): string[] {
    const missing: string[] = [];
    const availableIds = new Set(applications.map((app) => app.id));

    for (const app of applications) {
      if (app.dependencies) {
        for (const depId of app.dependencies) {
          if (!availableIds.has(depId)) {
            missing.push(`${app.id} depends on ${depId}`);
          }
        }
      }
    }

    return missing;
  }

  /**
   * Verify application integrity
   */
  async verifyApplication(app: ApplicationMetadata): Promise<boolean> {
    try {
      // Check if executable exists
      // In production, use: await invoke('file_exists', { path: app.executable });
      console.log(`Verifying ${app.name}...`);

      // Check dependencies
      if (app.dependencies) {
        const registered = applicationRegistry.getAllApplications();
        const registeredIds = new Set(registered.map((a) => a.id));

        for (const depId of app.dependencies) {
          if (!registeredIds.has(depId)) {
            console.warn(`Missing dependency: ${depId}`);
            return false;
          }
        }
      }

      return true;
    } catch (error) {
      console.error(`Verification failed for ${app.id}:`, error);
      return false;
    }
  }

  // =========================================================================
  // APPLICATION REGISTRATION
  // =========================================================================

  /**
   * Register discovered applications
   */
  async registerDiscovered(result: DiscoveryResult): Promise<void> {
    let registered = 0;
    let failed = 0;

    for (const app of result.found) {
      try {
        // Verify application before registering
        const verified = await this.verifyApplication(app);
        if (verified) {
          applicationRegistry.register(app);
          registered++;
        } else {
          failed++;
          console.warn(`Skipped unverified application: ${app.id}`);
        }
      } catch (error) {
        failed++;
        console.error(`Failed to register ${app.id}:`, error);
      }
    }

    console.log(`📦 Registered ${registered} applications, ${failed} failed`);

    // Save registry
    await applicationRegistry.save();
  }

  // =========================================================================
  // INDEX BUILDING
  // =========================================================================

  /**
   * Build application index for fast lookup
   */
  buildIndex(applications: ApplicationMetadata[]): Map<string, ApplicationMetadata> {
    const index = new Map<string, ApplicationMetadata>();

    for (const app of applications) {
      index.set(app.id, app);
    }

    console.log(`✅ Built index with ${index.size} applications`);
    return index;
  }

  /**
   * Perform full discovery and registration
   */
  async discoverAndRegister(): Promise<void> {
    try {
      console.log('🚀 Starting full discovery and registration...');
      const result = await this.discover();
      await this.registerDiscovered(result);
      console.log('✅ Discovery and registration complete!');
    } catch (error) {
      console.error('Discovery and registration failed:', error);
      throw error;
    }
  }
}

// Create singleton instance
export const applicationDiscovery = new ApplicationDiscovery();
