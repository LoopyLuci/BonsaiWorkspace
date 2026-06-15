# Web Framework API Reference

**Complete API reference for HTTP server, routing, and response handling**

---

## Module Overview

The Web Framework provides:
- **HTTP Protocol Support**: Full HTTP/1.1 implementation
- **Routing System**: Method-based request routing
- **Response Building**: JSON, HTML, text responses
- **Request Parsing**: Headers, query params, body parsing
- **Multi-threading**: Concurrent request handling

---

## Core Types

### HttpMethod

**Enum representing HTTP request methods**

```rust
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
}

// Methods
impl HttpMethod {
    pub fn from_str(s: &str) -> Option<Self>
    pub fn as_str(&self) -> &'static str
}
```

**Example:**
```rust
let method = HttpMethod::from_str("GET")?  // Some(Get)
println!("{}", method.as_str())  // "GET"
```

---

### HttpStatus

**Enum representing HTTP status codes**

```rust
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

// Methods
impl HttpStatus {
    pub fn code(&self) -> u16
    pub fn reason(&self) -> &'static str
}
```

**Example:**
```rust
let status = HttpStatus::NotFound
println!("{} {}", status.code(), status.reason())  // "404 Not Found"
```

---

### HttpRequest

**Represents an incoming HTTP request**

```rust
pub struct HttpRequest {
    pub method: HttpMethod,
    pub path: String,
    pub query_params: HashMap<String, String>,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

// Methods
impl HttpRequest {
    pub fn parse(raw: &str) -> Result<Self, ParseError>
    pub fn get_header(&self, name: &str) -> Option<String>
    pub fn get_query_param(&self, name: &str) -> Option<String>
    pub fn json_body(&self) -> Result<serde_json::Value, JsonError>
}
```

**Example:**
```rust
let request = HttpRequest::parse(raw_http)?
let auth = request.get_header("authorization")?
let user_id = request.get_query_param("user_id")?
let json = request.json_body()?
```

---

### HttpResponse

**Represents an HTTP response to send back**

```rust
pub struct HttpResponse {
    pub status: HttpStatus,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

// Creation
impl HttpResponse {
    pub fn new(status: HttpStatus) -> Self
    pub fn with_text(status: HttpStatus, text: &str) -> Self
    pub fn with_html(status: HttpStatus, html: &str) -> Self
    pub fn with_json<T: Serialize>(
        status: HttpStatus, 
        data: &T
    ) -> Result<Self, JsonError>
}

// Methods
impl HttpResponse {
    pub fn set_header(&mut self, name: &str, value: &str)
    pub fn set_body(&mut self, body: Vec<u8>)
    pub fn to_string(&self) -> String
    pub fn to_bytes(&self) -> Vec<u8>
}
```

**Example:**
```rust
let response = HttpResponse::with_text(HttpStatus::Ok, "Success")
response.set_header("X-Custom", "value")
let bytes = response.to_bytes()
```

---

### Router

**Routes incoming requests to handler functions**

```rust
pub struct Router {
    routes: HashMap<(HttpMethod, String), RouteHandler>,
    not_found_handler: RouteHandler,
}

// Creation and routing
impl Router {
    pub fn new() -> Self
    pub fn get<F>(&mut self, path: &str, handler: F) where F: Fn(&HttpRequest) -> HttpResponse + Send + Sync + 'static
    pub fn post<F>(&mut self, path: &str, handler: F) where F: Fn(&HttpRequest) -> HttpResponse + Send + Sync + 'static
    pub fn put<F>(&mut self, path: &str, handler: F) where F: Fn(&HttpRequest) -> HttpResponse + Send + Sync + 'static
    pub fn patch<F>(&mut self, path: &str, handler: F) where F: Fn(&HttpRequest) -> HttpResponse + Send + Sync + 'static
    pub fn delete<F>(&mut self, path: &str, handler: F) where F: Fn(&HttpRequest) -> HttpResponse + Send + Sync + 'static
    pub fn head<F>(&mut self, path: &str, handler: F) where F: Fn(&HttpRequest) -> HttpResponse + Send + Sync + 'static
    pub fn options<F>(&mut self, path: &str, handler: F) where F: Fn(&HttpRequest) -> HttpResponse + Send + Sync + 'static
    
    pub fn set_not_found_handler<F>(&mut self, handler: F) where F: Fn(&HttpRequest) -> HttpResponse + Send + Sync + 'static
    pub fn handle(&self, request: &HttpRequest) -> HttpResponse
}
```

