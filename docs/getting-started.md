# Getting Started with framealloc

Welcome to framealloc! This guide will get you up and running in under 2 hours.

## What is framealloc?

framealloc is an intent-aware, thread-smart memory allocator designed for high-performance applications like game engines. It provides three allocation strategies:

1. **Frame Allocation** - Fastest, reset per frame
2. **Pool Allocation** - Small objects, auto-freed
3. **Heap Allocation** - Large objects, auto-freed

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
framealloc = "0.10"
```

For optional features:
```toml
framealloc = { version = "0.10", features = [
    "tokio",      # Async/await support
    "rapier",     # Physics engine integration
    "debug",      # Memory poisoning and tracking
    "minimal",    # Maximum performance
    "prefetch"    # Cache hints (x86_64)
] }
```

## Your First Allocation

Let's write a simple frame-based loop:

```rust
use framealloc::SmartAlloc;

fn main() {
    // Create the allocator
    let alloc = SmartAlloc::new(Default::default());
    
    // Game loop
    loop {
        // Begin a new frame
        alloc.begin_frame();
        
        // Allocate some temporary data
        let scratch = alloc.frame_vec::<f32>();
        for i in 0..1000 {
            scratch.push(i as f32);
        }
        
        // Use the data
        process_data(&scratch);
        
        // End frame - all frame allocations are freed!
        alloc.end_frame();
        
        // Continue loop...
    }
}

fn process_data(data: &[f32]) {
    println!("Processed {} values", data.len());
}
```

## Understanding Frame Allocation

Frame allocation is the core concept. Think of it as a scratchpad that's automatically cleared each frame:

```rust
alloc.begin_frame();

// These allocations are automatically freed at end_frame()
let buffer = alloc.frame_box([0u8; 1024]);
let vector = alloc.frame_vec::<String>();
let slice = alloc.frame_slice::<Entity>(100);

alloc.end_frame(); // Everything is gone!
```

**Key Benefits:**
- Extremely fast (single pointer bump)
- No need to manually free memory
- Perfect for temporary per-frame data

## Pool Allocation

For data that needs to persist beyond one frame but should still be managed:

```rust
// Pool allocation - small objects, automatically tracked
let entity = alloc.pool_box(Entity::new());
let handle = alloc.pool_arc(SharedData::default());

// Freed when no longer referenced or at allocator shutdown
```

## Heap Allocation

For large objects that don't fit in pools:

```rust
// Heap allocation - large objects, still tracked
let texture = alloc.heap_box(TextureData::load("image.png"));
let mesh = alloc.heap_vec::<Vertex>(vertex_count);

// Still automatically freed when dropped
```

## Tags and Organization

Organize allocations by subsystem:

```rust
alloc.with_tag("physics", |a| {
    let contacts = a.frame_vec::<Contact>();
    let forces = a.frame_vec::<Vector3>();
    // ...
});

alloc.with_tag("rendering", |a| {
    let commands = a.frame_vec::<RenderCommand>();
    // ...
});
```

## Basic Threading

framealloc is thread-aware. Each thread gets its own frame allocator:

```rust
use std::thread;

let alloc = SmartAlloc::new(Default::default());
let alloc_clone = alloc.clone();

thread::spawn(move || {
    alloc_clone.begin_frame();
    let local_data = alloc_clone.frame_vec::<u32>();
    // Work on this thread...
    alloc_clone.end_frame();
});
```

## Common Pitfalls

### 1. Don't store frame allocations across frames
```rust
// ❌ WRONG - frame data dies at end_frame()
let cached_data: Box<[u8]> = alloc.frame_box([0; 1024]);

// ✅ CORRECT - use pool for persistence
let cached_data = alloc.pool_box([0; 1024]);
```

### 2. Don't send frame allocations to other threads
```rust
// ❌ WRONG - frame allocations aren't Send
let data = alloc.frame_vec::<u32>();
channel.send(data);

// ✅ CORRECT - use TransferHandle
let handle = alloc.frame_box_for_transfer(data);
channel.send(handle);
```

### 3. Always call begin_frame/end_frame in pairs
```rust
// ❌ WRONG - mismatched frames
alloc.begin_frame();
alloc.begin_frame(); // Forgot to end previous frame!

// ✅ CORRECT - proper nesting
alloc.begin_frame();
// ... work ...
alloc.end_frame();
```

## Next Steps

Now that you understand the basics:

1. Try the [Examples](../examples/) - Run `cargo run --example 01_hello_framealloc`
2. Read [Patterns Guide](patterns.md) - Learn common usage patterns
3. Check [Performance Guide](performance.md) - Optimize your allocations
4. Explore [Domain Guides](rapier-integration.md) - Physics, async, etc.

## Quick Reference

```rust
// Frame allocation (temporary)
alloc.begin_frame();
let data = alloc.frame_alloc::<T>();
let vec = alloc.frame_vec::<T>();
let slice = alloc.frame_slice::<T>(n);
alloc.end_frame();

// Pool allocation (persistent, small)
let boxed = alloc.pool_box(value);
let arc = alloc.pool_arc(value);

// Heap allocation (persistent, large)
let boxed = alloc.heap_box(value);
let vec = alloc.heap_vec::<T>();

// Organization
alloc.with_tag("system", |a| { /* allocations */ });
alloc.set_thread_frame_budget(bytes);
```

## Troubleshooting

If you encounter issues:

1. Check the [Troubleshooting Guide](troubleshooting.md)
2. Enable debug mode: `framealloc = { version = "0.10", features = ["debug"] }`
3. Run static analysis: `cargo install cargo-fa && cargo fa`

Happy allocating! 🚀
