//! GPU allocator traits and types
//! 
//! This module defines the GPU allocator interface WITHOUT pulling in any backend-specific dependencies.
//! This allows the coordinator to depend on traits, not implementations.

use std::fmt;

/// Errors that can occur during GPU allocation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GpuAllocError {
    /// Out of memory
    OutOfMemory,
    /// Invalid buffer size
    InvalidSize,
    /// Unsupported buffer usage
    UnsupportedUsage,
    /// Alignment requirements not met
    AlignmentFailed,
    /// Backend-specific error (opaque)
    BackendError(String),
}

impl fmt::Display for GpuAllocError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GpuAllocError::OutOfMemory => write!(f, "GPU out of memory"),
            GpuAllocError::InvalidSize => write!(f, "Invalid buffer size"),
            GpuAllocError::UnsupportedUsage => write!(f, "Unsupported buffer usage flags"),
            GpuAllocError::AlignmentFailed => write!(f, "Alignment requirements not met"),
            GpuAllocError::BackendError(msg) => write!(f, "Backend error: {}", msg),
        }
    }
}

impl std::error::Error for GpuAllocError {}

/// GPU memory intent - expresses WHAT the memory will be used for
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GpuMemoryIntent {
    /// GPU-only access, fastest for shaders
    DeviceOnly,
    /// CPU can map and write, GPU can read
    HostVisible,
    /// CPU can map and read, optimized for readback
    HostCached,
    /// Temporary CPU→GPU transfer buffer
    Staging,
    /// Temporary GPU→CPU transfer buffer
    Readback,
}

/// GPU allocation lifetime - expresses HOW LONG the allocation should live
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GpuLifetime {
    /// Valid for one frame only
    Frame,
    /// Valid for N frames, reuse expected
    MultiFrame(u8),
    /// Long-lived (assets, meshes)
    Persistent,
}

/// GPU allocation requirements - bundles all allocation parameters
#[derive(Debug, Clone)]
pub struct GpuAllocRequirements {
    /// Size in bytes
    pub size: usize,
    /// Alignment requirement
    pub alignment: usize,
    /// Memory intent
    pub intent: GpuMemoryIntent,
    /// Expected lifetime
    pub lifetime: GpuLifetime,
}

impl GpuAllocRequirements {
    /// Create a new requirements struct
    pub fn new(size: usize, intent: GpuMemoryIntent, lifetime: GpuLifetime) -> Self {
        Self {
            size,
            alignment: 1, // Default alignment
            intent,
            lifetime,
        }
    }
    
    /// Set alignment requirement
    pub fn with_alignment(mut self, alignment: usize) -> Self {
        self.alignment = alignment;
        self
    }
}

/// Legacy buffer usage flags - still supported for compatibility
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BufferUsage {
    pub bits: u32,
}

impl BufferUsage {
    pub const TRANSFER_SRC: Self = Self { bits: 0x0001 };
    pub const TRANSFER_DST: Self = Self { bits: 0x0002 };
    pub const UNIFORM_TEXEL_BUFFER: Self = Self { bits: 0x0004 };
    pub const STORAGE_TEXEL_BUFFER: Self = Self { bits: 0x0008 };
    pub const UNIFORM_BUFFER: Self = Self { bits: 0x0010 };
    pub const STORAGE_BUFFER: Self = Self { bits: 0x0020 };
    pub const INDEX_BUFFER: Self = Self { bits: 0x0040 };
    pub const VERTEX_BUFFER: Self = Self { bits: 0x0080 };
    pub const INDIRECT_BUFFER: Self = Self { bits: 0x0100 };
}

impl std::ops::BitOr for BufferUsage {
    type Output = Self;
    
    fn bitor(self, rhs: Self) -> Self {
        Self { bits: self.bits | rhs.bits }
    }
}

/// Legacy memory type hints - use GpuMemoryIntent instead
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryType {
    /// Device-local memory (fast for GPU access)
    DeviceLocal,
    /// Host-visible memory (CPU can map)
    HostVisible,
    /// Host-coherent memory (automatically synchronized)
    HostCoherent,
    /// Host-cached memory (cached on CPU side)
    HostCached,
    /// Lazily allocated memory
    Lazy,
}

/// GPU allocation statistics for unified budgeting
#[derive(Debug, Clone, Default)]
pub struct GpuAllocStats {
    /// Total bytes allocated
    pub total_bytes: usize,
    /// Currently allocated bytes
    pub allocated_bytes: usize,
    /// Transient/frame-allocated bytes
    pub transient_bytes: usize,
    /// Persistent bytes
    pub persistent_bytes: usize,
    /// Peak usage for tracking
    pub peak_usage: usize,
    /// Number of allocations
    pub allocation_count: usize,
}

