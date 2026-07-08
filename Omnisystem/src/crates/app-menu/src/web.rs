use anyhow::Result;
use crate::client::{LauncherClient, MockLauncherClient};
use std::sync::Arc;

/// Web UI server configuration
#[derive(Debug, Clone)]
pub struct WebConfig {
    pub port: u16,
    pub host: String,
    pub api_base: String,
}

impl Default for WebConfig {
    fn default() -> Self {
        Self {
            port: 3000,
            host: "127.0.0.1".to_string(),
            api_base: "/api".to_string(),
        }
    }
}

/// React component generator
#[derive(Debug)]
pub struct ReactComponents;

impl ReactComponents {
    /// Generate App List component
    pub fn app_list() -> String {
        String::from(
            r#"
// src/components/AppList.jsx
import React, { useState, useEffect } from 'react';
import axios from 'axios';
import './AppList.css';

export function AppList() {
  const [apps, setApps] = useState([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(null);

  useEffect(() => {
    fetchApps();
  }, []);

  const fetchApps = async () => {
    try {
      const response = await axios.get('/api/apps');
      setApps(response.data);
      setLoading(false);
    } catch (err) {
      setError(err.message);
      setLoading(false);
    }
  };

  const launchApp = async (appId) => {
    try {
      await axios.post('/api/launch', { app_id: appId });
      alert(`Launched ${appId}`);
    } catch (err) {
      alert(`Error launching app: ${err.message}`);
    }
  };

  if (loading) return <div>Loading...</div>;
  if (error) return <div>Error: {error}</div>;

  return (
    <div className="app-list">
      <h2>Available Applications</h2>
      <div className="app-grid">
        {apps.map(app => (
          <div key={app.id} className="app-card">
            <div className="app-icon">{app.icon || '📦'}</div>
            <h3>{app.name}</h3>
            <p className="app-version">v{app.version}</p>
            <p className="app-desc">{app.description}</p>
            <button onClick={() => launchApp(app.id)} className="launch-btn">
              Launch
            </button>
          </div>
        ))}
      </div>
    </div>
  );
}
            "#,
        )
    }

    /// Generate Search component
    pub fn search_component() -> String {
        String::from(
            r#"
// src/components/SearchApps.jsx
import React, { useState } from 'react';
import axios from 'axios';
import './SearchApps.css';

export function SearchApps() {
  const [query, setQuery] = useState('');
  const [results, setResults] = useState([]);
  const [searching, setSearching] = useState(false);

  const handleSearch = async (e) => {
    const q = e.target.value;
    setQuery(q);

    if (q.length < 2) {
      setResults([]);
      return;
    }

    setSearching(true);
    try {
      const response = await axios.get(`/api/search?q=${encodeURIComponent(q)}`);
      setResults(response.data);
    } catch (err) {
      console.error('Search error:', err);
    }
    setSearching(false);
  };

  return (
    <div className="search-component">
      <div className="search-box">
        <input
          type="text"
          placeholder="Search applications..."
          value={query}
          onChange={handleSearch}
          className="search-input"
        />
        {searching && <span className="spinner">⟳</span>}
      </div>
      {results.length > 0 && (
        <div className="search-results">
          {results.map(app => (
            <div key={app.id} className="search-result">
              <span className="result-name">{app.name}</span>
              <span className="result-desc">{app.description}</span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
            "#,
        )
    }

    /// Generate Status component
    pub fn status_component() -> String {
        String::from(
            r#"
// src/components/StatusBar.jsx
import React, { useState, useEffect } from 'react';
import axios from 'axios';
import './StatusBar.css';

export function StatusBar() {
  const [status, setStatus] = useState(null);

  useEffect(() => {
    const interval = setInterval(fetchStatus, 5000);
    fetchStatus();
    return () => clearInterval(interval);
  }, []);

  const fetchStatus = async () => {
    try {
      const response = await axios.get('/api/status');
      setStatus(response.data);
    } catch (err) {
      console.error('Status fetch error:', err);
    }
  };

  if (!status) return null;

  return (
    <div className="status-bar">
      <div className="status-item">
        <span className="status-label">System:</span>
        <span className={`status-health ${status.healthy ? 'healthy' : 'unhealthy'}`}>
          {status.healthy ? '🟢 Healthy' : '🔴 Unhealthy'}
        </span>
      </div>
      <div className="status-item">
        <span className="status-label">Instances:</span>
        <span>{status.active_instances}</span>
      </div>
      <div className="status-item">
        <span className="status-label">Total Apps:</span>
        <span>{status.total_apps}</span>
      </div>
    </div>
  );
}
            "#,
        )
    }
}

/// Web server builder
pub struct WebServer {
    config: WebConfig,
    client: Arc<dyn LauncherClient>,
}

impl WebServer {
    pub async fn new(config: WebConfig) -> Result<Self> {
        let client = Arc::new(MockLauncherClient::new());
        Ok(Self { config, client })
    }

    pub fn get_api_docs(&self) -> String {
        format!(
            r#"
# Launcher Web API

## Base URL
http://{}:{}{}

## Endpoints

### GET /apps
List all available applications.

**Response:**
```json
[
  {{
    "id": "app1",
    "name": "Text Editor",
    "version": "1.0.0",
    "description": "Edit text files",
    "icon": "📝",
    "executable": "/usr/bin/nano"
  }}
]
```

### GET /apps/:id
Get details for a specific application.

### POST /launch
Launch an application.

**Request:**
```json
{{
  "app_id": "app1",
  "args": [],
  "priority": "normal"
}}
```

**Response:**
```json
{{
  "instance_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "launched"
}}
```

### GET /search?q=query
Search for applications.

### GET /instances
List running application instances.

### POST /terminate/:instance_id
Terminate a running application.

### GET /status
Get system status.

**Response:**
```json
{{
  "healthy": true,
  "uptime_ms": 3600000,
  "active_instances": 2,
  "total_apps": 10
}}
```

## WebSocket Events

**WS /ws/events**

Subscribe to real-time events:
- app.launched
- app.terminated
- system.status_changed
"#,
            self.config.host, self.config.port, self.config.api_base
        )
    }
}

/// Main Web UI
pub struct UI {
    server: WebServer,
}

impl UI {
    pub async fn new(config: WebConfig) -> Result<Self> {
        let server = WebServer::new(config).await?;
        Ok(Self { server })
    }

    pub async fn render() -> Result<()> {
        let ui = Self::new(WebConfig::default()).await?;
        ui.start_server().await?;
        Ok(())
    }

    pub async fn start_server(&self) -> Result<()> {
        // In a real implementation, this would start an axum/actix web server
        // For now, we just verify the API docs can be generated
        let _docs = self.server.get_api_docs();
        Ok(())
    }

    pub fn get_react_components() -> ReactComponentsInfo {
        ReactComponentsInfo {
            app_list: ReactComponents::app_list(),
            search: ReactComponents::search_component(),
            status: ReactComponents::status_component(),
        }
    }
}

#[derive(Debug)]
pub struct ReactComponentsInfo {
    pub app_list: String,
    pub search: String,
    pub status: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_web_config() {
        let config = WebConfig::default();
        assert_eq!(config.port, 3000);
        assert_eq!(config.host, "127.0.0.1");
    }

    #[test]
    fn test_react_components_generation() {
        let app_list = ReactComponents::app_list();
        assert!(app_list.contains("AppList"));
        assert!(app_list.contains("fetchApps"));

        let search = ReactComponents::search_component();
        assert!(search.contains("SearchApps"));

        let status = ReactComponents::status_component();
        assert!(status.contains("StatusBar"));
    }

    #[test]
    fn test_api_docs() {
        let config = WebConfig::default();
        let server = futures::executor::block_on(async {
            WebServer::new(config).await.unwrap()
        });
        let docs = server.get_api_docs();
        assert!(docs.contains("/apps"));
        assert!(docs.contains("/launch"));
        assert!(docs.contains("/search"));
    }

    #[tokio::test]
    async fn test_web_ui_new() {
        let ui = UI::new(WebConfig::default()).await.unwrap();
        assert!(ui.server.config.port > 0);
    }

    #[tokio::test]
    async fn test_web_ui_components() {
        let components = UI::get_react_components();
        assert!(!components.app_list.is_empty());
        assert!(!components.search.is_empty());
        assert!(!components.status.is_empty());
    }

    #[tokio::test]
    async fn test_web_ui_render() {
        assert!(UI::render().await.is_ok());
    }
}
