//! Hello World example for framealloc
//! 
//! Demonstrates basic frame allocation usage

use framealloc::SmartAlloc;

fn main() {
    // Create the allocator
    let alloc = SmartAlloc::new(Default::default());
    
    println!("Hello, framealloc!");
    
    // Basic frame allocation
    alloc.begin_frame();
    
    // Allocate some temporary data
    let message = alloc.frame_box("Hello from frame allocation!");
    let numbers = alloc.frame_vec::<i32>();
    
    // Fill the vector
    for i in 0..10 {
        numbers.push(i * 2);
    }
    
    // Use the data
    println!("Message: {}", message);
    println!("Numbers: {:?}", numbers.as_slice());
    
    // End frame - everything is automatically freed!
    alloc.end_frame();
    
    println!("Frame ended - all memory freed!");
    
    // Demonstrate multiple frames
    println!("\nRunning 5 frames:");
    for frame in 0..5 {
        alloc.begin_frame();
        
        let frame_data = alloc.frame_vec::<String>();
        for i in 0..=frame {
            frame_data.push(format!("Item {} from frame {}", i, frame));
        }
        
        println!("Frame {}: {} items", frame, frame_data.len());
        
        alloc.end_frame();
    }
    
    println!("\nAll frames completed successfully!");
}
