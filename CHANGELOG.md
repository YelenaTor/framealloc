# Changelog

All notable changes to `framealloc` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.7.0] - 2025-12-22

### Added

#### IDE Integration: FA Insight Support

**Snapshot Emission** — Runtime snapshot support for IDE tooling:

```rust
use framealloc::{SnapshotConfig, SnapshotEmitter, Snapshot};

// Configure snapshots
let config = SnapshotConfig::default()
    .with_directory("target/framealloc")
    .with_max_snapshots(30);

let emitter = SnapshotEmitter::new(config);

// In your frame loop:
alloc.end_frame();
let snapshot = build_snapshot(&alloc, frame_number);
emitter.maybe_emit(&snapshot); // Checks for request file
```

Snapshots are:
- **Opt-in** — Only emitted when explicitly enabled
- **Aggregated** — No per-allocation data, only summaries
- **Bounded** — Rate-limited and cleaned up automatically
- **Safe boundary** — Only captured at frame end

**Snapshot Schema v1** — Structured JSON format for IDE consumption:

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

#### cargo-fa: JSON Report Format

New `generate_json_report()` function produces structured output for `fa-insight`:

```rust
// In cargo-fa
let report = generate_json_report(&diagnostics, files_analyzed, duration_ms);
println!("{}", report);
```

Output format matches fa-insight's expected schema with diagnostics array, 
summary statistics, and analysis metadata.

### Philosophy

v0.7.0 extends framealloc's observability to external tooling while maintaining core principles:

- **Opt-in** — Snapshot feature requires explicit enablement
- **Explicit** — Clear request/emit cycle, no background work
- **Zero-cost** — No overhead when snapshots disabled
- **Deterministic** — Snapshots only at frame boundaries

### Companion Tools

- **fa-insight** — VS Code extension for framealloc-aware IDE experience
  - Inline diagnostics from cargo-fa
  - Memory inspector sidebar panel  
  - Snapshot capture and visualization
  - Repository: https://github.com/YelenaTor/fa-insight

---

## [0.6.0] - 2025-12-21

### Added

#### Explicit Thread Coordination

**TransferHandle** - Declare cross-thread allocation intent explicitly:

```rust
// Allocate with transfer intent
let handle = alloc.frame_box_for_transfer(physics_result);

// Explicit handoff - visible cost, clear ownership
worker_channel.send(handle);

// Receiver explicitly accepts
let data = handle.receive();
```

**FrameBarrier** - Deterministic multi-thread frame synchronization:

```rust
// Create barrier for coordinated threads
let barrier = FrameBarrier::new(3);

// Each thread signals completion
barrier.signal_frame_complete();

// Coordinator waits, then resets
barrier.wait_all();
alloc.end_frame();
```

#### Per-Thread Frame Budgets

Explicit memory limits with deterministic exceeded behavior:

```rust
// Configure per-thread limits
alloc.set_thread_frame_budget(megabytes(8));
alloc.on_budget_exceeded(BudgetExceededPolicy::Fail);

// Check remaining budget
if alloc.frame_remaining() < size {
    // Handle gracefully
}
```

#### Deferred Processing Control

Explicit control over cross-thread free processing:

```rust
// Bounded queue prevents unbounded memory growth
let config = DeferredConfig::bounded(1024)
    .full_policy(QueueFullPolicy::ProcessImmediately);

// Or incremental processing (amortized cost)
let config = DeferredConfig::incremental(16); // Process 16 per alloc

// Or full manual control
alloc.set_deferred_processing(DeferredProcessing::Explicit);
alloc.process_deferred_frees(max_count: 64);
```

#### Frame Lifecycle Events (Opt-in)

Zero-overhead observability when enabled:

```rust
alloc.enable_lifecycle_tracking();

alloc.on_frame_event(|event| match event {
    FrameEvent::FrameBegin { thread_id, frame_number, .. } => { ... }
    FrameEvent::CrossThreadFreeQueued { from, to, size } => { ... }
    FrameEvent::FrameEnd { duration_us, peak_memory, .. } => { ... }
});

// Get per-thread statistics
let stats = alloc.thread_frame_stats(thread_id);
```

#### New Threading Diagnostics (FA2xx)

| Code | Severity | Description |
|------|----------|-------------|
| FA201 | Error | Cross-thread frame access without transfer |
| FA202 | Warning | Thread not in FrameBarrier but shares boundary |
| FA203 | Hint | Thread allocates without budget configured |
| FA204 | Warning | Pattern may overflow deferred queue |
| FA205 | Error | end_frame() without barrier sync |

### Changed

- **Deferred free queue** now supports bounded capacity with configurable overflow policy
- **Check ordering** optimized: architecture → dirtymem → budgets → async → threading

### Philosophy

v0.6.0 stays true to framealloc's core principles:

| Principle | How v0.6.0 Honors It |
|-----------|---------------------|
| **Deterministic** | Bounded queues, explicit barriers |
| **Frame-based** | All features center on frame lifecycle |
| **Explicit** | TransferHandle, budget policies, manual processing |
| **Predictable** | No hidden costs, configurable overhead |
| **Scales ST→MT** | Zero cost when features unused |

---

## [0.5.1] - 2025-12-21

### Added

#### Extended Output Formats
- **JUnit XML** (`--format junit`) - For test reporting systems
- **Checkstyle XML** (`--format checkstyle`) - For Jenkins and legacy CI

#### Filtering Options
- `--deny <CODE>` - Treat specific lint as error
- `--allow <CODE>` - Suppress specific lint
- `--exclude <PATTERN>` - Exclude paths from analysis (glob)
- `--fail-fast` - Stop on first error

