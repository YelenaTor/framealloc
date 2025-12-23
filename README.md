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
  <a href="#why-framealloc">Why</a> •
  <a href="#documentation--learning-path">Docs</a> •
  <a href="#quick-start">Quick Start</a> •
  <a href="#features">Features</a> •
  <a href="#gpu-support">GPU Support</a> •
  <a href="#static-analysis">Static Analysis</a>
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

## Why framealloc?

Traditional allocators (malloc, jemalloc) optimize for general-case throughput. Game engines have different needs:

**The Problem:**
```rust
for frame in 0..1000000 {
    let contacts: Vec<Contact> = physics.detect_collisions();
    // 1000+ malloc calls per frame
    // Memory scattered across heap
    // Fragmentation builds up
    // Unpredictable frame times
}
```

**The framealloc Solution:**
```rust
let alloc = SmartAlloc::new(Default::default());

for frame in 0..1000000 {
    alloc.begin_frame();
    let contacts = alloc.frame_vec::<Contact>();
    // Single bump pointer, contiguous memory, cache-friendly
    alloc.end_frame();
    // Everything freed in O(1), zero fragmentation
}
```

**Results:**
- **139x faster** than malloc for batch allocations
- **Stable frame times** — no GC pauses, no fragmentation
- **Explicit lifetimes** — frame/pool/heap explicit in code
- **Observable** — know exactly where memory goes

---

## Documentation & Learning Path

### Getting Started (0-2 hours)
**[Getting Started Guide](docs/getting-started.md)** — Install, write your first allocation, understand core concepts.

*Start here if:* You're evaluating framealloc or just installed it.

### Common Patterns (2-20 hours)
**[Patterns Guide](docs/patterns.md)** — Frame loops, threading, organization, common pitfalls.

*Start here if:* You've used framealloc basics and want to structure real applications.

### Domain Guides

| Domain | Guide | Description |
|--------|-------|-------------|
| **Game Development** | [Game Dev Guide](docs/game-dev.md) | ECS, rendering, audio, level streaming |
| **Physics** | [Rapier Integration](docs/rapier-integration.md) | Contact generation, queries, performance |
| **Async** | [Async Guide](docs/async.md) | Safe patterns, TaskAlloc, avoiding frame violations |
| **Performance** | [Performance Guide](docs/performance.md) | Batch allocation, profiling, benchmarks |

### Advanced Topics (20-100 hours)
**[Advanced Guide](docs/advanced.md)** — Custom allocators, internals, NUMA awareness, instrumentation.

*Start here if:* You're extending framealloc or need maximum performance.

### Reference

| Resource | Description |
|----------|-------------|
| [API Documentation](https://docs.rs/framealloc) | Complete API reference |
| [Cookbook](docs/cookbook.md) | Copy-paste recipes for common tasks |
| [Migration Guide](docs/migration.md) | Coming from other allocators |
| [Troubleshooting](docs/troubleshooting.md) | Common issues and solutions |
| [TECHNICAL.md](TECHNICAL.md) | Architecture and implementation details |
| [CHANGELOG.md](CHANGELOG.md) | Version history |

### Examples

```bash
# Beginner (0-2 hours)
cargo run --example 01_hello_framealloc    # Simplest: begin_frame, alloc, end_frame
cargo run --example 02_frame_loop          # Typical game loop with frame allocations
cargo run --example 03_pools_and_heaps     # When to use frame vs pool vs heap

# Intermediate (2-20 hours)
cargo run --example 04_threading           # TransferHandle and FrameBarrier
cargo run --example 05_tags_and_budgets    # Organizing allocations, enforcing limits

# Advanced (20+ hours)
cargo run --example 06_custom_allocator    # Implementing AllocatorBackend
cargo run --example 07_batch_optimization  # Using frame_alloc_batch for particles
```

---

## Coming From...

**Default Rust (`Vec`, `Box`):**
```rust
// Before:                      // After:
let scratch = vec![0u8; 1024];  let scratch = alloc.frame_slice::<u8>(1024);
```

**bumpalo:**
```rust
// bumpalo:                     // framealloc:
let bump = Bump::new();         alloc.begin_frame();
let x = bump.alloc(42);         let x = alloc.frame_alloc::<i32>();
bump.reset();                   alloc.end_frame();
```

**C++ game allocators:** Frame allocators → `frame_alloc()` | Object pools → `pool_alloc()` | Custom → `AllocatorBackend` trait

See [Migration Guide](docs/migration.md) for detailed conversion steps.

---

## Quick Start

### Basic Usage

```rust
use framealloc::{SmartAlloc, AllocConfig};

fn main() {
    let alloc = SmartAlloc::new(AllocConfig::default());

    loop {
        alloc.begin_frame();
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
}
```

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
    
    alloc.end_frame();
}
```

### Thread Coordination (v0.6.0)

```rust
// Explicit cross-thread transfers
let handle = alloc.frame_box_for_transfer(data);
worker_channel.send(handle);

