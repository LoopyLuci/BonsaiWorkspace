# TITAN Window Manager - Integration Guide

**Version**: 1.0.0  
**Date**: 2026-06-24  
**Status**: Production Ready

---

## Overview

This guide explains how to integrate the TITAN Window Manager into your Omnisystem applications and desktop environment.

---

## Files Overview

### Core Implementation (2,271+ lines of TITAN code)

| File | Lines | Purpose |
|------|-------|---------|
| `TitanWindowManager.titan` | 997 | Core window manager, event system, API |
| `WindowManager.Platform.Integration.titan` | 701 | Windows/macOS/Linux native integration |
| `WindowManager.Tests.titan` | 573 | Comprehensive test suite (18+ tests) |

### Documentation (1,100+ lines)

| File | Purpose |
|------|---------|
| `TITAN_WINDOW_MANAGER.md` | Complete API reference (400+ lines) |
| `WINDOW_MANAGER_QUICK_REFERENCE.md` | Quick API guide (300+ lines) |
| `WINDOW_MANAGER_SUMMARY.md` | Implementation overview (300+ lines) |
| `WINDOW_MANAGER_INTEGRATION_GUIDE.md` | This file |

---

## Quick Start

### 1. Import the Module

```titan
use TitanWindowManager::*;
use WindowManagerPlatformIntegration::*;
```

### 2. Initialize

```titan
// Create window manager with event queue capacity
let wm = create_window_manager(256);

// Start the manager
wm.start()?;
```

### 3. Create Windows

```titan
let main_window = wm.create_window(
    "My Application".into(),
    100, 100,      // x, y position
    800, 600       // width, height
)?;

// Optional: set properties
wm.set_window_border(main_window, WindowBorder::Standard)?;
wm.focus_window(main_window)?;
```

### 4. Event Loop

```titan
loop {
    while let Some(event) = wm.pop_event() {
        match event {
            Event::Input(input) => handle_input(input),
            Event::Window(window) => handle_window(window),
            Event::System(system) => handle_system(system),
        }
    }
    
    // Render frame
    render_scene();
    
    // Sleep briefly to prevent busy-waiting
    std::thread::sleep(std::time::Duration::from_millis(16));
}
```

### 5. Cleanup

```titan
// Destroy windows when done
wm.destroy_window(main_window)?;

// Stop the manager
wm.stop()?;
```

---

## Integration Patterns

### Desktop Environment Integration

For building a complete desktop shell:

```titan
pub struct DesktopEnvironment {
    window_manager: Arc<WindowManager>,
    taskbar: Window,
    panels: Vec<Window>,
}

impl DesktopEnvironment {
    pub fn new() -> Result<Self, String> {
        let wm = create_window_manager(512);
        wm.start()?;
        
        // Register all displays
        // ... (detect and register displays)
        
        // Create taskbar
        let taskbar = wm.create_window(
            "Taskbar".into(),
            0, 1080 - 50,
            1920, 50
        )?;
        
        Ok(DesktopEnvironment {
            window_manager: Arc::new(wm),
            taskbar,
            panels: Vec::new(),
        })
    }
}
```

### Application Window Management

For individual applications:

```titan
pub struct Application {
    window_manager: Arc<WindowManager>,
    main_window: u64,
    secondary_windows: Vec<u64>,
}

impl Application {
    pub fn new(title: &str) -> Result<Self, String> {
        let wm = create_window_manager(256);
        wm.start()?;
        
        let main_window = wm.create_window(
            title.into(),
            100, 100,
            1024, 768
        )?;
        
        Ok(Application {
            window_manager: Arc::new(wm),
            main_window,
            secondary_windows: Vec::new(),
        })
    }
}
```

### Modal Dialog Handling

```titan
pub fn show_dialog(wm: &WindowManager, title: &str) -> Result<u64, String> {
    let dialog = wm.create_window(title.into(), 400, 300, 400, 200)?;
    
    // Get center of screen
    if let Ok(Some(display)) = wm.get_primary_display() {
        let center_x = display.x + (display.width as i32 - 400) / 2;
        let center_y = display.y + (display.height as i32 - 200) / 2;
        wm.move_window(dialog, center_x, center_y)?;
    }
    
    // Modal dialogs should block parent window input
    wm.focus_window(dialog)?;
    
    Ok(dialog)
}
```

---

## Platform-Specific Integration

### Windows Integration

For Windows-native features:

```titan
use WindowManagerPlatformIntegration::*;

fn setup_windows_native() -> Result<(), String> {
    let wm = WindowsWindowManager::new();
    let platform = detect_platform();
    
    if platform.platform == Platform::Windows {
        // Create native Windows window
        let handle = wm.create_native_window(
            "Native Window",
            100, 100,
            800, 600
        )?;
        
        // Use Win32 APIs
        wm.show_window(handle.hwnd, 1)?; // SW_SHOWNORMAL
        wm.set_window_pos(handle.hwnd, 100, 100, 800, 600)?;
        
        // Cursor management
        let cursor_mgr = WindowsCursorManager::new();
        cursor_mgr.set_cursor(4)?; // Hand cursor
    }
    
    Ok(())
}
```

### macOS Integration

```titan
fn setup_macos_native() -> Result<(), String> {
    let wm = MacOSWindowManager::new();
    let platform = detect_platform();
    
    if platform.platform == Platform::MacOS {
        // Create native NSWindow
        let handle = wm.create_native_window(
            "macOS Window",
            100, 100,
            800, 600
        )?;
        
        // Cocoa operations
        wm.make_key_and_ordered_front(handle.nswindow)?;
        wm.set_window_frame(handle.nswindow, 100, 100, 800, 600)?;
        
        // Trackpad support
        let trackpad = MacOSTrackpadManager::new();
        // ... handle trackpad events
    }
    
    Ok(())
}
```

### Linux Integration

```titan
fn setup_linux_native() -> Result<(), String> {
    let wm = LinuxWindowManager::new();
    let platform = detect_platform();
    
    if platform.platform == Platform::Linux {
        // X11 support
        let handle = wm.create_native_window(
            "X11 Window",
            100, 100,
            800, 600
        )?;
        
        wm.map_window(handle.window_id)?;
        wm.move_window(handle.window_id, 100, 100)?;
        
        // Wayland support
        let wayland = WaylandWindowManager::new();
        let surface = wayland.create_surface("Wayland Surface", 800, 600)?;
        wayland.commit_surface(surface)?;
    }
    
    Ok(())
}
```

---

## Input Handling Patterns

### Mouse Input

```titan
fn handle_mouse_input(wm: &WindowManager, event: InputEvent) -> Result<(), String> {
    match event {
        InputEvent::MouseClick { button, x, y } => {
            // Find window at click position
            if let Ok(Some(window_id)) = wm.get_window_at_point(x, y) {
                // Focus the clicked window
                wm.focus_window(window_id)?;
                
                // Route click to window
                // ... (window-specific handling)
            }
        }
        InputEvent::MouseScroll { x, y, delta, is_horizontal } => {
            // Handle scroll in window at position
            if let Ok(Some(window_id)) = wm.get_window_at_point(x, y) {
                // Process scroll event
                // ... (window-specific handling)
            }
        }
        _ => {}
    }
    Ok(())
}
```

### Keyboard Input

```titan
fn handle_keyboard_input(wm: &WindowManager, event: InputEvent) -> Result<(), String> {
    match event {
        InputEvent::KeyDown { code, modifiers } => {
            // Check for global shortcuts
            match code {
                KeyCode::Char('S') if modifiers & 0x02 != 0 => {
                    // Ctrl+S - save
                    println!("Save action");
                }
                KeyCode::Char('Q') if modifiers & 0x02 != 0 => {
                    // Ctrl+Q - quit
                    println!("Quit action");
                }
                _ => {}
            }
            
            // Route to focused window
            if let Ok(Some(focused)) = wm.get_focused_window() {
                // Window handles rest of keyboard input
                // ... (window-specific handling)
            }
        }
        _ => {}
    }
    Ok(())
}
```

### Touch Input

```titan
fn handle_touch_input(wm: &WindowManager, event: InputEvent) -> Result<(), String> {
    match event {
        InputEvent::TouchDown { id, x, y } => {
            if let Ok(Some(window_id)) = wm.get_window_at_point(x, y) {
                wm.focus_window(window_id)?;
                // Handle touch down in window
            }
        }
        InputEvent::TouchMove { id, x, y } => {
            // Track touch movement
            // ... (window-specific handling)
        }
        InputEvent::TouchUp { id, x, y } => {
            // Handle touch release
            // ... (window-specific handling)
        }
        _ => {}
    }
    Ok(())
}
```

---

## Multi-Monitor Support

### Display Registration

```titan
pub fn setup_multi_monitor(wm: &WindowManager) -> Result<(), String> {
    // Example: Two displays in extended mode
    
    // Primary display (1920x1080)
    wm.register_display(Display {
        id: 0,
        name: "ASUS 27\" (Primary)".into(),
        x: 0,
        y: 0,
        width: 1920,
        height: 1080,
        dpi: 96,
        refresh_rate: 60,
        is_primary: true,
    })?;
    
    // Secondary display (2560x1440) positioned to the right
    wm.register_display(Display {
        id: 1,
        name: "Dell 27\"".into(),
        x: 1920,     // Adjacent to primary
        y: 0,
        width: 2560,
        height: 1440,
        dpi: 96,
        refresh_rate: 144,
        is_primary: false,
    })?;
    
    Ok(())
}
```

