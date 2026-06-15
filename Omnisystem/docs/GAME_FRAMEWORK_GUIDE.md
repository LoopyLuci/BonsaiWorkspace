# Game Framework Guide - Complete Game Development Engine

**Full-featured game engine with graphics, physics, audio, and entity systems**

---

## Overview

The Game Framework provides:
- **Entity-Component System (ECS)** - Data-driven architecture
- **Scene Management** - Hierarchical scenes and prefabs
- **Input Handling** - Keyboard, mouse, gamepad, touch
- **Asset Management** - Async loading, streaming, memory management
- **Game Loop** - Fixed timestep, frame pacing
- **Networking** - Multiplayer, replication, synchronization
- **Debugging Tools** - Profiling, inspection, visualization

---

## Core Architecture

```
Game Engine
    ├─ Graphics Context
    ├─ Physics World
    ├─ Audio Engine
    ├─ ECS World (entities, components, systems)
    ├─ Scene Graph
    ├─ Asset Manager
    ├─ Input System
    └─ Network Manager
```

---

## Quick Start

```titan
use omnisystem::game::*

fun main() -> Result<(), str> {
    // Initialize game engine
    let mut engine = GameEngine::new(
        title: "My Game",
        width: 1920,
        height: 1080,
        fullscreen: false
    )?
    
    // Load scene
    let scene = Scene::load("scenes/level1.scene")?
    engine.load_scene(scene)?
    
    // Main game loop
    while engine.is_running()? {
        engine.update()?
        engine.render()?
        engine.input_sync()?
    }
    
    Ok(())
}
```

---

## Entity-Component System (ECS)

### Creating Entities

```titan
fun create_player(world: &mut World) -> Result<Entity, str> {
    let entity = world.create_entity()?
    
    // Add components
    entity.add_component(Transform {
        position: vec3(0.0, 0.0, 0.0),
        rotation: vec3(0.0, 0.0, 0.0),
        scale: vec3(1.0, 1.0, 1.0),
    })?
    
    entity.add_component(Mesh {
        model: "player.gltf",
        material: "player_material",
    })?
    
    entity.add_component(RigidBody {
        shape: RigidBodyShape::Capsule { radius: 0.5, height: 2.0 },
        mass: 80.0,
        restitution: 0.0,
    })?
    
    entity.add_component(PlayerController {
        move_speed: 5.0,
        jump_force: 10.0,
        health: 100,
    })?
    
    entity.add_component(Animator {
        animation_tree: "player_animations",
        current_state: "idle",
    })?
    
    Ok(entity)
}
```

### Component Types

```titan
type Transform {
    position: Vec3,
    rotation: Vec3,
    scale: Vec3,
}

type Mesh {
    model: string,
    material: string,
    visible: bool,
}

type ParticleEmitter {
    system: string,
    enabled: bool,
    lifetime: f32,
}

type Audio {
    clip: string,
    volume: f32,
    loop: bool,
    playing: bool,
}

type Collider {
    enabled: bool,
    is_trigger: bool,
    layer: i32,
}

type Animator {
    animation_tree: string,
    current_state: string,
    blend_trees: Map<string, f32>,
}

type Script {
    script_path: string,
    state: Map<string, Value>,
}
```

---

## Systems (Game Logic)

### Creating Systems

```titan
fun create_player_movement_system() -> Result<System, str> {
    let system = System::new("PlayerMovement")
        .with_query::<(Transform, Velocity, PlayerController)>()
    
    system.on_update(|entities: Vec<(Entity, Transform, Velocity, PlayerController)>| {
        for (entity, mut transform, mut velocity, controller) in entities {
            // Get input
            let input = get_input()
            
            // Update velocity based on input
            if input.is_pressed(Key::W) {
                velocity.x += controller.move_speed
            }
            if input.is_pressed(Key::Space) {
                velocity.y = controller.jump_force
            }
            
            // Apply gravity
            velocity.y -= 9.81
            
            // Update transform
            transform.position += velocity * delta_time
            
            entity.set_component(transform)?
            entity.set_component(velocity)?
        }
    })?
    
    Ok(system)
}
```

