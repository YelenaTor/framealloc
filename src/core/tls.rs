//! Thread-local state management.

use std::cell::RefCell;
use std::sync::Arc;

use crate::allocators::deferred::DeferredFreeQueue;
use crate::allocators::frame::FrameArena;
use crate::allocators::slab::LocalPools;
use crate::api::stats::ThreadStats;
use crate::core::global::GlobalState;
use crate::util::size::mb;

/// Thread-local state for the allocator.
pub struct ThreadLocalState {
    /// Frame arena (bump allocator)
    frame: FrameArena,

    /// Local pools for small objects
    pools: LocalPools,

    /// Queue for cross-thread frees
    deferred: DeferredFreeQueue,

    /// Per-thread statistics
    stats: ThreadStats,

    /// Whether a frame is currently active
    frame_active: bool,
}

thread_local! {
    static TLS: RefCell<Option<ThreadLocalState>> = const { RefCell::new(None) };
}

impl ThreadLocalState {
    /// Create new thread-local state.
    fn new() -> Self {
        Self {
            frame: FrameArena::new(mb(16)), // Default size, can be configured
            pools: LocalPools::new(),
            deferred: DeferredFreeQueue::new(),
            stats: ThreadStats::new(),
            frame_active: false,
        }
    }

    /// Begin a new frame.
    pub fn begin_frame(&mut self) {
        // Process any deferred frees first
        self.deferred.drain(&mut self.pools);
        self.frame_active = true;
    }

    /// End the current frame.
    pub fn end_frame(&mut self) {
        self.frame.reset();
        self.frame_active = false;
    }

    /// Check if a frame is currently active.
    pub fn is_frame_active(&self) -> bool {
        self.frame_active
    }

    /// Get current frame arena head position.
    pub fn frame_head(&self) -> usize {
        self.frame.head()
    }

    /// Reset frame arena to a saved position.
    pub fn reset_frame_to(&mut self, head: usize) {
        self.frame.reset_to(head);
    }

    /// Allocate from frame arena.
    pub fn frame_alloc<T>(&mut self) -> *mut T {
        let ptr = self.frame.alloc::<T>();
        #[cfg(not(feature = "minimal"))]
        if !ptr.is_null() {
            self.stats.record_alloc(std::mem::size_of::<T>());
        }
        #[cfg(all(target_arch = "x86_64", feature = "prefetch"))]
        if !ptr.is_null() {
            // Prefetch for write - brings cache line into L1
            unsafe { std::arch::x86_64::_mm_prefetch(ptr as *const i8, std::arch::x86_64::_MM_HINT_T0); }
        }
        ptr
    }

    /// Fallible allocation from frame arena.
    /// 
    /// Returns None on OOM instead of returning null.
    pub fn try_frame_alloc<T>(&mut self) -> Option<*mut T> {
        let ptr = self.frame.alloc::<T>();
        #[cfg(not(feature = "minimal"))]
        if !ptr.is_null() {
            self.stats.record_alloc(std::mem::size_of::<T>());
        }
        #[cfg(all(target_arch = "x86_64", feature = "prefetch"))]
        if !ptr.is_null() {
            // Prefetch for write - brings cache line into L1
            unsafe { std::arch::x86_64::_mm_prefetch(ptr as *const i8, std::arch::x86_64::_MM_HINT_T0); }
        }
        if ptr.is_null() {
            None
        } else {
            Some(ptr)
        }
    }

    /// Allocate a slice from frame arena.
    pub fn frame_alloc_slice<T>(&mut self, count: usize) -> *mut T {
        let ptr = self.frame.alloc_slice::<T>(count);
        #[cfg(not(feature = "minimal"))]
        if !ptr.is_null() {
            self.stats.record_alloc(std::mem::size_of::<T>() * count);
        }
        #[cfg(all(target_arch = "x86_64", feature = "prefetch"))]
        if !ptr.is_null() && count > 0 {
            // Prefetch first cache line
            unsafe { std::arch::x86_64::_mm_prefetch(ptr as *const i8, std::arch::x86_64::_MM_HINT_T0); }
        }
        ptr
    }

    /// Allocate N instances of T with single bookkeeping update.
    /// 
    /// Returns a pointer to uninitialized memory for N values.
    /// More efficient than N separate allocations when statistics are enabled.
    pub fn frame_alloc_batch<T>(&mut self, count: usize) -> *mut T {
        let ptr = self.frame.alloc_slice::<T>(count);
        #[cfg(not(feature = "minimal"))]
        if !ptr.is_null() {
            // Single bookkeeping update for all allocations
            self.stats.record_alloc(std::mem::size_of::<T>() * count);
        }
        #[cfg(all(target_arch = "x86_64", feature = "prefetch"))]
        if !ptr.is_null() && count > 0 {
            // Prefetch first cache line
            unsafe { std::arch::x86_64::_mm_prefetch(ptr as *const i8, std::arch::x86_64::_MM_HINT_T0); }
        }
        ptr
    }

