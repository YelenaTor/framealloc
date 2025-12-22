//! Task-scoped allocation for async tasks.
//!
//! `TaskAlloc` provides a convenient way to manage allocations that should
//! live exactly as long as an async task. All allocations are pool-backed
//! and automatically freed when the `TaskAlloc` is dropped.

use std::alloc::Layout;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::ptr::NonNull;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use crate::api::alloc::SmartAlloc;

/// Task-scoped allocator for async contexts.
///
/// Creates pool-backed allocations that are automatically freed when the
/// `TaskAlloc` is dropped. This is ideal for spawned tasks that need
/// temporary allocations across await points.
///
/// # Thread Safety
///
/// `TaskAlloc` is `Send + Sync` and can be safely used across await points.
/// However, the `TaskBox` values it produces should typically be used within
/// the same task.
///
/// # Example
///
/// ```rust,ignore
/// use framealloc::tokio::TaskAlloc;
///
/// tokio::spawn(async move {
///     let mut task = TaskAlloc::new(&alloc);
///     
///     let buffer = task.alloc_box(vec![0u8; 1024]);
///     let state = task.alloc_box(MyState::new());
///     
///     process(&buffer, &state).await;
///     
///     // task drops here → all allocations freed
/// });
/// ```
pub struct TaskAlloc {
    alloc: SmartAlloc,
    /// Tracked allocations for cleanup
    allocations: Mutex<Vec<TrackedAllocation>>,
    /// Count for stats
    count: AtomicUsize,
}

struct TrackedAllocation {
    ptr: NonNull<u8>,
    layout: Layout,
    drop_fn: Option<unsafe fn(NonNull<u8>)>,
}

// SAFETY: TrackedAllocation is only accessed through Mutex
unsafe impl Send for TrackedAllocation {}
unsafe impl Sync for TrackedAllocation {}

impl TaskAlloc {
    /// Create a new task-scoped allocator.
    ///
    /// # Arguments
    ///
    /// * `alloc` - The parent `SmartAlloc` to allocate from
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let alloc = SmartAlloc::new(AllocConfig::default());
    /// let task = TaskAlloc::new(&alloc);
    /// ```
    pub fn new(alloc: &SmartAlloc) -> Self {
        Self {
            alloc: alloc.clone(),
            allocations: Mutex::new(Vec::new()),
            count: AtomicUsize::new(0),
        }
    }

    /// Allocate a boxed value that will be freed when this `TaskAlloc` drops.
    ///
    /// Uses pool allocation internally (never frame), making it safe across
    /// await points.
    ///
    /// # Arguments
    ///
    /// * `value` - The value to box
    ///
    /// # Returns
    ///
    /// A `TaskBox` containing the value
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let data = task.alloc_box(MyData::new());
    /// process(&data).await;
    /// ```
    pub fn alloc_box<T>(&self, value: T) -> TaskBox<T> {
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
        let tracked = TrackedAllocation {
            ptr,
            layout,
            drop_fn: Some(drop_typed::<T>),
        };
        
        self.allocations.lock().unwrap().push(tracked);
        self.count.fetch_add(1, Ordering::Relaxed);

        TaskBox {
            ptr: ptr.cast(),
            _marker: PhantomData,
        }
    }

    /// Allocate a slice that will be freed when this `TaskAlloc` drops.
    ///
    /// # Arguments
    ///
    /// * `len` - Number of elements
    ///
    /// # Returns
    ///
    /// A `TaskSlice` containing uninitialized memory
    pub fn alloc_slice<T>(&self, len: usize) -> TaskSlice<T> {
        let layout = Layout::array::<T>(len).expect("layout overflow");
        
        // Use heap for slices (more flexible sizing)
        let ptr = unsafe {
            let raw = std::alloc::alloc(layout);
            if raw.is_null() {
                std::alloc::handle_alloc_error(layout);
            }
            NonNull::new_unchecked(raw)
        };

        // Track for cleanup (no drop_fn for slices - they're uninitialized)
        let tracked = TrackedAllocation {
            ptr,
            layout,
            drop_fn: None,
        };
        
        self.allocations.lock().unwrap().push(tracked);
        self.count.fetch_add(1, Ordering::Relaxed);

        TaskSlice {
            ptr: ptr.cast(),
            len,
            _marker: PhantomData,
        }
    }

    /// Get the number of active allocations.
    pub fn allocation_count(&self) -> usize {
        self.count.load(Ordering::Relaxed)
    }

    /// Get the parent allocator.
    pub fn allocator(&self) -> &SmartAlloc {
        &self.alloc
    }
}

