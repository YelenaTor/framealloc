# framealloc Technical Guide

**Deterministic, frame-based memory allocation for Rust game engines.**

`framealloc` is a purpose-built memory allocator for game engines, renderers, simulations, and real-time systems. It provides predictable performance through explicit lifetimes and scales automatically from single-threaded to multi-threaded workloads.

---

## Table of Contents

1. [Core Concepts](#core-concepts)
2. [Architecture](#architecture)
3. [Version History](#version-history)
   - [v0.7.0 - IDE Integration](#v070---ide-integration)
   - [v0.6.0 - Thread Coordination](#v060---thread-coordination)
   - [v0.5.0 - Static Analysis](#v050---static-analysis)
   - [v0.4.0 - Behavior Filter](#v040---behavior-filter)
   - [v0.3.0 - Frame Retention](#v030---frame-retention)
   - [v0.2.0 - Frame Phases](#v020---frame-phases)
4. [Performance Characteristics](#performance-characteristics)
5. [Integration Patterns](#integration-patterns)

---

## Core Concepts

### Frame-Based Allocation

The primary allocator is a **frame arena** — a bump allocator that resets every frame:

```rust
use framealloc::{SmartAlloc, AllocConfig};

let alloc = SmartAlloc::new(AllocConfig::default());

loop {
    alloc.begin_frame();
    
    // Fast bump allocation - O(1), no locks
    let scratch = alloc.frame_alloc::<[f32; 1024]>();
    
    alloc.end_frame(); // All frame memory released
}
```

**Benefits:**
- Eliminates fragmentation
- Guarantees O(1) allocation
- Matches game engine frame lifecycle

### Allocation by Intent

Allocations are categorized by **intent**, not just size:

| Intent | Method | Lifetime | Performance |
|--------|--------|----------|-------------|
| **Frame** | `frame_alloc()` | Current frame only | Fastest - bump pointer |
| **Pool** | `pool_alloc()` | Until explicitly freed | Fast - free list |
| **Heap** | `heap_alloc()` | Until explicitly freed | Fallback - system allocator |

### Thread-Local Fast Paths

Every thread automatically gets:
- Its own frame arena
- Its own small-object pools
- Zero locks on hot paths

```text
Single-threaded: 1 TLS allocator
Multi-threaded:  N TLS allocators
Same API. Same behavior.
```

No mode switching required — scales automatically.

### Tagged Allocations

Attribute allocations to subsystems for profiling and budgeting:

```rust
alloc.with_tag("physics", |a| {
    let contacts = a.frame_vec::<Contact>();
    // All allocations tracked under "physics"
});
```

---

## Architecture

```text
SmartAlloc (Arc-wrapped, thread-safe)
 │
 ├── GlobalState (shared across threads)
 │    ├── SystemHeap (large allocations)
 │    ├── SlabRegistry (page pools)
 │    ├── BudgetManager (optional limits)
 │    └── Statistics (atomic counters)
 │
 └── ThreadLocalState (per-thread, no locks)
      ├── FrameArena (bump allocator)
      ├── PoolAllocator (free lists)
      └── LocalStats (thread metrics)
```

**Allocation Priority:**
1. Frame arena (bump pointer, no sync)
2. Thread-local pools (free list, no contention)
3. Global pool refill (mutex, batched)
4. System heap (fallback for large objects)

In typical workloads, **90%+ of allocations** hit the frame arena path.

---

## Version History

### v0.7.0 - IDE Integration

**Release:** 2025-12-22

#### Features

**Snapshot Emission API** — Runtime observability for IDE tooling:

```rust
use framealloc::{SnapshotConfig, SnapshotEmitter, Snapshot};

let config = SnapshotConfig::default()
    .with_directory("target/framealloc")
    .with_max_snapshots(30);

let emitter = SnapshotEmitter::new(config);

// In frame loop
alloc.end_frame();
let snapshot = Snapshot::new(frame_number);
// ... populate from allocator state ...
emitter.maybe_emit(&snapshot);
```

**Snapshot Schema v1:**
```json
{
  "version": 1,
  "frame": 18421,
  "summary": { "frame_bytes": 4194304, "pool_bytes": 2097152 },
  "threads": [...],
  "tags": [...],
  "promotions": { "to_pool": 12, "to_heap": 3 },
  "diagnostics": [...]
}
```

**cargo-fa JSON Output** — Structured diagnostics for IDE consumption:

```bash
cargo fa --all --format json > diagnostics.json
```

**fa-insight Extension** — VS Code integration:
- Inline diagnostics from `cargo fa`
- Memory inspector sidebar
- Real-time snapshot visualization
- Tag hierarchy and budget tracking

#### Use Cases
- **Development:** Live memory inspection during debugging
- **CI/CD:** Automated memory pattern analysis
- **Profiling:** Frame-by-frame allocation tracking

#### Philosophy
- **Opt-in:** Only emitted when explicitly enabled
- **Aggregated:** No per-allocation data, only summaries
- **Bounded:** Rate-limited (min 500ms), auto-cleanup
- **Safe:** Only captured at frame boundaries

---

### v0.6.0 - Thread Coordination

**Release:** 2025-12-21

#### Features

**TransferHandle** — Explicit cross-thread transfers:

```rust
use framealloc::TransferHandle;

// Producer thread
let handle: TransferHandle<Data> = alloc.frame_box_for_transfer(data);
channel.send(handle);

// Consumer thread
let data = handle.receive();
```

**FrameBarrier** — Deterministic multi-threaded sync:

```rust
use framealloc::FrameBarrier;

let barrier = FrameBarrier::new(3); // 3 threads

// Each thread
alloc.end_frame();
barrier.signal_frame_complete();
barrier.wait_all(); // All threads synchronized
```

**Per-Thread Budgets** — Memory limits per thread:

```rust
alloc.set_thread_frame_budget(megabytes(8));
alloc.set_thread_pool_budget(megabytes(4));
```

**Deferred Processing Control:**

```rust
use framealloc::{DeferredConfig, QueueFullPolicy};

let config = DeferredConfig::default()
    .with_max_queue_depth(1024)
    .with_policy(QueueFullPolicy::Block);

alloc.configure_deferred(config);
```

#### Use Cases
- **Parallel systems:** Explicit data transfer between worker threads
- **Frame sync:** Ensure all threads complete before next frame
- **Memory control:** Prevent runaway allocation in parallel tasks

---

### v0.5.0 - Static Analysis

**Release:** 2025-11-15

#### Features

**cargo-fa** — Build-time memory intent analysis:

```bash
# Check specific categories
cargo fa --dirtymem       # Frame escape, hot loops
cargo fa --async-safety   # Async/await boundaries
cargo fa --threading      # Cross-thread access
cargo fa --architecture   # Tag mismatches

# Run all checks
cargo fa --all

# CI integration
cargo fa --all --format sarif
cargo fa --all --format junit
```

**Diagnostic Categories:**

| Code Range | Category | Examples |
|------------|----------|----------|
| FA2xx | Threading | Cross-thread frame access, barrier misuse |
| FA3xx | Budgets | Unbounded loops, missing limits |
| FA6xx | Lifetime | Frame escape, hot loop allocation |
| FA7xx | Async | Allocation across await points |
| FA8xx | Architecture | Tag mismatch, unknown tags |

**Subcommands:**

```bash
cargo fa explain FA601    # Detailed explanation
cargo fa show src/file.rs # Single-file analysis
cargo fa list             # All diagnostic codes
cargo fa init             # Generate .fa.toml config
```

#### Use Cases
- **Development:** Catch memory mistakes before runtime
- **CI/CD:** Enforce allocation patterns in builds
- **Code review:** Automated memory intent verification

---

### v0.4.0 - Behavior Filter

**Release:** 2025-10-20

#### Features

**Runtime Pattern Detection** — Opt-in allocation behavior analysis:

```rust
alloc.enable_behavior_filter();

// Run your game loop
for _ in 0..1000 {
    alloc.begin_frame();
    // ... game logic ...
    alloc.end_frame();
}

// Analyze behavior
let report = alloc.behavior_report();
for issue in &report.issues {
    eprintln!("[{}] {}", issue.code, issue.message);
}
```

**Detected Patterns:**
- Frame allocations that behave like long-lived data
- Pool allocations freed same frame (should be frame)
- Excessive promotion churn
- Heap allocations in hot paths

**Diagnostic Codes:**

| Code | Issue | Recommendation |
|------|-------|----------------|
| BF001 | Frame allocation lives >10 frames | Use pool or heap |
| BF002 | Pool allocation freed same frame | Use frame allocation |
| BF003 | High promotion rate (>5% per frame) | Adjust retention policy |
| BF004 | Heap allocation in hot path | Use frame or pool |

#### Use Cases
- **Profiling:** Identify allocation anti-patterns
- **Optimization:** Find mismatched intent vs. behavior
- **Testing:** Validate allocation assumptions

---

### v0.3.0 - Frame Retention

**Release:** 2025-09-10

#### Features

**Retention Policies** — Keep frame data beyond frame boundary:

```rust
use framealloc::RetentionPolicy;

// Allocate with retention
let navmesh = alloc.frame_retained::<NavMesh>(
    RetentionPolicy::PromoteToPool
);

// Get promoted allocations at frame end
let promotions = alloc.end_frame_with_promotions();
for promo in promotions {
    match promo.policy {
        RetentionPolicy::PromoteToPool => {
            // Now in pool, manually free when done
        }
        RetentionPolicy::PromoteToHeap => {
            // Now in heap
        }
    }
}
```

**Policies:**

| Policy | Behavior | Use Case |
|--------|----------|----------|
| `PromoteToPool` | Move to pool allocator | Medium-lived data (entities, resources) |
| `PromoteToHeap` | Move to system heap | Long-lived data (level data, assets) |
| `Leak` | Intentional leak | Static data, never freed |

**Scratch Allocators:**

```rust
let scratch = alloc.create_scratch_allocator(megabytes(2));

scratch.with_scope(|s| {
    let temp = s.alloc::<TempData>();
    // ... use temp ...
}); // Automatically reset
```

#### Use Cases
- **Level loading:** Promote level data from frame to heap
- **Entity spawning:** Promote entity data to pool
- **Pathfinding:** Scratch allocator for A* node storage

---

### v0.2.0 - Frame Phases

**Release:** 2025-08-05

#### Features

**Named Phases** — Divide frames for profiling:

```rust
alloc.begin_frame();

alloc.begin_phase("physics");
let contacts = alloc.frame_alloc::<[Contact; 256]>();
alloc.end_phase();

alloc.begin_phase("render");
let verts = alloc.frame_alloc::<[Vertex; 4096]>();
alloc.end_phase();

alloc.end_frame();
```

**Phase Statistics:**

```rust
let stats = alloc.phase_stats();
for (name, stat) in stats {
    println!("{}: {} bytes", name, stat.bytes_allocated);
}
```

**Tagged Allocations:**

```rust
alloc.with_tag("audio", |a| {
    let buffer = a.frame_alloc::<[f32; 1024]>();
});

// Query by tag
let audio_usage = alloc.tag_stats("audio");
```

#### Use Cases
- **Profiling:** Identify which systems allocate most
- **Budgeting:** Set per-system memory limits
- **Debugging:** Track allocation sources

---

## Performance Characteristics

### Allocation Latency

| Operation | Typical Latency | Notes |
|-----------|----------------|-------|
| `frame_alloc()` | ~5-10ns | Bump pointer increment |
| `pool_alloc()` | ~15-30ns | Free list pop (TLS) |
| `heap_alloc()` | ~100-500ns | System allocator fallback |
| `begin_frame()` | ~50ns | Reset frame pointer |
| `end_frame()` | ~100ns | Cleanup + stats |

### Memory Overhead

| Component | Per-Thread | Global |
|-----------|------------|--------|
| Frame arena | 4-16 MB | - |
| Pool allocator | ~64 KB | 1-4 MB |
| Metadata | ~1 KB | ~10 KB |

### Scaling Characteristics

**Single-threaded:**
- Zero mutex contention
- No atomic operations in hot paths
- Predictable cache behavior

**Multi-threaded (N threads):**
- Thread-local allocation: O(1) per thread
- Global refill: O(1) amortized, infrequent
- Frame sync: O(N) with `FrameBarrier`

---

## Integration Patterns

### Bevy Integration

```rust
use bevy::prelude::*;
use framealloc::bevy::SmartAllocPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(SmartAllocPlugin::default())
        .run();
}

fn physics_system(alloc: Res<framealloc::bevy::AllocResource>) {
    let contacts = alloc.frame_vec::<Contact>();
    // Automatically reset each frame
}
```

### Custom Game Loop

```rust
use framealloc::{SmartAlloc, AllocConfig};

let alloc = SmartAlloc::new(AllocConfig::default()
    .with_frame_size(megabytes(16))
    .with_pool_size(megabytes(8))
);

loop {
    alloc.begin_frame();
    
    // Update systems
    physics_update(&alloc);
    render_update(&alloc);
    
    alloc.end_frame();
}
```

### Multi-Threaded Workload

```rust
use std::sync::Arc;
use framealloc::{SmartAlloc, FrameBarrier};

let alloc = Arc::new(SmartAlloc::with_defaults());
let barrier = Arc::new(FrameBarrier::new(4));

for i in 0..4 {
    let alloc = alloc.clone();
    let barrier = barrier.clone();
    
    std::thread::spawn(move || {
        loop {
            alloc.begin_frame();
            
            // Worker logic
            let data = alloc.frame_box_for_transfer(compute());
            
            alloc.end_frame();
            barrier.signal_frame_complete();
            barrier.wait_all();
        }
    });
}
```

---

## Best Practices

### Do's ✓

- Use `frame_alloc()` for per-frame scratch data
- Use `pool_alloc()` for entities and medium-lived objects
- Use `heap_alloc()` for large, long-lived data
- Tag allocations by subsystem for profiling
- Set budgets to prevent runaway allocation
- Use `cargo fa` in CI to enforce patterns

### Don'ts ✗

- Don't use frame allocations across frame boundaries
- Don't allocate in hot loops without budgets
- Don't mix framealloc with global allocator for same data
- Don't skip `end_frame()` — causes memory leaks
- Don't use framealloc for arbitrary object graphs

---

## Further Reading

- [API Documentation](https://docs.rs/framealloc)
- [README](README.md) — Quick start and features
- [CHANGELOG](CHANGELOG.md) — Version history and migration
- [cargo-fa README](cargo-fa/README.md) — Static analysis tool
- [TECHNICAL.old.md](TECHNICAL.old.md) — Detailed legacy documentation

---

**License:** MIT or Apache-2.0  
**Repository:** https://github.com/YelenaTor/framealloc  
**Crates.io:** https://crates.io/crates/framealloc
