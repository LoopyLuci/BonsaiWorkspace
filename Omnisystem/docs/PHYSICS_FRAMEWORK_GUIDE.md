# Physics Framework Guide - 3D Physics Engine

**High-performance physics simulation for games, robotics, and simulations**

---

## Overview

The Physics Framework provides:
- **Rigid Body Dynamics** - Constraints, joints, impulse resolution
- **Collision Detection** - Broadphase, narrowphase, shape testing
- **3D Shapes** - Spheres, boxes, capsules, convex meshes
- **Constraints & Joints** - Fixed, hinge, ball-socket, slider
- **Soft Bodies** - Cloth, hair, deformables
- **Particles & Fluids** - Particle systems, fluid simulation
- **Deterministic Physics** - Fixed-point deterministic simulation

---

## Core Architecture

```
Physics World
    ├─ Rigid Bodies
    ├─ Shapes & Colliders
    ├─ Constraints & Joints
    ├─ Broadphase (spatial hashing)
    ├─ Narrowphase (shape-specific)
    └─ Constraint Solver (iterative)
```

---

## Quick Start

```titan
use omnisystem::physics::*

fun main() -> Result<(), str> {
    // Create physics world
    let mut world = PhysicsWorld::new()
        .with_gravity(0.0, -9.81, 0.0)
        .with_simulation_rate(60.0)
    
    // Create ground
    let ground = RigidBody::static_box(
        width: 10.0, height: 1.0, depth: 10.0
    )?
    .with_position(0.0, -1.0, 0.0)
    
    // Create sphere
    let sphere = RigidBody::dynamic_sphere(radius: 0.5)?
        .with_position(0.0, 2.0, 0.0)
        .with_mass(1.0)
        .with_velocity(0.0, 0.0, 0.0)
    
    world.add_body("ground", ground)?
    world.add_body("sphere", sphere)?
    
    // Simulation loop
    for _ in 0..600 {
        world.step(dt: 1.0 / 60.0)?
        let sphere_pos = world.body_position("sphere")?
        println!("Y: {:.2}", sphere_pos.y)
    }
    
    Ok(())
}
```

---

## Rigid Bodies

### Creation & Properties

```titan
fun create_rigid_bodies() -> Result<Vec<RigidBody>, str> {
    // Static body (immovable)
    let wall = RigidBody::static_box(width: 1.0, height: 5.0, depth: 10.0)?
        .with_position(5.0, 0.0, 0.0)
    
    // Dynamic body (moves under forces)
    let cube = RigidBody::dynamic_box(
        width: 1.0, height: 1.0, depth: 1.0
    )?
    .with_position(0.0, 2.0, 0.0)
    .with_mass(2.0)
    .with_velocity(1.0, 0.0, 0.0)
    .with_angular_velocity(vec3(0.0, 0.0, 0.0))
    
    // Kinematic body (controlled movement)
    let platform = RigidBody::kinematic_box(
        width: 2.0, height: 0.5, depth: 5.0
    )?
    .with_position(0.0, 1.0, 0.0)
    
    // Dynamic sphere
    let ball = RigidBody::dynamic_sphere(radius: 0.5)?
        .with_position(0.0, 5.0, 0.0)
        .with_mass(1.0)
        .with_restitution(0.9)  // Bounciness
    
    Ok(vec![wall, cube, platform, ball])
}
```

### Forces & Impulses

```titan
fun apply_forces(body: &mut RigidBody) -> Result<(), str> {
    // Apply force (acceleration)
    body.apply_force(vec3(10.0, 0.0, 0.0))?
    
    // Apply impulse (instant velocity change)
    body.apply_impulse(vec3(0.0, 5.0, 0.0))?
    
    // Apply torque (angular acceleration)
    body.apply_torque(vec3(1.0, 0.0, 0.0))?
    
    // Apply angular impulse
    body.apply_angular_impulse(vec3(0.0, 2.0, 0.0))?
    
    // Set velocity directly
    body.set_velocity(vec3(5.0, 0.0, 0.0))?
    
    // Set angular velocity
    body.set_angular_velocity(vec3(0.0, 10.0, 0.0))?
    
    Ok(())
}
```

---

## Collision Shapes

### Shape Definition

```titan
fun create_shapes() -> Result<Vec<Shape>, str> {
    // Sphere
    let sphere = Shape::sphere(radius: 1.0)
    
    // Box (AABB)
    let aabb = Shape::box(
        width: 2.0, height: 3.0, depth: 1.5
    )
    
    // Capsule
    let capsule = Shape::capsule(
        radius: 0.5, height: 2.0
    )
    
    // Cylinder
    let cylinder = Shape::cylinder(
        radius: 0.5, height: 2.0
    )
    
    // Cone
    let cone = Shape::cone(
        radius: 1.0, height: 2.0
    )
    
    // Convex mesh
    let mesh = Mesh::load("convex_shape.gltf")?
    let convex = Shape::convex_mesh(&mesh)
    
    // Plane (infinite)
    let plane = Shape::plane(normal: vec3(0.0, 1.0, 0.0))
    
    Ok(vec![sphere, aabb, capsule, cylinder, cone, convex, plane])
}
```

