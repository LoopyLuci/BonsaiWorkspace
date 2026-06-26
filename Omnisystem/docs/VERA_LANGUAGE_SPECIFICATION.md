# VERA Language Specification v1.0
## The Omnisystem UI/Component/Reactive Language

---

## 1. OVERVIEW

**VERA** is a next-generation reactive UI and component language designed to replace React, Vue, Flutter, SwiftUI, and Jetpack Compose. It combines:
- Declarative component definitions
- Reactive state management (built-in, not framework)
- Type-safe prop and event handling
- Zero-runtime overhead (compiles to native code)
- Responsive design first-class feature
- Hot reloading & live editing support

### Design Principles
1. **Reactive by Default** - State changes automatically re-render
2. **Declarative** - Describe UI, not how to build it
3. **Type Safe** - All prop types checked at compile time
4. **Performance First** - Virtual DOM is optional, can target native
5. **Composable** - Small, reusable components build complex UIs
6. **Responsive Native** - No media queries needed, layout constraints solve it

---

## 2. SYNTAX

### 2.1 Basic Component

```vera
// Simple functional component
component Button {
    prop label: string
    prop onClick: fn() -> void
    
    view {
        <button class="btn" on:click=onClick>
            {label}
        </button>
    }
}

// Using the component
<Button label="Click me" onClick={handleClick} />
```

### 2.2 State Management

```vera
component Counter {
    // Reactive state (automatically triggers re-render on change)
    state count: i32 = 0
    
    fn increment() {
        count = count + 1  // Automatically reactive
    }
    
    fn decrement() {
        count = count - 1
    }
    
    view {
        <div class="counter">
            <button on:click=decrement>-</button>
            <span>{count}</span>
            <button on:click=increment>+</button>
        </div>
    }
}
```

### 2.3 Computed Properties

```vera
component Product {
    prop price: f64
    prop quantity: i32
    
    // Computed value (cached, updates when dependencies change)
    computed total: f64 = price * (quantity as f64)
    
    // Computed with logic
    computed discount: f64 {
        if quantity >= 10 {
            return total * 0.1
        }
        if quantity >= 5 {
            return total * 0.05
        }
        return 0.0
    }
    
    view {
        <div>
            <p>Price: ${price}</p>
            <p>Quantity: {quantity}</p>
            <p>Total: ${total}</p>
            <p>Discount: ${discount}</p>
        </div>
    }
}
```

### 2.4 Event Handling

```vera
component Form {
    state username: string = ""
    state email: string = ""
    state submitted: bool = false
    
    fn handle_change(field: string, value: string) {
        if field == "username" {
            username = value
        } else if field == "email" {
            email = value
        }
    }
    
    fn handle_submit(event: SubmitEvent) -> void {
        event.prevent_default()
        submitted = true
        submit_form(username, email)
    }
    
    view {
        <form on:submit=handle_submit>
            <input 
                type="text"
                placeholder="Username"
                value=username
                on:input={(e) => handle_change("username", e.target.value)}
            />
            
            <input
                type="email"
                placeholder="Email"
                value=email
                on:input={(e) => handle_change("email", e.target.value)}
            />
            
            {submitted && <p class="success">Form submitted!</p>}
            
            <button type="submit">Submit</button>
        </form>
    }
}
```

### 2.5 Conditional Rendering

```vera
component UserProfile {
    prop user_id: i32
    
    state user: User? = null
    state loading: bool = true
    
    on_mounted {
        fetch_user(user_id)
    }
    
    view {
        {loading && <LoadingSpinner />}
        
        {!loading && user && (
            <div class="profile">
                <h1>{user.name}</h1>
                <p>{user.bio}</p>
            </div>
        )}
        
        {!loading && !user && <NotFound />}
    }
}
```

### 2.6 Lists and Iteration

