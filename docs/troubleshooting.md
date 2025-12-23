# framealloc Troubleshooting Guide

Common issues and their solutions.

## Table of Contents

1. [Compilation Errors](#compilation-errors)
2. [Runtime Issues](#runtime-issues)
3. [Performance Problems](#performance-problems)
4. [Memory Issues](#memory-issues)
5. [Threading Issues](#threading-issues)
6. [Debugging Tools](#debugging-tools)

## Compilation Errors

### "Cannot find type `SmartAlloc`"

```rust
// Error
use framealloc::SmartAlloc; // Error: cannot find type

// Solution
use framealloc::{SmartAlloc, AllocConfig}; // Import both
```

### "FrameBox doesn't implement Send"

```rust
// Error
fn send_frame_data() {
    let data = alloc.frame_box(42);
    std::thread::spawn(move || {
        println!("{}", data); // Error: FrameBox isn't Send
    });
}

// Solution - use TransferHandle
fn send_frame_data() {
    let data = alloc.frame_box(42);
    let handle = alloc.frame_box_for_transfer(data);
    std::thread::spawn(move || {
        let data = handle.receive();
        println!("{}", data);
    });
}
```

### "Cannot return frame allocation"

```rust
// Error
fn get_data() -> FrameBox<i32> {
    alloc.begin_frame();
    let data = alloc.frame_box(42);
    alloc.end_frame();
    data // Error: data dies at end_frame
}

// Solution - use pool or frame retention
fn get_data_pooled() -> PoolBox<i32> {
    alloc.pool_box(42)
}

fn get_data_retained(alloc: &SmartAlloc) -> FrameRetained<i32> {
    alloc.begin_frame();
    let data = alloc.frame_retained(42, RetentionPolicy::PromoteToPool);
    alloc.end_frame();
    data
}
```

## Runtime Issues

### Use After Frame End

```rust
// Problem
fn use_after_frame() {
    alloc.begin_frame();
    let data = alloc.frame_vec::<i32>();
    alloc.end_frame();
    
    data.push(42); // Crash or undefined behavior!
}

// Solution - keep within frame
fn correct_usage() {
    alloc.begin_frame();
    let data = alloc.frame_vec::<i32>();
    data.push(42);
    // Use data here
    alloc.end_frame();
}
```

### Double begin_frame

```rust
// Problem
fn double_begin() {
    alloc.begin_frame();
    alloc.begin_frame(); // May cause issues
    // ...
    alloc.end_frame();
    alloc.end_frame();
}

// Solution - track frame state
fn safe_frames() {
    if !alloc.in_frame() {
        alloc.begin_frame();
    }
    // ...
    alloc.end_frame();
}
```

### Pool Exhaustion

```rust
// Problem - pool runs out
fn exhaust_pool() {
    for _ in 0..1000000 {
        let _data = alloc.pool_alloc::<LargeObject>();
        // Never freed!
    }
}

// Solution - let objects drop
fn correct_pool_usage() {
    {
        let data = alloc.pool_alloc::<LargeObject>();
        // Use data
    } // data automatically returned to pool
}
```

## Performance Problems

### Slow Frame Allocation

```rust
// Problem - too many small allocations
fn slow_allocation() {
    for i in 0..10000 {
        let _data = alloc.frame_alloc::<f32>(); // 10000 separate allocations
    }
}

// Solution - use batch allocation
fn fast_allocation() {
    let batch = unsafe { alloc.frame_alloc_batch::<f32>(10000) };
    for i in 0..10000 {
        let _data = unsafe { batch.add(i) };
    }
}
```

### Excessive Tag Overhead

```rust
// Problem - tag on every tiny allocation
fn tag_heavy() {
    for i in 0..1000 {
        alloc.with_tag("item", |a| a.frame_alloc::<u8>());
    }
}

// Solution - group allocations
fn tag_efficient() {
    alloc.with_tag("items", |a| {
        for i in 0..1000 {
            a.frame_alloc::<u8>();
        }
    });
}
```

### Fragmentation

```rust
// Problem - many different sizes
fn fragmenting_allocations() {
    let sizes = [1, 3, 7, 15, 31, 63, 127, 255, 511, 1023];
    for &size in &sizes {
        let _data = alloc.pool_alloc::<[u8; 1024]>();
        // Wastes most of the allocation
    }
}

// Solution - use appropriate sizes
fn efficient_allocations() {
    // Use size-appropriate pools
    let small = alloc.pool_alloc::<u8>();
    let medium = alloc.pool_alloc::<[u8; 64]>();
    let large = alloc.pool_alloc::<[u8; 1024]>();
}
```

## Memory Issues

### Memory Leaks

```rust
// Problem - frame data promoted but never freed
fn memory_leak() {
    alloc.begin_frame();
    let data = alloc.frame_retained(large_data(), RetentionPolicy::PromoteToPool);
    alloc.end_frame();
    // data is now in pool but never referenced again
}

// Solution - track promotions
fn track_promotions() {
    alloc.begin_frame();
    let data = alloc.frame_retained(large_data(), RetentionPolicy::PromoteToPool);
    alloc.end_frame();
    
    // Use the data or explicitly drop
    drop(data);
}
```

### Out of Memory

```rust
// Problem - unbounded allocation
fn unbounded() {
    loop {
        let data = alloc.frame_vec::<u8>();
        data.resize(data.len() + 1000000, 0);
        // Grows without bound!
    }
}

// Solution - set budgets
fn bounded() {
    alloc.set_frame_budget(megabytes(100));
    
    loop {
        if alloc.frame_usage() < megabytes(90) {
            let data = alloc.frame_vec::<u8>();
            data.resize(1000000, 0);
        }
    }
}
```

### High Memory Usage

```rust
// Problem - keeping too much data
fn high_usage() {
    alloc.begin_frame();
    let all_data = alloc.frame_vec::<HugeStruct>();
    for _ in 0..100000 {
        all_data.push(HugeStruct::new());
    }
    // Uses lots of memory
    alloc.end_frame();
}

// Solution - process in chunks
fn chunked_processing() {
    const CHUNK_SIZE: usize = 1000;
    
    for chunk in 0..100 {
        alloc.begin_frame();
        let data = alloc.frame_vec::<HugeStruct>();
        for i in 0..CHUNK_SIZE {
            data.push(HugeStruct::new());
        }
        process_chunk(&data);
        alloc.end_frame();
    }
}
```

## Threading Issues

### Data Race

```rust
// Problem - sharing frame data across threads
fn data_race() {
    alloc.begin_frame();
    let data = alloc.frame_vec::<i32>();
    
    let handle = std::thread::spawn(move || {
        data.push(42); // Data race!
    });
    
    handle.join().unwrap();
    alloc.end_frame();
}

// Solution - use thread-local allocators
fn thread_safe() {
    let alloc_clone = alloc.clone();
    
    std::thread::spawn(move || {
        alloc_clone.begin_frame();
        let data = alloc_clone.frame_vec::<i32>();
        data.push(42);
        alloc_clone.end_frame();
    }).join().unwrap();
}
```

### Deadlock with TransferHandle

```rust
// Problem - deadlock waiting for transfer
fn deadlock() {
    let (tx, rx) = std::sync::mpsc::channel();
    
    std::thread::spawn(move || {
        let handle = rx.recv().unwrap(); // Waits forever
        let data = handle.receive();
    });
    
    // Never send anything
}

// Solution - timeout or proper synchronization
fn no_deadlock() {
    let (tx, rx) = std::sync::mpsc::channel();
    
    let handle = std::thread::spawn(move || {
        if let Ok(handle) = rx.recv() {
            let data = handle.receive();
            // Process data
        }
    });
    
    // Send the data
    let data = alloc.frame_box(42);
    let transfer = alloc.frame_box_for_transfer(data);
    tx.send(transfer).unwrap();
    
    handle.join().unwrap();
}
```

## Debugging Tools

### Enable Debug Mode

```toml
# Cargo.toml
framealloc = { version = "0.10", features = ["debug"] }
```

### Behavior Filter

```rust
// Enable runtime detection
alloc.enable_behavior_filter();

// Run your code
run_application();

// Check for issues
let report = alloc.behavior_report();
for issue in &report.issues {
    match issue.code {
        "FA601" => println!("Frame allocation in async function at {:?}", issue.location),
        "FA602" => println!("Allocation in hot loop at {:?}", issue.location),
        "FA603" => println!("Frame data crossing await point at {:?}", issue.location),
        _ => println!("Issue {}: {}", issue.code, issue.message),
    }
}
```

### Memory Poisoning

```rust
#[cfg(feature = "debug")]
fn debug_allocations() {
    alloc.begin_frame();
    
    let data = alloc.frame_alloc::<u32>();
    // Memory is poisoned when allocated
    
    unsafe { *data = 42; } // OK
    // data is automatically checked for corruption at end_frame
    
    alloc.end_frame();
    // Any corruption will be detected
}
```

### Statistics and Metrics

```rust
// Enable statistics
alloc.enable_statistics();

// After running
let stats = alloc.statistics();
println!("Total allocations: {}", stats.total_allocations);
println!("Peak memory: {} bytes", stats.peak_memory);
println!("Frame allocations: {}", stats.frame_allocations);
println!("Pool allocations: {}", stats.pool_allocations);
```

### cargo-fa Static Analysis

```bash
# Install cargo-fa
cargo install cargo-fa

# Check for common issues
cargo fa --all

# Specific checks
cargo fa --dirtymem    # Frame escape issues
cargo fa --threading   # Thread safety
cargo fa --async-safety # Async issues
cargo fa --architecture # Architecture violations

# Explain specific errors
cargo fa explain FA601
```

## Common Error Messages

### "Frame allocation escaped frame boundary"

**Cause**: Storing frame allocation in a place that outlives the frame.

```rust
// Bad
static mut GLOBAL_DATA: Option<FrameBox<i32>> = None;

fn store_frame_data() {
    alloc.begin_frame();
    unsafe { GLOBAL_DATA = Some(alloc.frame_box(42)); } // Error
    alloc.end_frame();
}

// Good
fn store_frame_data() {
    alloc.begin_frame();
    let data = alloc.frame_box(42);
    use_data(&data);
    alloc.end_frame();
}
```

### "Attempted to use frame data after end_frame"

**Cause**: Using frame allocation after the frame has ended.

```rust
// Bad
fn late_usage() -> &'static i32 {
    alloc.begin_frame();
    let data = alloc.frame_alloc::<i32>();
    alloc.end_frame();
    unsafe { &*data } // Error
}

// Good
fn timely_usage() {
    alloc.begin_frame();
    let data = alloc.frame_alloc::<i32>();
    // Use data here
    unsafe { *data = 42; }
    alloc.end_frame();
}
```

### "Pool exhausted"

**Cause**: Pool ran out of objects for a size class.

```rust
// Bad - never returning to pool
fn exhaust_pool() {
    let mut handles = Vec::new();
    for _ in 0..10000 {
        handles.push(alloc.pool_box::<MyObject>());
    }
    // Pool is empty
}

// Good - let objects drop
fn use_pool_correctly() {
    for _ in 0..10000 {
        let obj = alloc.pool_box::<MyObject>();
        // Use object
        // Automatically returned to pool
    }
}
```

## Performance Debugging

### Profile Allocation Patterns

```rust
use std::time::Instant;

fn profile_allocations() {
    let mut times = Vec::new();
    
    for _ in 0..1000 {
        let start = Instant::now();
        alloc.begin_frame();
        
        // Your allocation pattern
        let data = alloc.frame_vec::<u32>();
        for i in 0..1000 {
            data.push(i);
        }
        
        alloc.end_frame();
        times.push(start.elapsed());
    }
    
    let avg = times.iter().sum::<Duration>() / times.len() as u32;
    println!("Average frame time: {:?}", avg);
}
```

### Memory Usage Analysis

```rust
fn analyze_memory() {
    alloc.begin_frame();
    
    // Track allocations by size
    let mut small_count = 0;
    let mut medium_count = 0;
    let mut large_count = 0;
    
    // Simulate allocations
    for i in 0..1000 {
        if i < 500 {
            let _ = alloc.frame_alloc::<u8>();
            small_count += 1;
        } else if i < 800 {
            let _ = alloc.frame_alloc::<[u8; 64]>();
            medium_count += 1;
        } else {
            let _ = alloc.frame_alloc::<[u8; 1024]>();
            large_count += 1;
        }
    }
    
    let stats = alloc.frame_stats();
    println!("Small: {}, Medium: {}, Large: {}", small_count, medium_count, large_count);
    println!("Total bytes: {}", stats.bytes_allocated);
    
    alloc.end_frame();
}
```

## Getting Help

### Check the Documentation

- [Getting Started](getting-started.md) - Basic concepts
- [Patterns Guide](patterns.md) - Common patterns
- [Performance Guide](performance.md) - Optimization

### Use Debug Features

```rust
// Comprehensive debugging
#[cfg(debug_assertions)]
{
    alloc.enable_behavior_filter();
    alloc.enable_statistics();
    alloc.enable_memory_poisoning();
}
```

### Report Issues

When reporting issues, include:

1. framealloc version
2. Rust version
3. Platform (OS, architecture)
4. Minimal reproducible example
5. Error message or backtrace
6. Debug output (if available)

### Example Issue Report

```
framealloc version: 0.10.0
Rust version: 1.70.0
Platform: Windows 11, x86_64

Issue: Frame allocation crashes when used in async function

Code:
async fn problem() {
    let alloc = SmartAlloc::new(Default::default());
    alloc.begin_frame();
    let data = alloc.frame_vec::<u8>();
    await something(); // Crashes here
    alloc.end_frame();
}

Error: "Attempted to use frame data after end_frame"
```

## Quick Fixes

| Problem | Quick Fix |
|---------|-----------|
| Frame data escapes | Use `pool_box()` instead |
| Cross-thread issues | Use `TransferHandle` |
| Slow allocations | Use `frame_alloc_batch()` |
| Memory leaks | Enable debug mode |
| Compilation errors | Check imports |
| Performance issues | Run `cargo fa` |

## Prevention Checklist

- [ ] Always match `begin_frame` with `end_frame`
- [ ] Don't store frame allocations across frames
- [ ] Use `TransferHandle` for cross-thread data
- [ ] Enable debug features during development
- [ ] Run `cargo fa` regularly
- [ ] Set memory budgets
- [ ] Profile before optimizing
- [ ] Test with realistic data sizes

Remember: Most issues are caught by `cargo fa`! Use it early and often. 🚀
