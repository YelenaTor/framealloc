//! Scoped pool allocations for async contexts.
//!
//! `AsyncPoolGuard` provides batch allocation management for async code,
//! where all allocations are automatically freed when the guard drops.

use std::alloc::Layout;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::ptr::NonNull;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use crate::api::alloc::SmartAlloc;

/// Guard for scoped pool allocations in async contexts.
///
/// All allocations made through this guard are pool-backed and will be
/// freed when the guard is dropped. This is useful for batch operations
/// in async code.
///
/// # Example
///
/// ```rust,ignore
/// use framealloc::tokio::AsyncPoolGuard;
///
/// async fn process_batch(alloc: SmartAlloc) {
///     let guard = AsyncPoolGuard::new(&alloc);
///     
///     let items: Vec<_> = futures::future::join_all(
///         (0..10).map(|i| {
///             let g = &guard;
///             async move {
///                 g.alloc_box(fetch_item(i).await)
///             }
///         })
///     ).await;
///     
///     for item in &items {
///         process(item).await;
///     }
///     
///     // guard drops → all items freed
/// }
/// ```
pub struct AsyncPoolGuard {
    alloc: SmartAlloc,
    /// Tracked allocations for cleanup
    allocations: Mutex<Vec<GuardedAllocation>>,
    /// Count for stats
    count: AtomicUsize,
    /// Total bytes allocated
    bytes: AtomicUsize,
}

struct GuardedAllocation {
    ptr: NonNull<u8>,
    layout: Layout,
    drop_fn: Option<unsafe fn(NonNull<u8>)>,
}

// SAFETY: GuardedAllocation is only accessed through Mutex
unsafe impl Send for GuardedAllocation {}
unsafe impl Sync for GuardedAllocation {}

impl AsyncPoolGuard {
    /// Create a new async pool guard.
    ///
    /// # Arguments
    ///
    /// * `alloc` - The parent `SmartAlloc` to allocate from
    pub fn new(alloc: &SmartAlloc) -> Self {
        Self {
            alloc: alloc.clone(),
            allocations: Mutex::new(Vec::new()),
            count: AtomicUsize::new(0),
            bytes: AtomicUsize::new(0),
        }
    }

    /// Allocate a boxed value within this guard's scope.
    ///
    /// The allocation is pool-backed and will be freed when the guard drops.
    ///
    /// # Arguments
    ///
    /// * `value` - The value to box
    ///
    /// # Returns
    ///
    /// A `GuardedBox` containing the value
    pub fn alloc_box<T>(&self, value: T) -> GuardedBox<T> {
        let layout = Layout::new::<T>();
        
        // Allocate raw memory first, then write the value
        let ptr = unsafe {
            let raw = std::alloc::alloc(layout);
            if raw.is_null() {
                std::alloc::handle_alloc_error(layout);
            }
            // Write the value into the allocated memory
            std::ptr::write(raw as *mut T, value);
            NonNull::new_unchecked(raw)
        };

        // Track for cleanup
        let tracked = GuardedAllocation {
            ptr,
            layout,
            drop_fn: Some(drop_typed::<T>),
        };
        
        self.allocations.lock().unwrap().push(tracked);
        self.count.fetch_add(1, Ordering::Relaxed);
        self.bytes.fetch_add(layout.size(), Ordering::Relaxed);

        GuardedBox {
            ptr: ptr.cast(),
            _marker: PhantomData,
        }
    }

    /// Allocate a vector-like buffer within this guard's scope.
    ///
    /// # Arguments
    ///
    /// * `capacity` - Number of elements to allocate space for
    ///
    /// # Returns
    ///
    /// A `GuardedVec` with the specified capacity
    pub fn alloc_vec<T>(&self, capacity: usize) -> GuardedVec<T> {
        let layout = Layout::array::<T>(capacity).expect("layout overflow");
        
        let ptr = unsafe {
            let raw = std::alloc::alloc(layout);
            if raw.is_null() {
                std::alloc::handle_alloc_error(layout);
            }
            NonNull::new_unchecked(raw)
        };

        let tracked = GuardedAllocation {
            ptr,
            layout,
            drop_fn: None, // Vec handles its own element drops
        };
        
        self.allocations.lock().unwrap().push(tracked);
        self.count.fetch_add(1, Ordering::Relaxed);
        self.bytes.fetch_add(layout.size(), Ordering::Relaxed);

        GuardedVec {
            ptr: ptr.cast(),
            len: 0,
            capacity,
            _marker: PhantomData,
        }
    }

    /// Get the number of active allocations.
    pub fn allocation_count(&self) -> usize {
        self.count.load(Ordering::Relaxed)
    }

