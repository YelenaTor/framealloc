//! Safe wrapper types for allocations.
//!
//! These provide RAII semantics and safe access to allocated memory.

use std::ops::{Deref, DerefMut};
use std::ptr::NonNull;
use std::marker::PhantomData;
use std::sync::Arc;

use crate::core::global::GlobalState;
use crate::core::tls;

/// A Box-like wrapper for frame-allocated memory.
///
/// The memory is valid until `end_frame()` is called on the allocator.
/// This type does NOT free memory on drop - frame memory is bulk-freed.
///
/// # Example
///
/// ```rust,ignore
/// let alloc = SmartAlloc::with_defaults();
/// alloc.begin_frame();
///
/// let data = alloc.frame_box(MyStruct::new());
/// println!("{}", data.field);
///
/// alloc.end_frame(); // data is now invalid
/// ```
pub struct FrameBox<'a, T> {
    ptr: NonNull<T>,
    _marker: PhantomData<&'a T>,
}

impl<'a, T> FrameBox<'a, T> {
    /// Create a new FrameBox from a raw pointer.
    ///
    /// # Safety
    ///
    /// The pointer must be valid and properly aligned for T.
    /// The memory must remain valid for the lifetime 'a.
    pub(crate) unsafe fn from_raw(ptr: *mut T) -> Option<Self> {
        NonNull::new(ptr).map(|ptr| Self {
            ptr,
            _marker: PhantomData,
        })
    }

    /// Get the raw pointer.
    pub fn as_ptr(&self) -> *const T {
        self.ptr.as_ptr()
    }

    /// Get the raw mutable pointer.
    pub fn as_mut_ptr(&mut self) -> *mut T {
        self.ptr.as_ptr()
    }

    /// Leak the FrameBox, returning the raw pointer.
    ///
    /// This is safe because frame memory is bulk-freed anyway.
    pub fn into_raw(self) -> *mut T {
        let ptr = self.ptr.as_ptr();
        std::mem::forget(self);
        ptr
    }
}

impl<'a, T> Deref for FrameBox<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { self.ptr.as_ref() }
    }
}

impl<'a, T> DerefMut for FrameBox<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.ptr.as_mut() }
    }
}

// FrameBox doesn't implement Drop - memory is freed in bulk at end_frame()

/// A Box-like wrapper for pool-allocated memory.
///
/// Automatically frees memory back to the pool when dropped.
///
/// # Example
///
/// ```rust,ignore
/// let alloc = SmartAlloc::with_defaults();
///
/// {
///     let data = alloc.pool_box(MyStruct::new());
///     println!("{}", data.field);
/// } // data is freed here
/// ```
pub struct PoolBox<T> {
    ptr: NonNull<T>,
    global: Arc<GlobalState>,
}

impl<T> PoolBox<T> {
    /// Create a new PoolBox.
    ///
    /// # Safety
    ///
    /// The pointer must have been allocated from the pool.
    pub(crate) unsafe fn from_raw(ptr: *mut T, global: Arc<GlobalState>) -> Option<Self> {
        NonNull::new(ptr).map(|ptr| Self { ptr, global })
    }

    /// Get the raw pointer.
    pub fn as_ptr(&self) -> *const T {
        self.ptr.as_ptr()
    }

    /// Get the raw mutable pointer.
    pub fn as_mut_ptr(&mut self) -> *mut T {
        self.ptr.as_ptr()
    }

    /// Leak the PoolBox, returning the raw pointer.
    ///
    /// The caller is responsible for freeing the memory.
    pub fn into_raw(self) -> *mut T {
        let ptr = self.ptr.as_ptr();
        std::mem::forget(self);
        ptr
    }
}

impl<T> Deref for PoolBox<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { self.ptr.as_ref() }
    }
}

impl<T> DerefMut for PoolBox<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.ptr.as_mut() }
    }
}

impl<T> Drop for PoolBox<T> {
    fn drop(&mut self) {
        tls::with_tls(|tls| {
            tls.pool_free(self.ptr.as_ptr(), &self.global);
        });
    }
}

// Safety: PoolBox can be sent between threads
unsafe impl<T: Send> Send for PoolBox<T> {}
unsafe impl<T: Sync> Sync for PoolBox<T> {}

