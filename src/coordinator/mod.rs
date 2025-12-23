//! CPU-GPU Memory Coordination
//!
//! **Requires both `gpu` and `coordinator` features.**
//! This module bridges the CPU and GPU allocators.

pub mod unified_allocator;
pub mod sync;
pub mod budget;

pub use unified_allocator::UnifiedAllocator;
pub use sync::{CpuGpuBarrier, TransferDirection};
pub use budget::UnifiedBudgetManager;
