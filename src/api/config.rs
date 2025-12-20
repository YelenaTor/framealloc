//! Allocator configuration.

use crate::util::size::{kb, mb};

/// Configuration for the smart allocator.
#[derive(Debug, Clone)]
pub struct AllocConfig {
    /// Size of the frame arena per thread (default: 16 MB)
    pub frame_arena_size: usize,

    /// Size classes for the slab allocator
    pub slab_size_classes: Vec<usize>,

    /// Number of pages to pre-allocate per size class
    pub slab_pages_per_class: usize,

    /// Page size for slab allocator (default: 64 KB)
    pub slab_page_size: usize,

    /// Enable memory budgeting
    pub enable_budgets: bool,

    /// Global memory limit (0 = unlimited)
    pub global_memory_limit: usize,

    /// Enable debug features (memory poisoning, etc.)
    pub debug_mode: bool,
}

impl Default for AllocConfig {
    fn default() -> Self {
        Self {
            frame_arena_size: mb(16),
            slab_size_classes: vec![16, 32, 64, 128, 256, 512, 1024, 2048, 4096],
            slab_pages_per_class: 4,
            slab_page_size: kb(64),
            enable_budgets: false,
            global_memory_limit: 0,
            debug_mode: cfg!(feature = "debug"),
        }
    }
}

impl AllocConfig {
    /// Create a minimal config for testing or constrained environments.
    pub fn minimal() -> Self {
        Self {
            frame_arena_size: mb(1),
            slab_size_classes: vec![32, 128, 512, 2048],
            slab_pages_per_class: 1,
            slab_page_size: kb(16),
            enable_budgets: false,
            global_memory_limit: 0,
            debug_mode: false,
        }
    }

    /// Create a config optimized for high-performance scenarios.
    pub fn high_performance() -> Self {
        Self {
            frame_arena_size: mb(64),
            slab_size_classes: vec![16, 32, 64, 128, 256, 512, 1024, 2048, 4096, 8192],
            slab_pages_per_class: 8,
            slab_page_size: kb(256),
            enable_budgets: false,
            global_memory_limit: 0,
            debug_mode: false,
        }
    }

    /// Builder pattern: set frame arena size.
    pub fn with_frame_arena_size(mut self, size: usize) -> Self {
        self.frame_arena_size = size;
        self
    }

    /// Builder pattern: set slab page size.
    pub fn with_slab_page_size(mut self, size: usize) -> Self {
        self.slab_page_size = size;
        self
    }

    /// Builder pattern: enable budgets.
    pub fn with_budgets(mut self, enable: bool) -> Self {
        self.enable_budgets = enable;
        self
    }

    /// Builder pattern: set global memory limit.
    pub fn with_memory_limit(mut self, limit: usize) -> Self {
        self.global_memory_limit = limit;
        self
    }

    /// Builder pattern: enable debug mode.
    pub fn with_debug(mut self, enable: bool) -> Self {
        self.debug_mode = enable;
        self
    }
}
