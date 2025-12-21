//! Deferred processing control for v0.6.0.
//!
//! Provides explicit control over when and how deferred cross-thread
//! frees are processed, making the cost predictable.

use std::sync::atomic::{AtomicUsize, AtomicBool, Ordering};

/// Processing mode for deferred frees.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeferredProcessing {
    /// Process all deferred frees at frame begin (default, current behavior).
    AtFrameBegin,
    /// Process deferred frees at frame end.
    AtFrameEnd,
    /// Process in batches during frame (amortized).
    Incremental {
        /// Maximum frees to process per allocation.
        per_alloc: usize,
    },
    /// Manual control - developer calls process_deferred().
    Explicit,
    /// Disabled - deferred frees accumulate until explicitly processed.
    Disabled,
}

impl Default for DeferredProcessing {
    fn default() -> Self {
        Self::AtFrameBegin
    }
}

/// Policy when deferred queue reaches capacity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueFullPolicy {
    /// Process immediately (blocking).
    ProcessImmediately,
    /// Drop oldest entries (lossy but non-blocking).
    DropOldest,
    /// Fail the free operation (caller must handle).
    Fail,
    /// Grow the queue (unbounded, current behavior).
    Grow,
}

impl Default for QueueFullPolicy {
    fn default() -> Self {
        Self::Grow
    }
}

/// Configuration for deferred processing.
#[derive(Debug, Clone)]
pub struct DeferredConfig {
    /// Processing mode.
    pub mode: DeferredProcessing,
    /// Maximum queue capacity (0 = unbounded).
    pub capacity: usize,
    /// Policy when queue is full.
    pub full_policy: QueueFullPolicy,
    /// High water mark for warnings.
    pub warning_threshold: usize,
}

impl Default for DeferredConfig {
    fn default() -> Self {
        Self {
            mode: DeferredProcessing::AtFrameBegin,
            capacity: 0, // Unbounded
            full_policy: QueueFullPolicy::Grow,
            warning_threshold: 1024,
        }
    }
}

impl DeferredConfig {
    /// Create a bounded configuration.
    pub fn bounded(capacity: usize) -> Self {
        Self {
            mode: DeferredProcessing::AtFrameBegin,
            capacity,
            full_policy: QueueFullPolicy::ProcessImmediately,
            warning_threshold: capacity * 80 / 100,
        }
    }

    /// Create an incremental processing configuration.
    pub fn incremental(per_alloc: usize) -> Self {
        Self {
            mode: DeferredProcessing::Incremental { per_alloc },
            capacity: 0,
            full_policy: QueueFullPolicy::Grow,
            warning_threshold: 1024,
        }
    }

    /// Create an explicit control configuration.
    pub fn explicit() -> Self {
        Self {
            mode: DeferredProcessing::Explicit,
            capacity: 0,
            full_policy: QueueFullPolicy::Grow,
            warning_threshold: 1024,
        }
    }
}

/// Statistics about deferred processing.
#[derive(Debug, Default, Clone)]
pub struct DeferredStats {
    /// Total frees queued.
    pub total_queued: u64,
    /// Total frees processed.
    pub total_processed: u64,
    /// Current queue depth.
    pub current_depth: usize,
    /// Peak queue depth.
    pub peak_depth: usize,
    /// Total bytes in queue.
    pub queued_bytes: usize,
    /// Number of times queue was full.
    pub full_count: u64,
    /// Number of times warning threshold was hit.
    pub warning_count: u64,
    /// Number of immediate processing events.
    pub immediate_process_count: u64,
}

/// Controller for deferred processing.
pub struct DeferredController {
    /// Configuration.
    config: DeferredConfig,
    /// Current queue depth.
    depth: AtomicUsize,
    /// Peak depth.
    peak: AtomicUsize,
    /// Queued bytes.
    bytes: AtomicUsize,
    /// Warning issued flag.
    warning_issued: AtomicBool,
    /// Statistics.
    stats: std::sync::Mutex<DeferredStats>,
}

impl DeferredController {
    /// Create a new controller with the given configuration.
    pub fn new(config: DeferredConfig) -> Self {
        Self {
            config,
            depth: AtomicUsize::new(0),
            peak: AtomicUsize::new(0),
            bytes: AtomicUsize::new(0),
            warning_issued: AtomicBool::new(false),
            stats: std::sync::Mutex::new(DeferredStats::default()),
        }
    }

    /// Get the current configuration.
    pub fn config(&self) -> &DeferredConfig {
        &self.config
    }

    /// Update the configuration.
    pub fn set_config(&mut self, config: DeferredConfig) {
        self.config = config;
    }

    /// Check if a free can be queued.
    pub fn can_queue(&self) -> bool {
        if self.config.capacity == 0 {
            return true; // Unbounded
        }
        self.depth.load(Ordering::Relaxed) < self.config.capacity
    }

