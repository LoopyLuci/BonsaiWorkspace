# NEXUS Responsive Design Module
## Mobile-First Adaptive Layout & Device-Aware UI System

**Version:** 31.0.0  
**Status:** Production-ready  
**Language:** NEXUS (Mobile/Responsive Design)  
**Lines of Code:** 3,200+  
**Location:** `src/responsive/NexusResponsiveDesign.nexus`

---

## Overview

NEXUS is a comprehensive responsive design module for Omnisystem that enables adaptive, device-aware interfaces across all form factors—from smartwatches (1.5") to 8K displays (40"+). Built with mobile-first philosophy, NEXUS handles responsive layouts, device detection, DPI-aware scaling, touch gestures, and adaptive content delivery.

### Key Capabilities

- **Responsive Breakpoints:** 8 standard breakpoints from XS (< 480px) to 8K (> 7680px)
- **Device Detection:** 6 device types (Phone, Tablet, Laptop, Desktop, TV, Wearable) with capability tracking
- **DPI-Aware Scaling:** Automatic scaling from 96dpi to 576dpi for crisp displays
- **Touch & Gesture Handling:** Complete gesture recognition (tap, swipe, pinch, rotate, pan)
- **Adaptive Layouts:** Responsive grids, fluid typography, container queries
- **Safe Area Management:** Notch detection, status bar avoidance, rounded corner handling
- **Device Capabilities:** Touch, sensors, connectivity, battery, thermal tracking
- **Orientation Detection:** Portrait, landscape, square mode support
- **Quality Adaptation:** Rendering quality based on device capabilities & battery
- **Responsive Images:** Multi-resolution asset selection and loading strategies

---

## Core Components

### 1. Breakpoint System

**Standard Breakpoints:**
```
XS   < 480px    Extra Small (smartwatch)
SM   480-576px  Small (small phone)
MD   576-768px  Medium (large phone, tablet)
LG   768-992px  Large (tablet, small laptop)
XL   992-1200px Extra Large (desktop)
XXL  1200-2560px Ultra-wide (large desktop)
UHD  2560-7680px 4K display
8K   > 7680px   8K resolution
```

**Usage:**
```nexus
let breakpoints = BreakpointConfig::new();
let class = breakpoints.classify_width(480);  // Returns: Mobile

if breakpoints.matches_breakpoint(800, "md") {
    // Apply medium breakpoint styles
}
```

### 2. Device Detection

**Device Type Classification:**
```nexus
pub enum DeviceType {
    Phone,       // 5-7" screens, high DPI (200-600dpi)
    Tablet,      // 8-12" screens, medium-high DPI
    Laptop,      // 13-15" screens, ~96-160dpi
    Desktop,     // 24-32" screens, 96dpi
    TV,          // 40+ screens, 25-30dpi
    Wearable,    // ~1.5" screens (smartwatch)
}
```

**Device Detector Usage:**
```nexus
let mut device = DeviceDetector::new(1080, 1920, 420);  // Pixel 6 parameters

println!("Device: {:?}", device.device_type);          // Phone
println!("Orientation: {:?}", device.orientation);     // Portrait
println!("Safe Area: {:?}", device.safe_area);         // Notch info
println!("DPI: {}", device.dpi);                       // 420 dpi
println!("Device Pixel Ratio: {}", device.device_pixel_ratio);  // 4.375x
```

### 3. Responsive Layouts

#### Adaptive Grid Layout
```nexus
let grid = AdaptiveGridLayout::new();

// Automatically adjusts columns per breakpoint
// Mobile: 1 col, Tablet: 3 cols, Desktop: 4 cols, UltraWide: 6 cols

let breakpoint = BreakpointClass::Tablet;
let columns = grid.get_columns(&breakpoint);       // 3
let gap = grid.get_gap(&breakpoint);               // 16px
let padding = grid.get_padding(&breakpoint);       // 16px

// Calculate item size in responsive grid
let item_width = grid.calculate_item_width(
    768,                           // container width
    &BreakpointClass::Tablet
);
// Returns: (768 - 32 padding - 32 gap) / 3 = 234px per item
```

