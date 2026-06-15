# Tutorial: Build a REST API with Omnisystem

**Complete walkthrough building a production-ready Todo API**

---

## Overview

We'll build a complete REST API with:
- ✅ POST /todos - Create todo
- ✅ GET /todos - List todos
- ✅ GET /todos/:id - Get single todo
- ✅ PUT /todos/:id - Update todo
- ✅ DELETE /todos/:id - Delete todo
- ✅ GET /health - Health check

**Time**: 30-45 minutes  
**Prerequisites**: TITAN Language Guide, Web Framework Guide  
**Difficulty**: Beginner-Intermediate

---

## Step 1: Project Setup

### Create project structure

```bash
mkdir omnisystem-todos
cd omnisystem-todos
touch main.ti
```

### Create main.ti

```titan
// main.ti - Todo REST API

use omnisystem::web_framework::*
use omnisystem::time::*
use std::collections::HashMap

// Data structures
type Todo {
    id: i64,
    title: string,
    description: string,
    completed: bool,
    created_at: u64,
}

// Application state
type AppState {
    todos: HashMap<i64, Todo>,
    next_id: i64,
}

fun main() -> Result<(), str> {
    println!("Starting Todo API...")
    run_server()
}
```

---

## Step 2: Basic Server

### Add server initialization

```titan
fun run_server() -> Result<(), str> {
    let mut router = Router::new()
    
    // Health check
    router.get("/health", |_req| {
        HttpResponse::with_text(
            HttpStatus::Ok,
            "OK"
        )
    })
    
    let server = WebServer::new("0.0.0.0:3000")
    println!("Server running on http://0.0.0.0:3000")
    
    server.start()
        .map_err(|e| format!("Server error: {:?}", e))
}
```

### Test it

```bash
omnisystem run main.ti
# Server running on http://0.0.0.0:3000

# In another terminal:
curl http://localhost:3000/health
# OK
```

---

## Step 3: Data Storage

### Add in-memory storage

```titan
// Global state (simplified - use Arc<Mutex> in production)
let todos: HashMap<i64, Todo> = HashMap::new()
let next_id: i64 = 1

fun create_todo(title: string, description: string) -> Todo {
    let todo = Todo {
        id: next_id,
        title,
        description,
        completed: false,
        created_at: current_timestamp(),
    }
    
    todos.insert(next_id, todo.clone())
    next_id += 1
    
    todo
}

fun get_todo(id: i64) -> Option<Todo> {
    todos.get(id).cloned()
}

fun current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
```

---

## Step 4: Implement POST /todos

### Create handler

```titan
fun handle_create_todo(req: &HttpRequest) -> HttpResponse {
    // Parse request JSON
    let json = match req.json_body() {
        Ok(j) => j,
        Err(_) => {
            return error_response(
                HttpStatus::BadRequest,
                "Invalid JSON"
            )
        }
    }
    
    // Extract fields
    let title = get_string_field(&json, "title")
        .unwrap_or_else(|| return error_response(
            HttpStatus::BadRequest,
            "Missing 'title' field"
        ))
    
    let description = get_string_field(&json, "description")
        .unwrap_or("")
    
    // Create todo
    let todo = create_todo(title, description)
    
    // Return JSON response
    HttpResponse::with_json(HttpStatus::Created, &todo)
        .unwrap_or_else(|_| server_error())
}

// Register route in run_server()
router.post("/todos", handle_create_todo)
```

### Test it

```bash
curl -X POST http://localhost:3000/todos \
  -H "Content-Type: application/json" \
  -d '{"title":"Buy milk","description":"From the store"}'

# Response:
# {
#   "id": 1,
#   "title": "Buy milk",
#   "description": "From the store",
#   "completed": false,
#   "created_at": 1718445600
# }
```

---

## Step 5: Implement GET /todos (List)

### Create handler

```titan
fun handle_list_todos(_req: &HttpRequest) -> HttpResponse {
    let todo_list: Vec<Todo> = todos.values()
        .cloned()
        .collect()
    
    HttpResponse::with_json(HttpStatus::Ok, &todo_list)
        .unwrap_or_else(|_| server_error())
}

// Register route
router.get("/todos", handle_list_todos)
```

### Test it

```bash
curl http://localhost:3000/todos

# Response:
# [
#   {
#     "id": 1,
#     "title": "Buy milk",
#     ...
#   }
# ]
```

---

## Step 6: Implement GET /todos/:id

### Create handler

```titan
fun handle_get_todo(req: &HttpRequest) -> HttpResponse {
    // Parse ID from path
    let id_str = req.path
        .split('/')
        .last()
        .unwrap_or("0")
    
    let id: i64 = match id_str.parse() {
        Ok(n) => n,
        Err(_) => {
            return error_response(
                HttpStatus::BadRequest,
                "Invalid ID"
            )
        }
    }
    
    // Get todo
    match get_todo(id) {
        Some(todo) => {
            HttpResponse::with_json(HttpStatus::Ok, &todo)
                .unwrap_or_else(|_| server_error())
        },
        None => {
            error_response(
                HttpStatus::NotFound,
                "Todo not found"
            )
        }
    }
}

// Register route
router.get("/todos/:id", handle_get_todo)
```

