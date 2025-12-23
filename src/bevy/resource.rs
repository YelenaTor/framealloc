//! Bevy resource wrapper for the allocator.

use bevy_ecs::system::Resource;

use crate::api::alloc::SmartAlloc;
use crate::api::stats::AllocStats;

/// Bevy resource that wraps the smart allocator.
///
/// This allows systems to access the allocator via `Res<AllocResource>`.
///
/// # Example
///
/// ```rust,ignore
/// fn my_system(alloc: Res<AllocResource>) {
///     let temp = alloc.frame_alloc::<[f32; 256]>();
///     // Use temp...
/// }
/// ```
#[derive(Resource, Clone)]
pub struct AllocResource(pub SmartAlloc);

impl AllocResource {
    /// Create a new allocator resource.
    pub fn new(alloc: SmartAlloc) -> Self {
        Self(alloc)
    }

    /// Allocate from the frame arena.
    ///
    /// Memory is valid until end of frame.
    pub fn frame_alloc<T>(&self) -> *mut T {
        self.0.frame_alloc::<T>()
    }

    /// Allocate from the small object pool.
    pub fn pool_alloc<T>(&self) -> *mut T {
        self.0.pool_alloc::<T>()
    }

    /// Free to the small object pool.
    ///
    /// # Safety
    ///
    /// Pointer must have been allocated with `pool_alloc`.
    pub unsafe fn pool_free<T>(&self, ptr: *mut T) {
        self.0.pool_free(ptr);
    }

    /// Allocate from the system heap.
    pub fn heap_alloc<T>(&self) -> *mut T {
        self.0.heap_alloc::<T>()
    }

    /// Free to the system heap.
    ///
    /// # Safety
    ///
    /// Pointer must have been allocated with `heap_alloc`.
    pub unsafe fn heap_free<T>(&self, ptr: *mut T) {
        self.0.heap_free(ptr);
    }

    /// Get current allocation statistics.
    pub fn stats(&self) -> AllocStats {
        self.0.stats()
    }
}

impl std::ops::Deref for AllocResource {
    type Target = SmartAlloc;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