/// Core GPU allocation trait - all GPU allocators must implement this
/// This trait is object-safe for use with Box<dyn GpuAllocator>
pub trait GpuAllocator: Send + Sync {
    /// Allocate a buffer based on requirements
    fn allocate(&self, req: GpuAllocRequirements) -> Result<Box<dyn GpuBuffer>, GpuAllocError>;
    
    /// Deallocate a buffer
    fn deallocate(&self, buffer: Box<dyn GpuBuffer>);
    
    /// Get allocation statistics
    fn stats(&self) -> GpuAllocStats;
    
    /// Check if the allocator supports the given intent
    fn supports_intent(&self, intent: GpuMemoryIntent) -> bool;
}

/// Map mode for CPU access
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapMode {
    Read,
    Write,
    ReadWrite,
}

/// Optional mapping capability for GPU allocations
pub trait GpuMappable {
    /// Map the buffer for CPU access
    fn map(&mut self, mode: MapMode) -> Result<*mut u8, GpuAllocError>;
    
    /// Unmap the buffer
    fn unmap(&mut self);
    
    /// Flush mapped memory (for non-coherent memory)
    fn flush(&self, offset: usize, size: usize) -> Result<(), GpuAllocError>;
    
    /// Invalidate mapped memory (for non-coherent memory)
    fn invalidate(&self, offset: usize, size: usize) -> Result<(), GpuAllocError>;
}

/// Frame-aware GPU allocation trait
pub trait GpuFrameAllocator: GpuAllocator {
    /// Begin a new GPU frame
    fn begin_gpu_frame(&mut self);
    
    /// End the current GPU frame
    fn end_gpu_frame(&mut self);
    
    /// Allocate for the current frame only
    fn frame_alloc(&mut self, req: GpuAllocRequirements) -> Result<Self::Buffer, GpuAllocError> {
        let mut frame_req = req;
        frame_req.lifetime = GpuLifetime::Frame;
        self.allocate(frame_req)
    }
    
    /// Get current frame index
    fn current_frame(&self) -> u64;
}

/// Trait for GPU buffer objects
pub trait GpuBuffer: Send + Sync {
    /// Get the size of the buffer in bytes
    fn size(&self) -> usize;
    
    /// Get the memory intent this buffer was created with
    fn intent(&self) -> GpuMemoryIntent;
    
    /// Get the expected lifetime of this buffer
    fn lifetime(&self) -> GpuLifetime;
    
    /// Get a raw handle to the buffer
    /// The exact type depends on the backend (e.g., vk::Buffer for Vulkan)
    fn raw_handle(&self) -> *mut std::ffi::c_void;
    
    /// Map the buffer for CPU access (if supported)
    /// Returns None if the buffer cannot be mapped
    fn map(&mut self) -> Option<*mut u8>;
    
    /// Unmap the buffer
    fn unmap(&mut self);
    
    /// Flush mapped memory (if needed)
    fn flush(&self, offset: usize, size: usize) -> Result<(), GpuAllocError>;
    
    /// Invalidate mapped memory (if needed)
    fn invalidate(&self, offset: usize, size: usize) -> Result<(), GpuAllocError>;
}

/// Trait for GPU allocators
pub trait GpuAllocator: Send + Sync {
    /// Allocate a new GPU buffer
    fn allocate_buffer(
        &mut self,
        size: usize,
        usage: BufferUsage,
        memory_type: MemoryType,
    ) -> Result<Box<dyn GpuBuffer>, GpuAllocError>;
    
    /// Free a GPU buffer
    fn free_buffer(&mut self, buffer: Box<dyn GpuBuffer>);
    
    /// Get total allocated memory
    fn total_allocated(&self) -> usize;
    
    /// Get available memory
    fn available_memory(&self) -> usize;
    
    /// Check if the allocator supports mapping buffers
    fn supports_mapping(&self) -> bool;
    
    /// Get the alignment requirement for the given usage
    fn alignment_for(&self, usage: BufferUsage) -> usize;
}

/// Statistics for GPU allocator
#[derive(Debug, Clone, Default)]
pub struct GpuAllocStats {
    /// Total number of allocations
    pub allocation_count: usize,
    /// Total allocated bytes
    pub allocated_bytes: usize,
    /// Number of failed allocations
    pub failed_allocations: usize,
    /// Peak memory usage
    pub peak_usage: usize,
    /// Current fragmentation (0.0 to 1.0)
    pub fragmentation: f32,
}
