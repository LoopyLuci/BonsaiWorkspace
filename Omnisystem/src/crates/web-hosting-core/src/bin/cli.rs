//! CLI demo: register a virtual host and inspect the server's counters.

use chrono::Utc;
use std::collections::HashMap;
use uuid::Uuid;
use web_hosting_core::{DomainName, VirtualHost, VirtualHostId, WebHostingManager, WebServer};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = WebServer::new();

    let vhost = server
        .create_virtual_host(VirtualHost {
            id: VirtualHostId(Uuid::new_v4()),
            domain: DomainName("example.com".to_string()),
            aliases: vec![DomainName("www.example.com".to_string())],
            backend_urls: vec!["http://127.0.0.1:8080".to_string()],
            tls_enabled: false,
            certificate_id: None,
            root_path: "/var/www/example".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            tags: HashMap::new(),
        })
        .await?;
    println!("Registered virtual host: {}", vhost.domain.0);
    println!("Total virtual hosts: {}", server.vhost_count());

    Ok(())
}