#### Fluid Typography
```nexus
let typography = FluidTypography::new();

// Font sizes scale smoothly across breakpoints
let h1_mobile = typography.calculate_size(&BreakpointClass::Mobile, "h1");
// 28px (base 14px * 2.0 scale)

let h1_desktop = typography.calculate_size(&BreakpointClass::Desktop, "h1");
// 36px (base 18px * 2.0 scale)

// Get appropriate line height
let line_height = typography.get_line_height(36.0);  // 1.6
```

### 4. Touch & Gesture Handling

**Gesture Recognition:**
```nexus
let mut recognizer = GestureRecognizer::new();

// Register touch events
recognizer.touch_began(TouchPoint {
    id: 0,
    x: 100.0,
    y: 150.0,
    pressure: 0.8,
    radius: 12.0,
    timestamp: get_current_millis(),
});

// Track motion
recognizer.touch_moved(0, 150.0, 200.0);

// Recognize gesture on touch end
if let Some(gesture) = recognizer.touch_ended(0) {
    match gesture {
        GestureType::SingleTap => { /* ... */ },
        GestureType::DoubleTap => { /* ... */ },
        GestureType::Swipe { direction, velocity } => { /* ... */ },
        GestureType::Pinch { scale, velocity } => { /* ... */ },
        _ => {},
    }
}
```

**Touch-Friendly Sizing:**
```nexus
let touch_sizes = TouchTargetSizes::new();
// minimum_size: 44pt (Apple standard)
// recommended_size: 48dp (Material Design)
// touch_padding: 12px
// minimum_spacing: 8px

// Check if touch is in target area
let hit = recognizer.is_touch_in_bounds(
    touch_x, touch_y,
    button_x, button_y, button_w, button_h,
    touch_sizes.touch_padding
);
```

### 5. DPI-Aware Scaling

**Automatic DPI Scaling:**
```nexus
let dpi_scaling = DPIScaling::new(420);  // Pixel 6 phone

// Convert between logical and physical pixels
let logical_16px = 16;
let physical_px = dpi_scaling.logical_to_physical(16);  // ~73px

let dpi_class = dpi_scaling.get_dpi_class();
// Returns: DPIClass::UltraHighDPI (220-330dpi range)

// Scale all dimensions automatically
let (scaled_w, scaled_h) = dpi_scaling.scale_dimensions(100, 200);
```

**DPI Classes:**
- **LowDPI** (< 96dpi): Old displays, external monitors
- **MediumDPI** (96-150dpi): Standard desktop monitors
- **HighDPI** (150-220dpi): Tablets, some phones
- **UltraHighDPI** (220-330dpi): Modern flagship phones
- **SuperHighDPI** (> 330dpi): Premium phones, flagship tablets

### 6. Responsive Images

**Multi-Resolution Asset Selection:**
```nexus
let image = ResponsiveImage::new("/images/photo.jpg".to_string());
image.source_1x = "/images/photo.jpg".to_string();
image.source_2x = "/images/photo@2x.jpg".to_string();
image.source_3x = "/images/photo@3x.jpg".to_string();

// Width-based sources for different screen sizes
image.width_sources.push((480, "/images/photo-mobile.jpg".to_string()));
image.width_sources.push((1024, "/images/photo-tablet.jpg".to_string()));

// Select best source automatically
let best_source = image.select_source(&device);  // Based on DPI & width
let srcset = image.get_srcset();  // For HTML srcset attribute
```

### 7. Safe Area Handling (Notches, Status Bars)

