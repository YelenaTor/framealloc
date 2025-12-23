//! Pools and Heaps example
//! 
//! Demonstrates pool and heap allocation for persistent data

use framealloc::SmartAlloc;
use std::collections::HashMap;

#[derive(Debug, Clone)]
struct Entity {
    id: u32,
    x: f32,
    y: f32,
    health: i32,
}

impl Entity {
    fn new(id: u32) -> Self {
        Self {
            id,
            x: 0.0,
            y: 0.0,
            health: 100,
        }
    }
}

#[derive(Debug)]
struct LargeTexture {
    width: u32,
    height: u32,
    data: Vec<u8>,
}

impl LargeTexture {
    fn new(width: u32, height: u32) -> Self {
        let size = (width * height * 4) as usize;
        Self {
            width,
            height,
            data: vec![0; size],
        }
    }
}

fn main() {
    let alloc = SmartAlloc::new(Default::default());
    
    println!("=== Pool Allocation Demo ===");
    
    // Pool allocation for small, reusable objects
    let mut entities = Vec::new();
    
    // Create entities using pool allocation
    for i in 0..10 {
        let entity = alloc.pool_box(Entity::new(i));
        entities.push(entity);
    }
    
    println!("Created {} entities in pool", entities.len());
    
    // Use the entities
    for entity in &entities {
        println!("Entity {}: health={}, pos=({:.1}, {:.1})", 
            entity.id, entity.health, entity.x, entity.y);
    }
    
    // Demonstrate automatic pool management
    println!("\nDropping half the entities...");
    entities.truncate(5);
    
    // Create more entities - they'll reuse the pool memory
    for i in 10..15 {
        let entity = alloc.pool_box(Entity::new(i));
        entities.push(entity);
    }
    
    println!("Pool automatically reused memory for new entities");
    
    println!("\n=== Heap Allocation Demo ===");
    
    // Heap allocation for large objects
    let mut textures = HashMap::new();
    
    // Create large textures using heap allocation
    let texture_names = ["grass", "stone", "water", "sand", "dirt"];
    for (i, name) in texture_names.iter().enumerate() {
        let texture = alloc.heap_box(LargeTexture::new(512, 512));
        println!("Created {} texture: {}x{} ({} bytes)", 
            name, texture.width, texture.height, texture.data.len());
        textures.insert(name.to_string(), texture);
    }
    
    println!("\nTotal textures: {}", textures.len());
    
    // Show heap vs frame allocation
    println!("\n=== Frame vs Pool vs Heap ===");
    
    alloc.begin_frame();
    
    // Frame allocation - temporary
    let temp_data = alloc.frame_vec::<u32>();
    for i in 0..1000 {
        temp_data.push(i);
    }
    println!("Frame allocation: {} items (temporary)", temp_data.len());
    
    // Pool allocation - persistent, small
    let pool_data = alloc.pool_box(vec![1, 2, 3, 4, 5]);
    println!("Pool allocation: {} items (persistent, small)", pool_data.len());
    
    // Heap allocation - persistent, large
    let heap_data = alloc.heap_box(vec![0u8; 1024 * 1024]);
    println!("Heap allocation: {} bytes (persistent, large)", heap_data.len());
    
    alloc.end_frame();
    println!("Frame ended - frame data freed, pool and heap data remain");
    
    // Demonstrate Arc sharing
    println!("\n=== Arc Sharing Demo ===");
    
    let shared_data = alloc.pool_arc(String::from("Shared configuration"));
    let mut handles = Vec::new();
    
    for i in 0..3 {
        let data_clone = shared_data.clone();
        let handle = std::thread::spawn(move || {
            println!("Thread {} sees: {}", i, data_clone);
        });
        handles.push(handle);
    }
    
    for handle in handles {
        handle.join().unwrap();
    }
    
    println!("\nAll demos completed!");
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_pool_reuse() {
        let alloc = SmartAlloc::new(Default::default());
        
        // Create and drop entities
        {
            let _e1 = alloc.pool_box(Entity::new(1));
            let _e2 = alloc.pool_box(Entity::new(2));
            let _e3 = alloc.pool_box(Entity::new(3));
        } // All dropped back to pool
        
        // New entities should reuse the pool memory
        let _e4 = alloc.pool_box(Entity::new(4));
        let _e5 = alloc.pool_box(Entity::new(5));
    }
    
    #[test]
    fn test_large_heap_allocation() {
        let alloc = SmartAlloc::new(Default::default());
        
        // Large allocation should use heap
        let large = alloc.heap_box(LargeTexture::new(2048, 2048));
        assert!(large.data.len() > 16_000_000); // 16MB+
    }
}
