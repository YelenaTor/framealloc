//! Dummy GPU allocator implementation for testing
//! 
//! This allocator uses simple Vec<u8> storage and doesn't require actual GPU hardware.

use super::traits::*;
use std::collections::HashMap;

/// A dummy GPU buffer that just stores bytes in RAM
#[derive(Debug)]
pub struct DummyBuffer {
    /// The actual data
    data: Vec<u8>,
    /// Memory intent
    intent: GpuMemoryIntent,
    /// Expected lifetime
    lifetime: GpuLifetime,
    /// Whether currently mapped
    mapped: bool,
}

impl DummyBuffer {
    /// Create a new dummy buffer
    fn new(size: usize, intent: GpuMemoryIntent, lifetime: GpuLifetime) -> Self {
        Self {
            data: vec![0u8; size],
            intent,
            lifetime,
            mapped: false,
        }
    }
}

impl GpuBuffer for DummyBuffer {
    fn size(&self) -> usize {
        self.data.len()
    }
    
    fn intent(&self) -> GpuMemoryIntent {
        self.intent
    }
    
    fn lifetime(&self) -> GpuLifetime {
        self.lifetime
    }
    
    fn raw_handle(&self) -> *mut std::ffi::c_void {
        self.data.as_ptr() as *mut std::ffi::c_void
    }
    
    fn map(&mut self) -> Option<*mut u8> {
        if matches!(self.intent, GpuMemoryIntent::DeviceOnly) {
            None
        } else {
            self.mapped = true;
            Some(self.data.as_mut_ptr())
        }
    }
    
    fn unmap(&mut self) {
        self.mapped = false;
    }
    
    fn flush(&self, _offset: usize, _size: usize) -> Result<(), GpuAllocError> {
        if self.mapped {
            Ok(())
        } else {
            Err(GpuAllocError::BackendError("Buffer not mapped".to_string()))
        }
    }
    
    fn invalidate(&self, _offset: usize, _size: usize) -> Result<(), GpuAllocError> {
        if self.mapped {
            Ok(())
        } else {
            Err(GpuAllocError::BackendError("Buffer not mapped".to_string()))
        }
    }
}

/// Dummy GPU allocator for testing
#[derive(Debug)]
pub struct DummyAllocator {
    /// Current statistics
    stats: GpuAllocStats,
    /// Current frame index
    current_frame: u64,
    /// Supported intents
    supported_intents: HashMap<GpuMemoryIntent, bool>,
}

impl DummyAllocator {
    /// Create a new dummy allocator
    pub fn new() -> Self {
        let mut supported_intents = HashMap::new();
        supported_intents.insert(GpuMemoryIntent::DeviceOnly, true);
        supported_intents.insert(GpuMemoryIntent::HostVisible, true);
        supported_intents.insert(GpuMemoryIntent::HostCached, true);
        supported_intents.insert(GpuMemoryIntent::Staging, true);
        supported_intents.insert(GpuMemoryIntent::Readback, true);
        
        Self {
            stats: GpuAllocStats::default(),
            current_frame: 0,
            supported_intents,
        }
    }
    
    /// Create a dummy allocator with limited support
    pub fn with_limited_support() -> Self {
        let mut supported_intents = HashMap::new();
        supported_intents.insert(GpuMemoryIntent::DeviceOnly, true);
        supported_intents.insert(GpuMemoryIntent::Staging, true);
        
        Self {
            stats: GpuAllocStats::default(),
            current_frame: 0,
            supported_intents,
        }
    }
}

impl Default for DummyAllocator {
    fn default() -> Self {
        Self::new()
    }
}

impl GpuAllocator for DummyAllocator {
    fn allocate(&self, req: GpuAllocRequirements) -> Result<Box<dyn GpuBuffer>, GpuAllocError> {
        // Check if intent is supported
        if !self.supports_intent(req.intent) {
            return Err(GpuAllocError::UnsupportedUsage);
        }
        
        // Check size
        if req.size == 0 {
            return Err(GpuAllocError::InvalidSize);
        }
        
        // Check alignment (dummy allocator just requires power of 2)
        if !req.alignment.is_power_of_two() {
            return Err(GpuAllocError::AlignmentFailed);
        }
        
        // Create buffer
        let buffer = DummyBuffer::new(req.size, req.intent, req.lifetime);
        
        // Update stats would need mutable self - in real implementation use interior mutability
        Ok(Box::new(buffer))
    }
    
    fn deallocate(&self, _buffer: Box<dyn GpuBuffer>) {
        // In real implementation, update stats
    }
    
    fn stats(&self) -> GpuAllocStats {
        self.stats.clone()
    }
    
    fn supports_intent(&self, intent: GpuMemoryIntent) -> bool {
        self.supported_intents.get(&intent).copied().unwrap_or(false)
    }
}

impl GpuFrameAllocator for DummyAllocator {
    fn begin_gpu_frame(&mut self) {
        self.current_frame += 1;
    }
    
    fn end_gpu_frame(&mut self) {
        // Clear frame-local allocations in real implementation
    }
    
    fn current_frame(&self) -> u64 {
        self.current_frame
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_dummy_allocation() {
        let allocator = DummyAllocator::new();
        
        let req = GpuAllocRequirements::new(
            1024,
            GpuMemoryIntent::Staging,
            GpuLifetime::Frame,
        );
        
        let buffer = allocator.allocate(req).unwrap();
        assert_eq!(buffer.size(), 1024);
        assert_eq!(buffer.intent(), GpuMemoryIntent::Staging);
        assert_eq!(buffer.lifetime(), GpuLifetime::Frame);
    }
    
    #[test]
    fn test_device_only_cannot_map() {
        let allocator = DummyAllocator::new();
        
        let req = GpuAllocRequirements::new(
            1024,
            GpuMemoryIntent::DeviceOnly,
            GpuLifetime::Persistent,
        );
        
        let mut buffer = allocator.allocate(req).unwrap();
        assert!(buffer.map().is_none());
    }
    
    #[test]
    fn test_host_visible_can_map() {
        let allocator = DummyAllocator::new();
        
        let req = GpuAllocRequirements::new(
            1024,
            GpuMemoryIntent::HostVisible,
            GpuLifetime::Persistent,
        );
        
        let mut buffer = allocator.allocate(req).unwrap();
        let ptr = buffer.map().unwrap();
        assert!(!ptr.is_null());
    }
    
    #[test]
    fn test_frame_allocator() {
        let mut allocator = DummyAllocator::new();
        
        assert_eq!(allocator.current_frame(), 0);
        
        allocator.begin_gpu_frame();
        assert_eq!(allocator.current_frame(), 1);
        
        allocator.end_gpu_frame();
    }
}
