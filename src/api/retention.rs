//! Frame retention policies - control what happens to allocations at frame end.
//!
//! By default, frame allocations are discarded when `end_frame()` is called.
//! Retention policies allow allocations to "escape" the frame by being
//! promoted to longer-lived allocators.
//!
//! # Philosophy
//!
//! This is NOT automatic garbage collection. Retention is:
//! - **Explicit**: You must opt-in per allocation
//! - **Deterministic**: Decisions happen at one point (frame end)
//! - **Bounded**: Subject to budgets and limits
//!
//! # Example
//!
//! ```rust,ignore
//! // Allocate with retention policy
//! let handle = alloc.frame_retained::<NavMesh>(RetentionPolicy::PromoteToPool);
//!
//! // Use during the frame
//! handle.get_mut().calculate();
//!
//! // At frame end, take the promoted allocation
//! let promoted = alloc.end_frame_with_promotions();
//! for item in promoted {
//!     // item is now a PoolBox or HeapBox
//! }
//! ```

use std::cell::RefCell;
use std::marker::PhantomData;

/// Policy for what happens to a frame allocation at frame end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetentionPolicy {
    /// Discard at frame end (default behavior).
    Discard,
    
    /// Promote to pool allocator at frame end.
    /// Returns a `PoolBox<T>` that must be explicitly freed.
    PromoteToPool,
    
    /// Promote to heap allocator at frame end.
    /// Returns a `HeapBox<T>` that auto-frees on drop.
    PromoteToHeap,
    
    /// Promote to a named scratch pool at frame end.
    /// The allocation persists until the scratch pool is reset.
    PromoteToScratch(&'static str),
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self::Discard
    }
}

impl RetentionPolicy {
    /// Check if this policy will promote the allocation.
    pub fn promotes(&self) -> bool {
        !matches!(self, Self::Discard)
    }
    
    /// Get the destination name for diagnostics.
    pub fn destination(&self) -> &'static str {
        match self {
            Self::Discard => "discard",
            Self::PromoteToPool => "pool",
            Self::PromoteToHeap => "heap",
            Self::PromoteToScratch(name) => name,
        }
    }
}

/// Semantic importance level that maps to retention policies.
///
/// This is sugar over `RetentionPolicy` for more intuitive usage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Importance {
    /// Ephemeral - discard at frame end.
    /// Use for truly temporary scratch data.
    Ephemeral,
    
    /// Reusable - promote to pool for potential reuse.
    /// Use for data that might be needed next frame.
    Reusable,
    
    /// Persistent - promote to heap for long-term storage.
    /// Use for data that should outlive multiple frames.
    Persistent,
    
    /// Scratch - promote to named scratch pool.
    /// Use for subsystem-specific semi-persistent data.
    Scratch(&'static str),
}

impl Importance {
    /// Convert importance to retention policy.
    pub fn to_policy(self) -> RetentionPolicy {
        match self {
            Self::Ephemeral => RetentionPolicy::Discard,
            Self::Reusable => RetentionPolicy::PromoteToPool,
            Self::Persistent => RetentionPolicy::PromoteToHeap,
            Self::Scratch(name) => RetentionPolicy::PromoteToScratch(name),
        }
    }
}

impl From<Importance> for RetentionPolicy {
    fn from(importance: Importance) -> Self {
        importance.to_policy()
    }
}

impl Default for Importance {
    fn default() -> Self {
        Self::Ephemeral
    }
}

/// Metadata for a retained frame allocation.
#[derive(Debug, Clone)]
pub struct RetainedMeta {
    /// The retention policy
    pub policy: RetentionPolicy,
    /// Size in bytes
    pub size: usize,
    /// Allocation tag (if any)
    pub tag: Option<&'static str>,
    /// Type name (for diagnostics)
    pub type_name: &'static str,
}

