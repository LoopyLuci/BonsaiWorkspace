/**
 * APP MENU COMPONENT
 * Complete application launcher and management interface
 */

import React, { useState, useEffect } from 'react';
import { ApplicationMetadata, ApplicationInstance, AppMenuState, AppMenuCategory } from '../../types/ApplicationTypes';
import { applicationRegistry } from '../../services/ApplicationRegistry';
import { applicationDiscovery } from '../../services/ApplicationDiscovery';
import { applicationLauncher } from '../../services/ApplicationLauncher';
import './AppMenu.css';

interface AppMenuProps {
  onAppLaunch?: (appId: string) => void;
  onAppTerminate?: (instanceId: string) => void;
}

export const AppMenu: React.FC<AppMenuProps> = ({ onAppLaunch, onAppTerminate }) => {
  const [state, setState] = useState<AppMenuState>({
    categories: [],
    running: [],
    favorites: [],
    recentlyUsed: [],
    searchQuery: '',
  });

  const [loading, setLoading] = useState(true);
  const [selectedApp, setSelectedApp] = useState<ApplicationMetadata | null>(null);
  const [showRunning, setShowRunning] = useState(false);

  // Initialize application discovery and registry
  useEffect(() => {
    const initialize = async () => {
      try {
        setLoading(true);

        // Load registry from storage
        await applicationRegistry.load();

        // Perform discovery
        const discoveryResult = await applicationDiscovery.discover();

        // Register discovered applications
        await applicationDiscovery.registerDiscovered(discoveryResult);

        // Build menu categories
        buildMenuCategories();

        // Start monitoring
        applicationLauncher.startMonitoring(1000);

        // Register event listeners
        applicationLauncher.addEventListener((event) => {
          if (event.type === 'launched' || event.type === 'terminated') {
            updateRunningApps();
          }
        });

        setLoading(false);
      } catch (error) {
        console.error('Failed to initialize App Menu:', error);
        setLoading(false);
      }
    };

    initialize();

    return () => {
      applicationLauncher.stopMonitoring();
    };
  }, []);

  /**
   * Build menu categories from registered applications
   */
  const buildMenuCategories = () => {
    const apps = applicationRegistry.getAllApplications();
    const categories: Record<string, ApplicationMetadata[]> = {};

    // Group apps by category
    for (const app of apps) {
      if (!categories[app.category]) {
        categories[app.category] = [];
      }
      categories[app.category].push(app);
    }

    // Create menu categories
    const menuCategories: AppMenuCategory[] = Object.entries(categories).map(
      ([category, apps]) => ({
        category,
        icon: getCategoryIcon(category),
        apps: apps.map((app) => ({
          app,
          isRunning: applicationLauncher.getInstancesForApp(app.id).length > 0,
        })),
        count: apps.length,
      })
    );

    setState((prev) => ({
      ...prev,
      categories: menuCategories,
    }));

    updateRunningApps();
  };

  /**
   * Update running applications list
   */
  const updateRunningApps = () => {
    const running = applicationLauncher.getRunningInstances();
    setState((prev) => ({
      ...prev,
      running,
    }));
  };

  /**
   * Get category icon
   */
  const getCategoryIcon = (category: string): string => {
    const icons: Record<string, string> = {
      system: '⚙️',
      development: '👨‍💻',
      productivity: '📊',
      utility: '🔧',
      other: '📦',
    };
    return icons[category] || '📦';
  };

  /**
   * Handle app launch
   */
  const handleLaunchApp = async (app: ApplicationMetadata) => {
    try {
      const result = await applicationLauncher.launch({
        appId: app.id,
      });

      if (result.success) {
        console.log(`✅ Launched ${app.name}`);
        onAppLaunch?.(app.id);
        updateRunningApps();

        // Add to recently used
        setState((prev) => ({
          ...prev,
          recentlyUsed: [app.id, ...prev.recentlyUsed].slice(0, 5),
        }));
      }
    } catch (error) {
      console.error(`Failed to launch ${app.name}:`, error);
      alert(`Failed to launch ${app.name}: ${String(error)}`);
    }
  };

  /**
   * Handle app termination
   */
  const handleTerminateApp = async (instance: ApplicationInstance) => {
    try {
      await applicationLauncher.terminate(instance.id);
      console.log(`✅ Terminated application instance ${instance.id}`);
      onAppTerminate?.(instance.id);
      updateRunningApps();
    } catch (error) {
      console.error('Failed to terminate application:', error);
      alert(`Failed to terminate application: ${String(error)}`);
    }
  };

  /**
   * Handle search
   */
  const handleSearch = (query: string) => {
    setState((prev) => ({
      ...prev,
      searchQuery: query,
    }));
  };

  /**
   * Filter apps based on search query
   */
  const filteredCategories = state.categories
    .map((category) => ({
      ...category,
      apps: category.apps.filter((item) =>
        item.app.name
          .toLowerCase()
          .includes(state.searchQuery.toLowerCase()) ||
        item.app.description
          .toLowerCase()
          .includes(state.searchQuery.toLowerCase())
      ),
    }))
    .filter((category) => category.apps.length > 0);

  if (loading) {
    return (
      <div className="app-menu loading">
        <div className="loading-spinner">⏳ Discovering applications...</div>
      </div>
    );
  }

  return (
    <div className="app-menu">
      <div className="app-menu-header">
        <h2>🚀 Application Menu</h2>
        <div className="header-stats">
          <div className="stat">
            <span className="stat-label">Registered</span>
            <span className="stat-value">
              {applicationRegistry.getAllApplications().length}
            </span>
          </div>
          <div className="stat">
            <span className="stat-label">Running</span>
            <span className="stat-value">{state.running.length}</span>
          </div>
        </div>
      </div>

      <div className="app-menu-controls">
        <input
          type="text"
          placeholder="🔍 Search applications..."
          className="app-search"
          value={state.searchQuery}
          onChange={(e) => handleSearch(e.target.value)}
        />
        <button
          className={`toggle-btn ${showRunning ? 'active' : ''}`}
          onClick={() => setShowRunning(!showRunning)}
        >
          {showRunning ? '📊 Show All' : '🏃 Running Only'}
        </button>
      </div>

      <div className="app-menu-content">
        {/* Running Applications Section */}
        {state.running.length > 0 && (
          <div className="app-menu-section">
            <h3 className="section-title">🏃 Running Applications</h3>
            <div className="app-grid running-apps">
              {state.running.map((instance) => {
                const app = applicationRegistry.getApplication(instance.appId);
                if (!app) return null;

                return (
                  <div key={instance.id} className="app-card running">
                    <div className="app-icon">{app.icon}</div>
                    <div className="app-info">
                      <h4>{app.name}</h4>
                      <p className="app-version">v{app.version}</p>
                      <div className="app-metrics">
                        <span>CPU: {instance.cpuUsage.toFixed(1)}%</span>
                        <span>RAM: {instance.memoryUsage.toFixed(0)}MB</span>
                      </div>
                    </div>
                    <button
                      className="app-action-btn stop"
                      onClick={() => handleTerminateApp(instance)}
                      title="Terminate application"
                    >
                      ⛔
                    </button>
                  </div>
                );
              })}
            </div>
          </div>
        )}

        {/* Applications by Category */}
        {!showRunning &&
          (filteredCategories.length > 0 ? (
            filteredCategories.map((category) => (
              <div key={category.category} className="app-menu-section">
                <h3 className="section-title">
                  {category.icon} {category.category.charAt(0).toUpperCase() + category.category.slice(1)}
                </h3>
                <div className="app-grid">
                  {category.apps.map((item) => (
                    <div
                      key={item.app.id}
                      className={`app-card ${item.isRunning ? 'running' : ''}`}
                      onClick={() => setSelectedApp(item.app)}
                    >
                      <div className="app-icon">{item.app.icon}</div>
                      <div className="app-info">
                        <h4>{item.app.name}</h4>
                        <p className="app-version">v{item.app.version}</p>
                        <p className="app-description">{item.app.description}</p>
                      </div>
                      {!item.isRunning ? (
                        <button
                          className="app-action-btn launch"
                          onClick={(e) => {
                            e.stopPropagation();
                            handleLaunchApp(item.app);
                          }}
                          title="Launch application"
                        >
                          ▶️
                        </button>
                      ) : (
                        <div className="app-status-badge">Running</div>
                      )}
                    </div>
                  ))}
                </div>
              </div>
            ))
          ) : (
            <div className="no-results">
              🔍 No applications found matching "{state.searchQuery}"
            </div>
          ))}
      </div>

      {/* Application Details Panel */}
      {selectedApp && (
        <div className="app-details-panel">
          <button
            className="close-btn"
            onClick={() => setSelectedApp(null)}
          >
            ✕
          </button>
          <div className="details-content">
            <div className="details-header">
              <div className="details-icon">{selectedApp.icon}</div>
              <div className="details-title">
                <h2>{selectedApp.name}</h2>
                <p className="details-version">v{selectedApp.version}</p>
              </div>
            </div>

            <div className="details-body">
              <div className="detail-section">
                <h4>Description</h4>
                <p>{selectedApp.description}</p>
              </div>

              <div className="detail-section">
                <h4>Information</h4>
                <div className="detail-grid">
                  <div>
                    <span className="label">Author:</span>
                    <span>{selectedApp.author}</span>
                  </div>
                  <div>
                    <span className="label">Category:</span>
                    <span>{selectedApp.category}</span>
                  </div>
                  <div>
                    <span className="label">Memory:</span>
                    <span>
                      {selectedApp.minMemory}MB - {selectedApp.maxMemory}MB
                    </span>
                  </div>
                  <div>
                    <span className="label">GPU:</span>
                    <span>{selectedApp.requiredGPU ? '✅ Required' : '❌ Not required'}</span>
                  </div>
                </div>
              </div>

              {selectedApp.permissions && selectedApp.permissions.length > 0 && (
                <div className="detail-section">
                  <h4>Permissions</h4>
                  <div className="permissions-list">
                    {selectedApp.permissions.map((perm) => (
                      <span key={perm} className="permission-badge">
                        {perm}
                      </span>
                    ))}
                  </div>
                </div>
              )}

              {selectedApp.dependencies && selectedApp.dependencies.length > 0 && (
                <div className="detail-section">
                  <h4>Dependencies</h4>
                  <ul>
                    {selectedApp.dependencies.map((dep) => (
                      <li key={dep}>{dep}</li>
                    ))}
                  </ul>
                </div>
              )}

              <button
                className="launch-btn"
                onClick={() => {
                  handleLaunchApp(selectedApp);
                  setSelectedApp(null);
                }}
              >
                🚀 Launch Application
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

export default AppMenu;
