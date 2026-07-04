use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceStatus {
    pub name: String,
    pub status: String, // running, starting, stopping, failed, offline
    pub pid: Option<u32>,
    pub port: Option<u16>,
    pub cpu_percent: f64,
    pub memory_mb: f64,
}

pub struct ServiceMonitor {
    api_url: String,
}

impl ServiceMonitor {
    pub fn new(api_url: String) -> Self {
        Self { api_url }
    }

    pub async fn get_status(&self) -> anyhow::Result<serde_json::Value> {
        let client = reqwest::Client::new();
        let url = format!("{}/status", self.api_url);

        let response = client
            .get(&url)
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await?;

        let status = response.json::<serde_json::Value>().await?;
        Ok(status)
    }

    pub async fn get_service(&self, service_name: &str) -> anyhow::Result<ServiceStatus> {
        let client = reqwest::Client::new();
        let url = format!("{}/services/{}", self.api_url, service_name);

        let response = client
            .get(&url)
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await?;

        let status = response.json::<ServiceStatus>().await?;
        Ok(status)
    }

    pub async fn get_all_services(&self) -> anyhow::Result<Vec<ServiceStatus>> {
        let client = reqwest::Client::new();
        let url = format!("{}/services", self.api_url);

        let response = client
            .get(&url)
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await?;

        let services = response.json::<Vec<ServiceStatus>>().await?;
        Ok(services)
    }

    pub async fn restart_service(&self, service_name: &str) -> anyhow::Result<()> {
        let client = reqwest::Client::new();
        let url = format!("{}/services/{}/restart", self.api_url, service_name);

        client
            .post(&url)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await?;

        Ok(())
    }

    pub async fn stop_service(&self, service_name: &str) -> anyhow::Result<()> {
        let client = reqwest::Client::new();
        let url = format!("{}/services/{}/stop", self.api_url, service_name);

        client
            .post(&url)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await?;

        Ok(())
    }

    pub async fn start_service(&self, service_name: &str) -> anyhow::Result<()> {
        let client = reqwest::Client::new();
        let url = format!("{}/services/{}/start", self.api_url, service_name);

        client
            .post(&url)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await?;

        Ok(())
    }
}
