#[cfg(feature = "rapier")]
mod tests {
    use framealloc::{SmartAlloc, rapier::rapier2d::{PhysicsWorld2D, PhysicsFrameAlloc}};
    use rapier2d::dynamics::{RigidBodyBuilder};
    use rapier2d::geometry::{ColliderBuilder};
    use rapier2d::na::Vector2;
    
    #[test]
    fn test_rapier2d_basic_integration() {
        let mut world = PhysicsWorld2D::new();
        let alloc = SmartAlloc::new(framealloc::AllocConfig::default());
        
        // Create a simple physics world with a ground and falling box
        let ground = RigidBodyBuilder::fixed();
        let ground_collider = ColliderBuilder::cuboid(10.0, 1.0);
        
        let falling = RigidBodyBuilder::dynamic();
        let falling_collider = ColliderBuilder::cuboid(1.0, 1.0);
        
        // Insert bodies
        let (ground_handle, _) = world.insert_body(ground, ground_collider, &alloc);
        let (falling_handle, _) = world.insert_body(falling, falling_collider, &alloc);
        
        // Step physics a few times
        for _ in 0..5 {
            world.step();
        }
        
        // Verify bodies exist
        assert!(world.bodies().contains(ground_handle));
        assert!(world.bodies().contains(falling_handle));
        
        // Test frame-allocated event collection
        let events = world.step_with_events(&alloc);
        assert_eq!(events.contacts.len(), 0); // No contacts yet
        assert_eq!(events.proximities.len(), 0);
    }
    
    #[test]
    fn test_rapier2d_ray_casting() {
        let mut world = PhysicsWorld2D::new();
        let alloc = SmartAlloc::new(framealloc::AllocConfig::default());
        
        // Create a ground plane
        let ground = RigidBodyBuilder::fixed();
        let ground_collider = ColliderBuilder::cuboid(10.0, 1.0);
        world.insert_body(ground, ground_collider, &alloc);
        
        // Step physics once to update the broad phase
        world.step();
        
        // Cast a ray downward
        let ray = rapier2d::geometry::Ray::new(
            rapier2d::na::Point2::new(0.0, 5.0),
            Vector2::new(0.0, -1.0)
        );
        
        let filter = rapier2d::pipeline::QueryFilter::default();
        let hits = world.cast_ray(&ray, 100.0, true, &filter, &alloc);
        
        // Should hit the ground
        assert_eq!(hits.len(), 1);
        assert!(hits[0].time_of_impact > 0.0);
        assert!(hits[0].time_of_impact < 5.0);
    }
    
    #[test]
    fn test_rapier2d_frame_allocation() {
        let mut world = PhysicsWorld2D::new();
        let alloc = SmartAlloc::new(framealloc::AllocConfig::default());
        
        // Test that frame allocation works correctly
        alloc.begin_frame();
        
        // Step with events to trigger frame allocation
        let events = world.step_with_events(&alloc);
        
        // Events should be valid for the frame
        assert_eq!(events.contacts.len(), 0);
        assert_eq!(events.proximities.len(), 0);
        
        // End frame should clean up
        alloc.end_frame();
        
        // New frame should work again
        alloc.begin_frame();
        let events2 = world.step_with_events(&alloc);
        assert_eq!(events2.contacts.len(), 0);
        alloc.end_frame();
    }
}

#[cfg(feature = "rapier")]
mod tests3d {
    use framealloc::{SmartAlloc, rapier::rapier3d::{PhysicsWorld3D, PhysicsFrameAlloc}};
    use rapier3d::dynamics::{RigidBodyBuilder};
    use rapier3d::geometry::{ColliderBuilder};
    use rapier3d::na::Vector3;
    
    #[test]
    fn test_rapier3d_basic_integration() {
        let mut world = PhysicsWorld3D::new();
        let alloc = SmartAlloc::new(framealloc::AllocConfig::default());
        
        // Create a simple physics world with a ground and falling box
        let ground = RigidBodyBuilder::fixed();
        let ground_collider = ColliderBuilder::cuboid(10.0, 1.0, 10.0);
        
        let falling = RigidBodyBuilder::dynamic();
        let falling_collider = ColliderBuilder::cuboid(1.0, 1.0, 1.0);
        
        // Insert bodies
        let (ground_handle, _) = world.insert_body(ground, ground_collider, &alloc);
        let (falling_handle, _) = world.insert_body(falling, falling_collider, &alloc);
        
        // Step physics a few times
        for _ in 0..5 {
            world.step();
        }
        
        // Verify bodies exist
        assert!(world.bodies().contains(ground_handle));
        assert!(world.bodies().contains(falling_handle));
        
        // Test frame-allocated event collection
        let events = world.step_with_events(&alloc);
        assert_eq!(events.contacts.len(), 0); // No contacts yet
        assert_eq!(events.proximities.len(), 0);
    }
    
    #[test]
    fn test_rapier3d_ray_casting() {
        let mut world = PhysicsWorld3D::new();
        let alloc = SmartAlloc::new(framealloc::AllocConfig::default());
        
        // Create a ground plane
        let ground = RigidBodyBuilder::fixed();
        let ground_collider = ColliderBuilder::cuboid(10.0, 1.0, 10.0);
        world.insert_body(ground, ground_collider, &alloc);
        
        // Step physics once to update the broad phase
        world.step();
        
        // Cast a ray downward
        let ray = rapier3d::geometry::Ray::new(
            rapier3d::na::Point3::new(0.0, 5.0, 0.0),
            Vector3::new(0.0, -1.0, 0.0)
        );
        
        let filter = rapier3d::pipeline::QueryFilter::default();
        let hits = world.cast_ray(&ray, 100.0, true, &filter, &alloc);
        
        // Should hit the ground
        assert_eq!(hits.len(), 1);
        assert!(hits[0].time_of_impact > 0.0);
        assert!(hits[0].time_of_impact < 5.0);
    }
}
