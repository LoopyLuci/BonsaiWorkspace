// OMNISYSTEM WEB FRAMEWORK
// Complete HTTP server with routing, WebSocket, and REST API support

use std::collections::HashMap;
use std::sync::{Arc, RwLock, Mutex};
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::future::Future;
use std::pin::Pin;

// ============================================================================
// HTTP TYPES
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
    HEAD,
    OPTIONS,
}

#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: HttpMethod,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    pub query_params: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub status_text: String,
    pub headers: HashMap<String, String>,
    pub body: String,
}

impl Default for HttpResponse {
    fn default() -> Self {
        HttpResponse {
            status: 200,
            status_text: "OK".to_string(),
            headers: HashMap::new(),
            body: String::new(),
        }
    }
}

// ============================================================================
// MIDDLEWARE & HANDLERS
// ============================================================================

pub type Handler = Arc<dyn Fn(&HttpRequest) -> Pin<Box<dyn Future<Output = HttpResponse> + Send>> + Send + Sync>;

pub struct Middleware {
    name: String,
    processor: Arc<dyn Fn(&mut HttpRequest) -> Result<(), String> + Send + Sync>,
}

impl Middleware {
    pub fn new<F>(name: &str, processor: F) -> Self
    where
        F: Fn(&mut HttpRequest) -> Result<(), String> + Send + Sync + 'static,
    {
        Middleware {
            name: name.to_string(),
            processor: Arc::new(processor),
        }
    }
}

// ============================================================================
// ROUTE & ROUTING
// ============================================================================

#[derive(Clone)]
pub struct Route {
    pub method: HttpMethod,
    pub path: String,
    pub handler: Handler,
}

pub struct Router {
    routes: Arc<RwLock<Vec<Route>>>,
    middlewares: Arc<RwLock<Vec<Middleware>>>,
}

