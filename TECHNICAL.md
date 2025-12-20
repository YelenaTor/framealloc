# `framealloc`

**Deterministic, frame-based memory allocation for Rust game engines.**

`framealloc` is an engine-shaped memory allocation crate designed for **predictable performance**, **explicit lifetimes**, and **out-of-the-box scaling from single-threaded to multi-threaded workloads**.

It is **not** a general-purpose replacement for Rust's global allocator.
It is a **purpose-built tool** for game engines, renderers, simulations, and real-time systems.

---

## Why `framealloc`?

Most game engine memory:

* Is short-lived
* Has a clear lifetime (per-frame, per-system, per-task)
* Is performance-sensitive
* Should *never* hit the system allocator in hot paths

Yet most Rust code still relies on:

* `Vec`, `Box`, and `Arc` everywhere
* Implicit heap allocations
* Allocators optimized for average-case workloads

`framealloc` makes **memory intent explicit** and **cheap**.

---

## Core Concepts

### 1. Frame-based allocation

The primary allocator is a **frame arena**:

* Fast bump allocation
* No per-allocation free
* Reset once per frame

```rust
use framealloc::{SmartAlloc, AllocConfig};

let alloc = SmartAlloc::new(AllocConfig::default());

alloc.begin_frame();

let tmp = alloc.frame_alloc::<ScratchData>();
let verts = alloc.frame_alloc::<[Vertex; 4096]>();

// All frame allocations are invalid after end_frame
alloc.end_frame();
```

This model:

* Eliminates fragmentation
* Guarantees O(1) allocation
* Matches how engines actually work

---

### 2. Thread-local fast paths

Every thread automatically gets:

* Its own frame arena
* Its own small-object pools
* Zero locks on hot paths

This is always enabled — even in single-threaded programs.

```text
Single-threaded: 1 TLS allocator
Multi-threaded:  N TLS allocators
Same API. Same behavior.
```

No mode switching. No configuration required.

---

### 3. Automatic single → multi-thread scaling

`framealloc` scales automatically when used across threads:

* **Single-threaded usage:**
  * No mutex contention
  * No atomic overhead in hot paths

* **Multi-threaded usage:**
  * Thread-local allocation remains lock-free
  * Shared state is only touched during refills

The user never toggles a "threaded mode".

```rust
use framealloc::SmartAlloc;
use std::sync::Arc;

let alloc = Arc::new(SmartAlloc::with_defaults());

std::thread::spawn({
    let alloc = alloc.clone();
    move || {
        alloc.begin_frame();
        let x = alloc.frame_alloc::<Foo>();
        alloc.end_frame();
    }
});
```

This works out of the box.

---

### 4. Allocation by intent

Allocations are categorized by **intent**, not just size:

| Intent | Method | Behavior |
|--------|--------|----------|
| `Frame` | `frame_alloc::<T>()` | Bump allocation, reset every frame |
| `Pool` | `pool_alloc::<T>()` | Thread-local pooled allocation |
| `Heap` | `heap_alloc::<T>()` | System allocator (large objects) |

This allows:

* Better locality
* Predictable behavior
* Meaningful diagnostics

---

### 5. Designed for game engines (and Bevy)

`framealloc` integrates cleanly with **Bevy**:

```rust
use bevy::prelude::*;
use framealloc::bevy::SmartAllocPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(SmartAllocPlugin::default())
        .run();
}
```

This automatically:

* Inserts the allocator as a Bevy resource
* Resets frame arenas at frame boundaries
* Works across Bevy's parallel systems

Inside a system:

```rust
fn my_system(alloc: Res<framealloc::bevy::AllocResource>) {
    let tmp = alloc.frame_alloc::<TempData>();
    // Use tmp...
}
```

No lifetimes exposed. No unsafe in user code. No boilerplate.

---

## What `framealloc` is **not**

To set expectations clearly:

❌ Not a global allocator replacement  
❌ Not a garbage collector  
❌ Not a drop-in replacement for `Vec`  
❌ Not optimized for long-lived arbitrary object graphs  

If you need:

* General heap allocation → use the standard allocator
* Reference-counted sharing → use `Arc`
* Data structures with unknown lifetimes → use `Vec` / `Box`

Use `framealloc` where **lifetime and performance are known**.

---

## Architecture Overview

```text
SmartAlloc (Arc)
 ├── GlobalState (shared)
 │    ├── SystemHeap (mutex-protected, large allocations)
 │    ├── SlabRegistry (mutex-protected page pools)
 │    ├── BudgetManager (optional limits)
 │    └── GlobalStats (atomics)
 │
 └── ThreadLocalState (TLS, per thread)
      ├── FrameArena (bump allocator, no sync)
      ├── LocalPools (small-object free lists, no sync)
      ├── DeferredFreeQueue (lock-free cross-thread frees)
      └── ThreadStats (local counters)
```

**Key rule:**

> Allocation always tries thread-local memory first.

Global synchronization only occurs during:

* Slab refills (rare, batched)
* Large allocations (>4KB default)
* Cross-thread frees (deferred, amortized)

---

## Performance Characteristics

| Operation | Cost | Synchronization |
|-----------|------|-----------------|
| Frame allocation | O(1) | None |
| Frame reset | O(1) | None |
| Pool allocation | O(1) | None (local hit) |
| Pool refill | O(1) amortized | Mutex (rare) |
| Pool free | O(1) | None |
| Cross-thread free | O(1) | Lock-free queue |
| Large alloc/free | O(1) | Mutex |

This design favors:

* Predictability over throughput
* Cache locality
* Stable frame times

---

## Safety Model

* All unsafe code is isolated inside `allocators/` module
* Public API is fully safe Rust
* Frame memory is invalidated explicitly at `end_frame()`
* Debug builds poison freed memory with `0xCD` pattern
* Optional allocation backtraces for leak detection

---

## Feature Flags

```toml
[features]
default = []

# Use parking_lot for faster mutexes
parking_lot = ["dep:parking_lot"]

# Bevy integration
bevy = ["dep:bevy_ecs", "dep:bevy_app"]

# Debug features: memory poisoning, allocation backtraces
debug = ["dep:backtrace"]
```

---

## Advanced Features

### Memory Budgets (Per-Tag Limits)

Track and limit memory usage by subsystem:

```rust
use framealloc::{SmartAlloc, AllocConfig, AllocationTag, BudgetManager};

let config = AllocConfig::default().with_budgets(true);
let alloc = SmartAlloc::new(config);

// Register budgets for subsystems
let rendering_tag = AllocationTag::new("rendering");
// Budget manager is accessed through global state
```

Budget events can trigger callbacks for monitoring:
- `SoftLimitExceeded` - Warning threshold crossed
- `HardLimitExceeded` - Allocation may fail
- `NewPeak` - High water mark updated

### Streaming Allocator

For large assets loaded incrementally (textures, meshes, audio):

```rust
use framealloc::{StreamingAllocator, StreamPriority};

let streaming = StreamingAllocator::new(64 * 1024 * 1024); // 64MB budget

// Reserve space for an asset
let id = streaming.reserve(1024 * 1024, StreamPriority::Normal).unwrap();

// Load data incrementally
let ptr = streaming.begin_load(id).unwrap();
// ... write data to ptr ...
streaming.report_progress(id, bytes_loaded);
streaming.finish_load(id);

// Access the data
let data = streaming.access(id).unwrap();

// Automatic eviction under memory pressure (LRU + priority)
```

### Diagnostics UI Hooks

For integration with imgui, egui, or custom debug UIs:

```rust
use framealloc::diagnostics::{DiagnosticsHooks, DiagnosticsEvent};

let mut hooks = DiagnosticsHooks::new();

// Register event listeners
hooks.add_listener(|event| {
    match event {
        DiagnosticsEvent::FrameBegin { frame_number } => { /* ... */ }
        DiagnosticsEvent::MemoryPressure { current, limit } => { /* ... */ }
        _ => {}
    }
});

// Get graph data for visualization
let graph_data = hooks.get_memory_graph_data(100);
```