### Built-in Systems

```titan
fun setup_game_systems(engine: &mut GameEngine) -> Result<(), str> {
    // Physics
    engine.add_system(PhysicsSystem::new())?
    
    // Animation
    engine.add_system(AnimationSystem::new())?
    
    // Particle updates
    engine.add_system(ParticleSystem::new())?
    
    // Audio updates
    engine.add_system(AudioSystem::new())?
    
    // Rendering
    engine.add_system(RenderSystem::new())?
    
    Ok(())
}
```

---

## Scene Management

### Scene Graph

```titan
fun create_scene_hierarchy() -> Result<Scene, str> {
    let mut scene = Scene::new("Level1")?
    
    // Root
    let root = scene.create_entity()?
    
    // Player
    let player = scene.create_entity()?
    player.set_parent(root)?
    player.add_component(create_player_mesh()?)?
    player.add_component(PlayerController { ... })?
    
    // Camera (child of player)
    let camera = scene.create_entity()?
    camera.set_parent(player)?
    camera.add_component(Camera::perspective(...))?
    camera.set_local_position(vec3(0.0, 0.6, 0.0))?
    
    // Environment
    let environment = scene.create_entity()?
    environment.set_parent(root)?
    
    // Terrain
    let terrain = scene.create_entity()?
    terrain.set_parent(environment)?
    terrain.add_component(create_terrain_mesh()?)?
    
    Ok(scene)
}
```

### Prefabs

```titan
fun create_prefab() -> Result<(), str> {
    // Define reusable entity template
    let prefab = Prefab::new("enemy")
    
    prefab.add_component(Mesh {
        model: "enemy.gltf",
        material: "enemy_material",
    })?
    
    prefab.add_component(RigidBody {
        shape: RigidBodyShape::Capsule { radius: 0.3, height: 1.5 },
        mass: 50.0,
    })?
    
    prefab.add_component(Enemy {
        health: 50,
        damage: 10,
        speed: 3.0,
    })?
    
    // Save prefab
    prefab.save("prefabs/enemy.prefab")?
    
    Ok(())
}

fun spawn_enemy(scene: &mut Scene, position: Vec3) -> Result<Entity, str> {
    let prefab = Prefab::load("prefabs/enemy.prefab")?
    let entity = prefab.instantiate()?
    entity.set_position(position)?
    scene.add_entity(entity)?
    Ok(entity)
}
```

---

## Input Handling

### Input System

```titan
fun handle_game_input(engine: &GameEngine, world: &mut World) -> Result<(), str> {
    let input = engine.input_manager()
    
    // Keyboard input
    if input.key_pressed(Key::W) {
        // Handle forward movement
    }
    
    if input.key_down(Key::Space) {
        // Keep jumping
    }
    
    if input.key_released(Key::Escape) {
        engine.toggle_pause()?
    }
    
    // Mouse input
    if input.mouse_button_pressed(MouseButton::Left) {
        let mouse_pos = input.mouse_position()
        // Handle click at position
    }
    
    if input.mouse_moved() {
        let (dx, dy) = input.mouse_delta()
        // Camera rotation
    }
    
    // Gamepad input
    if let Some(gamepad) = input.gamepad(0)? {
        let left_stick = gamepad.left_stick()
        let right_stick = gamepad.right_stick()
        
        if gamepad.button_pressed(GamepadButton::A) {
            // Jump
        }
        
        if gamepad.trigger_down(GamepadTrigger::Right) > 0.5 {
            // Attack
        }
    }
    
    // Touch input (mobile)
    for touch in input.touches()? {
        match touch.phase {
            TouchPhase::Started => {
                // Touch began
            },
            TouchPhase::Moved => {
                // Touch moving
            },
            TouchPhase::Ended => {
                // Touch ended
            },
        }
    }
    
    Ok(())
}
```

