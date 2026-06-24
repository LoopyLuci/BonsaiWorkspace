# NEXUS Responsive Design - Quick Reference
## Essential Snippets & Common Patterns

**Version:** 31.0.0  
**Last Updated:** 2026-06-24

---

## One-Liner Initialization

```nexus
let responsive = ResponsiveSystem::new(screen_width, screen_height, dpi);
```

---

## Core Queries

### Get Current Breakpoint
```nexus
let bp = responsive.current_breakpoint();
// Returns: BreakpointClass::Mobile | Tablet | Desktop | UltraWide
```

### Get Device Type
```nexus
let device_type = responsive.device.device_type;
// Returns: DeviceType::Phone | Tablet | Laptop | Desktop | TV | Wearable
```

### Is Touch Device?
```nexus
if responsive.is_touch_device() {
    // Device has touch input
}
```

### Get Current Orientation
```nexus
match responsive.device.orientation {
    Orientation::Portrait => { /* */ },
    Orientation::Landscape => { /* */ },
    Orientation::Square => { /* */ },
}
```

---

## Responsive Grid

### Get Column Count
```nexus
let columns = responsive.grid.get_columns(&responsive.current_breakpoint());
// Mobile: 1, Tablet: 3, Desktop: 4, UltraWide: 6
```

### Get Spacing
```nexus
let gap = responsive.grid.get_gap(&responsive.current_breakpoint());
let padding = responsive.grid.get_padding(&responsive.current_breakpoint());
```

### Calculate Item Width
```nexus
let item_width = responsive.grid.calculate_item_width(
    container_width,
    &responsive.current_breakpoint()
);
```

---

## Typography

### Get Font Size for Heading
```nexus
let h1_size = responsive.get_heading_size(1);        // h1
let h2_size = responsive.get_heading_size(2);        // h2
```

### Get Font Size for Any Level
```nexus
let size = responsive.typography.calculate_size(
    &responsive.current_breakpoint(),
    "body"  // or "h1", "h2", "h3", "small", "caption"
);
```

### Get Line Height
```nexus
let line_height = responsive.typography.get_line_height(font_size);
```

---

## Touch & Gestures

### Track Touch Input
```nexus
responsive.gestures.touch_began(TouchPoint {
    id: 0,
    x: x_pos,
    y: y_pos,
    pressure: 0.8,
    radius: 12.0,
    timestamp: now,
});

responsive.gestures.touch_moved(0, new_x, new_y);

if let Some(gesture) = responsive.gestures.touch_ended(0) {
    match gesture {
        GestureType::SingleTap => { /* */ },
        GestureType::Swipe { direction, velocity } => { /* */ },
        GestureType::Pinch { scale, velocity } => { /* */ },
        _ => {},
    }
}
```

### Get Touch Target Size
```nexus
let min_size = responsive.touch_sizes.minimum_size;       // 44
let rec_size = responsive.touch_sizes.recommended_size;   // 48
let padding = responsive.touch_sizes.touch_padding;       // 12
```

### Check Touch Hit
```nexus
let hit = responsive.gestures.is_touch_in_bounds(
    touch_x, touch_y,
    btn_x, btn_y, btn_w, btn_h,
    responsive.touch_sizes.touch_padding
);
```

---

## DPI Scaling

### Get Device DPI
```nexus
let dpi = responsive.device.dpi;
let dpi_ratio = responsive.device.device_pixel_ratio;
```

### Scale Dimensions
```nexus
let logical_px = 16;
let physical_px = responsive.dpi_scaling.logical_to_physical(logical_px);

let (scaled_w, scaled_h) = responsive.dpi_scaling.scale_dimensions(100, 200);
```

### Get DPI Class
```nexus
let dpi_class = responsive.dpi_scaling.get_dpi_class();
// DPIClass::LowDPI | MediumDPI | HighDPI | UltraHighDPI | SuperHighDPI
```

---

## Responsive Images

