//! Deferred free queue for cross-thread frees.
//!
//! When thread A frees memory that was allocated by thread B,
//! the free is queued and processed by thread B on its next allocation.

use crossbeam_queue::SegQueue;

use crate::allocators::slab::LocalPools;

/// A pending deferred free.
struct DeferredFree {
    ptr: *mut u8,
    size: usize,
}

// SAFETY: We're transferring ownership of the pointer across threads
unsafe impl Send for DeferredFree {}

/// Lock-free queue for deferred frees.
pub struct DeferredFreeQueue {
    queue: SegQueue<DeferredFree>,
}

impl DeferredFreeQueue {
    /// Create a new deferred free queue.
    pub fn new() -> Self {
        Self {
            queue: SegQueue::new(),
        }
    }

    /// Push a deferred free onto the queue.
    ///
    /// Called by a thread freeing memory it didn't allocate.
    pub fn push(&self, ptr: *mut u8, size: usize) {
        self.queue.push(DeferredFree { ptr, size });
    }

    /// Drain all pending frees into the local pools.
    ///
    /// Called by the owning thread to reclaim its memory.
    pub fn drain(&self, pools: &mut LocalPools) {
        while let Some(deferred) = self.queue.pop() {
            pools.drain_deferred(deferred.ptr, deferred.size);
        }
    }

    /// Check if there are pending frees.
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Get approximate number of pending frees.
    pub fn len(&self) -> usize {
        self.queue.len()
    }
}

impl Default for DeferredFreeQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deferred_queue() {
        let queue = DeferredFreeQueue::new();

        assert!(queue.is_empty());

        // Simulate pushing a free
        let ptr = Box::into_raw(Box::new(42u32)) as *mut u8;
        queue.push(ptr, 4);

        assert!(!queue.is_empty());
        assert_eq!(queue.len(), 1);

        // Clean up (normally done by drain)
        while let Some(deferred) = queue.queue.pop() {
            unsafe {
                let _ = Box::from_raw(deferred.ptr as *mut u32);
            }
        }
    }
}
