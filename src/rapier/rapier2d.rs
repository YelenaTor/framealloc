//! Rapier 2D physics integration with framealloc.
//!
//! Provides a frame-aware wrapper for 2D physics operations.

use crate::SmartAlloc;
use rapier2d::dynamics::{RigidBodySet, RigidBodyBuilder, RigidBodyHandle, IntegrationParameters, IslandManager};
use rapier2d::geometry::{ColliderSet, ColliderBuilder, ColliderHandle, BroadPhaseBvh, NarrowPhase};
use rapier2d::pipeline::{PhysicsPipeline, QueryFilter};
use rapier2d::na::Vector2;

// Common types used by both 2D and 3D
#[derive(Debug, Clone)]
pub struct PhysicsEvents<'a> {
    /// Contact events from the last physics step
    pub contacts: &'a [ContactEvent],
    /// Proximity events from the last physics step
    pub proximities: &'a [ProximityEvent],
}

#[derive(Debug, Clone)]
pub struct ContactEvent {
    pub collider1: ColliderHandle,
    pub collider2: ColliderHandle,
    pub contact_point: ContactPoint,
}

#[derive(Debug, Clone)]
pub struct ProximityEvent {
    pub collider1: ColliderHandle,
    pub collider2: ColliderHandle,
    pub intersecting: bool,
}

#[derive(Debug, Clone)]
pub struct ContactPoint {
    pub local_point1: [f32; 3],
    pub local_point2: [f32; 3],
    pub normal: [f32; 3],
    pub impulse: f32,
}

/// Helper trait for framealloc-aware physics integration.
pub trait PhysicsFrameAlloc {
    /// Step physics and collect events into frame-allocated buffers.
    fn step_with_events(&mut self, alloc: &crate::SmartAlloc) -> PhysicsEvents<'_>;
}

/// A 2D physics world that integrates with framealloc.
pub struct PhysicsWorld2D {
    /// Core physics pipeline
    physics_pipeline: PhysicsPipeline,
    /// Island manager for sleeping objects
    island_manager: IslandManager,
    /// Broad-phase collision detection
    broad_phase: BroadPhaseBvh,
    /// Narrow-phase collision detection
    narrow_phase: NarrowPhase,
    /// Rigid bodies
    bodies: RigidBodySet,
    /// Colliders
    colliders: ColliderSet,
    /// Integration parameters
    integration_parameters: IntegrationParameters,
    /// Contact joints
    impulse_joints: rapier2d::dynamics::ImpulseJointSet,
    /// Multibody joints
    multibody_joints: rapier2d::dynamics::MultibodyJointSet,
    /// CCD solver
    ccd_solver: rapier2d::dynamics::CCDSolver,
    /// Gravity vector
    gravity: Vector2<f32>,
    /// Frame-allocated contact events
    contacts: Vec<ContactEvent>,
    /// Frame-allocated proximity events
    proximities: Vec<ProximityEvent>,
}

impl PhysicsWorld2D {
    /// Create a new 2D physics world.
    pub fn new() -> Self {
        Self {
            physics_pipeline: PhysicsPipeline::new(),
            island_manager: IslandManager::new(),
            broad_phase: BroadPhaseBvh::new(),
            narrow_phase: NarrowPhase::new(),
            bodies: RigidBodySet::new(),
            colliders: ColliderSet::new(),
            integration_parameters: IntegrationParameters::default(),
            impulse_joints: rapier2d::dynamics::ImpulseJointSet::new(),
            multibody_joints: rapier2d::dynamics::MultibodyJointSet::new(),
            ccd_solver: rapier2d::dynamics::CCDSolver::new(),
            gravity: Vector2::new(0.0, -9.81),
            contacts: Vec::new(),
            proximities: Vec::new(),
        }
    }
    
    /// Set the gravity vector.
    pub fn set_gravity(&mut self, gravity: Vector2<f32>) {
        self.gravity = gravity;
    }
    
