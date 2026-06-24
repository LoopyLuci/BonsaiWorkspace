# TITAN Window Manager - Quick Reference Guide

## Initialization

```titan
use TitanWindowManager::*;

// Create window manager with event queue capacity
let wm = create_window_manager(256);

// Start the manager
wm.start()?;
```

## Window Operations

### Create & Destroy

```titan
// Create window at position (100, 100) with size 800x600
let window_id = wm.create_window("Title".into(), 100, 100, 800, 600)?;

// Destroy window
wm.destroy_window(window_id)?;
```

### Position & Size

```titan
// Move window to new position
wm.move_window(window_id, 150, 150)?;

// Resize window
wm.resize_window(window_id, 1024, 768)?;

// Move and get bounds
let window = wm.get_window(window_id)?;
let bounds = window.unwrap().get_bounds();
// Returns: Rect { x, y, width, height }
```

### Window State

```titan
// Minimize window
wm.minimize_window(window_id)?;

// Maximize window
wm.maximize_window(window_id)?;

// Restore to normal state
wm.restore_window(window_id)?;

// Toggle fullscreen
wm.set_fullscreen(window_id, true)?;
```

### Focus & Z-Order

```titan
// Focus window (bring to front)
wm.focus_window(window_id)?;

// Remove focus
wm.blur_window(window_id)?;

// Get focused window
let focused = wm.get_focused_window()?;

// Get z-order (bottom to top)
let z_order = wm.get_z_order()?;
```

### Window Properties

```titan
// Set window title
wm.set_window_title(window_id, "New Title".into())?;

// Set border style
wm.set_window_border(window_id, WindowBorder::Standard)?;
// Options: None, Thin, Standard, Thick

// Set DPI scaling
wm.set_dpi_scaling(window_id, 1.5, 144)?;
// Scale: 1.0 (96 DPI), 1.25 (120), 1.5 (144), 2.0 (192)
```

## Queries

```titan
// Get window by ID
let window = wm.get_window(window_id)?;

// Get all windows
let all_windows = wm.get_all_windows()?;

// Get window count
let count = wm.get_window_count()?;

// Find window at point
let window_at_point = wm.get_window_at_point(400, 300)?;
// Returns topmost visible window at coordinates
```

## Input Events

### Mouse

```titan
// Mouse movement
wm.push_input_event(InputEvent::MouseMove {
    x: 400, y: 300,
    delta_x: 5, delta_y: 5
})?;

// Mouse click
wm.push_input_event(InputEvent::MouseClick {
    button: MouseButton::Left,  // Left, Right, Middle, Forward, Back
    x: 400, y: 300
})?;

// Mouse release
wm.push_input_event(InputEvent::MouseRelease {
    button: MouseButton::Left,
    x: 400, y: 300
})?;

// Mouse scroll
wm.push_input_event(InputEvent::MouseScroll {
    x: 400, y: 300,
    delta: 120,
    is_horizontal: false
})?;

// Mouse drag
wm.push_input_event(InputEvent::MouseDrag {
    button: MouseButton::Left,
    x: 450, y: 350,
    delta_x: 50, delta_y: 50
})?;
```

### Keyboard

```titan
// Key down
wm.push_input_event(InputEvent::KeyDown {
    code: KeyCode::Enter,  // or KeyCode::Char('A')
    modifiers: 0
})?;

// Key up
wm.push_input_event(InputEvent::KeyUp {
    code: KeyCode::Space,
    modifiers: 0x02  // Ctrl modifier
})?;

// Text input
wm.push_input_event(InputEvent::TextInput {
    text: "Hello".into()
})?;
```

### Touch & Gamepad

```titan
// Touch events
wm.push_input_event(InputEvent::TouchDown { id: 0, x: 400, y: 300 })?;
wm.push_input_event(InputEvent::TouchMove { id: 0, x: 410, y: 310 })?;
wm.push_input_event(InputEvent::TouchUp { id: 0, x: 420, y: 320 })?;

// Gamepad button
wm.push_input_event(InputEvent::GamepadButtonDown {
    button: 0,  // A button
    gamepad_id: 0
})?;

// Gamepad analog
wm.push_input_event(InputEvent::GamepadAnalog {
    axis: 0,  // Left stick X
    value: 0.75,
    gamepad_id: 0
})?;
```

