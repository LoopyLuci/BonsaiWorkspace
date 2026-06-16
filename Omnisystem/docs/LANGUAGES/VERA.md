# VERA Language Guide
## Web Development Language | 1,200+ Functions
**Status:** ✅ Production Ready | **Tier:** Frontend & PWA Framework

---

## Overview

**VERA** is the web development language for building modern, reactive web applications. It provides everything needed for single-page apps (SPAs), Progressive Web Apps (PWAs), and real-time collaboration.

### Key Characteristics
- **Reactive System:** State changes automatically update UI
- **Component-Based:** Reusable, composable UI building blocks
- **Type-Safe:** Full TypeScript-style type safety for JavaScript
- **Performance:** Virtual DOM with efficient diffing
- **Offline-First:** Progressive Web App support built-in
- **Real-Time:** WebSocket and CRDT support for collaboration

### Best Use Cases
- Single-page applications (React-like)
- Progressive Web Apps (PWAs)
- Real-time collaborative tools (Figma-like)
- Dashboards and admin panels
- Desktop-like web experiences
- Offline-first applications

---

## Core Features

### 1. Reactive Components

#### Basic Component
```vera
pub component Counter {
    let count = reactive(0);
    
    fn increment() {
        count.value += 1;
    }
    
    fn decrement() {
        count.value -= 1;
    }
    
    render() {
        html! {
            <div class="counter">
                <p>Count: {count}</p>
                <button on:click={increment}>+</button>
                <button on:click={decrement}>-</button>
            </div>
        }
    }
}
```

#### With Props and Slots
```vera
pub component Button {
    prop label: String;
    prop on_click: fn();
    prop disabled: bool = false;
    
    render() {
        html! {
            <button 
                disabled={disabled}
                on:click={on_click}
            >
                {label}
            </button>
        }
    }
}

// Usage
<Button label="Click me" on_click={handle_click} />
```

#### Computed Properties
```vera
pub component PersonCard {
    let first_name = reactive("John");
    let last_name = reactive("Doe");
    
    // Automatically recomputes when dependencies change
    let full_name = computed(() => {
        format!("{} {}", first_name.value, last_name.value)
    });
    
    let age = reactive(30);
    let is_adult = computed(() => age.value >= 18);
    
    render() {
        html! {
            <div>
                <p>Name: {full_name}</p>
                <p>Adult: {is_adult}</p>
            </div>
        }
    }
}
```

### 2. Hooks (Composition API)

#### useState Hook
```vera
fn use_counter(initial: i32) -> (i32, fn(), fn()) {
    let count = reactive(initial);
    
    fn increment() { count.value += 1; }
    fn decrement() { count.value -= 1; }
    
    return (count.value, increment, decrement);
}

pub component App {
    let (count, inc, dec) = use_counter(0);
    
    render() {
        html! {
            <div>
                <p>{count}</p>
                <button on:click={inc}>+</button>
                <button on:click={dec}>-</button>
            </div>
        }
    }
}
```

#### useEffect Hook
```vera
pub component FetchData {
    let data = reactive(Option::None);
    let loading = reactive(true);
    let error = reactive(Option::None);
    
    use_effect(|| {
        // Fetch data when component mounts
        async {
            match http_get("/api/data").await {
                Ok(response) => {
                    data.value = Some(response.body);
                    loading.value = false;
                }
                Err(e) => {
                    error.value = Some(e);
                    loading.value = false;
                }
            }
        }
    }, vec![]);  // Empty dependency array = run once
    
    render() {
        if loading.value {
            html! { <p>Loading...</p> }
        } else if let Some(err) = error.value {
            html! { <p>Error: {err}</p> }
        } else if let Some(d) = data.value {
            html! { <p>Data: {d}</p> }
        } else {
            html! { <p>No data</p> }
        }
    }
}
```

### 3. State Management

#### Store Pattern
```vera
pub struct AppState {
    user: Option<User>,
    items: Vec<Item>,
    loading: bool,
}

pub enum AppAction {
    SetUser(User),
    AddItem(Item),
    SetLoading(bool),
    Clear,
}

pub fn app_reducer(state: AppState, action: AppAction) -> AppState {
    match action {
        AppAction::SetUser(user) => {
            state.user = Some(user);
            state
        }
        AppAction::AddItem(item) => {
            state.items.push(item);
            state
        }
        AppAction::SetLoading(loading) => {
            state.loading = loading;
            state
        }
        AppAction::Clear => {
            AppState { user: None, items: vec![], loading: false }
        }
    }
}

pub component App {
    let (state, dispatch) = use_reducer(app_reducer, initial_state);
    
    fn load_user() {
        dispatch(AppAction::SetLoading(true));
        // ... fetch user
        dispatch(AppAction::SetUser(user));
        dispatch(AppAction::SetLoading(false));
    }
    
    render() {
        // ...
    }
}
```

