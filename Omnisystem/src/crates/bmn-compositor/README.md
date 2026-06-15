# BMN Compositor — Real-Time GPU Composition Engine

## Overview

**BMN Compositor** provides real-time, GPU-accelerated scene composition for Bonsai Media Nexus. It manages complex scenes with multiple sources, effects, and transformations while maintaining <16.7ms latency @ 60fps.

## Architecture

### Core Components

```
┌─────────────────────────────────────────────────┐
│         Scene Graph (CPU-side)                  │
├─────────────────────────────────────────────────┤
│ ┌─────────────────────────────────────────────┐ │
│ │ Scene: "Main Stream"                        │ │
│ │  ├─ Element: Display Capture (1920x1080)   │ │
│ │  ├─ Element: Webcam (480x270, PiP)         │ │
│ │  ├─ Element: Title Bar (1920x60)           │ │
│ │  ├─ Element: Chat Overlay (300x800)        │ │
│ │  └─ Effects: Fade, Color Grade              │ │
│ └─────────────────────────────────────────────┘ │
│                    ↓                             │
│   ┌──────────────────────────────────────────┐  │
│   │ Compositor (GPU Pipeline)                │  │
│   │  • Vulkan render passes                  │  │
│   │  • Texture layout transitions            │  │
│   │  • Synchronization                       │  │
│   └──────────────────────────────────────────┘  │
│                    ↓                             │
│         VideoFrame (encoded frame)              │
└─────────────────────────────────────────────────┘
```

## Scene Graph

### Scene Structure

A **Scene** contains multiple **SceneElements**, each representing a source or overlay:

```rust
let mut scene = Scene::new("Main", 1920, 1080, 60);

// Add display source
scene.add_element(
    SceneElement::new("Display", SceneElementType::DisplayCapture)
        .with_position(0.0, 0.0)
        .with_size(1920, 1080)
);

// Add webcam PiP
scene.add_element(
    SceneElement::new("Webcam", SceneElementType::CameraCapture)
        .with_position(1400.0, 800.0)
        .with_size(480.0, 270.0)
        .with_opacity(0.9)
);
```

### Scene Elements

| Type | Purpose | Input |
|---|---|---|
| `DisplayCapture` | Screen capture | Video feed |
| `CameraCapture` | Webcam | Video feed |
| `VideoFile` | Pre-recorded video | File path |
| `BrowserSource` | Web content | URL |
| `Image` | Static image | File path |
| `ColorRect` | Solid color | RGB + alpha |
| `Text` | Dynamic text | String + font |
| `Group` | Nested elements | Sub-elements |

## Transforms

Apply **position, scale, rotation** with anchor points:

```rust
SceneElement::new("Source", SceneElementType::CameraCapture)
    .with_position(100.0, 50.0)       // Top-left position
    .with_scale(0.5, 0.5)              // 50% size
    .with_rotation(45.0)               // 45 degree rotation
    .with_anchor(0.5, 0.5)             // Center anchor
```

**Transforms** are computed on CPU, passed to GPU:

```rust
let matrix = transform.matrix(width, height);  // Mat4
// Vertex shader: gl_Position = matrix * position;
```

## Blending Modes

Compose elements with different blend operations:

```rust
SceneElement::new("Overlay", SceneElementType::Image)
    .with_blend_mode(BlendMode::Screen)  // Lighten blend
```

| Mode | Effect |
|---|---|
| `Alpha` | Standard alpha compositing |
| `Additive` | Add colors (brighten) |
| `Multiply` | Multiply colors (darken) |
| `Screen` | Inverse multiply (lighten) |
| `Overlay` | Mix of multiply and screen |
| `SoftLight` | Subtle soft light |
| `ColorDodge` | Brighten highlights |
| `ColorBurn` | Darken shadows |
| `Difference` | Absolute color difference |
| `Hue` / `Saturation` / `Color` / `Luminosity` | Adjust color space |

## Effects

Apply real-time effects to elements or scenes:

```rust
// Color effects
Effect::new("Brightness", EffectType::Brightness)
    .with_intensity(1.2)

// Blur
Effect::new("Background Blur", EffectType::Blur)
    .with_parameter("radius", 10.0)

// Transitions
Effect::new("Fade In", EffectType::Fade)
    .with_duration(1000)  // 1 second

// Artistic
Effect::new("Sepia", EffectType::Sepia)
    .with_intensity(0.5)
```

Available effects:
- **Color**: Brightness, Contrast, Saturation, Hue, Gamma, ColorBalance
- **Distortion**: Blur, Sharpen, Vignette, Distortion, Perspective
- **Artistic**: Pixelate, Posterize, Sepia
- **Transitions**: Fade, Slide, Wipe, Dissolve, CrossFade
- **Custom**: User-defined GLSL shaders

