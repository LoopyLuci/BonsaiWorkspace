# VERA Compiler Architecture v1.0
## Reactive Component Compilation System

---

## 1. COMPILER PIPELINE OVERVIEW

```
VERA Component File (.vera)
    ↓
[Lexer] → Tokens
    ↓
[Parser] → Component AST (with template AST)
    ↓
[Template Analyzer] → Virtual DOM Graph
    ↓
[Type Checker] → Typed Component AST
    ↓
[Reactivity Analyzer] → Dependency Graph
    ↓
[Optimizer] → Optimized AST + Dependency Graph
    ↓
[Code Generator] → JavaScript/WebAssembly/Native Code
    ↓
[Runtime Integration] → Interactive Component
```

---

## 2. COMPONENT PARSING

### 2.1 Token Types (VERA-Specific)

```
KEYWORDS:
  component, view, style, state, prop, computed, on_mounted, on_unmount,
  on_props_changed, watch, slot, inject, provide, context, hook,
  key, class, for, if, else, match

DELIMITERS:
  <, >, {, }, (, ), [, ], />, :, =, @, .

LITERALS:
  strings, numbers, true, false, null

SPECIAL:
  {expression} (template expressions)
  on:event (event binding)
  v-if, v-for, v-show (directive-like features)
```

### 2.2 Component AST Structure

```
Component
├── name: string
├── props: PropDecl[]
├── state: StateDecl[]
├── computed: ComputedDecl[]
├── hooks: HookCall[]
├── watchers: WatcherDecl[]
├── methods: MethodDecl[]
├── lifecycle: LifecycleHook[]
├── style: StyleBlock
├── view: TemplateNode
└── context_usage: ContextUsage

PropDecl
├── name: string
├── type: Type
└── default: Expression?

StateDecl
├── name: string
├── type: Type
├── initial_value: Expression
└── is_reactive: bool (always true in VERA)

ComputedDecl
├── name: string
├── type: Type
├── body: Block | Expression
└── dependencies: string[]

TemplateNode
├── type: "element" | "component" | "text" | "expression" | "conditional" | "list"
├── tag_name: string (for elements)
├── component_name: string (for components)
├── attributes: Attribute[]
├── event_handlers: EventHandler[]
├── children: TemplateNode[]
├── condition: Expression (for conditionals)
├── item_name: string (for lists)
├── items_expression: Expression (for lists)
└── key_expression: Expression (for lists)

Attribute
├── name: string
├── value: Expression | string
└── is_dynamic: bool

EventHandler
├── event_name: string
├── handler: Expression
└── modifiers: string[] (capture, passive, once, etc.)

StyleBlock
├── scoped: bool
└── rules: CSSRule[]
```

### 2.3 Parser Implementation (Pseudocode)

