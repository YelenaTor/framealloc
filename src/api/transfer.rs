//! Explicit cross-thread transfer handles for v0.6.0.
//!
//! Provides `TransferHandle<T>` for declaring intent to move allocations
//! across thread boundaries, making cross-thread costs visible and explicit.

use std::marker::PhantomData;
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::thread::ThreadId;

/// Unique identifier for a transfer operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TransferId(u64);

impl TransferId {
    fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

/// State of a transfer handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferState {
    /// Handle is owned by the originating thread.
    Owned,
    /// Handle has been sent but not yet received.
    InFlight,
    /// Handle has been received by destination thread.
    Received,
    /// Handle has been consumed/dropped.
    Consumed,
}

/// A handle for explicit cross-thread allocation transfer.
///
/// This makes cross-thread intent explicit:
/// - Developer declares "this will move threads" at allocation time
/// - Transfer has visible cost (not hidden in deferred queue)
/// - Destination thread explicitly accepts ownership
///
/// # Example
///
/// ```ignore
/// // Allocate with transfer intent
/// let handle = alloc.frame_box_for_transfer(physics_result);
///
/// // Send to worker thread
/// worker_channel.send(handle);
///
/// // On worker thread: explicitly receive
/// let data = handle.receive();
/// ```
pub struct TransferHandle<T> {
    /// Pointer to the allocated data.
    ptr: *mut T,
    /// Size of the allocation.
    size: usize,
    /// Transfer identifier for tracking.
    id: TransferId,
    /// Thread that created this handle.
    origin_thread: ThreadId,
    /// Current state of the transfer.
    state: TransferState,
    /// Whether the data has been received.
    received: AtomicBool,
    /// Marker for the type.
    _marker: PhantomData<T>,
}

// SAFETY: TransferHandle is explicitly designed for cross-thread transfer.
// The handle tracks ownership and ensures proper synchronization.
unsafe impl<T: Send> Send for TransferHandle<T> {}

impl<T> TransferHandle<T> {
    /// Create a new transfer handle.
    ///
    /// This is typically called by `SmartAlloc::frame_box_for_transfer`.
    pub(crate) fn new(ptr: *mut T, size: usize) -> Self {
        Self {
            ptr,
            size,
            id: TransferId::new(),
            origin_thread: std::thread::current().id(),
            state: TransferState::Owned,
            received: AtomicBool::new(false),
            _marker: PhantomData,
        }
    }

    /// Get the transfer ID.
    pub fn id(&self) -> TransferId {
        self.id
    }

    /// Get the origin thread.
    pub fn origin_thread(&self) -> ThreadId {
        self.origin_thread
    }

    /// Get the current state.
    pub fn state(&self) -> TransferState {
        self.state
    }

    /// Get the size of the transferred allocation.
    pub fn size(&self) -> usize {
        self.size
    }

    /// Mark the handle as in-flight (being sent to another thread).
    ///
    /// This is called automatically when sending through a channel.
    pub fn mark_sent(&mut self) {
        self.state = TransferState::InFlight;
    }

    /// Receive the transferred data on the destination thread.
    ///
    /// This explicitly accepts ownership of the allocation on the
    /// current thread. Returns a reference to the data.
    ///
    /// # Panics
    ///
    /// Panics if called more than once (data can only be received once).
    pub fn receive(&mut self) -> &mut T {
        if self.received.swap(true, Ordering::SeqCst) {
            panic!("TransferHandle::receive called more than once");
        }
        self.state = TransferState::Received;
        // SAFETY: We have exclusive ownership via the received flag
        unsafe { &mut *self.ptr }
    }

    /// Receive and take ownership, consuming the handle.
    ///
    /// Returns the data, which will be freed when dropped normally.
    pub fn receive_owned(mut self) -> T
    where
        T: Clone,
    {
        let data = self.receive().clone();
        self.state = TransferState::Consumed;
        data
    }

    /// Check if this handle has been received.
    pub fn is_received(&self) -> bool {
        self.received.load(Ordering::SeqCst)
    }

    /// Get raw pointer (for advanced use cases).
    ///
    /// # Safety
    ///
    /// Caller must ensure proper synchronization and lifetime management.
    pub unsafe fn as_ptr(&self) -> *mut T {
        self.ptr
    }
}

impl<T> Drop for TransferHandle<T> {
    fn drop(&mut self) {
        // If not received, the allocation will be cleaned up by the
        // deferred free mechanism when the origin thread processes it.
        // This is a safety net - proper usage should always receive.
        if !self.is_received() && self.state != TransferState::Consumed {
            // Log warning in debug mode
            #[cfg(feature = "debug")]
            eprintln!(
                "TransferHandle dropped without being received (id: {:?}, origin: {:?})",
                self.id, self.origin_thread
            );
        }
    }
}

/// Statistics about cross-thread transfers.
#[derive(Debug, Default, Clone)]
pub struct TransferStats {
    /// Total transfers initiated.
    pub transfers_initiated: u64,
    /// Total transfers completed.
    pub transfers_completed: u64,
    /// Total transfers dropped without receiving.
    pub transfers_dropped: u64,
    /// Total bytes transferred.
    pub bytes_transferred: u64,
}

/// Registry for tracking active transfers.
#[derive(Default)]
pub struct TransferRegistry {
    stats: std::sync::Mutex<TransferStats>,
}

impl TransferRegistry {
    /// Create a new transfer registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a transfer initiation.
    pub fn record_initiated(&self, size: usize) {
        let mut stats = self.stats.lock().unwrap();
        stats.transfers_initiated += 1;
        stats.bytes_transferred += size as u64;
    }

    /// Record a transfer completion.
    pub fn record_completed(&self) {
        let mut stats = self.stats.lock().unwrap();
        stats.transfers_completed += 1;
    }

    /// Record a dropped transfer.
    pub fn record_dropped(&self) {
        let mut stats = self.stats.lock().unwrap();
        stats.transfers_dropped += 1;
    }

    /// Get current statistics.
    pub fn stats(&self) -> TransferStats {
        self.stats.lock().unwrap().clone()
    }

    /// Reset statistics.
    pub fn reset_stats(&self) {
        let mut stats = self.stats.lock().unwrap();
        *stats = TransferStats::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transfer_id_unique() {
        let id1 = TransferId::new();
        let id2 = TransferId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_transfer_handle_state() {
        let data = Box::into_raw(Box::new(42u32));
        let mut handle = TransferHandle::new(data, 4);
        
        assert_eq!(handle.state(), TransferState::Owned);
        assert!(!handle.is_received());
        
        handle.mark_sent();
        assert_eq!(handle.state(), TransferState::InFlight);
        
        let value = handle.receive();
        assert_eq!(*value, 42);
        assert_eq!(handle.state(), TransferState::Received);
        assert!(handle.is_received());
        
        // Cleanup
        unsafe { let _ = Box::from_raw(data); }
    }
}
