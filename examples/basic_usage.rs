//! Basic usage example for framealloc.
//!
//! Run with: cargo run --example basic_usage

use framealloc::{SmartAlloc, AllocConfig};

fn main() {
    println!("=== framealloc Basic Usage ===\n");

    // Create allocator with default configuration
    let alloc = SmartAlloc::new(AllocConfig::default());

    // === Frame Allocation ===
    println!("1. Frame Allocation (fastest path)");
    
    alloc.begin_frame();
    
    // Safe wrapper - preferred API
    let data = alloc.frame_box(42u64).expect("allocation failed");
    println!("   Allocated frame_box with value: {}", *data);
    
    // Allocate a slice
    let mut slice = alloc.frame_slice::<f32>(100).expect("slice allocation failed");
    slice[0] = 3.14;
    slice[99] = 2.71;
    println!("   Allocated frame_slice with {} elements", slice.len());
    
    alloc.end_frame();
    println!("   Frame ended - all frame allocations invalidated\n");

    // === Pool Allocation ===
    println!("2. Pool Allocation (for small objects)");
    
    {
        let boxed = alloc.pool_box(123u64).expect("pool allocation failed");
        println!("   Allocated pool_box with value: {}", *boxed);
        // Automatically freed when dropped
    }
    println!("   pool_box automatically freed on drop\n");

    // === Heap Allocation ===
    println!("3. Heap Allocation (for large objects)");
    
    {
        let large = alloc.heap_box([0u8; 8192]).expect("heap allocation failed");
        println!("   Allocated heap_box with {} bytes", large.len());
        // Automatically freed when dropped
    }
    println!("   heap_box automatically freed on drop\n");

    // === Statistics ===
    println!("4. Allocation Statistics");
    let stats = alloc.stats();
    println!("   Total allocated: {} bytes", stats.total_allocated);
    println!("   Peak allocated: {} bytes", stats.peak_allocated);
    println!("   Allocation count: {}", stats.allocation_count);
    println!("   Deallocation count: {}", stats.deallocation_count);

    println!("\n=== Done ===");
}
