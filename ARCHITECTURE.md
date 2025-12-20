# Architecture

## High-Level Design

```
SmartAlloc (Arc)
 ├── GlobalState (shared, synchronized)
 │    ├── SystemHeap (large allocations)
 │    ├── SlabRegistry (page pools per size class)
 │    ├── BudgetManager (optional limits)
 │    └── GlobalStats (aggregated metrics)
 │
 └── ThreadLocalState (TLS, per thread)
      ├── FrameArena (bump allocator)
      ├── LocalPools (small object caches)
      ├── DeferredFreeQueue (cross-thread frees)
      └── ThreadStats (local metrics)
```

**Key principle**: Allocation always tries thread-local first. Global state is a refill/escape hatch.

## Module Structure

```
src/
├── lib.rs              # Crate root, feature gates
├── api/                # Public API (only user-facing module)
│   ├── alloc.rs        # SmartAlloc public type
│   ├── scope.rs        # Frame scope guards
│   ├── tag.rs          # AllocationIntent enum
│   └── stats.rs        # Statistics API
├── core/               # Internal shared state
│   ├── global.rs       # GlobalState
│   ├── tls.rs          # ThreadLocalState
│   ├── config.rs       # AllocConfig
│   └── budget.rs       # BudgetManager
├── allocators/         # Allocation backends (unsafe boundary)
│   ├── frame.rs        # FrameArena
│   ├── slab.rs         # Slab allocator
│   ├── heap.rs         # System heap wrapper
│   └── deferred.rs     # Cross-thread free queues
├── sync/               # Synchronization primitives
│   ├── mutex.rs        # Mutex wrapper (std/parking_lot)
│   └── atomics.rs      # Atomic helpers
├── bevy/               # Bevy integration (feature-gated)
│   ├── plugin.rs       # SmartAllocPlugin
│   ├── resource.rs     # AllocResource
│   └── systems.rs      # Frame lifecycle systems
├── debug/              # Debug utilities (feature-gated)
│   ├── poison.rs       # Memory poisoning
│   └── backtrace.rs    # Allocation traces
└── util/               # Helpers
    ├── layout.rs       # Layout utilities
    └── size.rs         # Size helpers (kb, mb)
```

## Unsafe Boundaries

Only these modules contain `unsafe` code:

- `allocators/frame.rs` - Raw pointer manipulation for bump allocation
- `allocators/slab.rs` - Slab page management
- `allocators/heap.rs` - System allocator calls
- `allocators/deferred.rs` - Cross-thread pointer handling

Everything above this layer is safe Rust.

## Allocation Flow

### `frame_alloc<T>()`
1. Get TLS (thread-local state)
2. Bump FrameArena pointer
3. Return pointer (no locks, no atomics)

### `pool_alloc<T>()`
1. Get TLS
2. Pop from LocalPool free list
3. If empty: refill from SlabRegistry (mutex)
4. Return pointer

### `heap_alloc<T>()`
1. Call SystemHeap (mutex)
2. Return pointer

### Cross-thread free
1. Push to DeferredFreeQueue (lock-free)
2. Owning thread drains queue on next alloc

## Synchronization Strategy

| Component | Strategy | Rationale |
|-----------|----------|-----------|
| FrameArena | None | Single-threaded access via TLS |
| LocalPools | None | Single-threaded access via TLS |
| SlabRegistry | Mutex | Infrequent refill, large batches |
| SystemHeap | Mutex | Rare, large allocations |
| Cross-thread free | Lock-free queue | Avoids blocking freeing thread |
| GlobalStats | Atomics | Contention-free updates |
| BudgetManager | Atomics + Mutex | Read-heavy, rare writes |

## Thread Scaling

No explicit ST/MT mode switching. The design scales automatically:

- **Single-threaded**: One TLS instance, global locks never contended
- **Multi-threaded**: Each thread gets own TLS, only slab refill touches global

## Bevy Integration

The Bevy plugin:
1. Inserts `AllocResource` (wraps `SmartAlloc`)
2. Adds `begin_frame` system to `First` schedule
3. Adds `end_frame` system to `Last` schedule
4. Optionally integrates with `bevy_diagnostic`

Systems access via `Res<AllocResource>` - Bevy handles cloning across threads.