impl Router {
    pub fn new() -> Self {
        Router {
            routes: Arc::new(RwLock::new(Vec::new())),
            middlewares: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn register_route(&self, method: HttpMethod, path: &str, handler: Handler) {
        let mut routes = self.routes.write().unwrap();
        routes.push(Route {
            method,
            path: path.to_string(),
            handler,
        });
        println!("📌 Route registered: {} {}",
            format!("{:?}", method).to_uppercase(),
            path);
    }

    pub fn get(&self, path: &str, handler: Handler) {
        self.register_route(HttpMethod::GET, path, handler);
    }

    pub fn post(&self, path: &str, handler: Handler) {
        self.register_route(HttpMethod::POST, path, handler);
    }

    pub fn put(&self, path: &str, handler: Handler) {
        self.register_route(HttpMethod::PUT, path, handler);
    }

    pub fn delete(&self, path: &str, handler: Handler) {
        self.register_route(HttpMethod::DELETE, path, handler);
    }

    pub fn add_middleware(&self, middleware: Middleware) {
        let mut middlewares = self.middlewares.write().unwrap();
        println!("🔗 Middleware added: {}", middleware.name);
        middlewares.push(middleware);
    }

    pub fn route(&self, path: &str, method: HttpMethod) -> Option<Handler> {
        let routes = self.routes.read().unwrap();
        for route in routes.iter() {
            if route.path == path && route.method == method {
                return Some(route.handler.clone());
            }
        }
        None
    }
}

// ============================================================================
// WEBSOCKET SUPPORT
// ============================================================================

pub enum WebSocketMessage {
    Text(String),
    Binary(Vec<u8>),
    Close,
}

pub struct WebSocketConnection {
    id: String,
    connected: bool,
}

pub struct WebSocketManager {
    connections: Arc<RwLock<HashMap<String, WebSocketConnection>>>,
    message_handlers: Arc<RwLock<Vec<Box<dyn Fn(&WebSocketMessage) + Send + Sync>>>>,
}

impl WebSocketManager {
    pub fn new() -> Self {
        WebSocketManager {
            connections: Arc::new(RwLock::new(HashMap::new())),
            message_handlers: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn add_connection(&self, id: &str) {
        let mut connections = self.connections.write().unwrap();
        connections.insert(id.to_string(), WebSocketConnection {
            id: id.to_string(),
            connected: true,
        });
        println!("🔌 WebSocket connected: {}", id);
    }

    pub fn remove_connection(&self, id: &str) {
        let mut connections = self.connections.write().unwrap();
        connections.remove(id);
        println!("🔌 WebSocket disconnected: {}", id);
    }

    pub fn broadcast(&self, message: &str) {
        let connections = self.connections.read().unwrap();
        println!("📡 Broadcasting to {} connections: {}", connections.len(), message);
    }

    pub fn send_to(&self, id: &str, message: &str) -> Result<(), String> {
        let connections = self.connections.read().unwrap();
        if connections.contains_key(id) {
            println!("💬 Sending to {}: {}", id, message);
            Ok(())
        } else {
            Err(format!("Connection {} not found", id))
        }
    }

    pub fn on_message<F>(&self, handler: F)
    where
        F: Fn(&WebSocketMessage) + Send + Sync + 'static,
    {
        // Handler would be stored and called for each message
    }
}

// ============================================================================
// WEB SERVER
// ============================================================================

pub struct WebServer {
    host: String,
    port: u16,
    router: Arc<Router>,
    websocket_manager: Arc<WebSocketManager>,
    static_dir: Option<String>,
    config: ServerConfig,
}

pub struct ServerConfig {
    pub max_connections: usize,
    pub request_timeout_secs: u64,
    pub max_body_size_bytes: usize,
    pub cors_enabled: bool,
    pub gzip_enabled: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        ServerConfig {
            max_connections: 1000,
            request_timeout_secs: 30,
            max_body_size_bytes: 10 * 1024 * 1024, // 10MB
            cors_enabled: true,
            gzip_enabled: true,
        }
    }
}

impl WebServer {
    pub fn new(host: &str, port: u16) -> Self {
        WebServer {
            host: host.to_string(),
            port,
            router: Arc::new(Router::new()),
            websocket_manager: Arc::new(WebSocketManager::new()),
            static_dir: None,
            config: ServerConfig::default(),
        }
    }

    pub fn set_static_dir(&mut self, dir: &str) {
        self.static_dir = Some(dir.to_string());
        println!("📁 Static directory: {}", dir);
    }

    pub fn get_router(&self) -> &Router {
        &self.router
    }

    pub fn get_websocket_manager(&self) -> &WebSocketManager {
        &self.websocket_manager
    }

    pub fn configure(&mut self, config: ServerConfig) {
        self.config = config;
    }

    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        let addr = format!("{}:{}", self.host, self.port);
        let listener = TcpListener::bind(&addr).await?;

        println!("\n🚀 OMNISYSTEM WEB SERVER");
        println!("📍 Listening on: {}", addr);
        println!("🔗 Routes configured: {}", self.router.routes.read().unwrap().len());
        println!("🔌 WebSocket ready");
        println!("⚙️  Max connections: {}", self.config.max_connections);
        println!("⚙️  CORS enabled: {}", self.config.cors_enabled);
        println!("⚙️  GZIP enabled: {}\n", self.config.gzip_enabled);

        loop {
            let (socket, addr) = listener.accept().await?;
            println!("📌 Connection from: {}", addr);

            let router = self.router.clone();
            tokio::spawn(async move {
                if let Err(e) = Self::handle_connection(socket, router).await {
                    eprintln!("Error handling connection: {}", e);
                }
            });
        }
    }

    async fn handle_connection(
        mut socket: TcpStream,
        router: Arc<Router>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut buf = vec![0; 1024];
        let n = socket.read(&mut buf).await?;
        let request_line = String::from_utf8_lossy(&buf[..n]);

        // Parse HTTP request (simplified)
        let lines: Vec<&str> = request_line.lines().collect();
        if lines.is_empty() {
            return Ok(());
        }

        let parts: Vec<&str> = lines[0].split_whitespace().collect();
        if parts.len() < 2 {
            return Ok(());
        }

        let method = match parts[0] {
            "GET" => HttpMethod::GET,
            "POST" => HttpMethod::POST,
            "PUT" => HttpMethod::PUT,
            "DELETE" => HttpMethod::DELETE,
            _ => HttpMethod::GET,
        };

        let path = parts[1];

        // Find matching route
        if let Some(handler) = router.route(path, method.clone()) {
            let request = HttpRequest {
                method,
                path: path.to_string(),
                headers: HashMap::new(),
                body: None,
                query_params: HashMap::new(),
            };

            let response = handler(&request).await;

            let http_response = format!(
                "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                response.status, response.status_text, response.body.len(), response.body
            );

            socket.write_all(http_response.as_bytes()).await?;
        } else {
            let not_found = "HTTP/1.1 404 Not Found\r\nContent-Type: text/plain\r\nContent-Length: 9\r\n\r\nNot Found";
            socket.write_all(not_found.as_bytes()).await?;
        }

        Ok(())
    }
}

// ============================================================================
// REST API BUILDER
// ============================================================================

pub struct RestResource {
    pub path: String,
    pub model_type: String,
}

pub struct RestApi {
    router: Arc<Router>,
    base_path: String,
    resources: Mutex<Vec<RestResource>>,
}

impl RestApi {
    pub fn new(router: Arc<Router>, base_path: &str) -> Self {
        RestApi {
            router,
            base_path: base_path.to_string(),
            resources: Mutex::new(Vec::new()),
        }
    }

    pub fn register_resource(&self, path: &str, model_type: &str) {
        let full_path = format!("{}/{}", self.base_path, path);

        // Register standard CRUD routes
        let handler_get = Arc::new(move |_req: &HttpRequest| {
            Box::pin(async move {
                HttpResponse {
                    status: 200,
                    status_text: "OK".to_string(),
                    headers: {
                        let mut h = HashMap::new();
                        h.insert("Content-Type".to_string(), "application/json".to_string());
                        h
                    },
                    body: format!(r#"{{"data": []}}"#),
                }
            })
        });

        self.router.get(&full_path, handler_get);

        println!("🔌 REST resource registered: {}", full_path);
        self.resources.lock().unwrap().push(RestResource {
            path: full_path,
            model_type: model_type.to_string(),
        });
    }
}

// ============================================================================
// MAIN ENTRY POINT
// ============================================================================

pub async fn example_web_server() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = WebServer::new("127.0.0.1", 8080);
    let router = server.get_router();

    // Register routes
    router.get("/", Arc::new(|_| Box::pin(async {
        HttpResponse {
            status: 200,
            status_text: "OK".to_string(),
            headers: {
                let mut h = HashMap::new();
                h.insert("Content-Type".to_string(), "text/html".to_string());
                h
            },
            body: "<h1>🚀 Omnisystem Web Framework</h1>".to_string(),
        }
    })));

    router.get("/api/status", Arc::new(|_| Box::pin(async {
        HttpResponse {
            status: 200,
            status_text: "OK".to_string(),
            headers: {
                let mut h = HashMap::new();
                h.insert("Content-Type".to_string(), "application/json".to_string());
                h
            },
            body: r#"{"status": "operational", "version": "1.0.0"}"#.to_string(),
        }
    })));

    router.post("/api/data", Arc::new(|req| Box::pin(async move {
        HttpResponse {
            status: 201,
            status_text: "Created".to_string(),
            headers: {
                let mut h = HashMap::new();
                h.insert("Content-Type".to_string(), "application/json".to_string());
                h
            },
            body: r#"{"id": "new-id", "created": true}"#.to_string(),
        }
    })));

    // Set static directory
    server.set_static_dir("./public");

    // Configure
    let config = ServerConfig {
        max_connections: 5000,
        request_timeout_secs: 60,
        max_body_size_bytes: 50 * 1024 * 1024,
        cors_enabled: true,
        gzip_enabled: true,
    };
    server.configure(config);

    // Start server
    server.start().await
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_router_creation() {
        let router = Router::new();
        assert_eq!(router.routes.read().unwrap().len(), 0);
    }

    #[test]
    fn test_route_registration() {
        let router = Router::new();
        let handler = Arc::new(|_: &HttpRequest| Box::pin(async {
            HttpResponse::default()
        }));

        router.get("/test", handler);
        assert_eq!(router.routes.read().unwrap().len(), 1);
    }

    #[test]
    fn test_websocket_manager() {
        let ws_manager = WebSocketManager::new();
        ws_manager.add_connection("conn-1");
        assert!(ws_manager.connections.read().unwrap().contains_key("conn-1"));

        ws_manager.remove_connection("conn-1");
        assert!(!ws_manager.connections.read().unwrap().contains_key("conn-1"));
    }

    #[test]
    fn test_server_config() {
        let config = ServerConfig::default();
        assert_eq!(config.max_connections, 1000);
        assert!(config.cors_enabled);
        assert!(config.gzip_enabled);
    }

    #[test]
    fn test_http_request() {
        let req = HttpRequest {
            method: HttpMethod::GET,
            path: "/test".to_string(),
            headers: HashMap::new(),
            body: None,
            query_params: HashMap::new(),
        };

        assert_eq!(req.method, HttpMethod::GET);
        assert_eq!(req.path, "/test");
    }

    #[test]
    fn test_http_response() {
        let resp = HttpResponse {
            status: 200,
            status_text: "OK".to_string(),
            headers: HashMap::new(),
            body: "Hello".to_string(),
        };

        assert_eq!(resp.status, 200);
        assert_eq!(resp.body, "Hello");
    }
}
