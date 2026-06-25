# VERA UI Framework - Quick Start Guide

## Installation & Setup

### Step 1: Import the Framework

```vera
use vera_ui_framework::*;
use std::collections::HashMap;
```

### Step 2: Create Framework Instance

```vera
fn main() {
    // Initialize the framework
    let mut ui = VeraUIFramework::new("1.0");
    
    // Configure settings
    ui.set_theme(Theme::light());
    ui.set_viewport(1920.0, 1080.0);
    ui.enable_accessibility(true);
    ui.set_dpi_scale(1.0);
}
```

---

## Basic Components

### Creating a Button

```vera
// Simple button
let button = Button::new("btn-main", "Click Me");

// Button with configuration
let button = Button::new("btn-save", "Save")
    .with_variant("primary")    // primary, secondary, danger, ghost
    .with_size("lg")            // sm, md, lg
    .with_icon("save")
    .with_position(100.0, 50.0)
    .with_size(200.0, 40.0);

// With event handler
button.on_click = Some(Box::new(|| {
    println!("Button clicked!");
}));

// Register with framework
ui.register_component(Box::new(button));
```

### Creating a TextBox

```vera
// Simple text input
let textbox = TextBox::new("txt-email", "Enter email")
    .with_input_type("email")
    .with_position(100.0, 100.0)
    .with_size(300.0, 40.0);

// With event handler
textbox.on_change = Some(Box::new(|value| {
    println!("Input changed: {}", value);
}));
```

### Creating a Label

```vera
let label = Label::new("lbl-title", "Welcome to VERA UI")
    .with_position(10.0, 10.0)
    .with_size(400.0, 40.0);

// Customize
label.font_size = 24.0;
label.font_weight = 700;
label.color = "#000000".to_string();
```

### Creating a Checkbox

```vera
let checkbox = Checkbox::new("chk-agree", "I agree to terms")
    .with_position(10.0, 200.0)
    .with_size(300.0, 24.0);

checkbox.on_change = Some(Box::new(|checked| {
    println!("Checked: {}", checked);
}));
```

### Creating a Dropdown

```vera
let dropdown = Dropdown::new("dd-colors", "Select color")
    .add_item("Red".to_string())
    .add_item("Green".to_string())
    .add_item("Blue".to_string())
    .with_position(10.0, 300.0)
    .with_size(200.0, 32.0);

dropdown.on_change = Some(Box::new(|index, value| {
    println!("Selected: {} ({})", value, index);
}));
```

### Creating a Slider

```vera
let slider = Slider::new("slider-volume", 0.0, 100.0)
    .with_position(10.0, 400.0)
    .with_size(300.0, 20.0);

slider.value = 50.0;
slider.on_change = Some(Box::new(|value| {
    println!("Volume: {}%", value);
}));
```

### Creating a ProgressBar

```vera
let progress = ProgressBar::new("progress-load")
    .set_value(65.0)
    .with_position(10.0, 500.0)
    .with_size(300.0, 8.0);

progress.color = "#007AFF".to_string();
```

---

## Containers & Layout

### Creating a Panel

```vera
let panel = Panel::new("panel-main")
    .with_position(0.0, 0.0)
    .with_size(800.0, 600.0)
    .with_background("#F2F2F7")
    .with_border("#CCCCCC", 1.0);

// Configure layout
let mut layout = LayoutConstraint {
    layout_type: LayoutType::Flex,
    width: LayoutValue::Percent(100.0),
    height: LayoutValue::Auto,
    padding: Spacing::new(16.0),
    margin: Spacing::new(8.0),
    gap: 12.0,
    flex_direction: FlexDirection::Column,
    align_items: Alignment::Center,
    justify_content: Alignment::Start,
    flex_grow: 1.0,
    flex_shrink: 0.0,
    flex_basis: LayoutValue::Auto,
};

panel.base.layout = layout;
```

### Creating a Window

```vera
let window = Window::new("win-main", "My Application")
    .with_position(100.0, 100.0)
    .with_size(1200.0, 800.0);

window.closeable = true;
window.resizable = true;
window.modal = false;
window.minimizable = true;
window.maximizable = true;

ui.register_component(Box::new(window));
```

### Creating a TabControl

