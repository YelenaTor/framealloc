//! Allocation intent tags.

/// Describes the intended lifetime and usage of an allocation.
///
/// This allows the allocator to route allocations to the optimal backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AllocationIntent {
    /// Frame-temporary allocation.
    ///
    /// Lives only until `end_frame()` is called.
    /// Uses bump allocation - extremely fast.
    Frame,

    /// Short-lived allocation from object pool.
    ///
    /// Should be explicitly freed when done.
    /// Uses thread-local free lists.
    Pool,

    /// Long-lived allocation.
    ///
    /// Uses the system heap.
    /// Should be explicitly freed when done.
    Heap,
}

impl Default for AllocationIntent {
    fn default() -> Self {
        Self::Frame
    }
}

/// A tag for categorizing allocations for budgeting and tracking.
///
/// Custom tags can be used to track memory usage by subsystem.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AllocationTag {
    name: &'static str,
}

impl AllocationTag {
    /// Create a new allocation tag.
    pub const fn new(name: &'static str) -> Self {
        Self { name }
    }

    /// Get the tag name.
    pub fn name(&self) -> &'static str {
        self.name
    }
}

// Common predefined tags
impl AllocationTag {
    pub const RENDERING: Self = Self::new("rendering");
    pub const PHYSICS: Self = Self::new("physics");
    pub const AUDIO: Self = Self::new("audio");
    pub const SCRIPTING: Self = Self::new("scripting");
    pub const ASSETS: Self = Self::new("assets");
    pub const UI: Self = Self::new("ui");
    pub const NETWORKING: Self = Self::new("networking");
    pub const GENERAL: Self = Self::new("general");
}