Snapshot history provides time-series data for memory graphs.

### Handle-Based Allocation

Stable handles that survive memory relocation:

```rust
use framealloc::{HandleAllocator, Handle};

let allocator = HandleAllocator::new();

// Allocate and get a handle (not a raw pointer)
let handle: Handle<MyData> = allocator.alloc().unwrap();

// Resolve handle to pointer when needed
let ptr = allocator.resolve_mut(handle).unwrap();
unsafe { *ptr = MyData::new(); }

// Pin to prevent relocation during critical sections
allocator.pin(handle);
// ... use raw pointer safely ...
allocator.unpin(handle);

// Defragment memory (relocates unpinned allocations)
let relocations = allocator.defragment();

// Handle remains valid after relocation
let ptr = allocator.resolve(handle).unwrap();
```

### Allocation Groups

Free multiple allocations at once:

```rust
use framealloc::SmartAlloc;

let alloc = SmartAlloc::with_defaults();
let groups = alloc.groups();

// Create a group for level assets
let level_group = groups.create_group("level_1_assets");

// Allocate into the group
groups.alloc_val(level_group, texture_data);
groups.alloc_val(level_group, mesh_data);
groups.alloc_val(level_group, audio_data);

// Free everything at once when unloading level
groups.free_group(level_group);
```

### Safe Wrapper Types

RAII wrappers for automatic memory management:

```rust
use framealloc::SmartAlloc;

let alloc = SmartAlloc::with_defaults();

// FrameBox - valid until end_frame()
alloc.begin_frame();
let data = alloc.frame_box(MyStruct::new()).unwrap();
println!("{}", data.field); // Deref works
alloc.end_frame();

// PoolBox - auto-freed on drop
{
    let boxed = alloc.pool_box(123u64).unwrap();
    // Use boxed...
} // Freed here

// HeapBox - auto-freed on drop
{
    let large = alloc.heap_box([0u8; 8192]).unwrap();
    // Use large...
} // Freed here
```

### Profiler Integration

Hooks for Tracy, Optick, or custom profilers:

```rust
use framealloc::diagnostics::{ProfilerHooks, MemoryEvent};

let mut hooks = ProfilerHooks::new();

hooks.set_callback(|event| {
    match event {
        MemoryEvent::Alloc { ptr, size, tag } => {
            // Report to profiler
        }
        MemoryEvent::Free { ptr } => {
            // Report to profiler
        }
        _ => {}
    }
});
```

---

## Diagnostics System

`framealloc` provides **allocator-specific diagnostics** at build time, compile time, and runtime.
It does not replace Rust compiler warnings — it explains *engine-level* mistakes.

### Diagnostic Codes Reference

All diagnostics use codes for easy searching and documentation:

| Code | Category | Meaning |
|------|----------|---------|
| **FA001** | Frame | Frame allocation used outside active frame |
| **FA002** | Frame | Frame memory reference escaped scope |
| **FA003** | Frame | Frame arena exhausted |
| **FA101** | Bevy | SmartAllocPlugin not registered |
| **FA102** | Bevy | Frame hooks not executed this frame |
| **FA201** | Thread | Invalid cross-thread memory free |
| **FA202** | Thread | Thread-local state accessed before init |
| **FA301** | Budget | Global memory budget exceeded |
| **FA302** | Budget | Tag-specific budget exceeded |
| **FA401** | Handle | Invalid or freed handle accessed |
| **FA402** | Stream | Streaming allocator budget exhausted |
| **FA901** | Internal | Internal allocator error (report bug) |

### Runtime Diagnostics

Diagnostics are emitted to stderr in debug builds:

```rust
use framealloc::{fa_diagnostic, fa_emit, DiagnosticKind};

// Emit a custom diagnostic
fa_diagnostic!(
    Error,
    code = "FA001",
    message = "frame allocation used outside an active frame",
    note = "this allocation was requested after end_frame()",
    help = "call alloc.begin_frame() before allocating"
);

// Emit a predefined diagnostic
fa_emit!(FA001);

// Emit with context (captures thread, frame number, etc.)
fa_emit_ctx!(FA001);
```

