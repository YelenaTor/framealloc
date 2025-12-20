//! Slab allocator for small object pools.
//!
//! Uses size classes to efficiently allocate small objects.
//! Thread-local pools avoid contention; global registry for refills.

use std::alloc::{alloc, Layout};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::api::config::AllocConfig;
use crate::sync::mutex::Mutex;

/// Number of size classes
const NUM_SIZE_CLASSES: usize = 9;

/// Default size classes (bytes)
const DEFAULT_SIZE_CLASSES: [usize; NUM_SIZE_CLASSES] = [16, 32, 64, 128, 256, 512, 1024, 2048, 4096];

/// Global slab registry - manages pages for all size classes.
pub struct SlabRegistry {
    /// Size classes
    size_classes: [usize; NUM_SIZE_CLASSES],

    /// Page size
    page_size: usize,

    /// Per-class page pools
    classes: [SlabClass; NUM_SIZE_CLASSES],

    /// Total refill count
    refill_count: AtomicU64,
}

/// A single size class in the slab registry.
struct SlabClass {
    /// Free pages available for distribution
    free_pages: Mutex<Vec<SlabPage>>,

    /// Object size for this class
    object_size: usize,
}

/// A page of slab memory.
struct SlabPage {
    /// Base pointer
    base: *mut u8,

    /// Page size
    size: usize,

    /// Free list within the page
    free_list: Vec<*mut u8>,
}

impl SlabRegistry {
    /// Create a new slab registry.
    pub fn new(config: &AllocConfig) -> Self {
        let mut size_classes = DEFAULT_SIZE_CLASSES;

        // Override with config if provided
        for (i, &size) in config.slab_size_classes.iter().take(NUM_SIZE_CLASSES).enumerate() {
            size_classes[i] = size;
        }

        let classes = std::array::from_fn(|i| SlabClass {
            free_pages: Mutex::new(Vec::new()),
            object_size: size_classes[i],
        });

        Self {
            size_classes,
            page_size: config.slab_page_size,
            classes,
            refill_count: AtomicU64::new(0),
        }
    }

    /// Find the size class for a given size.
    fn size_class_index(&self, size: usize) -> Option<usize> {
        self.size_classes.iter().position(|&s| s >= size)
    }

    /// Refill a local pool from the global registry.
    ///
    /// Returns a batch of pointers for the local pool.
    pub fn refill(&self, size: usize) -> Vec<*mut u8> {
        let class_idx = match self.size_class_index(size) {
            Some(idx) => idx,
            None => return Vec::new(), // Size too large for slab
        };

        let class = &self.classes[class_idx];
        let mut pages = class.free_pages.lock();

        // Try to get an existing page
        if let Some(mut page) = pages.pop() {
            self.refill_count.fetch_add(1, Ordering::Relaxed);
            let batch = std::mem::take(&mut page.free_list);
            // Put page back if it might have more allocations
            return batch;
        }

        // No free pages, allocate a new one
        drop(pages); // Release lock before allocation

        let page = self.allocate_page(class.object_size);
        self.refill_count.fetch_add(1, Ordering::Relaxed);

        page.free_list
    }

    /// Allocate a new page for a size class.
    fn allocate_page(&self, object_size: usize) -> SlabPage {
        let layout = Layout::from_size_align(self.page_size, 16).expect("Invalid page layout");

        // SAFETY: Allocating with valid layout
        let base = unsafe { alloc(layout) };

        if base.is_null() {
            panic!("Failed to allocate slab page");
        }

        // Carve page into objects
        let objects_per_page = self.page_size / object_size;
        let mut free_list = Vec::with_capacity(objects_per_page);

        for i in 0..objects_per_page {
            // SAFETY: We're within the allocated page
            let ptr = unsafe { base.add(i * object_size) };
            free_list.push(ptr);
        }

        SlabPage {
            base,
            size: self.page_size,
            free_list,
        }
    }

    /// Return objects to the global registry.
    pub fn return_batch(&self, size: usize, batch: Vec<*mut u8>) {
        if batch.is_empty() {
            return;
        }

        let class_idx = match self.size_class_index(size) {
            Some(idx) => idx,
            None => return,
        };

        let class = &self.classes[class_idx];
        let mut pages = class.free_pages.lock();

        // Create a "virtual" page with the returned objects
        // In a real implementation, we'd track which page objects belong to
        pages.push(SlabPage {
            base: std::ptr::null_mut(), // Virtual page
            size: 0,
            free_list: batch,
        });
    }

    /// Get the refill count.
    pub fn refill_count(&self) -> u64 {
        self.refill_count.load(Ordering::Relaxed)
    }

    /// Get size classes.
    pub fn size_classes(&self) -> &[usize; NUM_SIZE_CLASSES] {
        &self.size_classes
    }
}

// SAFETY: SlabRegistry is thread-safe through internal locking
unsafe impl Send for SlabRegistry {}
unsafe impl Sync for SlabRegistry {}

/// Thread-local pools for fast allocation.
pub struct LocalPools {
    /// Per-size-class free lists
    pools: [LocalPool; NUM_SIZE_CLASSES],
}

/// A single local pool for one size class.
struct LocalPool {
    /// Free list of available objects
    free_list: Vec<*mut u8>,

    /// Size of objects in this pool
    object_size: usize,
}

impl LocalPools {
    /// Create new local pools.
    pub fn new() -> Self {
        Self {
            pools: std::array::from_fn(|i| LocalPool {
                free_list: Vec::with_capacity(64),
                object_size: DEFAULT_SIZE_CLASSES[i],
            }),
        }
    }

    /// Find pool index for a size.
    fn pool_index(&self, size: usize) -> Option<usize> {
        DEFAULT_SIZE_CLASSES.iter().position(|&s| s >= size)
    }

    /// Allocate from local pool.
    pub fn alloc(&mut self, size: usize, registry: &SlabRegistry) -> *mut u8 {
        let pool_idx = match self.pool_index(size) {
            Some(idx) => idx,
            None => return std::ptr::null_mut(), // Too large for slab
        };

        let pool = &mut self.pools[pool_idx];

        // Try local free list first
        if let Some(ptr) = pool.free_list.pop() {
            return ptr;
        }

        // Refill from global registry
        let batch = registry.refill(pool.object_size);
        if batch.is_empty() {
            return std::ptr::null_mut();
        }

        pool.free_list = batch;
        pool.free_list.pop().unwrap_or(std::ptr::null_mut())
    }

    /// Free to local pool.
    pub fn free(&mut self, ptr: *mut u8, size: usize) {
        let pool_idx = match self.pool_index(size) {
            Some(idx) => idx,
            None => return, // Was not from slab
        };

        // Poison memory before returning to pool in debug mode
        #[cfg(feature = "debug")]
        unsafe {
            crate::debug::poison::poison_freed(ptr, size);
        }

        let pool = &mut self.pools[pool_idx];
        pool.free_list.push(ptr);

        // Could return excess to global here if pool is too large
    }

    /// Drain deferred frees into local pools.
    pub fn drain_deferred(&mut self, ptr: *mut u8, size: usize) {
        self.free(ptr, size);
    }
}

impl Default for LocalPools {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slab_allocation() {
        let config = AllocConfig::default();
        let registry = SlabRegistry::new(&config);
        let mut pools = LocalPools::new();

        let ptr = pools.alloc(32, &registry);
        assert!(!ptr.is_null());

        pools.free(ptr, 32);

        // Should get same pointer back
        let ptr2 = pools.alloc(32, &registry);
        assert_eq!(ptr, ptr2);
    }
}
