# Tokio Integration Guide

**framealloc v0.8.0+**

This guide covers safe async/await patterns with framealloc when using Tokio or other async runtimes.

---

## The Problem

Frame allocations are **unsafe across await points**:

```rust
// ❌ DANGEROUS: Frame data may be invalidated during await
async fn bad_example(alloc: &SmartAlloc) {
    let data = alloc.frame_box(MyData::new()); // Frame allocation
    
    some_io().await; // ⚠️ Frame could be reset here!
    
    use_data(&data); // 💥 Use-after-free if end_frame() was called
}
```

Why this fails:
1. `frame_box()` allocates from the frame arena
2. During `.await`, another task (or main thread) may call `end_frame()`
3. Frame arena is reset, invalidating `data`
4. Resumed task uses freed memory

**cargo-fa catches this with FA701, FA702, FA703.**

---

## The Solution: Hybrid Model

The safe pattern is simple:

| Context | Use | Why |
|---------|-----|-----|
| Main thread / sync code | `frame_alloc`, `frame_box`, `frame_vec` | Deterministic, resets at frame boundary |
| Async tasks | `pool_alloc`, `pool_box`, `heap_box` | Persists beyond frames, safe across await |

```rust
// ✅ SAFE: Hybrid model
fn game_loop(alloc: SmartAlloc, rt: &Runtime) {
    loop {
        alloc.begin_frame();
        
        // Frame allocations for main thread work
        let physics_scratch = alloc.frame_vec::<Contact>();
        simulate_physics(&mut physics_scratch);
        
        // Spawn async I/O tasks with POOL allocations
        let alloc_clone = alloc.clone();
        rt.spawn(async move {
            let asset = alloc_clone.pool_box(load_asset().await);
            process_asset(&asset);
            // pool_box automatically freed when dropped
        });
        
        // More frame work
        render(&physics_scratch);
        
        alloc.end_frame(); // Frame memory reset, async tasks unaffected
    }
}
```

---

## Opt-In API (v0.8.0)

### Feature Flag

```toml
[dependencies]
framealloc = { version = "0.8", features = ["tokio"] }
```

### TaskAlloc — Task-Scoped Allocations

For allocations that should live exactly as long as a spawned task:

```rust
use framealloc::tokio::TaskAlloc;

tokio::spawn(async move {
    // Create task-local allocator (uses pool internally)
    let mut task = TaskAlloc::new(&alloc);
    
    // All allocations tracked and freed when task completes
    let buffer = task.alloc_box(vec![0u8; 4096]);
    let state = task.alloc_box(TaskState::new());
    
    process(&buffer, &state).await;
    
    // task drops here → all allocations freed back to pool
});
```

**Guarantees:**
- Uses pool/heap internally, never frame
- All allocations freed when `TaskAlloc` drops
- Safe across any number of await points

### AsyncPoolGuard — Scoped Pool Allocations

For async code that needs a batch of related allocations:

```rust
use framealloc::tokio::AsyncPoolGuard;

async fn process_batch(alloc: SmartAlloc) {
    let guard = AsyncPoolGuard::new(&alloc);
    
    // All allocations through guard are pool-backed
    let items: Vec<_> = futures::future::join_all(
        (0..10).map(|i| async {
            guard.alloc_box(fetch_item(i).await)
        })
    ).await;
    
    for item in &items {
        process(item).await;
    }
    
    // guard drops → batch freed
}
```

---

## Pattern Reference

### ✅ Safe Patterns

```rust
// 1. Clone allocator for async tasks
let alloc_clone = alloc.clone();
tokio::spawn(async move {
    let data = alloc_clone.pool_box(value); // ✓
    work(&data).await;
});

// 2. Pool allocations in async functions
async fn safe_async(alloc: &SmartAlloc) -> PoolBox<Data> {
    let result = alloc.pool_box(Data::new()); // ✓
    expensive_io().await;
    result
}

// 3. Heap for long-lived async data
async fn long_lived(alloc: &SmartAlloc) {
    let cache = alloc.heap_box(Cache::new()); // ✓
    loop {
        update(&cache).await;
    }
}

// 4. Frame allocations in sync sections only
fn sync_work(alloc: &SmartAlloc) {
    let scratch = alloc.frame_vec::<f32>(); // ✓ No await possible
    compute(&scratch);
} // scratch valid until end_frame()
```

### ❌ Unsafe Patterns (Caught by cargo-fa)