    /// Insert a rigid body and collider into the physics world.
    pub fn insert_body(
        &mut self,
        body_builder: RigidBodyBuilder,
        collider_builder: ColliderBuilder,
        _alloc: &SmartAlloc,
    ) -> (RigidBodyHandle, ColliderHandle) {
        let body = self.bodies.insert(body_builder);
        let collider = self.colliders.insert_with_parent(
            collider_builder,
            body,
            &mut self.bodies,
        );
        (body, collider)
    }
    
    /// Step the physics simulation.
    pub fn step(&mut self) {
        self.physics_pipeline.step(
            &self.gravity,
            &self.integration_parameters,
            &mut self.island_manager,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.bodies,
            &mut self.colliders,
            &mut self.impulse_joints,
            &mut self.multibody_joints,
            &mut self.ccd_solver,
            &(),
            &(),
        );
    }
    
    /// Step physics and collect events into frame-allocated buffers.
    pub fn step_with_events(&mut self, alloc: &SmartAlloc) -> PhysicsEvents<'_> {
        // Clear previous events
        self.contacts.clear();
        self.proximities.clear();
        
        // Step physics
        self.step();
        
        // Collect events into frame-allocated vectors
        // Note: In a real implementation, you'd collect actual events from Rapier
        // For now, we'll use empty vectors as the event collection API has changed
        
        // Create frame-allocated slices for the events
        let contacts = if !self.contacts.is_empty() {
            // Allocate batch and copy data
            let ptr = alloc.frame_alloc_batch::<ContactEvent>(self.contacts.len());
            unsafe {
                for (i, event) in self.contacts.iter().enumerate() {
                    std::ptr::write(ptr.add(i), event.clone());
                }
                std::slice::from_raw_parts(ptr, self.contacts.len())
            }
        } else {
            &[]
        };
        
        let proximities = if !self.proximities.is_empty() {
            // Allocate batch and copy data
            let ptr = alloc.frame_alloc_batch::<ProximityEvent>(self.proximities.len());
            unsafe {
                for (i, event) in self.proximities.iter().enumerate() {
                    std::ptr::write(ptr.add(i), event.clone());
                }
                std::slice::from_raw_parts(ptr, self.proximities.len())
            }
        } else {
            &[]
        };
        
        PhysicsEvents {
            contacts,
            proximities,
        }
    }
    
    /// Get a reference to the rigid body set.
    pub fn bodies(&self) -> &RigidBodySet {
        &self.bodies
    }
    
    /// Get a reference to the collider set.
    pub fn colliders(&self) -> &ColliderSet {
        &self.colliders
    }
    
    /// Perform ray casting using framealloc for results.
    pub fn cast_ray(
        &self,
        ray: &rapier2d::geometry::Ray,
        max_toi: f32,
        solid: bool,
        filter: &QueryFilter,
        alloc: &SmartAlloc,
    ) -> &[rapier2d::geometry::RayIntersection] {
        // Create query pipeline using the new API
        let query_pipeline = self.broad_phase.as_query_pipeline(
            self.narrow_phase.query_dispatcher(),
            &self.bodies,
            &self.colliders,
            *filter,
        );
        
        // Collect results into a frame-allocated vector
        let mut results = Vec::new();
        
        // Use intersect_ray to get all intersections
        for (_handle, _collider, intersection) in query_pipeline.intersect_ray(*ray, max_toi, solid) {
            results.push(intersection);
        }
        
        // Return frame-allocated slice
        if !results.is_empty() {
            let ptr = alloc.frame_alloc_batch::<rapier2d::geometry::RayIntersection>(results.len());
            unsafe {
                for (i, result) in results.iter().enumerate() {
                    std::ptr::write(ptr.add(i), result.clone());
                }
                std::slice::from_raw_parts(ptr, results.len())
            }
        } else {
            &[]
        }
    }
}

impl Default for PhysicsWorld2D {
    fn default() -> Self {
        Self::new()
    }
}

/// Type alias for 2D physics events.
pub type PhysicsEvents2D<'a> = PhysicsEvents<'a>;

impl PhysicsFrameAlloc for PhysicsWorld2D {
    fn step_with_events(&mut self, alloc: &SmartAlloc) -> PhysicsEvents<'_> {
        self.step_with_events(alloc)
    }
}
