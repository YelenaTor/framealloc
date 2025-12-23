//! Unified allocator that coordinates CPU and GPU memory

use crate::cpu::SmartAlloc;
use crate::gpu::traits::{
    GpuAllocator, GpuBuffer, GpuAllocError, BufferUsage, MemoryType,
    GpuMemoryIntent, GpuLifetime, GpuAllocRequirements, GpuAllocStats,
};
use super::budget::UnifiedBudgetManager;
use std::sync::Arc;

/// A unified allocator that manages both CPU and GPU memory
pub struct UnifiedAllocator {
    /// CPU allocator
    cpu_alloc: SmartAlloc,
    /// GPU allocator
    gpu_alloc: Box<dyn GpuAllocator>,
    /// Budget manager for unified memory tracking
    budget_manager: UnifiedBudgetManager,
}

/// Errors that can occur in unified allocation
#[derive(Debug, Clone)]
pub enum UnifiedError {
    /// CPU allocation failed
    CpuAllocationFailed(String),
    /// GPU allocation failed
    GpuAllocationFailed(GpuAllocError),
    /// Budget exceeded
    BudgetExceeded,
    /// Invalid transfer between CPU and GPU
    InvalidTransfer,
}

impl std::fmt::Display for UnifiedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnifiedError::CpuAllocationFailed(msg) => write!(f, "CPU allocation failed: {}", msg),
            UnifiedError::GpuAllocationFailed(err) => write!(f, "GPU allocation failed: {}", err),
            UnifiedError::BudgetExceeded => write!(f, "Unified memory budget exceeded"),
            UnifiedError::InvalidTransfer => write!(f, "Invalid CPU-GPU transfer"),
        }
    }
}

impl std::error::Error for UnifiedError {}

/// A unified buffer that can exist on both CPU and GPU
pub struct UnifiedBuffer {
    /// CPU-side allocation (if any)
    cpu_data: Option<Vec<u8>>,
    /// GPU-side buffer (if any)
    gpu_buffer: Option<Box<dyn GpuBuffer>>,
    /// Current location of the data
    location: BufferLocation,
    /// Size in bytes
    size: usize,
}

/// Where the buffer data currently resides
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferLocation {
    /// Data is only on CPU
    CpuOnly,
    /// Data is only on GPU
    GpuOnly,
    /// Data is synchronized between CPU and GPU
    Synchronized,
}

impl UnifiedAllocator {
    /// Create a new unified allocator
    pub fn new(cpu_alloc: SmartAlloc, gpu_alloc: Box<dyn GpuAllocator>) -> Self {
        Self {
            cpu_alloc,
            gpu_alloc,
            budget_manager: UnifiedBudgetManager::new(),
        }
    }
    
    /// Begin a new frame (for CPU allocations)
    pub fn begin_frame(&mut self) {
        self.cpu_alloc.begin_frame();
    }
    
    /// End the current frame and clean up
    pub fn end_frame(&mut self) {
        self.cpu_alloc.end_frame();
        self.budget_manager.reset_frame_usage();
    }
    
    /// Create a CPU-only buffer
    pub fn create_cpu_buffer(&mut self, size: usize) -> Result<UnifiedBuffer, UnifiedError> {
        // Check budget
        if !self.budget_manager.check_cpu_budget(size) {
            return Err(UnifiedError::BudgetExceeded);
        }
        
        // Allocate on CPU
        let cpu_data = vec![0u8; size];
        
        self.budget_manager.add_cpu_usage(size);
        
        Ok(UnifiedBuffer {
            cpu_data: Some(cpu_data),
            gpu_buffer: None,
            location: BufferLocation::CpuOnly,
            size,
        })
    }
    
    /// Create a GPU-only buffer with intent
    pub fn create_gpu_buffer_with_intent(
        &mut self,
        size: usize,
        intent: GpuMemoryIntent,
        lifetime: GpuLifetime,
    ) -> Result<UnifiedBuffer, UnifiedError> {
        // Check budget
        if !self.budget_manager.check_gpu_budget(size) {
            return Err(UnifiedError::BudgetExceeded);
        }
        
        // Create requirements
        let req = GpuAllocRequirements::new(size, intent, lifetime);
        
        // Allocate on GPU
        let gpu_buffer = self.gpu_alloc.allocate(req)
            .map_err(UnifiedError::GpuAllocationFailed)?;
        
        self.budget_manager.add_gpu_usage(size);
        
        Ok(UnifiedBuffer {
            cpu_data: None,
            gpu_buffer: Some(gpu_buffer),
            location: BufferLocation::GpuOnly,
            size,
        })
    }
    