```vera
let mut tabs = TabControl::new("tabs-main")
    .with_position(0.0, 50.0)
    .with_size(800.0, 600.0);

// Add tabs
tabs = tabs.add_tab(Tab {
    id: "tab-overview".to_string(),
    label: "Overview".to_string(),
    icon: Some("chart".to_string()),
    content: "<div>Overview content</div>".to_string(),
    closeable: false,
});

tabs = tabs.add_tab(Tab {
    id: "tab-settings".to_string(),
    label: "Settings".to_string(),
    icon: Some("cog".to_string()),
    content: "<div>Settings content</div>".to_string(),
    closeable: true,
});

tabs.on_tab_change = Some(Box::new(|index| {
    println!("Tab changed to: {}", index);
}));
```

---

## Theming

### Using Built-in Themes

```vera
// Light theme (default)
ui.set_theme(Theme::light());

// Dark theme
ui.set_theme(Theme::dark());

// High contrast theme
ui.set_theme(Theme::high_contrast());
```

### Creating Custom Theme

```vera
let mut custom = Theme::light();
custom.name = "Ocean Blue".to_string();

// Customize colors
custom.colors.primary = "#0077B6".to_string();
custom.colors.secondary = "#00B4D8".to_string();
custom.colors.accent = "#90E0EF".to_string();
custom.colors.success = "#00B4D8".to_string();
custom.colors.background = "#E0F7FA".to_string();

// Customize typography
custom.typography.font_family = "Segoe UI, sans-serif".to_string();
custom.typography.size_base = 14.0;

// Apply theme
ui.set_theme(custom);
```

---

## Event Handling

### Basic Event Handling

```vera
let button = Button::new("btn", "Click");

button.on_click = Some(Box::new(|| {
    println!("Button was clicked!");
}));

// In your event loop:
let event = UIEvent {
    event_type: EventType::MouseClick,
    target_id: "btn".to_string(),
    timestamp: get_current_time_ms(),
    x: 100,
    y: 100,
    key_code: None,
    data: HashMap::new(),
};

ui.handle_event(event);
```

### Event Dispatcher

```vera
let mut dispatcher = EventDispatcher::new();

// Register listener for click events
dispatcher.on("MouseClick", |event| {
    println!("Clicked at ({}, {})", event.x, event.y);
});

// Register listener for key events
dispatcher.on("KeyDown", |event| {
    if let Some(code) = event.key_code {
        println!("Key pressed: {}", code);
    }
});

// Dispatch events
dispatcher.dispatch(&event);
```

---

## State Management

### Using Component State

```vera
let mut button = Button::new("btn", "Click");

// Set state
button.base.state.set("enabled", "true".to_string(), "btn".to_string());

// Get state
if let Some(enabled) = button.base.state.get("enabled") {
    println!("Button enabled: {}", enabled);
}

// Subscribe to changes
button.base.state.subscribe(|change| {
    println!("State changed: {} = {}", change.property, change.new_value);
});
```

### State Updates

```vera
let change = StateChange {
    component_id: "btn".to_string(),
    property: "enabled".to_string(),
    old_value: "true".to_string(),
    new_value: "false".to_string(),
};

button.update_state(change);
```

---

## Data Binding

### One-Way Binding

```vera
// Display user name from model
let binding = DataBinding {
    source: "model.user.name".to_string(),
    target: "label.text".to_string(),
    mode: "one-way".to_string(),
    transform: None,
};

let mut label = Label::new("lbl-name", "");
label.base.add_binding(binding);
```

### Two-Way Binding

```vera
// Sync slider value with model
let binding = DataBinding {
    source: "settings.volume".to_string(),
    target: "slider.value".to_string(),
    mode: "two-way".to_string(),
    transform: Some("scale(0, 100)".to_string()),
};

let mut slider = Slider::new("slider-volume", 0.0, 100.0);
slider.base.add_binding(binding);
```

---

## Animations

### Creating an Animation

```vera
// Create fade-in animation
let mut fade_in = Animation::new("fade-in", 300); // 300ms duration
fade_in.easing = "ease-out".to_string();

// Define keyframes
let mut start = HashMap::new();
start.insert("opacity".to_string(), 0.0);
fade_in.add_keyframe(0.0, start);

let mut end = HashMap::new();
end.insert("opacity".to_string(), 1.0);
fade_in.add_keyframe(100.0, end);

fade_in.auto_play = true;

// Attach to component
let mut button = Button::new("btn", "Click");
button.base.add_animation(fade_in);
```

### Creating a Transition

