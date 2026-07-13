// Example: Multi-source composition with effects

use bmn_compositor::{
    Scene, SceneElement, SceneElementType, Compositor, CompositorConfig,
    BlendMode, Effect, EffectType,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎨 Multi-Source Composition Example");

    // Create compositor with HDR support
    let mut compositor = Compositor::new(
        CompositorConfig::new(1920, 1080)
            .with_hdr()
    );
    compositor.initialize().await?;
    println!("✓ Compositor initialized with HDR");

    // Create main scene
    let mut scene = Scene::new("Gameplay", 1920, 1080, 60);

    // Background layer
    let bg = SceneElement::new("Game Window", SceneElementType::DisplayCapture)
        .with_position(0.0, 0.0)
        .with_size(1920, 1080);
    scene.add_element(bg);

    // Camera overlay (picture-in-picture)
    let mut camera = SceneElement::new("Webcam", SceneElementType::CameraCapture)
        .with_position(1400.0, 800.0)
        .with_size(480.0, 270.0)
        .with_opacity(0.95)
        .with_blend_mode(BlendMode::Alpha);

    // Add blur effect to webcam
    camera = camera.add_effect(
        Effect::new("Background Blur", EffectType::Blur)
            .with_intensity(0.3)
            .with_parameter("radius", 5.0)
    );

    scene.add_element(camera);

    // Title bar
    let mut title = SceneElement::new("Title", SceneElementType::ColorRect)
        .with_position(0.0, 0.0)
        .with_size(1920, 60.0)
        .with_opacity(0.7)
        .with_blend_mode(BlendMode::Multiply);

    title.properties.insert("color".into(), "#000000".into());
    scene.add_element(title);

    // Text overlay
    let mut text = SceneElement::new("Game Title", SceneElementType::Text)
        .with_position(30.0, 15.0)
        .with_opacity(1.0);

    text.properties.insert("text".into(), "Bonsai Stream".into());
    text.properties.insert("font_size".into(), "36".into());
    text.properties.insert("color".into(), "#FFFFFF".into());

    scene.add_element(text);

    // Add transition effect
    let scene = scene.add_effect(
        Effect::new("Fade In", EffectType::Fade)
            .with_intensity(1.0)
            .with_duration(1000)
    );

    println!("✓ Scene created with {} elements", scene.elements.len());

    // Add to compositor
    let mut graph = compositor.scene_graph().write().await;
    graph.add_scene(scene);
    drop(graph);

    // Render multiple frames
    for i in 0..10 {
        match compositor.render().await {
            Ok(frame) => {
                println!(
                    "Frame {}: {}x{} ({})",
                    i,
                    frame.width,
                    frame.height,
                    match frame.format {
                        bmn_common::frame::PixelFormat::P010 => "HDR P010",
                        bmn_common::frame::PixelFormat::RGBA => "RGBA",
                        _ => "other",
                    }
                );
            }
            Err(e) => eprintln!("Render error: {}", e),
        }
    }

    let stats = compositor.get_stats().await;
    println!(
        "✓ Composition complete: {} frames rendered",
        stats.frames_rendered
    );

    compositor.shutdown().await?;
    println!("✓ Compositor shutdown");

    Ok(())
}
