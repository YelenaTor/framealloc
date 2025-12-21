# framealloc

**Intent-aware, thread-smart memory allocation for Rust game engines.**

[![Crates.io](https://img.shields.io/crates/v/framealloc.svg)](https://crates.io/crates/framealloc)
[![Documentation](https://docs.rs/framealloc/badge.svg)](https://docs.rs/framealloc)
[![License](https://img.shields.io/crates/l/framealloc.svg)](LICENSE)

- Frame-based arenas (bump allocation, reset per frame)
- Thread-local fast paths (zero locks in common case)
- Automatic ST → MT scaling (no mode switching)
- Optional Bevy integration
- Allocation diagnostics & budgeting

## Why?

Game engines need **predictable memory behavior**. `framealloc` makes allocation intent explicit — and fast.

Most engine allocations fall into predictable categories:
- **Frame-temporary**: Lives for one frame, then discarded
- **Small objects**: Pooled for reuse
- **Large/persistent**: Traditional heap

By expressing intent, `framealloc` routes allocations to the optimal path automatically.

## Quick Start

```rust
use framealloc::{SmartAlloc, AllocConfig};

fn main() {
    // Create allocator with default config
    let alloc = SmartAlloc::new(AllocConfig::default());

    // Game loop
    loop {
        alloc.begin_frame();

        // Frame-scoped allocation (bump allocator, ultra-fast)
        let temp_buffer = alloc.frame_alloc::<[f32; 1024]>();

        // Small object from pool
        let entity_data = alloc.pool_alloc::<EntityData>();

        // ... game logic ...

        alloc.end_frame(); // Resets frame arena
    }
}
```

## Bevy Integration

```rust
use bevy::prelude::*;
use framealloc::bevy::SmartAllocPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(SmartAllocPlugin::default())
        .run();
}

fn my_system(alloc: Res<framealloc::bevy::AllocResource>) {
    // Frame allocation automatically reset each frame
    let temp = alloc.frame_alloc::<TempData>();
}
```

## v0.4.0: Memory Behavior Filter

```rust
// Enable behavior tracking
alloc.enable_behavior_filter();

// Run your game loop...
for _ in 0..1000 {
    alloc.begin_frame();
    alloc.with_tag("physics", |a| { /* allocations */ });
    alloc.end_frame();
}

// Check for issues
let report = alloc.behavior_report();
for issue in &report.issues {
    eprintln!("{}", issue);
}
// [FA501] warning: frame allocation behaves like long-lived data
//   suggestion: Consider using pool_alloc() or scratch_pool()
```

Detects "bad memory" — allocations that violate their declared intent. Opt-in, zero overhead when disabled.

## v0.3.0: Frame Retention & Promotion

```rust
// Allocate with retention policy
let data = alloc.frame_retained::<NavMesh>(RetentionPolicy::PromoteToPool);

// At frame end, get promoted allocations
let result = alloc.end_frame_with_promotions();
```

Frame allocations can optionally "escape" by being promoted to pool, heap, or scratch at frame end.

## v0.2.0 Features

```rust
// Frame phases - profile memory per game system
alloc.begin_phase("physics");
alloc.end_phase();

// Checkpoints - rollback speculative allocations
let checkpoint = alloc.frame_checkpoint();
if failed { alloc.rollback_to(checkpoint); }

// Frame collections - bounded, cannot escape frame
let mut entities = alloc.frame_vec::<Entity>(128);

// Tagged allocations - attribute to subsystems
alloc.with_tag("ai", |a| a.frame_alloc::<Scratch>());

// Scratch pools - cross-frame reusable memory
let pool = alloc.scratch_pool("pathfinding");
```

## Feature Philosophy

`framealloc` is intentionally modular. You do **not** need every feature.

At its core, `framealloc` provides:
- Frame-based bump allocation
- Thread-local fast paths
- Automatic single → multi-thread scaling

Everything else is optional and opt-in.

| Feature | Use it if you need… |
|---------|---------------------|
| Frame arenas | Ultra-fast per-frame scratch memory |
| Pool allocator | Small objects with reuse |
| Groups | Bulk free (levels, scenes, subsystems) |
| Streaming allocator | Incremental asset loading |
| Budgets | Memory caps and pressure detection |
| Phases | Per-system profiling within frames |
| Checkpoints | Speculative allocation with rollback |
| Scratch pools | Cross-frame reusable memory |
| Diagnostics | Detect engine-level allocation mistakes |
| Bevy integration | Automatic frame resets in Bevy |

**No feature changes the core fast path unless explicitly enabled.**

## Cargo Features

| Feature | Description |
|---------|-------------|
| `bevy` | Bevy plugin integration |
| `parking_lot` | Use parking_lot for faster mutexes |
| `debug` | Memory poisoning, allocation backtraces |
| `tracy` | Tracy profiler integration |
| `nightly` | std::alloc::Allocator trait |
| `diagnostics` | Enhanced runtime diagnostics |

## Documentation

See [TECHNICAL.md](TECHNICAL.md) for comprehensive documentation.

## Performance

The allocation flow prioritizes speed:

1. **Frame arena**: Bump pointer, no locks, no atomics
2. **Local pools**: Thread-local free lists, O(1)
3. **Global refill**: Mutex-protected, rare
4. **System heap**: Fallback for large allocations

In practice, 90%+ of game allocations hit the frame arena path.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