### DPI Scaling for High-Resolution Displays

```titan
pub fn apply_dpi_scaling(wm: &WindowManager, window_id: u64, display_id: u32) -> Result<(), String> {
    // Get display info
    if let Ok(Some(display)) = wm.display_manager.get_display(display_id) {
        // Calculate scale factor
        let scale_factor = display.dpi as f32 / 96.0;
        
        // Apply scaling to window
        wm.set_dpi_scaling(window_id, scale_factor, display.dpi)?;
        
        println!("Applied {:.2}x scaling ({}DPI)", scale_factor, display.dpi);
    }
    
    Ok(())
}
```

---

## Event Processing Loop

### Complete Example

```titan
pub fn run_application_event_loop(wm: Arc<WindowManager>) -> Result<(), String> {
    let mut running = true;
    
    while running {
        // Process all pending events
        while let Some(event) = wm.pop_event() {
            match event {
                Event::Input(input_event) => {
                    match input_event {
                        InputEvent::MouseClick { button, x, y } => {
                            // Handle mouse click
                            if let Ok(Some(window)) = wm.get_window_at_point(x, y) {
                                wm.focus_window(window)?;
                            }
                        }
                        InputEvent::KeyDown { code, modifiers } => {
                            // Handle keyboard
                            if code == KeyCode::Escape && modifiers & 0x02 != 0 {
                                running = false; // Ctrl+Escape exits
                            }
                        }
                        _ => {}
                    }
                }
                Event::Window(window_event) => {
                    match window_event {
                        WindowEvent::CloseRequested { window_id } => {
                            // Handle window close
                            wm.destroy_window(window_id)?;
                        }
                        WindowEvent::Resized { window_id, width, height } => {
                            // Handle resize - trigger re-render
                            println!("Window {} resized to {}x{}", window_id, width, height);
                        }
                        _ => {}
                    }
                }
                Event::System(system_event) => {
                    match system_event {
                        SystemEvent::DisplayConnected { display } => {
                            println!("Display connected: {}", display.name);
                            // Handle new display
                        }
                        SystemEvent::PowerStateChanged { state } => {
                            println!("Power state: {}", state);
                            // Handle power events
                        }
                        _ => {}
                    }
                }
            }
        }
        
        // Update application state
        // ... (application-specific logic)
        
        // Render frame
        // ... (rendering logic)
        
        // Sleep to maintain frame rate (60 FPS)
        std::thread::sleep(std::time::Duration::from_millis(16));
    }
    
    Ok(())
}
```

---

## Testing Integration

### Running Tests

```bash
# Run all window manager tests
cargo test --lib WindowManagerTests::run_all_tests

# Run specific test
cargo test --lib WindowManagerTests::test_event_latency

# Run with output
cargo test --lib WindowManagerTests -- --nocapture --test-threads=1
```

### Adding Custom Tests

```titan
pub fn test_my_integration() -> TestResult {
    let wm = create_window_manager(256);
    let start = get_time_ms();
    
    // Your test code here
    let window = wm.create_window("Test".into(), 0, 0, 100, 100)?;
    
    if window > 0 {
        TestResult::passed("My Integration Test", get_time_ms() - start)
    } else {
        TestResult::failed("My Integration Test", "Failed", get_time_ms() - start)
    }
}
```

---

## Performance Optimization Tips

### 1. Event Queue Size

```titan
// For low-frequency events (30 events/sec)
let wm = create_window_manager(64);

// For high-frequency events (500+ events/sec)
let wm = create_window_manager(512);
```

### 2. Batch Processing

```titan
// Process multiple events efficiently
let mut events_processed = 0;
while let Some(event) = wm.pop_event() {
    handle_event(event);
    events_processed += 1;
    
    // Process up to 10 events per frame
    if events_processed >= 10 {
        break;
    }
}
```

### 3. Window Query Caching

```titan
// Don't call get_window() repeatedly
let window = wm.get_window(window_id)?;
if let Some(w) = window {
    // Use cached window data
    println!("Title: {}", w.title);
    println!("Position: ({}, {})", w.position.x, w.position.y);
}
```

### 4. Z-Order Optimization

```titan
// Z-order is computed on demand
// Minimize calls in hot loops
let z_order = wm.get_z_order()?;
for window_id in z_order {
    // Process windows in z-order
}
```

---

## Common Patterns

### Focus Cycling