/// A Box-like wrapper for heap-allocated memory.
///
/// Automatically frees memory when dropped.
///
/// # Example
///
/// ```rust,ignore
/// let alloc = SmartAlloc::with_defaults();
///
/// {
///     let data = alloc.heap_box(LargeStruct::new());
///     println!("{}", data.field);
/// } // data is freed here
/// ```
pub struct HeapBox<T> {
    ptr: NonNull<T>,
    global: Arc<GlobalState>,
}

impl<T> HeapBox<T> {
    /// Create a new HeapBox.
    ///
    /// # Safety
    ///
    /// The pointer must have been allocated from the heap.
    pub(crate) unsafe fn from_raw(ptr: *mut T, global: Arc<GlobalState>) -> Option<Self> {
        NonNull::new(ptr).map(|ptr| Self { ptr, global })
    }

    /// Get the raw pointer.
    pub fn as_ptr(&self) -> *const T {
        self.ptr.as_ptr()
    }

    /// Get the raw mutable pointer.
    pub fn as_mut_ptr(&mut self) -> *mut T {
        self.ptr.as_ptr()
    }

    /// Leak the HeapBox, returning the raw pointer.
    ///
    /// The caller is responsible for freeing the memory.
    pub fn into_raw(self) -> *mut T {
        let ptr = self.ptr.as_ptr();
        std::mem::forget(self);
        ptr
    }
}

impl<T> Deref for HeapBox<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { self.ptr.as_ref() }
    }
}

impl<T> DerefMut for HeapBox<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.ptr.as_mut() }
    }
}

impl<T> Drop for HeapBox<T> {
    fn drop(&mut self) {
        unsafe {
            // Drop the value
            std::ptr::drop_in_place(self.ptr.as_ptr());
            // Free the memory
            self.global.heap_free(self.ptr.as_ptr());
        }
    }
}

// Safety: HeapBox can be sent between threads
unsafe impl<T: Send> Send for HeapBox<T> {}
unsafe impl<T: Sync> Sync for HeapBox<T> {}

/// A slice allocated from the frame arena.
pub struct FrameSlice<'a, T> {
    ptr: NonNull<T>,
    len: usize,
    _marker: PhantomData<&'a [T]>,
}

impl<'a, T> FrameSlice<'a, T> {
    /// Create a new FrameSlice.
    ///
    /// # Safety
    ///
    /// The pointer must point to `len` valid, initialized elements of T.
    pub(crate) unsafe fn from_raw_parts(ptr: *mut T, len: usize) -> Option<Self> {
        NonNull::new(ptr).map(|ptr| Self {
            ptr,
            len,
            _marker: PhantomData,
        })
    }

    /// Get the length of the slice.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Check if the slice is empty.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Get the raw pointer.
    pub fn as_ptr(&self) -> *const T {
        self.ptr.as_ptr()
    }

    /// Get the raw mutable pointer.
    pub fn as_mut_ptr(&mut self) -> *mut T {
        self.ptr.as_ptr()
    }
}

impl<'a, T> Deref for FrameSlice<'a, T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        unsafe { std::slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
    }
}

impl<'a, T> DerefMut for FrameSlice<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { std::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SmartAlloc;

    #[test]
    fn test_frame_box() {
        let alloc = SmartAlloc::with_defaults();
        alloc.begin_frame();

        let boxed = alloc.frame_box(42u64).unwrap();
        assert_eq!(*boxed, 42);

        alloc.end_frame();
    }

    #[test]
    fn test_pool_box() {
        let alloc = SmartAlloc::with_defaults();

        {
            let boxed = alloc.pool_box(123u64).unwrap();
            assert_eq!(*boxed, 123);
        } // Dropped here

        // Allocate again - should reuse
        let boxed2 = alloc.pool_box(456u64).unwrap();
        assert_eq!(*boxed2, 456);
    }

    #[test]
    fn test_heap_box() {
        let alloc = SmartAlloc::with_defaults();

        {
            let boxed = alloc.heap_box([0u8; 8192]).unwrap();
            assert_eq!(boxed[0], 0);
        } // Dropped and freed here
    }

    #[test]
    fn test_frame_slice() {
        let alloc = SmartAlloc::with_defaults();
        alloc.begin_frame();

        let mut slice = alloc.frame_slice::<u32>(100).unwrap();
        assert_eq!(slice.len(), 100);

        slice[0] = 42;
        slice[99] = 123;
        assert_eq!(slice[0], 42);
        assert_eq!(slice[99], 123);

        alloc.end_frame();
    }
}
