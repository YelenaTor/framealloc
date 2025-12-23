//! Handle-based allocation example with defragmentation.
//!
//! Run with: cargo run --example handle_allocation

use framealloc::{SmartAlloc, AllocConfig, Handle};

#[derive(Debug)]
struct GameObject {
    id: u32,
    x: f32,
    y: f32,
    health: i32,
}

fn main() {
    println!("=== Handle-Based Allocation Example ===\n");

    let alloc = SmartAlloc::new(AllocConfig::default());
    let handles = alloc.handles();

    // Allocate game objects using handles
    println!("1. Allocating game objects...");
    
    let mut object_handles: Vec<Handle<GameObject>> = Vec::new();
    
    for i in 0..5 {
        let handle: Handle<GameObject> = handles.alloc().expect("allocation failed");
        
        // Initialize the object
        if let Some(ptr) = handles.resolve_mut(handle) {
            unsafe {
                (*ptr) = GameObject {
                    id: i,
                    x: i as f32 * 10.0,
                    y: i as f32 * 5.0,
                    health: 100,
                };
            }
        }
        
        object_handles.push(handle);
        println!("   Allocated object {} with handle index {}", i, handle.raw_index());
    }

    // Access objects through handles
    println!("\n2. Accessing objects through handles...");
    
    for handle in &object_handles {
        if let Some(ptr) = handles.resolve(*handle) {
            let obj = unsafe { &*ptr };
            println!("   Object {}: pos=({}, {}), health={}", 
                     obj.id, obj.x, obj.y, obj.health);
        }
    }

    // Free some objects (creates fragmentation)
    println!("\n3. Freeing objects 1 and 3 (creating fragmentation)...");
    
    handles.free(object_handles[1]);
    handles.free(object_handles[3]);
    
    // Handles are now invalid
    println!("   Handle 1 valid: {}", handles.is_valid(object_handles[1]));
    println!("   Handle 3 valid: {}", handles.is_valid(object_handles[3]));

    // Allocate new objects (may reuse slots)
    println!("\n4. Allocating new objects (reusing slots)...");
    
    let new_handle: Handle<GameObject> = handles.alloc().expect("allocation failed");
    if let Some(ptr) = handles.resolve_mut(new_handle) {
        unsafe {
            (*ptr) = GameObject {
                id: 100,
                x: 0.0,
                y: 0.0,
                health: 50,
            };
        }
    }
    println!("   New object allocated at index {}", new_handle.raw_index());

    // Pin an object to prevent relocation
    println!("\n5. Pinning object 0...");
    handles.pin(object_handles[0]);
    
    let stats = handles.stats();
    println!("   Pinned count: {}", stats.pinned_count);
    println!("   Relocatable count: {}", stats.relocatable_count);

    // Defragment (would relocate unpinned allocations)
    println!("\n6. Running defragmentation...");
    let relocations = handles.defragment();
    println!("   Relocations performed: {}", relocations);

    // Unpin and cleanup
    handles.unpin(object_handles[0]);

    // Statistics
    println!("\n7. Final Statistics:");
    let stats = handles.stats();
    println!("   Active handles: {}", stats.active_handles);
    println!("   Total allocated: {} bytes", stats.total_allocated);
    println!("   Total relocations: {}", stats.relocation_count);

    println!("\n=== Done ===");
}
