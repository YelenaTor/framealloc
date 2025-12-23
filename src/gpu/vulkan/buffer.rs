//! Vulkan buffer implementation

use super::super::traits::{GpuBuffer, GpuAllocError, BufferUsage, MemoryType};
use std::ptr;

/// Vulkan-backed GPU buffer
pub struct VulkanBuffer {
    /// Raw Vulkan buffer handle
    pub vk_buffer: ash::vk::Buffer,
    /// Raw Vulkan device memory handle
    pub vk_memory: ash::vk::DeviceMemory,
    /// Size in bytes
    size: usize,
    /// Memory type
    memory_type: MemoryType,
    /// Usage flags
    usage: BufferUsage,
    /// Whether the buffer is currently mapped
    mapped: bool,
    /// Mapped pointer (if any)
    mapped_ptr: Option<*mut u8>,
    /// Device reference
    device: std::sync::Arc<ash::Device>,
}

impl VulkanBuffer {
    /// Create a new Vulkan buffer
    pub fn new(
        device: std::sync::Arc<ash::Device>,
        size: usize,
        usage: BufferUsage,
        memory_type: MemoryType,
        memory_properties: &ash::vk::PhysicalDeviceMemoryProperties,
    ) -> Result<Self, GpuAllocError> {
        // Create VkBuffer
        let buffer_info = ash::vk::BufferCreateInfo::builder()
            .size(size as u64)
            .usage(ash::vk::BufferUsageFlags::from_raw(usage.bits))
            .sharing_mode(ash::vk::SharingMode::EXCLUSIVE);

        let vk_buffer = unsafe {
            device.create_buffer(&buffer_info, None)
                .map_err(|_| GpuAllocError::InvalidSize)?
        };

        // Get memory requirements
        let mem_requirements = unsafe { device.get_buffer_memory_requirements(vk_buffer) };

        // Allocate memory
        let memory_type_index = find_memory_type(
            memory_properties,
            mem_requirements.memory_type_bits,
            memory_type,
        )?;

        let alloc_info = ash::vk::MemoryAllocateInfo::builder()
            .allocation_size(mem_requirements.size)
            .memory_type_index(memory_type_index);

        let vk_memory = unsafe {
            device.allocate_memory(&alloc_info, None)
                .map_err(|_| GpuAllocError::OutOfMemory)?
        };

        // Bind memory
        unsafe {
            device.bind_buffer_memory(vk_buffer, vk_memory, 0)
                .map_err(|_| GpuAllocError::AlignmentFailed)?;
        }

        Ok(Self {
            vk_buffer,
            vk_memory,
            size,
            memory_type,
            usage,
            mapped: false,
            mapped_ptr: None,
            device,
        })
    }
}

impl GpuBuffer for VulkanBuffer {
    fn size(&self) -> usize {
        self.size
    }

    fn memory_type(&self) -> MemoryType {
        self.memory_type
    }

    fn usage(&self) -> BufferUsage {
        self.usage
    }

    fn raw_handle(&self) -> *mut std::ffi::c_void {
        self.vk_buffer.as_raw() as *mut std::ffi::c_void
    }

    fn map(&mut self) -> Option<*mut u8> {
        if self.mapped {
            return self.mapped_ptr;
        }

        if !matches!(self.memory_type, MemoryType::HostVisible | MemoryType::HostCoherent | MemoryType::HostCached) {
            return None;
        }

        unsafe {
            let ptr = self.device.map_memory(
                self.vk_memory,
                0,
                self.size as u64,
                ash::vk::MemoryMapFlags::empty(),
            ).ok()? as *mut u8;
            
            self.mapped = true;
            self.mapped_ptr = Some(ptr);
            Some(ptr)
        }
    }

    fn unmap(&mut self) {
        if !self.mapped {
            return;
        }

        unsafe {
            self.device.unmap_memory(self.vk_memory);
        }
        self.mapped = false;
        self.mapped_ptr = None;
    }

    fn flush(&self, offset: usize, size: usize) -> Result<(), GpuAllocError> {
        if matches!(self.memory_type, MemoryType::HostCoherent) {
            return Ok(());
        }

        let flush_range = ash::vk::MappedMemoryRange::builder()
            .memory(self.vk_memory)
            .offset(offset as u64)
            .size(size as u64);

        unsafe {
            self.device.flush_mapped_memory_ranges(&[flush_range])
                .map_err(|_| GpuAllocError::BackendError("Failed to flush memory".to_string()))?;
        }

        Ok(())
    }

    fn invalidate(&self, offset: usize, size: usize) -> Result<(), GpuAllocError> {
        if matches!(self.memory_type, MemoryType::HostCoherent) {
            return Ok(());
        }

        let invalidate_range = ash::vk::MappedMemoryRange::builder()
            .memory(self.vk_memory)
            .offset(offset as u64)
            .size(size as u64);

        unsafe {
            self.device.invalidate_mapped_memory_ranges(&[invalidate_range])
                .map_err(|_| GpuAllocError::BackendError("Failed to invalidate memory".to_string()))?;
        }

        Ok(())
    }
}

// Vulkan buffers are Send + Sync because ash handles are thread-safe when used properly
unsafe impl Send for VulkanBuffer {}
unsafe impl Sync for VulkanBuffer {}

impl Drop for VulkanBuffer {
    fn drop(&mut self) {
        unsafe {
            if self.mapped {
                self.device.unmap_memory(self.vk_memory);
            }
            self.device.free_memory(self.vk_memory, None);
            self.device.destroy_buffer(self.vk_buffer, None);
        }
    }
}

/// Find a suitable memory type index
fn find_memory_type(
    memory_properties: ash::vk::PhysicalDeviceMemoryProperties,
    type_filter: u32,
    memory_type: MemoryType,
) -> Result<u32, GpuAllocError> {
    for (i, mem_type) in memory_properties.memory_types.iter().enumerate() {
        let properties = mem_type.property_flags;
        let required = match memory_type {
            MemoryType::DeviceLocal => properties.contains(ash::vk::MemoryPropertyFlags::DEVICE_LOCAL),
            MemoryType::HostVisible => properties.contains(ash::vk::MemoryPropertyFlags::HOST_VISIBLE),
            MemoryType::HostCoherent => properties.contains(ash::vk::MemoryPropertyFlags::HOST_COHERENT),
            MemoryType::HostCached => properties.contains(ash::vk::MemoryPropertyFlags::HOST_CACHED),
            MemoryType::Lazy => properties.contains(ash::vk::MemoryPropertyFlags::LAZILY_ALLOCATED),
        };
        
        if required {
            return Ok(i as u32);
        }
    }
    
    Err(GpuAllocError::UnsupportedUsage)
}
