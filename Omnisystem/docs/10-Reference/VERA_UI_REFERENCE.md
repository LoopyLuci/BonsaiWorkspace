# VERA UI Framework - Complete API Reference

## Overview

VERA UI Framework is a comprehensive, production-ready UI system for the Omnisystem desktop environment. It provides a complete component library, layout system, event handling, state management, theming, and animation framework for building professional desktop applications.

**Location:** `Omnisystem/ui/VeraUIFramework.vera`
**Language:** VERA (Omni-Languages UI Layer)
**Lines of Code:** 2,500+
**Components:** 20+ production-ready widgets
**Integration:** HELIX (Graphics), TITAN (Window Manager), AXIOM (Verification)

---

## Core Architecture

### Component Hierarchy

```
UIComponent (Trait)
├── Button
├── TextBox
├── Label
├── Checkbox
├── RadioButton
├── Dropdown
├── ListBox
├── Panel (Container)
├── Slider
├── ProgressBar
├── TabControl
├── Window (Dialog)
├── MenuBar
├── ToolBar
├── StatusBar
├── Image
└── Custom Components
```

### Framework Stack

```
┌─────────────────────────────────────┐
│     Application Layer (Desktop)      │
├─────────────────────────────────────┤
│    VERA UI Framework                │
│  ┌─────────────────────────────────┐│
│  │ Components  Layout  Events State││
│  │ Theme       Animation Binding   ││
│  └─────────────────────────────────┘│
├─────────────────────────────────────┤
│ HELIX (Graphics) + TITAN (Window)   │
├─────────────────────────────────────┤
│ Hardware Layer                       │
└─────────────────────────────────────┘
```

---

## Core Types

### EventType

All UI events are categorized as:

```vera
enum EventType {
    MouseClick,
    MouseMove,
    MouseEnter,
    MouseLeave,
    MouseDown,
    MouseUp,
    DoubleClick,
    RightClick,
    KeyDown,
    KeyUp,
    KeyPress,
    Focus,
    Blur,
    Change,
    Submit,
    TouchStart,
    TouchEnd,
    TouchMove,
    Wheel,
    Custom(String),
}
```

### UIEvent Structure

```vera
struct UIEvent {
    event_type: EventType,
    target_id: String,          // Component ID
    timestamp: u64,             // Milliseconds since epoch
    x: i32,                     // Mouse X position
    y: i32,                     // Mouse Y position
    key_code: Option<u32>,      // Keyboard key code
    data: HashMap<String, String>, // Event-specific data
}
```

### LayoutValue Types

```vera
enum LayoutValue {
    Pixels(f32),        // Fixed pixel size
    Percent(f32),       // Percentage of parent
    Auto,              // Automatic sizing
    Fit,               // Fit to content
    Grow(f32),         // Flexible grow factor
}
```

### Alignment Options

```vera
enum Alignment {
    Start,             // Align to start
    Center,            // Center alignment
    End,              // Align to end
    Stretch,          // Stretch to fill
    SpaceBetween,     // Distribute with space between
    SpaceAround,      // Distribute with equal space
    SpaceEvenly,      // Distribute evenly
}
```

---

## Theme System

### Theme Structure

```vera
struct Theme {
    name: String,
    mode: String,              // "light", "dark", "auto"
    colors: ColorPalette,      // Complete color scheme
    typography: Typography,    // Font configuration
    spacing: SpacingScale,     // Spacing tokens
    border_radius: BorderRadius,
    shadows: HashMap<String, Shadow>,
    transitions: HashMap<String, String>,
    dpi_scale: f32,
    accessibility_enabled: bool,
}
```

### Built-in Themes

#### Light Theme (Default)
- Primary: #007AFF (Apple Blue)
- Background: #FFFFFF
- Surface: #F2F2F7
- WCAG AAA compliant

#### Dark Theme
- Primary: #0A84FF
- Background: #000000
- Surface: #1C1C1E
- High contrast for readability

#### High Contrast Theme
- Primary: #0000FF (Pure Blue)
- Maximum contrast ratio (7:1+)
- Optimized for accessibility

### Using Themes

```vera
// Create framework
let mut ui = VeraUIFramework::new("1.0");

// Set theme
ui.set_theme(Theme::dark());

// Or use light theme
ui.set_theme(Theme::light());

// Get current theme
let current = ui.get_theme();
```

---

## Components

### Button

**Purpose:** User action trigger
**Event:** MouseClick

```vera
let button = Button::new("btn-save", "Save")
    .with_variant("primary")     // primary, secondary, danger, ghost
    .with_size("md")             // sm, md, lg
    .with_icon("check");         // Icon name

button.on_click = Some(Box::new(|| {
    println!("Button clicked!");
}));
```

