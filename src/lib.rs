//! # framealloc
//!
//! Intent-aware, thread-smart memory allocation for Rust game engines.
//!
//! ## Features
//!
//! - Frame-based arenas (bump allocation, reset per frame)
//! - Thread-local fast paths (zero locks in common case)
//! - Automatic ST → MT scaling
//! - Optional Bevy integration
//! - Allocation diagnostics & budgeting
//! - Streaming allocator for large assets
//! - Handle-based allocation with relocation support
//! - Allocation groups for bulk freeing
//! - Safe wrapper types (FrameBox, PoolBox, HeapBox)
//! - std::alloc::Allocator trait implementations
//!
//! ## v0.2.0 Features
//!
//! - **Frame phases**: Named scopes within frames for profiling
//! - **Frame checkpoints**: Save/restore points for speculative allocation
//! - **Frame collections**: FrameVec, FrameMap with fixed capacity
//! - **Tagged allocations**: First-class allocation attribution
//! - **Scratch pools**: Cross-frame reusable memory
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use framealloc::{SmartAlloc, AllocConfig};
//!
//! let alloc = SmartAlloc::new(AllocConfig::default());
//!
//! // Game loop
//! alloc.begin_frame();
//! let temp = alloc.frame_alloc::<[f32; 256]>();
//! // ... use temp ...
//! alloc.end_frame();
//! ```

#[allow(dead_code)]
pub mod api;
#[allow(dead_code)]
pub mod diagnostics;
pub mod handles;
pub mod streaming;

#[allow(dead_code)]
mod allocators;
#[allow(dead_code)]
mod core;
#[allow(dead_code)]
mod sync;
#[allow(dead_code)]
mod util;

#[cfg(feature = "bevy")]
pub mod bevy;

#[cfg(feature = "debug")]
pub mod debug;

#[cfg(feature = "tokio")]
pub mod tokio;

// Re-export public API at crate root for convenience
pub use api::alloc::SmartAlloc;
pub use api::config::AllocConfig;
pub use api::scope::{FrameGuard, FrameScope};
pub use api::stats::AllocStats;
pub use api::tag::{AllocationIntent, AllocationTag};

// Safe wrapper types
pub use api::wrappers::{FrameBox, FrameSlice, PoolBox, HeapBox};

// Allocator trait implementations (nightly only)
#[cfg(feature = "nightly")]
pub use api::allocator_impl::{FrameAllocator, PoolAllocator, HeapAllocator};

// Allocation groups
pub use api::groups::{GroupAllocator, GroupId, GroupHandle, GroupStats};

// Handle-based allocation
pub use allocators::handles::{Handle, HandleAllocator, HandleAllocatorStats, PinGuard};

// Streaming allocation
pub use allocators::streaming::{StreamId, StreamPriority, StreamState, StreamingAllocator, StreamingStats};

// Budgets
pub use core::budget::{BudgetEvent, BudgetManager, BudgetStatus, TagBudget};

// Diagnostics - UI hooks
pub use diagnostics::{DiagnosticsHooks, DiagnosticsEvent, SharedDiagnostics, MemoryGraphData};
pub use diagnostics::{ProfilerHooks, ProfilerZone, MemoryEvent};
pub use diagnostics::{AllocatorSnapshot, SnapshotHistory};

// Diagnostics - Core types and predefined codes
pub use diagnostics::{Diagnostic, DiagnosticKind};
pub use diagnostics::{StrictMode, set_strict_mode, StrictModeGuard};
pub use diagnostics::{FA001, FA002, FA003, FA101, FA102, FA201, FA202, FA301, FA302, FA401, FA402, FA901};

// v0.2.0: Frame phases
pub use api::phases::{Phase, PhaseGuard, PhaseTracker};
pub use api::phases::{begin_phase, end_phase, current_phase, is_in_phase};

// v0.2.0: Frame checkpoints
pub use api::checkpoint::{FrameCheckpoint, CheckpointGuard, SpeculativeResult};

// v0.2.0: Frame collections
pub use api::frame_collections::{FrameVec, FrameVecIntoIter, FrameMap};

// v0.2.0: Tagged allocations
pub use api::tagged::{TagGuard, TagStack, with_tag, current_tag, tag_path};

// v0.2.0: Scratch pools
pub use api::scratch::{ScratchPool, ScratchRegistry, ScratchPoolHandle, ScratchPoolStats};

// v0.3.0: Frame retention and promotion
pub use api::retention::{RetentionPolicy, Importance, FrameRetained, PromotedAllocation, PromotionFailure};
pub use api::promotion::{FrameSummary, PromotionResult, FailureBreakdown, TagSummary, PhaseSummary};

// v0.4.0: Behavior filter and memory intent analysis
pub use diagnostics::behavior::{
    AllocKind, BehaviorFilter, BehaviorIssue, BehaviorReport, BehaviorThresholds, TagBehaviorStats,
    FA501, FA502, FA510, FA520, FA530,
};
pub use diagnostics::{DiagnosticCode, DiagnosticLevel};

// v0.6.0: Thread coordination and observability
pub use api::transfer::{TransferHandle, TransferId, TransferState, TransferStats, TransferRegistry};
pub use api::barrier::{FrameBarrier, FrameBarrierBuilder, BarrierStats};
pub use api::lifecycle::{FrameEvent, LifecycleManager, LifecycleSummary, ThreadFrameStats, FrameLifecycleGuard};
pub use api::thread_budget::{
    ThreadBudgetManager, ThreadBudgetConfig, ThreadBudgetState, ThreadBudgetStats,
    BudgetExceededPolicy, BudgetCheckResult,
};
pub use api::deferred_control::{
    DeferredProcessing, DeferredConfig, DeferredController, DeferredStats as DeferredControlStats,
    QueueFullPolicy, QueueResult, DeferredConfigBuilder,
};

// v0.7.0: IDE integration and snapshots
pub use api::snapshot::{
    Snapshot, SnapshotConfig, SnapshotEmitter, SnapshotSummary,
    ThreadSnapshot, TagSnapshot, BudgetInfo, 
    PromotionStats as SnapshotPromotionStats, 
    TransferStats as SnapshotTransferStats,
    DeferredStats as SnapshotDeferredStats, 
    RuntimeDiagnostic, SNAPSHOT_VERSION,
};
