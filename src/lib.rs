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

// Internal modules (not directly exported)
#[allow(dead_code)]
mod api;
#[allow(dead_code)]
mod allocators;
#[allow(dead_code)]
mod core;
#[allow(dead_code)]
mod sync;
#[allow(dead_code)]
mod util;
#[allow(dead_code)]
mod diagnostics;
mod handles;
mod streaming;

// Feature-gated modules
#[cfg(feature = "rapier")]
pub mod rapier;

#[cfg(feature = "bevy")]
pub mod bevy;

#[cfg(feature = "debug")]
pub mod debug;

#[cfg(feature = "tokio")]
pub mod tokio;

// CPU module - always available
pub mod cpu;

// GPU module - only available with 'gpu' feature
#[cfg(feature = "gpu")]
pub mod gpu;

// Coordinator module - only available with both 'gpu' and 'coordinator' features
#[cfg(all(feature = "gpu", feature = "coordinator"))]
pub mod coordinator;

// Re-export all CPU functionality for backward compatibility
pub use cpu::*;