```vera
component TodoList {
    state todos: [Todo] = []
    state filter: string = "all"
    
    computed filtered_todos: [Todo] {
        if filter == "completed" {
            return todos.filter(fn(t) { return t.completed })
        }
        if filter == "pending" {
            return todos.filter(fn(t) { return !t.completed })
        }
        return todos
    }
    
    fn toggle_todo(id: i32) {
        for mut todo in todos {
            if todo.id == id {
                todo.completed = !todo.completed
                break
            }
        }
    }
    
    view {
        <ul class="todo-list">
            {filtered_todos.map(fn(todo) {
                <li 
                    key=todo.id
                    class={todo.completed ? "completed" : ""}
                >
                    <input
                        type="checkbox"
                        checked=todo.completed
                        on:change={() => toggle_todo(todo.id)}
                    />
                    <span>{todo.title}</span>
                </li>
            })}
        </ul>
    }
}
```

### 2.7 Props & Component Communication

```vera
component Parent {
    state message: string = "Hello"
    
    fn handle_child_message(msg: string) {
        message = msg
    }
    
    view {
        <div>
            <h1>{message}</h1>
            <Child 
                title="Child Component"
                on_send_message=handle_child_message
            />
        </div>
    }
}

component Child {
    prop title: string
    prop on_send_message: fn(string) -> void
    
    fn send() {
        on_send_message("Message from child")
    }
    
    view {
        <div>
            <h2>{title}</h2>
            <button on:click=send>Send Message</button>
        </div>
    }
}
```

### 2.8 Slot System (Content Distribution)

```vera
component Card {
    prop title: string
    
    slot header
    slot body
    slot footer
    
    view {
        <div class="card">
            <div class="card-header">
                {title}
                <slot.header />
            </div>
            <div class="card-body">
                <slot.body />
            </div>
            <div class="card-footer">
                <slot.footer />
            </div>
        </div>
    }
}

// Using Card with slots
<Card title="My Card">
    <slot.header>
        <button>X</button>
    </slot.header>
    
    <slot.body>
        <p>Card content goes here</p>
    </slot.body>
    
    <slot.footer>
        <button>OK</button>
    </slot.footer>
</Card>
```

### 2.9 Styling

```vera
component StyledButton {
    prop variant: string = "primary"
    
    style {
        .btn {
            padding: 10px 20px
            border: none
            border-radius: 4px
            cursor: pointer
            font-size: 16px
            transition: all 0.3s
        }
        
        .btn.primary {
            background: #007bff
            color: white
        }
        
        .btn.primary:hover {
            background: #0056b3
        }
        
        .btn.secondary {
            background: #6c757d
            color: white
        }
        
        .btn.secondary:hover {
            background: #545b62
        }
    }
    
    view {
        <button class={`btn ${variant}`}>
            Click me
        </button>
    }
}
```

### 2.10 Lifecycle Hooks

```vera
component DataFetcher {
    prop url: string
    
    state data: string = ""
    state error: string? = null
    state loading: bool = false
    
    fn fetch_data() {
        loading = true
        error = null
        
        match fetch(url) {
            Ok(response) => {
                data = response.text()
                loading = false
            },
            Error(e) => {
                error = e.message
                loading = false
            }
        }
    }
    
    // Run when component mounts
    on_mounted {
        fetch_data()
    }
    
    // Run when props change
    on_props_changed {
        fetch_data()
    }
    
    // Run before component unmounts
    on_unmount {
        // Cleanup if needed
        cancel_pending_requests()
    }
    
    view {
        {loading && <p>Loading...</p>}
        {error && <p class="error">{error}</p>}
        {!loading && data && <div>{data}</div>}
    }
}
```

### 2.11 Custom Hooks

```vera
// Reusable stateful logic
hook use_form(initial_values: [string: string]) -> (
    values: [string: string],
    set_value: fn(string, string) -> void,
    reset: fn() -> void
) {
    state values = initial_values.clone()
    
    fn set_value(name: string, value: string) {
        values[name] = value
    }
    
    fn reset() {
        values = initial_values.clone()
    }
    
    return (values, set_value, reset)
}

// Using custom hook
component LoginForm {
    let (values, set_value, reset) = use_form({
        "username": "",
        "password": ""
    })
    
    fn handle_submit() {
        authenticate(values["username"], values["password"])
        reset()
    }
    
    view {
        <form on:submit=handle_submit>
            <input
                value=values["username"]
                on:input={(e) => set_value("username", e.target.value)}
            />
            <input
                type="password"
                value=values["password"]
                on:input={(e) => set_value("password", e.target.value)}
            />
            <button type="submit">Login</button>
        </form>
    }
}
```

