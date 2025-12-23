# Rapier Physics Integration Guide

Using framealloc with Rapier physics engine v0.31 for high-performance physics simulations.

## Table of Contents

1. [Overview](#overview)
2. [Quick Start](#quick-start)
3. [PhysicsWorld2D](#physicsworld2d)
4. [PhysicsWorld3D](#physicsworld3d)
5. [Performance Optimization](#performance-optimization)
6. [Common Patterns](#common-patterns)
7. [Migration from Raw Rapier](#migration-from-raw-rapier)

## Overview

framealloc provides frame-aware wrappers for Rapier that:
- Eliminate manual memory management for physics data
- Provide bulk allocation for contact events and query results
- Integrate with frame boundaries for automatic cleanup
- Support both 2D and 3D physics

### Benefits

- **139x faster** contact buffer allocation
- Single bulk allocation per query
- Zero manual memory management
- Automatic cleanup at frame boundaries

## Quick Start

### Installation

```toml
# Cargo.toml
framealloc = { version = "0.10", features = ["rapier"] }
```

### Basic 2D Physics

```rust
use framealloc::{SmartAlloc, rapier::PhysicsWorld2D};
use rapier2d::dynamics::{RigidBodyBuilder, RigidBodyHandle};
use rapier2d::geometry::{ColliderBuilder};

fn main() {
    let alloc = SmartAlloc::new(Default::default());
    let mut physics = PhysicsWorld2D::new();
    
    // Create ground
    let ground = physics.insert_body(
        RigidBodyBuilder::static()
            .translation(0.0, -1.0),
        ColliderBuilder::cuboid(10.0, 1.0),
        &alloc
    );
    
    // Create falling box
    let box_body = physics.insert_body(
        RigidBodyBuilder::dynamic()
            .translation(0.0, 5.0),
        ColliderBuilder::cuboid(0.5, 0.5),
        &alloc
    );
    
    // Game loop
    loop {
        alloc.begin_frame();
        
        // Step physics with frame-allocated events
        let events = physics.step_with_events(&alloc);
        
        // Process contacts
        for contact in &events.contacts {
            println!("Contact: {:?}", contact);
        }
        
        alloc.end_frame();
    }
}
```

## PhysicsWorld2D

### Creating Bodies

```rust
use framealloc::SmartAlloc;
use framealloc::rapier::PhysicsWorld2D;
use rapier2d::dynamics::{RigidBodyBuilder, RigidBodyType};
use rapier2d::geometry::{ColliderBuilder, ShapeType};

fn create_bodies(physics: &mut PhysicsWorld2D, alloc: &SmartAlloc) {
    // Static body (immovable)
    let ground = physics.insert_body(
        RigidBodyBuilder::static()
            .translation(0.0, 0.0),
        ColliderBuilder::cuboid(20.0, 1.0),
        &alloc
    );
    
    // Dynamic body (affected by forces)
    let ball = physics.insert_body(
        RigidBodyBuilder::dynamic()
            .translation(0.0, 10.0)
            .can_sleep(false),
        ColliderBuilder::ball(0.5),
        &alloc
    );
    
    // Kinematic body (moves but isn't affected by forces)
    let platform = physics.insert_body(
        RigidBodyBuilder::kinematic_velocity_based()
            .translation(0.0, 5.0),
        ColliderBuilder::cuboid(3.0, 0.2),
        &alloc
    );
    
    // Complex shape
    let polygon = physics.insert_body(
        RigidBodyBuilder::dynamic()
            .translation(-5.0, 2.0),
        ColliderBuilder::convex_hull(&[
            rapier2d::na::Point2::new(0.0, 0.0),
            rapier2d::na::Point2::new(1.0, 0.0),
            rapier2d::na::Point2::new(0.5, 1.0),
        ]).unwrap(),
        &alloc
    );
}
```

### Forces and Impulses

```rust
fn apply_forces(physics: &mut PhysicsWorld2D) {
    // Apply force to all dynamic bodies
    for (handle, body) in physics.bodies.iter() {
        if body.is_dynamic() {
            let force = rapier2d::na::Vector2::new(0.0, -9.81 * body.mass());
            physics.rigid_bodies.get_mut(handle).unwrap()
                .add_force(force, true);
        }
    }
    
    // Apply impulse to specific body
    if let Some(ball) = physics.rigid_bodies.get_mut(&ball_handle) {
        let impulse = rapier2d::na::Vector2::new(10.0, 0.0);
        ball.apply_impulse(impulse, true);
    }
}
```

### Collision Events

```rust
fn process_collisions(events: &PhysicsEvents2D) {
    // Contact events
    for contact in &events.contacts {
        println!(
            "Contact between {:?} and {:?} at {:?}",
            contact.collider1,
            contact.collider2,
            contact.manifold.local_n1
        );
    }
    
    // Proximity events (near-misses)
    for proximity in &events.proximity_events {
        match proximity.new_status {
            rapier2d::geometry::Proximity::Intersecting => {
                println!("Objects {:?} and {:?} started intersecting", 
                    proximity.collider1, proximity.collider2);
            }
            rapier2d::geometry::Proximity::WithinMargin => {
                println!("Objects {:?} and {:?} are within margin", 
                    proximity.collider1, proximity.collider2);
            }
            rapier2d::geometry::Proximity::Disjoint => {
                println!("Objects {:?} and {:?} separated", 
                    proximity.collider1, proximity.collider2);
            }
        }
    }
}
```

## PhysicsWorld3D

### Creating 3D Bodies

```rust
use framealloc::SmartAlloc;
use framealloc::rapier::PhysicsWorld3D;
use rapier3d::dynamics::RigidBodyBuilder;
use rapier3d::geometry::ColliderBuilder;
use rapier3d::na::{Vector3, Point3};

fn create_3d_scene(physics: &mut PhysicsWorld3D, alloc: &SmartAlloc) {
    // Ground plane
    let ground = physics.insert_body(
        RigidBodyBuilder::static()
            .translation(Vector3::new(0.0, -1.0, 0.0)),
        ColliderBuilder::halfspace(Vector3::new(0.0, 1.0, 0.0), 0.0),
        &alloc
    );
    
    // Falling sphere
    let sphere = physics.insert_body(
        RigidBodyBuilder::dynamic()
            .translation(Vector3::new(0.0, 5.0, 0.0)),
        ColliderBuilder::ball(0.5),
        &alloc
    );
    
    // Rotating box
    let box_body = physics.insert_body(
        RigidBodyBuilder::dynamic()
            .translation(Vector3::new(2.0, 2.0, 0.0))
            .can_sleep(false),
        ColliderBuilder::cuboid(1.0, 1.0, 1.0),
        &alloc
    );
    
    // Add initial angular velocity
    if let Some(body) = physics.rigid_bodies.get_mut(&box_body) {
        body.set_angvel(rapier3d::na::Vector3::new(0.0, 5.0, 0.0), true);
    }
}
```

### 3D Ray Casting

```rust
fn cast_rays_3d(physics: &PhysicsWorld3D, alloc: &SmartAlloc) {
    // Cast multiple rays from camera
    let ray_origins = [
        Point3::new(0.0, 5.0, 0.0),
        Point3::new(1.0, 5.0, 0.0),
        Point3::new(-1.0, 5.0, 0.0),
    ];
    
    let ray_dir = Vector3::new(0.0, -1.0, 0.0);
    let max_toi = 100.0;
    let solid = true;
    let filter = rapier3d::pipeline::QueryFilter::default();
    
    for origin in &ray_origins {
        let hits = physics.cast_ray(
            &rapier3d::geometry::Ray::new(*origin, ray_dir),
            max_toi,
            solid,
            &filter,
            &alloc
        );
        
        for hit in hits {
            println!(
                "Ray hit entity {:?} at distance {}",
                hit.entity,
                hit.time_of_impact
            );
        }
    }
}
```

### 3D Shape Casting

```rust
fn shape_cast_3d(physics: &PhysicsWorld3D, alloc: &SmartAlloc) {
    // Cast a sphere to find where it would hit
    let shape_pos = rapier3d::na::Isometry3::new(
        Vector3::new(0.0, 10.0, 0.0),
        rapier3d::na::UnitQuaternion::identity()
    );
    let shape_vel = Vector3::new(0.0, -10.0, 0.0);
    
    let hits = physics.cast_shape(
        &shape_pos,
        &shape_vel,
        &rapier3d::geometry::Ball::new(0.5),
        1.0,
        rapier3d::pipeline::QueryFilter::default(),
        &alloc
    );
    
    for hit in hits {
        println!("Shape would hit at point {:?}", hit.witness1);
    }
}
```

## Performance Optimization

### Batch Operations

```rust
fn optimized_physics_step(physics: &mut PhysicsWorld2D, alloc: &SmartAlloc) {
    // Use step_with_events for frame-allocated contact buffers
    let events = physics.step_with_events(alloc);
    
    // Process all contacts in batch
    if !events.contacts.is_empty() {
        // Batch allocate response data
        let responses = unsafe {
            let batch = alloc.frame_alloc_batch::<ContactResponse>(events.contacts.len());
            for (i, contact) in events.contacts.iter().enumerate() {
                let response = batch.add(i);
                std::ptr::write(response, ContactResponse::new(contact));
            }
            batch
        };
        
        // Apply responses
        for i in 0..events.contacts.len() {
            let response = unsafe { responses.get(i) };
            apply_contact_response(response);
        }
    }
}
```

### Query Optimization

```rust
fn efficient_queries(physics: &PhysicsWorld2D, alloc: &SmartAlloc) {
    // Use spatial queries efficiently
    let aabb = rapier2d::bounding_volume::AABB {
        mins: rapier2d::na::Point2::new(-10.0, -10.0),
        maxs: rapier2d::na::Point2::new(10.0, 10.0),
    };
    
    // Find all entities in region
    let entities = physics.intersections_with_aabb(
        &aabb,
        rapier2d::pipeline::QueryFilter::default(),
        &alloc
    );
    
    // Process in parallel if needed
    entities.par_iter().for_each(|entity| {
        process_entity(*entity);
    });
}
```

### Memory Pooling

```rust
struct PhysicsCache {
    contact_buffers: Vec<Vec<Contact>>,
    query_results: Vec<Vec<RaycastHit>>,
    current_frame: usize,
}

impl PhysicsCache {
    fn get_contact_buffer(&mut self, alloc: &SmartAlloc) -> FrameBox<[Contact]> {
        if self.current_frame >= self.contact_buffers.len() {
            self.contact_buffers.push(Vec::new());
        }
        
        let buffer = &mut self.contact_buffers[self.current_frame];
        buffer.clear();
        
        // Return frame-allocated view
        alloc.frame_box(buffer.split_at(buffer.len()).0)
    }
}
```

## Common Patterns

### Character Controller

```rust
struct CharacterController {
    body: RigidBodyHandle,
    height: f32,
    radius: f32,
    slope_angle: f32,
}

impl CharacterController {
    fn move_character(
        &self,
        physics: &mut PhysicsWorld2D,
        alloc: &SmartAlloc,
        direction: rapier2d::na::Vector2<f32>,
        speed: f32,
    ) {
        // Cast ray downward to find ground
        let ray = rapier2d::geometry::Ray::new(
            rapier2d::na::Point2::new(0.0, self.height + 0.1),
            rapier2d::na::Vector2::new(0.0, -1.0)
        );
        
        let hits = physics.cast_ray(
            &ray,
            self.height + 0.2,
            true,
            &rapier2d::pipeline::QueryFilter::exclude_dynamic(),
            &alloc
        );
        
        let on_ground = hits.iter().any(|h| h.time_of_impact <= self.height + 0.1);
        
        // Apply movement
        if let Some(body) = physics.rigid_bodies.get_mut(&self.body) {
            if on_ground {
                // Ground movement
                let velocity = direction * speed;
                body.set_linvel(velocity, true);
            } else {
                // Air movement (reduced)
                let velocity = direction * speed * 0.5;
                body.set_linvel(velocity, true);
            }
        }
    }
}
```

### Destruction System

```rust
struct DestructionSystem {
    to_destroy: Vec<RigidBodyHandle>,
}

impl DestructionSystem {
    fn mark_for_destruction(&mut self, body: RigidBodyHandle) {
        self.to_destroy.push(body);
    }
    
    fn process_destructions(&mut self, physics: &mut PhysicsWorld2D) {
        for handle in self.to_destroy.drain(..) {
            physics.remove_body(handle);
        }
    }
    
    fn explode_at(
        &mut self,
        physics: &mut PhysicsWorld2D,
        alloc: &SmartAlloc,
        center: rapier2d::na::Point2<f32>,
        radius: f32,
        force: f32,
    ) {
        // Find bodies in explosion radius
        let aabb = rapier2d::bounding_volume::AABB {
            mins: rapier2d::na::Point2::new(center.x - radius, center.y - radius),
            maxs: rapier2d::na::Point2::new(center.x + radius, center.y + radius),
        };
        
        let entities = physics.intersections_with_aabb(
            &aabb,
            rapier2d::pipeline::QueryFilter::default(),
            &alloc
        );
        
        // Apply explosion force
        for entity in entities {
            if let Some(body) = physics.rigid_bodies.get(&entity.handle) {
                let to_body = body.position().translation.vector - center.coords;
                let distance = to_body.magnitude();
                
                if distance < radius && distance > 0.01 {
                    let explosion_force = (1.0 - distance / radius) * force;
                    let direction = to_body.normalize();
                    
                    // Mark for destruction if close enough
                    if distance < radius * 0.3 {
                        self.mark_for_destruction(entity.handle);
                    } else {
                        // Apply force
                        let impulse = direction * explosion_force;
                        body.apply_impulse(impulse, true);
                    }
                }
            }
        }
    }
}
```

### Trigger System

```rust
struct TriggerSystem {
    triggers: Vec<Trigger>,
}

struct Trigger {
    body: RigidBodyHandle,
    on_enter: Vec<fn(RigidBodyHandle)>,
    on_exit: Vec<fn(RigidBodyHandle)>,
    inside: HashSet<RigidBodyHandle>,
}

impl TriggerSystem {
    fn update(&mut self, events: &PhysicsEvents2D) {
        // Process proximity events for triggers
        for proximity in &events.proximity_events {
            if let Some(trigger) = self.triggers.iter_mut()
                .find(|t| t.body == proximity.collider1 || t.body == proximity.collider2) {
                
                let other = if trigger.body == proximity.collider1 {
                    proximity.collider2
                } else {
                    proximity.collider1
                };
                
                match proximity.new_status {
                    rapier2d::geometry::Proximity::Intersecting => {
                        if !trigger.inside.contains(&other) {
                            trigger.inside.insert(other);
                            for callback in &trigger.on_enter {
                                callback(other);
                            }
                        }
                    }
                    rapier2d::geometry::Proximity::Disjoint => {
                        if trigger.inside.remove(&other) {
                            for callback in &trigger.on_exit {
                                callback(other);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}
```

## Migration from Raw Rapier

### Before (Raw Rapier)

```rust
use rapier2d::dynamics::{RigidBodySet, ColliderSet, IntegrationParameters};
use rapier2d::geometry::{BroadPhase, NarrowPhase, ColliderSet};
use rapier2d::pipeline::{PhysicsPipeline, QueryPipeline};

struct PhysicsWorld {
    bodies: RigidBodySet,
    colliders: ColliderSet,
    broad_phase: BroadPhase,
    narrow_phase: NarrowPhase,
    pipeline: PhysicsPipeline,
    query_pipeline: QueryPipeline,
}

impl PhysicsWorld {
    fn step(&mut self, params: IntegrationParameters) -> Vec<ContactEvent> {
        self.pipeline.step(
            &params.gravity,
            &params,
            &mut self.bodies,
            &mut self.colliders,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            None,
            None,
            &(),
        );
        
        // Manually collect contacts
        self.narrow_phase.contact_events().to_vec()
    }
}
```

### After (framealloc Integration)

```rust
use framealloc::{SmartAlloc, rapier::PhysicsWorld2D};

struct PhysicsWorld {
    inner: PhysicsWorld2D,
}

impl PhysicsWorld {
    fn step(&mut self, alloc: &SmartAlloc) -> PhysicsEvents2D {
        // Automatic contact collection with frame allocation
        self.inner.step_with_events(alloc)
    }
}
```

### Key Changes

1. **No manual contact collection** - `step_with_events` handles it
2. **Automatic memory management** - No need to manage contact buffers
3. **Simplified API** - Single struct instead of multiple components
4. **Frame-aware** - Integrates with frame boundaries

## Best Practices

### Do's

- ✅ Use `step_with_events()` for contact collection
- ✅ Batch process contacts and queries
- ✅ Use frame allocation for temporary physics data
- ✅ Pool frequently reused physics objects
- ✅ Profile with realistic scene sizes

### Don'ts

- ❌ Store frame-allocated physics data across frames
- ❌ Mix raw Rapier types with framealloc wrappers
- ❌ Forget to call `begin_frame()`/`end_frame()`
- ❌ Use old Rapier APIs (pre-0.31)

### Performance Tips

1. **Batch Operations** - Process multiple contacts/queries together
2. **Spatial Queries** - Use AABB queries to limit work
3. **Object Pooling** - Reuse physics objects when possible
4. **Frame Budgeting** - Set limits for physics memory usage

## Troubleshooting

### "QueryFilter not found"

```rust
// Old (pre-0.31)
use rapier2d::geometry::QueryFilter;

// New (0.31+)
use rapier2d::pipeline::QueryFilter;
```

### "step method not found"

```rust
// Use the new method name
let events = physics.step_with_events(&alloc);
```

### Run cargo-fa lints

```bash
# Check for Rapier integration issues
cargo fa --all

# Specific Rapier lints
cargo fa explain FA901  # Wrong QueryFilter import
cargo fa explain FA902  # Old BroadPhase usage
cargo fa explain FA903  # Should use step_with_events
cargo fa explain FA904  # Ray casting without step
cargo fa explain FA905  # Old frame_alloc_slice usage
```

## Further Reading

- [Getting Started](getting-started.md) - Basic framealloc concepts
- [Performance Guide](performance.md) - Optimization techniques
- [Cookbook](cookbook.md) - More physics recipes

Happy physics simulation! 🚀
