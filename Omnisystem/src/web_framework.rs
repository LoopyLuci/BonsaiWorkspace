// WEB FRAMEWORK - Full-stack HTTP server and routing
// Production-ready web application framework
// Version: 2.0

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::net::{TcpListener, TcpStream};
use std::io::{Read, Write};

/// HTTP Methods
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
}

impl HttpMethod {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "GET" => Some(HttpMethod::Get),
            "POST" => Some(HttpMethod::Post),
            "PUT" => Some(HttpMethod::Put),
            "PATCH" => Some(HttpMethod::Patch),
            "DELETE" => Some(HttpMethod::Delete),
            "HEAD" => Some(HttpMethod::Head),
            "OPTIONS" => Some(HttpMethod::Options),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Patch => "PATCH",
            HttpMethod::Delete => "DELETE",
            HttpMethod::Head => "HEAD",
            HttpMethod::Options => "OPTIONS",
        }
    }
}

/// HTTP Status Codes
#[derive(Debug, Clone, Copy)]
pub enum HttpStatus {
    Ok = 200,
    Created = 201,
    Accepted = 202,
    NoContent = 204,
    BadRequest = 400,
    Unauthorized = 401,
    Forbidden = 403,
    NotFound = 404,
    MethodNotAllowed = 405,
    Conflict = 409,
    InternalServerError = 500,
    NotImplemented = 501,
    ServiceUnavailable = 503,
}

impl HttpStatus {
    pub fn reason(&self) -> &'static str {
        match self {
            HttpStatus::Ok => "OK",
            HttpStatus::Created => "Created",
            HttpStatus::Accepted => "Accepted",
            HttpStatus::NoContent => "No Content",
            HttpStatus::BadRequest => "Bad Request",
            HttpStatus::Unauthorized => "Unauthorized",
            HttpStatus::Forbidden => "Forbidden",
            HttpStatus::NotFound => "Not Found",
            HttpStatus::MethodNotAllowed => "Method Not Allowed",
            HttpStatus::Conflict => "Conflict",
            HttpStatus::InternalServerError => "Internal Server Error",
            HttpStatus::NotImplemented => "Not Implemented",
            HttpStatus::ServiceUnavailable => "Service Unavailable",
        }
    }

    pub fn code(&self) -> u16 {
        *self as u16
    }
}

/// HTTP Request
#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: HttpMethod,
    pub path: String,
    pub query_params: HashMap<String, String>,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl HttpRequest {
    pub fn parse(raw: &str) -> Result<Self, ParseError> {
        let lines: Vec<&str> = raw.lines().collect();
        if lines.is_empty() {
            return Err(ParseError::EmptyRequest);
        }

        // Parse request line
        let request_line_parts: Vec<&str> = lines[0].split_whitespace().collect();
        if request_line_parts.len() < 2 {
            return Err(ParseError::InvalidRequestLine);
        }

        let method = HttpMethod::from_str(request_line_parts[0])
            .ok_or(ParseError::UnknownMethod)?;

        let path_and_query = request_line_parts[1];
        let (path, query_params) = if let Some(pos) = path_and_query.find('?') {
            let path = path_and_query[..pos].to_string();
            let query_str = &path_and_query[pos + 1..];
            let params = parse_query_string(query_str);
            (path, params)
        } else {
            (path_and_query.to_string(), HashMap::new())
        };

        // Parse headers
        let mut headers = HashMap::new();
        let mut body_start = 0;

        for (i, line) in lines.iter().enumerate().skip(1) {
            if line.is_empty() {
                body_start = i + 1;
                break;
            }

            if let Some(colon_pos) = line.find(':') {
                let key = line[..colon_pos].trim().to_lowercase();
                let value = line[colon_pos + 1..].trim().to_string();
                headers.insert(key, value);
            }
        }

        // Parse body
        let body = if body_start > 0 && body_start < lines.len() {
            lines[body_start..].join("\n").into_bytes()
        } else {
            Vec::new()
        };

        Ok(HttpRequest {
            method,
            path,
            query_params,
            headers,
            body,
        })
    }

    pub fn json_body(&self) -> Result<serde_json::Value, JsonError> {
        let body_str = String::from_utf8(self.body.clone())
            .map_err(|_| JsonError::InvalidUtf8)?;
        serde_json::from_str(&body_str)
            .map_err(|_| JsonError::InvalidJson)
    }

    pub fn get_header(&self, name: &str) -> Option<String> {
        self.headers.get(&name.to_lowercase()).cloned()
    }

    pub fn get_query_param(&self, name: &str) -> Option<String> {
        self.query_params.get(name).cloned()
    }
}

