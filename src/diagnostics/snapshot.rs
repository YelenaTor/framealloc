//! Memory snapshots for diagnostics.

use std::time::Instant;

/// Complete snapshot of allocator state at a point in time.
#[derive(Debug, Clone)]
pub struct AllocatorSnapshot {
    /// When this snapshot was taken
    pub timestamp: Instant,
    
    /// Frame number when snapshot was taken
    pub frame_number: u64,
    
    /// Global memory statistics
    pub global: GlobalSnapshot,
    
    /// Per-frame arena statistics
    pub frame_arenas: Vec<FrameSnapshot>,
    
    /// Per-pool statistics
    pub pools: Vec<PoolSnapshot>,
    
    /// Per-tag budget statistics
    pub tags: Vec<TagSnapshot>,
    
    /// Streaming allocator statistics (if enabled)
    pub streaming: Option<StreamingSnapshot>,
}

impl AllocatorSnapshot {
    /// Create a new empty snapshot.
    pub fn new(frame_number: u64) -> Self {
        Self {
            timestamp: Instant::now(),
            frame_number,
            global: GlobalSnapshot::default(),
            frame_arenas: Vec::new(),
            pools: Vec::new(),
            tags: Vec::new(),
            streaming: None,
        }
    }
}

/// Global allocator statistics.
#[derive(Debug, Clone, Default)]
pub struct GlobalSnapshot {
    /// Total bytes currently allocated
    pub total_allocated: usize,
    
    /// Peak bytes allocated
    pub peak_allocated: usize,
    
    /// Total allocation count
    pub allocation_count: u64,
    
    /// Total deallocation count
    pub deallocation_count: u64,
    
    /// Bytes in frame arenas
    pub frame_bytes: usize,
    
    /// Bytes in pools
    pub pool_bytes: usize,
    
    /// Bytes in heap
    pub heap_bytes: usize,
}

/// Per-thread frame arena snapshot.
#[derive(Debug, Clone)]
pub struct FrameSnapshot {
    /// Thread ID (if available)
    pub thread_id: Option<u64>,
    
    /// Thread name (if available)
    pub thread_name: Option<String>,
    
    /// Arena capacity
    pub capacity: usize,
    
    /// Current usage
    pub used: usize,
    
    /// Peak usage this session
    pub peak: usize,
    
    /// Allocations this frame
    pub allocations_this_frame: u64,
}

impl FrameSnapshot {
    /// Calculate usage percentage.
    pub fn usage_percent(&self) -> f64 {
        if self.capacity == 0 {
            0.0
        } else {
            (self.used as f64 / self.capacity as f64) * 100.0
        }
    }
}

/// Per-size-class pool snapshot.
#[derive(Debug, Clone)]
pub struct PoolSnapshot {
    /// Size class (bytes per object)
    pub size_class: usize,
    
    /// Objects currently in use
    pub in_use: usize,
    
    /// Objects available in free list
    pub available: usize,
    
    /// Total objects allocated from system
    pub total_objects: usize,
    
    /// Number of refills from global
    pub refill_count: u64,
}

impl PoolSnapshot {
    /// Calculate pool efficiency (in_use / total).
    pub fn efficiency_percent(&self) -> f64 {
        if self.total_objects == 0 {
            100.0
        } else {
            (self.in_use as f64 / self.total_objects as f64) * 100.0
        }
    }
    
    /// Calculate total memory for this pool.
    pub fn total_bytes(&self) -> usize {
        self.total_objects * self.size_class
    }
}

/// Per-tag budget snapshot.
#[derive(Debug, Clone)]
pub struct TagSnapshot {
    /// Tag name
    pub name: String,
    
    /// Current usage
    pub current_usage: usize,
    
    /// Peak usage
    pub peak_usage: usize,
    
    /// Soft limit
    pub soft_limit: usize,
    
    /// Hard limit
    pub hard_limit: usize,
    
    /// Allocation count
    pub allocation_count: u64,
    
    /// Deallocation count
    pub deallocation_count: u64,
}

impl TagSnapshot {
    /// Calculate usage percentage relative to hard limit.
    pub fn usage_percent(&self) -> f64 {
        if self.hard_limit == 0 {
            0.0
        } else {
            (self.current_usage as f64 / self.hard_limit as f64) * 100.0
        }
    }
    
    /// Check if over soft limit.
    pub fn is_warning(&self) -> bool {
        self.soft_limit > 0 && self.current_usage > self.soft_limit
    }
    
    /// Check if over hard limit.
    pub fn is_exceeded(&self) -> bool {
        self.hard_limit > 0 && self.current_usage > self.hard_limit
    }
}

/// Streaming allocator snapshot.
#[derive(Debug, Clone)]
pub struct StreamingSnapshot {
    /// Budget
    pub budget: usize,
    
    /// Total reserved
    pub reserved: usize,
    
    /// Total loaded
    pub loaded: usize,
    
    /// Number of allocations
    pub allocation_count: usize,
    
    /// Allocations by state
    pub by_state: StreamingStateCount,
}

/// Count of streaming allocations by state.
#[derive(Debug, Clone, Default)]
pub struct StreamingStateCount {
    pub reserved: usize,
    pub loading: usize,
    pub ready: usize,
    pub evicting: usize,
}

/// History of snapshots for graphing.
#[derive(Debug, Clone)]
pub struct SnapshotHistory {
    /// Maximum number of snapshots to keep
    max_snapshots: usize,
    
    /// Historical snapshots
    snapshots: Vec<AllocatorSnapshot>,
}

impl SnapshotHistory {
    /// Create a new history with the given capacity.
    pub fn new(max_snapshots: usize) -> Self {
        Self {
            max_snapshots,
            snapshots: Vec::with_capacity(max_snapshots),
        }
    }
    
    /// Add a snapshot to the history.
    pub fn push(&mut self, snapshot: AllocatorSnapshot) {
        if self.snapshots.len() >= self.max_snapshots {
            self.snapshots.remove(0);
        }
        self.snapshots.push(snapshot);
    }
    
    /// Get all snapshots.
    pub fn snapshots(&self) -> &[AllocatorSnapshot] {
        &self.snapshots
    }
    
    /// Get the most recent snapshot.
    pub fn latest(&self) -> Option<&AllocatorSnapshot> {
        self.snapshots.last()
    }
    
    /// Get memory usage over time for graphing.
    pub fn memory_timeline(&self) -> Vec<(u64, usize)> {
        self.snapshots
            .iter()
            .map(|s| (s.frame_number, s.global.total_allocated))
            .collect()
    }
    
    /// Get peak memory over time.
    pub fn peak_timeline(&self) -> Vec<(u64, usize)> {
        self.snapshots
            .iter()
            .map(|s| (s.frame_number, s.global.peak_allocated))
            .collect()
    }
    
    /// Clear all history.
    pub fn clear(&mut self) {
        self.snapshots.clear();
    }
}

impl Default for SnapshotHistory {
    fn default() -> Self {
        Self::new(300) // ~5 seconds at 60fps
    }
}
