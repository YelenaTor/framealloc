# Changelog

All notable changes to `framealloc` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0] - 2025-12-21

### Added

#### Frame Retention & Promotion System

Frame allocations can now optionally "escape" the frame by being promoted to other allocators at frame end.

**Core API:**
```rust
// Allocate with retention policy
let data = alloc.frame_retained::<NavMesh>(RetentionPolicy::PromoteToPool);
data.calculate();

// At frame end, process promotions
let result = alloc.end_frame_with_promotions();
println!("Promoted {} bytes to pool", result.summary.promoted_pool_bytes);
```

**Retention Policies:**
| Policy | Destination | Use Case |
|--------|-------------|----------|
| `Discard` | None (default) | Temporary scratch data |
| `PromoteToPool` | Pool allocator | Reusable small objects |
| `PromoteToHeap` | Heap allocator | Long-lived data |
| `PromoteToScratch(name)` | Named scratch pool | Subsystem-specific data |

**Importance Levels (Semantic Sugar):**
```rust
// More intuitive API
let data = alloc.frame_with_importance::<Path>(Importance::Reusable);

// Mappings:
// Ephemeral  → Discard
// Reusable   → PromoteToPool
// Persistent → PromoteToHeap
// Scratch(n) → PromoteToScratch(n)
```

**Frame Summary Diagnostics:**
```rust
let result = alloc.end_frame_with_promotions();
let summary = result.summary;

println!("Discarded: {} bytes", summary.discarded_bytes);
println!("Promoted to pool: {} bytes", summary.promoted_pool_bytes);
println!("Promoted to heap: {} bytes", summary.promoted_heap_bytes);
println!("Failed: {} ({})", summary.failed_count, summary.failures_by_reason.budget_exceeded);
```

**Design Principles:**
- **Explicit**: Retention must be opted-in per allocation
- **Deterministic**: All decisions happen at `end_frame()` 
- **Bounded**: Subject to budgets and limits
- **No Magic**: This is NOT garbage collection

**Use Cases:**
- Pathfinding results that might be reused
- Computed data that proved useful
- Level-loading scratch that should persist
- AI state that outlives a single frame

---

## [0.2.1] - 2025-12-21

### Fixed

#### Thread Safety for Frame Collections

`FrameVec` and `FrameMap` are now explicitly `!Send` and `!Sync`.

**Problem:** Raw pointers are `Send` by default in Rust. This meant `FrameVec` 
could theoretically be moved across threads, which would be undefined behavior
since frame memory is thread-local.

**Fix:** Added `PhantomData<*const ()>` marker to both types, which makes them
neither `Send` nor `Sync`. This prevents accidental cross-thread usage at 
compile time.

```rust
// This now fails to compile (as it should):
let vec = alloc.frame_vec::<u32>(64);
std::thread::spawn(move || {
    vec.push(1); // ERROR: `FrameVec` cannot be sent between threads
});
```

This aligns with framealloc's thread-local storage model and prevents potential UB.

---

## [0.2.0] - 2025-12-21

### Added

#### Frame Phases
Named scopes within frames for profiling and diagnostics:
```rust
alloc.begin_frame();
alloc.begin_phase("physics");
// physics allocations tracked separately
alloc.end_phase();
alloc.begin_phase("render");
// render allocations tracked separately
alloc.end_phase();
alloc.end_frame();
```

**Use cases:**
- Profile memory usage per game system (physics, AI, rendering)
- Debug which phase is consuming the most memory
- Integrate with profiling tools (Tracy, Optick)

#### Frame Checkpoints
Save and restore points for speculative allocation:
```rust
let checkpoint = alloc.frame_checkpoint();
// speculative allocations
if operation_failed {
    alloc.rollback_to(checkpoint); // undo allocations
}
```

**Use cases:**
- Pathfinding with rollback on dead-ends
- Physics simulation with speculative contacts
- UI layout with try/fail patterns

#### Frame Collections
Bounded, frame-local collections that cannot escape the frame:
```rust
let mut entities = alloc.frame_vec::<Entity>(128);
entities.push(entity1);
entities.push(entity2);
// Freed automatically at end_frame()
```

