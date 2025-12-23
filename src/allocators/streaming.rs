//! Streaming allocator for large assets loaded over time.
//!
//! Designed for loading assets like textures, meshes, and audio that:
//! - Are loaded incrementally (streamed from disk/network)
//! - Have known final sizes
//! - May be evicted under memory pressure

use std::alloc::{alloc, dealloc, Layout};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use crate::sync::mutex::Mutex;

/// Unique identifier for a streaming allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StreamId(u64);

impl StreamId {
    /// Get the raw ID value.
    pub fn raw(&self) -> u64 {
        self.0
    }
}

/// Priority level for streaming allocations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum StreamPriority {
    /// Can be evicted immediately under pressure
    Low = 0,
    /// Normal priority
    Normal = 1,
    /// High priority, evict last
    High = 2,
    /// Critical, never evict automatically
    Critical = 3,
}

impl Default for StreamPriority {
    fn default() -> Self {
        Self::Normal
    }
}

/// State of a streaming allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamState {
    /// Memory reserved but not yet filled
    Reserved,
    /// Currently being filled with data
    Loading,
    /// Fully loaded and ready to use
    Ready,
    /// Marked for eviction
    Evicting,
}

/// Metadata for a streaming allocation.
#[derive(Debug)]
struct StreamAllocation {
    /// Unique ID
    id: StreamId,
    /// Base pointer
    ptr: *mut u8,
    /// Total reserved size
    reserved_size: usize,
    /// Currently loaded bytes
    loaded_bytes: usize,
    /// Current state
    state: StreamState,
    /// Priority for eviction
    priority: StreamPriority,
    /// Last access timestamp (frame number)
    last_access: u64,
    /// User-defined tag for categorization
    tag: Option<&'static str>,
}

/// Streaming allocator for large assets.
pub struct StreamingAllocator {
    /// Active allocations
    allocations: Mutex<HashMap<StreamId, StreamAllocation>>,
    
    /// Next allocation ID
    next_id: AtomicU64,
    
    /// Total reserved bytes
    total_reserved: AtomicUsize,
    
    /// Total loaded bytes
    total_loaded: AtomicUsize,
    
    /// Memory budget for streaming
    budget: usize,
    
    /// Current frame number for LRU tracking
    current_frame: AtomicU64,
    
    /// Eviction callback
    eviction_callback: Mutex<Option<Box<dyn Fn(StreamId) + Send + Sync>>>,
}

impl StreamingAllocator {
    /// Create a new streaming allocator with the given budget.
    pub fn new(budget: usize) -> Self {
        Self {
            allocations: Mutex::new(HashMap::new()),
            next_id: AtomicU64::new(1),
            total_reserved: AtomicUsize::new(0),
            total_loaded: AtomicUsize::new(0),
            budget,
            current_frame: AtomicU64::new(0),
            eviction_callback: Mutex::new(None),
        }
    }

    /// Set a callback for when allocations are evicted.
    pub fn set_eviction_callback<F>(&self, callback: F)
    where
        F: Fn(StreamId) + Send + Sync + 'static,
    {
        let mut cb = self.eviction_callback.lock();
        *cb = Some(Box::new(callback));
    }

    /// Reserve memory for a streaming asset.
    ///
    /// Returns None if the budget would be exceeded and eviction fails.
    pub fn reserve(&self, size: usize, priority: StreamPriority) -> Option<StreamId> {
        self.reserve_tagged(size, priority, None)
    }

    /// Reserve memory with a tag for categorization.
    pub fn reserve_tagged(
        &self,
        size: usize,
        priority: StreamPriority,
        tag: Option<&'static str>,
    ) -> Option<StreamId> {
        // Check if we need to evict
        let current_reserved = self.total_reserved.load(Ordering::Relaxed);
        if current_reserved + size > self.budget {
            let needed = (current_reserved + size) - self.budget;
            if !self.try_evict(needed, priority) {
                return None; // Cannot make room
            }
        }

        // Allocate the memory
        let layout = Layout::from_size_align(size, 16).ok()?;
        let ptr = unsafe { alloc(layout) };
        
        if ptr.is_null() {
            return None;
        }

        let id = StreamId(self.next_id.fetch_add(1, Ordering::Relaxed));
        let frame = self.current_frame.load(Ordering::Relaxed);

        let allocation = StreamAllocation {
            id,
            ptr,
            reserved_size: size,
            loaded_bytes: 0,
            state: StreamState::Reserved,
            priority,
            last_access: frame,
            tag,
        };

        let mut allocs = self.allocations.lock();
        allocs.insert(id, allocation);
        self.total_reserved.fetch_add(size, Ordering::Relaxed);

        Some(id)
    }

