use crate::{BweConfig, BweRequest, BweResponse, Handler, HttpMethod, MiddlewareChain, RequestContext, Result, Router};
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{info, error};

/// Main Bonsai Web Engine server
pub struct BweServer {
    config: BweConfig,
    router: Router,
    middleware_chain: MiddlewareChain,
}

impl BweServer {
    pub fn new(config: BweConfig, router: Router, middleware_chain: MiddlewareChain) -> Self {
        Self {
            config,
            router,
            middleware_chain,
        }
    }

    /// Start the server and listen for incoming connections
    pub async fn start(mut self) -> Result<()> {
        let addr = format!("{}:{}", self.config.host, self.config.port);
        let listener = TcpListener::bind(&addr).await?;

        info!(
            "Bonsai Web Engine listening on {} (service: {})",
            addr, self.config.service_name
        );

        loop {
            let (socket, peer_addr) = listener.accept().await?;
            let config = self.config.clone();
            let service_name = self.config.service_name.clone();

            // Create a simple HTTP handler
            tokio::spawn(async move {
                if let Err(e) = handle_connection(socket, peer_addr, service_name).await {
                    error!("Error handling connection: {}", e);
                }
            });
        }
    }
}

async fn handle_connection(
    socket: tokio::net::TcpStream,
    peer_addr: std::net::SocketAddr,
    service_name: String,
) -> Result<()> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let mut buf = vec![0; 4096];
    let n = socket.readable().await.map(|_| 0)?; // Simplified

    let request_line = String::from_utf8_lossy(&buf[..n]);
    let parts: Vec<&str> = request_line.lines().next().unwrap_or("").split_whitespace().collect();

    if parts.len() < 3 {
        return Ok(());
    }

    let method = match parts[0] {
        "GET" => HttpMethod::Get,
        "POST" => HttpMethod::Post,
        "PUT" => HttpMethod::Put,
        "DELETE" => HttpMethod::Delete,
        "PATCH" => HttpMethod::Patch,
        _ => HttpMethod::Get,
    };

    let path = parts[1].to_string();

    let req = BweRequest::new(method, path, Default::default(), peer_addr.to_string());
    let ctx = RequestContext::new(service_name);

    // Simple response
    let _response = BweResponse::ok("Hello, Bonsai Web Engine!");

    let mut socket = socket;
    let response_text = "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\nHello, Bonsai!";
    socket.write_all(response_text.as_bytes()).await?;

    Ok(())
}