**Types added:**
- `FrameVec<T>` - Fixed-capacity vector
- `FrameMap<K, V>` - Fixed-capacity hash map

**Use cases:**
- Temporary entity lists for spatial queries
- Frame-local lookup tables
- Scratch buffers for algorithms

#### Tagged Allocations
First-class allocation attribution:
```rust
alloc.with_tag("ai", |alloc| {
    let scratch = alloc.frame_alloc::<AIScratch>();
    // allocation attributed to "ai" tag
});
```

**Use cases:**
- Track memory by subsystem (rendering, physics, AI)
- Automatic budget attribution
- Better profiler integration

#### Scratch Pools
Cross-frame reusable memory pools:
```rust
let pool = alloc.scratch_pool("pathfinding");
let nodes = pool.alloc::<[PathNode; 1024]>();
// Use across multiple frames
pool.reset(); // Clear when done (e.g., on level unload)
```

**Use cases:**
- Pathfinding node storage
- Level-specific allocations
- Subsystem-owned scratch memory

#### Other Additions
- `frame_number()` method to get current frame count
- `PhaseGuard` for RAII-based phase scoping
- `CheckpointGuard` for automatic rollback on scope exit
- `TagGuard` for RAII-based tag scoping
- `ScratchRegistry` for managing named scratch pools
- `SpeculativeResult<T, E>` for speculative allocation results

### Changed
- `begin_frame()` now increments frame counter and resets phases
- `end_frame()` now resets phases automatically

### Fixed
- N/A

---

## [0.1.0] - 2025-12-20

### Added

#### Core Allocation
- **Frame arenas**: Ultra-fast bump allocation, reset per frame
- **Pool allocator**: Thread-local pools for small objects
- **Heap allocator**: System allocator wrapper for large objects
- **Thread-local fast paths**: Zero locks in common case
- **Automatic ST → MT scaling**: Same API for single and multi-threaded

#### Safe Wrapper Types
- `FrameBox<T>` - Frame-allocated box
- `FrameSlice<T>` - Frame-allocated slice
- `PoolBox<T>` - Pool-allocated box with auto-free
- `HeapBox<T>` - Heap-allocated box with auto-free

#### Advanced Features
- **Allocation groups**: Bulk free related allocations
- **Streaming allocator**: Incremental loading for large assets
- **Handle-based allocation**: Stable handles with relocation support
- **Memory budgets**: Per-tag limits with soft/hard thresholds

#### Diagnostics
- Diagnostic codes (FA001, FA002, etc.) for allocator-specific errors
- Runtime diagnostics with `fa_diagnostic!` macro
- Strict mode for CI (panic on errors/warnings)
- Build-time diagnostics via `build.rs`
- UI integration hooks for imgui/egui

#### Integrations
- **Bevy integration**: `SmartAllocPlugin` for automatic frame management
- **Tracy integration**: Memory profiling hooks (optional)
- **std::alloc::Allocator**: Nightly trait implementations

#### Documentation
- `TECHNICAL.md` with comprehensive documentation
- Example code for each feature
- Troubleshooting guide

### Use Cases

**Game Loop Pattern:**
```rust
let alloc = SmartAlloc::with_defaults();
loop {
    alloc.begin_frame();
    let scratch = alloc.frame_box(ScratchData::new())?;
    process(&scratch);
    alloc.end_frame(); // All frame memory freed
}
```

**Level Loading Pattern:**
```rust
let groups = alloc.groups();
let level_group = groups.create_group("level_1");
// Load assets into group
groups.free_group(level_group); // Free all on unload
```

**Streaming Assets Pattern:**
```rust
let streaming = alloc.streaming();
let id = streaming.reserve(size, StreamPriority::High)?;
// Load incrementally, access when ready
```

---

## Version History

| Version | Date | Highlights |
|---------|------|------------|
| 0.3.0 | 2025-12-21 | Frame retention & promotion system |
| 0.2.1 | 2025-12-21 | Thread safety fix for FrameVec/FrameMap (!Send/!Sync) |
| 0.2.0 | 2025-12-21 | Phases, checkpoints, frame collections, tags, scratch pools |
| 0.1.0 | 2025-12-20 | Initial release |
