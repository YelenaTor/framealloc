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

## Version History

### v0.7.0: IDE Integration & Snapshot Emission

**Release Date:** 2025-12-22

v0.7.0 extends framealloc's observability to external tooling while maintaining core principles.

#### Snapshot Emission

Runtime snapshot support for IDE integration:

```rust
use framealloc::{SnapshotConfig, SnapshotEmitter, Snapshot};

// Configure snapshots
let config = SnapshotConfig::default()
    .with_directory("target/framealloc")
    .with_max_snapshots(30);

let emitter = SnapshotEmitter::new(config);

// In your frame loop
alloc.end_frame();
let snapshot = Snapshot::new(frame_number);
// ... populate snapshot from allocator state ...
emitter.maybe_emit(&snapshot); // Checks for request file
```

**Snapshot Schema v1:**

```json
{
  "version": 1,
  "frame": 18421,
  "summary": { "frame_bytes": 4194304, "pool_bytes": 2097152, ... },
  "threads": [...],
  "tags": [...],
  "promotions": { "to_pool": 12, "to_heap": 3, "failed": 1 },
  "transfers": { "pending": 2, "completed_this_frame": 5 },
  "deferred": { "queue_depth": 128, "processed_this_frame": 64 },
  "diagnostics": [...]
}
```

**Characteristics:**
- **Opt-in** — Only emitted when explicitly enabled
- **Aggregated** — No per-allocation data, only summaries
- **Bounded** — Rate-limited (min 500ms) and cleaned up automatically
- **Safe boundary** — Only captured at frame end

#### cargo-fa: JSON Report Format

New `generate_json_report()` function produces structured output for IDE integration:

```rust
// In cargo-fa
let report = generate_json_report(&diagnostics, files_analyzed, duration_ms);
println!("{}", report);
```

Output format includes diagnostics array, summary statistics, and analysis metadata.

#### fa-insight VS Code Extension

Companion IDE extension for framealloc-aware development:
- Inline diagnostics from `cargo fa`
- Memory inspector sidebar panel
- Real-time snapshot visualization
- Tag hierarchy and budget tracking
- Repository: https://github.com/YelenaTor/fa-insight

**Philosophy:** v0.7.0 maintains framealloc's core principles — opt-in, explicit, zero-cost when disabled, deterministic.

---

### v0.6.0: Explicit Thread Coordination & Frame Observability

**Release Date:** 2025-12-21

See detailed section below.

---

### v0.5.1: Unified Versioning & cargo-fa Enhancements

See detailed section below.

---

### v0.5.0: Static Analysis with cargo-fa

See detailed section below.

---

### v0.4.0: Memory Behavior Filter

See detailed section below.

---

### v0.3.0: Frame Retention & Promotion

See detailed section below.

---

### v0.2.0: Frame Phases & Tagged Allocations

See detailed section below.

---

## v0.2.0 Features

### Frame Phases

Divide frames into named phases for profiling and diagnostics:

```rust
alloc.begin_frame();

alloc.begin_phase("physics");
let contacts = alloc.frame_alloc::<[Contact; 256]>();
// All allocations tracked under "physics"
alloc.end_phase();

alloc.begin_phase("render");
let verts = alloc.frame_alloc::<[Vertex; 4096]>();
alloc.end_phase();

alloc.end_frame();
```

**Features:**
- Zero overhead when not using phases
- Nested phase support
- Per-phase allocation statistics
- Integrates with diagnostics and profiler hooks

**Use with RAII guard:**
```rust
{
    let _phase = alloc.phase_scope("ai");
    // allocations here are in "ai" phase
} // phase ends automatically
```

### Frame Checkpoints

Save and rollback speculative allocations:

```rust
alloc.begin_frame();

let checkpoint = alloc.frame_checkpoint();

// Speculative allocations
let result = try_complex_operation();

if result.is_err() {
    alloc.rollback_to(checkpoint); // Undo all allocations
}

alloc.end_frame();
```

**Speculative blocks:**
```rust
let result = alloc.speculative(|| {
    let data = alloc.frame_alloc::<LargeData>();
    if validate(data).is_err() {
        return Err("validation failed");
    }
    Ok(process(data))
});
// Automatically rolled back on Err
```

**Use cases:**
- Pathfinding with dead-end rollback
- Physics with speculative contacts
- UI layout with try/fail patterns