## Event Processing

```titan
// Pop event from queue
while let Some(event) = wm.pop_event() {
    match event {
        Event::Input(input_event) => {
            // Handle input: mouse, keyboard, touch, gamepad
        }
        Event::Window(window_event) => {
            // Handle window: created, destroyed, moved, resized, focused, etc.
        }
        Event::System(system_event) => {
            // Handle system: display, power, theme changes
        }
    }
}

// Peek at next event without removing
let next = wm.peek_event();

// Check pending events
let pending = wm.pending_events();

// Process input (updates input state)
wm.process_input_event(InputEvent::KeyDown {
    code: KeyCode::Char('A'),
    modifiers: 0
})?;

// Get current input state
let state = wm.get_input_state()?;
println!("Mouse: ({}, {})", state.mouse_x, state.mouse_y);
println!("Shift: {}", state.is_shift_pressed());
println!("Ctrl: {}", state.is_ctrl_pressed());
println!("Alt: {}", state.is_alt_pressed());
```

## Window Events

```titan
// Create custom window events
wm.push_window_event(WindowEvent::Created { window_id: 1 })?;
wm.push_window_event(WindowEvent::Moved { window_id: 1, x: 100, y: 100 })?;
wm.push_window_event(WindowEvent::Resized { window_id: 1, width: 800, height: 600 })?;
wm.push_window_event(WindowEvent::Focused { window_id: 1 })?;
wm.push_window_event(WindowEvent::Blurred { window_id: 1 })?;
wm.push_window_event(WindowEvent::Minimized { window_id: 1 })?;
wm.push_window_event(WindowEvent::Maximized { window_id: 1 })?;
wm.push_window_event(WindowEvent::Restored { window_id: 1 })?;
wm.push_window_event(WindowEvent::CloseRequested { window_id: 1 })?;
wm.push_window_event(WindowEvent::Close { window_id: 1 })?;
```

## System Events

```titan
// Display events
wm.push_system_event(SystemEvent::DisplayConnected {
    display: Display { /* ... */ }
})?;
wm.push_system_event(SystemEvent::DisplayDisconnected { display_id: 1 })?;
wm.push_system_event(SystemEvent::DisplayModeChanged { display_id: 0 })?;

// Power/Theme events
wm.push_system_event(SystemEvent::PowerStateChanged { state: 1 })?;
wm.push_system_event(SystemEvent::ThemeChanged { is_dark: true })?;
```

## Display Management

### Register Display

```titan
let display = Display {
    id: 0,
    name: "Monitor 1".into(),
    x: 0, y: 0,
    width: 1920, height: 1080,
    dpi: 96,
    refresh_rate: 60,
    is_primary: true,
};
wm.register_display(display)?;
```

### Query Displays

```titan
// Get all displays
let displays = wm.get_displays()?;

// Get primary display
let primary = wm.get_primary_display()?;

// Single display by ID
let display = wm.get_primary_display()?.unwrap();
println!("Display: {}x{} @ {}Hz", display.width, display.height, display.refresh_rate);
```

## Statistics & Control

```titan
// Get statistics
let stats = wm.get_stats()?;
println!("{}", stats.report());
// Output:
// 📊 Window Manager Stats:
//   Windows: 5
//   Pending Events: 3
//   Focused: Some(1)
//   Running: true

// Check running status
let is_running = wm.is_running();

// Clear event queue
wm.clear_events()?;

// Stop window manager
wm.stop()?;
```

## Key Data Structures

### Window State
```titan
enum WindowState {
    Hidden,
    Visible,
    Minimized,
    Maximized,
    FullScreen,
    Closing,
}
```

### Window Border
```titan
enum WindowBorder {
    None,
    Thin,
    Standard,
    Thick,
}
```

### Mouse Button
```titan
enum MouseButton {
    Left,
    Right,
    Middle,
    Forward,
    Back,
}
```

### Key Code
```titan
enum KeyCode {
    Enter, Escape, Backspace, Tab, Space,
    ArrowUp, ArrowDown, ArrowLeft, ArrowRight,
    Delete, Home, End, PageUp, PageDown,
    F1, F2, ... F12,
    Char(char),
    Other(u32),
}
```

