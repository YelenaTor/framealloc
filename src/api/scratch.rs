//! Scratch pools - cross-frame reusable memory pools.
//!
//! Scratch pools bridge frame and pool semantics. They're like frame arenas
//! but live longer - cleared manually, on level unload, or via groups.

use std::alloc::{alloc, dealloc, Layout};
use std::collections::HashMap;
use std::ptr::NonNull;

use crate::sync::mutex::Mutex;

/// A named scratch pool for cross-frame temporary allocations.
///
/// Scratch pools are useful for memory that:
/// - Lives longer than a frame
/// - But is still scratch-like (bulk freed)
/// - Is associated with a subsystem or task
pub struct ScratchPool {
    /// Name of this pool
    name: &'static str,
    /// Base pointer
    base: NonNull<u8>,
    /// Current head
    head: usize,
    /// Capacity
    capacity: usize,
}

impl ScratchPool {
    /// Create a new scratch pool.
    pub fn new(name: &'static str, capacity: usize) -> Self {
        let layout = Layout::from_size_align(capacity, 16).expect("Invalid layout");
        let ptr = unsafe { alloc(layout) };
        let base = NonNull::new(ptr).expect("Failed to allocate scratch pool");

        Self {
            name,
            base,
            head: 0,
            capacity,
        }
    }

    /// Get the pool name.
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// Allocate memory from this pool.
    pub fn alloc<T>(&mut self) -> *mut T {
        self.alloc_layout(Layout::new::<T>()) as *mut T
    }

    /// Allocate with a specific layout.
    pub fn alloc_layout(&mut self, layout: Layout) -> *mut u8 {
        let align = layout.align();
        let size = layout.size();

        let aligned_head = (self.head + align - 1) & !(align - 1);

        if aligned_head + size > self.capacity {
            return std::ptr::null_mut();
        }

        let ptr = unsafe { self.base.as_ptr().add(aligned_head) };
        self.head = aligned_head + size;
        ptr
    }

    /// Allocate a slice.
    pub fn alloc_slice<T>(&mut self, count: usize) -> *mut T {
        let layout = Layout::array::<T>(count).expect("Invalid array layout");
        self.alloc_layout(layout) as *mut T
    }

    /// Reset the pool, invalidating all allocations.
    pub fn reset(&mut self) {
        self.head = 0;

        #[cfg(feature = "debug")]
        unsafe {
            std::ptr::write_bytes(self.base.as_ptr(), 0xCD, self.capacity);
        }
    }

    /// Get bytes allocated.
    pub fn allocated(&self) -> usize {
        self.head
    }

    /// Get remaining capacity.
    pub fn remaining(&self) -> usize {
        self.capacity - self.head
    }

    /// Get total capacity.
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

impl Drop for ScratchPool {
    fn drop(&mut self) {
        let layout = Layout::from_size_align(self.capacity, 16).expect("Invalid layout");
        unsafe {
            dealloc(self.base.as_ptr(), layout);
        }
    }
}

/// Registry of named scratch pools.
pub struct ScratchRegistry {
    pools: Mutex<HashMap<&'static str, ScratchPool>>,
    default_capacity: usize,
}

impl ScratchRegistry {
    /// Create a new registry.
    pub fn new(default_capacity: usize) -> Self {
        Self {
            pools: Mutex::new(HashMap::new()),
            default_capacity,
        }
    }

    /// Get or create a scratch pool by name.
    pub fn get_or_create(&self, name: &'static str) -> ScratchPoolHandle<'_> {
        let mut pools = self.pools.lock();
        if !pools.contains_key(name) {
            pools.insert(name, ScratchPool::new(name, self.default_capacity));
        }
        ScratchPoolHandle { registry: self, name }
    }

    /// Get a pool by name if it exists.
    pub fn get(&self, name: &'static str) -> Option<ScratchPoolHandle<'_>> {
        let pools = self.pools.lock();
        if pools.contains_key(name) {
            Some(ScratchPoolHandle { registry: self, name })
        } else {
            None
        }
    }

    /// Reset a pool by name.
    pub fn reset(&self, name: &'static str) {
        let mut pools = self.pools.lock();
        if let Some(pool) = pools.get_mut(name) {
            pool.reset();
        }
    }

    /// Reset all pools.
    pub fn reset_all(&self) {
        let mut pools = self.pools.lock();
        for pool in pools.values_mut() {
            pool.reset();
        }
    }

    /// Remove a pool by name.
    pub fn remove(&self, name: &'static str) {
        let mut pools = self.pools.lock();
        pools.remove(name);
    }

    /// Get stats for all pools.
    pub fn stats(&self) -> Vec<ScratchPoolStats> {
        let pools = self.pools.lock();
        pools
            .values()
            .map(|p| ScratchPoolStats {
                name: p.name,
                allocated: p.allocated(),
                capacity: p.capacity,
            })
            .collect()
    }

    /// Execute a closure with mutable access to a pool.
    pub fn with_pool<F, R>(&self, name: &'static str, f: F) -> Option<R>
    where
        F: FnOnce(&mut ScratchPool) -> R,
    {
        let mut pools = self.pools.lock();
        pools.get_mut(name).map(f)
    }
}

impl Default for ScratchRegistry {
    fn default() -> Self {
        Self::new(1024 * 1024) // 1MB default
    }
}

/// A handle to a named scratch pool.
pub struct ScratchPoolHandle<'a> {
    registry: &'a ScratchRegistry,
    name: &'static str,
}

impl<'a> ScratchPoolHandle<'a> {
    /// Get the pool name.
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// Allocate from this pool.
    pub fn alloc<T>(&self) -> *mut T {
        self.registry
            .with_pool(self.name, |p| p.alloc::<T>())
            .unwrap_or(std::ptr::null_mut())
    }

    /// Allocate a slice from this pool.
    pub fn alloc_slice<T>(&self, count: usize) -> *mut T {
        self.registry
            .with_pool(self.name, |p| p.alloc_slice::<T>(count))
            .unwrap_or(std::ptr::null_mut())
    }

    /// Reset this pool.
    pub fn reset(&self) {
        self.registry.reset(self.name);
    }

    /// Get allocated bytes.
    pub fn allocated(&self) -> usize {
        self.registry
            .with_pool(self.name, |p| p.allocated())
            .unwrap_or(0)
    }

    /// Get remaining capacity.
    pub fn remaining(&self) -> usize {
        self.registry
            .with_pool(self.name, |p| p.remaining())
            .unwrap_or(0)
    }
}

/// Statistics for a scratch pool.
#[derive(Debug, Clone)]
pub struct ScratchPoolStats {
    /// Pool name
    pub name: &'static str,
    /// Bytes currently allocated
    pub allocated: usize,
    /// Total capacity
    pub capacity: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scratch_pool() {
        let mut pool = ScratchPool::new("test", 4096);

        let ptr1 = pool.alloc::<u64>();
        assert!(!ptr1.is_null());

        let ptr2 = pool.alloc::<[u8; 1024]>();
        assert!(!ptr2.is_null());

        assert!(pool.allocated() > 0);

        pool.reset();
        assert_eq!(pool.allocated(), 0);
    }

    #[test]
    fn test_scratch_registry() {
        let registry = ScratchRegistry::new(4096);

        let handle = registry.get_or_create("pathfinding");
        let ptr = handle.alloc::<[u32; 256]>();
        assert!(!ptr.is_null());

        assert!(handle.allocated() > 0);

        handle.reset();
        assert_eq!(handle.allocated(), 0);
    }
}