#### Subcommands
- `cargo fa explain FA601` - Detailed explanation with examples
- `cargo fa show src/file.rs` - Single file analysis
- `cargo fa list` - List all diagnostic codes
- `cargo fa init` - Generate `.fa.toml` configuration

#### Optimized Check Ordering
`--all` now runs checks in optimized order (fast checks first) for better fail-fast behavior.

### Changed
- Check execution order optimized for `--all` flag
- Quiet mode for non-terminal output formats

---

## [0.5.0] - 2025-12-21

### Added

#### `cargo fa` - Static Analysis Tool

A cargo subcommand that detects memory intent violations before runtime by analyzing source code.

**Installation:**
```bash
cd cargo-fa && cargo install --path .
```

**Usage:**
```bash
# Check for dirty memory patterns
cargo fa --dirtymem

# Check async safety
cargo fa --async-safety

# Check threading issues
cargo fa --threading

# Run all checks
cargo fa --all

# Output for CI (GitHub Actions)
cargo fa --all --format sarif
```

**Detected Issues:**

| Code | Category | Description |
|------|----------|-------------|
| FA601 | Lifetime | Frame allocation escapes scope |
| FA602 | Lifetime | Allocation in hot loop |
| FA603 | Lifetime | Missing frame boundaries |
| FA604 | Lifetime | Retention policy mismatch |
| FA605 | Lifetime | Discard policy but stored beyond frame |
| FA701 | Async | Frame allocation in async function |
| FA702 | Async | Frame allocation crosses await point |
| FA703 | Async | FrameBox captured by closure/task |
| FA801 | Architecture | Tag mismatch |
| FA802 | Architecture | Unknown tag |
| FA803 | Architecture | Cross-module allocation |

**Example Output:**
```
error[FA701]: frame allocation in async function
  --> src/network.rs:45:13
   |
45 |             let data = alloc.frame_box(packet);
   |                       ^^^^^^^^^^^^^^^^^^^^^^^^^
   |
   = note: async functions may suspend across frame boundaries
   = help: use pool_box() or heap_box() for data in async contexts
   = see: https://docs.rs/framealloc/diagnostics#FA701
```

**Configuration (`.fa.toml`):**
```toml
[global]
min_severity = "hint"
deny_warnings = false

[lints.levels]
# FA602 = "allow"  # Disable loop allocation warnings

[tags]
known_tags = ["physics", "rendering", "ai"]
warn_unknown_tags = true

[thresholds]
loop_allocation_limit = 100
```

**Design Principles:**
- **Not clippy**: Domain-specific rules for frame allocation
- **Actionable**: Every issue includes suggestions
- **CI-ready**: SARIF output for GitHub Actions
- **Configurable**: Per-lint levels, thresholds

---

## [0.4.0] - 2025-12-21

### Added

#### Memory Behavior Filter

Runtime detection of allocation pattern issues — "bad memory" is memory that violates declared intent.

**Detected Issues:**
| Code | Issue | Description |
|------|-------|-------------|
| FA501 | Frame survives too long | Frame allocation avg lifetime > threshold |
| FA502 | High survival rate | Too many frame allocations survive beyond frame |
| FA510 | Pool as scratch | Pool allocations freed same frame (use frame_alloc) |
| FA520 | Promotion churn | Excessive promotions per frame |
| FA530 | Heap in hot path | Frequent heap allocations (use pool/frame) |

**Usage:**
```rust
// Enable behavior tracking
alloc.enable_behavior_filter();

// Run your game loop...
for _ in 0..1000 {
    alloc.begin_frame();
    // ... allocations with tags ...
    alloc.end_frame();
}

// Analyze and report issues
let report = alloc.behavior_report();
for issue in &report.issues {
    eprintln!("{}", issue);
}

// Output example:
// [FA501] warning: frame allocation behaves like long-lived data
//   tag: ai::pathfinding
//   observed: avg lifetime: 128.0 frames
//   threshold: expected < 60 frames
//   suggestion: Consider using pool_alloc() or scratch_pool()
```

**Configurable Thresholds:**
```rust
// Strict for CI
BehaviorThresholds::strict()

// Relaxed for development
BehaviorThresholds::relaxed()

// Custom
BehaviorThresholds {
    frame_survival_frames: 120,
    frame_survival_rate: 0.3,
    ..Default::default()
}
```

**Design Principles:**
- **Opt-in**: Disabled by default, zero overhead when off
- **Per-tag tracking**: O(tags) memory, not O(allocations)
- **Actionable**: Every issue includes a suggestion
- **Not a cop**: Advises, doesn't block

#### Enhanced Build-time Advisor

- Async runtime detection (warns about frame allocs across await)
- Memory filter feature guidance
- Ecosystem-aware suggestions

---

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
| 0.6.0 | 2025-12-21 | Thread coordination, barriers, budgets, lifecycle events |
| 0.5.1 | 2025-12-21 | Extended formats, filtering, subcommands |
| 0.5.0 | 2025-12-21 | `cargo fa` static analysis tool |
| 0.4.0 | 2025-12-21 | Memory behavior filter, async detection, build advisor |
| 0.3.0 | 2025-12-21 | Frame retention & promotion system |
| 0.2.1 | 2025-12-21 | Thread safety fix for FrameVec/FrameMap (!Send/!Sync) |
| 0.2.0 | 2025-12-21 | Phases, checkpoints, frame collections, tags, scratch pools |
| 0.1.0 | 2025-12-20 | Initial release |