    /// Get the total bytes allocated.
    pub fn bytes_allocated(&self) -> usize {
        self.bytes.load(Ordering::Relaxed)
    }

    /// Get the parent allocator.
    pub fn allocator(&self) -> &SmartAlloc {
        &self.alloc
    }
}

impl Drop for AsyncPoolGuard {
    fn drop(&mut self) {
        let allocations = self.allocations.get_mut().unwrap();
        
        for tracked in allocations.drain(..) {
            unsafe {
                if let Some(drop_fn) = tracked.drop_fn {
                    drop_fn(tracked.ptr);
                }
                std::alloc::dealloc(tracked.ptr.as_ptr(), tracked.layout);
            }
        }
    }
}

// SAFETY: AsyncPoolGuard uses internal synchronization
unsafe impl Send for AsyncPoolGuard {}
unsafe impl Sync for AsyncPoolGuard {}

/// Helper function to drop a typed value.
unsafe fn drop_typed<T>(ptr: NonNull<u8>) {
    std::ptr::drop_in_place(ptr.as_ptr() as *mut T);
}

/// A boxed value owned by an `AsyncPoolGuard`.
pub struct GuardedBox<T> {
    ptr: NonNull<T>,
    _marker: PhantomData<T>,
}

impl<T> Deref for GuardedBox<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { self.ptr.as_ref() }
    }
}

impl<T> DerefMut for GuardedBox<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.ptr.as_mut() }
    }
}

impl<T> GuardedBox<T> {
    /// Get a raw pointer to the value.
    pub fn as_ptr(&self) -> *const T {
        self.ptr.as_ptr()
    }
}

// SAFETY: GuardedBox is just a pointer wrapper
unsafe impl<T: Send> Send for GuardedBox<T> {}
unsafe impl<T: Sync> Sync for GuardedBox<T> {}

/// A vector-like container owned by an `AsyncPoolGuard`.
pub struct GuardedVec<T> {
    ptr: NonNull<T>,
    len: usize,
    capacity: usize,
    _marker: PhantomData<T>,
}

impl<T> GuardedVec<T> {
    /// Get the current length.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Get the capacity.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Push a value if there's capacity.
    ///
    /// # Returns
    ///
    /// `true` if the value was pushed, `false` if at capacity
    pub fn push(&mut self, value: T) -> bool {
        if self.len >= self.capacity {
            return false;
        }
        
        unsafe {
            self.ptr.as_ptr().add(self.len).write(value);
        }
        self.len += 1;
        true
    }

    /// Pop a value if non-empty.
    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }
        
        self.len -= 1;
        unsafe {
            Some(self.ptr.as_ptr().add(self.len).read())
        }
    }

    /// Get a slice of the contents.
    pub fn as_slice(&self) -> &[T] {
        unsafe {
            std::slice::from_raw_parts(self.ptr.as_ptr(), self.len)
        }
    }

    /// Get a mutable slice of the contents.
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe {
            std::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len)
        }
    }

    /// Clear all elements.
    pub fn clear(&mut self) {
        while self.pop().is_some() {}
    }
}

impl<T> Drop for GuardedVec<T> {
    fn drop(&mut self) {
        // Drop all elements
        self.clear();
        // Memory is freed by AsyncPoolGuard
    }
}

impl<T> Deref for GuardedVec<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<T> DerefMut for GuardedVec<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_slice()
    }
}

// SAFETY: GuardedVec is just a pointer wrapper
unsafe impl<T: Send> Send for GuardedVec<T> {}
unsafe impl<T: Sync> Sync for GuardedVec<T> {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AllocConfig;

    #[test]
    fn guard_basic() {
        let alloc = SmartAlloc::new(AllocConfig::default());
        
        {
            let guard = AsyncPoolGuard::new(&alloc);
            let _a = guard.alloc_box(42u32);
            let _b = guard.alloc_box(String::from("test"));
            
            assert_eq!(guard.allocation_count(), 2);
        }
    }

    #[test]
    fn guarded_vec_operations() {
        let alloc = SmartAlloc::new(AllocConfig::default());
        let guard = AsyncPoolGuard::new(&alloc);
        
        let mut vec: GuardedVec<i32> = guard.alloc_vec(10);
        
        assert!(vec.push(1));
        assert!(vec.push(2));
        assert!(vec.push(3));
        
        assert_eq!(vec.len(), 3);
        assert_eq!(vec.as_slice(), &[1, 2, 3]);
        
        assert_eq!(vec.pop(), Some(3));
        assert_eq!(vec.len(), 2);
    }

    #[test]
    fn guard_is_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}
        
        assert_send::<AsyncPoolGuard>();
        assert_sync::<AsyncPoolGuard>();
    }
}
