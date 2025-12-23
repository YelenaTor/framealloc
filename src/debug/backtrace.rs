//! Allocation backtrace tracking.
//!
//! Records stack traces for allocations to help debug memory issues.

use std::collections::HashMap;

use crate::sync::mutex::Mutex;

/// A captured backtrace for an allocation.
#[derive(Clone)]
pub struct AllocationTrace {
    /// The allocation address
    pub address: usize,

    /// Size of the allocation
    pub size: usize,

    /// Captured backtrace (as string for simplicity)
    pub backtrace: String,

    /// Timestamp (frame number or similar)
    pub timestamp: u64,
}

/// Global tracker for allocation backtraces.
pub struct BacktraceTracker {
    traces: Mutex<HashMap<usize, AllocationTrace>>,
    frame_counter: std::sync::atomic::AtomicU64,
}

impl BacktraceTracker {
    /// Create a new backtrace tracker.
    pub fn new() -> Self {
        Self {
            traces: Mutex::new(HashMap::new()),
            frame_counter: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Record an allocation with its backtrace.
    pub fn record_alloc(&self, address: usize, size: usize) {
        #[cfg(feature = "debug")]
        {
            let bt = backtrace::Backtrace::new();
            let trace = AllocationTrace {
                address,
                size,
                backtrace: format!("{:?}", bt),
                timestamp: self.frame_counter.load(std::sync::atomic::Ordering::Relaxed),
            };
            
            let mut traces = self.traces.lock();
            traces.insert(address, trace);
        }

        #[cfg(not(feature = "debug"))]
        {
            let _ = (address, size);
        }
    }

    /// Remove an allocation record.
    pub fn record_free(&self, address: usize) {
        let mut traces = self.traces.lock();
        traces.remove(&address);
    }

    /// Get the trace for an address (if tracked).
    pub fn get_trace(&self, address: usize) -> Option<AllocationTrace> {
        let traces = self.traces.lock();
        traces.get(&address).cloned()
    }

    /// Increment the frame counter.
    pub fn next_frame(&self) {
        self.frame_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Get all active allocations.
    pub fn active_allocations(&self) -> Vec<AllocationTrace> {
        let traces = self.traces.lock();
        traces.values().cloned().collect()
    }

    /// Print a leak report.
    pub fn print_leak_report(&self) {
        let traces = self.traces.lock();
        if traces.is_empty() {
            println!("[framealloc] No active allocations (no leaks detected)");
            return;
        }

        println!("[framealloc] Leak report: {} active allocations", traces.len());
        for (addr, trace) in traces.iter() {
            println!("  Address: 0x{:x}, Size: {} bytes", addr, trace.size);
            println!("  Allocated at frame: {}", trace.timestamp);
            // Could print backtrace here if needed
        }
    }
}

impl Default for BacktraceTracker {
    fn default() -> Self {
        Self::new()
    }
}
