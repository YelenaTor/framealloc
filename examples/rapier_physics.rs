//! Example of framealloc + Rapier physics integration.
//!
//! Demonstrates high-performance physics simulation with frame-allocated temporary data.

#[cfg(feature = "rapier2d")]
fn main_2d() {
    use framealloc::{SmartAlloc, rapier::PhysicsWorld2D};
    use rapier2d::dynamics::{RigidBodyBuilder, RigidBodyType};
    use rapier2d::geometry::{ColliderBuilder, Ball};
    use rapier2d::math::Vector;

    let alloc = SmartAlloc::new(Default::default());
    let mut physics = PhysicsWorld2D::new(Vector::y() * -9.81);

    // Create some falling balls
    for i in 0..10 {
        alloc.begin_frame();
        
        let body = physics.insert_body(
            RigidBodyBuilder::dynamic()
                .translation(i as f32 * 2.0, 10.0 + i as f32),
            ColliderBuilder::ball(0.5),
            &alloc,
        );
        
        // Step physics
        let events = physics.step(&alloc);
        
        // Process events (valid until end_frame)
        for contact in events.contacts {
            println!("Contact between {:?}", contact);
        }
        
        alloc.end_frame();
    }
}

#[cfg(feature = "rapier3d")]
fn main_3d() {
    use framealloc::{SmartAlloc, rapier::PhysicsWorld3D};
    use rapier3d::dynamics::{RigidBodyBuilder, RigidBodyType};
    use rapier3d::geometry::{ColliderBuilder, Ball};
    use rapier3d::math::Vector;

    let alloc = SmartAlloc::new(Default::default());
    let mut physics = PhysicsWorld3D::new(Vector::y() * -9.81);

    // Create ground
    alloc.begin_frame();
    
    let ground = physics.insert_body(
        RigidBodyBuilder::static()
            .translation(0.0, -5.0, 0.0),
        ColliderBuilder::cuboid(20.0, 1.0, 20.0),
        &alloc,
    );
    
    // Create falling spheres
    for i in 0..5 {
        for j in 0..5 {
            let body = physics.insert_body(
                RigidBodyBuilder::dynamic()
                    .translation(
                        (i - 2) as f32 * 2.0,
                        5.0 + j as f32 * 2.0,
                        0.0,
                    ),
                ColliderBuilder::ball(0.5),
                &alloc,
            );
        }
    }
    
    // Simulate for 60 frames
    for frame in 0..60 {
        let events = physics.step(&alloc);
        
        // Process collision events
        if frame == 0 && !events.contacts.is_empty() {
            println!("First collision at frame {}", frame);
        }
    }
    
    alloc.end_frame();
}

fn main() {
    #[cfg(feature = "rapier2d")]
    {
        println!("Running 2D physics example...");
        main_2d();
    }
    
    #[cfg(feature = "rapier3d")]
    {
        println!("Running 3D physics example...");
        main_3d();
    }
    
    #[cfg(not(any(feature = "rapier2d", feature = "rapier3d")))]
    {
        println!("Enable rapier2d or rapier3d feature to run this example:");
        println!("cargo run --example rapier_physics --features rapier2d");
        println!("cargo run --example rapier_physics --features rapier3d");
    }
}