/// Internal tracking for retained allocations.
pub(crate) struct RetainedAllocation {
    /// Pointer to the data
    pub ptr: *mut u8,
    /// Metadata
    pub meta: RetainedMeta,
    /// Promotion callback
    pub promote_fn: Box<dyn FnOnce(*mut u8) -> PromotedAllocation>,
}

// Safety: We only access this from the thread that created it
unsafe impl Send for RetainedAllocation {}

/// A promoted allocation after frame end.
#[derive(Debug)]
pub enum PromotedAllocation {
    /// Promoted to pool allocator
    Pool {
        ptr: *mut u8,
        size: usize,
        tag: Option<&'static str>,
        type_name: &'static str,
    },
    /// Promoted to heap allocator
    Heap {
        ptr: *mut u8,
        size: usize,
        tag: Option<&'static str>,
        type_name: &'static str,
    },
    /// Promoted to scratch pool
    Scratch {
        pool_name: &'static str,
        ptr: *mut u8,
        size: usize,
        tag: Option<&'static str>,
        type_name: &'static str,
    },
    /// Promotion failed (budget exceeded, etc.)
    Failed {
        reason: PromotionFailure,
        meta: RetainedMeta,
    },
}

impl PromotedAllocation {
    /// Check if promotion succeeded.
    pub fn is_success(&self) -> bool {
        !matches!(self, Self::Failed { .. })
    }
    
    /// Get the size of the allocation.
    pub fn size(&self) -> usize {
        match self {
            Self::Pool { size, .. } => *size,
            Self::Heap { size, .. } => *size,
            Self::Scratch { size, .. } => *size,
            Self::Failed { meta, .. } => meta.size,
        }
    }
    
    /// Get the tag (if any).
    pub fn tag(&self) -> Option<&'static str> {
        match self {
            Self::Pool { tag, .. } => *tag,
            Self::Heap { tag, .. } => *tag,
            Self::Scratch { tag, .. } => *tag,
            Self::Failed { meta, .. } => meta.tag,
        }
    }
    
    /// Get the type name.
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Pool { type_name, .. } => type_name,
            Self::Heap { type_name, .. } => type_name,
            Self::Scratch { type_name, .. } => type_name,
            Self::Failed { meta, .. } => meta.type_name,
        }
    }
}

/// Reason for promotion failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromotionFailure {
    /// Budget exceeded for destination allocator
    BudgetExceeded,
    /// Scratch pool not found
    ScratchPoolNotFound,
    /// Scratch pool full
    ScratchPoolFull,
    /// Allocation too large for destination
    TooLarge,
    /// Internal error
    InternalError,
}

impl std::fmt::Display for PromotionFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BudgetExceeded => write!(f, "budget exceeded"),
            Self::ScratchPoolNotFound => write!(f, "scratch pool not found"),
            Self::ScratchPoolFull => write!(f, "scratch pool full"),
            Self::TooLarge => write!(f, "allocation too large"),
            Self::InternalError => write!(f, "internal error"),
        }
    }
}

/// Handle to a frame allocation with retention policy.
///
/// This wraps a frame allocation and tracks it for potential promotion
/// at frame end.
pub struct FrameRetained<'a, T> {
    ptr: *mut T,
    id: usize,
    _marker: PhantomData<&'a mut T>,
}

impl<'a, T> FrameRetained<'a, T> {
    /// Create a new retained handle.
    pub(crate) fn new(ptr: *mut T, id: usize) -> Self {
        Self {
            ptr,
            id,
            _marker: PhantomData,
        }
    }
    
    /// Get a reference to the data.
    pub fn get(&self) -> &T {
        unsafe { &*self.ptr }
    }
    
    /// Get a mutable reference to the data.
    pub fn get_mut(&mut self) -> &mut T {
        unsafe { &mut *self.ptr }
    }
    
    /// Get the raw pointer.
    pub fn as_ptr(&self) -> *const T {
        self.ptr
    }
    
