# Graphics Framework Guide - 2D/3D Rendering Engine

**Enterprise-grade rendering framework for games, design tools, and simulations**

---

## Overview

The Graphics Framework provides:
- **Vulkan/Metal abstraction** - Cross-platform GPU rendering
- **2D Rendering** - Sprites, text, vector graphics
- **3D Rendering** - Meshes, materials, lighting
- **Shader System** - SPIR-V/Metal Shading Language support
- **Real-time Performance** - 60+ FPS rendering
- **Viewport Management** - Cameras, layers, transforms

---

## Core Architecture

```
Application Layer
    ↓
Graphics API (Vulkan/Metal)
    ↓
Device Management (GPU/CPU)
    ↓
Command Buffers & Queues
    ↓
Swapchain & Presentation
```

---

## Quick Start

```titan
use omnisystem::graphics::*

fun main() -> Result<(), str> {
    // Create window and context
    let window = Window::new("My App", 1920, 1080)?
    let graphics = GraphicsContext::new(&window)?
    
    // Create renderpass
    let renderpass = RenderPass::new()
        .with_color_attachment(Format::RGBA8)?
        .with_depth_attachment(Format::D32)?
    
    // Create pipeline
    let pipeline = GraphicsPipeline::new(&renderpass)
        .with_vertex_shader("shaders/vert.spv")?
        .with_fragment_shader("shaders/frag.spv")?
        .build()?
    
    // Main loop
    while window.is_open() {
        let mut cmd = graphics.record_commands()?
        
        cmd.begin_renderpass(&renderpass)?
        cmd.bind_pipeline(&pipeline)?
        cmd.draw(3, 1)?  // 3 vertices, 1 instance
        cmd.end_renderpass()?
        
        graphics.submit(cmd)?
        graphics.present()?
    }
    
    Ok(())
}
```

---

## Rendering Pipeline

### Vertex & Fragment Shaders

```glsl
// vertex.glsl
#version 450

layout(binding = 0) uniform UniformBuffer {
    mat4 proj;
    mat4 view;
    mat4 model;
} ubo;

layout(location = 0) in vec3 position;
layout(location = 1) in vec3 normal;
layout(location = 2) in vec2 texCoord;

layout(location = 0) out vec3 outNormal;
layout(location = 1) out vec2 outTexCoord;

void main() {
    gl_Position = ubo.proj * ubo.view * ubo.model * vec4(position, 1.0);
    outNormal = mat3(ubo.model) * normal;
    outTexCoord = texCoord;
}
```

```glsl
// fragment.glsl
#version 450

layout(binding = 1) uniform sampler2D texSampler;

layout(location = 0) in vec3 inNormal;
layout(location = 1) in vec2 inTexCoord;

layout(location = 0) out vec4 outColor;

void main() {
    vec3 color = texture(texSampler, inTexCoord).rgb;
    float brightness = dot(inNormal, normalize(vec3(1, 1, 1)));
    outColor = vec4(color * brightness, 1.0);
}
```

---

## 2D Rendering

### Sprite Rendering

```titan
fun render_sprites(graphics: &mut GraphicsContext) -> Result<(), str> {
    // Load sprite
    let texture = Texture::load("sprite.png")?
    let sprite = Sprite::new(texture)
        .with_position(100.0, 100.0)
        .with_scale(2.0, 2.0)
        .with_rotation(45.0)
    
    // Render batched
    let mut batch = SpriteBatch::new()
    batch.draw(&sprite)?
    batch.draw(&sprite.with_position(200.0, 200.0))?
    
    graphics.draw_batch(&batch)?
    Ok(())
}
```

### Text Rendering

```titan
fun render_text(graphics: &mut GraphicsContext) -> Result<(), str> {
    let font = Font::load("arial.ttf", size: 32)?
    
    let text = Text::new("Hello, World!")
        .with_font(&font)
        .with_color(Color::White)
        .with_position(50.0, 50.0)
    
    graphics.draw_text(&text)?
    Ok(())
}
```

### Vector Graphics

```titan
fun render_shapes(graphics: &mut GraphicsContext) -> Result<(), str> {
    let circle = Shape::circle(center: (100.0, 100.0), radius: 50.0)
        .with_fill(Color::Blue)
        .with_stroke(Color::Black, width: 2.0)
    
    let rect = Shape::rect(x: 200.0, y: 200.0, width: 100.0, height: 50.0)
        .with_fill(Color::Red)
    
    let line = Shape::line(from: (0.0, 0.0), to: (500.0, 500.0))
        .with_stroke(Color::Green, width: 3.0)
    
    graphics.draw_shape(&circle)?
    graphics.draw_shape(&rect)?
    graphics.draw_shape(&line)?
    
    Ok(())
}
```

