//! Custom Allocator example
//! 
//! Demonstrates implementing a custom allocator backend

use framealloc::{SmartAlloc, AllocConfig, AllocatorBackend, AllocationResult};
use std::alloc::{GlobalAlloc, Layout, System};
use std::ptr::NonNull;
use std::sync::atomic::{AtomicUsize, Ordering};

// Custom allocator that tracks allocations
struct TrackingAllocator {
    total_allocated: AtomicUsize,
    allocation_count: AtomicUsize,
    peak_usage: AtomicUsize,
}

impl TrackingAllocator {
    fn new() -> Self {
        Self {
            total_allocated: AtomicUsize::new(0),
            allocation_count: AtomicUsize::new(0),
            peak_usage: AtomicUsize::new(0),
        }
    }
    
    fn stats(&self) -> AllocatorStats {
        AllocatorStats {
            total_allocated: self.total_allocated.load(Ordering::Relaxed),
            allocation_count: self.allocation_count.load(Ordering::Relaxed),
            peak_usage: self.peak_usage.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug)]
struct AllocatorStats {
    total_allocated: usize,
    allocation_count: usize,
    peak_usage: usize,
}

unsafe impl AllocatorBackend for TrackingAllocator {
    fn allocate(&mut self, layout: std::alloc::Layout) -> AllocationResult {
        // Try our custom allocation first
        if layout.size() <= 4096 {
            // Use system allocator but track it
            unsafe {
                let ptr = System.alloc(layout);
                if let Some(ptr) = NonNull::new(ptr) {
                    let size = layout.size();
                    self.allocation_count.fetch_add(1, Ordering::Relaxed);
                    
                    // Update peak usage
                    let current = self.total_allocated.fetch_add(size, Ordering::Relaxed) + size;
                    loop {
                        let peak = self.peak_usage.load(Ordering::Relaxed);
                        if current <= peak {
                            break;
                        }
                        if self.peak_usage.compare_exchange_weak(
                            peak,
                            current,
                            Ordering::Relaxed,
                            Ordering::Relaxed,
                        ).is_ok() {
                            break;
                        }
                    }
                    
                    return AllocationResult::Ok(ptr);
                }
            }
        }
        
        // Fall back to default for large allocations
        AllocationResult::Fallback
    }
    
    fn deallocate(&mut self, ptr: NonNull<u8>, layout: std::alloc::Layout) {
        if layout.size() <= 4096 {
            let size = layout.size();
            self.total_allocated.fetch_sub(size, Ordering::Relaxed);
            self.allocation_count.fetch_sub(1, Ordering::Relaxed);
            unsafe {
                System.dealloc(ptr.as_ptr(), layout);
            }
        }
    }
}

// Arena allocator for contiguous memory
struct ArenaAllocator {
    memory: NonNull<u8>,
    size: usize,
    offset: AtomicUsize,
}

impl ArenaAllocator {
    fn new(size: usize) -> Self {
        let layout = std::alloc::Layout::from_size_align(size, 8).unwrap();
        let memory = unsafe {
            NonNull::new(System.alloc(layout)).expect("Failed to allocate arena")
        };
        
        Self {
            memory,
            size,
            offset: AtomicUsize::new(0),
        }
    }
    
    fn allocate(&self, layout: std::alloc::Layout) -> Option<NonNull<u8>> {
        let current_offset = self.offset.load(Ordering::Relaxed);
        let new_offset = current_offset + layout.size();
        let aligned_offset = (new_offset + (layout.align() - 1)) & !(layout.align() - 1);
        
        if aligned_offset <= self.size {
            if self.offset.compare_exchange_weak(
                current_offset,
                aligned_offset,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ).is_ok() {
                unsafe {
                    let ptr = self.memory.as_ptr().add(current_offset);
                    Some(NonNull::new_unchecked(ptr))
                }
            } else {
                self.allocate(layout) // Retry
            }
        } else {
            None
        }
    }
}

impl Drop for ArenaAllocator {
    fn drop(&mut self) {
        unsafe {
            let layout = std::alloc::Layout::from_size_align(self.size, 8).unwrap();
            System.dealloc(self.memory.as_ptr(), layout);
        }
    }
}

// Pool allocator for fixed-size objects
struct PoolAllocator {
    object_size: usize,
    alignment: usize,
    free_list: Vec<NonNull<u8>>,
    chunks: Vec<NonNull<u8>>,
}

impl PoolAllocator {
    fn new(object_size: usize, alignment: usize) -> Self {
        Self {
            object_size,
            alignment,
            free_list: Vec::new(),
            chunks: Vec::new(),
        }
    }
    
    fn allocate(&mut self) -> Option<NonNull<u8>> {
        if let Some(ptr) = self.free_list.pop() {
            Some(ptr)
        } else {
            self.grow_chunk()
        }
    }
    
    fn deallocate(&mut self, ptr: NonNull<u8>) {
        self.free_list.push(ptr);
    }
    
