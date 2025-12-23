//! Threading example
//! 
//! Demonstrates thread-safe allocation patterns and cross-thread data transfer

use framealloc::SmartAlloc;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
struct WorkItem {
    id: u32,
    data: Vec<f32>,
    complexity: u32,
}

#[derive(Debug)]
struct WorkResult {
    id: u32,
    result: f64,
    processing_time: Duration,
}

fn worker_thread(
    receiver: mpsc::Receiver<framealloc::TransferHandle<WorkItem>>,
    sender: mpsc::Sender<WorkResult>,
    alloc: SmartAlloc,
) {
    println!("Worker thread started");
    
    while let Ok(handle) = receiver.recv() {
        // Receive transferred data
        let work_item = handle.receive();
        
        let start = Instant::now();
        
        // Process the work item
        let result = process_work_item(&work_item);
        let processing_time = start.elapsed();
        
        // Send result back
        let work_result = WorkResult {
            id: work_item.id,
            result,
            processing_time,
        };
        
        if sender.send(work_result).is_err() {
            break; // Main thread dropped
        }
    }
    
    println!("Worker thread finished");
}

fn process_work_item(item: &WorkItem) -> f64 {
    // Simulate complex computation
    let mut sum = 0.0;
    for (i, &value) in item.data.iter().enumerate() {
        sum += value * (i as f32).sin();
        // Simulate work
        thread::sleep(Duration::from_micros(10));
    }
    sum as f64 / item.data.len() as f64
}

fn main() {
    let alloc = SmartAlloc::new(Default::default());
    
    println!("=== Threading Demo ===\n");
    
    // Create communication channels
    let (work_tx, work_rx) = mpsc::channel();
    let (result_tx, result_rx) = mpsc::channel();
    
    // Spawn worker threads
    let num_workers = 4;
    let mut handles = Vec::new();
    
    for i in 0..num_workers {
        let work_rx = work_rx.clone();
        let result_tx = result_tx.clone();
        let alloc_clone = alloc.clone();
        
        let handle = thread::spawn(move || {
            worker_thread(work_rx, result_tx, alloc_clone);
        });
        
        handles.push(handle);
    }
    
    // Drop extra receivers
    drop(work_rx);
    drop(result_tx);
    
    // Generate work items on main thread
    println!("Generating work items...");
    let num_items = 20;
    
    for i in 0..num_items {
        alloc.begin_frame();
        
        // Create work item with frame allocation
        let data = alloc.frame_vec();
        for j in 0..100 {
            data.push((i * 100 + j) as f32);
        }
        
        let work_item = WorkItem {
            id: i,
            data: data.into_inner(),
            complexity: 100,
        };
        
        // Transfer to worker thread
        let transfer_handle = alloc.frame_box_for_transfer(work_item);
        work_tx.send(transfer_handle).unwrap();
        
        alloc.end_frame();
    }
    
    // Drop sender to signal workers to finish when done
    drop(work_tx);
    
    // Collect results
    println!("\nCollecting results...");
    let mut results = Vec::new();
    
    while results.len() < num_items {
        if let Ok(result) = result_rx.recv_timeout(Duration::from_secs(1)) {
            println!("Item {} completed in {:?}: {:.2}", 
                result.id, result.processing_time, result.result);
            results.push(result);
        }
    }
    
    // Wait for all workers to finish
    for handle in handles {
        handle.join().unwrap();
    }
    
    // Calculate statistics
    println!("\n=== Statistics ===");
    let total_time: Duration = results.iter().map(|r| r.processing_time).sum();
    let avg_time = total_time / results.len() as u32;
    let avg_result = results.iter().map(|r| r.result).sum::<f64>() / results.len() as f64;
    
    println!("Total processing time: {:?}", total_time);
    println!("Average per item: {:?}", avg_time);
    println!("Average result: {:.2}", avg_result);
    
    // Demonstrate thread-local allocation
    println!("\n=== Thread-Local Allocation Demo ===");
    demonstrate_thread_local_allocation();
    
    println!("\nAll threading demos completed!");
}

fn demonstrate_thread_local_allocation() {
    let alloc = SmartAlloc::new(Default::default());
    
    // Each thread gets its own frame allocator
    let mut handles = Vec::new();
    
    for thread_id in 0..4 {
        let alloc_clone = alloc.clone();
        
        let handle = thread::spawn(move || {
            // Thread-local frame allocation
            alloc_clone.begin_frame();
            
            // Allocate thread-local data
            let local_data = alloc_clone.frame_vec::<u32>();
            for i in 0..100 {
                local_data.push(thread_id * 1000 + i);
            }
            
            // Simulate work
            thread::sleep(Duration::from_millis(10));
            
            let sum: u32 = local_data.iter().sum();
            
            alloc_clone.end_frame();
            
            (thread_id, sum)
        });
        
        handles.push(handle);
    }
    
    // Collect results
    for handle in handles {
        let (thread_id, sum) = handle.join().unwrap();
        println!("Thread {} sum: {}", thread_id, sum);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cross_thread_transfer() {
        let alloc = SmartAlloc::new(Default::default());
        
        alloc.begin_frame();
        let data = alloc.frame_box(vec![1, 2, 3, 4, 5]);
        let handle = alloc.frame_box_for_transfer(data);
        alloc.end_frame();
        
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let received = handle.receive();
            tx.send(received).unwrap();
        });
        
        let received = rx.recv().unwrap();
        assert_eq!(received, vec![1, 2, 3, 4, 5]);
    }
    
    #[test]
    fn test_thread_local_allocators() {
        let alloc = SmartAlloc::new(Default::default());
        let alloc_clone = alloc.clone();
        
        let handle = thread::spawn(move || {
            alloc_clone.begin_frame();
            let data = alloc_clone.frame_vec::<u32>();
            data.push(42);
            alloc_clone.end_frame();
            data.len()
        });
        
        assert_eq!(handle.join().unwrap(), 1);
    }
}