```rust
// FA701: Frame allocation in async function
async fn bad_1(alloc: &SmartAlloc) {
    let data = alloc.frame_box(X); // ❌ FA701
    work().await;
}

// FA702: Frame allocation crosses await
async fn bad_2(alloc: &SmartAlloc) {
    let data = alloc.frame_box(X); // Allocation here
    work().await;                   // ❌ FA702: crosses await
    use_data(&data);
}

// FA703: Frame data captured by spawned task
fn bad_3(alloc: &SmartAlloc) {
    let data = alloc.frame_box(X);
    tokio::spawn(async move {
        use_data(&data); // ❌ FA703: frame data in task
    });
}
```

---

## Integration Patterns

### Game Engine Main Loop

```rust
struct GameEngine {
    alloc: SmartAlloc,
    runtime: Runtime,
}

impl GameEngine {
    fn run(&self) {
        loop {
            self.alloc.begin_frame();
            
            // === SYNC PHASE: Frame allocations OK ===
            let input = self.alloc.frame_box(poll_input());
            let physics = self.alloc.frame_vec::<PhysicsResult>();
            
            self.update_physics(&input, &mut physics);
            self.render(&physics);
            
            // === ASYNC PHASE: Pool/Heap only ===
            self.runtime.block_on(async {
                // Asset loading (pool)
                let asset = self.alloc.pool_box(
                    load_asset("texture.png").await
                );
                self.asset_cache.insert(asset);
                
                // Network I/O (pool)
                let packet = self.alloc.pool_box(
                    receive_packet().await
                );
                self.process_packet(&packet);
            });
            
            self.alloc.end_frame();
        }
    }
}
```

### Bevy + Tokio

```rust
use bevy::prelude::*;
use framealloc::bevy::FrameallocPlugin;

fn async_system(
    alloc: Res<SmartAlloc>,
    runtime: Res<TokioRuntime>,
) {
    // Spawn async work with pool allocations
    let alloc = alloc.clone();
    runtime.spawn(async move {
        let data = alloc.pool_box(fetch_data().await);
        // Process data...
    });
}

fn sync_system(alloc: Res<SmartAlloc>) {
    // Frame allocations safe in sync systems
    let scratch = alloc.frame_vec::<Entity>();
    // ...
}
```

---

## Performance Considerations

| Allocation Type | Async Safety | Speed | Use Case |
|-----------------|--------------|-------|----------|
| `frame_*` | ❌ Unsafe | ⚡⚡⚡ Fastest | Sync-only scratch data |
| `pool_*` | ✅ Safe | ⚡⚡ Fast | Short-lived async data |
| `heap_*` | ✅ Safe | ⚡ Normal | Long-lived async data |
| `TaskAlloc` | ✅ Safe | ⚡⚡ Fast | Task-scoped batches |

**Recommendation:** Use `pool_*` for most async work. Reserve `heap_*` for data that outlives many frames.

---

## cargo-fa Configuration

Enable async safety checks in `cargo-fa.toml`:

```toml
[lints]
async-safety = "warn"  # or "deny" for strict mode

[lints.FA701]
level = "error"  # Frame allocation in async: always error

[lints.FA702]
level = "warn"   # Frame crosses await: warn (may have false positives)

[lints.FA703]
level = "error"  # Frame captured by task: always error
```

Run:
```bash
cargo fa --check async-safety
```

---

## FAQ

### Q: Can I use `frame_alloc` if I never await?

**A:** Yes, but be careful. If the function is `async`, cargo-fa will warn because the compiler doesn't guarantee no suspension points exist.

```rust
// Technically safe but warned:
async fn technically_ok(alloc: &SmartAlloc) {
    let data = alloc.frame_box(X); // FA701 warning
    sync_only_work(&data);         // No await, but...
}                                  // ...function is still async
```

### Q: What about `tokio::task::spawn_blocking`?

**A:** Same rules apply. The blocking task runs on a separate thread, so frame allocations from the main thread would be invalid.

```rust
// ❌ Bad
let data = alloc.frame_box(X);
tokio::task::spawn_blocking(move || {
    use_data(&data); // Frame data on wrong thread
});

// ✅ Good
let data = alloc.pool_box(X);
tokio::task::spawn_blocking(move || {
    use_data(&data); // Pool data is thread-safe
});
```

### Q: Can I mix async runtimes?

**A:** Yes. framealloc's async safety is runtime-agnostic. The same rules apply to `async-std`, `smol`, or any other runtime.

---

## Summary

1. **Frame allocations** → Main thread, sync code only
2. **Pool/Heap allocations** → Async tasks, spawned work
3. **TaskAlloc** → Convenient task-scoped cleanup
4. **cargo-fa** → Catches violations at compile time

The hybrid model gives you the best of both worlds: blazing-fast frame allocations for your game loop, and safe async I/O without memory corruption.