**Notch & Safe Area Detection:**
```nexus
let mut device = DeviceDetector::new(1080, 2340, 420);

// Set safe area insets (e.g., iPhone with notch)
device.safe_area = SafeAreaInsets::from_notch(
    40,    // top (notch height)
    50,    // bottom (home indicator)
    0,     // left
    0      // right
);

// Apply safe area to rectangles
let content_rect = Rectangle { x: 0, y: 0, width: 1080, height: 2340 };
let safe_rect = device.safe_area.apply_to_rect(&content_rect);
// Returns: Rectangle { x: 0, y: 40, width: 1080, height: 2250 }
```

### 8. Device Capabilities

**Tracking Device Features:**
```nexus
let mut capabilities = DeviceCapabilities::default();

// Set device features
capabilities.has_touch = true;
capabilities.max_touch_points = 10;
capabilities.has_accelerometer = true;
capabilities.has_gyroscope = true;
capabilities.has_front_camera = true;
capabilities.has_rear_camera = true;
capabilities.gpu_capable = true;
capabilities.supports_hdr = true;
capabilities.max_refresh_rate = 120;

// Battery awareness
capabilities.battery_level = 0.5;
capabilities.low_power_mode = false;

// Network efficiency
capabilities.is_low_bandwidth = false;

// Get quality level based on capabilities
let quality = capabilities.get_quality_level();
// Returns: QualityLevel::High (if all capabilities are good)
```

### 9. Responsive System Manager

**Central Control & Coordination:**
```nexus
// Initialize responsive system
let mut responsive = ResponsiveSystem::new(1080, 1920, 420);

// Get current state
let breakpoint = responsive.current_breakpoint();      // Mobile
let font_size = responsive.get_heading_size(1);        // h1 size
let is_touch = responsive.is_touch_device();           // true
let should_save_battery = responsive.should_enable_battery_saver();

// Handle events
responsive.on_resize(1440, 1920);     // Device rotated
responsive.on_orientation_change();
responsive.update_capabilities(new_capabilities);

// Get rendering quality
let quality = responsive.get_rendering_quality();
if quality.blur_effects {
    // Apply blur effects
}
```

### 10. Viewport Configuration

**Web/Mobile Viewport Settings:**
```nexus
// Mobile viewport
let mobile_viewport = ViewportConfig::new_mobile();
// width=device-width, initial-scale=1.0, user-scalable=yes
// viewport-fit=contain (avoid notch)

// Desktop viewport  
let desktop_viewport = ViewportConfig::new_desktop();
// No explicit width, scalable from 0.5x to 2.0x
// viewport-fit=auto

let meta_tag = mobile_viewport.get_meta_tag();
// Output: "viewport=device-width, initial-scale=1, minimum-scale=1, maximum-scale=5, user-scalable=yes"
```

---

## Integration Patterns

### With VERA UI Components
```nexus
// VERA component adapts to responsive system
struct ResponsiveButton {
    base_width: i32,
    base_height: i32,
    responsive: ResponsiveSystem,
}

impl ResponsiveButton {
    fn render(&self) {
        let (width, height) = self.responsive
            .adapter
            .adapt_component(self.base_width, self.base_height);
        
        // Render button with adaptive size
    }
}
```

### With HELIX Rendering
```nexus
// Render quality adapts to device capabilities
let quality = responsive.get_rendering_quality();

if quality.anti_aliasing {
    // Enable MSAA/FXAA in HELIX renderer
    helix::enable_antialiasing();
}

if quality.blur_effects {
    // Apply post-processing blur
    helix::enable_bloom();
}
```

### With TITAN Touch Input
```nexus
// TITAN delivers raw touch events to NEXUS
gesture_recognizer.touch_began(touch_point);
gesture_recognizer.touch_moved(touch_id, x, y);

if let Some(gesture) = gesture_recognizer.touch_ended(touch_id) {
    // Route gesture to UI component
}
```