    fn grow_chunk(&mut self) -> Option<NonNull<u8>> {
        const CHUNK_SIZE: usize = 64;
        let chunk_size = self.object_size * CHUNK_SIZE;
        let layout = std::alloc::Layout::from_size_align(chunk_size, self.alignment).unwrap();
        
        unsafe {
            let chunk = System.alloc(layout);
            if let Some(chunk) = NonNull::new(chunk) {
                self.chunks.push(chunk);
                
                // Add to free list
                for i in 0..CHUNK_SIZE {
                    let ptr = chunk.as_ptr().add(i * self.object_size);
                    self.free_list.push(NonNull::new_unchecked(ptr));
                }
                
                self.free_list.pop()
            } else {
                None
            }
        }
    }
}

impl Drop for PoolAllocator {
    fn drop(&mut self) {
        for chunk in &self.chunks {
            unsafe {
                let layout = std::alloc::Layout::from_size_align(
                    self.object_size * 64, 
                    self.alignment
                ).unwrap();
                System.dealloc(chunk.as_ptr(), layout);
            }
        }
    }
}

fn main() {
    println!("=== Custom Allocator Demo ===\n");
    
    // Demo 1: Tracking allocator
    println!("1. Tracking Allocator:");
    demo_tracking_allocator();
    
    // Demo 2: Arena allocator
    println!("\n2. Arena Allocator:");
    demo_arena_allocator();
    
    // Demo 3: Pool allocator
    println!("\n3. Pool Allocator:");
    demo_pool_allocator();
    
    // Demo 4: Custom backend with FrameAlloc
    println!("\n4. FrameAlloc with Custom Backend:");
    demo_framealloc_custom_backend();
    
    println!("\nAll custom allocator demos completed!");
}

fn demo_tracking_allocator() {
    let mut tracking = TrackingAllocator::new();
    
    // Simulate some allocations
    let layouts = [
        std::alloc::Layout::new::<u8>(),
        std::alloc::Layout::new::<u32>(),
        std::alloc::Layout::new::<[u8; 1024]>(),
    ];
    
    for layout in &layouts {
        match tracking.allocate(*layout) {
            AllocationResult::Ok(ptr) => {
                println!("  Allocated {} bytes", layout.size());
                // Simulate usage
                unsafe {
                    std::ptr::write(ptr.as_ptr() as *mut u8, 42);
                }
            }
            AllocationResult::Fallback => {
                println!("  Used fallback for {} bytes", layout.size());
            }
        }
    }
    
    let stats = tracking.stats();
    println!("  Stats: {} allocations, {} bytes total, {} peak",
        stats.allocation_count, stats.total_allocated, stats.peak_usage);
}

fn demo_arena_allocator() {
    let arena = ArenaAllocator::new(1024 * 1024); // 1MB arena
    
    // Allocate various sizes
    let mut allocations = Vec::new();
    
    for i in 0..10 {
        let layout = std::alloc::Layout::array::<u32>(100).unwrap();
        if let Some(ptr) = arena.allocate(layout) {
            allocations.push((ptr, layout));
            println!("  Arena allocated {} bytes at {:?}", layout.size(), ptr);
        }
    }
    
    println!("  Arena used {} allocations", allocations.len());
    
    // Arena automatically frees all at once
}

fn demo_pool_allocator() {
    let mut pool = PoolAllocator::new(64, 8); // 64-byte objects, 8-byte aligned
    
    // Allocate and deallocate
    let mut objects = Vec::new();
    
    // Allocate many objects
    for i in 0..100 {
        if let Some(ptr) = pool.allocate() {
            objects.push(ptr);
            unsafe {
                std::ptr::write(ptr.as_ptr() as *mut u64, i as u64);
            }
        }
    }
    
    println!("  Pool allocated {} objects", objects.len());
    
    // Return all to pool
    for ptr in objects {
        pool.deallocate(ptr);
    }
    
    // Allocate again - should reuse
    for i in 0..10 {
        if let Some(ptr) = pool.allocate() {
            println!("  Reused pool object at {:?}", ptr);
        }
    }
}

fn demo_framealloc_custom_backend() {
    // Create FrameAlloc with custom tracking backend
    let config = AllocConfig::default()
        .with_backend(Box::new(TrackingAllocator::new()));
    
    let alloc = SmartAlloc::new(config);
    
    alloc.begin_frame();
    
    // Make various allocations
    let frame_data = alloc.frame_vec::<u32>();
    for i in 0..1000 {
        frame_data.push(i);
    }
    
    let pool_data = alloc.pool_box(String::from("Hello, custom allocator!"));
    let heap_data = alloc.heap_box(vec![0u8; 1024]);
    
    println!("  FrameAlloc with custom backend:");
    println!("    Frame data: {} items", frame_data.len());
    println!("    Pool data: {} bytes", pool_data.len());
    println!("    Heap data: {} bytes", heap_data.len());
    
    alloc.end_frame();
    
    // Note: In a real implementation, you'd need to expose the backend
    // to get statistics. This is just a demonstration of the API.
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_tracking_allocator() {
        let mut tracking = TrackingAllocator::new();
        
        let layout = std::alloc::Layout::new::<u32>();
        match tracking.allocate(layout) {
            AllocationResult::Ok(_) => {
                assert_eq!(tracking.allocation_count.load(Ordering::Relaxed), 1);
                assert_eq!(tracking.total_allocated.load(Ordering::Relaxed), 4);
            }
            _ => panic!("Allocation failed"),
        }
    }
    
    #[test]
    fn test_arena_allocator() {
        let arena = ArenaAllocator::new(1024);
        
        let layout = std::alloc::Layout::new::<u32>();
        let ptr1 = arena.allocate(layout).unwrap();
        let ptr2 = arena.allocate(layout).unwrap();
        
        // Should be different pointers
        assert_ne!(ptr1, ptr2);
        
        // Should be aligned
        assert_eq!(ptr1.as_ptr() as usize % layout.align(), 0);
    }
    
    #[test]
    fn test_pool_allocator() {
        let mut pool = PoolAllocator::new(32, 8);
        
        let ptr1 = pool.allocate().unwrap();
        let ptr2 = pool.allocate().unwrap();
        
        pool.deallocate(ptr1);
        pool.deallocate(ptr2);
        
        // Should reuse after deallocation
        let ptr3 = pool.allocate().unwrap();
        assert_eq!(ptr3, ptr2); // Last deallocated
    }
}
