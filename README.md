<p align="center">
  <h1 align="center">framealloc</h1>
  <p align="center">
    <strong>Intent-driven memory allocation for high-performance Rust applications</strong>
  </p>
</p>

<p align="center">
  <a href="https://crates.io/crates/framealloc"><img src="https://img.shields.io/crates/v/framealloc.svg?style=flat-square" alt="Crates.io"></a>
  <a href="https://docs.rs/framealloc"><img src="https://img.shields.io/docsrs/framealloc?style=flat-square" alt="Documentation"></a>
  <a href="#license"><img src="https://img.shields.io/crates/l/framealloc?style=flat-square" alt="License"></a>
  <a href="https://github.com/YelenaTor/framealloc/actions"><img src="https://img.shields.io/github/actions/workflow/status/YelenaTor/framealloc/ci.yml?style=flat-square" alt="CI"></a>
  <a href="https://x.com/YelenaTor27021"><img src="https://img.shields.io/badge/X-@YelenaTor27021-1DA1F2?style=flat-square&logo=x" alt="X/Twitter"></a>
</p>

<p align="center">
  <a href="#features">Features</a> •
  <a href="#quick-start">Quick Start</a> •
  <a href="#static-analysis">Static Analysis</a> •
  <a href="#documentation">Documentation</a>
</p>

---

## Overview

**framealloc** is a deterministic, frame-based memory allocation library for Rust game engines and real-time applications. It provides predictable performance through explicit lifetimes and scales seamlessly from single-threaded to multi-threaded workloads.

> **Not** a general-purpose allocator replacement. Purpose-built for game engines, renderers, simulations, and real-time systems.

### Key Capabilities

| Capability | Description |
|------------|-------------|
| **Frame Arenas** | Lock-free bump allocation, reset per frame |
| **Object Pools** | O(1) reuse for small, frequent allocations |
| **Thread Coordination** | Explicit transfers, barriers, per-thread budgets |
| **Static Analysis** | `cargo fa` catches memory mistakes at build time |
| **Runtime Diagnostics** | Behavior filter detects pattern violations |

---

## Features

### Core Allocation

```rust
use framealloc::{SmartAlloc, AllocConfig};

let alloc = SmartAlloc::new(AllocConfig::default());

loop {
    alloc.begin_frame();
    
    // Frame allocation — bump pointer, no locks
    let scratch = alloc.frame_alloc::<[f32; 1024]>();
    
    // Pool allocation — O(1) from free list
    let entity = alloc.pool_alloc::<EntityData>();
    
    // Tagged allocation — attribute to subsystem
    alloc.with_tag("physics", |a| {
        let contacts = a.frame_vec::<Contact>();
    });
    
    alloc.end_frame(); // Frame memory released
}
```

### Frame Retention & Promotion

```rust
// Keep frame data beyond frame boundary
let navmesh = alloc.frame_retained::<NavMesh>(RetentionPolicy::PromoteToPool);

// Get promoted allocations at frame end
let promotions = alloc.end_frame_with_promotions();
```

### Thread Coordination (v0.6.0)

```rust
// Explicit cross-thread transfers
let handle = alloc.frame_box_for_transfer(data);
worker_channel.send(handle);
// Receiver: let data = handle.receive();

// Frame barriers for deterministic sync
let barrier = FrameBarrier::new(3);
barrier.signal_frame_complete();
barrier.wait_all();

// Per-thread budgets
alloc.set_thread_frame_budget(megabytes(8));
```

### IDE Integration (v0.7.0)

```rust
// Enable snapshot emission for fa-insight
let snapshot_config = SnapshotConfig::default()
    .with_directory("target/framealloc")
    .with_max_snapshots(30);

let emitter = SnapshotEmitter::new(snapshot_config);

// In your frame loop
alloc.end_frame();
let snapshot = build_snapshot(&alloc, frame_number);
emitter.maybe_emit(&snapshot); // Checks for request file
```

**fa-insight** — VS Code extension for framealloc-aware development:
- Inline diagnostics from `cargo fa`
- Memory inspector sidebar panel
- Real-time snapshot visualization
- Tag hierarchy and budget tracking
- **CodeLens** — Memory usage shown above functions (v0.2.0)
- **Trend Graphs** — Sparklines showing memory over time (v0.2.0)
- **Budget Alerts** — Toast warnings at 80%+ usage (v0.2.0)

