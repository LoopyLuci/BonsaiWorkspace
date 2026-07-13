use anyhow::Result;
use app_menu::client::LauncherClient;
use app_menu::server::LauncherServer;
use clap::Parser;
use std::sync::Arc;
use std::collections::HashMap;
use std::process::Command;
use tokio::sync::RwLock;

#[derive(Parser)]
#[command(name = "launcher-web")]
#[command(about = "Omnisystem Launcher Web Server", long_about = None)]
struct Args {
    #[arg(short, long, default_value = "8080")]
    port: u16,

    #[arg(short, long, default_value = "127.0.0.1")]
    host: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    let addr = format!("{}:{}", args.host, args.port);

    // Create a mock client with instance tracking
    let client: Arc<dyn LauncherClient> = Arc::new(MockClient {
        instances: Arc::new(RwLock::new(HashMap::new())),
    });

    let server = LauncherServer::new(client, addr);
    server.start().await?;

    Ok(())
}

// Mock client implementation for testing
struct MockClient {
    instances: Arc<RwLock<HashMap<uuid::Uuid, app_menu::client::AppInstance>>>,
}

#[async_trait::async_trait]
impl LauncherClient for MockClient {
    async fn list_apps(&self) -> Result<Vec<app_menu::client::AppMetadata>> {
        Ok(vec![
            app_menu::client::AppMetadata {
                id: "buddy".to_string(),
                name: "Buddy Assistant".to_string(),
                version: "1.0.0".to_string(),
                description: "Universal Interactive Assistant for productivity and automation".to_string(),
                icon: Some("🤖".to_string()),
                executable: "buddy.exe".to_string(),
            },
            app_menu::client::AppMetadata {
                id: "omnibot".to_string(),
                name: "OmniBot Orchestrator".to_string(),
                version: "1.0.0".to_string(),
                description: "Autonomous Backend Orchestrator for system management and automation".to_string(),
                icon: Some("⚡".to_string()),
                executable: "omnibot.exe".to_string(),
            },
            app_menu::client::AppMetadata {
                id: "usee-search".to_string(),
                name: "USEE Search Engine".to_string(),
                version: "1.0.0".to_string(),
                description: "Ultra-high-performance semantic search with 100K+ QPS distributed support".to_string(),
                icon: Some("🔍".to_string()),
                executable: "usee-search.exe".to_string(),
            },
            app_menu::client::AppMetadata {
                id: "transfer-client".to_string(),
                name: "TransferDaemon Client".to_string(),
                version: "1.0.0".to_string(),
                description: "Multi-path P2P file transfer with edge computing and AI routing".to_string(),
                icon: Some("📤".to_string()),
                executable: "transfer-client.exe".to_string(),
            },
            app_menu::client::AppMetadata {
                id: "remote-access".to_string(),
                name: "Remote Access System".to_string(),
                version: "1.0.0".to_string(),
                description: "Enterprise-grade secure remote access and command execution".to_string(),
                icon: Some("🔐".to_string()),
                executable: "remote-access-support.exe".to_string(),
            },
            app_menu::client::AppMetadata {
                id: "app-manager-cli".to_string(),
                name: "App Manager CLI".to_string(),
                version: "1.0.0".to_string(),
                description: "Advanced application management and registry control from command line".to_string(),
                icon: Some("📦".to_string()),
                executable: "app-manager-cli.exe".to_string(),
            },
            app_menu::client::AppMetadata {
                id: "launcher-pre".to_string(),
                name: "Pre-Launcher".to_string(),
                version: "1.0.0".to_string(),
                description: "Dependency checker and system bootstrapper for Omnisystem".to_string(),
                icon: Some("🔧".to_string()),
                executable: "pre-launcher.exe".to_string(),
            },
        ])
    }

    async fn get_app(&self, app_id: &str) -> Result<Option<app_menu::client::AppMetadata>> {
        let all_apps = self.list_apps().await?;
        Ok(all_apps.into_iter().find(|app| app.id == app_id))
    }

    async fn search_apps(&self, query: &str) -> Result<Vec<app_menu::client::AppMetadata>> {
        let all_apps = self.list_apps().await?;
        let query_lower = query.to_lowercase();
        Ok(all_apps
            .into_iter()
            .filter(|app| {
                app.name.to_lowercase().contains(&query_lower)
                    || app.description.to_lowercase().contains(&query_lower)
            })
            .collect())
    }

    async fn launch_app(
        &self,
        request: app_menu::client::LaunchRequest,
    ) -> Result<app_menu::client::LaunchResponse> {
        let instance_id = uuid::Uuid::new_v4();

        // Get app info to find executable
        let app = self.get_app(&request.app_id).await?;

        if let Some(app_info) = app {
            // Try to launch the app
            let mut cmd = Command::new(&app_info.executable);
            for arg in &request.args {
                cmd.arg(arg);
            }

            match cmd.spawn() {
                Ok(child) => {
                    let pid = child.id();

                    // Track instance
                    let instance = app_menu::client::AppInstance {
                        instance_id,
                        app_id: request.app_id.clone(),
                        status: "running".to_string(),
                        pid: Some(pid),
                        started_at: chrono::Local::now().to_rfc3339(),
                    };

                    self.instances.write().await.insert(instance_id, instance);

                    tracing::info!(
                        "Launched {} ({}): pid={}",
                        app_info.name,
                        request.app_id,
                        pid
                    );

                    Ok(app_menu::client::LaunchResponse {
                        instance_id,
                        status: "launched".to_string(),
                    })
                }
                Err(e) => {
                    tracing::warn!("Failed to launch {}: {}", app_info.name, e);
                    Ok(app_menu::client::LaunchResponse {
                        instance_id,
                        status: format!("error: {}", e),
                    })
                }
            }
        } else {
            Ok(app_menu::client::LaunchResponse {
                instance_id,
                status: "error: app not found".to_string(),
            })
        }
    }

    async fn list_instances(&self) -> Result<Vec<app_menu::client::AppInstance>> {
        let instances = self.instances.read().await;
        Ok(instances.values().cloned().collect())
    }

    async fn terminate_app(&self, instance_id: &uuid::Uuid) -> Result<()> {
        let mut instances = self.instances.write().await;
        if let Some(instance) = instances.remove(instance_id) {
            if let Some(pid) = instance.pid {
                // Try to kill the process
                #[cfg(target_os = "windows")]
                {
                    let _ = Command::new("taskkill")
                        .args(&["/PID", &pid.to_string(), "/F"])
                        .output();
                }
                #[cfg(not(target_os = "windows"))]
                {
                    let _ = Command::new("kill")
                        .arg("-9")
                        .arg(pid.to_string())
                        .output();
                }
                tracing::info!("Terminated instance {}: pid={}", instance_id, pid);
            }
        }
        Ok(())
    }

    async fn get_system_status(&self) -> Result<app_menu::client::SystemStatus> {
        let instances = self.instances.read().await;
        Ok(app_menu::client::SystemStatus {
            healthy: true,
            uptime_ms: 0,
            active_instances: instances.len(),
            total_apps: 7,
        })
    }
}