**Sample output:**

```
[framealloc][FA001] error: frame allocation used outside an active frame
  note: this allocation was requested when no frame was active
  help: call alloc.begin_frame() before allocating, or use pool_alloc()/heap_alloc()
```

### Compile-Time Diagnostics

For errors detectable at compile time:

```rust
// Hard compiler error with formatted message
fa_compile_error!(
    code = "FA101",
    message = "Bevy support enabled but plugin not registered",
    help = "add .add_plugins(SmartAllocPlugin) to your App"
);
```

### Strict Mode (CI Integration)

Configure diagnostics to panic instead of warn — useful for CI:

```rust
use framealloc::{set_strict_mode, StrictMode, StrictModeGuard};

// Panic on any error diagnostic
set_strict_mode(StrictMode::PanicOnError);

// Or use a guard for scoped strict mode
{
    let _guard = StrictModeGuard::panic_on_error();
    // Errors in this scope will panic
}
// Back to normal behavior
```

**Environment variable:**

```bash
# In CI
FRAMEALLOC_STRICT=error cargo test

# Options: warn, error, warning (panics on warnings too)
```

### Conditional Assertions

Assert conditions with automatic diagnostics:

```rust
use framealloc::fa_assert;

// Emits FA001 if condition is false
fa_assert!(frame_active, FA001);

// With context capture
fa_assert!(frame_active, FA001, ctx);
```

### Diagnostic Context

Diagnostics can capture runtime context:

```rust
use framealloc::diagnostics::{DiagContext, set_bevy_context};

// Mark that we're in a Bevy app
set_bevy_context(true);

// Capture current context
let ctx = DiagContext::capture();
println!("Frame: {}, Thread: {:?}", ctx.frame_number, ctx.thread_id);
```

Context includes:
- Whether Bevy integration is active
- Current frame number
- Whether a frame is active
- Thread ID and name
- Whether this is the main thread

---

## Build-Time Diagnostics

The `build.rs` script provides helpful messages during compilation:

### Feature Detection

```
[framealloc] ℹ️  Bevy integration enabled
[framealloc]    Remember to add SmartAllocPlugin to your Bevy App:
[framealloc]      app.add_plugins(framealloc::bevy::SmartAllocPlugin::default())
```

### Debug Mode Hints

```
[framealloc] ℹ️  Debug features enabled
[framealloc]    Debug mode provides:
[framealloc]      • Memory poisoning (freed memory filled with 0xCD)
[framealloc]      • Allocation backtraces (for leak detection)
[framealloc]      • Extended validation checks
```

### Release Build Recommendations

```
[framealloc] ℹ️  Building in release mode
[framealloc]    Tip: Consider enabling 'parking_lot' for better mutex performance
```

### Quick Reference (printed during build)

```
[framealloc] ────────────────────────────────────────
[framealloc] ℹ️  framealloc Quick Reference
[framealloc] ────────────────────────────────────────
[framealloc]    Frame allocation (fastest, reset per frame):
[framealloc]      alloc.begin_frame();
[framealloc]      let data = alloc.frame_box(value);
[framealloc]      alloc.end_frame();
```

---

## Complete Feature Flags

```toml
[features]
default = []

# Use parking_lot for faster mutexes
parking_lot = ["dep:parking_lot"]

# Bevy integration
bevy = ["dep:bevy_ecs", "dep:bevy_app"]

# Debug features: memory poisoning, allocation backtraces
debug = ["dep:backtrace"]

# Tracy profiler integration
tracy = ["dep:tracy-client"]

# Nightly: std::alloc::Allocator trait implementations
nightly = []

# Enhanced runtime diagnostics
diagnostics = []
```

---

## std::alloc::Allocator Trait (Nightly)

With the `nightly` feature, use framealloc with standard collections:

```rust
#![feature(allocator_api)]

use framealloc::{SmartAlloc, FrameAllocator};

let alloc = SmartAlloc::with_defaults();
alloc.begin_frame();

// Use with Vec
let frame_alloc = FrameAllocator::new();
let mut vec: Vec<u32, _> = Vec::new_in(frame_alloc);
vec.push(42);

alloc.end_frame();
```

**Enable with:**

```toml
framealloc = { version = "0.1", features = ["nightly"] }
```

```bash
rustup override set nightly
```

---

## Common Patterns

### Pattern 1: Game Loop

```rust
let alloc = SmartAlloc::with_defaults();

loop {
    alloc.begin_frame();
    
    // All frame allocations here
    let scratch = alloc.frame_box(ScratchData::new())?;
    process(&scratch);
    
    alloc.end_frame(); // All frame memory freed
}
```

### Pattern 2: Level Loading

```rust
let alloc = SmartAlloc::with_defaults();
let groups = alloc.groups();

// Load level
let level_group = groups.create_group("level_1");
for asset in level_assets {
    groups.alloc_val(level_group, load_asset(asset)?);
}

// ... play level ...

// Unload - free everything at once
groups.free_group(level_group);
```

### Pattern 3: Streaming Assets

```rust
let streaming = alloc.streaming();

// Reserve space before loading
let texture_id = streaming.reserve(texture_size, StreamPriority::High)?;

// Load asynchronously
let ptr = streaming.begin_load(texture_id)?;
load_texture_async(ptr, texture_id, |progress| {
    streaming.report_progress(texture_id, progress);
});
streaming.finish_load(texture_id);

// Access when ready
if let Some(data) = streaming.access(texture_id) {
    use_texture(data);
}
```

### Pattern 4: Safe Wrappers

```rust
let alloc = SmartAlloc::with_defaults();

// Prefer safe wrappers over raw pointers
let pool_data = alloc.pool_box(MyData::new())?;  // Auto-freed on drop
let heap_data = alloc.heap_box(LargeData::new())?;  // Auto-freed on drop

alloc.begin_frame();
let frame_data = alloc.frame_box(TempData::new())?;  // Valid until end_frame
// Use frame_data...
alloc.end_frame();
```

---

## Troubleshooting

### "Frame allocation used outside an active frame" (FA001)

**Problem:** You called `frame_alloc()` without `begin_frame()`.

**Fix:**
```rust
alloc.begin_frame();  // Add this
let data = alloc.frame_alloc::<T>();
// ...
alloc.end_frame();
```

**Or use persistent allocation:**
```rust
let data = alloc.pool_box(value);  // Lives until dropped
```

### "Bevy plugin missing" (FA101)

**Problem:** Bevy feature enabled but plugin not added.

**Fix:**
```rust
App::new()
    .add_plugins(SmartAllocPlugin::default())  // Add this
    .run();
```

### "Frame arena exhausted" (FA003)

**Problem:** Too many frame allocations for the arena size.

**Fix:**
```rust
let config = AllocConfig::default()
    .with_frame_arena_size(64 * 1024 * 1024);  // Increase to 64MB
let alloc = SmartAlloc::new(config);
```

### Memory appears corrupted

**Debug with poisoning:**
```toml
framealloc = { version = "0.1", features = ["debug"] }
```

Freed memory is filled with `0xCD`. If you see this pattern, you're using freed memory.

### Finding memory leaks

**Enable backtraces:**
```toml
framealloc = { version = "0.1", features = ["debug"] }
```

```rust
// In debug builds, leaked allocations are tracked
let stats = alloc.stats();
println!("Active allocations: {}", stats.allocation_count - stats.deallocation_count);
```

---

## When should I use `framealloc`?

You should consider `framealloc` if you are building:

* A game engine
* A renderer
* A physics or simulation engine
* A real-time or embedded system
* A performance-critical tool with frame-like execution

---

## Philosophy

> **Make memory lifetime explicit.**  
> **Make the fast path obvious.**  
> **Make the slow path predictable.**

`framealloc` exists to give Rust game developers the same level of control engine developers expect — without sacrificing safety or ergonomics.

---

## License

Licensed under either of:

* Apache License, Version 2.0
* MIT license

at your option.
