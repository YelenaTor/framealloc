# framealloc Advanced Guide

Deep dive into framealloc internals and advanced techniques (20-100 hours experience).

## Table of Contents

1. [Internal Architecture](#internal-architecture)
2. [Custom Allocators](#custom-allocators)
3. [Memory Layout Optimization](#memory-layout-optimization)
4. [Advanced Threading](#advanced-threading)
5. [Instrumentation and Debugging](#instrumentation-and-debugging)
6. [Integration Patterns](#integration-patterns)
7. [Performance Profiling](#performance-profiling)

## Internal Architecture

### Allocator Hierarchy

framealloc uses a three-tier architecture:

```
SmartAlloc
├── GlobalState (shared, atomic)
│   ├── PoolManager (thread-safe pools)
│   ├── BudgetManager (limits and tracking)
│   └── Statistics (global metrics)
└── ThreadLocalState (per-thread, lock-free)
    ├── FrameArena (bump allocator)
    ├── LocalPool (thread-local cache)
    └── FrameMetrics (per-thread stats)
```

### Frame Arena Internals

The frame arena is a bump allocator with chunked growth:

```rust
struct FrameArena {
    chunks: Vec<Chunk>,
    current: *mut u8,
    end: *mut u8,
    total_allocated: usize,
}

struct Chunk {
    ptr: NonNull<u8>,
    size: usize,
    // Backing allocation (pool or system)
}
```

Growth strategy:
1. Start with 64KB chunk
2. Double size up to 1MB
3. Fixed 1MB chunks thereafter
4. Chunks returned to pool at frame end

### Pool Management

Pools use size classes and per-thread caches:

```rust
struct PoolManager {
    // Global pools for fallback
    global_pools: [Mutex<FreeList>; NUM_SIZE_CLASSES],
    // Per-thread caches
    thread_caches: ThreadLocal<Cache>,
}

struct Cache {
    size_class: usize,
    local: Vec<NonNull<u8>>,
    limit: usize,
}
```

Size classes are powers of two from 8 bytes to 4KB.

## Custom Allocators

### Implementing a Custom Backend

```rust
use framealloc::{AllocatorBackend, AllocationResult};

struct CustomBackend {
    // Your custom state
}

impl AllocatorBackend for CustomBackend {
    fn allocate(&mut self, layout: Layout) -> AllocationResult {
        // Implement your allocation strategy
        if layout.size() <= 4096 {
            // Use custom allocator
            AllocationResult::Ok(ptr)
        } else {
            // Fall back to system
            AllocationResult::Fallback
        }
    }
    
    fn deallocate(&mut self, ptr: NonNull<u8>, layout: Layout) {
        // Implement deallocation
    }
}

// Use with SmartAlloc
let config = AllocConfig::default()
    .with_backend(Box::new(CustomBackend::new()));
let alloc = SmartAlloc::new(config);
```

### Custom Memory Sources

```rust
struct MmapBackend {
    mappings: Vec<Mmap>,
}

impl MmapBackend {
    fn new() -> Self {
        Self {
            mappings: Vec::new(),
        }
    }
    
    fn reserve(&mut self, size: usize) -> Result<NonNull<u8>, Error> {
        let mmap = unsafe {
            Mmap::map_anon(size)?
        };
        let ptr = mmap.as_ptr() as *mut u8;
        self.mappings.push(mmap);
        Ok(NonNull::new(ptr).unwrap())
    }
}
```

### Arena Customization

```rust
struct CustomArena {
    allocator: System,
    chunk_size: usize,
    alignment: usize,
}

impl CustomArena {
    fn new(chunk_size: usize, alignment: usize) -> Self {
        Self {
            allocator: System,
            chunk_size,
            alignment,
        }
    }
    
    fn allocate_chunk(&mut self) -> Result<NonNull<u8>, Error> {
        let layout = Layout::from_size_align(
            self.chunk_size,
            self.alignment,
        )?;
        unsafe {
            let ptr = self.allocator.alloc(layout)?;
            Ok(NonNull::new_unchecked(ptr))
        }
    }
}
```

## Memory Layout Optimization

### Cache-Line Alignment

```rust
#[repr(align(64))] // Cache line size
struct CacheAligned<T> {
    data: T,
    _padding: [u8; 64],
}

// Usage in frame allocation
alloc.begin_frame();
let aligned_data = alloc.frame_alloc::<CacheAligned<ParticleData>>();
alloc.end_frame();
```

### Structure of Arrays (SoA)

```rust
// Instead of Array of Structures (AoS):
struct Particle {
    position: Vector3,
    velocity: Vector3,
    color: Color,
    mass: f32,
}

// Use Structure of Arrays (SoA):
struct Particles {
    positions: Vec<Vector3>,
    velocities: Vec<Vector3>,
    colors: Vec<Color>,
    masses: Vec<f32>,
}

impl Particles {
    fn new(count: usize, alloc: &SmartAlloc) -> Self {
        Self {
            positions: alloc.frame_vec_with_capacity(count),
            velocities: alloc.frame_vec_with_capacity(count),
            colors: alloc.frame_vec_with_capacity(count),
            masses: alloc.frame_vec_with_capacity(count),
        }
    }
}
```

### Batch Processing

```rust
struct BatchProcessor<T> {
    batch_size: usize,
    batches: Vec<Batch<T>>,
}

struct Batch<T> {
    items: *mut T,
    count: usize,
}

impl<T> BatchProcessor<T> {
    fn process_all<F>(&mut self, mut f: F) 
    where 
        F: FnMut(&mut [T]),
    {
        for batch in &mut self.batches {
            unsafe {
                f(std::slice::from_raw_parts_mut(
                    batch.items,
                    batch.count,
                ));
            }
        }
    }
}
```

## Advanced Threading

### Work-Stealing Queue

```rust
struct WorkStealingQueue<T> {
    local: VecDeque<T>,
    stolen: Arc<Mutex<VecDeque<T>>>,
}

impl<T> WorkStealingQueue<T> {
    fn push_local(&mut self, item: T) {
        self.local.push_back(item);
    }
    
    fn pop_local(&mut self) -> Option<T> {
        self.local.pop_front()
    }
    
    fn steal(&self) -> Option<T> {
        let mut stolen = self.stolen.lock().unwrap();
        stolen.pop_front()
    }
}
```

### NUMA-Aware Allocation

```rust
struct NumaAwareAlloc {
    nodes: Vec<SmartAlloc>,
    current_node: usize,
}

impl NumaAwareAlloc {
    fn new() -> Result<Self, Error> {
        let mut nodes = Vec::new();
        for node in 0..numa::num_configured_cpus()? {
            let config = AllocConfig::default()
                .with_numa_node(node);
            nodes.push(SmartAlloc::new(config));
        }
        
        Ok(Self {
            nodes,
            current_node: 0,
        })
    }
    
    fn allocate_on_node<T>(&mut self, node: usize) -> FrameBox<T> {
        self.nodes[node].frame_box()
    }
}
```

### Lock-Free Statistics

```rust
use std::sync::atomic::{AtomicU64, Ordering};

struct LockFreeStats {
    allocations: AtomicU64,
    deallocations: AtomicU64,
    bytes_allocated: AtomicU64,
    peak_usage: AtomicU64,
}

impl LockFreeStats {
    fn record_allocation(&self, size: usize) {
        self.allocations.fetch_add(1, Ordering::Relaxed);
        self.bytes_allocated.fetch_add(size, Ordering::Relaxed);
        
        // Update peak (race condition acceptable for stats)
        let current = self.bytes_allocated.load(Ordering::Relaxed);
        loop {
            let peak = self.peak_usage.load(Ordering::Relaxed);
            if current <= peak {
                break;
            }
            if self.peak_usage.compare_exchange_weak(
                peak, 
                current, 
                Ordering::Relaxed, 
                Ordering::Relaxed
            ).is_ok() {
                break;
            }
        }
    }
}
```

## Instrumentation and Debugging

### Memory Tracing

```rust
#[cfg(feature = "debug")]
struct MemoryTracer {
    allocations: HashMap<*const u8, AllocationInfo>,
    stack_traces: bool,
}

#[derive(Debug)]
struct AllocationInfo {
    size: usize,
    backtrace: Backtrace,
    thread: ThreadId,
    timestamp: Instant,
}

impl MemoryTracer {
    fn trace_allocation(&mut self, ptr: *const u8, size: usize) {
        if self.stack_traces {
            let info = AllocationInfo {
                size,
                backtrace: Backtrace::new(),
                thread: thread::current().id(),
                timestamp: Instant::now(),
            };
            self.allocations.insert(ptr, info);
        }
    }
    
    fn find_leaks(&self) -> Vec<&AllocationInfo> {
        self.allocations.values()
            .filter(|info| info.age() > Duration::from_secs(60))
            .collect()
    }
}
```

### Memory Poisoning

```rust
#[cfg(feature = "debug")]
struct PoisonedMemory<T> {
    data: MaybeUninit<T>,
    poison: u64,
}

impl<T> PoisonedMemory<T> {
    fn new(value: T) -> Self {
        Self {
            data: MaybeUninit::new(value),
            poison: 0xDEADBEEFCAFEBABE,
        }
    }
    
    fn get(&self) -> &T {
        assert_eq!(self.poison, 0xDEADBEEFCAFEBABE);
        unsafe { self.data.assume_init_ref() }
    }
    
    fn get_mut(&mut self) -> &mut T {
        assert_eq!(self.poison, 0xDEADBEEFCAFEBABE);
        unsafe { self.data.assume_init_mut() }
    }
}
```

### Allocation Guard

```rust
struct AllocationGuard<'a, T> {
    data: *mut T,
    allocator: &'a SmartAlloc,
    magic: u64,
}

const MAGIC: u64 = 0xFRA_ME_AL_LOC_ATION;

impl<'a, T> AllocationGuard<'a, T> {
    fn new(allocator: &'a SmartAlloc, data: *mut T) -> Self {
        // Write canaries
        unsafe {
            let canary = MAGIC as *mut u8;
            ptr::write(data.offset(-1) as *mut u64, MAGIC);
            ptr::write(data.offset(1) as *mut u64, MAGIC);
        }
        
        Self {
            data,
            allocator,
            magic: MAGIC,
        }
    }
}

impl<'a, T> Drop for AllocationGuard<'a, T> {
    fn drop(&mut self) {
        // Check canaries
        unsafe {
            let start_canary = ptr::read(self.data.offset(-1) as *const u64);
            let end_canary = ptr::read(self.data.offset(1) as *const u64);
            
            assert_eq!(start_canary, MAGIC, "Buffer underflow detected");
            assert_eq!(end_canary, MAGIC, "Buffer overflow detected");
        }
    }
}
```

## Integration Patterns

### ECS Integration

```rust
trait FrameallocComponent {
    fn allocate_frame(alloc: &SmartAlloc) -> Self;
}

#[derive(Component)]
struct Transform {
    position: Vector3,
    rotation: Quaternion,
}

impl FrameallocComponent for Transform {
    fn allocate_frame(alloc: &SmartAlloc) -> Self {
        Self {
            position: *alloc.frame_alloc(),
            rotation: *alloc.frame_alloc(),
        }
    }
}

// Usage in system
fn update_transforms_system(
    mut query: Query<&mut Transform>,
    alloc: Res<SmartAlloc>,
) {
    alloc.begin_frame();
    
    for transform in &mut query {
        *transform = Transform::allocate_frame(&alloc);
    }
    
    alloc.end_frame();
}
```

### Renderer Integration

```rust
struct FrameRenderer {
    command_buffer: FrameBox<CommandBuffer>,
    uniform_buffers: HashMap<String, FrameBox<UniformBuffer>>,
    vertex_buffers: Vec<FrameBox<VertexBuffer>>,
}

impl FrameRenderer {
    fn new(alloc: &SmartAlloc) -> Self {
        Self {
            command_buffer: alloc.frame_box(CommandBuffer::new()),
            uniform_buffers: HashMap::new(),
            vertex_buffers: Vec::new(),
        }
    }
    
    fn begin_frame(&mut self, alloc: &SmartAlloc) {
        alloc.begin_frame();
        
        // Reset for new frame
        self.command_buffer = alloc.frame_box(CommandBuffer::new());
        self.uniform_buffers.clear();
        self.vertex_buffers.clear();
    }
    
    fn end_frame(self, alloc: &SmartAlloc) {
        // Submit all commands
        self.command_buffer.submit();
        
        alloc.end_frame();
        // Everything automatically freed
    }
}
```

### Physics Integration

```rust
struct PhysicsFrame {
    contacts: FrameBox<[Contact]>,
    manifolds: FrameBox<[Manifold]>,
    impulses: FrameBox<[Impulse]>,
    query_results: FrameBox<[RaycastHit]>,
}

impl PhysicsFrame {
    fn new(alloc: &SmartAlloc, max_contacts: usize) -> Self {
        Self {
            contacts: alloc.frame_slice(max_contacts),
            manifolds: alloc.frame_slice(max_contacts / 2),
            impulses: alloc.frame_slice(max_contacts),
            query_results: alloc.frame_slice(1000),
        }
    }
}
```

## Performance Profiling

### Custom Metrics

```rust
struct PerformanceMetrics {
    frame_times: VecDeque<Duration>,
    allocation_sizes: VecDeque<usize>,
    allocation_counts: VecDeque<usize>,
    peak_memory: usize,
}

impl PerformanceMetrics {
    fn record_frame(&mut self, frame_time: Duration, alloc_stats: &FrameStats) {
        self.frame_times.push_back(frame_time);
        self.allocation_sizes.push_back(alloc_stats.bytes_allocated);
        self.allocation_counts.push_back(alloc_stats.allocation_count);
        
        // Keep only last 60 seconds
        if self.frame_times.len() > 60 {
            self.frame_times.pop_front();
            self.allocation_sizes.pop_front();
            self.allocation_counts.pop_front();
        }
        
        self.peak_memory = self.peak_memory.max(alloc_stats.bytes_allocated);
    }
    
    fn generate_report(&self) -> PerformanceReport {
        PerformanceReport {
            avg_frame_time: self.frame_times.iter().sum::<Duration>() / self.frame_times.len() as u32,
            avg_allocations: self.allocation_counts.iter().sum::<usize>() / self.allocation_counts.len(),
            peak_memory: self.peak_memory,
            allocation_efficiency: self.calculate_efficiency(),
        }
    }
}
```

### Hot Path Analysis

```rust
#[cfg(feature = "profiling")]
struct HotPathAnalyzer {
    allocation_sites: HashMap<*mut u8, AllocationSite>,
    call_stack: Vec<*mut u8>,
}

#[derive(Debug)]
struct AllocationSite {
    address: *mut u8,
    size: usize,
    call_stack: Vec<usize>,
    frequency: usize,
}

impl HotPathAnalyzer {
    fn record_allocation(&mut self, ptr: *mut u8, size: usize) {
        let site = AllocationSite {
            address: ptr,
            size,
            call_stack: self.capture_call_stack(),
            frequency: 0,
        };
        
        self.allocation_sites.insert(ptr, site);
    }
    
    fn find_hot_spots(&self, threshold: f64) -> Vec<&AllocationSite> {
        let total: usize = self.allocation_sites.values()
            .map(|s| s.frequency)
            .sum();
        
        self.allocation_sites.values()
            .filter(|s| s.frequency as f64 / total as f64 > threshold)
            .collect()
    }
}
```

### Memory Bandwidth Analysis

```rust
struct BandwidthAnalyzer {
    reads: AtomicU64,
    writes: AtomicU64,
    start_time: Instant,
}

impl BandwidthAnalyzer {
    fn record_read(&self, bytes: usize) {
        self.reads.fetch_add(bytes as u64, Ordering::Relaxed);
    }
    
    fn record_write(&self, bytes: usize) {
        self.writes.fetch_add(bytes as u64, Ordering::Relaxed);
    }
    
    fn get_bandwidth(&self) -> (f64, f64) {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        let read_mb = self.reads.load(Ordering::Relaxed) as f64 / (1024.0 * 1024.0);
        let write_mb = self.writes.load(Ordering::Relaxed) as f64 / (1024.0 * 1024.0);
        
        (read_mb / elapsed, write_mb / elapsed)
    }
}
```

## Best Practices for Advanced Users

1. **Profile before optimizing** - Use built-in metrics
2. **Consider NUMA topology** - Allocate close to where used
3. **Align to cache lines** - For hot data structures
4. **Use batch APIs** - For many small allocations
5. **Monitor fragmentation** - Especially with pools
6. **Implement custom backends** - For special hardware
7. **Use debug features** - During development
8. **Measure real impact** - Don't optimize prematurely

## Further Reading

- [Performance Guide](performance.md) - Detailed optimization
- [Technical Documentation](../TECHNICAL.md) - Implementation details
- [Source Code](https://github.com/YelenaTor/framealloc) - Reference implementation