### 4. Routing

#### Client-Side Router
```vera
pub component App {
    let router = create_router(vec![
        Route {
            path: "/",
            component: HomePage,
            guard: None,
        },
        Route {
            path: "/users/:id",
            component: UserPage,
            guard: Some(require_auth),
        },
        Route {
            path: "/admin",
            component: AdminPanel,
            guard: Some(require_admin),
        },
        Route {
            path: "/*",
            component: NotFound,
            guard: None,
        },
    ]);
    
    render() {
        html! {
            <div>
                <nav>
                    <RouterLink to="/">Home</RouterLink>
                    <RouterLink to="/users/123">User</RouterLink>
                    <RouterLink to="/admin">Admin</RouterLink>
                </nav>
                
                <RouterView router={router} />
            </div>
        }
    }
}
```

#### Route Parameters
```vera
pub component UserPage {
    let route_params = use_route_params();
    let user_id = route_params.get("id")?;
    
    let user = reactive(Option::None);
    
    use_effect(|| {
        async {
            let u = fetch_user(user_id).await?;
            user.value = Some(u);
        }
    }, vec![user_id]);
    
    render() {
        if let Some(u) = user.value {
            html! { <div>{u.name}</div> }
        } else {
            html! { <p>Loading...</p> }
        }
    }
}
```

### 5. HTTP Client

#### REST API
```vera
pub component DataFetcher {
    let client = create_http_client("https://api.example.com");
    
    // Add interceptors
    client.add_interceptor(|request| {
        request.headers.insert("Authorization", "Bearer token");
        request
    });
    
    let data = reactive(Option::None);
    
    use_effect(|| {
        async {
            match client.get("/api/data").await {
                Ok(response) => {
                    let json = parse_json(response.body)?;
                    data.value = Some(json);
                }
                Err(e) => println!("Error: {}", e),
            }
        }
    }, vec![]);
    
    render() {
        // ...
    }
}
```

#### GraphQL
```vera
pub component GraphQLQuery {
    let client = create_graphql_client("https://api.example.com/graphql");
    
    let query = r#"
        query GetUser($id: ID!) {
            user(id: $id) {
                id
                name
                email
            }
        }
    "#;
    
    let result = client.query(
        query,
        map!("id" => "123"),
    ).await?;
    
    render() {
        html! { <div>{result}</div> }
    }
}
```

### 6. Forms

#### Form Handling
```vera
pub component LoginForm {
    let email = reactive("".to_string());
    let password = reactive("".to_string());
    let errors = reactive(vec![]);
    let loading = reactive(false);
    
    async fn handle_submit() {
        errors.value.clear();
        loading.value = true;
        
        // Validate
        if email.value.is_empty() {
            errors.value.push("Email required");
        }
        if password.value.len() < 8 {
            errors.value.push("Password must be 8+ chars");
        }
        
        if !errors.value.is_empty() {
            loading.value = false;
            return;
        }
        
        // Submit
        match http_post("/api/login", LoginRequest {
            email: email.value.clone(),
            password: password.value.clone(),
        }).await {
            Ok(response) => {
                // Success
                navigate("/dashboard");
            }
            Err(e) => {
                errors.value.push(format!("Login failed: {}", e));
            }
        }
        
        loading.value = false;
    }
    
    render() {
        html! {
            <form on:submit={handle_submit}>
                <input 
                    type="email"
                    bind:value={email}
                    placeholder="Email"
                />
                <input 
                    type="password"
                    bind:value={password}
                    placeholder="Password"
                />
                {errors.value.iter().map(|e| html! { <p>{e}</p> })}
                <button disabled={loading.value}>
                    {if loading.value { "Logging in..." } else { "Login" }}
                </button>
            </form>
        }
    }
}
```

### 7. Progressive Web Apps

#### Service Worker
```vera
pub component PWAApp {
    use_effect(|| {
        // Register service worker
        register_service_worker("/sw.js").await?;
        
        // Enable offline mode
        enable_offline_mode();
        
        // Handle background sync
        register_background_sync("sync_data", || {
            sync_with_server()
        });
        
        // Handle push notifications
        on_push_notification(|notification| {
            show_notification(notification.title, notification.body);
        });
    }, vec![]);
    
    render() {
        html! { <MainApp /> }
    }
}
```

