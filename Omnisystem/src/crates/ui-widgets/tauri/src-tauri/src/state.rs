// Application state management

use crate::models::*;
use dashmap::DashMap;
use parking_lot::RwLock;
use std::sync::Arc;

pub struct LauncherState {
    // Mock data for demo - in production, this connects to daemon via IPC
    apps: Arc<DashMap<String, AppInfo>>,
    instances: Arc<DashMap<String, AppInstance>>,
    config: Arc<RwLock<LauncherConfig>>,
    logs: Arc<RwLock<Vec<LogEntry>>>,
}

impl LauncherState {
    pub fn new() -> Self {
        let state = Self {
            apps: Arc::new(DashMap::new()),
            instances: Arc::new(DashMap::new()),
            config: Arc::new(RwLock::new(LauncherConfig::default())),
            logs: Arc::new(RwLock::new(Vec::new())),
        };

        // Initialize with mock apps for demo
        state.init_mock_apps();

        state
    }

    fn init_mock_apps(&self) {
        let mock_apps = vec![
            AppInfo {
                id: "text-editor".to_string(),
                name: "Text Editor".to_string(),
                version: "1.0.0".to_string(),
                description: "Powerful text editing with syntax highlighting".to_string(),
                icon: Some("📝".to_string()),
                category: "Development".to_string(),
                executable: "code".to_string(),
                args: None,
                working_dir: None,
                tags: vec!["editor".to_string(), "development".to_string()],
            },
            AppInfo {
                id: "file-manager".to_string(),
                name: "File Manager".to_string(),
                version: "2.0.0".to_string(),
                description: "Browse and manage files with ease".to_string(),
                icon: Some("📁".to_string()),
                category: "System".to_string(),
                executable: "explorer".to_string(),
                args: None,
                working_dir: None,
                tags: vec!["files".to_string(), "system".to_string()],
            },
            AppInfo {
                id: "terminal".to_string(),
                name: "Terminal".to_string(),
                version: "1.5.0".to_string(),
                description: "Command-line interface for power users".to_string(),
                icon: Some("⌨️".to_string()),
                category: "System".to_string(),
                executable: "powershell".to_string(),
                args: None,
                working_dir: None,
                tags: vec!["cli".to_string(), "terminal".to_string(), "development".to_string()],
            },
            AppInfo {
                id: "web-browser".to_string(),
                name: "Web Browser".to_string(),
                version: "3.0.0".to_string(),
                description: "Fast and secure web browsing".to_string(),
                icon: Some("🌐".to_string()),
                category: "Internet".to_string(),
                executable: "chrome".to_string(),
                args: None,
                working_dir: None,
                tags: vec!["browser".to_string(), "internet".to_string()],
            },
            AppInfo {
                id: "omnisystem-shell".to_string(),
                name: "Omnisystem Shell".to_string(),
                version: "1.0.0".to_string(),
                description: "Advanced shell with Omnisystem integration".to_string(),
                icon: Some("🔧".to_string()),
                category: "Development".to_string(),
                executable: "omnisystem-shell".to_string(),
                args: None,
                working_dir: None,
                tags: vec!["shell".to_string(), "omnisystem".to_string()],
            },
        ];

        for app in mock_apps {
            self.apps.insert(app.id.clone(), app);
        }
    }

    pub async fn list_apps(&self) -> Result<Vec<AppInfo>, String> {
        let apps: Vec<_> = self
            .apps
            .iter()
            .map(|entry| entry.value().clone())
            .collect();
        Ok(apps)
    }

    pub async fn search_apps(&self, query: &str) -> Result<Vec<AppInfo>, String> {
        let query_lower = query.to_lowercase();
        let results: Vec<_> = self
            .apps
            .iter()
            .filter(|entry| {
                let app = entry.value();
                app.name.to_lowercase().contains(&query_lower)
                    || app.description.to_lowercase().contains(&query_lower)
                    || app.tags.iter().any(|tag| tag.to_lowercase().contains(&query_lower))
            })
            .map(|entry| entry.value().clone())
            .collect();
        Ok(results)
    }

    pub async fn launch_app(&self, app_id: &str) -> Result<LaunchResult, String> {
        match self.apps.get(app_id) {
            Some(app) => {
                let instance_id = uuid::Uuid::new_v4().to_string();
                let instance = AppInstance {
                    id: instance_id.clone(),
                    app_id: app_id.to_string(),
                    app_name: app.value().name.clone(),
                    status: "running".to_string(),
                    pid: (std::process::id()),
                    memory_mb: 0,
                    cpu_percent: 0.0,
                    launched_at: chrono::Local::now().timestamp(),
                };

                self.instances.insert(instance_id.clone(), instance);

                Ok(LaunchResult {
                    success: true,
                    instance_id: Some(instance_id),
                    message: format!("Launched {}", app.value().name),
                    error: None,
                })
            }
            None => Ok(LaunchResult {
                success: false,
                instance_id: None,
                message: String::new(),
                error: Some(format!("App not found: {}", app_id)),
            }),
        }
    }

    pub async fn get_app_details(&self, app_id: &str) -> Result<AppInfo, String> {
        self.apps
            .get(app_id)
            .map(|entry| entry.value().clone())
            .ok_or_else(|| format!("App not found: {}", app_id))
    }

    pub async fn terminate_app(&self, instance_id: &str) -> Result<(), String> {
        self.instances.remove(instance_id);
        Ok(())
    }

    pub async fn get_running_instances(&self) -> Result<Vec<AppInstance>, String> {
        let instances: Vec<_> = self
            .instances
            .iter()
            .map(|entry| entry.value().clone())
            .collect();
        Ok(instances)
    }

    pub async fn get_system_status(&self) -> Result<SystemStatus, String> {
        Ok(SystemStatus {
            healthy: true,
            uptime_seconds: 3600,
            active_instances: self.instances.len(),
            total_apps: self.apps.len(),
            memory_used_mb: 512,
            memory_available_mb: 7680,
            cpu_cores: num_cpus::get(),
            load_average: 0.45,
        })
    }

    pub async fn get_daemon_status(&self) -> Result<DaemonStatus, String> {
        Ok(DaemonStatus {
            running: true,
            address: "127.0.0.1".to_string(),
            port: 9000,
            uptime_seconds: 86400,
            connections: 5,
            version: "1.0.0".to_string(),
            last_heartbeat: chrono::Local::now().timestamp(),
        })
    }

    pub async fn get_launcher_config(&self) -> Result<LauncherConfig, String> {
        Ok(self.config.read().clone())
    }

    pub async fn update_launcher_config(&self, config: LauncherConfig) -> Result<(), String> {
        *self.config.write() = config;
        Ok(())
    }

    pub async fn get_logs(&self, limit: usize) -> Result<Vec<LogEntry>, String> {
        let logs = self.logs.read();
        let start = if logs.len() > limit {
            logs.len() - limit
        } else {
            0
        };
        Ok(logs[start..].to_vec())
    }
}

impl Clone for LauncherState {
    fn clone(&self) -> Self {
        Self {
            apps: Arc::clone(&self.apps),
            instances: Arc::clone(&self.instances),
            config: Arc::clone(&self.config),
            logs: Arc::clone(&self.logs),
        }
    }
}
