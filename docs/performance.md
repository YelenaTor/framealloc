# framealloc Performance Guide

Optimize your framealloc usage for maximum performance.

## Table of Contents

1. [Understanding Performance](#understanding-performance)
2. [Benchmarking](#benchmarking)
3. [Optimization Techniques](#optimization-techniques)
4. [Memory Layout](#memory-layout)
5. [Threading Performance](#threading-performance)
6. [Platform-Specific Optimizations](#platform-specific-optimizations)

## Understanding Performance

### Performance Characteristics

framealloc provides different performance characteristics for each allocation type:

| Allocation Type | Speed | Use Case | Overhead |
|----------------|-------|----------|----------|
| Frame | ~4ns | Temporary data | None (bump pointer) |
| Pool | ~8ns | Small persistent | Free list management |
| Heap | ~50ns | Large objects | System allocator |
| Batch | ~0.06ns per item | Many small items | Single bookkeeping |

### When to Optimize

Don't optimize prematurely. Consider optimization when:
- Profile shows allocation overhead > 10% of frame time
- Allocating > 1000 items per frame
- Memory bandwidth is a bottleneck
- Cache miss rate is high

## Benchmarking

### Built-in Benchmarks

```rust
use framealloc::SmartAlloc;
use std::time::Instant;

fn benchmark_allocations() {
    let alloc = SmartAlloc::new(Default::default());
    const ITERATIONS: usize = 1_000_000;
    
    // Benchmark frame allocations
    let start = Instant::now();
    for _ in 0..ITERATIONS {
        alloc.begin_frame();
        let _data = alloc.frame_alloc::<u64>();
        alloc.end_frame();
    }
    let frame_time = start.elapsed();
    
    // Benchmark pool allocations
    let start = Instant::now();
    for _ in 0..ITERATIONS {
        let _data = alloc.pool_alloc::<u64>();
    }
    let pool_time = start.elapsed();
    
    // Benchmark batch allocations
    let start = Instant::now();
    alloc.begin_frame();
    let batch = unsafe { alloc.frame_alloc_batch::<u64>(ITERATIONS) };
    for i in 0..ITERATIONS {
        unsafe { batch.add(i); }
    }
    alloc.end_frame();
    let batch_time = start.elapsed();
    
    println!("Frame: {:?} ({:.2}ns per alloc)", frame_time, frame_time.as_nanos() as f64 / ITERATIONS as f64);
    println!("Pool: {:?} ({:.2}ns per alloc)", pool_time, pool_time.as_nanos() as f64 / ITERATIONS as f64);
    println!("Batch: {:?} ({:.2}ns per alloc)", batch_time, batch_time.as_nanos() as f64 / ITERATIONS as f64);
}
```

### Memory Usage Profiling

```rust
use framealloc::SmartAlloc;

fn profile_memory_usage() {
    let alloc = SmartAlloc::new(Default::default());
    
    alloc.begin_frame();
    
    // Simulate typical game frame
    let positions = alloc.frame_vec::<Vector3>();
    let velocities = alloc.frame_vec::<Vector3>();
    let indices = alloc.frame_vec::<u32>();
    
    for i in 0..10000 {
        positions.push(Vector3::new(i as f32, 0.0, 0.0));
        velocities.push(Vector3::new(0.0, i as f32, 0.0));
        indices.push(i as u32);
    }
    
    let stats = alloc.frame_stats();
    println!("Frame stats:");
    println!("  Allocations: {}", stats.allocation_count);
    println!("  Bytes allocated: {}", stats.bytes_allocated);
    println!("  Peak usage: {}", stats.peak_bytes);
    
    alloc.end_frame();
}
```

### Custom Benchmarks

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_frame_alloc(c: &mut Criterion) {
    let alloc = SmartAlloc::new(Default::default());
    
    c.bench_function("frame_alloc", |b| {
        b.iter(|| {
            alloc.begin_frame();
            let data = alloc.frame_alloc::<u64>();
            black_box(data);
            alloc.end_frame();
        })
    });
}

fn bench_batch_alloc(c: &mut Criterion) {
    let alloc = SmartAlloc::new(Default::default());
    
    c.bench_function("batch_alloc", |b| {
        b.iter(|| {
            alloc.begin_frame();
            let batch = unsafe { alloc.frame_alloc_batch::<u64>(1000) };
            for i in 0..1000 {
                black_box(unsafe { batch.add(i) });
            }
            alloc.end_frame();
        })
    });
}

criterion_group!(benches, bench_frame_alloc, bench_batch_alloc);
criterion_main!(benches);
```

## Optimization Techniques

### Use Batch Allocation

For many small allocations, batch is dramatically faster:

```rust
// Slow - individual allocations
fn process_particles_slow(alloc: &SmartAlloc, count: usize) {
    alloc.begin_frame();
    
    let mut particles = Vec::new();
    for _ in 0..count {
        particles.push(alloc.frame_alloc::<Particle>());
    }
    
    // Process particles...
    
    alloc.end_frame();
}

// Fast - batch allocation
fn process_particles_fast(alloc: &SmartAlloc, count: usize) {
    alloc.begin_frame();
    
    let particles = unsafe {
        let batch = alloc.frame_alloc_batch::<Particle>(count);
        for i in 0..count {
            let p = batch.add(i);
            std::ptr::write(p, Particle::new());
        }
        batch
    };
    
    // Process particles...
    
    alloc.end_frame();
}
```

### Pre-allocate with Capacity

Avoid repeated vector growth:

```rust
// Bad - grows multiple times
fn collect_bad(alloc: &SmartAlloc, items: &[Item]) -> FrameBox<Vec<Item>> {
    alloc.begin_frame();
    let mut result = alloc.frame_vec::<Item>();
    for item in items {
        result.push(*item); // May reallocate
    }
    alloc.end_frame();
    alloc.frame_box(result.into_inner())
}

// Good - pre-allocate capacity
fn collect_good(alloc: &SmartAlloc, items: &[Item]) -> FrameBox<Vec<Item>> {
    alloc.begin_frame();
    let mut result = alloc.frame_vec_with_capacity(items.len());
    result.extend_from_slice(items);
    alloc.end_frame();
    alloc.frame_box(result.into_inner())
}
```

### Use Specialized Sizes

For known small counts, use specialized methods:

```rust
// Generic - works for any size
fn pair_generic(alloc: &SmartAlloc) -> FrameBox<[u32; 2]> {
    alloc.begin_frame();
    let pair = alloc.frame_box([0, 1]);
    alloc.end_frame();
    pair
}

// Specialized - zero overhead
fn pair_specialized(alloc: &SmartAlloc) -> [u32; 2] {
    alloc.begin_frame();
    let [a, b] = alloc.frame_alloc_2::<u32>();
    *a = 0;
    *b = 1;
    alloc.end_frame();
    // Note: Can't return frame allocation from scope
    // Use within frame only
    [a, b] // This won't compile - see patterns.md for correct usage
}
```

### Minimize Tag Overhead

Tags add a small overhead. Use them judiciously:

```rust
// Too many tags - overhead
fn too_many_tags(alloc: &SmartAlloc) {
    alloc.with_tag("system1", |a| a.frame_alloc::<u8>());
    alloc.with_tag("system2", |a| a.frame_alloc::<u8>());
    // ... many more
}

// Better - group related allocations
fn better_tagging(alloc: &SmartAlloc) {
    alloc.with_tag("rendering", |a| {
        let vertices = a.frame_vec::<Vertex>();
        let indices = a.frame_vec::<u32>();
        let uniforms = a.frame_vec::<Uniform>();
        // Use all rendering data together
    });
}
```

## Memory Layout

### Cache-Friendly Structures

Align structures to cache lines for hot data:

```rust
#[repr(align(64))] // Cache line size
struct CacheAlignedParticle {
    position: Vector3,
    velocity: Vector3,
    _padding: [u8; 64 - 2 * std::mem::size_of::<Vector3>()],
}

// Use in frame allocation
alloc.begin_frame();
let particles = alloc.frame_slice::<CacheAlignedParticle>(1000);
alloc.end_frame();
```

### Structure of Arrays

Better cache locality for processing:

```rust
// Array of Structures (AoS) - bad for SIMD
struct ParticleAoS {
    positions: Vec<Vector3>,
    velocities: Vec<Vector3>,
    colors: Vec<Color>,
}

// Structure of Arrays (SoA) - good for SIMD
struct ParticleSoA {
    positions: Vec<Vector3>,
    velocities: Vec<Vector3>,
    colors: Vec<Color>,
}

impl ParticleSoA {
    fn new(count: usize, alloc: &SmartAlloc) -> Self {
        Self {
            positions: alloc.frame_vec_with_capacity(count),
            velocities: alloc.frame_vec_with_capacity(count),
            colors: alloc.frame_vec_with_capacity(count),
        }
    }
    
    fn update_positions(&mut self, dt: f32) {
        // SIMD-friendly - all positions contiguous
        for i in 0..self.positions.len() {
            self.positions[i] += self.velocities[i] * dt;
        }
    }
}
```

### Memory Prefetching

On x86_64, enable prefetch hints:

```rust
// In Cargo.toml
framealloc = { version = "0.10", features = ["prefetch"] }

// Code benefits automatically
fn process_with_prefetch(alloc: &SmartAlloc, data: &[f32]) {
    alloc.begin_frame();
    
    let result = alloc.frame_slice::<f32>(data.len());
    for (i, &value) in data.iter().enumerate() {
        result[i] = value * 2.0; // Prefetch helps with write-heavy patterns
    }
    
    alloc.end_frame();
}
```

## Threading Performance

### Thread-Local Caches

Each thread has its own cache for pools:

```rust
use framealloc::SmartAlloc;
use std::thread;

fn threaded_processing() {
    let alloc = SmartAlloc::new(Default::default());
    let mut handles = Vec::new();
    
    for _ in 0..8 {
        let alloc_clone = alloc.clone();
        let handle = thread::spawn(move || {
            // Each thread has its own pool cache
            alloc_clone.begin_frame();
            
            for _ in 0..10000 {
                let _data = alloc_clone.pool_alloc::<u64>();
                // Fast - no contention
            }
            
            alloc_clone.end_frame();
        });
        handles.push(handle);
    }
    
    for handle in handles {
        handle.join().unwrap();
    }
}
```

### NUMA Awareness

On NUMA systems, allocate close to where used:

```rust
// Enable NUMA awareness in config
let config = AllocConfig::default()
    .with_numa_awareness(true);

let alloc = SmartAlloc::new(config);

// Each thread gets allocator from its NUMA node
let thread_alloc = alloc.for_current_thread();
```

### Avoid False Sharing

Separate frequently accessed data:

```rust
// Bad - false sharing
#[repr(C)]
struct BadCounter {
    counter1: u64, // On same cache line
    counter2: u64, // On same cache line
}

// Good - separate cache lines
#[repr(align(64))]
struct GoodCounter {
    counter1: u64,
    _padding1: [u8; 64 - 8],
    counter2: u64,
    _padding2: [u8; 64 - 8],
}
```

## Platform-Specific Optimizations

### x86_64 Optimizations

```toml
# Enable all x86_64 optimizations
framealloc = { version = "0.10", features = [
    "prefetch",    # Hardware prefetch hints
    "minimal",     # Remove statistics overhead
    "simd",        # SIMD optimizations (when available)
] }
```

### ARM Optimizations

```rust
// ARM-specific cache line size
#[cfg(target_arch = "aarch64")]
const CACHE_LINE_SIZE: usize = 128;

#[cfg(target_arch = "arm")]
const CACHE_LINE_SIZE: usize = 64;

#[repr(align(CACHE_LINE_SIZE))]
struct AlignedData {
    data: [u8; 1024],
}
```

### WASM Considerations

```rust
// WASM has different performance characteristics
#[cfg(target_arch = "wasm32")]
fn wasm_optimized_alloc(alloc: &SmartAlloc) {
    // WASM benefits more from larger allocations
    // due to JavaScript GC overhead
    let data = alloc.frame_slice::<u8>(4096);
}
```

## Performance Checklist

### Before Optimizing

- [ ] Profile with realistic data
- [ ] Identify actual bottlenecks
- [ ] Measure baseline performance
- [ ] Set clear performance goals

### Optimization Techniques

- [ ] Use batch allocation for >100 items
- [ ] Pre-allocate vector capacities
- [ ] Align hot structures to cache lines
- [ ] Use SoA for SIMD-friendly data
- [ ] Enable prefetch for write-heavy patterns
- [ ] Minimize tag overhead
- [ ] Use thread-local caches effectively

### Platform Tuning

- [ ] Enable x86_64 prefetch hints
- [ ] Use minimal mode in production
- [ ] Consider NUMA for multi-socket systems
- [ ] Align to platform cache line size
- [ ] Use specialized size methods

### Validation

- [ ] Measure after each optimization
- [ ] Verify correctness with debug mode
- [ ] Test on target hardware
- [ ] Monitor memory usage
- [ ] Check for regressions

## Common Performance Pitfalls

### 1. Overusing Tags

```rust
// Bad - tag overhead for tiny allocations
alloc.with_tag("tiny", |a| a.frame_alloc::<u8>());

// Good - only tag substantial allocations
alloc.with_tag("particles", |a| {
    a.frame_slice::<Particle>(1000)
});
```

### 2. Mixing Allocation Types

```rust
// Bad - mixing breaks optimizations
let mut vec = alloc.frame_vec::<u32>();
vec.push(alloc.pool_alloc::<u32>()); // Type mismatch

// Good - consistent allocation type
let mut vec = alloc.frame_vec::<u32>();
vec.push(42); // Direct value
```

### 3. Premature Optimization

```rust
// Don't do this unless profiling shows it's needed
unsafe {
    let batch = alloc.frame_alloc_batch::<u8>(1);
    let item = batch.add(0);
    std::ptr::write(item, 42);
}

// Simple is better for small cases
let item = alloc.frame_alloc::<u8>();
```

## Performance Tools

### Built-in Profiling

```rust
// Enable performance tracking
alloc.enable_performance_tracking();

// Get detailed stats
let stats = alloc.performance_stats();
println!("Allocations per frame: {}", stats.avg_allocations);
println!("Bytes per frame: {}", stats.avg_bytes);
println!("Cache miss rate: {:.2}%", stats.cache_miss_rate * 100.0);
```

### External Tools

- `perf` (Linux) - Hardware counters
- VTune (Intel) - Deep profiling
- Instruments (macOS) - Time profiling
- Tracy - Real-time visualization

## Further Reading

- [Advanced Guide](advanced.md) - Deep internals
- [Cookbook](cookbook.md) - Performance recipes
- [Technical Documentation](../TECHNICAL.md) - Implementation details

Remember: Profile first, optimize second! 🚀