    /// Record a queued free.
    pub fn record_queued(&self, size: usize) -> QueueResult {
        let new_depth = self.depth.fetch_add(1, Ordering::Relaxed) + 1;
        self.peak.fetch_max(new_depth, Ordering::Relaxed);
        self.bytes.fetch_add(size, Ordering::Relaxed);

        let mut stats = self.stats.lock().unwrap();
        stats.total_queued += 1;
        stats.current_depth = new_depth;
        if new_depth > stats.peak_depth {
            stats.peak_depth = new_depth;
        }
        stats.queued_bytes = self.bytes.load(Ordering::Relaxed);

        // Check capacity
        if self.config.capacity > 0 && new_depth >= self.config.capacity {
            stats.full_count += 1;
            return QueueResult::Full(self.config.full_policy);
        }

        // Check warning threshold
        if new_depth >= self.config.warning_threshold 
            && !self.warning_issued.swap(true, Ordering::Relaxed) 
        {
            stats.warning_count += 1;
            return QueueResult::Warning;
        }

        QueueResult::Ok
    }

    /// Record processed frees.
    pub fn record_processed(&self, count: usize, bytes: usize) {
        self.depth.fetch_sub(count, Ordering::Relaxed);
        self.bytes.fetch_sub(bytes, Ordering::Relaxed);

        let mut stats = self.stats.lock().unwrap();
        stats.total_processed += count as u64;
        stats.current_depth = self.depth.load(Ordering::Relaxed);
        stats.queued_bytes = self.bytes.load(Ordering::Relaxed);
    }

    /// Get current queue depth.
    pub fn depth(&self) -> usize {
        self.depth.load(Ordering::Relaxed)
    }

    /// Get queued bytes.
    pub fn queued_bytes(&self) -> usize {
        self.bytes.load(Ordering::Relaxed)
    }

    /// Should process based on current mode?
    pub fn should_process_at_frame_begin(&self) -> bool {
        matches!(self.config.mode, DeferredProcessing::AtFrameBegin)
    }

    /// Should process at frame end?
    pub fn should_process_at_frame_end(&self) -> bool {
        matches!(self.config.mode, DeferredProcessing::AtFrameEnd)
    }

    /// Get incremental process count (if in incremental mode).
    pub fn incremental_count(&self) -> Option<usize> {
        match self.config.mode {
            DeferredProcessing::Incremental { per_alloc } => Some(per_alloc),
            _ => None,
        }
    }

    /// Reset warning flag (called at frame boundaries).
    pub fn reset_warning(&self) {
        self.warning_issued.store(false, Ordering::Relaxed);
    }

    /// Get current statistics.
    pub fn stats(&self) -> DeferredStats {
        self.stats.lock().unwrap().clone()
    }

    /// Reset statistics.
    pub fn reset_stats(&self) {
        let mut stats = self.stats.lock().unwrap();
        *stats = DeferredStats::default();
    }
}

impl Default for DeferredController {
    fn default() -> Self {
        Self::new(DeferredConfig::default())
    }
}

/// Result of attempting to queue a deferred free.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueResult {
    /// Successfully queued.
    Ok,
    /// Queued but warning threshold reached.
    Warning,
    /// Queue is full, action needed per policy.
    Full(QueueFullPolicy),
}

/// Builder for deferred configuration.
pub struct DeferredConfigBuilder {
    config: DeferredConfig,
}

impl DeferredConfigBuilder {
    /// Create a new builder with defaults.
    pub fn new() -> Self {
        Self {
            config: DeferredConfig::default(),
        }
    }

    /// Set processing mode.
    pub fn mode(mut self, mode: DeferredProcessing) -> Self {
        self.config.mode = mode;
        self
    }

    /// Set queue capacity.
    pub fn capacity(mut self, capacity: usize) -> Self {
        self.config.capacity = capacity;
        self
    }

    /// Set full policy.
    pub fn full_policy(mut self, policy: QueueFullPolicy) -> Self {
        self.config.full_policy = policy;
        self
    }

    /// Set warning threshold.
    pub fn warning_threshold(mut self, threshold: usize) -> Self {
        self.config.warning_threshold = threshold;
        self
    }

    /// Build the configuration.
    pub fn build(self) -> DeferredConfig {
        self.config
    }
}

impl Default for DeferredConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = DeferredConfig::default();
        assert_eq!(config.mode, DeferredProcessing::AtFrameBegin);
        assert_eq!(config.capacity, 0);
    }

    #[test]
    fn test_bounded_config() {
        let config = DeferredConfig::bounded(100);
        assert_eq!(config.capacity, 100);
        assert_eq!(config.full_policy, QueueFullPolicy::ProcessImmediately);
    }

    #[test]
    fn test_controller_queuing() {
        let controller = DeferredController::new(DeferredConfig::bounded(10));
        
        // Queue up to capacity
        for i in 0..10 {
            let result = controller.record_queued(100);
            if i < 8 {
                assert_eq!(result, QueueResult::Ok);
            }
        }
        
        // Should be full
        let result = controller.record_queued(100);
        assert!(matches!(result, QueueResult::Full(_)));
    }

    #[test]
    fn test_controller_stats() {
        let controller = DeferredController::new(DeferredConfig::default());
        
        controller.record_queued(100);
        controller.record_queued(200);
        
        let stats = controller.stats();
        assert_eq!(stats.total_queued, 2);
        assert_eq!(stats.current_depth, 2);
        assert_eq!(stats.queued_bytes, 300);
        
        controller.record_processed(1, 100);
        
        let stats = controller.stats();
        assert_eq!(stats.total_processed, 1);
        assert_eq!(stats.current_depth, 1);
    }
}
