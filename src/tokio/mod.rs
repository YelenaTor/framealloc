//! Tokio integration for async-safe allocation (v0.8.0).
//!
//! This module provides async-safe allocation patterns for use with Tokio
//! and other async runtimes. It enforces the hybrid model where frame
//! allocations stay on the main thread and async tasks use pool/heap.
//!
//! # Philosophy
//!
//! - **Opt-in**: Requires `tokio` feature flag
//! - **Safe by default**: All allocations are pool-backed (never frame)
//! - **Task-scoped**: Allocations freed when task completes
//! - **Runtime-agnostic**: Works with Tokio, async-std, smol, etc.
//!
//! # Why Frame Allocations Are Unsafe in Async
//!
//! Frame allocations are tied to frame boundaries (`begin_frame`/`end_frame`).
//! When an async task suspends at an `.await` point, the main thread may call
//! `end_frame()`, invalidating any frame allocations the task holds.
//!
//! # The Hybrid Model
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │ Main Thread (Sync)              │ Async Tasks               │
//! ├─────────────────────────────────┼───────────────────────────┤
//! │ frame_alloc, frame_box ✓        │ frame_* ❌ UNSAFE         │
//! │ pool_alloc, pool_box ✓          │ pool_*, heap_* ✓ SAFE     │
//! │ heap_alloc, heap_box ✓          │ TaskAlloc ✓ SAFE          │
//! └─────────────────────────────────┴───────────────────────────┘
//! ```
//!
//! # Example
//!
//! ```rust,ignore
//! use framealloc::{SmartAlloc, AllocConfig};
//! use framealloc::tokio::TaskAlloc;
//!
//! let alloc = SmartAlloc::new(AllocConfig::default());
//!
//! // Main thread: frame allocations OK
//! alloc.begin_frame();
//! let scratch = alloc.frame_vec::<f32>();
//!
//! // Spawn async task: use TaskAlloc or pool_*
//! let alloc_clone = alloc.clone();
//! tokio::spawn(async move {
//!     let mut task = TaskAlloc::new(&alloc_clone);
//!     let data = task.alloc_box(expensive_computation().await);
//!     process(&data).await;
//!     // task drops → allocations freed
//! });
//!
//! alloc.end_frame();
//! ```

mod task_alloc;
mod async_pool_guard;

pub use task_alloc::{TaskAlloc, TaskBox};
pub use async_pool_guard::AsyncPoolGuard;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SmartAlloc, AllocConfig};

    #[test]
    fn task_alloc_basic() {
        let alloc = SmartAlloc::new(AllocConfig::default());
        
        {
            let mut task = TaskAlloc::new(&alloc);
            let _a = task.alloc_box(42u32);
            let _b = task.alloc_box(vec![1, 2, 3]);
            let _c = task.alloc_box("hello".to_string());
            
            assert_eq!(task.allocation_count(), 3);
        }
        // All allocations freed when task drops
    }

    #[test]
    fn async_pool_guard_basic() {
        let alloc = SmartAlloc::new(AllocConfig::default());
        
        {
            let guard = AsyncPoolGuard::new(&alloc);
            let _a = guard.alloc_box(100i32);
            let _b = guard.alloc_box(200i32);
            
            assert_eq!(guard.allocation_count(), 2);
        }
        // All allocations freed when guard drops
    }

    #[test]
    fn task_alloc_is_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}
        
        // TaskAlloc should be Send + Sync for use across await points
        assert_send::<TaskAlloc>();
        assert_sync::<TaskAlloc>();
    }
}