```vera
let transition = Transition {
    property: "opacity".to_string(),
    duration_ms: 150,
    delay_ms: 0,
    easing: "ease-out".to_string(),
    enabled: true,
};

let mut button = Button::new("btn", "Hover");
button.base.transitions.push(transition);
```

---

## Layout Examples

### Vertical Layout (Column)

```vera
let layout = LayoutConstraint {
    layout_type: LayoutType::Flex,
    flex_direction: FlexDirection::Column,
    align_items: Alignment::Stretch,
    justify_content: Alignment::Start,
    gap: 12.0,
    padding: Spacing::new(16.0),
    ..Default::new()
};
```

### Horizontal Layout (Row)

```vera
let layout = LayoutConstraint {
    layout_type: LayoutType::Flex,
    flex_direction: FlexDirection::Row,
    align_items: Alignment::Center,
    justify_content: Alignment::SpaceBetween,
    gap: 16.0,
    padding: Spacing::symmetric(8.0, 16.0),
    ..Default::new()
};
```

### Centered Layout

```vera
let layout = LayoutConstraint {
    layout_type: LayoutType::Flex,
    flex_direction: FlexDirection::Column,
    align_items: Alignment::Center,
    justify_content: Alignment::Center,
    width: LayoutValue::Percent(100.0),
    height: LayoutValue::Percent(100.0),
    ..Default::new()
};
```

---

## Accessibility

### Enable Accessibility

```vera
// Enable WCAG AAA compliance
ui.enable_accessibility(true);

// Configure accessibility
let mut accessibility = AccessibilityContext::new();
accessibility.apply_wcag_aaa_compliance();

// Set font size multiplier for visibility
accessibility.font_size_multiplier = 1.5;

// Enable high contrast mode
accessibility.high_contrast_enabled = true;
```

### Make Components Accessible

```vera
let button = Button::new("btn-save", "Save")
    .with_tooltip("Save changes (Ctrl+S)");
// Adds accessibility attributes for screen readers

let image = Image::new("img-logo", "/assets/logo.png");
image.alt_text = "Company Logo".to_string();
// Alternative text for screen readers
```

---

## DPI Awareness

### Set DPI Scale

```vera
// Standard scales
ui.set_dpi_scale(1.0);      // 96 DPI (100%)
ui.set_dpi_scale(1.25);     // 120 DPI (125%)
ui.set_dpi_scale(1.5);      // 144 DPI (150%)
ui.set_dpi_scale(2.0);      // 192 DPI (200%)

// Components automatically scale
let button = Button::new("btn", "Click");
// At 1.5 DPI, 100px becomes 150px
```

### Manual Scaling

```vera
let scaled_value = ui.layout_engine.scale_to_dpi(100.0);
// Returns 100.0 * dpi_scale
```

---

## Complete Example: Simple Form

```vera
use vera_ui_framework::*;

fn main() {
    // Setup
    let mut ui = VeraUIFramework::new("1.0");
    ui.set_theme(Theme::light());
    ui.set_viewport(600.0, 400.0);

    // Create window
    let mut window = Window::new("form-win", "Simple Form");
    window.base.width = 500.0;
    window.base.height = 350.0;

    // Create components
    let title = Label::new("lbl-title", "Contact Form");
    
    let mut name_field = TextBox::new("txt-name", "Enter your name");
    name_field.base.width = 450.0;
    
    let mut email_field = TextBox::new("txt-email", "Enter your email");
    email_field.base.width = 450.0;
    email_field = email_field.with_input_type("email");
    
    let mut message_field = TextBox::new("txt-message", "Enter message");
    message_field.base.width = 450.0;
    
    let mut submit = Button::new("btn-submit", "Submit")
        .with_variant("primary")
        .with_size(450.0, 40.0);

    submit.on_click = Some(Box::new(|| {
        println!("Form submitted!");
    }));

    // Register components
    ui.register_component(Box::new(window));
    ui.register_component(Box::new(title));
    ui.register_component(Box::new(name_field));
    ui.register_component(Box::new(email_field));
    ui.register_component(Box::new(message_field));
    ui.register_component(Box::new(submit));

    // Render
    let html = ui.render();
    println!("{}", html);
}
```

---

## Complete Example: Calculator

