// Example: Simple scene composition

use bmn_compositor::{
    Scene, SceneElement, SceneElementType, Compositor, CompositorConfig,
    Transform, BlendMode, Effect, EffectType,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎬 Simple Composition Example");

    // Create compositor
    let mut compositor = Compositor::new(CompositorConfig::new(1920, 1080));
    compositor.initialize().await?;
    println!("✓ Compositor initialized");

    // Create scene
    let mut scene = Scene::new("Main", 1920, 1080, 60);

    // Add display source
    let display = SceneElement::new("Display", SceneElementType::DisplayCapture)
        .with_position(0.0, 0.0)
        .with_size(1920, 1080);

    let display_id = display.id;
    scene.add_element(display);

    // Add text overlay
    let mut text = SceneElement::new("Title", SceneElementType::Text)
        .with_position(100.0, 100.0)
        .with_opacity(0.8);

    text.properties.insert("text".into(), "Stream Title".into());
    text.properties.insert("font_size".into(), "48".into());

    scene.add_element(text);

    println!("✓ Scene created with {} elements", scene.element_count());

    // Get scene graph
    let mut graph = compositor.scene_graph().write().await;
    let scene_id = scene.id;
    graph.add_scene(scene);
    drop(graph);

    // Render a frame
    let frame = compositor.render().await?;
    println!(
        "✓ Rendered frame: {}x{} ({} bytes)",
        frame.width,
        frame.height,
        frame.data.len()
    );

    let stats = compositor.get_stats().await;
    println!("✓ Composition stats: {} frames rendered", stats.frames_rendered);

    compositor.shutdown().await?;
    println!("✓ Compositor shutdown");

    Ok(())
}