```
class ComponentParser {
    fn parse_component() -> Component {
        expect("component")
        name = expect("identifier")
        
        props = []
        state = []
        computed = []
        methods = []
        lifecycle = []
        style = null
        view = null
        
        expect("{")
        
        while not check("}") {
            if match("prop") {
                props.push(parse_prop_decl())
            } else if match("state") {
                state.push(parse_state_decl())
            } else if match("computed") {
                computed.push(parse_computed_decl())
            } else if match("style") {
                style = parse_style_block()
            } else if match("view") {
                view = parse_template()
            } else if match("on_mounted", "on_unmount", "on_props_changed") {
                lifecycle.push(parse_lifecycle_hook())
            } else if match("watch") {
                // Parse watchers
            } else if check("identifier") && peek_next() == "(" {
                methods.push(parse_method())
            }
        }
        
        expect("}")
        
        return Component(name, props, state, computed, methods, lifecycle, style, view)
    }
    
    fn parse_template() -> TemplateNode {
        expect("{")
        
        nodes = []
        while not check("}") {
            if match("<") {
                nodes.push(parse_element())
            } else if match("{") {
                // Expression
                expr = parse_expression()
                nodes.push(TemplateNode::Expression(expr))
                expect("}")
            } else if check("identifier") || check("string") {
                // Text content
                text = advance().value
                nodes.push(TemplateNode::Text(text))
            }
        }
        
        expect("}")
        
        if nodes.len() == 1 {
            return nodes[0]
        }
        return TemplateNode::Fragment(nodes)
    }
    
    fn parse_element() -> TemplateNode {
        tag_name = expect("identifier")
        
        attributes = []
        event_handlers = []
        
        while not check(">") and not check("/>") {
            if match("on:") {
                event_name = expect("identifier")
                expect("=")
                handler = parse_expression()
                event_handlers.push(EventHandler(event_name, handler))
            } else if check("identifier") {
                attr_name = advance().value
                
                if match("=") {
                    // Dynamic or static value
                    if match("{") {
                        value = parse_expression()
                        expect("}")
                        attributes.push(Attribute(attr_name, value, true))
                    } else {
                        value = expect("string").value
                        attributes.push(Attribute(attr_name, value, false))
                    }
                } else {
                    // Boolean attribute
                    attributes.push(Attribute(attr_name, true, false))
                }
            } else {
                advance()
            }
        }
        
        if match("/>") {
            // Self-closing
            return TemplateNode::Element(tag_name, attributes, event_handlers, [])
        }
        
        expect(">")
        
        children = []
        while not check("</") {
            if match("<") {
                children.push(parse_element())
            } else if match("{") {
                expr = parse_expression()
                expect("}")
                children.push(TemplateNode::Expression(expr))
            } else {
                text = advance().value
                children.push(TemplateNode::Text(text))
            }
        }
        
        expect("</")
        expect(tag_name)
        expect(">")
        
        return TemplateNode::Element(tag_name, attributes, event_handlers, children)
    }
    
    fn parse_conditional() -> TemplateNode {
        condition = parse_expression()
        then_branch = parse_template()
        
        else_branch = null
        if match("else") {
            if match("if") {
                else_branch = parse_conditional()
            } else {
                else_branch = parse_template()
            }
        }
        
        return TemplateNode::Conditional(condition, then_branch, else_branch)
    }
    
    fn parse_list() -> TemplateNode {
        expect("{")
        
        items_expr = parse_expression()
        expect(".map(fn(")
        item_name = expect("identifier")
        expect(")") 
        
        template = parse_template()
        
        expect("})")
        expect("}")
        
        return TemplateNode::List(items_expr, item_name, template)
    }
}
```

---

## 3. TEMPLATE ANALYSIS

### 3.1 Virtual DOM Construction

```
fn analyze_template(node: TemplateNode, component: Component) -> VNodeTree {
    match node {
        TemplateNode::Element(tag, attrs, handlers, children) => {
            vnode = VNode::Element {
                tag: tag,
                attributes: analyze_attributes(attrs, component),
                event_handlers: analyze_handlers(handlers, component),
                children: children.map(|child| analyze_template(child, component))
            }
            return vnode
        },
        
        TemplateNode::Component(name, props, children) => {
            vnode = VNode::Component {
                name: name,
                props: analyze_props(props, component),
                children: children.map(|child| analyze_template(child, component))
            }
            return vnode
        },
        
        TemplateNode::Expression(expr) => {
            vnode = VNode::Dynamic {
                expression: expr,
                dependencies: extract_dependencies(expr, component)
            }
            return vnode
        },
        
        TemplateNode::Conditional(cond, then_br, else_br) => {
            vnode = VNode::Conditional {
                condition: cond,
                dependencies: extract_dependencies(cond, component),
                then_branch: analyze_template(then_br, component),
                else_branch: else_br.map(|e| analyze_template(e, component))
            }
            return vnode
        },
        
        TemplateNode::List(items, item_name, template) => {
            vnode = VNode::List {
                items_expression: items,
                item_binding: item_name,
                template: analyze_template(template, component),
                key_expression: extract_key_expression(template)
            }
            return vnode
        }
    }
}

fn extract_dependencies(expr: Expression, component: Component) -> [string] {
    // Find all state/computed/prop references in expression
    dependencies = []
    
    for identifier in expr.identifiers() {
        if component.has_state(identifier) {
            dependencies.push(identifier)
        }
        if component.has_computed(identifier) {
            dependencies.push(identifier)
        }
        if component.has_prop(identifier) {
            dependencies.push(identifier)
        }
    }
    
    return dependencies
}
```

