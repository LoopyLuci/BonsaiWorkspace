use mdns_sd::{ServiceDaemon, ServiceInfo};
use reqwest::get;
use std::collections::HashMap;
use std::net::IpAddr;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

pub async fn announce_peer(
    port: u16,
    model_list: Vec<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mdns = ServiceDaemon::new()?;
    let service_type = "_bonsai._tcp.local.";
    let instance_name = format!("bonsai-{}", uuid::Uuid::new_v4());
    let hostname = "localhost".to_string();
    let mut properties = HashMap::new();
    properties.insert("models".to_string(), model_list.join(","));
    let ip = "127.0.0.1".parse::<IpAddr>().unwrap();
    let info = ServiceInfo::new(
        service_type,
        &instance_name,
        &hostname,
        ip,
        port,
        Some(properties.clone()),
    )?;
    mdns.register(info)?;
    Ok(())
}

pub async fn request_model(url: &str, local_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let response = get(url).await?;
    let bytes = response.bytes().await?;
    let mut file = File::create(local_path).await?;
    file.write_all(&bytes).await?;
    Ok(())
}

pub async fn discover_peers() -> Result<Vec<(String, u16, Vec<String>)>, Box<dyn std::error::Error>>
{
    let mdns = ServiceDaemon::new()?;
    let browser = mdns.browse("_bonsai._tcp.local.")?;
    let mut peers = Vec::new();
    // Poll for a short window to discover peers
    for _ in 0..5 {
        if let Ok(event) = browser.recv_timeout(std::time::Duration::from_secs(1)) {
            if let mdns_sd::ServiceEvent::ServiceResolved(info) = event {
                // `get_properties()` returns a reference to TxtProperties. Extract
                // the "models" entry if present and split it into a Vec<String>.
                let props = info.get_properties();
                let models: Vec<String> = if let Some(m) = props.get("models") {
                    m.to_string().split(',').map(|s| s.to_string()).collect()
                } else {
                    Vec::new()
                };
                peers.push((info.get_hostname().to_string(), info.get_port(), models));
            }
        }
    }
    Ok(peers)
}
