//! The main allocator type.

use std::sync::Arc;

use crate::allocators::handles::HandleAllocator;
use crate::allocators::streaming::StreamingAllocator;
use crate::api::config::AllocConfig;
use crate::api::groups::GroupAllocator;
use crate::api::scope::FrameGuard;
use crate::api::stats::AllocStats;
use crate::api::tag::AllocationIntent;
use crate::api::wrappers::{FrameBox, FrameSlice, HeapBox, PoolBox};
use crate::core::global::GlobalState;
use crate::core::tls;
use crate::diagnostics::SharedDiagnostics;
use crate::util::size::mb;

/// The main smart allocator type.
///
/// This is the primary entry point for all allocation operations.
/// It is cheap to clone (internally uses `Arc`) and thread-safe.
///
/// # Example
///
/// ```rust,no_run
/// use framealloc::{SmartAlloc, AllocConfig};
///
/// let alloc = SmartAlloc::new(AllocConfig::default());
///
/// alloc.begin_frame();
/// let data = alloc.frame_alloc::<[f32; 256]>();
/// alloc.end_frame();
/// ```
#[derive(Clone)]
pub struct SmartAlloc {
    inner: Arc<GlobalState>,
    streaming: Arc<StreamingAllocator>,
    handles: Arc<HandleAllocator>,
    groups: Arc<GroupAllocator>,
    diagnostics: Arc<SharedDiagnostics>,
}

impl SmartAlloc {
    /// Create a new allocator with the given configuration.
    pub fn new(config: AllocConfig) -> Self {
        let streaming_budget = if config.global_memory_limit > 0 {
            config.global_memory_limit / 4 // 25% for streaming by default
        } else {
            mb(256) // 256MB default streaming budget
        };

        Self {
            inner: Arc::new(GlobalState::new(config)),
            streaming: Arc::new(StreamingAllocator::new(streaming_budget)),
            handles: Arc::new(HandleAllocator::new()),
            groups: Arc::new(GroupAllocator::new()),
            diagnostics: Arc::new(SharedDiagnostics::new()),
        }
    }