### 3.2 Reactivity Dependency Graph

```
fn build_dependency_graph(component: Component) -> DependencyGraph {
    graph = new DependencyGraph()
    
    // State variables are reactive roots
    for state_decl in component.state {
        graph.add_node(state_decl.name, "state")
    }
    
    // Computed properties depend on state
    for computed_decl in component.computed {
        for dep in extract_dependencies(computed_decl.body, component) {
            graph.add_edge(computed_decl.name, dep)
        }
    }
    
    // Template nodes depend on state/computed
    for dependency in extract_template_dependencies(component.view, component) {
        graph.add_edge("view", dependency)
    }
    
    // Watch expressions depend on state
    for watcher in component.watchers {
        for dep in extract_dependencies(watcher.expression, component) {
            graph.add_edge(watcher.name, dep)
        }
    }
    
    return graph
}

type DependencyGraph {
    nodes: [string: Node],
    edges: [string: [string]]
}

fn compute_invalidation_graph(dep_graph: DependencyGraph) -> InvalidationGraph {
    // Reverse the dependency graph
    // When state changes, what needs to re-compute?
    
    invalidation = new InvalidationGraph()
    
    for (node, deps) in dep_graph.edges {
        for dep in deps {
            // If dep changes, node needs to re-compute
            invalidation.add_edge(dep, node)
        }
    }
    
    return invalidation
}
```

---

## 4. TYPE CHECKER

### 4.1 Type Checking for Components

```
fn type_check_component(component: Component, env: Environment) -> TypedComponent {
    // Type check props
    for prop in component.props {
        if prop.default {
            default_type = infer_type(prop.default, env)
            if not unify(default_type, prop.type) {
                error("Default value type mismatch for prop '{}'", prop.name)
            }
        }
    }
    
    // Type check state initializers
    for state in component.state {
        init_type = infer_type(state.initial_value, env)
        if not unify(init_type, state.type) {
            error("Initial value type mismatch for state '{}'", state.name)
        }
    }
    
    // Type check computed properties
    for computed in component.computed {
        body_type = infer_type(computed.body, env)
        if not unify(body_type, computed.type) {
            error("Computed property '{}' returns wrong type", computed.name)
        }
    }
    
    // Type check methods
    for method in component.methods {
        type_check_function_body(method, env)
    }
    
    // Type check template
    type_check_template(component.view, component, env)
    
    return TypedComponent(component, env)
}

fn type_check_template(node: TemplateNode, component: Component, env: Environment) -> void {
    match node {
        TemplateNode::Element(tag, attrs, handlers, children) => {
            // Check attributes
            for attr in attrs {
                if attr.is_dynamic {
                    expr_type = infer_type(attr.value, env)
                    if not is_compatible_with_html_attr(attr.name, expr_type) {
                        error("Attribute '{}' expects different type", attr.name)
                    }
                }
            }
            
            // Check event handlers
            for handler in handlers {
                handler_type = infer_type(handler.handler, env)
                if not is_event_handler(handler_type, handler.event_name) {
                    error("Event handler for '{}' has wrong type", handler.event_name)
                }
            }
            
            // Check children
            for child in children {
                type_check_template(child, component, env)
            }
        },
        
        TemplateNode::Component(name, props, children) => {
            // Look up component definition
            comp_def = env.get_component(name)
            if not comp_def {
                error("Component '{}' not found", name)
                return
            }
            
            // Check props match component's prop declarations
            for prop in props {
                comp_prop = comp_def.get_prop(prop.name)
                if not comp_prop {
                    error("Component '{}' doesn't have prop '{}'", name, prop.name)
                    continue
                }
                
                prop_type = infer_type(prop.value, env)
                if not unify(prop_type, comp_prop.type) {
                    error("Prop '{}' type mismatch for component '{}'", prop.name, name)
                }
            }
            
            // Check required props are provided
            for comp_prop in comp_def.props {
                if not comp_prop.has_default && not props.contains(comp_prop.name) {
                    error("Required prop '{}' not provided for component '{}'", comp_prop.name, name)
                }
            }
            
            // Check children
            for child in children {
                type_check_template(child, component, env)
            }
        },
        
        TemplateNode::Expression(expr) => {
            expr_type = infer_type(expr, env)
            // Expression should be renderable (string, number, or component)
            if not is_renderable(expr_type) {
                error("Expression type {} is not renderable", expr_type)
            }
        },
        
        TemplateNode::Conditional(cond, then_br, else_br) => {
            cond_type = infer_type(cond, env)
            if cond_type != bool {
                error("Condition must be boolean, got {}", cond_type)
            }
            
            type_check_template(then_br, component, env)
            if else_br {
                type_check_template(else_br, component, env)
            }
        },
        
        TemplateNode::List(items, item_name, template) => {
            items_type = infer_type(items, env)
            
            if items_type is not Array {
                error("List items must be an array, got {}", items_type)
                return
            }
            
            element_type = items_type.element_type
            env = env.with_binding(item_name, element_type)
            
            type_check_template(template, component, env)
        }
    }
}
```