```vera
use vera_ui_framework::*;

fn main() {
    let mut ui = VeraUIFramework::new("1.0");
    ui.set_viewport(320.0, 480.0);

    // Display
    let mut display = TextBox::new("calc-display", "");
    display.base.width = 300.0;
    display.base.height = 60.0;
    display.read_only = true;
    display.value = "0".to_string();

    // Button grid
    let buttons = vec![
        ("7", "btn-7"), ("8", "btn-8"), ("9", "btn-9"), ("÷", "btn-div"),
        ("4", "btn-4"), ("5", "btn-5"), ("6", "btn-6"), ("×", "btn-mul"),
        ("1", "btn-1"), ("2", "btn-2"), ("3", "btn-3"), ("−", "btn-sub"),
        ("0", "btn-0"), (".", "btn-dot"), ("=", "btn-eq"), ("+", "btn-add"),
    ];

    for (label, id) in buttons {
        let button = Button::new(id, label)
            .with_size(60.0, 60.0);
        
        ui.register_component(Box::new(button));
    }

    // Register display
    ui.register_component(Box::new(display));
}
```

---

## Tips & Best Practices

### 1. Always Set Viewport

```vera
ui.set_viewport(1920.0, 1080.0);
```

### 2. Use Flex Layout by Default

```vera
// More responsive and flexible
let layout = LayoutConstraint {
    layout_type: LayoutType::Flex,
    // ...
};
```

### 3. Enable Accessibility

```vera
ui.enable_accessibility(true);
```

### 4. Use Themes Instead of Hardcoded Colors

```vera
// Good: Use theme colors
let color = ui.theme.colors.primary;

// Avoid: Hardcoding colors
let color = "#007AFF";
```

### 5. Clean Up Event Listeners

```vera
// Register listener
dispatcher.on("MouseClick", |event| { /* ... */ });

// Later, create new dispatcher when done
dispatcher = EventDispatcher::new();
```

### 6. Test at Multiple DPI Scales

```vera
ui.set_dpi_scale(1.0);    // Test at 100%
ui.set_dpi_scale(1.5);    // Test at 150%
ui.set_dpi_scale(2.0);    // Test at 200%
```

### 7. Use Animations Sparingly

```vera
// Good: Short UI feedback animations (150-300ms)
let animation = Animation::new("button-click", 150);

// Avoid: Long animations (>500ms) for common actions
```

---

## Common Patterns

### Master-Detail View

```vera
// Master list on left
let mut list = ListBox::new("list-items");

// Detail panel on right
let mut detail = Panel::new("panel-detail");

// When list item selected, update detail panel
list.on_selection_change = Some(Box::new(|indices| {
    // Update detail view
}));
```

### Modal Dialog

```vera
let mut dialog = Window::new("dialog-confirm", "Confirm");
dialog.modal = true;
dialog.resizable = false;

let yes_btn = Button::new("btn-yes", "Yes");
let no_btn = Button::new("btn-no", "No");

// Handle buttons
```

### Settings Form

```vera
// Use form with validation
let mut form = RegistrationForm::new();

// Validate before submit
if form.validate() {
    // Save settings
} else {
    // Show errors
}
```

---

## Debugging

### Enable Logging

```vera
// Subscribe to state changes
button.base.state.subscribe(|change| {
    println!("State change: {:?}", change);
});

// Subscribe to events
dispatcher.on("*", |event| {
    println!("Event: {:?}", event);
});
```

### Check Component Bounds

```vera
let (x, y, width, height) = button.get_bounds();
println!("Button bounds: {} x {} @ {}, {}", width, height, x, y);
```

---

## Next Steps

1. **Read the API Reference:** `VERA_UI_REFERENCE.md`
2. **Study Examples:** `VeraUIFramework_Examples.vera`
3. **Explore Architecture:** `VERA_ARCHITECTURE.md`
4. **Build Your App:** Start with a simple window and add components

---

## Resources

- **Framework Code:** `VeraUIFramework.vera` (2,500+ LOC)
- **Examples:** `VeraUIFramework_Examples.vera` (1,200+ LOC)
- **Full Reference:** `VERA_UI_REFERENCE.md`
- **Architecture:** `VERA_ARCHITECTURE.md`

---

## Support

For issues, feature requests, or contributions:
- Check the API reference for detailed documentation
- Review example code for implementation patterns
- Refer to architecture guide for design decisions

---

**VERA UI Framework v1.0 - Production Ready**
*Building Beautiful, Accessible Desktop UIs for Omnisystem*
