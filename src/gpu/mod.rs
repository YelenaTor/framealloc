//! GPU memory management module
//! 
//! This module is **only available** when the `gpu` feature flag is enabled.
//! It provides abstractions for GPU-visible memory allocation.
//! 
//! ## Backends
//! - `vulkan`: Via the `ash` crate (enable `gpu-vulkan` feature)
//! - `wgpu`: Planned (enable `gpu-wgpu` feature)

// Always present for API stability: traits define the interface
pub mod traits;
pub use traits::{
    GpuAllocator, GpuBuffer, GpuAllocError, BufferUsage, MemoryType,
    GpuMemoryIntent, GpuLifetime, GpuAllocRequirements, GpuAllocStats,
    GpuFrameAllocator, GpuMappable,
};

// Dummy allocator for testing (always available)
pub mod dummy;
pub use dummy::{DummyAllocator, DummyBuffer};

// Backend implementations are conditionally compiled
#[cfg(feature = "gpu-vulkan")]
pub mod vulkan;

#[cfg(feature = "gpu-wgpu")]
pub mod wgpu;