---

## 5. REACTIVITY TRACKING

### 5.1 Fine-Grained Reactivity

```
fn analyze_reactivity(component: TypedComponent) -> ReactivityMap {
    reactivity_map = new ReactivityMap()
    
    // For each state variable, track what depends on it
    for state in component.state {
        dependents = find_dependents(state.name, component)
        reactivity_map.add(state.name, dependents)
    }
    
    // Generate update functions for each dependency chain
    for (state, dependents) in reactivity_map {
        update_fn = generate_update_function(state, dependents)
        reactivity_map.set_update_fn(state, update_fn)
    }
    
    return reactivity_map
}

fn find_dependents(variable: string, component: Component) -> [ReactivityNode] {
    dependents = []
    
    // Direct computed dependencies
    for computed in component.computed {
        if computed.dependencies.contains(variable) {
            dependents.push(ReactivityNode::Computed(computed.name))
        }
    }
    
    // Template dependencies
    for template_dep in find_template_deps(component.view, variable) {
        dependents.push(ReactivityNode::TemplateNode(template_dep))
    }
    
    // Indirect dependencies (through other computed properties)
    for computed in component.computed {
        if find_dependents(computed.name, component).is_not_empty() {
            dependents.push(ReactivityNode::Computed(computed.name))
        }
    }
    
    return dependents
}

type ReactivityNode = union {
    Computed(string),
    TemplateNode(TemplateNode),
    Watch(string)
}
```

### 5.2 Change Detection Algorithm

```
fn generate_change_handler(state_name: string, dependents: [ReactivityNode]) -> fn() -> void {
    fn on_change(old_value: Any, new_value: Any) -> void {
        // Mark affected computed properties as dirty
        for dependent in dependents {
            match dependent {
                ReactivityNode::Computed(name) => {
                    computed_cache[name].invalidate()
                },
                ReactivityNode::TemplateNode(node) => {
                    recompute_template_node(node)
                    schedule_render_update(node)
                },
                ReactivityNode::Watch(watcher_name) => {
                    trigger_watcher(watcher_name)
                }
            }
        }
        
        schedule_render_pass()
    }
    
    return on_change
}

fn schedule_render_pass() -> void {
    // Use requestAnimationFrame for efficient batched updates
    if not render_scheduled {
        render_scheduled = true
        requestAnimationFrame(fn() {
            render_scheduled = false
            
            // Recompute all dirty computed values
            recompute_dirty_computed()
            
            // Generate new virtual DOM
            new_vdom = render_view()
            
            // Diff and patch
            patch_dom(old_vdom, new_vdom)
            
            old_vdom = new_vdom
        })
    }
}
```

---

## 6. CODE GENERATION

### 6.1 JavaScript/WebAssembly Generation

