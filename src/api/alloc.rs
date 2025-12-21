//! The main allocator type.

use std::sync::Arc;

use crate::allocators::handles::HandleAllocator;
use crate::allocators::streaming::StreamingAllocator;
use crate::api::checkpoint::{CheckpointGuard, FrameCheckpoint, SpeculativeResult};
use crate::api::config::AllocConfig;
use crate::api::frame_collections::{FrameMap, FrameVec};
use crate::api::groups::GroupAllocator;
use crate::api::phases::{self, Phase, PhaseGuard};
use crate::api::scope::FrameGuard;
use crate::api::scratch::ScratchRegistry;
use crate::api::stats::AllocStats;
use crate::api::tag::AllocationIntent;
use crate::api::tagged::{self, TagGuard};
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
    scratch: Arc<ScratchRegistry>,
    frame_counter: Arc<std::sync::atomic::AtomicU64>,
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
            scratch: Arc::new(ScratchRegistry::default()),
            frame_counter: Arc::new(std::sync::atomic::AtomicU64::new(0)),
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
        self.frame_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        phases::reset_phases();
        tls::with_tls(|tls| {
            tls.begin_frame();
        });
    }

    /// End the current frame.
    ///
    /// This resets the frame arena, invalidating all frame allocations.
    /// Any pointers from `frame_alloc` become invalid after this call.
    pub fn end_frame(&self) {
        phases::reset_phases();
        tls::with_tls(|tls| {
            tls.end_frame();
        });
    }

    /// Get the current frame number.
    pub fn frame_number(&self) -> u64 {
        self.frame_counter.load(std::sync::atomic::Ordering::Relaxed)
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

    // ==================== Frame Phases (v0.2.0) ====================

    /// Begin a named phase within the current frame.
    ///
    /// Phases divide frames into logical sections for profiling and diagnostics.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// alloc.begin_frame();
    /// alloc.begin_phase("physics");
    /// // physics allocations...
    /// alloc.end_phase();
    /// alloc.begin_phase("render");
    /// // render allocations...
    /// alloc.end_phase();
    /// alloc.end_frame();
    /// ```
    pub fn begin_phase(&self, name: &'static str) {
        phases::begin_phase(name);
    }

    /// End the current phase.
    pub fn end_phase(&self) -> Option<Phase> {
        phases::end_phase()
    }

    /// Create a phase scope guard.
    ///
    /// The phase is automatically ended when the guard is dropped.
    pub fn phase_scope(&self, name: &'static str) -> PhaseGuard {
        PhaseGuard::new(name)
    }

    /// Get the current phase name.
    pub fn current_phase(&self) -> Option<&'static str> {
        phases::current_phase()
    }

    // ==================== Frame Checkpoints (v0.2.0) ====================

    /// Create a checkpoint at the current frame arena position.
    ///
    /// Checkpoints allow rolling back speculative allocations.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let checkpoint = alloc.frame_checkpoint();
    /// // speculative allocations...
    /// if failed {
    ///     alloc.rollback_to(checkpoint);
    /// }
    /// ```
    pub fn frame_checkpoint(&self) -> FrameCheckpoint {
        let head = tls::with_tls(|tls| tls.frame_head());
        FrameCheckpoint::new(head, self.frame_number())
    }

    /// Rollback to a previously saved checkpoint.
    ///
    /// All allocations made after the checkpoint are invalidated.
    pub fn rollback_to(&self, checkpoint: FrameCheckpoint) {
        debug_assert_eq!(
            checkpoint.frame_id(),
            self.frame_number(),
            "Cannot rollback to checkpoint from different frame"
        );
        tls::with_tls(|tls| tls.reset_frame_to(checkpoint.head()));
    }

    /// Create a checkpoint guard for automatic rollback.
    ///
    /// If not explicitly committed, allocations are rolled back on drop.
    pub fn checkpoint_guard(&self) -> CheckpointGuard<'_> {
        CheckpointGuard::new(self.frame_checkpoint())
    }

    /// Execute a speculative allocation block.
    ///
    /// If the closure returns `Err`, all allocations are rolled back.
    pub fn speculative<T, E, F>(&self, f: F) -> SpeculativeResult<T, E>
    where
        F: FnOnce() -> Result<T, E>,
    {
        let checkpoint = self.frame_checkpoint();
        match f() {
            Ok(value) => SpeculativeResult::Success(value),
            Err(e) => {
                self.rollback_to(checkpoint);
                SpeculativeResult::RolledBack(e)
            }
        }
    }

    // ==================== Frame Collections (v0.2.0) ====================

    /// Create a frame-allocated vector with fixed capacity.
    ///
    /// The vector cannot grow beyond its initial capacity.
    /// Memory is automatically freed at frame end.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let mut list = alloc.frame_vec::<Entity>(128);
    /// list.push(entity1);
    /// list.push(entity2);
    /// ```
    pub fn frame_vec<T>(&self, capacity: usize) -> Option<FrameVec<'_, T>> {
        let ptr = tls::with_tls(|tls| tls.frame_alloc_slice::<T>(capacity));
        unsafe { FrameVec::from_raw_parts(ptr, capacity) }
    }

    /// Create a frame-allocated hash map with fixed capacity.
    ///
    /// Simple open-addressing map for frame-temporary lookups.
    pub fn frame_map<K: Eq + std::hash::Hash, V>(&self, capacity: usize) -> Option<FrameMap<'_, K, V>> {
        let keys = tls::with_tls(|tls| tls.frame_alloc_slice::<Option<K>>(capacity));
        let values = tls::with_tls(|tls| tls.frame_alloc_slice::<V>(capacity));
        unsafe { FrameMap::from_raw_parts(keys, values, capacity) }
    }

    // ==================== Tagged Allocations (v0.2.0) ====================

    /// Execute a closure with a tag active.
    ///
    /// All allocations within the closure are attributed to the tag.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// alloc.with_tag("ai", |alloc| {
    ///     let scratch = alloc.frame_alloc::<Scratch>();
    ///     // allocation is attributed to "ai"
    /// });
    /// ```
    pub fn with_tag<F, R>(&self, tag: &'static str, f: F) -> R
    where
        F: FnOnce(&Self) -> R,
    {
        let _guard = TagGuard::new(tag);
        f(self)
    }

    /// Get the current allocation tag.
    pub fn current_tag(&self) -> Option<&'static str> {
        tagged::current_tag()
    }

    /// Get the full tag path (for nested tags).
    pub fn tag_path(&self) -> String {
        tagged::tag_path()
    }

    // ==================== Scratch Pools (v0.2.0) ====================

    /// Access the scratch pool registry.
    ///
    /// Scratch pools are for cross-frame reusable memory.
    pub fn scratch(&self) -> &ScratchRegistry {
        &self.scratch
    }

    /// Get or create a named scratch pool.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let pool = alloc.scratch_pool("pathfinding");
    /// let buf = pool.alloc::<[Node; 1024]>();
    /// // ...
    /// pool.reset(); // Clear when done
    /// ```
    pub fn scratch_pool(&self, name: &'static str) -> crate::api::scratch::ScratchPoolHandle<'_> {
        self.scratch.get_or_create(name)
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