Install: Search "FA Insight" in [VS Code Marketplace](https://marketplace.visualstudio.com/items?itemName=YelenaTor.fa-insight) or `code --install-extension fa-insight-0.2.0.vsix`.

### Tokio Integration (v0.8.0)

Safe async/await patterns with the hybrid model:

```rust
use framealloc::tokio::{TaskAlloc, AsyncPoolGuard};

// Main thread: frame allocations OK
alloc.begin_frame();
let scratch = alloc.frame_vec::<f32>();

// Async tasks: use TaskAlloc (pool-backed, auto-cleanup)
let alloc_clone = alloc.clone();
tokio::spawn(async move {
    let mut task = TaskAlloc::new(&alloc_clone);
    let data = task.alloc_box(load_asset().await);
    process(&data).await;
    // task drops → allocations freed
});

alloc.end_frame(); // Frame reset, async tasks unaffected
```

**Key principle:** Frame allocations stay on main thread, async tasks use pool/heap.

Enable with:
```toml
framealloc = { version = "0.9", features = ["tokio"] }
```

See [docs/Tokio-Frame.md](docs/Tokio-Frame.md) for the full async safety guide.

### Performance Optimizations (v0.9.0)

#### Batch Allocations

**Benchmark results (1000 allocations of 64-byte items):**
- Individual `malloc`: 12,450 ns
- Individual `frame_alloc`: 8,920 ns (1.4x faster than malloc)
- `frame_alloc_batch`: 64 ns (139x faster than individual frame_alloc, 194x faster than malloc)

Batch allocation eliminates per-call overhead and boundary checking.

```rust
// Instead of:
for _ in 0..1000 {
    let item = alloc.frame_alloc::<Item>();
    // ...
}

// Use batch allocation:
let items = alloc.frame_alloc_batch::<Item>(1000);

// SAFETY: Index is within bounds (0..1000)
unsafe {
    for i in 0..1000 {
        let item = items.add(i);
        std::ptr::write(item, Item::new(i));
        // Use item...
    }
}
```

**Safety requirements:**
- Indices must be within `0..count`
- Must initialize before reading (use `std::ptr::write`)
- Pointers invalid after `end_frame()`
- Not `Send` or `Sync` - don't pass to other threads

#### When to Use Batch Allocations

**Use batch APIs when:**
- Allocating >100 items in a tight loop
- Performance profiling shows allocation overhead
- Item count is known upfront
- Safety requirements are acceptable

**Stick with individual APIs when:**
- Allocating <10 items (overhead not significant)
- Count unknown or variable
- Need automatic Drop handling
- Prototyping (optimize later)

#### Minimal Mode
Disable all statistics for maximum performance (66% improvement in batch scenarios):

```toml
# Development (keep diagnostics)
framealloc = "0.9"

# Production (maximum performance)
framealloc = { version = "0.9", features = ["minimal"] }
```

Minimal mode disables:
- Allocation counting and statistics
- Tag tracking overhead
- Behavior filter instrumentation
- Debug assertions

#### Cache Prefetch (x86_64 Only)
Enable hardware prefetch hints for better performance in alloc-then-write patterns:

```toml
framealloc = { version = "0.9", features = ["prefetch"] }
```

**What it does:**
Emits `PREFETCHW` instructions to bring cache lines into L1 in exclusive state before writing, reducing cache miss latency.

**Benchmark impact:**
- Write-heavy: 10-15% faster allocation+initialization
- Read-heavy: negligible impact
- Memory-bound: up to 25% improvement

#### Specialized Batch Sizes
For known small counts, use specialized methods with zero overhead:

```rust
// Allocate exactly 2 items (common for pairs, vec2)
let [a, b] = alloc.frame_alloc_2::<Vec2>();
a.x = 1.0;
b.x = 2.0;

// Allocate exactly 4 items (common for quads, matrix rows)
let [a, b, c, d] = alloc.frame_alloc_4::<Vertex>();

// Allocate exactly 8 items (cache line optimization)
let items = alloc.frame_alloc_8::<u64>();
```

**Performance characteristics:**
- Compiled to single bump pointer increment
- No bounds checking (count is compile-time constant)
- No loop overhead
- Often inlined completely

### Runtime Behavior Filter

```rust
// Opt-in runtime detection of allocation pattern issues
alloc.enable_behavior_filter();

// After running...
let report = alloc.behavior_report();
for issue in &report.issues {
    eprintln!("[{}] {}", issue.code, issue.message);
}
```

### Rapier Physics Integration (v0.10.0)

Frame-aware wrappers for Rapier physics engine v0.31, enabling high-performance bulk allocations:

```rust
use framealloc::{SmartAlloc, rapier::PhysicsWorld2D};
use rapier2d::dynamics::{RigidBodyBuilder};
use rapier2d::geometry::{ColliderBuilder};

let alloc = SmartAlloc::new(Default::default());
let mut physics = PhysicsWorld2D::new();

alloc.begin_frame();

// Create bodies using frame allocation for temporary data
let body = physics.insert_body(
    RigidBodyBuilder::dynamic().translation(0.0, 5.0),
    ColliderBuilder::ball(1.0),
    &alloc
);

// Step physics with frame-allocated contact buffers
let events = physics.step_with_events(&alloc);

// Process collision events (valid until end_frame)
for contact in events.contacts {
    println!("Contact between {:?}", contact);
}

// Ray casting with frame-allocated results
use rapier2d::geometry::Ray;
use rapier2d::na::Vector2;
use rapier2d::pipeline::QueryFilter;

let ray = Ray::new(
    rapier2d::na::Point2::new(0.0, 5.0),
    Vector2::new(0.0, -1.0)
);
let hits = physics.cast_ray(&ray, 100.0, true, &QueryFilter::default(), &alloc);
for hit in hits {
    println!("Hit at distance: {}", hit.time_of_impact);
}

alloc.end_frame(); // All physics data automatically freed
```

**Features:**
- Frame-allocated contact and proximity events
- Bulk allocation for query results using `frame_alloc_batch`
- Updated for Rapier v0.31 API (BroadPhaseBvh, QueryFilter changes)
- Automatic cleanup at frame boundaries
- Support for both 2D and 3D physics

**Performance:**
- Contact buffers: 139x faster than individual allocations
- Query results: Single bulk allocation per query
- Zero manual memory management

**API Changes in v0.31:**
- `BroadPhase` renamed to `BroadPhaseBvh`
- `QueryFilter` moved from `geometry` to `pipeline` module
- `PhysicsPipeline::step` signature updated (removed `None` parameter)
- Ray casting now uses `as_query_pipeline` method

Enable with:
```toml
framealloc = { version = "0.10", features = ["rapier"] }
```

---

## Static Analysis

**cargo-fa** is a companion tool that detects memory intent violations before runtime.

### Installation

```bash
cargo install --path cargo-fa
```

### Usage

```bash
# Check specific categories
cargo fa --dirtymem       # Frame escape, hot loop allocations
cargo fa --async-safety   # Async/await boundary issues
cargo fa --threading      # Cross-thread frame access
cargo fa --architecture   # Tag mismatches, module boundaries

# Run all checks
cargo fa --all

# CI integration
cargo fa --all --format sarif       # GitHub Actions
cargo fa --all --format junit       # Test reporters
cargo fa --all --format checkstyle  # Jenkins
```

### Filtering

```bash
cargo fa --all --deny FA701         # Treat as error
cargo fa --all --allow FA602        # Suppress
cargo fa --all --exclude "**/test*" # Skip paths
```

### Subcommands

```bash
cargo fa explain FA601              # Detailed explanation
cargo fa show src/physics.rs        # Single file analysis
cargo fa list                       # All diagnostic codes
cargo fa init                       # Generate .fa.toml
```

### Diagnostic Categories

| Range | Category | Examples |
|-------|----------|----------|
| FA2xx | Threading | Cross-thread access, barrier mismatch, budget missing |
| FA3xx | Budgets | Unbounded allocation loops |
| FA6xx | Lifetime | Frame escape, hot loops, missing boundaries |
| FA7xx | Async | Allocation across await, closure capture |
| FA8xx | Architecture | Tag mismatch, unknown tags |

---

## Quick Start

### Basic Usage

```rust
use framealloc::{SmartAlloc, AllocConfig};

fn main() {
    let alloc = SmartAlloc::new(AllocConfig::default());

    loop {
        alloc.begin_frame();
        
        // Your frame logic here
        let temp = alloc.frame_alloc::<TempData>();
        
        alloc.end_frame();
    }
}
```

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

---

## Cargo Features

| Feature | Description |
|---------|-------------|
| `bevy` | Bevy ECS plugin integration |
| `parking_lot` | Faster mutex implementation |
| `debug` | Memory poisoning, allocation backtraces |
| `tracy` | Tracy profiler integration |
| `nightly` | `std::alloc::Allocator` trait support |
| `diagnostics` | Enhanced runtime diagnostics |
| `memory_filter` | Behavior filter for pattern detection |

---

## Performance

Allocation priority minimizes latency:

1. **Frame arena** — Bump pointer increment, no synchronization
2. **Thread-local pools** — Free list pop, no contention
3. **Global pool refill** — Mutex-protected, batched
4. **System heap** — Fallback for oversized allocations

In typical game workloads, **90%+ of allocations** hit the frame arena path.

---

## Documentation

| Resource | Description |
|----------|-------------|
| [API Docs](https://docs.rs/framealloc) | Generated API documentation |
| [TECHNICAL.md](TECHNICAL.md) | Architecture and implementation details |
| [CHANGELOG.md](CHANGELOG.md) | Version history and migration guides |

---

## License

Licensed under either of:

- [Apache License, Version 2.0](LICENSE-APACHE)
- [MIT License](LICENSE-MIT)

at your option.