**Example:**
```rust
let mut router = Router::new()
router.get("/api/users", |req| {
    HttpResponse::with_text(HttpStatus::Ok, "Users")
})
router.set_not_found_handler(|_| {
    HttpResponse::with_text(HttpStatus::NotFound, "Not Found")
})
let response = router.handle(&request)
```

---

### WebServer

**HTTP server that listens and routes requests**

```rust
pub struct WebServer {
    router: Arc<Mutex<Router>>,
    address: String,
}

// Creation and execution
impl WebServer {
    pub fn new(address: &str) -> Self
    pub fn get_router(&self) -> Arc<Mutex<Router>>
    pub fn start(&self) -> Result<(), ServerError>
}
```

**Example:**
```rust
let server = WebServer::new("0.0.0.0:8080")
let router = server.get_router()
{
    let mut r = router.lock().unwrap()
    r.get("/", |_| HttpResponse::with_text(HttpStatus::Ok, "Home"))
}
server.start()?  // Blocks and listens
```

---

## Error Types

### ParseError

**Errors during HTTP request parsing**

```rust
pub enum ParseError {
    EmptyRequest,
    InvalidRequestLine,
    UnknownMethod,
    InvalidHeader,
}
```

### JsonError

**Errors during JSON processing**

```rust
pub enum JsonError {
    InvalidJson,
    InvalidUtf8,
    SerializationFailed,
    DeserializationFailed,
}
```

### ServerError

**Errors during server operation**

```rust
pub enum ServerError {
    BindFailed(String),
    ReadFailed(String),
    WriteFailed(String),
    ParseFailed,
}
```

---

## Helper Functions

### Query String Parsing

```rust
// Parse "key1=value1&key2=value2"
fn parse_query_string(query: &str) -> HashMap<String, String>
```

**Example:**
```rust
let params = parse_query_string("search=rust&limit=10")
// {
//   "search" => "rust",
//   "limit" => "10"
// }
```

---

## Constants

### HTTP Status Codes (as u16)

```rust
pub const OK: u16 = 200
pub const CREATED: u16 = 201
pub const ACCEPTED: u16 = 202
pub const NO_CONTENT: u16 = 204
pub const BAD_REQUEST: u16 = 400
pub const UNAUTHORIZED: u16 = 401
pub const FORBIDDEN: u16 = 403
pub const NOT_FOUND: u16 = 404
pub const METHOD_NOT_ALLOWED: u16 = 405
pub const CONFLICT: u16 = 409
pub const INTERNAL_SERVER_ERROR: u16 = 500
pub const NOT_IMPLEMENTED: u16 = 501
pub const SERVICE_UNAVAILABLE: u16 = 503
```

---

## Usage Patterns

### Basic Request Handler

```rust
fn handler(req: &HttpRequest) -> HttpResponse {
    println!("Request: {} {}", req.method.as_str(), req.path);
    HttpResponse::with_text(HttpStatus::Ok, "OK")
}
```

### Request with Validation

```rust
fn create_user(req: &HttpRequest) -> HttpResponse {
    let json = match req.json_body() {
        Ok(j) => j,
        Err(_) => {
            return HttpResponse::with_text(
                HttpStatus::BadRequest,
                "Invalid JSON"
            )
        }
    };
    
    HttpResponse::with_json(HttpStatus::Created, &json)
        .unwrap_or_else(|_| {
            HttpResponse::with_text(
                HttpStatus::InternalServerError,
                "Error"
            )
        })
}
```

### Query Parameter Handling

```rust
fn search(req: &HttpRequest) -> HttpResponse {
    let query = req.get_query_param("q")
        .unwrap_or_else(|| "default".to_string());
    
    let limit = req.get_query_param("limit")
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(20);
    
    let response = format!("Query: {}, Limit: {}", query, limit);
    HttpResponse::with_text(HttpStatus::Ok, &response)
}
```