    /// Get a pointer for writing data into a streaming allocation.
    ///
    /// Returns None if the ID is invalid or the allocation is not in a writable state.
    pub fn begin_load(&self, id: StreamId) -> Option<*mut u8> {
        let mut allocs = self.allocations.lock();
        let alloc = allocs.get_mut(&id)?;

        match alloc.state {
            StreamState::Reserved | StreamState::Loading => {
                alloc.state = StreamState::Loading;
                Some(alloc.ptr)
            }
            _ => None,
        }
    }

    /// Report progress on loading.
    pub fn report_progress(&self, id: StreamId, bytes_loaded: usize) {
        let mut allocs = self.allocations.lock();
        if let Some(alloc) = allocs.get_mut(&id) {
            let old_loaded = alloc.loaded_bytes;
            alloc.loaded_bytes = bytes_loaded.min(alloc.reserved_size);
            
            let delta = alloc.loaded_bytes as isize - old_loaded as isize;
            if delta > 0 {
                self.total_loaded.fetch_add(delta as usize, Ordering::Relaxed);
            } else if delta < 0 {
                self.total_loaded.fetch_sub((-delta) as usize, Ordering::Relaxed);
            }
        }
    }

    /// Mark a streaming allocation as fully loaded.
    pub fn finish_load(&self, id: StreamId) {
        let mut allocs = self.allocations.lock();
        if let Some(alloc) = allocs.get_mut(&id) {
            alloc.state = StreamState::Ready;
            alloc.loaded_bytes = alloc.reserved_size;
            alloc.last_access = self.current_frame.load(Ordering::Relaxed);
        }
    }

    /// Access a ready allocation.
    ///
    /// Updates the LRU timestamp.
    pub fn access(&self, id: StreamId) -> Option<*const u8> {
        let mut allocs = self.allocations.lock();
        let alloc = allocs.get_mut(&id)?;

        if alloc.state == StreamState::Ready {
            alloc.last_access = self.current_frame.load(Ordering::Relaxed);
            Some(alloc.ptr as *const u8)
        } else {
            None
        }
    }

    /// Access a ready allocation mutably.
    pub fn access_mut(&self, id: StreamId) -> Option<*mut u8> {
        let mut allocs = self.allocations.lock();
        let alloc = allocs.get_mut(&id)?;

        if alloc.state == StreamState::Ready {
            alloc.last_access = self.current_frame.load(Ordering::Relaxed);
            Some(alloc.ptr)
        } else {
            None
        }
    }

    /// Free a streaming allocation.
    pub fn free(&self, id: StreamId) {
        let mut allocs = self.allocations.lock();
        if let Some(alloc) = allocs.remove(&id) {
            self.total_reserved.fetch_sub(alloc.reserved_size, Ordering::Relaxed);
            self.total_loaded.fetch_sub(alloc.loaded_bytes, Ordering::Relaxed);
            
            let layout = Layout::from_size_align(alloc.reserved_size, 16)
                .expect("Invalid layout");
            unsafe {
                dealloc(alloc.ptr, layout);
            }
        }
    }

    /// Try to evict allocations to free up the specified amount.
    ///
    /// Returns true if enough memory was freed.
    fn try_evict(&self, bytes_needed: usize, min_priority: StreamPriority) -> bool {
        let mut allocs = self.allocations.lock();
        
        // Collect candidates for eviction (lower priority than requested)
        let mut candidates: Vec<_> = allocs
            .values()
            .filter(|a| a.priority < min_priority && a.state == StreamState::Ready)
            .map(|a| (a.id, a.priority, a.last_access, a.reserved_size))
            .collect();

        // Sort by priority (ascending), then by last access (ascending = LRU first)
        candidates.sort_by(|a, b| {
            a.1.cmp(&b.1).then_with(|| a.2.cmp(&b.2))
        });

        let mut freed = 0;
        let mut to_evict = Vec::new();

        for (id, _, _, size) in candidates {
            if freed >= bytes_needed {
                break;
            }
            to_evict.push(id);
            freed += size;
        }

        // Actually evict
        for id in &to_evict {
            if let Some(alloc) = allocs.remove(id) {
                self.total_reserved.fetch_sub(alloc.reserved_size, Ordering::Relaxed);
                self.total_loaded.fetch_sub(alloc.loaded_bytes, Ordering::Relaxed);
                
                let layout = Layout::from_size_align(alloc.reserved_size, 16)
                    .expect("Invalid layout");
                unsafe {
                    dealloc(alloc.ptr, layout);
                }
            }
        }

        drop(allocs); // Release lock before callback

        // Notify about evictions
        if let Some(ref callback) = *self.eviction_callback.lock() {
            for id in to_evict {
                callback(id);
            }
        }

        freed >= bytes_needed
    }

    /// Advance to the next frame (for LRU tracking).
    pub fn next_frame(&self) {
        self.current_frame.fetch_add(1, Ordering::Relaxed);
    }

