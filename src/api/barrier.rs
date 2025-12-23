//! Frame barriers for deterministic multi-thread synchronization.
//!
//! Provides `FrameBarrier` for coordinating frame boundaries across
//! multiple threads in a deterministic, explicit manner.

use std::sync::atomic::{AtomicUsize, AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::ThreadId;
use std::collections::HashSet;

/// A barrier for synchronizing frame boundaries across threads.
///
/// Game engines often need deterministic frame sync - all threads must
/// complete their frame work before any thread can reset frame memory.
///
/// # Example
///
/// ```ignore
/// // Create barrier for main + 2 workers
/// let barrier = FrameBarrier::new(3);
///
/// // Each thread signals completion
/// barrier.signal_frame_complete();
///
/// // Coordinator waits for all
/// barrier.wait_all();
/// alloc.end_frame();
/// ```
pub struct FrameBarrier {
    /// Number of threads in the barrier.
    thread_count: usize,
    /// Number of threads that have signaled.
    arrived: AtomicUsize,
    /// Current generation (incremented each time barrier resets).
    generation: AtomicUsize,
    /// Whether all threads have arrived.
    all_arrived: AtomicBool,
    /// Mutex for wait coordination.
    lock: Mutex<()>,
    /// Condition variable for waiting.
    cvar: Condvar,
    /// Registered thread IDs (for debugging).
    registered_threads: Mutex<HashSet<ThreadId>>,
}

impl FrameBarrier {
    /// Create a new frame barrier for the given number of threads.
    pub fn new(thread_count: usize) -> Arc<Self> {
        Arc::new(Self {
            thread_count,
            arrived: AtomicUsize::new(0),
            generation: AtomicUsize::new(0),
            all_arrived: AtomicBool::new(false),
            lock: Mutex::new(()),
            cvar: Condvar::new(),
            registered_threads: Mutex::new(HashSet::new()),
        })
    }

    /// Register the current thread with this barrier.
    ///
    /// Optional but recommended for debugging - allows detection of
    /// threads signaling without being registered.
    pub fn register_thread(&self) {
        let mut threads = self.registered_threads.lock().unwrap();
        threads.insert(std::thread::current().id());
    }

    /// Unregister the current thread from this barrier.
    pub fn unregister_thread(&self) {
        let mut threads = self.registered_threads.lock().unwrap();
        threads.remove(&std::thread::current().id());
    }

    /// Check if a thread is registered.
    pub fn is_registered(&self, thread_id: ThreadId) -> bool {
        let threads = self.registered_threads.lock().unwrap();
        threads.contains(&thread_id)
    }

    /// Get the number of threads in this barrier.
    pub fn thread_count(&self) -> usize {
        self.thread_count
    }

    /// Get the number of threads that have arrived.
    pub fn arrived_count(&self) -> usize {
        self.arrived.load(Ordering::SeqCst)
    }

    /// Get the current generation.
    pub fn generation(&self) -> usize {
        self.generation.load(Ordering::SeqCst)
    }

    /// Signal that the current thread has completed its frame work.
    ///
    /// This is non-blocking - the thread can continue with other work
    /// or immediately call `wait_all()`.
    pub fn signal_frame_complete(&self) {
        let prev = self.arrived.fetch_add(1, Ordering::SeqCst);
        
        // If we're the last thread, signal completion
        if prev + 1 == self.thread_count {
            self.all_arrived.store(true, Ordering::SeqCst);
            self.cvar.notify_all();
        }
    }

    /// Wait for all threads to complete their frame work.
    ///
    /// Blocks until all threads have called `signal_frame_complete()`.
    pub fn wait_all(&self) {
        let mut guard = self.lock.lock().unwrap();
        let current_gen = self.generation.load(Ordering::SeqCst);
        
        while !self.all_arrived.load(Ordering::SeqCst) 
            && self.generation.load(Ordering::SeqCst) == current_gen 
        {
            guard = self.cvar.wait(guard).unwrap();
        }
    }

    /// Wait with a timeout.
    ///
    /// Returns `true` if all threads arrived, `false` if timeout expired.
    pub fn wait_timeout(&self, timeout: std::time::Duration) -> bool {
        let mut guard = self.lock.lock().unwrap();
        let current_gen = self.generation.load(Ordering::SeqCst);
        let deadline = std::time::Instant::now() + timeout;
        
        while !self.all_arrived.load(Ordering::SeqCst)
            && self.generation.load(Ordering::SeqCst) == current_gen
        {
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            if remaining.is_zero() {
                return false;
            }
            let result = self.cvar.wait_timeout(guard, remaining).unwrap();
            guard = result.0;
        }
        
        true
    }

    /// Reset the barrier for the next frame.
    ///
    /// Should be called by the coordinator after `wait_all()` returns
    /// and frame cleanup is complete.
    pub fn reset(&self) {
        self.arrived.store(0, Ordering::SeqCst);
        self.all_arrived.store(false, Ordering::SeqCst);
        self.generation.fetch_add(1, Ordering::SeqCst);
    }

    /// Combined wait and reset - convenience for coordinator thread.
    pub fn wait_and_reset(&self) {
        self.wait_all();
        self.reset();
    }

    /// Check if all threads have arrived (non-blocking).
    pub fn is_complete(&self) -> bool {
        self.all_arrived.load(Ordering::SeqCst)
    }
}

/// Builder for creating frame barriers with specific thread configurations.
pub struct FrameBarrierBuilder {
    thread_count: usize,
    thread_names: Vec<String>,
}

impl FrameBarrierBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            thread_count: 0,
            thread_names: Vec::new(),
        }
    }

    /// Add a thread to the barrier.
    pub fn with_thread(mut self, name: &str) -> Self {
        self.thread_count += 1;
        self.thread_names.push(name.to_string());
        self
    }

    /// Set the total thread count.
    pub fn with_count(mut self, count: usize) -> Self {
        self.thread_count = count;
        self
    }

    /// Build the frame barrier.
    pub fn build(self) -> Arc<FrameBarrier> {
        FrameBarrier::new(self.thread_count)
    }
}

impl Default for FrameBarrierBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about barrier usage.
#[derive(Debug, Default, Clone)]
pub struct BarrierStats {
    /// Total wait operations.
    pub total_waits: u64,
    /// Total time spent waiting (microseconds).
    pub total_wait_time_us: u64,
    /// Maximum wait time (microseconds).
    pub max_wait_time_us: u64,
    /// Number of timeout occurrences.
    pub timeout_count: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_barrier_single_thread() {
        let barrier = FrameBarrier::new(1);
        barrier.signal_frame_complete();
        assert!(barrier.is_complete());
        barrier.wait_all();
        barrier.reset();
        assert!(!barrier.is_complete());
    }

    #[test]
    fn test_barrier_multi_thread() {
        let barrier = FrameBarrier::new(3);
        let b1 = Arc::clone(&barrier);
        let b2 = Arc::clone(&barrier);
        
        let h1 = thread::spawn(move || {
            b1.signal_frame_complete();
        });
        
        let h2 = thread::spawn(move || {
            b2.signal_frame_complete();
        });
        
        barrier.signal_frame_complete();
        barrier.wait_all();
        
        h1.join().unwrap();
        h2.join().unwrap();
        
        assert!(barrier.is_complete());
    }

    #[test]
    fn test_barrier_builder() {
        let barrier = FrameBarrierBuilder::new()
            .with_thread("main")
            .with_thread("worker1")
            .with_thread("worker2")
            .build();
        
        assert_eq!(barrier.thread_count(), 3);
    }
}