### With SYLVA Layout Algorithms
```nexus
// SYLVA computes layout, NEXUS applies responsive rules
let grid_config = responsive.grid.get_columns(&breakpoint);
let gap = responsive.grid.get_gap(&breakpoint);

// SYLVA uses these values to layout components
sylva::compute_grid_layout(grid_config, gap);
```

---

## Mobile Optimization Features

### 1. Battery Efficiency

```nexus
if responsive.should_enable_battery_saver() {
    // Reduce frame rate
    set_target_fps(30);
    
    // Disable expensive effects
    disable_animations();
    disable_shadows();
}
```

### 2. Network Awareness

```nexus
if device.capabilities.is_low_bandwidth {
    // Use lower resolution images
    image.select_source_for_bandwidth(&device);
    
    // Defer non-critical assets
    defer_background_images();
}
```

### 3. Thermal Throttling Detection

```nexus
if device.capabilities.is_thermal_throttling {
    // Reduce computational load
    reduce_particle_count();
    lower_physics_simulation_rate();
}
```

### 4. Memory-Aware Rendering

```nexus
let available_memory = device.capabilities.available_memory_mb;

if available_memory < 256 {
    // Reduce texture resolution
    // Decrease draw call count
    // Clear caches more frequently
}
```

---

## Responsive Breakpoint Reference

| Breakpoint | Range | Primary Devices | Columns | Gap | Padding |
|---|---|---|---|---|---|
| **XS** | < 480px | Smartwatch | 1 | 8px | 8px |
| **SM** | 480-576px | Small phone | 1-2 | 8px | 8px |
| **MD** | 576-768px | Phone/tablet | 2-3 | 12px | 12px |
| **LG** | 768-992px | Tablet | 3-4 | 16px | 16px |
| **XL** | 992-1200px | Desktop | 4-6 | 20px | 20px |
| **XXL** | 1200-2560px | Large desktop | 6-8 | 24px | 24px |
| **UHD** | 2560-7680px | 4K display | 8+ | 28px | 28px |
| **8K** | > 7680px | 8K display | 12+ | 32px | 32px |

---

## Device Type Characteristics

| Device Type | Diagonal | Common Sizes | DPI | Touch | Interaction |
|---|---|---|---|---|---|
| **Wearable** | ~1.5" | 280x360 | 300+ | Yes | Gesture-heavy |
| **Phone** | 5-7" | 1080x2340 | 200-600 | Yes | Touch-primary |
| **Tablet** | 8-12" | 2560x1600 | 160-300 | Yes | Touch+Keyboard |
| **Laptop** | 13-15" | 1920x1080 | 90-160 | Mixed | Trackpad+Keyboard |
| **Desktop** | 24-32" | 2560x1440 | 80-110 | No | Mouse+Keyboard |
| **TV** | 40-85" | 3840x2160 | 25-35 | No | Remote |

---

## Performance Considerations

### Rendering Quality Levels

**Low Quality** (Limited device):
- No anti-aliasing
- No shadows
- No blur effects
- 0 particles/effects
- Reduced frame rate (30fps)
- Simpler textures

**Medium Quality** (Typical device):
- FXAA anti-aliasing
- Real-time shadows
- No blur effects
- 50 particles
- Standard frame rate (60fps)
- Standard textures

**High Quality** (Premium device):
- MSAA/TAA anti-aliasing
- Advanced shadows
- Bloom/blur effects enabled
- 200+ particles
- High frame rate (120fps+)
- High-resolution textures

---

## Example: Complete Responsive App Setup