```titan
pub fn cycle_window_focus(wm: &WindowManager) -> Result<(), String> {
    let z_order = wm.get_z_order()?;
    
    if z_order.is_empty() {
        return Ok(());
    }
    
    // Get current focused window
    let current_focused = wm.get_focused_window()?;
    
    // Find next window
    let next_idx = if let Some(focused) = current_focused {
        if let Some(pos) = z_order.iter().position(|&id| id == focused) {
            (pos + 1) % z_order.len()
        } else {
            z_order.len() - 1
        }
    } else {
        z_order.len() - 1
    };
    
    wm.focus_window(z_order[next_idx])?;
    Ok(())
}
```

### Maximize to Available Space

```titan
pub fn maximize_window_to_space(
    wm: &WindowManager,
    window_id: u64,
) -> Result<(), String> {
    if let Ok(Some(display)) = wm.get_primary_display() {
        // Get taskbar height (example: 50px)
        let taskbar_height = 50;
        
        wm.move_window(window_id, display.x, display.y)?;
        wm.resize_window(
            window_id,
            display.width,
            display.height - taskbar_height as u32,
        )?;
    }
    
    Ok(())
}
```

### Window Snapping

```titan
pub fn snap_window_to_grid(
    wm: &WindowManager,
    window_id: u64,
    snap_size: i32,
) -> Result<(), String> {
    if let Ok(Some(mut window)) = wm.get_window(window_id) {
        let snapped_x = (window.position.x / snap_size) * snap_size;
        let snapped_y = (window.position.y / snap_size) * snap_size;
        
        wm.move_window(window_id, snapped_x, snapped_y)?;
    }
    
    Ok(())
}
```

---

## Troubleshooting

### Issue: High Event Latency

**Symptom**: Sluggish input response  
**Solution**:
1. Increase queue size: `create_window_manager(512)`
2. Process events more frequently
3. Profile with `test_event_latency()`

### Issue: Memory Leaks

**Symptom**: Memory usage increasing over time  
**Solution**:
1. Ensure windows destroyed: `wm.destroy_window(id)`
2. Clear events: `wm.clear_events()`
3. Monitor with `wm.get_stats()`

### Issue: Window Not Appearing

**Symptom**: Window created but not visible  
**Solution**:
1. Ensure `wm.start()` called
2. Check platform integration (native handles)
3. Verify display registration
4. Check z-order (may be behind others)

### Issue: Platform-Specific Crashes

**Symptom**: Crash on Windows/macOS/Linux  
**Solution**:
1. Run platform-specific tests
2. Check error messages
3. Verify platform libraries are available
4. Review platform-specific code

---

## Advanced Topics

### Custom Event Filtering

```titan
pub fn create_filtered_event_loop<F>(
    wm: &WindowManager,
    filter: F,
) where
    F: Fn(&Event) -> bool,
{
    while let Some(event) = wm.pop_event() {
        if filter(&event) {
            // Process this event
        }
        // Other events are dropped
    }
}
```

### Multi-Threaded Event Handling

```titan
use std::sync::Arc;
use std::thread;

pub fn spawn_event_handler(wm: Arc<WindowManager>) {
    thread::spawn(move || {
        while let Some(event) = wm.pop_event() {
            // Handle event in separate thread
            handle_event_async(event);
        }
    });
}
```

### Event Timing Analysis

```titan
pub fn profile_event_processing(wm: &WindowManager) {
    let stats = wm.get_stats().unwrap();
    println!("📊 Window Manager Profile:");
    println!("  Windows: {}", stats.total_windows);
    println!("  Pending Events: {}", stats.pending_events);
    println!("  Focused: {:?}", stats.focused_window);
    println!("  Running: {}", stats.is_running);
}
```

---

## API Quick Reference

### Initialization
```titan
let wm = create_window_manager(256);
wm.start()?;
```

### Window Management
```titan
let win = wm.create_window("Title".into(), 100, 100, 800, 600)?;
wm.move_window(win, 150, 150)?;
wm.resize_window(win, 900, 700)?;
wm.focus_window(win)?;
wm.destroy_window(win)?;
```

### Events
```titan
wm.push_input_event(event)?;
while let Some(event) = wm.pop_event() { /* handle */ }
```

### Queries
```titan
let window = wm.get_window(window_id)?;
let focused = wm.get_focused_window()?;
let window_at = wm.get_window_at_point(x, y)?;
let z_order = wm.get_z_order()?;
```

---

## Conclusion

The TITAN Window Manager provides a complete, production-ready desktop window management system. For detailed API reference, see `TITAN_WINDOW_MANAGER.md`. For quick reference, see `WINDOW_MANAGER_QUICK_REFERENCE.md`.

**Ready for production integration!**
