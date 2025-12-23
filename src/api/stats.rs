//! Allocation statistics.

/// Aggregated allocation statistics.
#[derive(Debug, Clone, Default)]
pub struct AllocStats {
    /// Total bytes currently allocated.
    pub total_allocated: usize,

    /// Peak bytes allocated (high water mark).
    pub peak_allocated: usize,

    /// Total number of allocations performed.
    pub allocation_count: u64,

    /// Total number of deallocations performed.
    pub deallocation_count: u64,

    /// Bytes allocated from frame arenas.
    pub frame_allocated: usize,

    /// Bytes allocated from slab pools.
    pub pool_allocated: usize,

    /// Bytes allocated from system heap.
    pub heap_allocated: usize,

    /// Number of slab refills from global.
    pub slab_refill_count: u64,

    /// Number of cross-thread frees processed.
    pub deferred_free_count: u64,
}

impl AllocStats {
    /// Create empty stats.
    pub fn new() -> Self {
        Self::default()
    }

    /// Calculate active allocations.
    pub fn active_allocations(&self) -> u64 {
        self.allocation_count.saturating_sub(self.deallocation_count)
    }

    /// Calculate fragmentation estimate (pool + heap vs total).
    pub fn fragmentation_ratio(&self) -> f64 {
        if self.total_allocated == 0 {
            return 0.0;
        }
        let non_frame = self.pool_allocated + self.heap_allocated;
        non_frame as f64 / self.total_allocated as f64
    }
}

impl std::fmt::Display for AllocStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Allocation Statistics:")?;
        writeln!(f, "  Total allocated: {} bytes", self.total_allocated)?;
        writeln!(f, "  Peak allocated:  {} bytes", self.peak_allocated)?;
        writeln!(f, "  Allocations:     {}", self.allocation_count)?;
        writeln!(f, "  Deallocations:   {}", self.deallocation_count)?;
        writeln!(f, "  Active:          {}", self.active_allocations())?;
        writeln!(f, "  Frame arena:     {} bytes", self.frame_allocated)?;
        writeln!(f, "  Pool:            {} bytes", self.pool_allocated)?;
        writeln!(f, "  Heap:            {} bytes", self.heap_allocated)?;
        Ok(())
    }
}

/// Per-thread statistics (aggregated into global stats).
#[derive(Debug, Default)]
pub(crate) struct ThreadStats {
    pub alloc_count: usize,
    pub dealloc_count: usize,
    pub bytes_allocated: usize,
    pub bytes_deallocated: usize,
}

impl ThreadStats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_alloc(&mut self, size: usize) {
        self.alloc_count += 1;
        self.bytes_allocated += size;
    }

    pub fn record_dealloc(&mut self, size: usize) {
        self.dealloc_count += 1;
        self.bytes_deallocated += size;
    }
}