### Header Handling

```rust
fn protected(req: &HttpRequest) -> HttpResponse {
    match req.get_header("authorization") {
        Some(token) if token.starts_with("Bearer ") => {
            HttpResponse::with_text(HttpStatus::Ok, "Authorized")
        }
        _ => {
            HttpResponse::with_text(
                HttpStatus::Unauthorized,
                "Missing or invalid authorization"
            )
        }
    }
}
```

### Custom Response with Headers

```rust
fn download(req: &HttpRequest) -> HttpResponse {
    let mut response = HttpResponse::new(HttpStatus::Ok);
    response.set_header("Content-Type", "application/octet-stream");
    response.set_header("Content-Disposition", "attachment; filename=\"file.bin\"");
    response.set_body(b"file contents".to_vec());
    response
}
```

---

## Complete Server Example

```rust
use omnisystem::web_framework::*

fn main() -> Result<(), ServerError> {
    let mut router = Router::new()
    
    // GET /
    router.get("/", |_req| {
        HttpResponse::with_html(
            HttpStatus::Ok,
            "<h1>Welcome</h1>"
        )
    })
    
    // GET /api/users
    router.get("/api/users", |_req| {
        let users = vec![
            ("Alice", 100),
            ("Bob", 95),
        ]
        HttpResponse::with_json(HttpStatus::Ok, &users)
            .unwrap()
    })
    
    // GET /api/users/:id
    router.get("/api/users/:id", |req| {
        let id = req.path.split('/').last().unwrap_or("0")
        let response = format!("User {}", id)
        HttpResponse::with_text(HttpStatus::Ok, &response)
    })
    
    // POST /api/users
    router.post("/api/users", |req| {
        match req.json_body() {
            Ok(json) => HttpResponse::with_json(HttpStatus::Created, &json).unwrap(),
            Err(_) => HttpResponse::with_text(
                HttpStatus::BadRequest,
                "Invalid JSON"
            )
        }
    })
    
    // 404 Handler
    router.set_not_found_handler(|req| {
        let msg = format!("Not Found: {} {}", req.method.as_str(), req.path)
        HttpResponse::with_text(HttpStatus::NotFound, &msg)
    })
    
    let server = WebServer::new("0.0.0.0:8080")
    println!("Server running on http://0.0.0.0:8080")
    server.start()
}
```

---

## Testing

### Test Request Parsing

```rust
#[test]
fn test_parse_request() {
    let raw = "GET /path HTTP/1.1\r\nHost: localhost\r\n\r\n"
    let req = HttpRequest::parse(raw).unwrap()
    assert_eq!(req.method, HttpMethod::Get)
    assert_eq!(req.path, "/path")
}
```

### Test Response Building

```rust
#[test]
fn test_response_building() {
    let resp = HttpResponse::with_text(HttpStatus::Ok, "Hello")
    assert_eq!(resp.status.code(), 200)
    assert_eq!(resp.body, b"Hello")
}
```

### Test Routing

```rust
#[test]
fn test_routing() {
    let mut router = Router::new()
    router.get("/test", |_| {
        HttpResponse::with_text(HttpStatus::Ok, "OK")
    })
    
    let req = HttpRequest {
        method: HttpMethod::Get,
        path: "/test".to_string(),
        query_params: HashMap::new(),
        headers: HashMap::new(),
        body: Vec::new(),
    }
    
    let response = router.handle(&req)
    assert_eq!(response.status.code(), 200)
}
```

---

## Thread Safety

All types are **Send + Sync** and safe for use across threads:
- Use `Arc<Mutex<Router>>` for shared mutable router
- Use `Arc<dyn Fn>` for shared handler functions
- Requests are handled in separate threads automatically

---

## See Also
- [WEB_FRAMEWORK_GUIDE.md](WEB_FRAMEWORK_GUIDE.md) - Tutorial and examples
- [TUTORIAL_WEB_APP.md](TUTORIAL_WEB_APP.md) - Complete working application
- [API reference index](README.md#api-references) - All API docs

---

**Last Updated**: 2026-06-15