    /// Create an allocator with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(AllocConfig::default())
    }

    /// Begin a new frame.
    ///
    /// This should be called at the start of each game frame.
    /// It prepares the frame arena for new allocations.
    pub fn begin_frame(&self) {
        tls::with_tls(|tls| {
            tls.begin_frame();
        });
    }

    /// End the current frame.
    ///
    /// This resets the frame arena, invalidating all frame allocations.
    /// Any pointers from `frame_alloc` become invalid after this call.
    pub fn end_frame(&self) {
        tls::with_tls(|tls| {
            tls.end_frame();
        });
    }

    /// Create a frame scope guard.
    ///
    /// The frame arena is reset when the guard is dropped.
    /// This is useful for sub-frame temporary allocations.
    pub fn frame_scope(&self) -> FrameGuard<'_> {
        FrameGuard::new(self)
    }

    /// Allocate memory from the frame arena.
    ///
    /// This is the fastest allocation path - a simple bump pointer.
    /// The memory is valid until `end_frame()` is called.
    ///
    /// # Safety
    ///
    /// The returned pointer is valid only until `end_frame()` is called.
    /// Using the pointer after that is undefined behavior.
    pub fn frame_alloc<T>(&self) -> *mut T {
        self.frame_alloc_with_intent::<T>(AllocationIntent::Frame)
    }

    /// Allocate memory from the frame arena with explicit intent.
    pub fn frame_alloc_with_intent<T>(&self, _intent: AllocationIntent) -> *mut T {
        tls::with_tls(|tls| tls.frame_alloc::<T>())
    }

    /// Allocate a value from the small object pool.
    ///
    /// This is fast O(1) allocation from thread-local pools.
    /// The memory must be explicitly freed with `pool_free`.
    pub fn pool_alloc<T>(&self) -> *mut T {
        tls::with_tls(|tls| tls.pool_alloc::<T>(&self.inner))
    }

    /// Free a value back to the small object pool.
    ///
    /// # Safety
    ///
    /// The pointer must have been allocated with `pool_alloc`.
    pub unsafe fn pool_free<T>(&self, ptr: *mut T) {
        tls::with_tls(|tls| tls.pool_free(ptr, &self.inner));
    }

    /// Allocate memory from the system heap.
    ///
    /// This is the slowest path, used for large allocations.
    /// The memory must be explicitly freed with `heap_free`.
    pub fn heap_alloc<T>(&self) -> *mut T {
        self.inner.heap_alloc::<T>()
    }

    /// Free memory allocated from the system heap.
    ///
    /// # Safety
    ///
    /// The pointer must have been allocated with `heap_alloc`.
    pub unsafe fn heap_free<T>(&self, ptr: *mut T) {
        self.inner.heap_free(ptr);
    }

    /// Get current allocation statistics.
    pub fn stats(&self) -> AllocStats {
        self.inner.stats()
    }

    /// Get a reference to the global state (for advanced usage).
    pub(crate) fn global(&self) -> &Arc<GlobalState> {
        &self.inner
    }

    // ==================== Safe Wrapper Methods ====================

    /// Allocate and initialize a value in the frame arena.
    ///
    /// Returns a safe wrapper that derefs to T.
    /// Memory is valid until `end_frame()` is called.
    pub fn frame_box<T>(&self, value: T) -> Option<FrameBox<'_, T>> {
        let ptr = self.frame_alloc::<T>();
        if ptr.is_null() {
            return None;
        }
        unsafe {
            std::ptr::write(ptr, value);
            FrameBox::from_raw(ptr)
        }
    }

    /// Allocate a slice in the frame arena.
    ///
    /// Elements are zero-initialized for primitive types.
    pub fn frame_slice<T: Default + Clone>(&self, len: usize) -> Option<FrameSlice<'_, T>> {
        let ptr = tls::with_tls(|tls| {
            tls.frame_alloc_slice::<T>(len)
        });
        if ptr.is_null() {
            return None;
        }
        // Initialize elements
        for i in 0..len {
            unsafe {
                std::ptr::write(ptr.add(i), T::default());
            }
        }
        unsafe { FrameSlice::from_raw_parts(ptr, len) }
    }

    /// Allocate and initialize a value in the pool.
    ///
    /// Returns a safe wrapper that automatically frees on drop.
    pub fn pool_box<T>(&self, value: T) -> Option<PoolBox<T>> {
        let ptr = self.pool_alloc::<T>();
        if ptr.is_null() {
            return None;
        }
        unsafe {
            std::ptr::write(ptr, value);
            PoolBox::from_raw(ptr, self.inner.clone())
        }
    }

    /// Allocate and initialize a value on the heap.
    ///
    /// Returns a safe wrapper that automatically frees on drop.
    pub fn heap_box<T>(&self, value: T) -> Option<HeapBox<T>> {
        let ptr = self.heap_alloc::<T>();
        if ptr.is_null() {
            return None;
        }
        unsafe {
            std::ptr::write(ptr, value);
            HeapBox::from_raw(ptr, self.inner.clone())
        }
    }

    // ==================== Integrated Allocators ====================

    /// Access the streaming allocator for large assets.
    pub fn streaming(&self) -> &StreamingAllocator {
        &self.streaming
    }

    /// Access the handle-based allocator.
    pub fn handles(&self) -> &HandleAllocator {
        &self.handles
    }

    /// Access the group allocator.
    pub fn groups(&self) -> &GroupAllocator {
        &self.groups
    }

    /// Access the diagnostics system.
    pub fn diagnostics(&self) -> &SharedDiagnostics {
        &self.diagnostics
    }

    /// Get the budget manager if budgets are enabled.
    pub fn budgets(&self) -> Option<&crate::core::budget::BudgetManager> {
        self.inner.budgets()
    }
}

impl Default for SmartAlloc {
    fn default() -> Self {
        Self::with_defaults()
    }
}

// Safety: SmartAlloc is thread-safe because GlobalState is thread-safe
// and TLS access is inherently thread-local.
unsafe impl Send for SmartAlloc {}
unsafe impl Sync for SmartAlloc {}
