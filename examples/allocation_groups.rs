//! Allocation groups example - free multiple allocations at once.
//!
//! Run with: cargo run --example allocation_groups

use framealloc::{SmartAlloc, AllocConfig};

fn main() {
    println!("=== Allocation Groups Example ===\n");

    let alloc = SmartAlloc::new(AllocConfig::default());
    let groups = alloc.groups();

    // Create groups for different game levels
    println!("1. Creating allocation groups...");
    
    let level_1 = groups.create_group("level_1_assets");
    let level_2 = groups.create_group("level_2_assets");
    
    println!("   Created group: {:?}", groups.group_name(level_1));
    println!("   Created group: {:?}", groups.group_name(level_2));

    // Allocate assets for level 1
    println!("\n2. Allocating Level 1 assets...");
    
    for i in 0..10 {
        let ptr = groups.alloc_val(level_1, [0u8; 1024]).expect("alloc failed");
        if i == 0 {
            println!("   First allocation at {:?}", ptr);
        }
    }
    println!("   Allocated 10 assets");
    println!("   Level 1 size: {} bytes", groups.group_size(level_1));
    println!("   Level 1 count: {}", groups.group_count(level_1));

    // Allocate assets for level 2
    println!("\n3. Allocating Level 2 assets...");
    
    for _ in 0..5 {
        groups.alloc_val(level_2, [0u8; 2048]).expect("alloc failed");
    }
    println!("   Allocated 5 assets");
    println!("   Level 2 size: {} bytes", groups.group_size(level_2));
    println!("   Level 2 count: {}", groups.group_count(level_2));

    // Statistics
    println!("\n4. Group Statistics:");
    let stats = groups.stats();
    println!("   Total groups: {}", stats.total_groups);
    println!("   Total allocations: {}", stats.total_allocations);
    println!("   Total bytes: {}", stats.total_bytes);

    // Unload level 1 (free all its allocations at once)
    println!("\n5. Unloading Level 1 (freeing all at once)...");
    groups.free_group(level_1);
    println!("   Level 1 exists: {}", groups.group_exists(level_1));

    // Level 2 still active
    println!("\n6. Level 2 still active:");
    println!("   Level 2 exists: {}", groups.group_exists(level_2));
    println!("   Level 2 count: {}", groups.group_count(level_2));

    // Using GroupHandle for convenience
    println!("\n7. Using GroupHandle API...");
    
    use framealloc::api::groups::GroupHandle;
    
    let level_3_id = groups.create_group("level_3");
    let level_3 = GroupHandle::new(groups, level_3_id);
    
    level_3.alloc_val([0u8; 512]).expect("alloc failed");
    level_3.alloc_val([0u8; 512]).expect("alloc failed");
    
    println!("   Level 3 allocations: {}", level_3.count());
    println!("   Level 3 size: {} bytes", level_3.size());
    
    level_3.free_all();
    println!("   Level 3 freed");

    // Cleanup
    groups.free_group(level_2);

    println!("\n=== Done ===");
}