    /// Create a GPU-only buffer (legacy API for compatibility)
    pub fn create_gpu_buffer(
        &mut self,
        size: usize,
        usage: BufferUsage,
        memory_type: MemoryType,
    ) -> Result<UnifiedBuffer, UnifiedError> {
        // Convert legacy types to intent
        let intent = match memory_type {
            MemoryType::DeviceLocal => GpuMemoryIntent::DeviceOnly,
            MemoryType::HostVisible | MemoryType::HostCoherent => GpuMemoryIntent::HostVisible,
            MemoryType::HostCached => GpuMemoryIntent::HostCached,
            MemoryType::Lazy => GpuMemoryIntent::DeviceOnly, // Best effort
        };
        
        self.create_gpu_buffer_with_intent(size, intent, GpuLifetime::Persistent)
    }
    
    /// Create a staging buffer for CPU-GPU transfers
    pub fn create_staging_buffer(&mut self, size: usize) -> Result<UnifiedBuffer, UnifiedError> {
        // Check combined budget
        if !self.budget_manager.check_combined_budget(size * 2) {
            return Err(UnifiedError::BudgetExceeded);
        }
        
        // Allocate on both CPU and GPU
        let cpu_data = vec![0u8; size];
        
        // Create staging requirements
        let req = GpuAllocRequirements::new(size, GpuMemoryIntent::Staging, GpuLifetime::Frame);
        
        let gpu_buffer = self.gpu_alloc.allocate(req)
            .map_err(UnifiedError::GpuAllocationFailed)?;
        
        self.budget_manager.add_cpu_usage(size);
        self.budget_manager.add_gpu_usage(size);
        
        Ok(UnifiedBuffer {
            cpu_data: Some(cpu_data),
            gpu_buffer: Some(gpu_buffer),
            location: BufferLocation::Synchronized,
            size,
        })
    }
    
    /// Transfer data from CPU to GPU
    pub fn transfer_to_gpu(&mut self, buffer: &mut UnifiedBuffer) -> Result<(), UnifiedError> {
        if buffer.cpu_data.is_none() {
            return Err(UnifiedError::InvalidTransfer);
        }
        
        if buffer.gpu_buffer.is_none() {
            // Create GPU buffer if it doesn't exist
            let gpu_buffer = self.gpu_alloc.allocate_buffer(
                buffer.size,
                BufferUsage::TRANSFER_DST,
                MemoryType::DeviceLocal,
            ).map_err(UnifiedError::GpuAllocationFailed)?;
            
            buffer.gpu_buffer = Some(gpu_buffer);
            self.budget_manager.add_gpu_usage(buffer.size);
        }
        
        // Map GPU buffer and copy data
        if let Some(gpu_buf) = buffer.gpu_buffer.as_mut() {
            if let Some(ptr) = gpu_buf.map() {
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        buffer.cpu_data.as_ref().unwrap().as_ptr(),
                        ptr,
                        buffer.size,
                    );
                }
                gpu_buf.flush(0, buffer.size)
                    .map_err(UnifiedError::GpuAllocationFailed)?;
                gpu_buf.unmap();
            }
        }
        
        buffer.location = BufferLocation::Synchronized;
        Ok(())
    }
    
    /// Get current memory usage statistics
    pub fn get_usage(&self) -> (usize, usize) {
        (self.budget_manager.cpu_usage(), self.budget_manager.gpu_usage())
    }
    
    /// Get the underlying CPU allocator
    pub fn cpu_allocator(&self) -> &SmartAlloc {
        &self.cpu_alloc
    }
    
    /// Get the underlying GPU allocator
    pub fn gpu_allocator(&self) -> &dyn GpuAllocator {
        self.gpu_alloc.as_ref()
    }
}

impl UnifiedBuffer {
    /// Get CPU data slice (if available)
    pub fn cpu_slice(&self) -> Option<&[u8]> {
        self.cpu_data.as_deref()
    }
    
    /// Get mutable CPU data slice (if available)
    pub fn cpu_slice_mut(&mut self) -> Option<&mut [u8]> {
        self.cpu_data.as_deref_mut()
    }
    
    /// Get GPU buffer (if available)
    pub fn gpu_buffer(&self) -> Option<&dyn GpuBuffer> {
        self.gpu_buffer.as_deref()
    }
    
    /// Get the current location of the buffer
    pub fn location(&self) -> BufferLocation {
        self.location
    }
    
    /// Get the size of the buffer
    pub fn size(&self) -> usize {
        self.size
    }
}