#### Offline Support
```vera
pub async fn fetch_with_fallback(url: String) -> Result<Vec<u8>> {
    // Try network first
    match http_get(&url).await {
        Ok(response) => {
            // Cache response for offline
            cache_response(&url, response.clone()).await?;
            Ok(response.body)
        }
        Err(_) => {
            // Fall back to cache
            cache_get(&url).await
                .ok_or("No cached data available".into())
        }
    }
}
```

### 8. Real-Time Collaboration

#### CRDT-Based Collaboration
```vera
pub component CollaborativeDocument {
    let document = reactive(CRDTText::new());
    let cursor = reactive(0);
    
    // Open WebSocket
    let ws = WebSocket::connect("wss://collab.example.com/doc/123")?;
    
    // Handle remote changes
    on_message(ws, |message| {
        let operation: CRDTOperation = parse_json(message)?;
        document.value.apply(operation);
    });
    
    fn insert_text(text: String) {
        let op = document.value.insert(cursor.value, text);
        
        // Apply locally
        document.value.apply(op.clone());
        
        // Send to remote
        ws.send(op.to_json())?;
    }
    
    render() {
        html! {
            <textarea 
                value={document.value.text()}
                on:input={|e| insert_text(e.target.value)}
            />
        }
    }
}
```

---

## Standard Library (1,200+ Functions)

### Components (300+)
- Functional components with hooks
- Web components compatibility
- Fragment and Portal support
- Suspense and Error Boundary
- Lazy loading and code splitting

### Hooks (150+)
- useState, useEffect, useReducer
- useContext, useMemo, useCallback
- useRef, useLayoutEffect
- Custom hooks and composition

### State Management (150+)
- Store pattern with reducers
- Middleware system
- DevTools integration
- Time-travel debugging
- Selectors and computed state

### Routing (120+)
- Client-side router
- Route guards and middleware
- Dynamic route matching
- Nested routes
- Lazy-loaded routes

### HTTP (150+)
- REST client
- GraphQL client
- Request/response interceptors
- Retry and timeout
- Request pooling

### Forms (100+)
- Form validation
- Field state management
- Custom input components
- File uploads
- Multi-step forms

### Styling (100+)
- CSS modules
- CSS-in-JS
- Tailwind CSS integration
- Responsive helpers
- Theme switching

---

## Performance & Best Practices

### Virtual DOM Optimization
```vera
// Use keys for lists to maintain component state
pub component UserList {
    let users = reactive(vec![]);
    
    render() {
        html! {
            <ul>
                {users.value.iter().map(|user| html! {
                    <li key={user.id}>
                        {user.name}
                    </li>
                })}
            </ul>
        }
    }
}
```

### Code Splitting
```vera
pub let HomePage = lazy_load(() => import("./pages/Home"));
pub let UserPage = lazy_load(() => import("./pages/User"));

pub component App {
    let router = create_router(vec![
        Route { path: "/", component: HomePage },
        Route { path: "/users/:id", component: UserPage },
    ]);
}
```

### Memoization
```vera
pub let UserCard = memo(|user: User| {
    html! {
        <div>
            <h3>{user.name}</h3>
            <p>{user.email}</p>
        </div>
    }
}, |user1: User, user2: User| {
    user1.id == user2.id  // Custom equality
});
```

---

## Code Examples

### Example 1: Todo App with Local Storage
```vera
pub component TodoApp {
    let todos = reactive(load_from_storage("todos") || vec![]);
    let input = reactive("".to_string());
    
    fn add_todo() {
        todos.value.push(Todo {
            id: generate_id(),
            title: input.value.clone(),
            completed: false,
        });
        input.value = "".to_string();
        save_to_storage("todos", todos.value.clone());
    }
    
    fn toggle_todo(id: String) {
        if let Some(todo) = todos.value.iter_mut().find(|t| t.id == id) {
            todo.completed = !todo.completed;
        }
        save_to_storage("todos", todos.value.clone());
    }
    
    render() {
        html! {
            <div>
                <input 
                    bind:value={input}
                    on:keydown={|e| {
                        if e.key == "Enter" { add_todo(); }
                    }}
                />
                <ul>
                    {todos.value.iter().map(|todo| html! {
                        <li 
                            key={todo.id}
                            on:click={|| toggle_todo(todo.id.clone())}
                            class:completed={todo.completed}
                        >
                            {todo.title}
                        </li>
                    })}
                </ul>
            </div>
        }
    }
}
```

---

**VERA: Building Modern, Reactive Web Applications**

🚀 [Back to Language Guide](../LANGUAGES.md)