### Input Binding

```titan
fun setup_input_bindings(engine: &mut GameEngine) -> Result<(), str> {
    let mut bindings = InputBindings::new()
    
    // Keyboard bindings
    bindings.bind_action("move_forward", vec![Key::W, Key::Up])?
    bindings.bind_action("move_backward", vec![Key::S, Key::Down])?
    bindings.bind_action("move_left", vec![Key::A, Key::Left])?
    bindings.bind_action("move_right", vec![Key::D, Key::Right])?
    bindings.bind_action("jump", vec![Key::Space, GamepadButton::A])?
    bindings.bind_action("attack", vec![MouseButton::Left, GamepadButton::X])?
    
    // Axis bindings
    bindings.bind_axis("look_x", vec![
        AxisInput::MouseX(sensitivity: 0.1),
        AxisInput::GamepadRightStickX(sensitivity: 1.0),
    ])?
    
    bindings.bind_axis("look_y", vec![
        AxisInput::MouseY(sensitivity: 0.1),
        AxisInput::GamepadRightStickY(sensitivity: 1.0),
    ])?
    
    engine.set_input_bindings(bindings)?
    
    Ok(())
}
```

---

## Asset Management

### Asset Loading

```titan
fun load_assets(engine: &mut GameEngine) -> Result<AssetBundle, str> {
    let mut bundle = AssetBundle::new()
    
    // Async texture loading
    bundle.load_texture_async("textures/player_albedo.png")?
    bundle.load_texture_async("textures/terrain_height.png")?
    
    // Async mesh loading
    bundle.load_mesh_async("models/player.gltf")?
    bundle.load_mesh_async("models/buildings.gltf")?
    
    // Async audio loading
    bundle.load_audio_async("audio/music/exploration.ogg")?
    bundle.load_audio_async("audio/sfx/footstep.wav")?
    
    // Wait for loading
    bundle.wait_for_completion()?
    
    Ok(bundle)
}
```

### Asset Caching

```titan
fun cache_management(engine: &mut GameEngine) -> Result<(), str> {
    let cache = engine.asset_cache()
    
    // Set memory limits
    cache.set_max_memory(512 * 1024 * 1024)?  // 512 MB
    cache.set_eviction_policy(EvictionPolicy::LRU)?
    
    // Preload frequently used assets
    cache.preload("textures/ui.png")?
    cache.preload("models/player.gltf")?
    cache.preload("audio/music/background.ogg")?
    
    Ok(())
}
```

---

## Game Loop & Timing

### Custom Game Loop

```titan
fun custom_game_loop(engine: &mut GameEngine) -> Result<(), str> {
    let target_fps = 60.0
    let target_frametime = 1.0 / target_fps
    
    let mut accumulator = 0.0
    let fixed_timestep = 1.0 / 60.0  // Physics at 60 Hz
    
    let mut last_time = engine.current_time()?
    
    while engine.is_running()? {
        let current_time = engine.current_time()?
        let mut delta_time = (current_time - last_time) as f32
        last_time = current_time
        
        // Cap delta time (prevent spiral of death)
        if delta_time > 0.25 {
            delta_time = 0.25
        }
        
        accumulator += delta_time
        
        // Fixed timestep physics
        while accumulator >= fixed_timestep {
            engine.update_physics(fixed_timestep)?
            accumulator -= fixed_timestep
        }
        
        // Variable timestep updates
        engine.update_logic(delta_time)?
        engine.update_animation(delta_time)?
        engine.update_particles(delta_time)?
        
        // Render
        engine.render()?
        
        // Frame pacing
        let frame_time = (engine.current_time()? - current_time) as f32
        if frame_time < target_frametime {
            let sleep_time = target_frametime - frame_time
            std::thread::sleep(Duration::from_secs_f32(sleep_time))
        }
    }
    
    Ok(())
}
```

---

## Networking (Multiplayer)

### Network Connection