### 2.12 Context & State Sharing

```vera
// Create a context for global state
context ThemeContext {
    theme: string = "light"
    fn toggle_theme() -> void
}

component App {
    provide ThemeContext {
        theme: "dark",
        toggle_theme: fn() {
            theme = theme == "dark" ? "light" : "dark"
        }
    }
    
    view {
        <div>
            <Header />
            <Content />
            <Footer />
        </div>
    }
}

component Header {
    // Consume context
    inject theme: string from ThemeContext
    
    view {
        <header class={theme}>
            <h1>My App</h1>
        </header>
    }
}
```

---

## 3. TEMPLATE SYNTAX (JSX-like)

### 3.1 Element Syntax

```vera
// Simple element
<div></div>

// With attributes
<div id="main" class="container" style="color: red">
    Content
</div>

// Self-closing
<img src="image.png" alt="Image" />

// Dynamic content
<div>{variable}</div>
<div>{expression + 1}</div>

// Event handlers
<button on:click={handler}>Click</button>
<input on:input={(e) => set_value(e.target.value)} />

// Conditional rendering
<div>
    {condition ? <p>True</p> : <p>False</p>}
</div>

// List rendering
<ul>
    {items.map(fn(item) {
        <li key={item.id}>{item.name}</li>
    })}
</ul>

// Component
<MyComponent prop1="value" prop2={variable} />

// Spread props
let props = {label: "Click", onClick: handler}
<Button ...props />
```

### 3.2 Attribute Binding

```vera
// Static
<div class="box"></div>

// Dynamic
<div class={is_active ? "active" : "inactive"}></div>

// Template literals
<div class={`item ${selected ? "selected" : ""}`}></div>

// Two-way binding
<input value=name on:input={(e) => name = e.target.value} />
```

---

## 4. TYPE SYSTEM

### 4.1 Component Type Definition

```vera
// Component type
type ButtonProps = {
    label: string,
    onClick: fn() -> void,
    disabled: bool?
}

// Using in component
component Button(props: ButtonProps) {
    view {
        <button disabled=props.disabled on:click=props.onClick>
            {props.label}
        </button>
    }
}
```

### 4.2 Generic Components

```vera
component List<T> {
    prop items: [T]
    prop render_item: fn(T) -> VNode
    
    view {
        <ul>
            {items.map(fn(item) {
                <li key={item.id}>
                    {render_item(item)}
                </li>
            })}
        </ul>
    }
}

// Using generic component
<List<User>
    items=users
    render_item={fn(user) {
        <span>{user.name} ({user.email})</span>
    }}
/>
```

### 4.3 Event Types

```vera
type MouseEvent {
    target: HTMLElement,
    client_x: i32,
    client_y: i32,
    prevent_default: fn() -> void,
    stop_propagation: fn() -> void
}

type ChangeEvent {
    target: HTMLElement,
    value: string
}

type KeyboardEvent {
    key: string,
    key_code: i32,
    shift: bool,
    ctrl: bool,
    alt: bool
}

type FormSubmitEvent {
    prevent_default: fn() -> void
}

type FocusEvent {
    target: HTMLElement
}
```

---

## 5. REACTIVITY MODEL

### 5.1 Reactive State

```vera
component ReactivityExample {
    // Automatic reactivity
    state count: i32 = 0
    state message: string = "Hello"
    
    // When count changes, component re-renders
    fn increment() {
        count = count + 1  // Triggers update
    }
    
    // Computed automatically re-evaluates
    computed doubled: i32 = count * 2
    
    view {
        <div>
            <p>{message}</p>
            <p>Count: {count}</p>
            <p>Doubled: {doubled}</p>
            <button on:click=increment>Increment</button>
        </div>
    }
}
```

### 5.2 Dependency Tracking

```vera
component DependencyTracking {
    state user_id: i32 = 1
    state user: User? = null
    
    // Re-fetch whenever user_id changes
    on_props_changed(user_id) {
        fetch_user(user_id)
    }
    
    // Only re-compute when dependencies change
    computed user_display: string {
        if user {
            return `${user.name} (${user.email})`
        }
        return "No user"
    }
    
    view {
        <div>{user_display}</div>
    }
}
```