/// HTTP Response
#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: HttpStatus,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl HttpResponse {
    pub fn new(status: HttpStatus) -> Self {
        HttpResponse {
            status,
            headers: HashMap::new(),
            body: Vec::new(),
        }
    }

    pub fn with_json<T: serde_json::Serialize>(status: HttpStatus, data: &T) -> Result<Self, JsonError> {
        let json = serde_json::to_string(data)
            .map_err(|_| JsonError::SerializationFailed)?;

        let mut response = HttpResponse::new(status);
        response.set_header("Content-Type", "application/json");
        response.body = json.into_bytes();
        Ok(response)
    }

    pub fn with_html(status: HttpStatus, html: &str) -> Self {
        let mut response = HttpResponse::new(status);
        response.set_header("Content-Type", "text/html; charset=utf-8");
        response.body = html.as_bytes().to_vec();
        response
    }

    pub fn with_text(status: HttpStatus, text: &str) -> Self {
        let mut response = HttpResponse::new(status);
        response.set_header("Content-Type", "text/plain; charset=utf-8");
        response.body = text.as_bytes().to_vec();
        response
    }

    pub fn set_header(&mut self, name: &str, value: &str) {
        self.headers.insert(name.to_string(), value.to_string());
    }

    pub fn set_body(&mut self, body: Vec<u8>) {
        self.body = body;
    }

    pub fn to_string(&self) -> String {
        let mut result = format!(
            "HTTP/1.1 {} {}\r\n",
            self.status.code(),
            self.status.reason()
        );

        // Add content length
        result.push_str(&format!("Content-Length: {}\r\n", self.body.len()));

        // Add headers
        for (key, value) in &self.headers {
            result.push_str(&format!("{}: {}\r\n", key, value));
        }

        result.push_str("\r\n");
        result
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = self.to_string().into_bytes();
        bytes.extend_from_slice(&self.body);
        bytes
    }
}

/// Route Handler
pub type RouteHandler = Arc<dyn Fn(&HttpRequest) -> HttpResponse + Send + Sync>;

/// Router
pub struct Router {
    routes: HashMap<(HttpMethod, String), RouteHandler>,
    not_found_handler: RouteHandler,
}

impl Router {
    pub fn new() -> Self {
        Router {
            routes: HashMap::new(),
            not_found_handler: Arc::new(|_| {
                HttpResponse::with_text(HttpStatus::NotFound, "Not Found")
            }),
        }
    }

    pub fn get<F>(&mut self, path: &str, handler: F)
    where
        F: Fn(&HttpRequest) -> HttpResponse + Send + Sync + 'static,
    {
        self.routes.insert(
            (HttpMethod::Get, path.to_string()),
            Arc::new(handler),
        );
    }

    pub fn post<F>(&mut self, path: &str, handler: F)
    where
        F: Fn(&HttpRequest) -> HttpResponse + Send + Sync + 'static,
    {
        self.routes.insert(
            (HttpMethod::Post, path.to_string()),
            Arc::new(handler),
        );
    }

    pub fn put<F>(&mut self, path: &str, handler: F)
    where
        F: Fn(&HttpRequest) -> HttpResponse + Send + Sync + 'static,
    {
        self.routes.insert(
            (HttpMethod::Put, path.to_string()),
            Arc::new(handler),
        );
    }

    pub fn delete<F>(&mut self, path: &str, handler: F)
    where
        F: Fn(&HttpRequest) -> HttpResponse + Send + Sync + 'static,
    {
        self.routes.insert(
            (HttpMethod::Delete, path.to_string()),
            Arc::new(handler),
        );
    }

    pub fn patch<F>(&mut self, path: &str, handler: F)
    where
        F: Fn(&HttpRequest) -> HttpResponse + Send + Sync + 'static,
    {
        self.routes.insert(
            (HttpMethod::Patch, path.to_string()),
            Arc::new(handler),
        );
    }

    pub fn set_not_found_handler<F>(&mut self, handler: F)
    where
        F: Fn(&HttpRequest) -> HttpResponse + Send + Sync + 'static,
    {
        self.not_found_handler = Arc::new(handler);
    }

    pub fn handle(&self, request: &HttpRequest) -> HttpResponse {
        if let Some(handler) = self.routes.get(&(request.method, request.path.clone())) {
            handler(request)
        } else {
            (self.not_found_handler)(request)
        }
    }
}

/// Web Server
pub struct WebServer {
    router: Arc<Mutex<Router>>,
    address: String,
}

impl WebServer {
    pub fn new(address: &str) -> Self {
        WebServer {
            router: Arc::new(Mutex::new(Router::new())),
            address: address.to_string(),
        }
    }