// Frame barriers for deterministic sync
let barrier = FrameBarrier::new(3);
barrier.signal_frame_complete();
barrier.wait_all();

// Per-thread budgets
alloc.set_thread_frame_budget(megabytes(8));
```

### IDE Integration (v0.7.0)

**fa-insight** — VS Code extension for framealloc-aware development:

```rust
fn physics_update(alloc: &SmartAlloc) {  // 💾 2.1 MB ↗ 📊
    // CodeLens shows: current usage, trend, sparkline
    alloc.with_tag("physics", |a| {
        let contacts = a.frame_vec::<Contact>();
    });
}
```

Features: CodeLens memory display, trend graphs, budget alerts at 80%+ usage.

Install: Search "FA Insight" in [VS Code Marketplace](https://marketplace.visualstudio.com/items?itemName=YelenaTor.fa-insight)

### Tokio Integration (v0.8.0)

```rust
use framealloc::tokio::{TaskAlloc, AsyncPoolGuard};

// Main thread: frame allocations OK
alloc.begin_frame();
let scratch = alloc.frame_vec::<f32>();

// Async tasks: use TaskAlloc (pool-backed, auto-cleanup)
tokio::spawn(async move {
    let mut task = TaskAlloc::new(&alloc_clone);
    let data = task.alloc_box(load_asset().await);
});

alloc.end_frame();
```

**Key principle:** Frame allocations stay on main thread, async tasks use pool/heap.

Enable: `framealloc = { version = "0.10", features = ["tokio"] }`

See [Async Guide](docs/async.md) for the full async safety guide.

### Batch Allocations (v0.9.0)

> ⚠️ **SAFETY FIRST:** Batch APIs use raw pointers

**139x faster** than individual allocations, but requires `unsafe`:

```rust
let items = alloc.frame_alloc_batch::<Item>(1000);

// SAFETY REQUIREMENTS:
// 1. Indices must be within 0..count
// 2. Must initialize with std::ptr::write before reading
// 3. Pointers invalid after end_frame()
// 4. Not Send/Sync - don't pass to other threads

unsafe {
    for i in 0..1000 {
        let item = items.add(i);
        std::ptr::write(item, Item::new(i));
    }
}
```

**Specialized sizes** (zero overhead, no unsafe):
```rust
let [a, b] = alloc.frame_alloc_2::<Vec2>();       // Pairs
let [a, b, c, d] = alloc.frame_alloc_4::<Vertex>(); // Quads
let items = alloc.frame_alloc_8::<u64>();         // Cache line
```

### Rapier Physics Integration (v0.10.0)

Frame-aware wrappers for Rapier physics engine v0.31:

```rust
use framealloc::{SmartAlloc, rapier::PhysicsWorld2D};

let mut physics = PhysicsWorld2D::new();

alloc.begin_frame();
let events = physics.step_with_events(&alloc);
for contact in events.contacts {
    println!("Contact: {:?}", contact);
}
alloc.end_frame();
```

**Why Rapier v0.31 matters:** Rapier v0.31 refactored broad-phase and query APIs. If you're using Rapier ≤v0.30, use framealloc v0.9.0 instead.

Enable: `framealloc = { version = "0.10", features = ["rapier"] }`

See [Rapier Integration Guide](docs/rapier-integration.md) for full documentation.

---

## GPU Support (v0.11.0)

framealloc now supports **unified CPU-GPU memory management** with clean separation and optional GPU backends.

### Architecture

- **CPU Module**: Always available, zero GPU dependencies
- **GPU Module**: Feature-gated (`gpu`), backend-agnostic traits
- **Coordinator Module**: Bridges CPU and GPU (`coordinator` feature)

### Feature Flags

```toml
# Enable GPU support (no backend yet)
framealloc = { version = "0.11", features = ["gpu"] }

# Enable Vulkan backend
framealloc = { version = "0.11", features = ["gpu-vulkan"] }