### Test it

```bash
curl http://localhost:3000/todos/1

# Response:
# {
#   "id": 1,
#   "title": "Buy milk",
#   ...
# }

# Test 404
curl http://localhost:3000/todos/999
# {"error": "Todo not found"}
```

---

## Step 7: Implement PUT /todos/:id

### Create handler

```titan
fun handle_update_todo(req: &HttpRequest) -> HttpResponse {
    // Parse ID
    let id_str = req.path.split('/').last().unwrap_or("0")
    let id: i64 = match id_str.parse() {
        Ok(n) => n,
        Err(_) => return error_response(HttpStatus::BadRequest, "Invalid ID"),
    }
    
    // Parse JSON
    let json = match req.json_body() {
        Ok(j) => j,
        Err(_) => return error_response(HttpStatus::BadRequest, "Invalid JSON"),
    }
    
    // Get existing todo
    let mut todo = match get_todo(id) {
        Some(t) => t,
        None => return error_response(HttpStatus::NotFound, "Todo not found"),
    }
    
    // Update fields
    if let Some(title) = get_string_field(&json, "title") {
        todo.title = title
    }
    
    if let Some(desc) = get_string_field(&json, "description") {
        todo.description = desc
    }
    
    if let Some(completed) = get_bool_field(&json, "completed") {
        todo.completed = completed
    }
    
    // Save updated todo
    todos.insert(id, todo.clone())
    
    HttpResponse::with_json(HttpStatus::Ok, &todo)
        .unwrap_or_else(|_| server_error())
}

// Register route
router.put("/todos/:id", handle_update_todo)
```

### Test it

```bash
curl -X PUT http://localhost:3000/todos/1 \
  -H "Content-Type: application/json" \
  -d '{"completed":true}'

# Response: Updated todo with completed=true
```

---

## Step 8: Implement DELETE /todos/:id

### Create handler

```titan
fun handle_delete_todo(req: &HttpRequest) -> HttpResponse {
    let id_str = req.path.split('/').last().unwrap_or("0")
    let id: i64 = match id_str.parse() {
        Ok(n) => n,
        Err(_) => return error_response(HttpStatus::BadRequest, "Invalid ID"),
    }
    
    match todos.remove(&id) {
        Some(_) => {
            HttpResponse::with_text(
                HttpStatus::NoContent,
                ""
            )
        },
        None => {
            error_response(HttpStatus::NotFound, "Todo not found")
        }
    }
}

// Register route
router.delete("/todos/:id", handle_delete_todo)
```

### Test it

```bash
curl -X DELETE http://localhost:3000/todos/1

# Response: 204 No Content
```

---

## Step 9: Helper Functions

### Add error handling helpers

```titan
type ErrorResponse {
    error: string,
}

fun error_response(status: HttpStatus, message: str) -> HttpResponse {
    let response = ErrorResponse {
        error: message.to_string(),
    }
    
    HttpResponse::with_json(status, &response)
        .unwrap_or_else(|_| {
            HttpResponse::with_text(status, message)
        })
}

fun server_error() -> HttpResponse {
    error_response(
        HttpStatus::InternalServerError,
        "Internal server error"
    )
}

fun get_string_field(json: &serde_json::Value, field: &str) -> Option<string> {
    json.get(field)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

fun get_bool_field(json: &serde_json::Value, field: &str) -> Option<bool> {
    json.get(field)
        .and_then(|v| v.as_bool())
}
```

---

## Step 10: Complete Application

### Full main.ti