---

## 3D Rendering

### Mesh Loading

```titan
fun load_3d_scene() -> Result<Scene, str> {
    let mesh = Mesh::load("models/character.gltf")?
    
    let material = Material::new()
        .with_albedo_map("textures/albedo.png")?
        .with_normal_map("textures/normal.png")?
        .with_roughness(0.5)
        .with_metallic(0.2)
    
    let model = Model::new(mesh)
        .with_material(material)
        .with_position(0.0, 0.0, 0.0)
        .with_scale(1.0, 1.0, 1.0)
    
    let mut scene = Scene::new()
    scene.add_model("character", model)?
    
    Ok(scene)
}
```

### Lighting

```titan
fun setup_lighting(scene: &mut Scene) -> Result<(), str> {
    // Directional light
    let sun = DirectionalLight::new()
        .with_direction(vec3(1.0, 1.0, 1.0))
        .with_color(Color::White)
        .with_intensity(1.5)
        .with_shadow_map(2048)
    
    scene.add_light("sun", sun)?
    
    // Point light
    let lamp = PointLight::new()
        .with_position(5.0, 5.0, 5.0)
        .with_color(Color::Yellow)
        .with_intensity(2.0)
        .with_radius(20.0)
    
    scene.add_light("lamp", lamp)?
    
    // Spot light
    let spotlight = SpotLight::new()
        .with_position(10.0, 10.0, 10.0)
        .with_direction(vec3(0.0, -1.0, 0.0))
        .with_color(Color::White)
        .with_intensity(3.0)
        .with_inner_angle(20.0)
        .with_outer_angle(45.0)
    
    scene.add_light("spotlight", spotlight)?
    
    Ok(())
}
```

### Camera & Transform

```titan
fun setup_camera(scene: &mut Scene) -> Result<(), str> {
    let camera = Camera::perspective(
        fov: 45.0,
        aspect: 1920.0 / 1080.0,
        near: 0.1,
        far: 1000.0
    )
    .with_position(0.0, 2.0, 5.0)
    .look_at(0.0, 1.0, 0.0)
    
    scene.set_camera(camera)?
    
    Ok(())
}
```

---

## Advanced Features

### Post-Processing

```titan
fun apply_post_effects(graphics: &mut GraphicsContext) -> Result<(), str> {
    let mut effects = PostProcessing::new()
    
    // Bloom effect
    effects.add(PostEffect::Bloom {
        threshold: 0.8,
        intensity: 1.5,
        blur_radius: 10.0,
    })?
    
    // Tone mapping
    effects.add(PostEffect::ToneMap {
        exposure: 1.0,
        gamma: 2.2,
    })?
    
    // Color grading
    effects.add(PostEffect::ColorGrade {
        lookup_table: "lut.png",
    })?
    
    graphics.apply_post_processing(&effects)?
    Ok(())
}
```

### Particle Systems

```titan
fun create_particle_system() -> Result<ParticleSystem, str> {
    let particles = ParticleSystem::new()
        .with_max_particles(10000)
        .with_emission_rate(100.0)
        .with_lifetime(2.0)
        .with_velocity_range(
            min: vec3(-1.0, -1.0, -1.0),
            max: vec3(1.0, 5.0, 1.0)
        )
        .with_size_range(min: 0.1, max: 0.5)
        .with_color_gradient(&[
            (0.0, Color::White),
            (0.5, Color::Yellow),
            (1.0, Color::Transparent),
        ])?
    
    Ok(particles)
}
```

### Instanced Rendering

```titan
fun render_many_objects(graphics: &mut GraphicsContext) -> Result<(), str> {
    let mesh = Mesh::load("cube.gltf")?
    
    // Prepare instance data (transforms)
    let mut transforms = vec![]
    for x in 0..10 {
        for y in 0..10 {
            for z in 0..10 {
                transforms.push(Matrix4::translate(
                    x as f32 * 2.0,
                    y as f32 * 2.0,
                    z as f32 * 2.0
                ))
            }
        }
    }
    
    // Render with instancing
    graphics.draw_instanced(&mesh, &transforms)?
    
    Ok(())
}
```

---

## Performance Optimization

### GPU Memory Management