```
fn generate_javascript(typed_component: TypedComponent) -> string {
    code = ""
    
    // Generate component class
    code += "class {}" .format(typed_component.name)
    code += " {\n"
    code += "  constructor(props) {\n"
    
    // Initialize props
    for prop in typed_component.props {
        code += "    this.{} = props.{};\n".format(prop.name, prop.name)
    }
    
    // Initialize state with reactivity
    for state in typed_component.state {
        code += "    this._{} = {};\n".format(state.name, generate_expr(state.initial_value))
        code += "    Object.defineProperty(this, '{}', {{\n".format(state.name)
        code += "      get() { return this._{}; },\n".format(state.name)
        code += "      set(value) {\n"
        code += "        if (this._{} !== value) {\n".format(state.name)
        code += "          const oldValue = this._{};\n".format(state.name)
        code += "          this._{} = value;\n".format(state.name)
        code += "          this._on_{}Change(oldValue, value);\n".format(state.name)
        code += "        }\n"
        code += "      }\n"
        code += "    });\n"
    }
    
    code += "  }\n"
    
    // Generate computed properties
    for computed in typed_component.computed {
        code += "  get {}() {{\n".format(computed.name)
        code += "    if (this._cache['{}']} === undefined) {{\n".format(computed.name)
        code += "      this._cache['{}'] = {};\n".format(computed.name, generate_expr(computed.body))
        code += "    }\n"
        code += "    return this._cache['{}'];\n".format(computed.name)
        code += "  }\n"
    }
    
    // Generate render method
    code += "  render() {\n"
    code += "    return " + generate_vnode(typed_component.view) + ";\n"
    code += "  }\n"
    
    // Generate methods
    for method in typed_component.methods {
        code += "  {}({}) {{\n".format(method.name, method.params.join(", "))
        code += "    " + generate_block(method.body) + "\n"
        code += "  }\n"
    }
    
    code += "}\n"
    
    return code
}

fn generate_vnode(node: TemplateNode) -> string {
    match node {
        TemplateNode::Element(tag, attrs, handlers, children) => {
            code = "h('{}', {{\n".format(tag)
            
            // Generate attributes
            for attr in attrs {
                if attr.is_dynamic {
                    code += "  {}: {},\n".format(attr.name, generate_expr(attr.value))
                } else {
                    code += "  {}: '{}',\n".format(attr.name, attr.value)
                }
            }
            
            // Generate event handlers
            for handler in handlers {
                code += "  on{}: {},\n".format(capitalize(handler.event_name), generate_expr(handler.handler))
            }
            
            code += "}, [\n"
            
            // Generate children
            for child in children {
                code += "  " + generate_vnode(child) + ",\n"
            }
            
            code += "])\n"
            return code
        },
        
        TemplateNode::Expression(expr) => {
            return generate_expr(expr)
        },
        
        TemplateNode::Conditional(cond, then_br, else_br) => {
            return "({} ? {} : {})".format(
                generate_expr(cond),
                generate_vnode(then_br),
                generate_vnode(else_br)
            )
        },
        
        TemplateNode::List(items, item_name, template) => {
            return "{}.map(({}) => {})".format(
                generate_expr(items),
                item_name,
                generate_vnode(template)
            )
        }
    }
}
```

### 6.2 Native Code Generation

```
fn generate_native_code(typed_component: TypedComponent, target: string) -> string {
    // For iOS/Android/Desktop targets
    // Generate native view controller/activity code
    
    match target {
        "ios" => generate_swift_code(typed_component),
        "android" => generate_kotlin_code(typed_component),
        "windows" => generate_xaml_code(typed_component),
        "macos" => generate_swiftui_code(typed_component)
    }
}
```

---

## 7. OPTIMIZATION

### 7.1 Template Optimization

```
fn optimize_template(node: TemplateNode) -> TemplateNode {
    // Compile-time static detection
    
    // If conditional is always true/false, remove dead branch
    if node is Conditional {
        if is_constant_true(node.condition) {
            return optimize_template(node.then_branch)
        }
        if is_constant_false(node.condition) {
            return node.else_branch ? optimize_template(node.else_branch) : null
        }
    }
    
    // Hoist static content out of dynamic contexts
    if node is List {
        static_children = extract_static_children(node.template)
        dynamic_children = extract_dynamic_children(node.template)
        // Generate optimized list render function
    }
    
    return node
}
```

### 7.2 Reactivity Optimization