    /// Get the current memory budget.
    pub fn budget(&self) -> usize {
        self.budget
    }

    /// Get total reserved bytes.
    pub fn total_reserved(&self) -> usize {
        self.total_reserved.load(Ordering::Relaxed)
    }

    /// Get total loaded bytes.
    pub fn total_loaded(&self) -> usize {
        self.total_loaded.load(Ordering::Relaxed)
    }

    /// Get available budget.
    pub fn available(&self) -> usize {
        self.budget.saturating_sub(self.total_reserved.load(Ordering::Relaxed))
    }

    /// Get the state of an allocation.
    pub fn state(&self, id: StreamId) -> Option<StreamState> {
        let allocs = self.allocations.lock();
        allocs.get(&id).map(|a| a.state)
    }

    /// Get statistics about streaming allocations.
    pub fn stats(&self) -> StreamingStats {
        let allocs = self.allocations.lock();
        
        let mut stats = StreamingStats {
            budget: self.budget,
            total_reserved: self.total_reserved.load(Ordering::Relaxed),
            total_loaded: self.total_loaded.load(Ordering::Relaxed),
            allocation_count: allocs.len(),
            reserved_count: 0,
            loading_count: 0,
            ready_count: 0,
        };

        for alloc in allocs.values() {
            match alloc.state {
                StreamState::Reserved => stats.reserved_count += 1,
                StreamState::Loading => stats.loading_count += 1,
                StreamState::Ready => stats.ready_count += 1,
                StreamState::Evicting => {}
            }
        }

        stats
    }
}

// SAFETY: StreamingAllocator uses internal synchronization
unsafe impl Send for StreamingAllocator {}
unsafe impl Sync for StreamingAllocator {}

/// Statistics about streaming allocations.
#[derive(Debug, Clone, Default)]
pub struct StreamingStats {
    /// Total budget
    pub budget: usize,
    /// Total reserved bytes
    pub total_reserved: usize,
    /// Total loaded bytes
    pub total_loaded: usize,
    /// Number of active allocations
    pub allocation_count: usize,
    /// Allocations in Reserved state
    pub reserved_count: usize,
    /// Allocations in Loading state
    pub loading_count: usize,
    /// Allocations in Ready state
    pub ready_count: usize,
}

impl StreamingStats {
    /// Calculate budget utilization percentage.
    pub fn utilization_percent(&self) -> f64 {
        if self.budget == 0 {
            0.0
        } else {
            (self.total_reserved as f64 / self.budget as f64) * 100.0
        }
    }

    /// Calculate load progress percentage.
    pub fn load_progress_percent(&self) -> f64 {
        if self.total_reserved == 0 {
            100.0
        } else {
            (self.total_loaded as f64 / self.total_reserved as f64) * 100.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reserve_and_load() {
        let streaming = StreamingAllocator::new(1024 * 1024); // 1MB budget
        
        let id = streaming.reserve(1024, StreamPriority::Normal).unwrap();
        assert_eq!(streaming.state(id), Some(StreamState::Reserved));
        
        let ptr = streaming.begin_load(id).unwrap();
        assert!(!ptr.is_null());
        assert_eq!(streaming.state(id), Some(StreamState::Loading));
        
        streaming.report_progress(id, 512);
        streaming.finish_load(id);
        assert_eq!(streaming.state(id), Some(StreamState::Ready));
        
        let read_ptr = streaming.access(id).unwrap();
        assert!(!read_ptr.is_null());
        
        streaming.free(id);
        assert_eq!(streaming.state(id), None);
    }

    #[test]
    fn test_budget_enforcement() {
        let streaming = StreamingAllocator::new(1024); // 1KB budget
        
        // Should succeed
        let id1 = streaming.reserve(512, StreamPriority::Normal);
        assert!(id1.is_some());
        
        // Should succeed (just fits)
        let id2 = streaming.reserve(512, StreamPriority::Normal);
        assert!(id2.is_some());
        
        // Should fail (over budget, nothing to evict)
        let id3 = streaming.reserve(512, StreamPriority::Critical);
        assert!(id3.is_none());
    }

    #[test]
    fn test_eviction() {
        let streaming = StreamingAllocator::new(1024);
        
        // Fill with low priority
        let id1 = streaming.reserve(512, StreamPriority::Low).unwrap();
        streaming.finish_load(id1);
        
        let id2 = streaming.reserve(512, StreamPriority::Low).unwrap();
        streaming.finish_load(id2);
        
        // High priority should evict low priority
        let id3 = streaming.reserve(512, StreamPriority::High);
        assert!(id3.is_some());
        
        // One of the low priority allocations should be gone
        let remaining = [id1, id2].iter().filter(|id| streaming.state(**id).is_some()).count();
        assert_eq!(remaining, 1);
    }
}