    pub fn get_router(&self) -> Arc<Mutex<Router>> {
        self.router.clone()
    }

    pub fn start(&self) -> Result<(), ServerError> {
        let listener = TcpListener::bind(&self.address)
            .map_err(|e| ServerError::BindFailed(e.to_string()))?;

        println!("Server listening on {}", self.address);

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let router = self.router.clone();
                    std::thread::spawn(move || {
                        let _ = handle_connection(stream, router);
                    });
                }
                Err(e) => {
                    eprintln!("Connection error: {}", e);
                }
            }
        }

        Ok(())
    }
}

fn handle_connection(
    mut stream: TcpStream,
    router: Arc<Mutex<Router>>,
) -> Result<(), ServerError> {
    let mut buffer = [0; 4096];
    let n = stream
        .read(&mut buffer)
        .map_err(|e| ServerError::ReadFailed(e.to_string()))?;

    let request_str = String::from_utf8_lossy(&buffer[..n]);
    let request = HttpRequest::parse(&request_str)
        .map_err(|_| ServerError::ParseFailed)?;

    let router = router.lock().unwrap();
    let response = router.handle(&request);

    stream
        .write_all(&response.to_bytes())
        .map_err(|e| ServerError::WriteFailed(e.to_string()))?;

    Ok(())
}

fn parse_query_string(query: &str) -> HashMap<String, String> {
    let mut params = HashMap::new();
    for pair in query.split('&') {
        if let Some(eq_pos) = pair.find('=') {
            let key = pair[..eq_pos].to_string();
            let value = pair[eq_pos + 1..].to_string();
            params.insert(key, value);
        }
    }
    params
}

/// JSON Module (placeholder for serde_json)
pub mod json {
    use serde_json;

    pub fn parse(s: &str) -> Result<serde_json::Value, String> {
        serde_json::from_str(s).map_err(|e| e.to_string())
    }

    pub fn stringify<T: serde::Serialize>(v: &T) -> Result<String, String> {
        serde_json::to_string(v).map_err(|e| e.to_string())
    }
}

/// Errors
#[derive(Debug)]
pub enum ParseError {
    EmptyRequest,
    InvalidRequestLine,
    UnknownMethod,
    InvalidHeader,
}

#[derive(Debug)]
pub enum JsonError {
    InvalidJson,
    InvalidUtf8,
    SerializationFailed,
    DeserializationFailed,
}

#[derive(Debug)]
pub enum ServerError {
    BindFailed(String),
    ReadFailed(String),
    WriteFailed(String),
    ParseFailed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_method_from_str() {
        assert_eq!(HttpMethod::from_str("GET"), Some(HttpMethod::Get));
        assert_eq!(HttpMethod::from_str("POST"), Some(HttpMethod::Post));
        assert_eq!(HttpMethod::from_str("PUT"), Some(HttpMethod::Put));
    }

    #[test]
    fn test_http_status_code() {
        assert_eq!(HttpStatus::Ok.code(), 200);
        assert_eq!(HttpStatus::NotFound.code(), 404);
        assert_eq!(HttpStatus::InternalServerError.code(), 500);
    }

    #[test]
    fn test_parse_query_string() {
        let params = parse_query_string("key1=value1&key2=value2");
        assert_eq!(params.get("key1"), Some(&"value1".to_string()));
        assert_eq!(params.get("key2"), Some(&"value2".to_string()));
    }

    #[test]
    fn test_http_request_parse() {
        let raw = "GET /path HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let req = HttpRequest::parse(raw).unwrap();
        assert_eq!(req.method, HttpMethod::Get);
        assert_eq!(req.path, "/path");
    }

    #[test]
    fn test_http_response_creation() {
        let response = HttpResponse::with_text(HttpStatus::Ok, "Hello");
        assert_eq!(response.status.code(), 200);
        assert_eq!(response.body, b"Hello");
    }

    #[test]
    fn test_router_get_route() {
        let mut router = Router::new();
        router.get("/test", |_| HttpResponse::with_text(HttpStatus::Ok, "OK"));

        let req = HttpRequest {
            method: HttpMethod::Get,
            path: "/test".to_string(),
            query_params: HashMap::new(),
            headers: HashMap::new(),
            body: Vec::new(),
        };

        let response = router.handle(&req);
        assert_eq!(response.status.code(), 200);
    }

    #[test]
    fn test_router_not_found() {
        let router = Router::new();

        let req = HttpRequest {
            method: HttpMethod::Get,
            path: "/nonexistent".to_string(),
            query_params: HashMap::new(),
            headers: HashMap::new(),
            body: Vec::new(),
        };

        let response = router.handle(&req);
        assert_eq!(response.status.code(), 404);
    }
}