```
fn optimize_reactivity(reactivity_map: ReactivityMap) -> OptimizedReactivityMap {
    // Batch related updates
    for (state, dependents) in reactivity_map {
        batch = group_dependencies_by_type(dependents)
        reactivity_map[state].batch_updates = batch
    }
    
    // Skip unnecessary computed recomputation
    for computed in component.computed {
        if has_same_dependencies_as_state(computed) {
            // Can skip intermediate computed, directly recompute template
        }
    }
    
    return reactivity_map
}
```

---

## 8. RUNTIME INTEGRATION

### 8.1 Virtual DOM Diffing & Patching

```
fn patch_dom(old_vdom: VNode, new_vdom: VNode, dom_node: HTMLElement) -> void {
    // Skip if same
    if old_vdom === new_vdom {
        return
    }
    
    // Different node type
    if old_vdom.type != new_vdom.type {
        new_dom = create_dom_node(new_vdom)
        dom_node.parent.replace_child(new_dom, dom_node)
        return
    }
    
    // Same element type
    if old_vdom is Element and new_vdom is Element {
        // Update attributes
        for (name, value) in new_vdom.attributes {
            if old_vdom.attributes[name] != value {
                dom_node.setAttribute(name, value)
            }
        }
        
        // Remove removed attributes
        for name in old_vdom.attributes {
            if new_vdom.attributes[name] == null {
                dom_node.removeAttribute(name)
            }
        }
        
        // Update children
        patch_children(old_vdom.children, new_vdom.children, dom_node)
    }
}

fn patch_children(old_children: [VNode], new_children: [VNode], parent: HTMLElement) -> void {
    // Use key-based matching for lists
    old_by_key = {}
    for (i, child) in old_children {
        key = child.key or i
        old_by_key[key] = (i, child)
    }
    
    for (i, new_child) in new_children {
        key = new_child.key or i
        
        if key in old_by_key {
            (old_i, old_child) = old_by_key[key]
            patch_dom(old_child, new_child, parent.children[old_i])
            delete old_by_key[key]
        } else {
            // New element
            new_dom = create_dom_node(new_child)
            parent.insert_child(new_dom, i)
        }
    }
    
    // Remove removed elements
    for (key, (i, _)) in old_by_key {
        parent.remove_child(parent.children[i])
    }
}
```

---

## 9. EXAMPLE: COMPLETE COMPILATION

```
VERA Component:
────────────────
component Counter {
    state count: i32 = 0
    
    fn increment() {
        count = count + 1
    }
    
    view {
        <div class="counter">
            <p>Count: {count}</p>
            <button on:click=increment>+</button>
        </div>
    }
}

AST (After Parsing):
─────────────────────
Component {
    name: "Counter",
    state: [StateDecl { name: "count", type: i32, value: 0 }],
    methods: [MethodDecl { name: "increment", ... }],
    view: TemplateNode::Element {
        tag: "div",
        attributes: [Attribute { name: "class", value: "counter" }],
        children: [
            TemplateNode::Element {
                tag: "p",
                children: [
                    TemplateNode::Text("Count: "),
                    TemplateNode::Expression(Identifier("count"))
                ]
            },
            TemplateNode::Element {
                tag: "button",
                event_handlers: [EventHandler { event: "click", handler: "increment" }],
                children: [TemplateNode::Text("+")]
            }
        ]
    }
}

Reactivity Graph:
──────────────────
count (state) → [view, increment (method)]

JavaScript Output:
────────────────────
class Counter {
    constructor(props) {
        this._count = 0;
        Object.defineProperty(this, 'count', {
            get() { return this._count; },
            set(value) {
                if (this._count !== value) {
                    this._count = value;
                    this.scheduleRender();
                }
            }
        });
    }
    
    increment() {
        this.count = this.count + 1;
    }
    
    render() {
        return h('div', { class: 'counter' }, [
            h('p', {}, [
                'Count: ',
                String(this.count)
            ]),
            h('button', { onClick: () => this.increment() }, ['+'])
        ]);
    }
}
```

---

This compiler transforms VERA components into efficient, reactive runtime code that works across web and native platforms.