### Select Best Image
```nexus
let image = ResponsiveImage::new("/img/default.jpg".to_string());
image.source_1x = "/img/photo.jpg".to_string();
image.source_2x = "/img/photo@2x.jpg".to_string();
image.source_3x = "/img/photo@3x.jpg".to_string();

let best_source = image.select_source(&responsive.device);
```

### Get Srcset
```nexus
let srcset = image.get_srcset();
// "photo.jpg 1x, photo@2x.jpg 2x, photo@3x.jpg 3x"
```

---

## Aspect Ratio

### Standard Ratios
```nexus
let square = AspectRatio::square();           // 1:1
let portrait = AspectRatio::portrait();       // 3:4
let landscape = AspectRatio::landscape();     // 16:9
let cinema = AspectRatio::cinema();           // 21:9
```

### Calculate Dimensions
```nexus
let aspect = AspectRatio::landscape();
let height = aspect.calculate_height(width);
let width = aspect.calculate_width(height);
let padding_pct = aspect.get_padding_bottom_percent();
```

---

## Safe Areas (Notches/Rounded Corners)

### Set Safe Area
```nexus
responsive.device.safe_area = SafeAreaInsets::from_notch(
    40,    // top inset
    50,    // bottom inset
    0,     // left inset
    0      // right inset
);
```

### Apply Safe Area to Rectangle
```nexus
let content_rect = Rectangle { x: 0, y: 0, width: 1080, height: 2340 };
let safe_rect = responsive.device.safe_area.apply_to_rect(&content_rect);
```

---

## Device Capabilities

### Check Capabilities
```nexus
if responsive.device.capabilities.has_touch {
    // Enable touch input
}

if responsive.device.capabilities.has_accelerometer {
    // Listen to accelerometer
}

if responsive.device.capabilities.gpu_capable {
    // Use GPU rendering
}
```

### Battery & Power
```nexus
if responsive.should_enable_battery_saver() {
    // Reduce rendering quality
    set_fps(30);
    disable_animations();
}

let battery = responsive.device.capabilities.battery_level;
if battery < 0.20 {
    // Critical battery
}
```

### Network
```nexus
if responsive.device.capabilities.is_low_bandwidth {
    // Use lower resolution assets
}
```

---

## Quality Levels

### Get Current Quality
```nexus
let quality = responsive.get_rendering_quality();

if quality.anti_aliasing {
    // Enable FXAA/MSAA
}

if quality.shadows {
    // Enable shadow rendering
}

if quality.blur_effects {
    // Enable post-processing
}

println!("Particles: {}", quality.particle_count);
```

### Quality by Device
```nexus
// Low:    No AA, no shadows, no blur, 0 particles
// Medium: FXAA, shadows, no blur, 50 particles
// High:   MSAA, shadows, blur, 200+ particles
```

---

## Viewport Configuration

### Mobile Viewport
```nexus
let mobile_vp = ViewportConfig::new_mobile();
// width=device-width, initial-scale=1.0
// viewport-fit=contain (avoid notch)
```

### Desktop Viewport
```nexus
let desktop_vp = ViewportConfig::new_desktop();
// No explicit width, scalable 0.5x - 2.0x
// viewport-fit=auto
```

### Get Meta Tag
```nexus
let meta_tag = responsive.viewport.get_meta_tag();
// For HTML <meta> tag
```

---

## Handle Device Changes

### Window Resize
```nexus
responsive.on_resize(new_width, new_height);
// Automatically updates breakpoint, recalculates layout
```

### Orientation Change
```nexus
responsive.on_orientation_change();
// Updates orientation, may trigger breakpoint change
```

### Update Capabilities
```nexus
let mut caps = DeviceCapabilities::default();
caps.battery_level = 0.25;
caps.is_thermal_throttling = true;
responsive.update_capabilities(caps);
// Updates quality level
```

---

## Media Queries

### Create Media Query
```nexus
let mut query = MediaQuery::new();
query.min_width = Some(768);
query.max_width = Some(1024);
query.orientation = Some(Orientation::Portrait);

if query.matches(&responsive.device) {
    // Applies to tablets in portrait
}
```