### Frame Collections

Bounded, frame-local collections:

```rust
// FrameVec - fixed capacity vector
let mut entities = alloc.frame_vec::<Entity>(128);
entities.push(entity1);
entities.push(entity2);

for entity in entities.iter() {
    process(entity);
}
// Freed at end_frame()
```

```rust
// FrameMap - fixed capacity hash map
let mut lookup = alloc.frame_map::<EntityId, Transform>(64);
lookup.insert(id, transform);

if let Some(t) = lookup.get(&id) {
    use_transform(t);
}
```

**Properties:**
- Cannot grow beyond initial capacity
- Cannot escape the frame (lifetime-bound)
- Full iterator support
- Familiar API (push, pop, get, iter)

### Tagged Allocations

First-class allocation attribution:

```rust
alloc.with_tag("ai", |alloc| {
    let scratch = alloc.frame_alloc::<AIScratch>();
    // Allocation attributed to "ai" tag
    
    alloc.with_tag("pathfinding", |alloc| {
        let nodes = alloc.frame_alloc::<[Node; 256]>();
        // Nested: attributed to "ai::pathfinding"
    });
});

// Check current tag
println!("Tag: {:?}", alloc.current_tag());
println!("Path: {}", alloc.tag_path()); // "ai::pathfinding"
```

**Benefits:**
- Automatic budget attribution
- Better profiling granularity
- Clearer diagnostics

### Scratch Pools

Cross-frame reusable memory:

```rust
// Get or create a named pool
let pool = alloc.scratch_pool("pathfinding");

// Allocate (persists across frames)
let nodes = pool.alloc::<[PathNode; 1024]>();

// Use across multiple frames...

// Reset when done (e.g., level unload)
pool.reset();
```

**Registry access:**
```rust
let scratch = alloc.scratch();

// Get stats for all pools
for stat in scratch.stats() {
    println!("{}: {} / {} bytes", stat.name, stat.allocated, stat.capacity);
}

// Reset all pools
scratch.reset_all();
```

**Use cases:**
- Pathfinding node storage
- Level-specific allocations
- Subsystem scratch memory that outlives frames

---

## v0.3.0: Frame Retention & Promotion (Detailed)

### Overview

Frame allocations normally vanish at `end_frame()`. The retention system lets allocations optionally "escape" to other allocators.

**Key principle:** This is NOT garbage collection. It's explicit, deterministic post-frame ownership transfer.

### Retention Policies

```rust
pub enum RetentionPolicy {
    Discard,                      // Default - freed at frame end
    PromoteToPool,                // Copy to pool allocator
    PromoteToHeap,                // Copy to heap allocator  
    PromoteToScratch(&'static str), // Copy to named scratch pool
}
```

### Basic Usage

```rust
// Allocate with retention policy
let mut data = alloc.frame_retained::<NavMesh>(RetentionPolicy::PromoteToPool);
data.calculate_paths();

// At frame end, get promoted allocations
let result = alloc.end_frame_with_promotions();

for item in result.promoted {
    match item {
        PromotedAllocation::Pool { ptr, size, .. } => {
            // Data now lives in pool allocator
        }
        PromotedAllocation::Heap { ptr, size, .. } => {
            // Data now lives on heap
        }
        PromotedAllocation::Failed { reason, meta } => {
            // Promotion failed (budget exceeded, etc.)
            eprintln!("Failed to promote {}: {}", meta.type_name, reason);
        }
        _ => {}
    }
}
```

### Importance Levels (Semantic Sugar)

For more intuitive usage:

```rust
pub enum Importance {
    Ephemeral,   // → Discard
    Reusable,    // → PromoteToPool
    Persistent,  // → PromoteToHeap
    Scratch(n),  // → PromoteToScratch(n)
}

// Usage
let path = alloc.frame_with_importance::<Path>(Importance::Reusable);
let config = alloc.frame_with_importance::<Config>(Importance::Persistent);
```

### Frame Summary Diagnostics

Get detailed statistics about what happened at frame end:

```rust
let result = alloc.end_frame_with_promotions();
let summary = result.summary;

println!("Frame Summary:");
println!("  Discarded: {} bytes ({} allocs)", 
    summary.discarded_bytes, summary.discarded_count);
println!("  Promoted to pool: {} bytes ({} allocs)", 
    summary.promoted_pool_bytes, summary.promoted_pool_count);
println!("  Promoted to heap: {} bytes ({} allocs)", 
    summary.promoted_heap_bytes, summary.promoted_heap_count);
println!("  Failed: {} allocs", summary.failed_count);

// Breakdown by failure reason
let failures = &summary.failures_by_reason;
if failures.budget_exceeded > 0 {
    println!("    Budget exceeded: {}", failures.budget_exceeded);
}
if failures.scratch_pool_full > 0 {
    println!("    Scratch pool full: {}", failures.scratch_pool_full);
}
```

### Integration with Tags

Retained allocations preserve their tag attribution:

```rust
alloc.with_tag("ai", |alloc| {
    let data = alloc.frame_retained::<AIState>(RetentionPolicy::PromoteToPool);
    // When promoted, data retains "ai" tag for budgeting
});
```

### Design Principles

| Principle | Implementation |
|-----------|----------------|
| **Explicit** | Must opt-in per allocation |
| **Deterministic** | All decisions at `end_frame()` |
| **Bounded** | Subject to budgets and limits |
| **No Magic** | No heuristics or auto-promotion |

### When to Use Retention

| Scenario | Recommendation |
|----------|----------------|
| Pathfinding result might be reused | `Reusable` / `PromoteToPool` |
| Computed data proved useful | `Reusable` / `PromoteToPool` |
| Config loaded during frame | `Persistent` / `PromoteToHeap` |
| Subsystem scratch that persists | `Scratch("name")` |
| Truly temporary data | `Ephemeral` / `Discard` (default) |

### API Reference

```rust
// Allocate with retention
fn frame_retained<T>(&self, policy: RetentionPolicy) -> FrameRetained<'_, T>

// Allocate with importance (sugar)
fn frame_with_importance<T>(&self, importance: Importance) -> FrameRetained<'_, T>

// End frame and process promotions
fn end_frame_with_promotions(&self) -> PromotionResult

// End frame and get summary only
fn end_frame_with_summary(&self) -> FrameSummary

// Get pending retained count
fn retained_count(&self) -> usize

// Clear retained without processing
fn clear_retained(&self)
```

---

## v0.6.0: Explicit Thread Coordination & Frame Observability (Detailed)

### Design Philosophy

v0.6.0 makes the existing cross-thread behavior **explicit** and **controllable** while staying true to framealloc's core principles:

| Principle | How v0.6.0 Honors It |
|-----------|---------------------|
| **Deterministic** | Bounded queues, explicit barriers, predictable costs |
| **Frame-based** | All features center on frame lifecycle |
| **Explicit** | TransferHandle, budget policies, manual processing |
| **Predictable** | No hidden costs, configurable overhead |
| **Scales ST→MT** | Zero cost when features unused |

### TransferHandle: Explicit Cross-Thread Transfers

Previously, cross-thread allocations were handled silently via the deferred free queue. v0.6.0 adds explicit declaration of transfer intent:

```rust
use framealloc::{SmartAlloc, TransferHandle};

// Allocate with declared transfer intent
let handle: TransferHandle<PhysicsResult> = alloc.frame_box_for_transfer(result);

// Send to worker thread - transfer is explicit
worker_channel.send(handle);

// On worker thread: explicitly accept ownership
let data = handle.receive();
```

**Key properties:**
- Transfer intent is visible in the type system
- Ownership transfer is explicit, not implicit
- Dropped handles without receiving trigger warnings (debug mode)

### FrameBarrier: Deterministic Multi-Thread Sync

Coordinate frame boundaries across threads without races:

```rust
use framealloc::FrameBarrier;
use std::sync::Arc;

// Create barrier for main + 2 workers
let barrier = FrameBarrier::new(3);

// Each thread signals when frame work complete
barrier.signal_frame_complete();

// Coordinator waits for all, then resets
barrier.wait_all();
alloc.end_frame();
barrier.reset();
```

**Builder pattern:**
```rust
let barrier = FrameBarrierBuilder::new()
    .with_thread("main")
    .with_thread("physics")
    .with_thread("rendering")
    .build();
```

### Per-Thread Frame Budgets

Explicit per-thread memory limits with deterministic exceeded behavior:

```rust
use framealloc::{ThreadBudgetConfig, BudgetExceededPolicy};

// Configure per-thread limits
let config = ThreadBudgetConfig {
    frame_budget: 8 * 1024 * 1024,  // 8 MB
    frame_exceeded_policy: BudgetExceededPolicy::Fail,
    warning_threshold: 80,  // Warn at 80%
    ..Default::default()
};

alloc.set_thread_budget_config(thread_id, config);

// Check before large allocation
if alloc.frame_remaining() < large_size {
    // Handle gracefully
}
```

**Policies:**
| Policy | Behavior |
|--------|----------|
| `Fail` | Return null/error |
| `Warn` | Log warning, allow |
| `Allow` | Silent allow |
| `Promote` | Attempt promotion to larger allocator |

### Deferred Processing Control

Control when and how cross-thread frees are processed:

```rust
use framealloc::{DeferredConfig, DeferredProcessing, QueueFullPolicy};

// Bounded queue with explicit capacity
let config = DeferredConfig::bounded(1024)
    .full_policy(QueueFullPolicy::ProcessImmediately);

// Incremental processing (amortized cost per alloc)
let config = DeferredConfig::incremental(16);

// Full manual control
alloc.set_deferred_processing(DeferredProcessing::Explicit);
alloc.process_deferred_frees(64);  // Process up to 64
```

**Queue policies:**
| Policy | Behavior |
|--------|----------|
| `ProcessImmediately` | Block and drain |
| `DropOldest` | Lossy but non-blocking |
| `Fail` | Caller handles |
| `Grow` | Unbounded (legacy) |

### Frame Lifecycle Events

Opt-in observability with zero overhead when disabled:

```rust
use framealloc::{FrameEvent, LifecycleManager};

alloc.enable_lifecycle_tracking();

alloc.on_frame_event(|event| match event {
    FrameEvent::FrameBegin { thread_id, frame_number, .. } => {
        println!("Frame {} started on {:?}", frame_number, thread_id);
    }
    FrameEvent::CrossThreadFreeQueued { from, to, size } => {
        println!("Cross-thread: {:?} -> {:?}, {} bytes", from, to, size);
    }
    FrameEvent::FrameEnd { duration_us, peak_memory, .. } => {
        println!("Frame took {}μs, peak {} bytes", duration_us, peak_memory);
    }
    _ => {}
});

// Get per-thread statistics
let stats = alloc.thread_frame_stats(thread_id);
println!("Frames: {}, Peak: {} bytes", stats.frames_completed, stats.peak_memory);
```

### New Diagnostic Codes (FA2xx)

| Code | Severity | Description |
|------|----------|-------------|
| FA201 | Error | Cross-thread frame access without explicit transfer |
| FA202 | Warning | Thread not in FrameBarrier but shares frame boundary |
| FA203 | Hint | Thread allocates without budget configured |
| FA204 | Warning | Pattern may overflow deferred queue |
| FA205 | Error | `end_frame()` called without barrier synchronization |

### Performance Characteristics

When disabled (default): **Zero overhead**

When enabled:
| Feature | Overhead |
|---------|----------|
| TransferHandle | ~10ns per transfer |
| FrameBarrier | ~50ns per signal |
| Budget tracking | ~5ns per allocation |
| Lifecycle events | ~100ns per event (callback overhead) |

---

## v0.5.1: Unified Versioning & cargo-fa Enhancements (Detailed)

### Version Note

Starting with v0.5.1, `framealloc` and `cargo-fa` share version numbers to simplify tracking. This version bump reflects:

1. **Tooling parity** — The `cargo-fa` static analyzer is now a mature companion tool
2. **Documentation overhaul** — README professionally reformatted, documentation updated
3. **Ecosystem alignment** — Library and tool versions now stay in sync

**No runtime code changes** were made to the core allocator in this release. The frame arena, pools, and all allocation APIs remain identical to v0.4.0.

### cargo-fa v0.5.1 Features

Extended output formats for CI integration:
- `--format junit` — JUnit XML for test reporters
- `--format checkstyle` — Checkstyle XML for Jenkins

New filtering options:
- `--deny <CODE>` — Treat specific diagnostic as error
- `--allow <CODE>` — Suppress specific diagnostic
- `--exclude <PATTERN>` — Skip paths matching glob
- `--fail-fast` — Stop on first error