```nexus
// Initialize responsive system for current device
let mut responsive = ResponsiveSystem::new(1080, 1920, 420);

// Update with actual device capabilities
let mut capabilities = DeviceCapabilities::default();
capabilities.has_touch = true;
capabilities.battery_level = 0.75;
capabilities.gpu_capable = true;
responsive.update_capabilities(capabilities);

// Get rendering configuration
let quality = responsive.get_rendering_quality();
let is_touch = responsive.is_touch_device();

// Setup touch input handling
if is_touch {
    let mut gestures = responsive.gestures.clone();
    input_system.on_touch = |touch_point| {
        gestures.touch_began(touch_point);
    };
}

// Configure layout grid
let grid = &responsive.grid;
let breakpoint = responsive.current_breakpoint();
let columns = grid.get_columns(&breakpoint);
let gap = grid.get_gap(&breakpoint);

// Setup typography
let typography = &responsive.typography;
let h1 = typography.calculate_size(&breakpoint, "h1");
let body = typography.calculate_size(&breakpoint, "body");

// Monitor for changes
responsive.on_resize(new_width, new_height);
responsive.on_orientation_change();

// Scale all dimensions via DPI system
let (scaled_w, scaled_h) = responsive.dpi_scaling.scale_dimensions(100, 100);
```

---

## Common Use Cases

### 1. Responsive Container

```nexus
pub struct ResponsiveContainer {
    breakpoint_styles: HashMap<BreakpointClass, ContainerStyle>,
    responsive: ResponsiveSystem,
}

impl ResponsiveContainer {
    pub fn apply_styles(&self) {
        let breakpoint = self.responsive.current_breakpoint();
        if let Some(style) = self.breakpoint_styles.get(&breakpoint) {
            // Apply style to container
        }
    }
}
```

### 2. Adaptive Image Gallery

```nexus
pub fn render_image_gallery(images: Vec<String>, responsive: &ResponsiveSystem) {
    let breakpoint = responsive.current_breakpoint();
    let columns = responsive.grid.get_columns(&breakpoint);
    
    // Render gallery with responsive columns
    for (i, image) in images.iter().enumerate() {
        let responsive_img = ResponsiveImage::new(image.clone());
        let best_source = responsive_img.select_source(&responsive.device);
        // Render image from best_source
    }
}
```

### 3. Gesture-Based Navigation

```nexus
pub fn handle_swipe_navigation(responsive: &mut ResponsiveSystem, direction: SwipeDirection) {
    match direction {
        SwipeDirection::Left => navigate_next(),
        SwipeDirection::Right => navigate_previous(),
        _ => {},
    }
}
```

---

## Testing & Validation

### Breakpoint Testing
```nexus
#[test]
fn test_breakpoint_classification() {
    let bp = BreakpointConfig::new();
    assert_eq!(bp.classify_width(400), BreakpointClass::Mobile);
    assert_eq!(bp.classify_width(800), BreakpointClass::Tablet);
    assert_eq!(bp.classify_width(1920), BreakpointClass::Desktop);
}
```

### Device Detection Testing
```nexus
#[test]
fn test_device_detection() {
    let device = DeviceDetector::new(1080, 2340, 420);
    assert_eq!(device.device_type, DeviceType::Phone);
    assert_eq!(device.orientation, Orientation::Portrait);
    assert!(device.device_pixel_ratio > 4.0);
}
```

### Gesture Recognition Testing
```nexus
#[test]
fn test_swipe_detection() {
    let mut recognizer = GestureRecognizer::new();
    // Simulate swipe gesture
    recognizer.touch_began(/* ... */);
    recognizer.touch_moved(0, 50.0, 200.0);
    let gesture = recognizer.touch_ended(0);
    assert!(matches!(gesture, Some(GestureType::Swipe { .. })));
}
```

---

## Conclusion

NEXUS provides a complete, production-ready responsive design system for Omnisystem that handles all aspects of adaptive UI—from breakpoints and device detection to touch gestures and DPI scaling. By integrating with VERA components, HELIX rendering, TITAN input, and SYLVA layouts, NEXUS enables truly responsive applications that deliver optimal experiences across all device types and screen sizes.

**Current Version:** 31.0.0  
**Status:** Production-ready  
**Maintenance:** Actively maintained  
**Last Updated:** 2026-06-24
