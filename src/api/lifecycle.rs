//! Frame lifecycle events for v0.6.0 observability.
//!
//! Provides opt-in event callbacks for frame lifecycle monitoring
//! with zero overhead when disabled.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use std::thread::ThreadId;
use std::time::Instant;

/// A frame lifecycle event.
#[derive(Debug, Clone)]
pub enum FrameEvent {
    /// A frame has begun on a thread.
    FrameBegin {
        thread_id: ThreadId,
        frame_number: u64,
        timestamp: Instant,
    },
    /// An allocation occurred within a frame.
    Alloc {
        thread_id: ThreadId,
        size: usize,
        tag: Option<&'static str>,
        frame_number: u64,
    },
    /// A deallocation occurred.
    Free {
        thread_id: ThreadId,
        size: usize,
        was_cross_thread: bool,
    },
    /// A frame has ended.
    FrameEnd {
        thread_id: ThreadId,
        frame_number: u64,
        duration_us: u64,
        total_allocated: usize,
        peak_memory: usize,
    },
    /// A cross-thread free was queued.
    CrossThreadFreeQueued {
        from_thread: ThreadId,
        to_thread: ThreadId,
        size: usize,
    },
    /// Deferred frees were processed.
    DeferredProcessed {
        thread_id: ThreadId,
        count: usize,
        total_bytes: usize,
    },
    /// A transfer handle was created.
    TransferInitiated {
        from_thread: ThreadId,
        size: usize,
    },
    /// A transfer was completed.
    TransferCompleted {
        to_thread: ThreadId,
        size: usize,
    },
    /// Memory pressure detected.
    MemoryPressure {
        thread_id: ThreadId,
        used: usize,
        budget: usize,
    },
    /// Budget exceeded.
    BudgetExceeded {
        thread_id: ThreadId,
        requested: usize,
        available: usize,
        budget: usize,
    },
}

/// Callback type for frame events.
pub type FrameEventCallback = Box<dyn Fn(&FrameEvent) + Send + Sync>;

/// Manager for frame lifecycle events.
pub struct LifecycleManager {
    /// Whether lifecycle tracking is enabled.
    enabled: AtomicBool,
    /// Current frame number (global).
    frame_number: AtomicU64,
    /// Registered event callbacks.
    callbacks: Mutex<Vec<FrameEventCallback>>,
    /// Per-thread statistics.
    thread_stats: Mutex<std::collections::HashMap<ThreadId, ThreadFrameStats>>,
}

impl LifecycleManager {
    /// Create a new lifecycle manager.
    pub fn new() -> Self {
        Self {
            enabled: AtomicBool::new(false),
            frame_number: AtomicU64::new(0),
            callbacks: Mutex::new(Vec::new()),
            thread_stats: Mutex::new(std::collections::HashMap::new()),
        }
    }

    /// Enable lifecycle tracking.
    pub fn enable(&self) {
        self.enabled.store(true, Ordering::SeqCst);
    }

    /// Disable lifecycle tracking.
    pub fn disable(&self) {
        self.enabled.store(false, Ordering::SeqCst);
    }

    /// Check if lifecycle tracking is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    /// Register an event callback.
    pub fn on_event<F>(&self, callback: F)
    where
        F: Fn(&FrameEvent) + Send + Sync + 'static,
    {
        let mut callbacks = self.callbacks.lock().unwrap();
        callbacks.push(Box::new(callback));
    }

    /// Clear all callbacks.
    pub fn clear_callbacks(&self) {
        let mut callbacks = self.callbacks.lock().unwrap();
        callbacks.clear();
    }

    /// Emit an event to all registered callbacks.
    pub fn emit(&self, event: FrameEvent) {
        if !self.is_enabled() {
            return;
        }

        let callbacks = self.callbacks.lock().unwrap();
        for callback in callbacks.iter() {
            callback(&event);
        }

        // Update internal stats
        self.update_stats(&event);
    }

    /// Update internal statistics based on event.
    fn update_stats(&self, event: &FrameEvent) {
        let mut stats = self.thread_stats.lock().unwrap();
        
        match event {
            FrameEvent::FrameBegin { thread_id, frame_number, .. } => {
                let entry = stats.entry(*thread_id).or_insert_with(ThreadFrameStats::new);
                entry.frames_started += 1;
                entry.current_frame = *frame_number;
            }
            FrameEvent::FrameEnd { thread_id, total_allocated, peak_memory, .. } => {
                let entry = stats.entry(*thread_id).or_insert_with(ThreadFrameStats::new);
                entry.frames_completed += 1;
                entry.total_allocated += *total_allocated as u64;
                if *peak_memory > entry.peak_memory as usize {
                    entry.peak_memory = *peak_memory as u64;
                }
            }
            FrameEvent::CrossThreadFreeQueued { from_thread, .. } => {
                let entry = stats.entry(*from_thread).or_insert_with(ThreadFrameStats::new);
                entry.cross_thread_frees += 1;
            }
            _ => {}
        }
    }

    /// Get current frame number.
    pub fn frame_number(&self) -> u64 {
        self.frame_number.load(Ordering::SeqCst)
    }

    /// Increment frame number.
    pub fn increment_frame(&self) -> u64 {
        self.frame_number.fetch_add(1, Ordering::SeqCst)
    }