# Enable unified CPU-GPU coordination
framealloc = { version = "0.11", features = ["gpu-vulkan", "coordinator"] }
```

### Quick Example

```rust
#[cfg(feature = "coordinator")]
use framealloc::coordinator::UnifiedAllocator;
use framealloc::gpu::traits::{BufferUsage, MemoryType};

// Create unified allocator
let mut unified = UnifiedAllocator::new(cpu_alloc, gpu_alloc);

// Begin frame
unified.begin_frame();

// Create staging buffer for CPU-GPU transfer
let staging = unified.create_staging_buffer(2048)?;
if let Some(slice) = staging.cpu_slice_mut() {
    slice.copy_from_slice(&vertex_data);
}

// Transfer to GPU
unified.transfer_to_gpu(&mut staging)?;

// Check usage
let (cpu_mb, gpu_mb) = unified.get_usage();
println!("CPU: {} MB, GPU: {} MB", cpu_mb / 1024 / 1024, gpu_mb / 1024 / 1024);

unified.end_frame();
```

### Key Benefits

- **Zero overhead** for CPU-only users (no new deps)
- **Backend-agnostic** GPU traits (Vulkan today, more tomorrow)
- **Unified budgeting** across CPU and GPU memory
- **Explicit transfers** - no hidden synchronization costs

### GPU Backend Roadmap

**Why Vulkan First?**
Vulkan provides the most explicit control over memory allocation, making it ideal for demonstrating framealloc's intent-driven approach. Its low-level nature exposes all the memory concepts we abstract (device-local, host-visible, staging buffers), serving as the perfect reference implementation.

**Planned Backend Support**

| Platform | Status | Notes |
|----------|--------|-------|
| **Vulkan** | ✅ Available | Low-level, explicit memory control |
| **Direct3D 11/12** | 🔄 Planned | Windows gaming platforms |
| **Metal** | 🔄 Planned | Apple ecosystem (iOS/macOS) |
| **WebGPU** | 🔄 Future | Browser-based applications |

**Generic GPU Usage**
You can use framealloc's GPU traits without committing to a specific backend:

```rust
use framealloc::gpu::{GpuMemoryIntent, GpuLifetime, GpuAllocRequirements};

// Intent-driven allocation works with any backend
let req = GpuAllocRequirements::new(
    size,
    GpuMemoryIntent::Staging,  // Expresses WHAT, not HOW
    GpuLifetime::Frame,        // Clear lifetime semantics
);

// Backend-agnostic allocation
let buffer = allocator.allocate(req)?;
```

The intent-based design ensures your code remains portable as new backends are added. Simply swap the allocator implementation without changing allocation logic.

---

## Static Analysis

**cargo-fa** detects memory intent violations before runtime.

```bash
cargo install --path cargo-fa

# Check specific categories
cargo fa --dirtymem       # Frame escape, hot loop allocations
cargo fa --async-safety   # Async/await boundary issues
cargo fa --threading      # Cross-thread frame access
cargo fa --all            # Run all checks

# CI integration
cargo fa --all --format sarif  # GitHub Actions
```

| Range | Category | Examples |
|-------|----------|----------|
| FA2xx | Threading | Cross-thread access, barrier mismatch |
| FA6xx | Lifetime | Frame escape, hot loops, missing boundaries |
| FA7xx | Async | Allocation across await, closure capture |
| FA9xx | Rapier | QueryFilter import, step_with_events usage |

---

## Cargo Features

| Feature | Description |
|---------|-------------|
| `bevy` | Bevy ECS plugin integration |
| `rapier` | Rapier physics engine integration |
| `tokio` | Async/await support with Tokio |
| `parking_lot` | Faster mutex implementation |
| `debug` | Memory poisoning, allocation backtraces |
| `minimal` | Disable statistics for max performance |
| `prefetch` | Hardware prefetch hints (x86_64) |

---

## Performance

Allocation priority minimizes latency:

1. **Frame arena** — Bump pointer increment, no synchronization
2. **Thread-local pools** — Free list pop, no contention
3. **Global pool refill** — Mutex-protected, batched
4. **System heap** — Fallback for oversized allocations

In typical game workloads, **90%+ of allocations** hit the frame arena path.

---

## License

Licensed under either of:

- [Apache License, Version 2.0](LICENSE-APACHE)
- [MIT License](LICENSE-MIT)

at your option.