### Compound Shapes

```titan
fun create_compound_shape() -> Result<Shape, str> {
    let mut compound = Shape::compound()
    
    // Add spheres
    compound.add_shape(
        Shape::sphere(radius: 0.5),
        offset: vec3(0.0, 0.0, 0.0)
    )?
    
    compound.add_shape(
        Shape::sphere(radius: 0.3),
        offset: vec3(1.0, 0.0, 0.0)
    )?
    
    compound.add_shape(
        Shape::sphere(radius: 0.3),
        offset: vec3(-1.0, 0.0, 0.0)
    )?
    
    Ok(compound)
}
```

---

## Collision Detection

### Collision Events

```titan
fun handle_collisions(world: &PhysicsWorld) -> Result<(), str> {
    for contact in world.contacts()? {
        println!("Body {} collided with {}", 
            contact.body_a, contact.body_b
        )
        
        println!("  Position: {:?}", contact.position)
        println!("  Normal: {:?}", contact.normal)
        println!("  Depth: {:.3}", contact.penetration_depth)
    }
    
    Ok(())
}
```

### Ray Casting

```titan
fun raycast(world: &PhysicsWorld) -> Result<(), str> {
    let ray = Ray::new(
        origin: vec3(0.0, 0.0, 0.0),
        direction: vec3(0.0, 1.0, 0.0),
        max_distance: 100.0
    )
    
    if let Some(hit) = world.raycast(&ray)? {
        println!("Hit: {}", hit.body_name)
        println!("Distance: {:.3}", hit.distance)
        println!("Position: {:?}", hit.position)
        println!("Normal: {:?}", hit.normal)
    }
    
    Ok(())
}
```

### Shape Testing

```titan
fun shape_tests(world: &PhysicsWorld) -> Result<(), str> {
    let sphere = Shape::sphere(radius: 1.0)
    let position = vec3(0.0, 5.0, 0.0)
    
    // Point containment
    let point = vec3(0.0, 5.0, 0.0)
    let contains = world.shape_contains_point(&sphere, position, &point)?
    
    // Sphere sweep
    let sweep_result = world.sweep_sphere(
        shape: &sphere,
        from: vec3(0.0, 5.0, 0.0),
        to: vec3(0.0, 0.0, 0.0)
    )?
    
    Ok(())
}
```

---

## Constraints & Joints

### Joint Types

```titan
fun create_joints(world: &mut PhysicsWorld) -> Result<(), str> {
    // Fixed joint (rigid connection)
    let fixed = FixedJoint::new("body1", "body2")
        .with_max_force(1000.0)
    world.add_joint("fixed", fixed)?
    
    // Ball-socket joint (point constraint)
    let ball = BallSocketJoint::new("body1", "body2")
        .with_anchor1(vec3(0.5, 0.0, 0.0))
        .with_anchor2(vec3(-0.5, 0.0, 0.0))
    world.add_joint("ball", ball)?
    
    // Hinge joint (1D rotation)
    let hinge = HingeJoint::new("body1", "body2")
        .with_axis(vec3(0.0, 1.0, 0.0))
        .with_anchor(vec3(0.0, 0.0, 0.0))
        .with_angle_limits(min: -90.0, max: 90.0)
    world.add_joint("hinge", hinge)?
    
    // Slider joint (1D translation)
    let slider = SliderJoint::new("body1", "body2")
        .with_axis(vec3(1.0, 0.0, 0.0))
        .with_limits(min: -1.0, max: 1.0)
    world.add_joint("slider", slider)?
    
    // Distance constraint
    let distance = DistanceJoint::new("body1", "body2")
        .with_distance(2.0)
        .with_tolerance(0.1)
    world.add_joint("distance", distance)?
    
    Ok(())
}
```

### Motors & Drives

```titan
fun motorized_joint() -> Result<HingeJoint, str> {
    let mut joint = HingeJoint::new("body1", "body2")
        .with_axis(vec3(0.0, 1.0, 0.0))
    
    // Add motor
    joint.enable_motor(true)
    joint.set_motor_target_velocity(10.0)  // 10 rad/s
    joint.set_motor_max_torque(100.0)
    
    Ok(joint)
}
```

---

## Soft Bodies & Cloth

### Cloth Simulation

```titan
fun create_cloth(world: &mut PhysicsWorld) -> Result<(), str> {
    let cloth = ClothBody::new()
        .with_size(width: 4.0, height: 4.0)
        .with_segments(width: 20, height: 20)
        .with_mass_per_particle(0.1)
        .with_damping(0.99)
    
    // Pin top corners
    cloth.pin_particle(0, vec3(0.0, 3.0, 0.0))?
    cloth.pin_particle(19, vec3(4.0, 3.0, 0.0))?
    
    // Wind force
    cloth.add_force(vec3(0.5, 0.0, 0.0))?
    
    world.add_cloth("cloth", cloth)?
    
    Ok(())
}
```

