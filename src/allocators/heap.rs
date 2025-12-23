//! System heap wrapper for large allocations.

use std::alloc::{alloc, dealloc, Layout};
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::sync::mutex::Mutex;

/// Wrapper around the system allocator for large allocations.
///
/// Uses a mutex for thread safety - this path is rare.
pub struct SystemHeap {
    /// Lock for allocation operations
    lock: Mutex<()>,

    /// Total bytes currently allocated
    allocated_bytes: AtomicUsize,

    /// Total allocation count
    allocation_count: AtomicUsize,
}

impl SystemHeap {
    /// Create a new system heap wrapper.
    pub fn new() -> Self {
        Self {
            lock: Mutex::new(()),
            allocated_bytes: AtomicUsize::new(0),
            allocation_count: AtomicUsize::new(0),
        }
    }

    /// Allocate memory with the given layout.
    pub fn alloc(&self, layout: Layout) -> *mut u8 {
        let _guard = self.lock.lock();

        // SAFETY: Using system allocator with valid layout
        let ptr = unsafe { alloc(layout) };

        if !ptr.is_null() {
            self.allocated_bytes.fetch_add(layout.size(), Ordering::Relaxed);
            self.allocation_count.fetch_add(1, Ordering::Relaxed);
        }

        ptr
    }

    /// Deallocate memory.
    ///
    /// # Safety
    ///
    /// The pointer must have been allocated by this heap with the same layout.
    pub unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let _guard = self.lock.lock();

        // Poison memory before freeing in debug mode
        #[cfg(feature = "debug")]
        {
            crate::debug::poison::poison_freed(ptr, layout.size());
        }

        dealloc(ptr, layout);

        self.allocated_bytes.fetch_sub(layout.size(), Ordering::Relaxed);
    }

    /// Allocate memory for a type T.
    pub fn alloc_typed<T>(&self) -> *mut T {
        let layout = Layout::new::<T>();
        self.alloc(layout) as *mut T
    }

    /// Deallocate memory for a type T.
    ///
    /// # Safety
    ///
    /// The pointer must have been allocated by `alloc_typed::<T>()`.
    pub unsafe fn dealloc_typed<T>(&self, ptr: *mut T) {
        let layout = Layout::new::<T>();
        self.dealloc(ptr as *mut u8, layout);
    }

    /// Get total bytes currently allocated.
    pub fn allocated_bytes(&self) -> usize {
        self.allocated_bytes.load(Ordering::Relaxed)
    }

    /// Get total allocation count.
    pub fn allocation_count(&self) -> usize {
        self.allocation_count.load(Ordering::Relaxed)
    }
}

impl Default for SystemHeap {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heap_allocation() {
        let heap = SystemHeap::new();

        let ptr = heap.alloc_typed::<u64>();
        assert!(!ptr.is_null());

        assert_eq!(heap.allocated_bytes(), std::mem::size_of::<u64>());

        unsafe {
            heap.dealloc_typed(ptr);
        }

        assert_eq!(heap.allocated_bytes(), 0);
    }
}
