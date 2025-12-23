//! Vulkan backend for GPU allocation
//! 
//! This module provides a Vulkan implementation of the GpuAllocator trait using the ash crate.

pub mod allocator;
pub mod buffer;

pub use allocator::VulkanAllocator;
pub use buffer::VulkanBuffer;