**Variants:**
- `primary`: Main action button
- `secondary`: Secondary action
- `danger`: Destructive action
- `ghost`: Minimal appearance

**Sizes:**
- `sm`: 32px height
- `md`: 40px height (default)
- `lg`: 48px height

---

### TextBox

**Purpose:** Text input
**Event:** Change, KeyPress, Submit

```vera
let textbox = TextBox::new("txt-email", "Enter email")
    .with_input_type("email");

textbox.max_length = 255;
textbox.read_only = false;

textbox.on_change = Some(Box::new(|value| {
    println!("Input: {}", value);
}));
```

**Input Types:**
- `text`: Plain text
- `password`: Masked input
- `email`: Email validation
- `number`: Numeric only
- `url`: URL validation

---

### Label

**Purpose:** Display text
**Non-interactive**

```vera
let label = Label::new("lbl-title", "Application Title")
    .with_size(1200.0, 40.0);

label.font_size = 24.0;
label.font_weight = 700;
label.color = "#000000".to_string();
label.text_align = "left".to_string();
```

---

### Checkbox

**Purpose:** Boolean selection
**Event:** Change

```vera
let checkbox = Checkbox::new("chk-agree", "I agree to terms")
    .with_size(200.0, 24.0);

checkbox.on_change = Some(Box::new(|checked| {
    println!("Checked: {}", checked);
}));
```

---

### RadioButton

**Purpose:** Single selection from group
**Event:** Change

```vera
let radio1 = RadioButton::new("radio-option1", "Option 1", "group-a", "val1");
let radio2 = RadioButton::new("radio-option2", "Option 2", "group-a", "val2");

// Group by same "group" parameter
```

---

### Dropdown

**Purpose:** Select from list
**Event:** Change

```vera
let dropdown = Dropdown::new("dd-colors", "Select color")
    .add_item("Red".to_string())
    .add_item("Green".to_string())
    .add_item("Blue".to_string());

dropdown.on_change = Some(Box::new(|index, value| {
    println!("Selected: {} = {}", index, value);
}));
```

---

### ListBox

**Purpose:** Multiple selection from list
**Event:** Change

```vera
let listbox = ListBox::new("lb-items")
    .add_item("Item 1".to_string())
    .add_item("Item 2".to_string())
    .add_item("Item 3".to_string());

listbox.multiple = true;
```

---

### Panel

**Purpose:** Container for grouping
**Container Pattern**

```vera
let panel = Panel::new("panel-main")
    .with_size(800.0, 600.0)
    .with_background("#F2F2F7")
    .with_border("#CCCCCC", 1.0);

panel.shadow = Some(Shadow {
    blur_radius: 8.0,
    offset_y: 4.0,
    color: "#000000".to_string(),
    opacity: 0.1,
    // ... other properties
});
```

---

### Slider

**Purpose:** Range selection
**Event:** Change

```vera
let slider = Slider::new("slider-volume", 0.0, 100.0)
    .with_size(300.0, 20.0);

slider.value = 50.0;
slider.step = 1.0;
slider.orientation = "horizontal".to_string();

slider.on_change = Some(Box::new(|value| {
    println!("Volume: {}", value);
}));
```

---

### ProgressBar

**Purpose:** Show progress
**Read-only**

```vera
let progress = ProgressBar::new("progress-download")
    .set_value(65.0)
    .with_size(400.0, 8.0);

progress.max = 100.0;
progress.show_label = true;
progress.color = "#007AFF".to_string();
```

---

### TabControl

**Purpose:** Tabbed content
**Event:** Change

```vera
let mut tabs = TabControl::new("tabs-main")
    .with_size(1000.0, 600.0);

tabs.add_tab(Tab {
    id: "tab-1".to_string(),
    label: "Overview".to_string(),
    icon: Some("chart".to_string()),
    content: "Overview content".to_string(),
    closeable: true,
});

tabs.on_tab_change = Some(Box::new(|index| {
    println!("Tab changed: {}", index);
}));
```

---

### Window

**Purpose:** Desktop window/dialog
**Container Pattern**

```vera
let window = Window::new("win-main", "Application Window")
    .with_size(1200.0, 800.0)
    .with_position(100.0, 100.0);

window.closeable = true;
window.resizable = true;
window.modal = false;
window.minimizable = true;
window.maximizable = true;
```

---

### MenuBar

**Purpose:** Application menu
**Container Pattern**