    /// Get the raw mutable pointer.
    pub fn as_mut_ptr(&mut self) -> *mut T {
        self.ptr
    }
    
    /// Get the tracking ID.
    pub fn id(&self) -> usize {
        self.id
    }
}

impl<'a, T> std::ops::Deref for FrameRetained<'a, T> {
    type Target = T;
    
    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

impl<'a, T> std::ops::DerefMut for FrameRetained<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.get_mut()
    }
}

/// Thread-local registry of retained allocations.
pub(crate) struct RetentionRegistry {
    allocations: Vec<RetainedAllocation>,
    next_id: usize,
}

impl RetentionRegistry {
    pub fn new() -> Self {
        Self {
            allocations: Vec::with_capacity(64),
            next_id: 0,
        }
    }
    
    pub fn register(&mut self, alloc: RetainedAllocation) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        self.allocations.push(alloc);
        id
    }
    
    pub fn take_all(&mut self) -> Vec<RetainedAllocation> {
        self.next_id = 0;
        std::mem::take(&mut self.allocations)
    }
    
    pub fn clear(&mut self) {
        self.allocations.clear();
        self.next_id = 0;
    }
    
    pub fn len(&self) -> usize {
        self.allocations.len()
    }
    
    pub fn is_empty(&self) -> bool {
        self.allocations.is_empty()
    }
}

impl Default for RetentionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

thread_local! {
    pub(crate) static RETENTION_REGISTRY: RefCell<RetentionRegistry> = 
        RefCell::new(RetentionRegistry::new());
}

/// Register a retained allocation.
pub(crate) fn register_retained(alloc: RetainedAllocation) -> usize {
    RETENTION_REGISTRY.with(|r| r.borrow_mut().register(alloc))
}

/// Take all retained allocations for processing.
pub(crate) fn take_retained() -> Vec<RetainedAllocation> {
    RETENTION_REGISTRY.with(|r| r.borrow_mut().take_all())
}

/// Clear retained allocations without processing.
pub(crate) fn clear_retained() {
    RETENTION_REGISTRY.with(|r| r.borrow_mut().clear());
}

/// Get count of retained allocations.
pub(crate) fn retained_count() -> usize {
    RETENTION_REGISTRY.with(|r| r.borrow().len())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_retention_policy_default() {
        assert_eq!(RetentionPolicy::default(), RetentionPolicy::Discard);
    }
    
    #[test]
    fn test_importance_to_policy() {
        assert_eq!(Importance::Ephemeral.to_policy(), RetentionPolicy::Discard);
        assert_eq!(Importance::Reusable.to_policy(), RetentionPolicy::PromoteToPool);
        assert_eq!(Importance::Persistent.to_policy(), RetentionPolicy::PromoteToHeap);
    }
    
    #[test]
    fn test_policy_promotes() {
        assert!(!RetentionPolicy::Discard.promotes());
        assert!(RetentionPolicy::PromoteToPool.promotes());
        assert!(RetentionPolicy::PromoteToHeap.promotes());
        assert!(RetentionPolicy::PromoteToScratch("test").promotes());
    }
    
    #[test]
    fn test_retention_registry() {
        clear_retained();
        
        let meta = RetainedMeta {
            policy: RetentionPolicy::PromoteToPool,
            size: 64,
            tag: None,
            type_name: "TestType",
        };
        
        let alloc = RetainedAllocation {
            ptr: std::ptr::null_mut(),
            meta,
            promote_fn: Box::new(|_| PromotedAllocation::Pool {
                ptr: std::ptr::null_mut(),
                size: 64,
                tag: None,
                type_name: "TestType",
            }),
        };
        
        let id = register_retained(alloc);
        assert_eq!(id, 0);
        assert_eq!(retained_count(), 1);
        
        let taken = take_retained();
        assert_eq!(taken.len(), 1);
        assert_eq!(retained_count(), 0);
    }
}
