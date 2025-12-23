//! Streaming allocator example for large assets.
//!
//! Run with: cargo run --example streaming_assets

use framealloc::{SmartAlloc, AllocConfig, StreamPriority};

fn main() {
    println!("=== Streaming Allocator Example ===\n");

    let alloc = SmartAlloc::new(AllocConfig::default());
    let streaming = alloc.streaming();

    // Simulate loading textures
    println!("1. Reserving space for textures...");
    
    let texture_1 = streaming.reserve(1024 * 1024, StreamPriority::Normal)
        .expect("failed to reserve texture 1");
    println!("   Reserved 1MB for texture 1 (id: {:?})", texture_1.raw());
    
    let texture_2 = streaming.reserve(512 * 1024, StreamPriority::High)
        .expect("failed to reserve texture 2");
    println!("   Reserved 512KB for texture 2 (high priority, id: {:?})", texture_2.raw());

    // Simulate loading data
    println!("\n2. Loading texture data...");
    
    let ptr = streaming.begin_load(texture_1).expect("begin_load failed");
    println!("   Started loading texture 1 at {:?}", ptr);
    
    // Simulate progress
    streaming.report_progress(texture_1, 512 * 1024);
    println!("   Texture 1: 50% loaded");
    
    streaming.report_progress(texture_1, 1024 * 1024);
    streaming.finish_load(texture_1);
    println!("   Texture 1: 100% loaded, ready to use");

    // Load texture 2
    streaming.begin_load(texture_2);
    streaming.finish_load(texture_2);
    println!("   Texture 2: loaded and ready");

    // Access textures
    println!("\n3. Accessing textures...");
    
    if let Some(data) = streaming.access(texture_1) {
        println!("   Texture 1 accessible at {:?}", data);
    }

    // Statistics
    println!("\n4. Streaming Statistics:");
    let stats = streaming.stats();
    println!("   Budget: {} bytes", stats.budget);
    println!("   Reserved: {} bytes", stats.total_reserved);
    println!("   Loaded: {} bytes", stats.total_loaded);
    println!("   Active allocations: {}", stats.allocation_count);
    println!("   Ready: {}", stats.ready_count);

    // Free textures
    println!("\n5. Freeing textures...");
    streaming.free(texture_1);
    streaming.free(texture_2);
    println!("   All textures freed");

    let stats = streaming.stats();
    println!("   Final reserved: {} bytes", stats.total_reserved);

    println!("\n=== Done ===");
}
