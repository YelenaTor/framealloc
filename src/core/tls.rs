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
        if !ptr.is_null() {
            self.stats.record_alloc(std::mem::size_of::<T>());
        }
        ptr
    }

    /// Allocate a slice from frame arena.
    pub fn frame_alloc_slice<T>(&mut self, count: usize) -> *mut T {
        let ptr = self.frame.alloc_slice::<T>(count);
        if !ptr.is_null() {
            self.stats.record_alloc(std::mem::size_of::<T>() * count);
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