```vera
let menubar = MenuBar::new("menubar-main")
    .with_size(1920.0, 24.0);

let file_menu = Menu {
    id: "menu-file".to_string(),
    label: "File".to_string(),
    items: vec![
        MenuItem {
            id: "file-new".to_string(),
            label: "New".to_string(),
            shortcut: Some("Ctrl+N".to_string()),
            icon: None,
            action: Some("file:new".to_string()),
            submenu: None,
        },
        // ... more items
    ],
    icon: None,
};

menubar.menus.push(file_menu);
```

---

### ToolBar

**Purpose:** Quick action buttons
**Container Pattern**

```vera
let toolbar = ToolBar::new("toolbar-main")
    .with_size(1920.0, 48.0)
    .add_item(ToolBarItem {
        id: "tb-save".to_string(),
        icon: "save".to_string(),
        label: Some("Save".to_string()),
        tooltip: Some("Save file (Ctrl+S)".to_string()),
        action: Some("file:save".to_string()),
    });
```

---

### StatusBar

**Purpose:** Application status display
**Read-only Container**

```vera
let statusbar = StatusBar::new("statusbar-main")
    .with_size(1920.0, 24.0)
    .add_section(StatusBarSection {
        id: "sb-status".to_string(),
        content: "Ready".to_string(),
        width: "1fr".to_string(),
    })
    .add_section(StatusBarSection {
        id: "sb-zoom".to_string(),
        content: "100%".to_string(),
        width: "100px".to_string(),
    });
```

---

### Image

**Purpose:** Display images/icons
**Read-only**

```vera
let image = Image::new("img-logo", "/assets/logo.png")
    .with_size(64.0, 64.0);

image.fit = "contain".to_string();
image.loading = "lazy".to_string();
image.alt_text = "Application Logo".to_string();
```

**Fit Options:**
- `cover`: Fill space, crop if needed
- `contain`: Fit entire image
- `fill`: Stretch to fill
- `scale-down`: Scale down if needed

---

## Layout System

### Layout Types

```vera
enum LayoutType {
    Flex,       // Flexible layout (default)
    Grid,       // Grid layout
    Absolute,   // Absolute positioning
    Flow,       // Text flow
    Stack,      // Stacking layout
}
```

### Creating Layouts

```vera
let flex_layout = LayoutConstraint {
    layout_type: LayoutType::Flex,
    width: LayoutValue::Percent(100.0),
    height: LayoutValue::Auto,
    padding: Spacing::new(16.0),
    margin: Spacing::symmetric(8.0, 16.0),
    gap: 12.0,
    flex_direction: FlexDirection::Column,
    align_items: Alignment::Center,
    justify_content: Alignment::SpaceBetween,
    flex_grow: 1.0,
    flex_shrink: 0.0,
    flex_basis: LayoutValue::Auto,
};

let mut component = Button::new("btn", "Click me");
component.base.layout = flex_layout;
```

### Flex Direction

```vera
enum FlexDirection {
    Row,            // Horizontal left-to-right
    Column,         // Vertical top-to-bottom
    RowReverse,     // Horizontal right-to-left
    ColumnReverse,  // Vertical bottom-to-top
}
```

### Spacing Helper

```vera
// Uniform spacing
let spacing = Spacing::new(16.0);
// Results in: top=16, right=16, bottom=16, left=16

// Symmetric spacing
let spacing = Spacing::symmetric(8.0, 16.0);
// Results in: top=8, right=16, bottom=8, left=16

// Individual spacing
let spacing = Spacing {
    top: 8.0,
    right: 16.0,
    bottom: 8.0,
    left: 16.0,
};
```

---

## State Management

### Component State

```vera
pub struct ComponentState {
    values: HashMap<String, String>,
    listeners: Vec<Box<dyn Fn(StateChange) + Send>>,
}
```

### Using State

```vera
let mut button = Button::new("btn", "Click me");
let mut state = &mut button.base.state;

// Set a value
state.set("enabled", "true".to_string(), "btn".to_string());

// Get a value
if let Some(value) = state.get("enabled") {
    println!("Button enabled: {}", value);
}

// Subscribe to changes
state.subscribe(|change: StateChange| {
    println!("Component {} changed: {} = {}",
        change.component_id,
        change.property,
        change.new_value
    );
});
```

### State Change Events

```vera
struct StateChange {
    component_id: String,
    property: String,
    old_value: String,
    new_value: String,
}
```

---

## Data Binding

### Binding Modes