### Modifier Flags
```titan
// Shift: 0x01
// Ctrl:  0x02
// Alt:   0x04
// Super: 0x08
let modifiers = 0x02 | 0x04;  // Ctrl + Alt
```

## Common Patterns

### Event Loop

```titan
let wm = create_window_manager(256);
wm.start()?;

loop {
    // Process all pending events
    while let Some(event) = wm.pop_event() {
        match event {
            Event::Input(_) => { /* handle */ }
            Event::Window(WindowEvent::CloseRequested { window_id }) => {
                wm.destroy_window(window_id)?;
            }
            _ => {}
        }
    }
    
    // Render frame
    // ...
}
```

### Window Click Detection

```titan
wm.push_input_event(InputEvent::MouseClick {
    button: MouseButton::Left,
    x: 400, y: 300
})?;

if let Ok(Some(window_id)) = wm.get_window_at_point(400, 300) {
    wm.focus_window(window_id)?;
}
```

### Keyboard Shortcuts

```titan
wm.push_input_event(InputEvent::KeyDown {
    code: KeyCode::Char('S'),
    modifiers: 0x02  // Ctrl+S
})?;

let state = wm.get_input_state()?;
if state.is_ctrl_pressed() {
    // Save action
}
```

### Multi-Monitor Setup

```titan
// Primary monitor
wm.register_display(Display {
    id: 0,
    name: "Primary".into(),
    x: 0, y: 0,
    width: 1920, height: 1080,
    dpi: 96,
    refresh_rate: 60,
    is_primary: true,
})?;

// Secondary monitor (to the right)
wm.register_display(Display {
    id: 1,
    name: "Secondary".into(),
    x: 1920, y: 0,  // Adjacent to primary
    width: 2560, height: 1440,
    dpi: 96,
    refresh_rate: 144,
    is_primary: false,
})?;
```

## Platform Integration

### Windows-Specific

```titan
use WindowManagerPlatformIntegration::*;

let wm = WindowsWindowManager::new();
let handle = wm.create_native_window("Title", 100, 100, 800, 600)?;
wm.show_window(handle.hwnd, 1)?;  // SW_SHOWNORMAL

let cursor = WindowsCursorManager::new();
cursor.set_cursor(4)?;  // Hand cursor
cursor.clip_cursor(0, 0, 1920, 1080)?;
```

### macOS-Specific

```titan
let wm = MacOSWindowManager::new();
let handle = wm.create_native_window("Title", 100, 100, 800, 600)?;
wm.make_key_and_ordered_front(handle.nswindow)?;

let trackpad = MacOSTrackpadManager::new();
trackpad.handle_pinch(1.5)?;
```

### Linux X11-Specific

```titan
let wm = LinuxWindowManager::new();
let handle = wm.create_native_window("Title", 100, 100, 800, 600)?;
wm.map_window(handle.window_id)?;
```

## Error Handling

```titan
match wm.create_window("Title".into(), 100, 100, 800, 600) {
    Ok(window_id) => println!("Created: {}", window_id),
    Err(e) => eprintln!("Failed: {}", e),
}

// All functions return Result<T, String>
wm.move_window(id, 0, 0).expect("Move failed");
```

## Performance Tips

1. **Batch Events**: Process multiple events per frame
2. **Cache Queries**: Store frequently accessed data
3. **Minimize Locks**: Reduce lock contention
4. **Clear Queue**: Periodically clear old events
5. **Monitor Stats**: Use get_stats() to track performance

## Complete Example

```titan
use TitanWindowManager::*;

fn main() -> Result<(), String> {
    let wm = create_window_manager(256);
    wm.start()?;

    // Register display
    wm.register_display(Display {
        id: 0,
        name: "Primary".into(),
        x: 0, y: 0,
        width: 1920, height: 1080,
        dpi: 96,
        refresh_rate: 60,
        is_primary: true,
    })?;

    // Create window
    let win = wm.create_window("App".into(), 100, 100, 800, 600)?;
    wm.focus_window(win)?;

    // Event loop
    loop {
        while let Some(event) = wm.pop_event() {
            match event {
                Event::Window(WindowEvent::CloseRequested { window_id }) => {
                    if window_id == win {
                        return wm.stop();
                    }
                }
                _ => {}
            }
        }
        // Render...
    }
}
```

---

For full documentation, see: `TITAN_WINDOW_MANAGER.md`