### Container Queries
```nexus
let mut cq = ContainerQuery::new();
cq.min_container_width = Some(300);
cq.max_container_width = Some(600);

if cq.matches(container_width, container_height) {
    // Component layout applies
}
```

---

## Debug & Info

### Print Responsive State
```nexus
println!("Screen: {}x{}", 
    responsive.device.screen_width, 
    responsive.device.screen_height);
println!("DPI: {} ({:?})", 
    responsive.device.dpi, 
    responsive.dpi_scaling.get_dpi_class());
println!("Device: {:?}", responsive.device.device_type);
println!("Breakpoint: {:?}", responsive.current_breakpoint());
println!("Quality: {:?}", responsive.quality_level);
```

---

## Common Patterns

### Conditional Component Display
```nexus
let should_show_desktop_menu = matches!(
    responsive.current_breakpoint(),
    BreakpointClass::Desktop | BreakpointClass::UltraWide
);
```

### Adaptive Spacing
```nexus
let spacing = responsive.grid.get_gap(&responsive.current_breakpoint());
// Auto-adapt spacing based on device
```

### Mobile-First Rendering
```nexus
match responsive.device.device_type {
    DeviceType::Phone | DeviceType::Tablet => {
        render_mobile_layout();
    },
    _ => {
        render_desktop_layout();
    }
}
```

### Performance-Based Rendering
```nexus
if responsive.device.capabilities.available_memory_mb < 512 {
    reduce_texture_resolution();
}

if responsive.device.dpi > 300 {
    enable_high_quality_rendering();
}
```

---

## Breakpoint Reference Table

| Breakpoint | Range | Use Case | Columns |
|---|---|---|---|
| **XS** | < 480px | Smartwatch | 1 |
| **SM** | 480-576px | Small phone | 1-2 |
| **MD** | 576-768px | Large phone | 2-3 |
| **LG** | 768-992px | Tablet | 3-4 |
| **XL** | 992-1200px | Desktop | 4-6 |
| **XXL** | 1200-2560px | Large desktop | 6-8 |
| **UHD** | 2560-7680px | 4K | 8+ |
| **8K** | > 7680px | 8K | 12+ |

---

## Device Type Reference

| Type | Diagonal | DPI | Touch | Common Sizes |
|---|---|---|---|---|
| Wearable | ~1.5" | 300+ | ✓ | 280x360 |
| Phone | 5-7" | 200-600 | ✓ | 1080x2340 |
| Tablet | 8-12" | 160-300 | ✓ | 2560x1600 |
| Laptop | 13-15" | 90-160 | Mixed | 1920x1080 |
| Desktop | 24-32" | 80-110 | ✗ | 2560x1440 |
| TV | 40-85" | 25-35 | ✗ | 3840x2160 |

---

## DPI Class Reference

| Class | Range | Typical Devices |
|---|---|---|
| **LowDPI** | < 96 | Old monitors |
| **MediumDPI** | 96-150 | Standard desktop |
| **HighDPI** | 150-220 | Tablets, some phones |
| **UltraHighDPI** | 220-330 | Modern phones |
| **SuperHighDPI** | > 330 | Premium phones |

---

## Integration Checklist

- [ ] Initialize ResponsiveSystem on app start
- [ ] Handle on_resize events
- [ ] Handle on_orientation_change events
- [ ] Update capabilities when they change
- [ ] Use responsive.current_breakpoint() for layout decisions
- [ ] Apply gesture recognizer to touch input
- [ ] Check device.capabilities for features
- [ ] Scale rendering quality based on quality_level
- [ ] Apply safe_area insets for notch-aware layout
- [ ] Test on multiple device sizes/orientations

---

## Related Files

- **Full Module:** `NexusResponsiveDesign.nexus`
- **Design Guide:** `NEXUS_RESPONSIVE_DESIGN_GUIDE.md`
- **Integration Guide:** `NEXUS_INTEGRATION_GUIDE.md`

---

**Need more details? Check the full NEXUS Responsive Design Guide!**
