//! Frame arena - bump allocator for frame-temporary allocations.
//!
//! This is the hot path for most game allocations.
//! No locks, no atomics - just pointer bumping.

use std::alloc::{alloc, dealloc, Layout};
use std::ptr::NonNull;

/// A bump allocator for frame-temporary allocations.
///
/// Allocations are extremely fast (just pointer increment).
/// All allocations are invalidated on `reset()`.
pub struct FrameArena {
    /// Base pointer of the arena
    base: NonNull<u8>,

    /// Current allocation head (offset from base)
    head: usize,

    /// Total capacity in bytes
    capacity: usize,
}

impl FrameArena {
    /// Create a new frame arena with the given capacity.
    pub fn new(capacity: usize) -> Self {
        let layout = Layout::from_size_align(capacity, 16).expect("Invalid arena layout");

        // SAFETY: We're allocating a block of memory with proper alignment
        let ptr = unsafe { alloc(layout) };

        let base = NonNull::new(ptr).expect("Failed to allocate frame arena");

        Self {
            base,
            head: 0,
            capacity,
        }
    }

    /// Allocate memory for a value of type T.
    ///
    /// Returns null if the arena is exhausted.
    pub fn alloc<T>(&mut self) -> *mut T {
        self.alloc_layout(Layout::new::<T>()) as *mut T
    }

    /// Allocate memory with a specific layout.
    ///
    /// Returns null if the arena is exhausted.
    pub fn alloc_layout(&mut self, layout: Layout) -> *mut u8 {
        let align = layout.align();
        let size = layout.size();

        // Align the head
        let aligned_head = (self.head + align - 1) & !(align - 1);

        // Check if we have enough space
        if aligned_head + size > self.capacity {
            return std::ptr::null_mut();
        }

        // SAFETY: We've verified the allocation fits within our arena
        let ptr = unsafe { self.base.as_ptr().add(aligned_head) };

        self.head = aligned_head + size;

        ptr
    }

    /// Allocate a slice of T with the given count.
    pub fn alloc_slice<T>(&mut self, count: usize) -> *mut T {
        let layout = Layout::array::<T>(count).expect("Invalid array layout");
        self.alloc_layout(layout) as *mut T
    }

    /// Get current head position (for scope save/restore).
    pub fn head(&self) -> usize {
        self.head
    }

    /// Reset the arena, invalidating all allocations.
    pub fn reset(&mut self) {
        self.head = 0;

        // Optionally poison memory in debug mode
        #[cfg(feature = "debug")]
        unsafe {
            std::ptr::write_bytes(self.base.as_ptr(), 0xCD, self.capacity);
        }
    }

    /// Reset to a previously saved head position.
    pub fn reset_to(&mut self, head: usize) {
        debug_assert!(head <= self.head, "Cannot reset forward");
        self.head = head;
    }

    /// Get remaining capacity.
    pub fn remaining(&self) -> usize {
        self.capacity - self.head
    }

    /// Get total capacity.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Get bytes currently allocated.
    pub fn allocated(&self) -> usize {
        self.head
    }
}

impl Drop for FrameArena {
    fn drop(&mut self) {
        let layout = Layout::from_size_align(self.capacity, 16).expect("Invalid arena layout");

        // SAFETY: We allocated this memory in `new()`
        unsafe {
            dealloc(self.base.as_ptr(), layout);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_allocation() {
        let mut arena = FrameArena::new(1024);

        let ptr1 = arena.alloc::<u32>();
        assert!(!ptr1.is_null());

        let ptr2 = arena.alloc::<u64>();
        assert!(!ptr2.is_null());

        // Pointers should be different
        assert_ne!(ptr1 as *mut u8, ptr2 as *mut u8);
    }

    #[test]
    fn test_reset() {
        let mut arena = FrameArena::new(1024);

        let ptr1 = arena.alloc::<u32>();
        let _head_before = arena.head();

        arena.reset();

        assert_eq!(arena.head(), 0);

        // New allocation should reuse the same memory
        let ptr2 = arena.alloc::<u32>();
        assert_eq!(ptr1, ptr2);
    }

    #[test]
    fn test_exhaustion() {
        let mut arena = FrameArena::new(32);

        // Allocate until exhausted
        let _ = arena.alloc::<[u8; 16]>();
        let _ = arena.alloc::<[u8; 16]>();

        // This should fail
        let ptr = arena.alloc::<[u8; 16]>();
        assert!(ptr.is_null());
    }
}