    /// Allocate 2 instances of T with optimized single allocation.
    #[inline(always)]
    pub fn frame_alloc_2<T>(&mut self) -> *mut [T; 2] {
        let ptr = self.frame.alloc_slice::<T>(2) as *mut [T; 2];
        #[cfg(not(feature = "minimal"))]
        if !ptr.is_null() {
            self.stats.record_alloc(std::mem::size_of::<T>() * 2);
        }
        #[cfg(all(target_arch = "x86_64", feature = "prefetch"))]
        if !ptr.is_null() {
            unsafe { std::arch::x86_64::_mm_prefetch(ptr as *const i8, std::arch::x86_64::_MM_HINT_T0); }
        }
        ptr
    }

    /// Allocate 4 instances of T with optimized single allocation.
    #[inline(always)]
    pub fn frame_alloc_4<T>(&mut self) -> *mut [T; 4] {
        let ptr = self.frame.alloc_slice::<T>(4) as *mut [T; 4];
        #[cfg(not(feature = "minimal"))]
        if !ptr.is_null() {
            self.stats.record_alloc(std::mem::size_of::<T>() * 4);
        }
        #[cfg(all(target_arch = "x86_64", feature = "prefetch"))]
        if !ptr.is_null() {
            unsafe { std::arch::x86_64::_mm_prefetch(ptr as *const i8, std::arch::x86_64::_MM_HINT_T0); }
        }
        ptr
    }

    /// Allocate 8 instances of T with optimized single allocation.
    #[inline(always)]
    pub fn frame_alloc_8<T>(&mut self) -> *mut [T; 8] {
        let ptr = self.frame.alloc_slice::<T>(8) as *mut [T; 8];
        #[cfg(not(feature = "minimal"))]
        if !ptr.is_null() {
            self.stats.record_alloc(std::mem::size_of::<T>() * 8);
        }
        #[cfg(all(target_arch = "x86_64", feature = "prefetch"))]
        if !ptr.is_null() {
            unsafe { std::arch::x86_64::_mm_prefetch(ptr as *const i8, std::arch::x86_64::_MM_HINT_T0); }
        }
        ptr
    }

    /// Allocate from local pool.
    pub fn pool_alloc<T>(&mut self, global: &Arc<GlobalState>) -> *mut T {
        let size = std::mem::size_of::<T>();
        let ptr = self.pools.alloc(size, global.slabs());
        if !ptr.is_null() {
            self.stats.record_alloc(size);
        }
        ptr as *mut T
    }

    /// Free to local pool (or defer if cross-thread).
    pub fn pool_free<T>(&mut self, ptr: *mut T, _global: &Arc<GlobalState>) {
        let size = std::mem::size_of::<T>();
        self.pools.free(ptr as *mut u8, size);
        self.stats.record_dealloc(size);
    }

    /// Queue a deferred free from another thread.
    #[allow(dead_code)]
    pub fn queue_deferred_free(&self, ptr: *mut u8, size: usize) {
        self.deferred.push(ptr, size);
    }

    /// Allocate from frame arena with a specific layout.
    pub fn frame_alloc_layout(&mut self, layout: std::alloc::Layout) -> *mut u8 {
        let ptr = self.frame.alloc_layout(layout);
        #[cfg(not(feature = "minimal"))]
        if !ptr.is_null() {
            self.stats.record_alloc(layout.size());
        }
        ptr
    }

    /// Allocate from pool with a specific layout.
    pub fn pool_alloc_layout(&mut self, layout: std::alloc::Layout, global: &Arc<GlobalState>) -> *mut u8 {
        let ptr = self.pools.alloc(layout.size(), global.slabs());
        if !ptr.is_null() {
            self.stats.record_alloc(layout.size());
        }
        ptr
    }

    /// Free to pool with a specific layout.
    pub fn pool_free_layout(&mut self, ptr: *mut u8, layout: std::alloc::Layout, _global: &Arc<GlobalState>) {
        self.pools.free(ptr, layout.size());
        self.stats.record_dealloc(layout.size());
    }
}

/// Execute a closure with access to thread-local state.
///
/// Initializes TLS lazily on first access.
pub fn with_tls<F, R>(f: F) -> R
where
    F: FnOnce(&mut ThreadLocalState) -> R,
{
    TLS.with(|cell| {
        let mut borrow = cell.borrow_mut();
        let tls = borrow.get_or_insert_with(ThreadLocalState::new);
        f(tls)
    })
}

/// Check if TLS is initialized for the current thread.
pub fn is_tls_initialized() -> bool {
    TLS.with(|cell| cell.borrow().is_some())
}
