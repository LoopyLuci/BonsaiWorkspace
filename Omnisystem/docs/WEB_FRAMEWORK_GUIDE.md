# Web Framework Guide - Complete Tutorial

**Build HTTP servers, REST APIs, and web services**

---

## Table of Contents
1. [Introduction](#introduction)
2. [Getting Started](#getting-started)
3. [Creating a Server](#creating-a-server)
4. [Routing](#routing)
5. [Handling Requests](#handling-requests)
6. [Sending Responses](#sending-responses)
7. [JSON APIs](#json-apis)
8. [Query Parameters](#query-parameters)
9. [Error Handling](#error-handling)
10. [Advanced Features](#advanced-features)

---

## Introduction

The Web Framework provides everything needed to build production-ready HTTP servers:

- **HTTP/1.1 Support**: Full protocol implementation
- **Routing**: Pattern matching and method-based routing
- **JSON Support**: Automatic serialization/deserialization
- **Multi-threaded**: Handle concurrent connections
- **Type-safe**: Catch errors at compile time

### Quick Facts
- **Language**: TITAN
- **API Style**: RESTful
- **Response Types**: JSON, HTML, plain text
- **Threading**: Multi-threaded per default
- **Overhead**: Low latency, high throughput

---

## Getting Started

### Minimal Server

```titan
use omnisystem::web_framework::*

fun main() -> Result<(), str> {
    let mut router = Router::new()
    
    router.get("/", |_req| {
        HttpResponse::with_text(HttpStatus::Ok, "Hello, World!")
    })
    
    let server = WebServer::new("0.0.0.0:8080")
    server.start()?
    
    Ok(())
}
```

### Run It

```bash
omnisystem run server.ti
# Server listening on 0.0.0.0:8080
```

### Test It

```bash
curl http://localhost:8080/
# Hello, World!
```

---

## Creating a Server

### Basic Setup

```titan
// Create router
let mut router = Router::new()

// Register routes
router.get("/api/users", handle_list_users)
router.post("/api/users", handle_create_user)
router.get("/api/users/:id", handle_get_user)
router.delete("/api/users/:id", handle_delete_user)

// Create server
let server = WebServer::new("127.0.0.1:8080")

// Start listening
server.start()?
```

### Configuration

```titan
// Specify host and port
let server = WebServer::new("0.0.0.0:3000")  // Any interface, port 3000
let server = WebServer::new("127.0.0.1:8080")  // Localhost only
let server = WebServer::new("192.168.1.100:5000")  // Specific IP

// Custom not found handler
router.set_not_found_handler(|_req| {
    HttpResponse::with_text(HttpStatus::NotFound, "Route not found")
})
```

---

## Routing

### HTTP Methods

```titan
// GET request
router.get("/items", handle_list)

// POST request (create)
router.post("/items", handle_create)

// PUT request (replace)
router.put("/items/:id", handle_update)

// PATCH request (partial update)
router.patch("/items/:id", handle_patch)

// DELETE request
router.delete("/items/:id", handle_delete)

// HEAD request
router.head("/status", handle_status)

// OPTIONS request
router.options("/api/*", handle_options)
```

### Route Patterns

```titan
// Static routes
router.get("/api/users", handle_users)
router.get("/health", handle_health)

// Dynamic parameters
router.get("/api/users/:id", |req| {
    // Access parameter: req.path = "/api/users/123"
    HttpResponse::with_text(HttpStatus::Ok, "User 123")
})

// Multiple segments
router.get("/api/users/:id/posts/:post_id", handle_user_post)
```

---

## Handling Requests

### Request Structure

```titan
type HttpRequest {
    method: HttpMethod,
    path: string,
    query_params: HashMap<string, string>,
    headers: HashMap<string, string>,
    body: Vec<u8>,
}
```

### Accessing Request Data

```titan
fun handle_request(req: &HttpRequest) -> HttpResponse {
    // HTTP method
    match req.method {
        HttpMethod::Get => println!("GET request"),
        HttpMethod::Post => println!("POST request"),
        _ => {}
    }
    
    // Path
    println!("Path: {}", req.path)
    
    // Query parameters
    if let Some(name) = req.get_query_param("name") {
        println!("Name: {}", name)
    }
    
    // Headers
    if let Some(auth) = req.get_header("authorization") {
        println!("Auth: {}", auth)
    }
    
    // Body
    let body_str = String::from_utf8(req.body).ok()
    
    HttpResponse::with_text(HttpStatus::Ok, "OK")
}
```

### Example: GET with Query Params

```titan
// Query: /search?q=rust&limit=10
fun search(req: &HttpRequest) -> HttpResponse {
    let query = req.get_query_param("q").unwrap_or("default")
    let limit = req.get_query_param("limit")
        .and_then(|s| { s.parse::<i64>().ok() })
        .unwrap_or(20)
    
    let result = format!(
        "Search: {}, Limit: {}",
        query, limit
    )
    HttpResponse::with_text(HttpStatus::Ok, &result)
}
```

---

## Sending Responses

### Response Types

```titan
// Plain text
let response = HttpResponse::with_text(
    HttpStatus::Ok,
    "Hello, World!"
)

// HTML
let html = "<h1>Welcome</h1>"
let response = HttpResponse::with_html(
    HttpStatus::Ok,
    html
)

// JSON
let data = UserData { id: 1, name: "Alice" }
let response = HttpResponse::with_json(
    HttpStatus::Created,
    &data
)?
```

### Status Codes

```titan
HttpStatus::Ok                  // 200
HttpStatus::Created             // 201
HttpStatus::Accepted            // 202
HttpStatus::NoContent           // 204
HttpStatus::BadRequest          // 400
HttpStatus::Unauthorized        // 401
HttpStatus::Forbidden           // 403
HttpStatus::NotFound            // 404
HttpStatus::MethodNotAllowed    // 405
HttpStatus::Conflict            // 409
HttpStatus::InternalServerError // 500
HttpStatus::NotImplemented      // 501
HttpStatus::ServiceUnavailable  // 503
```

### Custom Headers

```titan
let mut response = HttpResponse::new(HttpStatus::Ok)
response.set_header("Content-Type", "application/json")
response.set_header("X-Custom-Header", "value")
response.set_header("Cache-Control", "no-cache")
response.set_body(b"data".to_vec())
```

---

## JSON APIs

### Serializing JSON

```titan
type User {
    id: i64,
    name: string,
    email: string,
}

fun get_user(req: &HttpRequest) -> Result<HttpResponse, str> {
    let user = User {
        id: 1,
        name: "Alice".to_string(),
        email: "alice@example.com".to_string(),
    }
    
    HttpResponse::with_json(HttpStatus::Ok, &user)
}
```

### Deserializing JSON

```titan
fun create_user(req: &HttpRequest) -> HttpResponse {
    // Parse JSON from request body
    match req.json_body() {
        Ok(json) => {
            // Process JSON
            let response = json::stringify(&json).ok()
            HttpResponse::with_json(
                HttpStatus::Created,
                &response
            ).unwrap_or_else(|_| {
                HttpResponse::with_text(
                    HttpStatus::InternalServerError,
                    "Error"
                )
            })
        },
        Err(_) => {
            HttpResponse::with_text(
                HttpStatus::BadRequest,
                "Invalid JSON"
            )
        }
    }
}
```

### API Response Pattern

```titan
type ApiResponse<T> {
    success: bool,
    data: Option<T>,
    error: Option<string>,
}

fun api_success<T>(data: T) -> ApiResponse<T> {
    ApiResponse {
        success: true,
        data: Some(data),
        error: None,
    }
}

fun api_error(msg: string) -> ApiResponse<null> {
    ApiResponse {
        success: false,
        data: None,
        error: Some(msg),
    }
}
```

---

## Query Parameters

### Parsing Parameters

```titan
fun filter_items(req: &HttpRequest) -> HttpResponse {
    // ?category=electronics&min_price=100&max_price=500
    
    let category = req.get_query_param("category")
    let min_price = req.get_query_param("min_price")
        .and_then(|s| { s.parse::<f64>().ok() })
    let max_price = req.get_query_param("max_price")
        .and_then(|s| { s.parse::<f64>().ok() })
    
    // Use parameters
    let response = format!(
        "Category: {:?}, Price: {:?}-{:?}",
        category, min_price, max_price
    )
    
    HttpResponse::with_text(HttpStatus::Ok, &response)
}
```

### Optional Parameters

```titan
fun list_users(req: &HttpRequest) -> HttpResponse {
    let page = req.get_query_param("page")
        .and_then(|s| { s.parse::<i64>().ok() })
        .unwrap_or(1)
    
    let limit = req.get_query_param("limit")
        .and_then(|s| { s.parse::<i64>().ok() })
        .unwrap_or(20)
    
    let sort = req.get_query_param("sort").unwrap_or("id")
    
    let result = format!("Page: {}, Limit: {}, Sort: {}", page, limit, sort)
    HttpResponse::with_text(HttpStatus::Ok, &result)
}
```

---

## Error Handling

### Request Validation

```titan
fun validate_user(req: &HttpRequest) -> Result<UserData, string> {
    // Parse JSON
    let json = req.json_body()
        .map_err(|_| "Invalid JSON")?
    
    // Validate data
    if json.get("name").is_none() {
        return Err("Missing 'name' field".to_string())
    }
    
    Ok(UserData { /* ... */ })
}

fun create_with_validation(req: &HttpRequest) -> HttpResponse {
    match validate_user(req) {
        Ok(user) => {
            // Create user
            HttpResponse::with_json(HttpStatus::Created, &user)
                .unwrap()
        },
        Err(err) => {
            HttpResponse::with_text(HttpStatus::BadRequest, &err)
        }
    }
}
```

### Error Responses

```titan
fn handle_error(code: HttpStatus, message: str) -> HttpResponse {
    let error_response = ErrorResponse {
        error: message.to_string(),
        code: code.code(),
    }
    
    HttpResponse::with_json(code, &error_response)
        .unwrap_or_else(|_| {
            HttpResponse::with_text(code, message)
        })
}
```

---

## Advanced Features

### Middleware Pattern

```titan
fun add_logging(req: &HttpRequest) -> HttpRequest {
    println!("{} {}", req.method.as_str(), req.path)
    req
}

fun add_auth_check(req: &HttpRequest) -> Result<HttpRequest, HttpResponse> {
    match req.get_header("authorization") {
        Some(_) => Ok(req),
        None => Err(HttpResponse::with_text(
            HttpStatus::Unauthorized,
            "Missing authorization header"
        ))
    }
}
```

### Routing Groups

```titan
fun setup_user_routes(router: &mut Router) {
    router.get("/api/users", list_users)
    router.post("/api/users", create_user)
    router.get("/api/users/:id", get_user)
    router.put("/api/users/:id", update_user)
    router.delete("/api/users/:id", delete_user)
}

fun setup_item_routes(router: &mut Router) {
    router.get("/api/items", list_items)
    router.post("/api/items", create_item)
    // ...
}

fun main() -> Result<(), str> {
    let mut router = Router::new()
    
    setup_user_routes(&mut router)
    setup_item_routes(&mut router)
    
    let server = WebServer::new("0.0.0.0:8080")
    server.start()?
    
    Ok(())
}
```

### Request Timing

```titan
use omnisystem::time::*

fun timed_handler(req: &HttpRequest) -> HttpResponse {
    let start = Instant::now()
    
    // Do work
    let result = expensive_operation()
    
    let elapsed = start.elapsed()
    println!("Handler took: {}ms", elapsed.as_millis())
    
    HttpResponse::with_text(HttpStatus::Ok, &result)
}
```

---

## Complete Example: User API

```titan
use omnisystem::web_framework::*

type User {
    id: i64,
    name: string,
    email: string,
}

let mut users: HashMap<i64, User> = HashMap::new()
let mut next_id = 1

fun list_users(_req: &HttpRequest) -> HttpResponse {
    let user_list: Vec<User> = users.values().cloned().collect()
    HttpResponse::with_json(HttpStatus::Ok, &user_list).unwrap()
}

fun create_user(req: &HttpRequest) -> HttpResponse {
    match req.json_body() {
        Ok(json) => {
            let user = User {
                id: next_id,
                name: get_field(&json, "name").unwrap_or("Unknown"),
                email: get_field(&json, "email").unwrap_or(""),
            }
            next_id += 1
            users.insert(user.id, user.clone())
            HttpResponse::with_json(HttpStatus::Created, &user).unwrap()
        },
        Err(_) => {
            HttpResponse::with_text(HttpStatus::BadRequest, "Invalid JSON")
        }
    }
}

fun get_user(req: &HttpRequest) -> HttpResponse {
    // Parse :id from path
    let id_str = req.path.split('/').last().unwrap_or("0")
    let id: i64 = id_str.parse().unwrap_or(0)
    
    match users.get(id) {
        Some(user) => {
            HttpResponse::with_json(HttpStatus::Ok, user).unwrap()
        },
        None => {
            HttpResponse::with_text(HttpStatus::NotFound, "User not found")
        }
    }
}

fun main() -> Result<(), str> {
    let mut router = Router::new()
    
    router.get("/api/users", list_users)
    router.post("/api/users", create_user)
    router.get("/api/users/:id", get_user)
    
    let server = WebServer::new("0.0.0.0:8080")
    println!("Server running on http://0.0.0.0:8080")
    server.start()?
    
    Ok(())
}
```

---

## Best Practices

✅ **DO**
- Validate all input data
- Use appropriate status codes
- Add meaningful error messages
- Log requests and responses
- Use consistent API response format
- Handle timeouts gracefully
- Document API endpoints

❌ **DON'T**
- Return sensitive data in errors
- Ignore invalid JSON
- Forget error handling
- Use hardcoded paths
- Ignore request validation
- Return 500 for client errors
- Leave handlers without response

---

## Performance Tips

1. **Connection pooling** for databases
2. **Response caching** for static data
3. **Async I/O** for slow operations
4. **Compression** for large responses
5. **Connection timeouts** for slow clients
6. **Rate limiting** for abuse protection

---

## Debugging

### Request Logging

```titan
fun log_request(req: &HttpRequest) {
    println!("{} {} from {:?}", 
        req.method.as_str(), 
        req.path,
        req.get_header("x-forwarded-for")
    )
}

fun log_response(status: HttpStatus, elapsed: Duration) {
    println!("Response: {} in {}ms", 
        status.code(), 
        elapsed.as_millis()
    )
}
```

---

## See Also
- [API_WEB.md](API_WEB.md) - Complete API reference
- [TUTORIAL_WEB_APP.md](TUTORIAL_WEB_APP.md) - Full example app
- [OQL_GUIDE.md](OQL_GUIDE.md) - Query language for APIs

---

**Next**: [TUTORIAL_WEB_APP.md](TUTORIAL_WEB_APP.md) - Build a complete REST API
