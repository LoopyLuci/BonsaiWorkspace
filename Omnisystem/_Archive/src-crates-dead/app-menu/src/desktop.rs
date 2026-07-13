use anyhow::Result;
use crate::client::{LauncherClient, MockLauncherClient, AppMetadata, AppInstance};
use crate::tauri;
use std::sync::Arc;
use std::fs;

/// Desktop UI configuration
#[derive(Debug, Clone)]
pub struct DesktopConfig {
    pub window_title: String,
    pub window_width: u32,
    pub window_height: u32,
    pub dark_mode: bool,
}

impl Default for DesktopConfig {
    fn default() -> Self {
        Self {
            window_title: "Application Launcher".to_string(),
            window_width: 1024,
            window_height: 768,
            dark_mode: true,
        }
    }
}

/// Desktop UI components
#[derive(Debug)]
pub struct AppGrid {
    apps: Vec<AppMetadata>,
}

impl AppGrid {
    pub fn new(apps: Vec<AppMetadata>) -> Self {
        Self { apps }
    }

    pub fn render_html(&self) -> String {
        let mut html = String::from(
            r#"
        <div class="app-grid">
            <div class="grid-container">
        "#,
        );

        for app in &self.apps {
            html.push_str(&format!(
                r#"
            <div class="app-card" data-app-id="{}">
                <div class="app-icon">{}</div>
                <div class="app-name">{}</div>
                <div class="app-version">{}</div>
                <button class="launch-btn" onclick="launch('{}')">Launch</button>
            </div>
            "#,
                app.id,
                app.icon.as_deref().unwrap_or("📦"),
                app.name,
                app.version,
                app.id
            ));
        }

        html.push_str(
            r#"
            </div>
        </div>
        "#,
        );
        html
    }
}

#[derive(Debug)]
pub struct SearchBar;

impl SearchBar {
    pub fn render_html() -> String {
        String::from(
            r#"
        <div class="search-container">
            <input type="text" class="search-input" placeholder="Search applications..."
                   onkeyup="search(this.value)">
            <div class="search-results" id="search-results"></div>
        </div>
        "#,
        )
    }
}

#[derive(Debug)]
pub struct StatusBar {
    pub active_instances: usize,
    pub total_apps: usize,
    pub healthy: bool,
}

impl StatusBar {
    pub fn render_html(&self) -> String {
        let health_indicator = if self.healthy { "🟢" } else { "🔴" };
        format!(
            r#"
        <div class="status-bar">
            <span class="status-health">{} System Health</span>
            <span class="status-instances">Active: {}</span>
            <span class="status-apps">Total Apps: {}</span>
        </div>
        "#,
            health_indicator, self.active_instances, self.total_apps
        )
    }
}

/// Main Desktop UI
pub struct UI {
    client: Arc<dyn LauncherClient>,
    config: DesktopConfig,
}

impl UI {
    pub async fn new(config: DesktopConfig) -> Result<Self> {
        let client = Arc::new(MockLauncherClient::new());
        Ok(Self { client, config })
    }

    pub async fn render() -> Result<()> {
        let ui = Self::new(DesktopConfig::default()).await?;
        ui.render_window().await?;
        Ok(())
    }

