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
