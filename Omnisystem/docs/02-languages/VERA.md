# VERA Guide - Web Development

**VERA** is Omnisystem's web development language, optimized for building responsive, interactive web applications.

## Overview

- **Purpose**: Web development, user interfaces
- **Model**: React-like functional components
- **State**: Hooks system (useState, useEffect)
- **Routing**: Client-side routing

## Core Features

### 1. Functional Components
```vera
component Counter() {
    let [count, set_count] = useState(0);
    
    return (
        <div>
            <p>Count: {count}</p>
            <button onClick={|_| set_count(count + 1)}>
                Increment
            </button>
        </div>
    );
}
```

### 2. Hooks
```vera
// useState - manage component state
let [value, set_value] = useState("");

// useEffect - side effects
useEffect(|| {
    fetch_data();
}, []);

// useContext - access context
let theme = useContext(ThemeContext);

// useReducer - complex state
let [state, dispatch] = useReducer(reducer, initial_state);
```

### 3. Routing
```vera
let router = Router::new();
router.add_route(Route {
    path: "/home".to_string(),
    component: "HomePage".to_string(),
});

router.navigate("/about".to_string())?;
```

### 4. Virtual DOM
```vera
// Efficient rendering
let vdom = reconciler.reconcile(old_tree, new_tree)?;
reconciler.apply_patches(vdom)?;
```

### 5. Event Handling
```vera
<button onClick={handle_click}>
    Click Me
</button>

<input onChange={handle_change} />

<form onSubmit={handle_submit}>
    // form elements
</form>
```

## Standard Library Modules

- **components** - Component system
- **hooks** - Hooks (useState, useEffect, etc.)
- **router** - Routing
- **context** - Context API
- **events** - Event handling
- **dom** - Virtual DOM

## Common Patterns

### Form Handling
```vera
component LoginForm() {
    let [email, set_email] = useState("");
    let [password, set_password] = useState("");
    
    fun handle_submit(e) {
        e.prevent_default();
        api::login(email, password)?;
    }
    
    return (
        <form onSubmit={handle_submit}>
            <input onChange={|e| set_email(e.target.value)} />
            <input type="password" onChange={|e| set_password(e.target.value)} />
            <button type="submit">Login</button>
        </form>
    );
}
```

### Data Fetching
```vera
component DataList() {
    let [data, set_data] = useState(vec![]);
    let [loading, set_loading] = useState(true);
    
    useEffect(|| {
        async {
            let d = api::fetch_data().await?;
            set_data(d);
            set_loading(false);
        }
    }, []);
    
    return if loading {
        <div>Loading...</div>
    } else {
        <ul>
            {data.iter().map(|item| <li>{item}</li>).collect()}
        </ul>
    };
}
```

## Best Practices

1. **Component Reusability**: Create small, reusable components
2. **State Management**: Keep state close to where it's used
3. **Performance**: Optimize re-renders
4. **Accessibility**: Make apps accessible
5. **Testing**: Test components thoroughly

## Related Documentation

- [API Reference](../05-reference/VERA_API.md)
- [Building Web Apps](../04-guides/WEB_APPS.md)
- [Widget System](../03-frameworks/WIDGETS.md)

---

**Status**: Production Ready | **Updated**: 2026-06-16