    pub async fn render_window(&self) -> Result<()> {
        let apps = self.client.list_apps().await?;
        let status = self.client.get_system_status().await?;

        let grid = AppGrid::new(apps);
        let search = SearchBar::render_html();
        let status_bar = StatusBar {
            active_instances: status.active_instances,
            total_apps: status.total_apps,
            healthy: status.healthy,
        }
        .render_html();

        let _html = format!(
            r#"
        <!DOCTYPE html>
        <html>
        <head>
            <meta charset="UTF-8">
            <title>{}</title>
            <style>
                * {{ margin: 0; padding: 0; box-sizing: border-box; }}
                body {{
                    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
                    background: #1e1e1e;
                    color: #e0e0e0;
                }}
                .container {{ display: flex; flex-direction: column; height: 100vh; }}
                .header {{
                    background: #2d2d2d;
                    padding: 20px;
                    border-bottom: 1px solid #444;
                }}
                .search-container {{
                    padding: 20px;
                    background: #252525;
                }}
                .search-input {{
                    width: 100%;
                    padding: 12px;
                    border: 1px solid #444;
                    background: #3a3a3a;
                    color: #e0e0e0;
                    border-radius: 4px;
                    font-size: 14px;
                }}
                .search-input:focus {{
                    outline: none;
                    border-color: #007acc;
                }}
                .app-grid {{ flex: 1; overflow-y: auto; padding: 20px; }}
                .grid-container {{
                    display: grid;
                    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
                    gap: 20px;
                }}
                .app-card {{
                    background: #2d2d2d;
                    border: 1px solid #444;
                    border-radius: 8px;
                    padding: 15px;
                    text-align: center;
                    cursor: pointer;
                    transition: all 0.2s;
                }}
                .app-card:hover {{
                    background: #3a3a3a;
                    border-color: #007acc;
                    transform: translateY(-2px);
                }}
                .app-icon {{ font-size: 48px; margin: 10px 0; }}
                .app-name {{ font-weight: 600; margin: 10px 0; }}
                .app-version {{ font-size: 12px; color: #888; margin: 5px 0; }}
                .launch-btn {{
                    width: 100%;
                    padding: 8px;
                    margin-top: 10px;
                    background: #007acc;
                    color: white;
                    border: none;
                    border-radius: 4px;
                    cursor: pointer;
                    font-weight: 600;
                    transition: background 0.2s;
                }}
                .launch-btn:hover {{ background: #0098ff; }}
                .status-bar {{
                    background: #252525;
                    padding: 12px 20px;
                    border-top: 1px solid #444;
                    display: flex;
                    justify-content: space-between;
                    font-size: 12px;
                    color: #888;
                }}
            </style>
        </head>
        <body>
            <div class="container">
                <div class="header">
                    <h1>{}</h1>
                </div>
                {}
                {}
                {}
            </div>
        </body>
        </html>
        "#,
            self.config.window_title, self.config.window_title, search, grid.render_html(), status_bar
        );

        Ok(())
    }

    pub async fn launch_app(&self, app_id: &str) -> Result<AppInstance> {
        let response = self.client.launch_app(crate::client::LaunchRequest {
            app_id: app_id.to_string(),
            args: vec![],
            priority: "normal".to_string(),
        }).await?;

        Ok(AppInstance {
            instance_id: response.instance_id,
            app_id: app_id.to_string(),
            status: response.status,
            pid: Some(0),
            started_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    pub async fn search_apps(&self, query: &str) -> Result<Vec<AppMetadata>> {
        self.client.search_apps(query).await
    }

    /// Generate Tauri + Svelte project structure
    pub fn generate_tauri_project(output_dir: &str) -> Result<()> {
        let base_path = std::path::Path::new(output_dir);

        // Create directory structure
        fs::create_dir_all(base_path.join("src/components"))?;
        fs::create_dir_all(base_path.join("src-tauri/src"))?;

        // Write Svelte components
        fs::write(
            base_path.join("src/App.svelte"),
            tauri::app_component(),
        )?;
        fs::write(
            base_path.join("src/components/SearchBar.svelte"),
            tauri::search_bar_component(),
        )?;
        fs::write(
            base_path.join("src/components/AppList.svelte"),
            tauri::app_list_component(),
        )?;
        fs::write(
            base_path.join("src/components/AppCard.svelte"),
            tauri::app_card_component(),
        )?;
        fs::write(
            base_path.join("src/components/StatusBar.svelte"),
            tauri::status_bar_component(),
        )?;

        // Write config files
        fs::write(
            base_path.join("package.json"),
            tauri::svelte_package_json(),
        )?;
        fs::write(
            base_path.join("src-tauri/tauri.conf.json"),
            tauri::tauri_config(),
        )?;

        tracing::info!("Tauri project generated at {}", output_dir);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_desktop_config() {
        let config = DesktopConfig::default();
        assert_eq!(config.window_width, 1024);
        assert_eq!(config.window_height, 768);
        assert!(config.dark_mode);
    }

    #[test]
    fn test_app_grid_render() {
        let apps = vec![AppMetadata {
            id: "app1".to_string(),
            name: "Test App".to_string(),
            version: "1.0.0".to_string(),
            description: "Test".to_string(),
            icon: Some("📦".to_string()),
            executable: "/test".to_string(),
        }];

        let grid = AppGrid::new(apps);
        let html = grid.render_html();
        assert!(html.contains("app1"));
        assert!(html.contains("Test App"));
    }

    #[test]
    fn test_search_bar_render() {
        let html = SearchBar::render_html();
        assert!(html.contains("search-input"));
        assert!(html.contains("Search applications"));
    }

    #[test]
    fn test_status_bar_render() {
        let status = StatusBar {
            active_instances: 2,
            total_apps: 10,
            healthy: true,
        };
        let html = status.render_html();
        assert!(html.contains("System Health"));
        assert!(html.contains("Active: 2"));
        assert!(html.contains("Total Apps: 10"));
    }

    #[tokio::test]
    async fn test_desktop_ui_new() {
        let ui = UI::new(DesktopConfig::default()).await.unwrap();
        assert_eq!(ui.config.window_title, "Application Launcher");
    }

    #[tokio::test]
    async fn test_desktop_ui_launch() {
        let ui = UI::new(DesktopConfig::default()).await.unwrap();
        let instance = ui.launch_app("app1").await.unwrap();
        assert_eq!(instance.app_id, "app1");
    }

    #[tokio::test]
    async fn test_desktop_ui_search() {
        let ui = UI::new(DesktopConfig::default()).await.unwrap();
        let results = ui.search_apps("file").await.unwrap();
        assert!(!results.is_empty());
    }

    #[tokio::test]
    async fn test_desktop_ui_render() {
        assert!(UI::render().await.is_ok());
    }
}
