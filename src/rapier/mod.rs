//! Rapier physics engine integration with framealloc.
//!
//! Provides utilities to use framealloc with Rapier physics for
//! high-performance bulk allocations of physics data.

#[cfg(feature = "rapier")]
pub mod rapier2d;

#[cfg(feature = "rapier")]
pub mod rapier3d;

#[cfg(feature = "rapier")]
pub use rapier2d::{PhysicsWorld2D, PhysicsEvents2D};

#[cfg(feature = "rapier")]
pub use rapier3d::{PhysicsWorld3D, PhysicsEvents3D};
