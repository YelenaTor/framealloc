# framealloc Patterns Guide

This guide covers common patterns and best practices for using framealloc effectively (2-20 hours experience).

## Table of Contents

1. [Frame Allocation Patterns](#frame-allocation-patterns)
2. [Pool Allocation Patterns](#pool-allocation-patterns)
3. [Threading Patterns](#threading-patterns)
4. [Organization Patterns](#organization-patterns)
5. [Lifecycle Patterns](#lifecycle-patterns)
6. [Error Handling Patterns](#error-handling-patterns)

## Frame Allocation Patterns

### The Scratch Buffer Pattern

Most common pattern - temporary data per frame:

```rust
struct FrameScratch {
    positions: Vec<Vector3>,
    velocities: Vec<Vector3>,
    forces: Vec<Vector3>,
    indices: Vec<u32>,
}

impl FrameScratch {
    fn new(alloc: &SmartAlloc) -> Self {
        Self {
            positions: alloc.frame_vec(),
            velocities: alloc.frame_vec(),
            forces: alloc.frame_vec(),
            indices: alloc.frame_vec(),
        }
    }
}

// Usage
alloc.begin_frame();
let scratch = FrameScratch::new(&alloc);
// Fill with data...
process_physics(&scratch);
alloc.end_frame(); // Everything freed automatically
```

### The Builder Pattern

Build complex structures with frame allocations:

```rust
struct MeshBuilder<'a> {
    vertices: &'a mut Vec<Vertex>,
    indices: &'a mut Vec<u32>,
    normals: &'a mut Vec<Vector3>,
}

impl<'a> MeshBuilder<'a> {
    fn new(alloc: &'a SmartAlloc) -> Self {
        Self {
            vertices: alloc.frame_vec(),
            indices: alloc.frame_vec(),
            normals: alloc.frame_vec(),
        }
    }
    
    fn add_vertex(&mut self, pos: Vector3) -> &mut Self {
        self.vertices.push(Vertex::new(pos));
        self
    }
    
    fn build(self) -> Mesh {
        Mesh::new(self.vertices, self.indices)
    }
}

// Usage
alloc.begin_frame();
let mesh = MeshBuilder::new(&alloc)
    .add_vertex(Vector3::new(0, 0, 0))
    .add_vertex(Vector3::new(1, 0, 0))
    .add_vertex(Vector3::new(0, 1, 0))
    .build();
alloc.end_frame();
```

### The Temporary View Pattern

Create temporary views without copying:

```rust
struct SubSlice<'a, T> {
    data: &'a [T],
    offset: usize,
    len: usize,
}

impl<'a, T> SubSlice<'a, T> {
    fn from_slice(slice: &'a [T], range: Range<usize>) -> Self {
        Self {
            data: &slice[range],
            offset: range.start,
            len: range.end - range.start,
        }
    }
}

// Usage with frame allocation
alloc.begin_frame();
let all_vertices = alloc.frame_slice::<Vertex>(1000);
let visible_subset = SubSlice::from_slice(all_vertices, 100..200);
process_visible(&visible_subset);
alloc.end_frame();
```

## Pool Allocation Patterns

### The Component Pool Pattern

Game entities with pooled components:

```rust
#[derive(Component)]
struct Transform {
    position: Vector3,
    rotation: Quaternion,
}

#[derive(Component)]
struct Velocity {
    vector: Vector3,
}

struct EntityWorld {
    transforms: PoolHandle<Transform>,
    velocities: PoolHandle<Velocity>,
    // ... other component pools
}

impl EntityWorld {
    fn new(alloc: &SmartAlloc) -> Self {
        Self {
            transforms: alloc.pool_handle(),
            velocities: alloc.pool_handle(),
        }
    }
    
    fn spawn_entity(&mut self, pos: Vector3) -> EntityId {
        let id = EntityId::new();
        self.transforms.insert(id, Transform::new(pos));
        id
    }
}
```

### The Cache Pattern

Pool as a cache for frequently reused objects:

```rust
struct ObjectCache<T> {
    available: Vec<T>,
    in_use: HashSet<usize>,
}

impl<T: Default> ObjectCache<T> {
    fn get(&mut self, alloc: &SmartAlloc) -> PoolHandle<T> {
        if let Some(obj) = self.available.pop() {
            alloc.pool_box(obj)
        } else {
            alloc.pool_box(T::default())
        }
    }
    
    fn return_object(&mut self, obj: T) {
        self.available.push(obj);
    }
}
```

### The Resource Pool Pattern

Limited resources with pool tracking:

```rust
struct ResourcePool<T> {
    max_size: usize,
    current_size: usize,
    pool: PoolHandle<T>,
}

impl<T> ResourcePool<T> {
    fn acquire(&mut self, alloc: &SmartAlloc) -> Option<PoolHandle<T>> {
        if self.current_size < self.max_size {
            self.current_size += 1;
            Some(alloc.pool_box(T::new()))
        } else {
            None
        }
    }
    
    fn release(&mut self, _handle: PoolHandle<T>) {
        self.current_size -= 1;
    }
}
```

## Threading Patterns

### The Job System Pattern

Frame allocations in worker threads:

```rust
struct JobSystem {
    alloc: SmartAlloc,
    workers: Vec<JoinHandle<()>>,
}

impl JobSystem {
    fn dispatch_jobs(&mut self, jobs: Vec<Job>) {
        for job in jobs {
            let alloc_clone = self.alloc.clone();
            let handle = thread::spawn(move || {
                alloc_clone.begin_frame();
                job.execute(&alloc_clone);
                alloc_clone.end_frame();
            });
            self.workers.push(handle);
        }
        
        // Wait for all jobs
        for handle in self.workers.drain(..) {
            handle.join().unwrap();
        }
    }
}
```

### The Producer-Consumer Pattern

Cross-thread data transfer:

```rust
fn producer_thread(alloc: SmartAlloc, sender: Sender<TransferHandle<Data>>) {
    loop {
        alloc.begin_frame();
        
        // Produce data
        let data = alloc.frame_box(Data::generate());
        
        // Transfer to consumer
        let handle = alloc.frame_box_for_transfer(data);
        sender.send(handle).unwrap();
        
        alloc.end_frame();
    }
}

fn consumer_thread(receiver: Receiver<TransferHandle<Data>>) {
    while let Ok(handle) = receiver.recv() {
        let data = handle.receive();
        process_data(&data);
        // Data automatically freed when dropped
    }
}
```

### The Thread-Local Pattern

Each thread maintains its own allocator state:

```rust
thread_local! {
    static THREAD_ALLOC: RefCell<SmartAlloc> = RefCell::new(SmartAlloc::new(Default::default()));
}

fn process_on_thread() {
    THREAD_ALLOC.with(|alloc| {
        let mut alloc = alloc.borrow_mut();
        alloc.begin_frame();
        
        let local_data = alloc.frame_vec::<WorkItem>();
        // Process work...
        
        alloc.end_frame();
    });
}
```

## Organization Patterns

### The Tag Pattern

Organize allocations by subsystem:

```rust
struct SystemAllocators {
    physics: TaggedAllocator,
    rendering: TaggedAllocator,
    audio: TaggedAllocator,
    ai: TaggedAllocator,
}

impl SystemAllocators {
    fn new(alloc: &SmartAlloc) -> Self {
        Self {
            physics: TaggedAllocator::new(alloc, "physics"),
            rendering: TaggedAllocator::new(alloc, "rendering"),
            audio: TaggedAllocator::new(alloc, "audio"),
            ai: TaggedAllocator::new(alloc, "ai"),
        }
    }
}

// Usage
let systems = SystemAllocators::new(&alloc);
let contacts = systems.physics.frame_vec::<Contact>();
let draw_calls = systems.rendering.frame_vec::<DrawCall>();
```

### The Budget Pattern

Per-system memory budgets:

```rust
struct BudgetManager {
    budgets: HashMap<String, usize>,
    current: HashMap<String, usize>,
}

impl BudgetManager {
    fn check_budget(&mut self, tag: &str, size: usize) -> bool {
        let used = self.current.entry(tag.to_string()).or_insert(0);
        let budget = *self.budgets.get(tag).unwrap_or(0);
        
        if *used + size <= budget {
            *used += size;
            true
        } else {
            false
        }
    }
    
    fn reset_frame(&mut self) {
        for (_, used) in self.current.iter_mut() {
            *used = 0;
        }
    }
}
```

### The Scoped Pattern

RAII for tagged allocations:

```rust
struct ScopedTag<'a> {
    alloc: &'a SmartAlloc,
    tag: String,
}

impl<'a> ScopedTag<'a> {
    fn new(alloc: &'a SmartAlloc, tag: &str) -> Self {
        Self {
            alloc,
            tag: tag.to_string(),
        }
    }
    
    fn alloc_frame<T>(&self) -> FrameBox<T> {
        self.alloc.with_tag(&self.tag, |a| a.frame_box())
    }
}

impl<'a> Drop for ScopedTag<'a> {
    fn drop(&mut self) {
        // Cleanup if needed
    }
}
```

## Lifecycle Patterns

### The Frame Retention Pattern

Keep specific frame data beyond frame boundary:

```rust
struct FrameRetainer<T> {
    data: Option<FrameRetained<T>>,
    policy: RetentionPolicy,
}

impl<T> FrameRetainer<T> {
    fn retain(&mut self, alloc: &SmartAlloc, data: T) {
        self.data = Some(alloc.frame_retained(data, self.policy));
    }
    
    fn get(&self) -> Option<&T> {
        self.data.as_ref().map(|d| d.get())
    }
}

// Usage
let mut navmesh_cache = FrameRetainer::new(RetentionPolicy::PromoteToPool);
if should_rebuild_navmesh {
    navmesh_cache.retain(&alloc, build_navmesh());
}
```

### The Promotion Pattern

Promote frequently used frame data to pool:

```rust
struct PromotionTracker<T> {
    frame_count: usize,
    threshold: usize,
    data: Option<T>,
}

impl<T> PromotionTracker<T> {
    fn update(&mut self, alloc: &SmartAlloc, data: T) {
        self.frame_count += 1;
        
        if self.frame_count >= self.threshold {
            // Promote to pool
            self.data = Some(alloc.pool_box(data).into_inner());
            self.frame_count = 0;
        }
    }
}
```

### The Cleanup Pattern

Explicit cleanup at specific points:

```rust
struct CleanupGuard<'a> {
    alloc: &'a SmartAlloc,
    cleanup_points: Vec<fn(&SmartAlloc)>,
}

impl<'a> CleanupGuard<'a> {
    fn new(alloc: &'a SmartAlloc) -> Self {
        Self {
            alloc,
            cleanup_points: Vec::new(),
        }
    }
    
    fn add_cleanup(&mut self, cleanup: fn(&SmartAlloc)) {
        self.cleanup_points.push(cleanup);
    }
}

impl<'a> Drop for CleanupGuard<'a> {
    fn drop(&mut self) {
        for cleanup in &self.cleanup_points {
            cleanup(self.alloc);
        }
    }
}
```

## Error Handling Patterns

### The Fallible Allocation Pattern

Handle allocation failures gracefully:

```rust
fn try_allocate_frame<T>(alloc: &SmartAlloc) -> Option<FrameBox<T>> {
    alloc.try_frame_alloc()
}

fn safe_frame_alloc<T>(alloc: &SmartAlloc, fallback: T) -> FrameBox<T> {
    alloc.try_frame_alloc().unwrap_or_else(|| {
        // Use fallback allocation strategy
        alloc.pool_box(fallback).into_frame_box()
    })
}
```

### The Recovery Pattern

Recover from allocation errors:

```rust
struct AllocationRecovery {
    fallback_strategy: Box<dyn Fn() -> RecoveryAction>,
}

enum RecoveryAction {
    Retry,
    UseFallback,
    Abort,
}

impl AllocationRecovery {
    fn handle_error<T>(&self, alloc: &SmartAlloc) -> Result<FrameBox<T>, AllocError> {
        match alloc.try_frame_alloc() {
            Ok(data) => Ok(data),
            Err(_) => match (self.fallback_strategy)() {
                RecoveryAction::Retry => self.handle_error(alloc),
                RecoveryAction::UseFallback => {
                    // Implement fallback
                    Err(AllocError::FallbackUsed)
                }
                RecoveryAction::Abort => Err(AllocError::OutOfMemory),
            },
        }
    }
}
```

## Best Practices Summary

1. **Use frame allocation for temporary data** - Reset automatically
2. **Pool for persistent small objects** - Automatic lifecycle
3. **Heap for large persistent data** - Still tracked
4. **Tag allocations by system** - Better organization
5. **Set budgets** - Prevent memory bloat
6. **Use TransferHandle for cross-thread** - Safe transfers
7. **Promote hot data** - Frame → Pool → Heap
8. **Handle allocation failures** - Graceful degradation

## Next Steps

- Read [Advanced Guide](advanced.md) for deep internals
- Check [Performance Guide](performance.md) for optimization
- Explore [Cookbook](cookbook.md) for copy-paste recipes
- See domain-specific guides for your use case