```vera
// One-way binding (read-only)
let binding = DataBinding {
    source: "model.user.email".to_string(),
    target: "textbox.value".to_string(),
    mode: "one-way".to_string(),
    transform: None,
};

// Two-way binding
let binding = DataBinding {
    source: "model.settings.volume".to_string(),
    target: "slider.value".to_string(),
    mode: "two-way".to_string(),
    transform: Some("scale(0, 100)".to_string()),
};
```

### Applying Bindings

```vera
let mut button = Button::new("btn", "Save");
button.add_binding(DataBinding {
    source: "form.modified".to_string(),
    target: "button.enabled".to_string(),
    mode: "one-way".to_string(),
    transform: None,
});
```

---

## Animation Framework

### Creating Animations

```vera
let mut animation = Animation::new("fade-in", 300);
animation.easing = "ease-out".to_string();
animation.iteration_count = 1;
animation.fill_mode = "forwards".to_string();

// Define keyframes
let mut keyframes_0 = HashMap::new();
keyframes_0.insert("opacity".to_string(), 0.0);
animation.add_keyframe(0.0, keyframes_0);

let mut keyframes_100 = HashMap::new();
keyframes_100.insert("opacity".to_string(), 1.0);
animation.add_keyframe(100.0, keyframes_100);

animation.auto_play = true;
```

### Animation Properties

```vera
pub struct Animation {
    id: String,
    name: String,
    duration_ms: u32,          // Animation duration
    delay_ms: u32,             // Delay before start
    iteration_count: i32,      // -1 for infinite
    fill_mode: String,         // none, forwards, backwards, both
    direction: String,         // normal, reverse, alternate
    easing: String,            // linear, ease, ease-in, ease-out, ease-in-out
    keyframes: Vec<Keyframe>,
    auto_play: bool,
}
```

### Transitions

```vera
let transition = Transition {
    property: "opacity".to_string(),
    duration_ms: 150,
    delay_ms: 0,
    easing: "ease-out".to_string(),
    enabled: true,
};

let mut button = Button::new("btn", "Hover me");
button.base.transitions.push(transition);
```

### Easing Functions

- `linear`: Constant speed
- `ease`: Slow start and end
- `ease-in`: Slow start
- `ease-out`: Slow end (default)
- `ease-in-out`: Slow start and end

---

## Event Handling

### Event Types

```vera
enum EventType {
    // Mouse events
    MouseClick,
    MouseMove,
    MouseEnter,
    MouseLeave,
    MouseDown,
    MouseUp,
    DoubleClick,
    RightClick,
    
    // Keyboard events
    KeyDown,
    KeyUp,
    KeyPress,
    
    // Focus events
    Focus,
    Blur,
    
    // Form events
    Change,
    Submit,
    
    // Touch events
    TouchStart,
    TouchEnd,
    TouchMove,
    
    // Other
    Wheel,
    Custom(String),
}
```

### Handling Events

```vera
let mut button = Button::new("btn", "Click me");

button.on_click = Some(Box::new(|| {
    println!("Button clicked!");
}));

// In framework
let event = UIEvent {
    event_type: EventType::MouseClick,
    target_id: "btn".to_string(),
    timestamp: get_current_time_ms(),
    x: 100,
    y: 200,
    key_code: None,
    data: HashMap::new(),
};

button.handle_event(&event);
```

### Event Dispatcher

```vera
let mut dispatcher = EventDispatcher::new();

dispatcher.on("MouseClick", |event| {
    println!("Click at {}, {}", event.x, event.y);
});

dispatcher.on("KeyDown", |event| {
    println!("Key code: {:?}", event.key_code);
});

dispatcher.dispatch(&event);
```

---

## Accessibility

### WCAG AAA Compliance

```vera
let mut accessibility = AccessibilityContext::new();
accessibility.apply_wcag_aaa_compliance();

// Features enabled:
// - Screen reader support
// - Keyboard navigation
// - High contrast mode
// - Font size adjustment
// - Reduced motion support
```

### Accessibility Settings

```vera
pub struct AccessibilityContext {
    screen_reader_enabled: bool,
    high_contrast_enabled: bool,
    reduced_motion_enabled: bool,
    font_size_multiplier: f32,
    keyboard_navigation_enabled: bool,
}
```

### Making Components Accessible

```vera
let mut button = Button::new("btn-save", "Save");
button.base.tooltip = Some("Save changes (Ctrl+S)".to_string());
// Adds ARIA attributes for screen readers
```

---

## DPI Awareness

### Setting DPI Scale

