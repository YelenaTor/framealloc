//! Vulkan allocator implementation

use super::super::traits::{GpuAllocator, GpuBuffer, GpuAllocError, BufferUsage, MemoryType, GpuAllocStats};
use super::buffer::VulkanBuffer;
use std::sync::Arc;

/// Vulkan-based GPU allocator
pub struct VulkanAllocator {
    /// Vulkan device
    device: Arc<ash::Device>,
    /// Physical device
    physical_device: ash::vk::PhysicalDevice,
    /// Memory properties
    memory_properties: ash::vk::PhysicalDeviceMemoryProperties,
    /// Statistics
    stats: GpuAllocStats,
}

impl VulkanAllocator {
    /// Create a new Vulkan allocator
    pub fn new(device: Arc<ash::Device>, physical_device: ash::vk::PhysicalDevice, instance: &ash::Instance) -> Self {
        let memory_properties = unsafe { instance.get_physical_device_memory_properties(physical_device) };
        
        Self {
            device,
            physical_device,
            memory_properties,
            stats: GpuAllocStats::default(),
        }
    }
    
    /// Get the total memory available for the given memory type
    fn get_memory_heap_size(&self, memory_type: MemoryType) -> u64 {
        for (i, mem_type) in self.memory_properties.memory_types.iter().enumerate() {
            let properties = mem_type.property_flags;
            let matches = match memory_type {
                MemoryType::DeviceLocal => properties.contains(ash::vk::MemoryPropertyFlags::DEVICE_LOCAL),
                MemoryType::HostVisible => properties.contains(ash::vk::MemoryPropertyFlags::HOST_VISIBLE),
                MemoryType::HostCoherent => properties.contains(ash::vk::MemoryPropertyFlags::HOST_COHERENT),
                MemoryType::HostCached => properties.contains(ash::vk::MemoryPropertyFlags::HOST_CACHED),
                MemoryType::Lazy => properties.contains(ash::vk::MemoryPropertyFlags::LAZILY_ALLOCATED),
            };
            
            if matches {
                let heap_index = mem_type.heap_index;
                return self.memory_properties.memory_heaps[heap_index as usize].size;
            }
        }
        0
    }
}

impl GpuAllocator for VulkanAllocator {
    fn allocate_buffer(
        &mut self,
        size: usize,
        usage: BufferUsage,
        memory_type: MemoryType,
    ) -> Result<Box<dyn GpuBuffer>, GpuAllocError> {
        // Update stats
        self.stats.allocation_count += 1;
        
        // Create buffer
        let buffer = VulkanBuffer::new(
            self.device.clone(),
            size,
            usage,
            memory_type,
            &self.memory_properties,
        )?;
        
        // Update allocated bytes
        self.stats.allocated_bytes += size;
        
        // Update peak usage
        if self.stats.allocated_bytes > self.stats.peak_usage {
            self.stats.peak_usage = self.stats.allocated_bytes;
        }
        
        Ok(Box::new(buffer))
    }
    
    fn free_buffer(&mut self, _buffer: Box<dyn GpuBuffer>) {
        // In a real implementation, we would track and update stats
        // For now, the buffer is dropped automatically
    }
    
    fn total_allocated(&self) -> usize {
        self.stats.allocated_bytes
    }
    
    fn available_memory(&self) -> usize {
        // Return device local memory as available
        self.get_memory_heap_size(MemoryType::DeviceLocal) as usize
    }
    
    fn supports_mapping(&self) -> bool {
        true // Vulkan supports memory mapping for host-visible memory
    }
    
    fn alignment_for(&self, usage: BufferUsage) -> usize {
        // Return standard alignment requirements
        // In a real implementation, this would query the device
        if usage.bits & (BufferUsage::UNIFORM_BUFFER.bits | BufferUsage::STORAGE_BUFFER.bits) != 0 {
            256 // Min alignment for uniform/storage buffers
        } else {
            16 // Default alignment
        }
    }
}