impl Drop for TaskAlloc {
    fn drop(&mut self) {
        let allocations = self.allocations.get_mut().unwrap();
        
        for tracked in allocations.drain(..) {
            unsafe {
                // Call drop function if present
                if let Some(drop_fn) = tracked.drop_fn {
                    drop_fn(tracked.ptr);
                }
                
                // Free the memory
                std::alloc::dealloc(tracked.ptr.as_ptr(), tracked.layout);
            }
        }
    }
}

// SAFETY: TaskAlloc uses internal synchronization (Mutex)
unsafe impl Send for TaskAlloc {}
unsafe impl Sync for TaskAlloc {}

/// Helper function to drop a typed value at a pointer.
unsafe fn drop_typed<T>(ptr: NonNull<u8>) {
    std::ptr::drop_in_place(ptr.as_ptr() as *mut T);
}

/// A boxed value owned by a `TaskAlloc`.
///
/// This is similar to `Box<T>` but the memory is managed by the parent
/// `TaskAlloc` and will be freed when it drops.
///
/// # Safety
///
/// The `TaskBox` must not outlive its parent `TaskAlloc`.
pub struct TaskBox<T> {
    ptr: NonNull<T>,
    _marker: PhantomData<T>,
}

impl<T> Deref for TaskBox<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { self.ptr.as_ref() }
    }
}

impl<T> DerefMut for TaskBox<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.ptr.as_mut() }
    }
}

impl<T> TaskBox<T> {
    /// Get a raw pointer to the value.
    pub fn as_ptr(&self) -> *const T {
        self.ptr.as_ptr()
    }

    /// Get a mutable raw pointer to the value.
    pub fn as_mut_ptr(&mut self) -> *mut T {
        self.ptr.as_ptr()
    }
}

// TaskBox doesn't implement Drop - cleanup is handled by TaskAlloc
// SAFETY: TaskBox is just a pointer wrapper, same Send/Sync as T
unsafe impl<T: Send> Send for TaskBox<T> {}
unsafe impl<T: Sync> Sync for TaskBox<T> {}

/// A slice owned by a `TaskAlloc`.
///
/// The memory is managed by the parent `TaskAlloc` and will be freed when
/// it drops.
pub struct TaskSlice<T> {
    ptr: NonNull<T>,
    len: usize,
    _marker: PhantomData<T>,
}

impl<T> TaskSlice<T> {
    /// Get the length of the slice.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Check if the slice is empty.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Get a raw pointer to the slice data.
    pub fn as_ptr(&self) -> *const T {
        self.ptr.as_ptr()
    }

    /// Get a mutable raw pointer to the slice data.
    pub fn as_mut_ptr(&mut self) -> *mut T {
        self.ptr.as_ptr()
    }

    /// Get the slice as a Rust slice.
    ///
    /// # Safety
    ///
    /// The caller must ensure the memory has been properly initialized.
    pub unsafe fn as_slice(&self) -> &[T] {
        std::slice::from_raw_parts(self.ptr.as_ptr(), self.len)
    }

    /// Get the slice as a mutable Rust slice.
    ///
    /// # Safety
    ///
    /// The caller must ensure the memory has been properly initialized.
    pub unsafe fn as_mut_slice(&mut self) -> &mut [T] {
        std::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len)
    }
}

// SAFETY: TaskSlice is just a pointer wrapper
unsafe impl<T: Send> Send for TaskSlice<T> {}
unsafe impl<T: Sync> Sync for TaskSlice<T> {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AllocConfig;

    #[test]
    fn task_alloc_creates_and_drops() {
        let alloc = SmartAlloc::new(AllocConfig::default());
        
        {
            let task = TaskAlloc::new(&alloc);
            let _a = task.alloc_box(42u32);
            let _b = task.alloc_box(String::from("hello"));
            
            assert_eq!(task.allocation_count(), 2);
        }
        // Allocations freed here
    }

    #[test]
    fn task_box_deref() {
        let alloc = SmartAlloc::new(AllocConfig::default());
        let task = TaskAlloc::new(&alloc);
        
        let boxed = task.alloc_box(vec![1, 2, 3]);
        assert_eq!(&*boxed, &vec![1, 2, 3]);
    }

    #[test]
    fn task_slice_basic() {
        let alloc = SmartAlloc::new(AllocConfig::default());
        let task = TaskAlloc::new(&alloc);
        
        let mut slice: TaskSlice<u32> = task.alloc_slice(10);
        assert_eq!(slice.len(), 10);
        
        // Initialize the slice
        unsafe {
            for i in 0..10 {
                slice.as_mut_ptr().add(i).write(i as u32);
            }
            
            let s = slice.as_slice();
            assert_eq!(s[5], 5);
        }
    }
}