    /// Get statistics for a specific thread.
    pub fn thread_stats(&self, thread_id: ThreadId) -> Option<ThreadFrameStats> {
        let stats = self.thread_stats.lock().unwrap();
        stats.get(&thread_id).cloned()
    }

    /// Get statistics for all threads.
    pub fn all_thread_stats(&self) -> std::collections::HashMap<ThreadId, ThreadFrameStats> {
        self.thread_stats.lock().unwrap().clone()
    }

    /// Reset all statistics.
    pub fn reset_stats(&self) {
        let mut stats = self.thread_stats.lock().unwrap();
        stats.clear();
    }

    /// Generate a summary report.
    pub fn summary(&self) -> LifecycleSummary {
        let stats = self.thread_stats.lock().unwrap();
        
        let mut total_frames = 0u64;
        let mut total_allocated = 0u64;
        let mut peak_memory = 0u64;
        let mut cross_thread_frees = 0u64;
        
        for thread_stats in stats.values() {
            total_frames += thread_stats.frames_completed;
            total_allocated += thread_stats.total_allocated;
            if thread_stats.peak_memory > peak_memory {
                peak_memory = thread_stats.peak_memory;
            }
            cross_thread_frees += thread_stats.cross_thread_frees;
        }
        
        LifecycleSummary {
            thread_count: stats.len(),
            total_frames,
            total_allocated,
            peak_memory,
            cross_thread_frees,
        }
    }
}

impl Default for LifecycleManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Per-thread frame statistics.
#[derive(Debug, Clone, Default)]
pub struct ThreadFrameStats {
    /// Number of frames started.
    pub frames_started: u64,
    /// Number of frames completed.
    pub frames_completed: u64,
    /// Current frame number.
    pub current_frame: u64,
    /// Total bytes allocated across all frames.
    pub total_allocated: u64,
    /// Peak memory usage.
    pub peak_memory: u64,
    /// Number of cross-thread frees initiated.
    pub cross_thread_frees: u64,
}

impl ThreadFrameStats {
    /// Create new empty stats.
    pub fn new() -> Self {
        Self::default()
    }
}

/// Summary of lifecycle statistics.
#[derive(Debug, Clone)]
pub struct LifecycleSummary {
    /// Number of threads tracked.
    pub thread_count: usize,
    /// Total frames across all threads.
    pub total_frames: u64,
    /// Total bytes allocated.
    pub total_allocated: u64,
    /// Peak memory usage.
    pub peak_memory: u64,
    /// Total cross-thread frees.
    pub cross_thread_frees: u64,
}

impl std::fmt::Display for LifecycleSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Lifecycle Summary:")?;
        writeln!(f, "  Threads: {}", self.thread_count)?;
        writeln!(f, "  Total frames: {}", self.total_frames)?;
        writeln!(f, "  Total allocated: {} bytes", self.total_allocated)?;
        writeln!(f, "  Peak memory: {} bytes", self.peak_memory)?;
        writeln!(f, "  Cross-thread frees: {}", self.cross_thread_frees)?;
        Ok(())
    }
}

/// A guard that emits FrameEnd when dropped.
pub struct FrameLifecycleGuard<'a> {
    manager: &'a LifecycleManager,
    thread_id: ThreadId,
    frame_number: u64,
    start_time: Instant,
    allocated: usize,
    peak: usize,
}

impl<'a> FrameLifecycleGuard<'a> {
    /// Create a new frame lifecycle guard.
    pub fn new(manager: &'a LifecycleManager) -> Self {
        let thread_id = std::thread::current().id();
        let frame_number = manager.frame_number();
        
        manager.emit(FrameEvent::FrameBegin {
            thread_id,
            frame_number,
            timestamp: Instant::now(),
        });
        
        Self {
            manager,
            thread_id,
            frame_number,
            start_time: Instant::now(),
            allocated: 0,
            peak: 0,
        }
    }

    /// Record an allocation.
    pub fn record_alloc(&mut self, size: usize) {
        self.allocated += size;
        if self.allocated > self.peak {
            self.peak = self.allocated;
        }
    }

    /// Record a deallocation.
    pub fn record_free(&mut self, size: usize) {
        self.allocated = self.allocated.saturating_sub(size);
    }
}

impl<'a> Drop for FrameLifecycleGuard<'a> {
    fn drop(&mut self) {
        let duration = self.start_time.elapsed();
        
        self.manager.emit(FrameEvent::FrameEnd {
            thread_id: self.thread_id,
            frame_number: self.frame_number,
            duration_us: duration.as_micros() as u64,
            total_allocated: self.allocated,
            peak_memory: self.peak,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::AtomicUsize;

    #[test]
    fn test_lifecycle_disabled_by_default() {
        let manager = LifecycleManager::new();
        assert!(!manager.is_enabled());
    }

    #[test]
    fn test_lifecycle_enable_disable() {
        let manager = LifecycleManager::new();
        manager.enable();
        assert!(manager.is_enabled());
        manager.disable();
        assert!(!manager.is_enabled());
    }

    #[test]
    fn test_lifecycle_callback() {
        let manager = LifecycleManager::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&counter);
        
        manager.enable();
        manager.on_event(move |_| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        });
        
        manager.emit(FrameEvent::FrameBegin {
            thread_id: std::thread::current().id(),
            frame_number: 0,
            timestamp: Instant::now(),
        });
        
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }
}
