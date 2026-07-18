use crate::{BweConfig, BweRequest, BweResponse, HttpMethod, MiddlewareChain, RequestContext, Result, Router};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{error, info};

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
    pub async fn start(self) -> Result<()> {
        let addr = format!("{}:{}", self.config.host, self.config.port);
        let listener = TcpListener::bind(&addr).await?;

        info!(
            "Bonsai Web Engine listening on {} (service: {})",
            addr, self.config.service_name
        );

        let router = Arc::new(self.router);
        let middleware_chain = Arc::new(self.middleware_chain);
        let service_name = self.config.service_name;

        loop {
            let (socket, peer_addr) = listener.accept().await?;
            let router = router.clone();
            let middleware_chain = middleware_chain.clone();
            let service_name = service_name.clone();

            tokio::spawn(async move {
                if let Err(e) = handle_connection(socket, peer_addr, service_name, router, middleware_chain).await {
                    error!("Error handling connection: {}", e);
                }
            });
        }
    }
}

async fn handle_connection(
    mut socket: tokio::net::TcpStream,
    peer_addr: std::net::SocketAddr,
    service_name: String,
    router: Arc<Router>,
    middleware_chain: Arc<MiddlewareChain>,
) -> Result<()> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let mut buf = vec![0u8; 8192];
    let n = socket.read(&mut buf).await?;
    if n == 0 {
        return Ok(());
    }

    let request_text = String::from_utf8_lossy(&buf[..n]).into_owned();
    let mut lines = request_text.lines();
    let request_line = lines.next().unwrap_or("");
    let parts: Vec<&str> = request_line.split_whitespace().collect();

    if parts.len() < 3 {
        socket
            .write_all(b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\n\r\n")
            .await?;
        return Ok(());
    }

    let method = match parts[0] {
        "GET" => HttpMethod::Get,
        "POST" => HttpMethod::Post,
        "PUT" => HttpMethod::Put,
        "DELETE" => HttpMethod::Delete,
        "PATCH" => HttpMethod::Patch,
        "HEAD" => HttpMethod::Head,
        "OPTIONS" => HttpMethod::Options,
        _ => HttpMethod::Get,
    };

    let full_path = parts[1];
    let (path, query) = match full_path.split_once('?') {
        Some((p, q)) => (p.to_string(), parse_query(q)),
        None => (full_path.to_string(), HashMap::new()),
    };

    let mut headers = HashMap::new();
    for line in lines.by_ref() {
        if line.is_empty() {
            break;
        }
        if let Some((key, value)) = line.split_once(':') {
            headers.insert(key.trim().to_string(), value.trim().to_string());
        }
    }

    let mut req = BweRequest::new(method, path, Default::default(), peer_addr.to_string());
    req.query = query;
    req.headers = headers;

    let ctx = RequestContext::new(service_name);

    let response = middleware_chain
        .execute(req, &ctx, move |r, c| {
            let router = router.clone();
            let ctx_owned = c.clone();
            Box::pin(async move { router.route(r, &ctx_owned).await })
        })
        .await
        .unwrap_or_else(|e| BweResponse::internal_error(&e.to_string()));

    write_response(&mut socket, &response).await?;

    Ok(())
}

fn parse_query(q: &str) -> HashMap<String, String> {
    q.split('&')
        .filter_map(|pair| pair.split_once('='))
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

async fn write_response(socket: &mut tokio::net::TcpStream, response: &BweResponse) -> Result<()> {
    use tokio::io::AsyncWriteExt;

    let mut head = format!("HTTP/1.1 {} {}\r\n", response.status, status_text(response.status));
    for (k, v) in &response.headers {
        head.push_str(&format!("{}: {}\r\n", k, v));
    }
    head.push_str(&format!("Content-Length: {}\r\n", response.body.len()));
    head.push_str("\r\n");

    socket.write_all(head.as_bytes()).await?;
    socket.write_all(&response.body).await?;
    Ok(())
}

fn status_text(status: u16) -> &'static str {
    match status {
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        500 => "Internal Server Error",
        _ => "Unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_query_parses_key_value_pairs() {
        let q = parse_query("a=1&b=2&c=hello");
        assert_eq!(q.get("a"), Some(&"1".to_string()));
        assert_eq!(q.get("b"), Some(&"2".to_string()));
        assert_eq!(q.get("c"), Some(&"hello".to_string()));
    }

    #[test]
    fn parse_query_ignores_malformed_pairs() {
        let q = parse_query("valid=1&novalue&");
        assert_eq!(q.len(), 1);
        assert_eq!(q.get("valid"), Some(&"1".to_string()));
    }

    #[test]
    fn status_text_covers_common_codes() {
        assert_eq!(status_text(200), "OK");
        assert_eq!(status_text(404), "Not Found");
        assert_eq!(status_text(500), "Internal Server Error");
        assert_eq!(status_text(999), "Unknown");
    }
}