### Rope/Hair

```titan
fun create_rope(world: &mut PhysicsWorld) -> Result<(), str> {
    let rope = RopeBody::new(length: 5.0, segments: 20)
        .with_mass_per_particle(0.1)
        .with_damping(0.98)
        .with_slack(0.0)
    
    // Attach to point
    rope.attach_start(vec3(0.0, 5.0, 0.0))?
    
    world.add_rope("rope", rope)?
    
    Ok(())
}
```

---

## Particle Systems

### Particle Dynamics

```titan
fun particle_system(world: &mut PhysicsWorld) -> Result<(), str> {
    let particles = ParticleSystem::new()
        .with_capacity(10000)
        .with_radius(0.1)
        .with_damping(0.999)
    
    // Add particles
    for i in 0..100 {
        let x = (i % 10) as f32 * 0.2
        let y = 2.0 + (i / 10) as f32 * 0.2
        particles.add_particle(
            position: vec3(x, y, 0.0),
            velocity: vec3(0.0, 0.0, 0.0),
            mass: 1.0
        )?
    }
    
    world.add_particles("water", particles)?
    
    Ok(())
}
```

### Fluid Simulation

```titan
fun fluid_simulation(world: &mut PhysicsWorld) -> Result<(), str> {
    let fluid = FluidSimulation::new()
        .with_domain(min: vec3(-5.0, -5.0, -5.0), max: vec3(5.0, 5.0, 5.0))
        .with_grid_resolution(20, 20, 20)
        .with_viscosity(0.0001)
        .with_density(1000.0)
    
    world.add_fluid("liquid", fluid)?
    
    Ok(())
}
```

---

## Performance Optimization

### Spatial Partitioning

```titan
fun setup_broadphase(world: &mut PhysicsWorld) -> Result<(), str> {
    // Dynamic Axis-Aligned Bounding Box Tree
    world.set_broadphase(BroadphaseType::AABB)?
    
    // Or spatial hashing
    world.set_broadphase(BroadphaseType::SpatialHash {
        cell_size: 2.0
    })?
    
    Ok(())
}
```

### Sleeping & Deactivation

```titan
fun optimize_sleeping(world: &mut PhysicsWorld) -> Result<(), str> {
    // Enable sleeping for static/slow bodies
    world.enable_sleeping(true)?
    world.set_sleep_threshold(velocity: 0.1, angular_velocity: 0.1)?
    world.set_sleep_delay(frames: 10)?
    
    Ok(())
}
```

### Constraint Iterations

```titan
fun optimize_solver(world: &mut PhysicsWorld) -> Result<(), str> {
    // Fewer iterations = faster but less stable
    world.set_solver_iterations(velocity: 4, position: 1)?
    
    // Use warm starting
    world.enable_warm_starting(true)?
    
    Ok(())
}
```

---

## Advanced Features

### Vehicle Simulation

```titan
fun create_vehicle(world: &mut PhysicsWorld) -> Result<(), str> {
    let vehicle = Vehicle::new()
        .with_chassis_mass(1500.0)
        .with_wheel_count(4)
    
    // Front left wheel
    vehicle.add_wheel(
        position: vec3(-0.8, 0.3, 1.0),
        radius: 0.4,
        suspension_length: 0.3
    )?
    
    // Add more wheels...
    
    world.add_vehicle("car", vehicle)?
    
    Ok(())
}
```

### Terrain

```titan
fun create_terrain(world: &mut PhysicsWorld) -> Result<(), str> {
    let heights = vec![
        vec![0.0, 0.1, 0.2],
        vec![0.1, 0.3, 0.2],
        vec![0.2, 0.2, 0.1],
    ]
    
    let terrain = TerrainBody::from_heightmap(
        &heights,
        width: 10.0,
        depth: 10.0,
        height_scale: 1.0
    )?
    
    world.add_terrain("ground", terrain)?
    
    Ok(())
}
```

---

## Best Practices

✅ **DO**
- Use fixed timestep
- Enable sleeping for efficiency
- Use appropriate collision shapes
- Limit velocity to prevent tunneling
- Test with various mass ratios

❌ **DON'T**
- Teleport bodies through walls
- Use variable timesteps
- Create chains of bodies > 4 deep
- Ignore penetration depth
- Mix scales dramatically

---

## Next Steps

- [GAME_FRAMEWORK_GUIDE.md](GAME_FRAMEWORK_GUIDE.md) - Game development
- [GAME_DESIGN_PLATFORM.md](GAME_DESIGN_PLATFORM.md) - Game editor
- [PHYSICS_TUTORIAL.md](PHYSICS_TUTORIAL.md) - Physics simulation examples

---

**Physics Framework** - High-performance 3D physics for interactive applications!