### 5.3 Watchers

```vera
component Watcher {
    state search: string = ""
    state results: [Result] = []
    
    // Watch a value and react to changes
    watch search {
        if search.len() > 2 {
            search_api(search)
        } else {
            results = []
        }
    }
    
    fn search_api(query: string) {
        match api_search(query) {
            Ok(items) => results = items,
            Error(_) => results = []
        }
    }
    
    view {
        <div>
            <input value=search on:input={(e) => search = e.target.value} />
            <ul>
                {results.map(fn(r) {
                    <li>{r.title}</li>
                })}
            </ul>
        </div>
    }
}
```

---

## 6. STANDARD LIBRARY

### 6.1 DOM API

```vera
// Query elements
let el = document.query_selector("#main")
let els = document.query_selector_all(".item")

// Create elements
let div = document.create_element("div")
div.set_text_content("Hello")
div.set_class("container")

// Manipulate
div.add_class("active")
div.remove_class("disabled")
div.set_attribute("data-id", "123")
let value = div.get_attribute("id")

// Listen to events
element.add_event_listener("click", fn(event) {
    print("Clicked!")
})
```

### 6.2 HTTP Requests

```vera
// GET request
match fetch("https://api.example.com/users") {
    Ok(response) => {
        let json = response.json()
        print(json)
    },
    Error(e) => print("Error: {}", e)
}

// POST request
let body = {
    "name": "John",
    "email": "john@example.com"
}

match fetch_post("https://api.example.com/users", body) {
    Ok(response) => print("Created"),
    Error(e) => print("Error: {}", e)
}
```

### 6.3 Local Storage

```vera
// Store data
local_storage.set("user_id", "123")
local_storage.set("theme", "dark")

// Retrieve data
let user_id = local_storage.get("user_id")
let theme = local_storage.get("theme")

// Remove data
local_storage.remove("user_id")
local_storage.clear()
```

### 6.4 Animation

```vera
component Animated {
    state opacity: f64 = 1.0
    
    fn fade_out() {
        animate(
            target: &opacity,
            from: 1.0,
            to: 0.0,
            duration: 300,  // milliseconds
            easing: "ease-in-out",
            on_complete: fn() {
                print("Animation done")
            }
        )
    }
    
    view {
        <div style={`opacity: ${opacity}`}>
            <button on:click=fade_out>Fade Out</button>
        </div>
    }
}
```

---

## 7. RESPONSIVE DESIGN

### 7.1 Constraint-Based Layout

```vera
component ResponsiveGrid {
    style {
        .grid {
            display: grid
            grid-template-columns: repeat(auto-fit, minmax(300px, 1fr))
            gap: 20px
        }
        
        .card {
            padding: 20px
            border: 1px solid #eee
            border-radius: 8px
        }
    }
    
    view {
        <div class="grid">
            {items.map(fn(item) {
                <div class="card">
                    {item.title}
                </div>
            })}
        </div>
    }
}
```

### 7.2 Media Query Alternatives (Responsive State)

```vera
component ResponsiveNav {
    state screen_size: string = "desktop"  // "mobile", "tablet", "desktop"
    
    on_mounted {
        update_screen_size()
        window.on_resize(fn() {
            update_screen_size()
        })
    }
    
    fn update_screen_size() {
        let width = window.inner_width
        if width < 600 {
            screen_size = "mobile"
        } else if width < 1024 {
            screen_size = "tablet"
        } else {
            screen_size = "desktop"
        }
    }
    
    view {
        {screen_size == "mobile" && <MobileNav />}
        {screen_size == "tablet" && <TabletNav />}
        {screen_size == "desktop" && <DesktopNav />}
    }
}
```

---

## 8. COMPILATION

### 8.1 Compilation Target

```
VERA Component
    ↓
[Parser] → Component AST
    ↓
[Type Checker] → Typed Component AST
    ↓
[Optimizer] → Optimized AST
    ↓
[Code Generator] → JavaScript/WebAssembly/Native Code
    ↓
[Runtime] → Interactive Component
```

### 8.2 Output Formats