```titan
fun manage_memory(graphics: &GraphicsContext) -> Result<(), str> {
    // Query available memory
    let memory = graphics.query_memory_info()?
    println!("Available: {} MB", memory.available / 1024 / 1024)
    println!("Used: {} MB", memory.used / 1024 / 1024)
    
    // Allocate memory pool
    let pool = MemoryPool::new(size: 1024 * 1024 * 512)?  // 512 MB
    
    // Allocate resources from pool
    let buffer = pool.allocate_buffer(size: 1024 * 1024)?
    let texture = pool.allocate_texture(width: 2048, height: 2048)?
    
    Ok(())
}
```

### Culling & LOD

```titan
fun optimize_rendering(scene: &mut Scene) -> Result<(), str> {
    // Frustum culling
    scene.enable_frustum_culling(true)
    
    // Occlusion culling
    scene.enable_occlusion_culling(true)
    
    // Level of Detail
    let mesh_high = Mesh::load("model_high.gltf")?
    let mesh_medium = Mesh::load("model_medium.gltf")?
    let mesh_low = Mesh::load("model_low.gltf")?
    
    let lod = LODModel::new()
        .add_level(mesh_high, distance: 0.0)
        .add_level(mesh_medium, distance: 100.0)
        .add_level(mesh_low, distance: 500.0)
    
    scene.add_lod_model("character", lod)?
    
    Ok(())
}
```

---

## Viewport Management

### Layers & Sorting

```titan
fun setup_layers(graphics: &mut GraphicsContext) -> Result<(), str> {
    // Create render layers
    let background = RenderLayer::new("background", order: 0)?
    let gameplay = RenderLayer::new("gameplay", order: 1)?
    let ui = RenderLayer::new("ui", order: 2)?
    
    graphics.add_layer(&background)?
    graphics.add_layer(&gameplay)?
    graphics.add_layer(&ui)?
    
    Ok(())
}
```

### Multi-viewport Rendering

```titan
fun multi_viewport() -> Result<(), str> {
    let mut viewport1 = Viewport::new(x: 0, y: 0, width: 960, height: 1080)
    let mut viewport2 = Viewport::new(x: 960, y: 0, width: 960, height: 1080)
    
    // Render different cameras to different viewports
    // viewport1 → main camera
    // viewport2 → debug camera
    
    Ok(())
}
```

---

## Advanced Rendering Techniques

### Deferred Rendering

```titan
fun deferred_render(graphics: &mut GraphicsContext) -> Result<(), str> {
    // G-Buffer pass
    let g_buffer = graphics.create_framebuffer()
        .with_attachment("position", Format::RGBA32F)?
        .with_attachment("normal", Format::RGBA32F)?
        .with_attachment("albedo", Format::RGBA8)?
        .build()?
    
    // Lighting pass uses G-Buffer data
    graphics.deferred_lighting(&g_buffer)?
    
    Ok(())
}
```

### Path Tracing

```titan
fun render_with_path_tracing(graphics: &mut GraphicsContext) -> Result<(), str> {
    let tracer = PathTracer::new()
        .with_samples_per_pixel(256)
        .with_max_bounces(5)
        .with_denoise(true)
    
    graphics.render_path_traced(&tracer)?
    
    Ok(())
}
```

---

## Input Handling

```titan
fun handle_input(window: &Window) -> Result<(), str> {
    if window.key_pressed(Key::W) {
        // Move forward
    }
    
    if window.mouse_moved() {
        let (x, y) = window.mouse_position()
        // Update camera rotation
    }
    
    if window.mouse_button_pressed(MouseButton::Left) {
        // Handle click
    }
    
    Ok(())
}
```

---

## Best Practices

✅ **DO**
- Use GPU memory pools
- Batch draw calls
- Implement frustum culling
- Use level-of-detail
- Pre-compile shaders

❌ **DON'T**
- Change render targets per object
- Upload data every frame
- Render off-screen unnecessarily
- Use immediate-mode rendering
- Debug on GPU-limited platforms

---

## Next Steps

- [AUDIO_FRAMEWORK_GUIDE.md](AUDIO_FRAMEWORK_GUIDE.md) - Audio processing
- [PHYSICS_FRAMEWORK_GUIDE.md](PHYSICS_FRAMEWORK_GUIDE.md) - Physics simulation
- [GAME_FRAMEWORK_GUIDE.md](GAME_FRAMEWORK_GUIDE.md) - Game development

---

**Graphics Framework** - High-performance rendering for games and creative tools!