```titan
use omnisystem::web_framework::*
use std::collections::HashMap

type Todo {
    id: i64,
    title: string,
    description: string,
    completed: bool,
    created_at: u64,
}

type ErrorResponse {
    error: string,
}

// Global state
let todos: HashMap<i64, Todo> = HashMap::new()
let next_id: i64 = 1

fun main() -> Result<(), str> {
    println!("Starting Todo API on http://0.0.0.0:3000...")
    run_server()
}

fun run_server() -> Result<(), str> {
    let mut router = Router::new()
    
    router.get("/health", |_| {
        HttpResponse::with_text(HttpStatus::Ok, "OK")
    })
    
    router.get("/todos", handle_list_todos)
    router.post("/todos", handle_create_todo)
    router.get("/todos/:id", handle_get_todo)
    router.put("/todos/:id", handle_update_todo)
    router.delete("/todos/:id", handle_delete_todo)
    
    router.set_not_found_handler(|req| {
        error_response(
            HttpStatus::NotFound,
            &format!("Route not found: {} {}", req.method.as_str(), req.path)
        )
    })
    
    let server = WebServer::new("0.0.0.0:3000")
    server.start()
        .map_err(|e| format!("Server error: {:?}", e))
}

fun handle_list_todos(_req: &HttpRequest) -> HttpResponse {
    let todo_list: Vec<Todo> = todos.values().cloned().collect()
    HttpResponse::with_json(HttpStatus::Ok, &todo_list)
        .unwrap_or_else(|_| server_error())
}

fun handle_create_todo(req: &HttpRequest) -> HttpResponse {
    let json = match req.json_body() {
        Ok(j) => j,
        Err(_) => return error_response(HttpStatus::BadRequest, "Invalid JSON"),
    }
    
    let title = match get_string_field(&json, "title") {
        Some(t) => t,
        None => return error_response(HttpStatus::BadRequest, "Missing 'title'"),
    }
    
    let description = get_string_field(&json, "description").unwrap_or("")
    
    let todo = Todo {
        id: next_id,
        title,
        description,
        completed: false,
        created_at: current_timestamp(),
    }
    
    todos.insert(next_id, todo.clone())
    next_id += 1
    
    HttpResponse::with_json(HttpStatus::Created, &todo)
        .unwrap_or_else(|_| server_error())
}

fun handle_get_todo(req: &HttpRequest) -> HttpResponse {
    let id = extract_id_from_path(req.path)
        .map_err(|e| error_response(HttpStatus::BadRequest, e))?
    
    match todos.get(id) {
        Some(todo) => HttpResponse::with_json(HttpStatus::Ok, todo)
            .unwrap_or_else(|_| server_error()),
        None => error_response(HttpStatus::NotFound, "Todo not found"),
    }
}

fun handle_update_todo(req: &HttpRequest) -> HttpResponse {
    let id = extract_id_from_path(req.path)
        .map_err(|e| error_response(HttpStatus::BadRequest, e))?
    
    let json = match req.json_body() {
        Ok(j) => j,
        Err(_) => return error_response(HttpStatus::BadRequest, "Invalid JSON"),
    }
    
    let mut todo = match todos.get(id) {
        Some(t) => t.clone(),
        None => return error_response(HttpStatus::NotFound, "Todo not found"),
    }
    
    if let Some(title) = get_string_field(&json, "title") {
        todo.title = title
    }
    if let Some(desc) = get_string_field(&json, "description") {
        todo.description = desc
    }
    if let Some(completed) = get_bool_field(&json, "completed") {
        todo.completed = completed
    }
    
    todos.insert(id, todo.clone())
    
    HttpResponse::with_json(HttpStatus::Ok, &todo)
        .unwrap_or_else(|_| server_error())
}

fun handle_delete_todo(req: &HttpRequest) -> HttpResponse {
    let id = extract_id_from_path(req.path)
        .map_err(|e| error_response(HttpStatus::BadRequest, e))?
    
    if todos.remove(&id).is_some() {
        HttpResponse::new(HttpStatus::NoContent)
    } else {
        error_response(HttpStatus::NotFound, "Todo not found")
    }
}

fun extract_id_from_path(path: &str) -> Result<i64, string> {
    path.split('/').last()
        .and_then(|s| s.parse::<i64>().ok())
        .ok_or_else(|| "Invalid ID".to_string())
}

fun get_string_field(json: &serde_json::Value, field: &str) -> Option<string> {
    json.get(field)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

fun get_bool_field(json: &serde_json::Value, field: &str) -> Option<bool> {
    json.get(field).and_then(|v| v.as_bool())
}

fun error_response(status: HttpStatus, message: &str) -> HttpResponse {
    let response = ErrorResponse {
        error: message.to_string(),
    }
    HttpResponse::with_json(status, &response)
        .unwrap_or_else(|_| {
            HttpResponse::with_text(status, message)
        })
}

fun server_error() -> HttpResponse {
    error_response(HttpStatus::InternalServerError, "Internal server error")
}

fun current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
```

---

## Testing Checklist

- [ ] Health check returns 200
- [ ] POST /todos creates todo with all fields
- [ ] GET /todos lists all todos
- [ ] GET /todos/:id returns specific todo
- [ ] PUT /todos/:id updates todo correctly
- [ ] DELETE /todos/:id removes todo
- [ ] 404 on non-existent todo
- [ ] 400 on invalid JSON
- [ ] 400 on missing required fields
- [ ] Response Content-Type is application/json

---

## Exercises

### 1. Add filtering
Add `GET /todos?completed=true` to filter todos by status.

### 2. Add pagination
Add `GET /todos?page=1&limit=10` for paginated results.

### 3. Add validation
Validate that title is not empty and not longer than 255 characters.

### 4. Add timestamps
Track `updated_at` in addition to `created_at`.

### 5. Add persistence
Save todos to a JSON file on disk instead of memory.

---

## Next Steps

- Read [API_WEB.md](API_WEB.md) for complete API reference
- Study [WEB_FRAMEWORK_GUIDE.md](WEB_FRAMEWORK_GUIDE.md) for advanced features
- Deploy using [DEPLOYMENT.md](DEPLOYMENT.md) guide

---

**Congratulations!** You've built a complete REST API. From here, add authentication, database storage, and deploy to production.
