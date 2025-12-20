//! Global shared state.

use std::alloc::Layout;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::allocators::heap::SystemHeap;
use crate::allocators::slab::SlabRegistry;
use crate::api::config::AllocConfig;
use crate::api::stats::AllocStats;
use crate::core::budget::BudgetManager;

/// Global state shared across all threads.
///
/// This is wrapped in an `Arc` by `SmartAlloc` for thread-safe sharing.
pub struct GlobalState {
    /// Configuration
    config: AllocConfig,

    /// System heap for large allocations
    heap: SystemHeap,

    /// Slab registry for small object pools
    slabs: SlabRegistry,

    /// Budget manager (optional)
    budgets: Option<BudgetManager>,

    /// Global statistics (atomics)
    total_allocated: AtomicUsize,
    peak_allocated: AtomicUsize,
    allocation_count: AtomicUsize,
    deallocation_count: AtomicUsize,
}

impl GlobalState {
    /// Create new global state with the given configuration.
    pub fn new(config: AllocConfig) -> Self {
        let budgets = if config.enable_budgets {
            Some(BudgetManager::new(config.global_memory_limit))
        } else {
            None
        };

        Self {
            slabs: SlabRegistry::new(&config),
            heap: SystemHeap::new(),
            budgets,
            config,
            total_allocated: AtomicUsize::new(0),
            peak_allocated: AtomicUsize::new(0),
            allocation_count: AtomicUsize::new(0),
            deallocation_count: AtomicUsize::new(0),
        }
    }

    /// Get the configuration.
    pub fn config(&self) -> &AllocConfig {
        &self.config
    }

    /// Get the slab registry.
    pub fn slabs(&self) -> &SlabRegistry {
        &self.slabs
    }

    /// Allocate from system heap.
    pub fn heap_alloc<T>(&self) -> *mut T {
        let layout = Layout::new::<T>();
        let ptr = self.heap.alloc(layout);
        
        if !ptr.is_null() {
            self.record_alloc(layout.size());
        }
        
        ptr as *mut T
    }

    /// Free to system heap.
    ///
    /// # Safety
    /// Pointer must have been allocated by `heap_alloc`.
    pub unsafe fn heap_free<T>(&self, ptr: *mut T) {
        let layout = Layout::new::<T>();
        self.heap.dealloc(ptr as *mut u8, layout);
        self.record_dealloc(layout.size());
    }

    /// Record an allocation in global stats.
    pub fn record_alloc(&self, size: usize) {
        self.allocation_count.fetch_add(1, Ordering::Relaxed);
        let new_total = self.total_allocated.fetch_add(size, Ordering::Relaxed) + size;
        
        // Update peak if needed
        let mut peak = self.peak_allocated.load(Ordering::Relaxed);
        while new_total > peak {
            match self.peak_allocated.compare_exchange_weak(
                peak,
                new_total,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(p) => peak = p,
            }
        }

        // Check budget if enabled
        if let Some(ref budgets) = self.budgets {
            budgets.check_allocation(size, new_total);
        }
    }

    /// Record a deallocation in global stats.
    pub fn record_dealloc(&self, size: usize) {
        self.deallocation_count.fetch_add(1, Ordering::Relaxed);
        self.total_allocated.fetch_sub(size, Ordering::Relaxed);
    }

    /// Get current statistics.
    pub fn stats(&self) -> AllocStats {
        AllocStats {
            total_allocated: self.total_allocated.load(Ordering::Relaxed),
            peak_allocated: self.peak_allocated.load(Ordering::Relaxed),
            allocation_count: self.allocation_count.load(Ordering::Relaxed) as u64,
            deallocation_count: self.deallocation_count.load(Ordering::Relaxed) as u64,
            frame_allocated: 0,  // TODO: aggregate from TLS
            pool_allocated: 0,   // TODO: aggregate from TLS
            heap_allocated: self.heap.allocated_bytes(),
            slab_refill_count: self.slabs.refill_count(),
            deferred_free_count: 0, // TODO: aggregate from TLS
        }
    }

    /// Get the budget manager if enabled.
    pub fn budgets(&self) -> Option<&BudgetManager> {
        self.budgets.as_ref()
    }

    /// Allocate from system heap with a specific layout.
    pub fn heap_alloc_layout(&self, layout: Layout) -> *mut u8 {
        let ptr = self.heap.alloc(layout);
        if !ptr.is_null() {
            self.record_alloc(layout.size());
        }
        ptr
    }

    /// Free to system heap with a specific layout.
    ///
    /// # Safety
    /// Pointer must have been allocated with the same layout.
    pub unsafe fn heap_free_layout(&self, ptr: *mut u8, layout: Layout) {
        self.heap.dealloc(ptr, layout);
        self.record_dealloc(layout.size());
    }
}

// Safety: All fields are either Sync or protected by synchronization
unsafe impl Send for GlobalState {}
unsafe impl Sync for GlobalState {}
