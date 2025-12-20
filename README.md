# framealloc

**Intent-aware, thread-smart memory allocation for Rust game engines.**

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

## Features

| Feature | Description |
|---------|-------------|
| `bevy` | Bevy plugin integration |
| `parking_lot` | Use parking_lot for faster mutexes |
| `debug` | Memory poisoning, allocation backtraces |

## Architecture

See [ARCHITECTURE.md](ARCHITECTURE.md) for internal design details.

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