```vera
// Compilation targets:
// 1. Web (JavaScript/WebAssembly)
// 2. Native (iOS/Android via native bindings)
// 3. Desktop (Windows/macOS/Linux via native bindings)
// 4. VR/AR (via native bindings)

@compile_target("web")
component WebButton { ... }

@compile_target("native")
component NativeButton { ... }
```

---

## 9. EXAMPLE PROGRAMS

### 9.1 Simple Counter

```vera
component Counter {
    state count: i32 = 0
    
    view {
        <div class="counter">
            <button on:click={() => count -= 1}>-</button>
            <span class="count">{count}</span>
            <button on:click={() => count += 1}>+</button>
        </div>
    }
}
```

### 9.2 Todo App

```vera
component TodoApp {
    state todos: [Todo] = []
    state input_value: string = ""
    
    fn add_todo() {
        if input_value.len() > 0 {
            todos.push(Todo {
                id: todos.len() + 1,
                title: input_value,
                completed: false
            })
            input_value = ""
        }
    }
    
    fn remove_todo(id: i32) {
        todos = todos.filter(fn(t) { return t.id != id })
    }
    
    fn toggle_todo(id: i32) {
        for mut todo in todos {
            if todo.id == id {
                todo.completed = !todo.completed
            }
        }
    }
    
    view {
        <div class="todo-app">
            <h1>My Todos</h1>
            
            <div class="input-group">
                <input
                    value=input_value
                    on:input={(e) => input_value = e.target.value}
                    on:key_press={(e) => {
                        if e.key == "Enter" { add_todo() }
                    }}
                    placeholder="Add a new todo..."
                />
                <button on:click=add_todo>Add</button>
            </div>
            
            <ul class="todo-list">
                {todos.map(fn(todo) {
                    <li key=todo.id class={todo.completed ? "completed" : ""}>
                        <input
                            type="checkbox"
                            checked=todo.completed
                            on:change={() => toggle_todo(todo.id)}
                        />
                        <span>{todo.title}</span>
                        <button on:click={() => remove_todo(todo.id)}>Delete</button>
                    </li>
                })}
            </ul>
        </div>
    }
}
```

### 9.3 Data Dashboard

```vera
component Dashboard {
    state data: DashboardData? = null
    state loading: bool = true
    state error: string? = null
    
    on_mounted {
        fetch_dashboard_data()
    }
    
    fn fetch_dashboard_data() {
        loading = true
        error = null
        
        match fetch_api("/api/dashboard") {
            Ok(response) => {
                data = response
                loading = false
            },
            Error(e) => {
                error = e.message
                loading = false
            }
        }
    }
    
    view {
        <div class="dashboard">
            <h1>Dashboard</h1>
            
            {loading && (
                <div class="loading">
                    <p>Loading...</p>
                </div>
            )}
            
            {error && (
                <div class="error">
                    <p>Error: {error}</p>
                    <button on:click=fetch_dashboard_data>Retry</button>
                </div>
            )}
            
            {data && (
                <div class="content">
                    <div class="stats">
                        {data.metrics.map(fn(metric) {
                            <div class="metric-card">
                                <h3>{metric.name}</h3>
                                <p class="value">{metric.value}</p>
                                <p class="change" class={metric.change > 0 ? "positive" : "negative"}>
                                    {metric.change > 0 ? "+" : ""}{metric.change}%
                                </p>
                            </div>
                        })}
                    </div>
                    
                    <div class="charts">
                        {data.charts.map(fn(chart) {
                            <Chart 
                                title=chart.title
                                data=chart.data
                            />
                        })}
                    </div>
                </div>
            )}
        </div>
    }
}
```

---

## 10. DESIGN PHILOSOPHY

VERA is built on the principle that **UI development should be declarative, reactive, and safe**. Every feature is designed to prevent common UI bugs:

- **Type Safety**: All props and events are type-checked
- **Reactivity**: State changes automatically update UI
- **Performance**: Efficient rendering, no unnecessary re-renders
- **Composability**: Small components combine into complex UIs
- **Accessibility**: Built-in support for ARIA and semantic HTML
- **Responsive**: No media queries needed, constraint-based
- **Maintainability**: Clear component structure, easy to understand

---

This specification enables VERA to be the definitive UI language for the next 100 years.