```titan
fun setup_multiplayer(engine: &mut GameEngine) -> Result<(), str> {
    let network = NetworkManager::new()?
    
    // Host game
    network.host_game(
        port: 7777,
        max_players: 8,
        game_mode: "TDM"
    )?
    
    // Or join game
    network.connect_to_server(
        address: "192.168.1.100:7777",
        player_name: "Player1"
    )?
    
    engine.set_network_manager(network)?
    
    Ok(())
}
```

### Entity Replication

```titan
fun replicate_entity(entity: &Entity, network: &NetworkManager) -> Result<(), str> {
    // Mark entity for replication
    entity.add_component(NetworkReplicated {
        network_id: 1,
        owner: PlayerId::Server,
        authority: ReplicaAuthority::Server,
    })?
    
    // Replicate transform
    entity.replicate_component::<Transform>()?
    
    // Replicate custom data
    entity.replicate_component::<Enemy>()?
    
    Ok(())
}
```

---

## Save/Load System

```titan
fun save_game(scene: &Scene) -> Result<(), str> {
    let save = GameSave::new()
    
    // Save player data
    save.set("player_health", 85)?
    save.set("player_position", scene.player_position()?)?
    save.set("inventory", player_inventory()?)?
    
    // Save level state
    save.set("enemies_defeated", 15)?
    save.set("objectives_complete", vec![true, false, true])?
    
    // Save to file
    save.write_to_file("saves/save_1.sav")?
    
    Ok(())
}

fun load_game(engine: &mut GameEngine) -> Result<(), str> {
    let save = GameSave::load_from_file("saves/save_1.sav")?
    
    // Restore player state
    let health = save.get_i32("player_health")?
    let position = save.get_vec3("player_position")?
    let inventory = save.get_vec("inventory")?
    
    // Restore level state
    let enemies = save.get_i32("enemies_defeated")?
    
    Ok(())
}
```

---

## Debugging Tools

### Debug Visualization

```titan
fun enable_debug_mode(engine: &mut GameEngine) -> Result<(), str> {
    let debug = engine.debug_manager()
    
    // Visualize physics colliders
    debug.draw_colliders(true)?
    debug.draw_forces(true)?
    
    // Show bounding volumes
    debug.draw_aabbs(true)?
    debug.draw_octrees(true)?
    
    // Show text overlays
    debug.draw_fps(true)?
    debug.draw_performance_graph(true)?
    
    // Draw custom lines
    debug.draw_line(
        from: vec3(0.0, 0.0, 0.0),
        to: vec3(1.0, 0.0, 0.0),
        color: Color::Red
    )?
    
    Ok(())
}
```

### Performance Profiling

```titan
fun profile_game(engine: &GameEngine) -> Result<(), str> {
    let profiler = engine.profiler()
    
    let stats = profiler.frame_stats()?
    println!("FPS: {:.1}", stats.fps)
    println!("Frame time: {:.2}ms", stats.frame_time_ms)
    println!("Physics time: {:.2}ms", stats.physics_time_ms)
    println!("Render time: {:.2}ms", stats.render_time_ms)
    println!("Memory used: {:.1}MB", stats.memory_used_mb)
    
    Ok(())
}
```

---

## Best Practices

✅ **DO**
- Use fixed timestep for physics
- Batch entity updates
- Async load large assets
- Profile regularly
- Use object pooling for frequently created objects

❌ **DON'T**
- Allocate memory every frame
- Serialize entire world state
- Use unlimited draw calls
- Create entities during iteration
- Store raw pointers to entities

---

## Next Steps

- [GAME_DESIGN_PLATFORM.md](GAME_DESIGN_PLATFORM.md) - Game editor
- [GRAPHICS_FRAMEWORK_GUIDE.md](GRAPHICS_FRAMEWORK_GUIDE.md) - Advanced rendering
- [GAME_TUTORIAL_FPS.md](GAME_TUTORIAL_FPS.md) - Build an FPS game

---

**Game Framework** - Professional game engine powered by Omnisystem!
