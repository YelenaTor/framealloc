//! Unified CPU-GPU allocation example
//! 
//! Demonstrates using framealloc for both CPU and GPU memory management
//! with the unified coordinator.

#[cfg(all(feature = "gpu-vulkan", feature = "coordinator"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use framealloc::{SmartAlloc, AllocConfig};
    use framealloc::gpu::vulkan::VulkanAllocator;
    use framealloc::coordinator::UnifiedAllocator;
    use framealloc::gpu::traits::{BufferUsage, MemoryType};
    
    println!("=== Unified CPU-GPU Allocation Example ===\n");
    
    // Create CPU allocator
    let cpu_alloc = SmartAlloc::new(AllocConfig::default());
    
    // Note: In a real application, you would initialize Vulkan properly
    // and pass actual device/physical device instances
    println!("This example requires a properly initialized Vulkan instance.");
    println!("For demonstration purposes, we'll show the API usage:\n");
    
    println!("// Create unified allocator");
    println!("let mut unified = UnifiedAllocator::new(cpu_alloc, gpu_alloc);");
    println!();
    
    println!("// Begin frame");
    println!("unified.begin_frame();");
    println!();
    
    println!("// Create CPU-only buffer");
    println!("let cpu_buffer = unified.create_cpu_buffer(1024)?;");
    println!();
    
    println!("// Create GPU-only buffer");
    println!("let gpu_buffer = unified.create_gpu_buffer(");
    println!("    4096, ");
    println!("    BufferUsage::VERTEX_BUFFER | BufferUsage::TRANSFER_DST, ");
    println!("    MemoryType::DeviceLocal");
    println!(")?;");
    println!();
    
    println!("// Create staging buffer for CPU-GPU transfer");
    println!("let staging = unified.create_staging_buffer(2048)?;");
    println!();
    
    println!("// Transfer data to GPU");
    println!("unified.transfer_to_gpu(&mut staging)?;");
    println!();
    
    println!("// Check memory usage");
    println!("let (cpu_usage, gpu_usage) = unified.get_usage();");
    println!("println!(\"CPU: {} MB, GPU: {} MB\", ");
    println!("    cpu_usage / 1024 / 1024, ");
    println!("    gpu_usage / 1024 / 1024);");
    println!();
    
    println!("// End frame (cleans up frame allocations)");
    println!("unified.end_frame();");
    
    Ok(())
}

#[cfg(not(all(feature = "gpu-vulkan", feature = "coordinator")))]
fn main() {
    println!("This example requires the 'gpu-vulkan' and 'coordinator' features enabled.");
    println!("Run with: cargo run --example 08_unified_allocation --features gpu-vulkan,coordinator");
}