## Compositor

Main GPU rendering engine:

```rust
// Create compositor
let mut compositor = Compositor::new(
    CompositorConfig::new(1920, 1080)
        .with_hdr()  // Enable HDR P010
        .with_gpu(true)
);

// Initialize GPU resources
compositor.initialize().await?;

// Add scenes
let graph = compositor.scene_graph().write().await;
graph.add_scene(scene);
drop(graph);

// Render frame
let frame = compositor.render().await?;

// Shutdown
compositor.shutdown().await?;
```

## Performance

| Target | Performance |
|---|---|
| Composition latency | <1 frame (16.7ms @ 60fps) |
| GPU memory per scene | <500MB |
| Maximum layers | 100+ |
| Maximum resolution | 4K60 (configurable) |
| HDR support | Yes (P010 10-bit) |

## GPU Implementation (Vulkan)

### Render Pipeline

```
Input: Scene Graph (CPU-side)
  │
  ├─ Render Pass 1: Blit sources to textures
  ├─ Render Pass 2: Apply effects (blur, color grade)
  ├─ Render Pass 3: Composite layers
  ├─ Render Pass 4: Post-processing
  │
  └─ Output: Video frame (GPU memory)
```

### Texture Management

- **Staging textures**: Input from sources
- **Working textures**: Intermediate processing
- **Output texture**: Final composited frame
- **Metadata buffers**: Transforms, opacity, blend modes

### Synchronization

- **Frame synchronization**: Double/triple buffering
- **Pipeline stages**: Semaphore synchronization
- **Memory barriers**: Correct cache coherency
- **Presentation**: GPU→CPU readback or direct streaming

## AI Auto-Composition

Integrate **BonsAI V2** for intelligent scene management:

```
┌─────────────────────────────────┐
│ AI Scene Analysis               │
│  • Detect speakers              │
│  • Identify action regions      │
│  • Suggest layouts              │
│  • Auto-transition scenes       │
└─────────────────────────────────┘
        ↓
┌─────────────────────────────────┐
│ Auto-Composer                   │
│  • Adjust source positions      │
│  • Apply focus effects          │
│  • Optimize framing             │
│  • Suggest crops/zoom           │
└─────────────────────────────────┘
```

Example: **Speaker Detection**
- AI detects which person is speaking
- Auto-center speaker in frame
- Blur/desaturate other sources
- Smooth transitions between speakers

## Scripting (Lua + Sylva)

Automate complex compositions:

```lua
-- Bonsai Media Nexus Composition Script

-- Define scenes
scene_main = Scene.new("Main", 1920, 1080, 60)
scene_breakdown = Scene.new("Breakdown", 1920, 1080, 60)

-- Define transitions
transition(scene_main, scene_breakdown, {
    effect = "slide",
    duration = 500,
    direction = "left"
})

-- Auto-switch on schedule
on_timer(300000, function() -- 5 minutes
    switch_scene(scene_breakdown)
    add_effect("fade", 500)
end)

-- On event
on_event("chat", function(event)
    highlight_chat_overlay()
end)
```

## NDI Output

Stream to other software as a virtual camera:

```rust
compositor.enable_ndi_output("Bonsai NDI", true)?;
// Now visible in OBS, Wirecast, vMix, etc.
```

## Integration with BMN Pipeline

Compositor is the **bridge** between capture and encoding:

```
Sources (display, camera, audio)
         ↓
      [Compositor]  ← Scene management, effects, transforms
         ↓
   AI Enhancement (BonsAI V2)
         ↓
   Encoder Pool (NVENC, x264, etc.)
         ↓
   Transport (RTMP, SRT, Echo P2P)
```

## Testing

```bash
# Run all compositor tests
cargo test --package bmn-compositor

# Run specific test
cargo test --package bmn-compositor scene_graph

# Run with backtrace
RUST_BACKTRACE=1 cargo test --package bmn-compositor

# Run examples
cargo run --example simple_composition --package bmn-compositor
cargo run --example multi_source --package bmn-compositor
```

## Next Steps

1. **Vulkan backend** — Full GPU rendering with synchronization
2. **Shader library** — GLSL shaders for all effects
3. **AI integration** — BonsAI V2 scene analysis
4. **Scripting** — Lua-based composition automation
5. **Performance** — Optimize render passes and memory

---

**Status:** Phase 2 — Scene graph and CPU-side logic complete, GPU backend in progress. 🚀
