//! Implementation of std::alloc::Allocator trait for framealloc types.
//!
//! This module requires the `nightly` feature and a nightly Rust compiler.
//! It allows using framealloc with standard library collections.
//!
//! Enable with:
//! ```toml
//! [dependencies]
//! framealloc = { version = "0.1", features = ["nightly"] }
//! ```

#![cfg(feature = "nightly")]
#![cfg_attr(feature = "nightly", feature(allocator_api))]

use std::alloc::{AllocError, Allocator, Layout};
use std::ptr::NonNull;
use std::sync::Arc;

use crate::core::global::GlobalState;
use crate::core::tls;

/// A frame allocator that implements the std::alloc::Allocator trait.
///
/// This can be used with standard library collections like Vec and Box.
///
/// # Example
///
/// ```rust,ignore
/// use framealloc::{SmartAlloc, FrameAllocator};
///
/// let alloc = SmartAlloc::with_defaults();
/// alloc.begin_frame();
///
/// let frame_alloc = alloc.frame_allocator();
/// let mut vec: Vec<u32, _> = Vec::new_in(frame_alloc);
/// vec.push(42);
///
/// alloc.end_frame();
/// ```
#[derive(Clone)]
pub struct FrameAllocator {
    _marker: std::marker::PhantomData<*const ()>,
}

impl FrameAllocator {
    /// Create a new frame allocator.
    pub fn new() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

impl Default for FrameAllocator {
    fn default() -> Self {
        Self::new()
    }
}

unsafe impl Allocator for FrameAllocator {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        let ptr = tls::with_tls(|tls| {
            tls.frame_alloc_layout(layout)
        });

        NonNull::new(ptr)
            .map(|p| NonNull::slice_from_raw_parts(p, layout.size()))
            .ok_or(AllocError)
    }

    unsafe fn deallocate(&self, _ptr: NonNull<u8>, _layout: Layout) {
        // Frame allocator doesn't deallocate individual allocations
        // Memory is freed in bulk at end_frame()
    }
}

/// A pool allocator that implements the std::alloc::Allocator trait.
#[derive(Clone)]
pub struct PoolAllocator {
    global: Arc<GlobalState>,
}

impl PoolAllocator {
    /// Create a new pool allocator.
    pub fn new(global: Arc<GlobalState>) -> Self {
        Self { global }
    }
}

unsafe impl Allocator for PoolAllocator {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        let ptr = tls::with_tls(|tls| {
            tls.pool_alloc_layout(layout, &self.global)
        });

        NonNull::new(ptr)
            .map(|p| NonNull::slice_from_raw_parts(p, layout.size()))
            .ok_or(AllocError)
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        tls::with_tls(|tls| {
            tls.pool_free_layout(ptr.as_ptr(), layout, &self.global);
        });
    }
}

/// A heap allocator that implements the std::alloc::Allocator trait.
#[derive(Clone)]
pub struct HeapAllocator {
    global: Arc<GlobalState>,
}

impl HeapAllocator {
    /// Create a new heap allocator.
    pub fn new(global: Arc<GlobalState>) -> Self {
        Self { global }
    }
}

unsafe impl Allocator for HeapAllocator {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        let ptr = self.global.heap_alloc_layout(layout);

        NonNull::new(ptr)
            .map(|p| NonNull::slice_from_raw_parts(p, layout.size()))
            .ok_or(AllocError)
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        self.global.heap_free_layout(ptr.as_ptr(), layout);
    }
}