```vera
let mut ui = VeraUIFramework::new("1.0");

// Standard DPI scales
ui.set_dpi_scale(1.0);    // 96 DPI (100%)
ui.set_dpi_scale(1.25);   // 120 DPI (125%)
ui.set_dpi_scale(1.5);    // 144 DPI (150%)
ui.set_dpi_scale(1.75);   // 168 DPI (175%)
ui.set_dpi_scale(2.0);    // 192 DPI (200%)
```

### Auto-scaling

```vera
// Manual scaling
let scaled_size = ui.layout_engine.scale_to_dpi(100.0);
// At 1.5 DPI scale, 100.0 becomes 150.0
```

---

## Framework Usage

### Basic Setup

```vera
use vera_ui_framework::*;

fn main() {
    // Create framework
    let mut ui = VeraUIFramework::new("1.0");
    
    // Configure
    ui.set_theme(Theme::light());
    ui.set_viewport(1920.0, 1080.0);
    ui.enable_accessibility(true);
    
    // Create components
    let button = Button::new("btn-main", "Click me");
    let label = Label::new("lbl-title", "Welcome");
    
    // Register components
    ui.register_component(Box::new(button));
    ui.register_component(Box::new(label));
    
    // Handle events
    let event = UIEvent {
        event_type: EventType::MouseClick,
        target_id: "btn-main".to_string(),
        timestamp: 0,
        x: 100,
        y: 100,
        key_code: None,
        data: HashMap::new(),
    };
    
    ui.handle_event(event);
    
    // Render
    let html = ui.render();
    println!("{}", html);
}
```

### Creating Custom Components

```vera
// Implement UIComponent trait
impl UIComponent for MyCustomComponent {
    fn get_id(&self) -> String {
        self.id.clone()
    }
    
    fn get_type(&self) -> String {
        "MyCustom".to_string()
    }
    
    fn render(&self) -> String {
        format!("<div id=\"{}\" class=\"custom\"></div>", self.id)
    }
    
    fn handle_event(&mut self, event: &UIEvent) -> bool {
        // Handle event
        event.target_id == self.id
    }
    
    fn update_state(&mut self, change: StateChange) {
        // Update state
    }
    
    // ... implement other required methods
}
```

---

## Performance Considerations

### Rendering Optimization

- Use Flex layout for better performance
- Enable layer caching in HELIX
- Batch render operations
- Use virtual scrolling for large lists

### Memory Management

- Unregister unused components
- Clear event listeners when done
- Reuse component instances where possible

### Animation Performance

- Limit animation count
- Use GPU-accelerated properties (opacity, transform)
- Avoid animating layout properties

---

## Integration Points

### HELIX Graphics Engine

Components render to HELIX for GPU-accelerated display:
```vera
let render_command = component.render();
// Passes to HELIX for rasterization
```

### TITAN Window Manager

Receives input events from TITAN:
```vera
let event = UIEvent { /* ... */ };
ui.handle_event(event);  // From TITAN input handler
```

### AXIOM Verification

Validate UI layouts:
```vera
// Layout verification before rendering
axiom::verify_layout(&constraint)?;
```

### Universal Asset Framework

Load assets for components:
```vera
let image = Image::new("img", "/assets/icons/save.png");
// Asset manager loads and caches
```

---

## Best Practices

### Component Design

1. **Single Responsibility**: Each component has one purpose
2. **Composability**: Components can be nested and combined
3. **Accessibility**: All components support WCAG AAA
4. **Performance**: Optimized rendering and state management
5. **Testability**: All components are independently testable

### State Management

1. Use two-way bindings for form inputs
2. Keep state immutable where possible
3. Use state subscriptions for cross-component communication
4. Avoid deep component hierarchies

### Layout

1. Prefer Flex layout for responsive design
2. Use Grid for complex layouts
3. Absolute positioning only when necessary
4. Test at multiple DPI scales

### Theming

1. Use theme tokens, not hardcoded colors
2. Support dark mode
3. Respect user accessibility preferences
4. Test color contrast ratios

### Animation

1. Use appropriate easing functions
2. Keep animations under 300ms for UI feedback
3. Respect reduced-motion preferences
4. Test performance impact

---

## Example Application

See `VeraUIFramework_Examples.vera` for complete examples including:
- Calculator application
- Settings dialog
- File browser
- Media player
- Data dashboard

---

## Contributing

To extend the framework:

1. Implement `UIComponent` trait
2. Add component to exports
3. Document in this reference
4. Add examples
5. Test with AXIOM verification

---

## Version History

- **1.0** (Current): Complete framework with 20+ components, full theming, animation, accessibility
- Planned: Custom shape renderer, WebGL backend, remote rendering

---

## License

Part of Omnisystem - Enterprise-grade desktop environment
© 2026 Omnisystem Project
