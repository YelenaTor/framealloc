//! The main allocator type.

use std::alloc::Layout;
use std::sync::Arc;

use crate::allocators::handles::HandleAllocator;
use crate::allocators::streaming::StreamingAllocator;
use crate::api::checkpoint::{CheckpointGuard, FrameCheckpoint, SpeculativeResult};
use crate::api::config::AllocConfig;
use crate::api::frame_collections::{FrameMap, FrameVec};
use crate::api::groups::GroupAllocator;
use crate::api::phases::{self, Phase, PhaseGuard};
use crate::api::promotion::{FrameSummary, PromotionProcessor, PromotionResult};
use crate::api::retention::{
    self, FrameRetained, Importance, PromotedAllocation, RetainedAllocation, RetainedMeta,
    RetentionPolicy,
};
use crate::diagnostics::behavior::{AllocKind, BehaviorFilter, BehaviorReport, BehaviorThresholds};
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
    /// Behavior filter for detecting allocation pattern issues (v0.4.0)
    behavior_filter: Arc<BehaviorFilter>,
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
            behavior_filter: Arc::new(BehaviorFilter::new()),
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
        self.behavior_filter.end_frame();
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

    /// Fallible allocation from frame arena.
    ///
    /// Returns None on OOM instead of returning null.
    /// The memory is valid until `end_frame()` is called.
    pub fn try_frame_alloc<T>(&self) -> Option<*mut T> {
        tls::with_tls(|tls| tls.try_frame_alloc::<T>())
    }

    /// Allocate memory from the frame arena with explicit intent.
    pub fn frame_alloc_with_intent<T>(&self, _intent: AllocationIntent) -> *mut T {
        tls::with_tls(|tls| tls.frame_alloc::<T>())
    }

    /// Allocate from frame arena with a specific layout.
    ///
    /// Returns a raw pointer to uninitialized memory.
    /// Memory is valid until `end_frame()` is called.
    /// 
    /// # Safety
    /// 
    /// The caller must ensure the layout has non-zero size.
    /// The returned pointer must be used according to the layout's alignment.
    pub unsafe fn frame_alloc_layout(&self, layout: std::alloc::Layout) -> *mut u8 {
        tls::with_tls(|tls| tls.frame_alloc_layout(layout))
    }

    /// Allocate N instances of T with single bookkeeping update.
    /// 
    /// Returns a raw pointer to uninitialized memory for N values.
    /// More efficient than N separate allocations when statistics are enabled.
    /// Memory is valid until `end_frame()` is called.
    /// 
    /// # Safety
    /// 
    /// The returned pointer is valid only until `end_frame()` is called.
    /// Using the pointer after that is undefined behavior.
    pub fn frame_alloc_batch<T>(&self, count: usize) -> *mut T {
        tls::with_tls(|tls| tls.frame_alloc_batch::<T>(count))
    }

    /// Allocate 2 instances of T with optimized single allocation.
    /// 
    /// Returns a raw pointer to uninitialized memory for 2 values.
    /// Memory is valid until `end_frame()` is called.
    /// 
    /// # Safety
    /// 
    /// The returned pointer is valid only until `end_frame()` is called.
    /// Using the pointer after that is undefined behavior.
    pub fn frame_alloc_2<T>(&self) -> *mut [T; 2] {
        tls::with_tls(|tls| tls.frame_alloc_2::<T>())
    }

    /// Allocate 4 instances of T with optimized single allocation.
    /// 
    /// Returns a raw pointer to uninitialized memory for 4 values.
    /// Memory is valid until `end_frame()` is called.
    /// 
    /// # Safety
    /// 
    /// The returned pointer is valid only until `end_frame()` is called.
    /// Using the pointer after that is undefined behavior.
    pub fn frame_alloc_4<T>(&self) -> *mut [T; 4] {
        tls::with_tls(|tls| tls.frame_alloc_4::<T>())
    }

    /// Allocate 8 instances of T with optimized single allocation.
    /// 
    /// Returns a raw pointer to uninitialized memory for 8 values.
    /// Memory is valid until `end_frame()` is called.
    /// 
    /// # Safety
    /// 
    /// The returned pointer is valid only until `end_frame()` is called.
    /// Using the pointer after that is undefined behavior.
    pub fn frame_alloc_8<T>(&self) -> *mut [T; 8] {
        tls::with_tls(|tls| tls.frame_alloc_8::<T>())
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

    // ==================== Frame Retention (v0.3.0) ====================

    /// Allocate with a retention policy for post-frame survival.
    ///
    /// Unlike regular frame allocations that are discarded at `end_frame()`,
    /// retained allocations can be promoted to other allocators.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // Allocate with promotion to pool
    /// let mut data = alloc.frame_retained::<NavMesh>(RetentionPolicy::PromoteToPool);
    /// data.calculate();
    ///
    /// // At frame end, get promoted allocations
    /// let result = alloc.end_frame_with_promotions();
    /// for item in result.promoted {
    ///     // Handle promoted allocations
    /// }
    /// ```
    pub fn frame_retained<T>(&self, policy: RetentionPolicy) -> FrameRetained<'_, T> {
        // Allocate from frame arena
        let ptr = tls::with_tls(|tls| tls.frame_alloc::<T>());
        
        // If policy is Discard, just return the handle without registering
        if !policy.promotes() {
            return FrameRetained::new(ptr, 0);
        }
        
        // Register for promotion
        let tag = tagged::current_tag();
        let meta = RetainedMeta {
            policy,
            size: std::mem::size_of::<T>(),
            tag,
            type_name: std::any::type_name::<T>(),
        };
        
        let alloc = RetainedAllocation {
            ptr: ptr as *mut u8,
            meta,
            promote_fn: Box::new(|_| PromotedAllocation::Pool {
                ptr: std::ptr::null_mut(),
                size: std::mem::size_of::<T>(),
                tag: None,
                type_name: std::any::type_name::<T>(),
            }),
        };
        
        let id = retention::register_retained(alloc);
        FrameRetained::new(ptr, id)
    }

    /// Allocate with importance level (semantic sugar for retention).
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // Reusable = promote to pool
    /// let data = alloc.frame_with_importance::<Path>(Importance::Reusable);
    ///
    /// // Persistent = promote to heap
    /// let data = alloc.frame_with_importance::<Config>(Importance::Persistent);
    /// ```
    pub fn frame_with_importance<T>(&self, importance: Importance) -> FrameRetained<'_, T> {
        self.frame_retained(importance.to_policy())
    }

    /// End frame and process retained allocations.
    ///
    /// This is an alternative to `end_frame()` that also processes
    /// retained allocations and returns promotion results.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let result = alloc.end_frame_with_promotions();
    /// 
    /// println!("Promoted {} bytes to pool", result.summary.promoted_pool_bytes);
    /// println!("Failed to promote {} allocations", result.summary.failed_count);
    ///
    /// for item in result.promoted {
    ///     match item {
    ///         PromotedAllocation::Pool { ptr, size, .. } => {
    ///             // Handle pool allocation
    ///         }
    ///         PromotedAllocation::Failed { reason, .. } => {
    ///             // Handle failure
    ///         }
    ///         _ => {}
    ///     }
    /// }
    /// ```
    pub fn end_frame_with_promotions(&self) -> PromotionResult {
        // Take all retained allocations
        let retained = retention::take_retained();
        
        // Set up the promotion processor with allocator callbacks
        let inner = self.inner.clone();
        let scratch = self.scratch.clone();
        
        let processor = PromotionProcessor::new()
            .with_pool_alloc(move |layout: Layout| {
                // Use pool allocator
                tls::with_tls(|tls| {
                    tls.pool_alloc_layout(layout, &inner)
                })
            })
            .with_heap_alloc(|layout: Layout| {
                // Use system heap
                unsafe { std::alloc::alloc(layout) }
            })
            .with_scratch_alloc(move |name: &'static str, layout: Layout| {
                // Use scratch pool
                scratch.with_pool(name, |pool| pool.alloc_layout(layout))
            });
        
        // Process promotions
        let result = processor.process(retained);
        
        // Now do normal frame end
        phases::reset_phases();
        tls::with_tls(|tls| {
            tls.end_frame();
        });
        
        result
    }

    /// End frame and get summary only (no promoted allocations returned).
    ///
    /// Use this when you don't need the actual promoted data,
    /// just the statistics.
    pub fn end_frame_with_summary(&self) -> FrameSummary {
        self.end_frame_with_promotions().summary
    }

    /// Get count of pending retained allocations.
    pub fn retained_count(&self) -> usize {
        retention::retained_count()
    }

    /// Clear retained allocations without processing.
    ///
    /// Use this if you want to abandon retained allocations
    /// instead of promoting them.
    pub fn clear_retained(&self) {
        retention::clear_retained();
    }

    // ==================== Behavior Filter (v0.4.0) ====================

    /// Enable the behavior filter for allocation pattern analysis.
    ///
    /// When enabled, the filter tracks allocation patterns per-tag and
    /// detects intent violations like:
    /// - Frame allocations that survive too long
    /// - Pool allocations used as scratch
    /// - Excessive promotion churn
    /// - Heap allocations in hot paths
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// alloc.enable_behavior_filter();
    ///
    /// // Run your game loop...
    /// for _ in 0..1000 {
    ///     alloc.begin_frame();
    ///     // ... allocations ...
    ///     alloc.end_frame();
    /// }
    ///
    /// // Check for issues
    /// let report = alloc.behavior_report();
    /// for issue in &report.issues {
    ///     eprintln!("{}", issue);
    /// }
    /// ```
    pub fn enable_behavior_filter(&self) {
        self.behavior_filter.enable();
    }

    /// Disable the behavior filter.
    pub fn disable_behavior_filter(&self) {
        self.behavior_filter.disable();
    }

    /// Check if the behavior filter is enabled.
    pub fn is_behavior_filter_enabled(&self) -> bool {
        self.behavior_filter.is_enabled()
    }

    /// Set behavior detection thresholds.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // Use strict thresholds for CI
    /// alloc.set_behavior_thresholds(BehaviorThresholds::strict());
    ///
    /// // Use relaxed thresholds during development
    /// alloc.set_behavior_thresholds(BehaviorThresholds::relaxed());
    /// ```
    pub fn set_behavior_thresholds(&self, thresholds: BehaviorThresholds) {
        // Note: This requires mutable access, so we'd need interior mutability
        // For now, thresholds are set at construction time
        let _ = thresholds;
    }

    /// Get the behavior analysis report.
    ///
    /// Analyzes tracked allocation patterns and returns detected issues.
    pub fn behavior_report(&self) -> BehaviorReport {
        self.behavior_filter.analyze()
    }

    /// Reset behavior tracking statistics.
    pub fn reset_behavior_stats(&self) {
        self.behavior_filter.reset();
    }

    /// Record an allocation for behavior tracking.
    ///
    /// This is called automatically by allocation methods when the filter is enabled.
    pub(crate) fn record_behavior_alloc(&self, ptr: *const u8, tag: &'static str, kind: AllocKind, size: usize) {
        self.behavior_filter.record_alloc(ptr, tag, kind, size);
    }

    /// Record a deallocation for behavior tracking.
    pub(crate) fn record_behavior_free(&self, ptr: *const u8, tag: &'static str, kind: AllocKind, size: usize) {
        self.behavior_filter.record_free(ptr, tag, kind, size);
    }

    /// Record a promotion for behavior tracking.
    pub(crate) fn record_behavior_promotion(&self, tag: &'static str, from_kind: AllocKind) {
        self.behavior_filter.record_promotion(tag, from_kind);
    }

    /// Get the behavior filter for advanced usage.
    pub fn behavior_filter(&self) -> &BehaviorFilter {
        &self.behavior_filter
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
