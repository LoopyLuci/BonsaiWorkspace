# HELIX Guide - Graphics & Physics

**HELIX** is Omnisystem's graphics and physics language, optimized for 3D rendering, game development, and simulation.

## Core Features

### Graphics
- 3D rendering pipeline
- Material system (PBR)
- Lighting (directional, point, spot)
- Texture mapping
- Particle effects
- Camera management

### Physics
- Rigid body dynamics
- Collision detection
- Impulse resolution
- Gravity and forces
- Raycast queries

## Common Usage

```helix
let renderer = Renderer::new(1920, 1080);
let physics = Physics::new();

// Load model
let mesh = load_model("model.obj")?;

// Create physics body
let body = RigidBody::new(1.0, mesh);
physics.add_body(body)?;

// Update and render
physics.update(delta_time)?;
renderer.render_scene()?;
```

## Related Documentation

- [API Reference](../05-reference/HELIX_API.md)
- [Building Games](../04-guides/GAMES.md)

---

**Status**: Production Ready | **Updated**: 2026-06-16