New subcommands:
- `cargo fa explain FA601` — Detailed explanation with examples
- `cargo fa show src/file.rs` — Single-file analysis
- `cargo fa list` — List all diagnostic codes
- `cargo fa init` — Generate `.fa.toml` configuration

Optimized `--all` check ordering runs fast checks first for better fail-fast behavior.

---

## v0.5.0: Static Analysis with cargo-fa (Detailed)

The `cargo-fa` tool provides build-time detection of memory intent violations. See the [cargo-fa README](cargo-fa/README.md) for full documentation.

---

## v0.4.0: Memory Behavior Filter (Detailed)

### Overview

The behavior filter detects "bad memory" — allocations that **violate their declared intent**. This is NOT about unsafe memory; it's about catching patterns like:

- Frame allocations that behave like long-lived data
- Pool allocations used as scratch (freed same frame)
- Excessive promotion churn
- Heap allocations in hot paths

### Enabling the Filter

```rust
// Enable tracking (disabled by default, zero overhead when off)
alloc.enable_behavior_filter();

// Run your game loop
for _ in 0..1000 {
    alloc.begin_frame();
    
    alloc.with_tag("physics", |alloc| {
        let scratch = alloc.frame_alloc::<PhysicsScratch>();
        // ...
    });
    
    alloc.end_frame();
}

// Analyze behavior
let report = alloc.behavior_report();
println!("{}", report.summary());

for issue in &report.issues {
    eprintln!("{}", issue);
}
```

### Diagnostic Codes

| Code | Severity | Meaning |
|------|----------|---------|
| FA501 | Warning | Frame allocation survives too long (avg lifetime > threshold) |
| FA502 | Warning | High frame survival rate (>50% survive beyond frame) |
| FA510 | Hint | Pool allocation used as scratch (>80% freed same frame) |
| FA520 | Warning | Promotion churn (>0.5 promotions/frame) |
| FA530 | Warning | Heap allocation in hot path (frequent heap allocs) |

### Example Output

```
[FA501] warning: frame allocation behaves like long-lived data
  tag: ai::pathfinding
  observed: avg lifetime: 128.0 frames
  threshold: expected < 60 frames
  suggestion: Consider using pool_alloc() or scratch_pool()

[FA520] warning: Excessive promotion churn detected
  tag: rendering::meshes
  observed: 0.75 promotions/frame
  threshold: expected < 0.50/frame
  suggestion: Consider allocating directly in the target allocator
```

### Configurable Thresholds

```rust
// Default thresholds
let default = BehaviorThresholds::default();

// Strict thresholds for CI/testing
let strict = BehaviorThresholds::strict();

// Relaxed thresholds for development
let relaxed = BehaviorThresholds::relaxed();

// Custom thresholds
let custom = BehaviorThresholds {
    frame_survival_frames: 120,        // Frames before warning
    frame_survival_rate: 0.3,          // 30% survival rate
    pool_same_frame_free_rate: 0.9,    // 90% freed same frame
    promotion_churn_rate: 0.3,         // 0.3 promotions/frame
    heap_in_hot_path_count: 50,        // 50 heap allocs
    min_samples: 20,                   // Min allocs before analysis
};
```

### Per-Tag Tracking

Statistics are tracked **per-tag**, not per-allocation. This keeps memory overhead at O(tags) instead of O(allocations):

```rust
// Each unique (tag, alloc_kind) pair gets its own stats
pub struct TagBehaviorStats {
    pub tag: &'static str,
    pub kind: AllocKind,
    pub total_allocs: u64,
    pub survived_frame_count: u64,
    pub promotion_count: u64,
    pub same_frame_frees: u64,
    // ...
}
```

### CI Integration

```bash
# Enable strict mode in CI
FRAMEALLOC_STRICT=warning cargo test

# Or programmatically
alloc.enable_behavior_filter();
// ... run tests ...
let report = alloc.behavior_report();
if report.has_warnings() {
    std::process::exit(1);
}
```

### Design Principles

| Principle | Implementation |
|-----------|----------------|
| **Opt-in** | Disabled by default, no overhead |
| **Per-tag** | O(tags) memory, not O(allocations) |
| **Actionable** | Every issue includes a suggestion |
| **Not a cop** | Advises, doesn't block |
| **Deterministic** | Same inputs → same outputs |

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
