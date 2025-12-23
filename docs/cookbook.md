# framealloc Cookbook

Copy-paste recipes for common framealloc tasks.

## Table of Contents

1. [Basic Recipes](#basic-recipes)
2. [Game Development](#game-development)
3. [Data Processing](#data-processing)
4. [System Integration](#system-integration)
5. [Debugging Tools](#debugging-tools)

## Basic Recipes

### Hello World

```rust
use framealloc::SmartAlloc;

fn main() {
    let alloc = SmartAlloc::new(Default::default());
    
    alloc.begin_frame();
    let message = alloc.frame_box("Hello, framealloc!");
    println!("{}", message);
    alloc.end_frame();
}
```

### Simple Counter

```rust
use framealloc::SmartAlloc;

fn count_items(alloc: &SmartAlloc, items: &[i32]) -> usize {
    alloc.begin_frame();
    
    let filtered = alloc.frame_vec::<&i32>();
    for item in items {
        if *item > 0 {
            filtered.push(item);
        }
    }
    
    let count = filtered.len();
    alloc.end_frame();
    count
}
```

### Temporary Buffer

```rust
use framealloc::SmartAlloc;

fn process_data(data: &[u8]) -> Vec<u8> {
    let alloc = SmartAlloc::new(Default::default());
    
    alloc.begin_frame();
    
    // Temporary buffer for processing
    let temp = alloc.frame_slice::<u8>(data.len() * 2);
    
    // Process into temporary buffer
    let mut output_len = 0;
    for &byte in data {
        temp[output_len] = byte;
        temp[output_len + 1] = byte.wrapping_add(1);
        output_len += 2;
    }
    
    // Copy result to owned Vec
    let result = temp[..output_len].to_vec();
    
    alloc.end_frame();
    result
}
```

## Game Development

### Entity Component System

```rust
use framealloc::SmartAlloc;

#[derive(Clone, Copy)]
struct EntityId(u32);

struct ComponentStorage<T> {
    entities: Vec<EntityId>,
    components: Vec<T>,
}

impl<T> ComponentStorage<T> {
    fn new() -> Self {
        Self {
            entities: Vec::new(),
            components: Vec::new(),
        }
    }
    
    fn frame_snapshot(&self, alloc: &SmartAlloc) -> FrameBox<[(EntityId, T)]> {
        alloc.begin_frame();
        
        let snapshot = alloc.frame_box(
            self.entities.iter()
                .zip(&self.components)
                .map(|(&e, &c)| (e, c))
                .collect::<Vec<_>>()
        );
        
        alloc.end_frame();
        snapshot
    }
}

// Usage
let positions = ComponentStorage::<Vector3>::new();
let frame_positions = positions.frame_snapshot(&alloc);
```

### Spatial Grid

```rust
use framealloc::SmartAlloc;

struct SpatialGrid {
    cells: Vec<Vec<EntityId>>,
    width: usize,
    height: usize,
    cell_size: f32,
}

impl SpatialGrid {
    fn query_region(&self, alloc: &SmartAlloc, x: f32, y: f32, w: f32, h: f32) -> FrameBox<Vec<EntityId>> {
        alloc.begin_frame();
        
        let mut results = alloc.frame_vec::<EntityId>();
        
        let min_x = (x / self.cell_size) as usize;
        let min_y = (y / self.cell_size) as usize;
        let max_x = ((x + w) / self.cell_size) as usize;
        let max_y = ((y + h) / self.cell_size) as usize;
        
        for cy in min_y..=max_y.min(self.height - 1) {
            for cx in min_x..=max_x.min(self.width - 1) {
                let idx = cy * self.width + cx;
                results.extend_from_slice(&self.cells[idx]);
            }
        }
        
        alloc.end_frame();
        alloc.frame_box(results.into_inner())
    }
}
```

### Particle System

```rust
use framealloc::SmartAlloc;

struct Particle {
    position: Vector3,
    velocity: Vector3,
    life: f32,
}

struct ParticleSystem {
    particles: Vec<Particle>,
    max_particles: usize,
}

impl ParticleSystem {
    fn update(&mut self, alloc: &SmartAlloc, dt: f32) {
        alloc.begin_frame();
        
        // Temporary arrays for updates
        let positions = alloc.frame_slice::<Vector3>(self.particles.len());
        let velocities = alloc.frame_slice::<Vector3>(self.particles.len());
        
        // Update particles
        for (i, particle) in self.particles.iter_mut().enumerate() {
            particle.velocity.y -= 9.81 * dt;
            particle.position += particle.velocity * dt;
            particle.life -= dt;
            
            positions[i] = particle.position;
            velocities[i] = particle.velocity;
        }
        
        // Remove dead particles
        self.particles.retain(|p| p.life > 0.0);
        
        alloc.end_frame();
    }
    
    fn spawn(&mut self, alloc: &SmartAlloc, count: usize) {
        alloc.begin_frame();
        
        let new_particles = alloc.frame_slice::<Particle>(count);
        for i in 0..count {
            self.particles.push(Particle {
                position: Vector3::new(0.0, 0.0, 0.0),
                velocity: Vector3::new(
                    rand::random::<f32>() - 0.5,
                    rand::random::<f32>(),
                    rand::random::<f32>() - 0.5,
                ),
                life: 1.0,
            });
        }
        
        alloc.end_frame();
    }
}
```

## Data Processing

### Map-Reduce Pattern

```rust
use framealloc::SmartAlloc;

fn map_reduce<T, R, M, F>(data: &[T], alloc: &SmartAlloc, map_fn: M, reduce_fn: F) -> R
where
    M: Fn(&T) -> R,
    F: Fn(R, R) -> R,
    R: Copy + Default,
{
    alloc.begin_frame();
    
    // Map phase
    let mapped = alloc.frame_slice::<R>(data.len());
    for (i, item) in data.iter().enumerate() {
        mapped[i] = map_fn(item);
    }
    
    // Reduce phase
    let mut result = R::default();
    for value in mapped.iter() {
        result = reduce_fn(result, *value);
    }
    
    alloc.end_frame();
    result
}

// Usage
let sum = map_reduce(&numbers, &alloc, |x| x * x, |a, b| a + b);
```

### Sliding Window

```rust
use framealloc::SmartAlloc;

fn sliding_window<T>(data: &[T], window_size: usize, alloc: &SmartAlloc) -> FrameBox<Vec<&[T]>> {
    alloc.begin_frame();
    
    let mut windows = alloc.frame_vec::<&[T]>();
    
    for i in 0..=data.len().saturating_sub(window_size) {
        windows.push(&data[i..i + window_size]);
    }
    
    alloc.end_frame();
    alloc.frame_box(windows.into_inner())
}

// Usage
let windows = sliding_window(&data, 3, &alloc);
```

### Group By

```rust
use framealloc::SmartAlloc;
use std::collections::HashMap;

fn group_by<K, V, F>(data: &[V], alloc: &SmartAlloc, key_fn: F) -> FrameBox<HashMap<K, Vec<&V>>>
where
    K: Eq + Hash,
    F: Fn(&V) -> K,
{
    alloc.begin_frame();
    
    let mut groups = HashMap::new();
    
    for item in data {
        let key = key_fn(item);
        groups.entry(key).or_insert_with(|| alloc.frame_vec()).push(item);
    }
    
    // Convert frame vectors to owned vectors
    let owned_groups = groups.into_iter()
        .map(|(k, v)| (k, v.into_inner()))
        .collect();
    
    alloc.end_frame();
    alloc.frame_box(owned_groups)
}
```

## System Integration

### Buffer Pool

```rust
use framealloc::SmartAlloc;

struct BufferPool {
    buffers: Vec<Vec<u8>>,
    available: Vec<usize>,
}

impl BufferPool {
    fn get(&mut self, alloc: &SmartAlloc, size: usize) -> FrameBox<[u8]> {
        if let Some(idx) = self.available.pop() {
            alloc.frame_box(self.buffers[idx].split_at_mut(size).0)
        } else {
            alloc.frame_slice::<u8>(size)
        }
    }
    
    fn return_buffer(&mut self, buffer: Vec<u8>) {
        self.buffers.push(buffer);
        self.available.push(self.buffers.len() - 1);
    }
}
```

### Command Queue

```rust
use framealloc::SmartAlloc;

enum Command {
    SpawnEntity(Vector3),
    DestroyEntity(u32),
    MoveEntity(u32, Vector3),
}

struct CommandQueue {
    commands: Vec<Command>,
}

impl CommandQueue {
    fn drain_frame(&mut self, alloc: &SmartAlloc) -> FrameBox<Vec<Command>> {
        alloc.begin_frame();
        
        let frame_commands = alloc.frame_box(self.commands.clone());
        self.commands.clear();
        
        alloc.end_frame();
        frame_commands
    }
}
```

### Resource Cache

```rust
use framealloc::SmartAlloc;

struct ResourceCache<T> {
    cache: HashMap<String, PoolBox<T>>,
    access_order: Vec<String>,
    max_size: usize,
}

impl<T> ResourceCache<T> {
    fn get_or_load<F>(&mut self, alloc: &SmartAlloc, key: &str, loader: F) -> &T
    where
        F: FnOnce() -> T,
    {
        if !self.cache.contains_key(key) {
            if self.cache.len() >= self.max_size {
                // Evict oldest
                let oldest = self.access_order.remove(0);
                self.cache.remove(&oldest);
            }
            
            let resource = alloc.pool_box(loader());
            self.cache.insert(key.to_string(), resource);
            self.access_order.push(key.to_string());
        }
        
        self.cache.get(key).unwrap()
    }
}
```

## Debugging Tools

### Allocation Tracker

```rust
#[cfg(debug_assertions)]
use framealloc::SmartAlloc;

#[cfg(debug_assertions)]
struct AllocationTracker {
    allocations: HashMap<*const u8, AllocationInfo>,
}

#[cfg(debug_assertions)]
#[derive(Debug)]
struct AllocationInfo {
    size: usize,
    backtrace: Vec<String>,
    timestamp: std::time::Instant,
}

#[cfg(debug_assertions)]
impl AllocationTracker {
    fn track<T>(&mut self, ptr: *const T, size: usize) {
        let info = AllocationInfo {
            size,
            backtrace: std::backtrace::Backtrace::new()
                .frames()
                .iter()
                .skip(3)
                .take(5)
                .map(|f| format!("{}", f))
                .collect(),
            timestamp: std::time::Instant::now(),
        };
        self.allocations.insert(ptr as *const u8, info);
    }
    
    fn find_leaks(&self) -> Vec<&AllocationInfo> {
        self.allocations.values()
            .filter(|info| info.timestamp.elapsed() > std::time::Duration::from_secs(60))
            .collect()
    }
}
```

### Memory Profiler

```rust
use framealloc::SmartAlloc;

struct MemoryProfiler {
    frame_stats: Vec<FrameStats>,
    current_frame: usize,
}

impl MemoryProfiler {
    fn begin_frame(&mut self) {
        self.current_frame += 1;
    }
    
    fn record_stats(&mut self, stats: FrameStats) {
        self.frame_stats.push(stats);
        
        // Keep only last 1000 frames
        if self.frame_stats.len() > 1000 {
            self.frame_stats.remove(0);
        }
    }
    
    fn get_average_usage(&self) -> f64 {
        if self.frame_stats.is_empty() {
            return 0.0;
        }
        
        let total: usize = self.frame_stats.iter()
            .map(|s| s.bytes_allocated)
            .sum();
        total as f64 / self.frame_stats.len() as f64
    }
}
```

### Stress Test

```rust
use framealloc::SmartAlloc;

fn stress_test(alloc: &SmartAlloc, iterations: usize) {
    for i in 0..iterations {
        alloc.begin_frame();
        
        // Random allocation pattern
        let count = rand::random::<usize>() % 1000 + 100;
        let data = alloc.frame_vec::<u8>();
        
        for _ in 0..count {
            data.push(rand::random());
        }
        
        // Process data
        let sum: u64 = data.iter().map(|&x| x as u64).sum();
        
        alloc.end_frame();
        
        if i % 100 == 0 {
            println!("Completed {} iterations", i);
        }
    }
}
```

## Performance Recipes

### Batch Allocation

```rust
use framealloc::SmartAlloc;

fn process_particles_optimized(alloc: &SmartAlloc, count: usize) {
    alloc.begin_frame();
    
    // Batch allocate all particles at once
    let particles = unsafe {
        let batch = alloc.frame_alloc_batch::<Particle>(count);
        
        for i in 0..count {
            let particle = batch.add(i);
            std::ptr::write(particle, Particle::random());
        }
        
        batch
    };
    
    // Process all particles
    for i in 0..count {
        let particle = unsafe { particles.get(i) };
        update_particle(particle);
    }
    
    alloc.end_frame();
}
```

### Zero-Copy Buffer Sharing

```rust
use framealloc::SmartAlloc;

struct SharedBuffer<'a> {
    data: &'a [u8],
    regions: Vec<BufferRegion>,
}

struct BufferRegion {
    offset: usize,
    len: usize,
}

impl<'a> SharedBuffer<'a> {
    fn split(&self, alloc: &SmartAlloc, pieces: usize) -> FrameBox<Vec<&'a [u8]>> {
        alloc.begin_frame();
        
        let mut regions = alloc.frame_vec::<&[u8]>();
        let chunk_size = self.data.len() / pieces;
        
        for i in 0..pieces {
            let start = i * chunk_size;
            let end = if i == pieces - 1 {
                self.data.len()
            } else {
                start + chunk_size
            };
            regions.push(&self.data[start..end]);
        }
        
        alloc.end_frame();
        alloc.frame_box(regions.into_inner())
    }
}
```

## Quick Reference

### Common Patterns

```rust
// Frame allocation (temporary)
alloc.begin_frame();
let data = alloc.frame_alloc::<T>();
let vec = alloc.frame_vec::<T>();
let slice = alloc.frame_slice::<T>(n);
alloc.end_frame();

// Pool allocation (persistent)
let boxed = alloc.pool_box(value);
let arc = alloc.pool_arc(value);

// Heap allocation (large)
let boxed = alloc.heap_box(value);
let vec = alloc.heap_vec::<T>();

// Organization
alloc.with_tag("system", |a| { /* allocations */ });

// Threading
let handle = alloc.frame_box_for_transfer(data);
let data = handle.receive();

// Batch allocation
let batch = unsafe { alloc.frame_alloc_batch::<T>(count) };
let item = unsafe { batch.add(index) };
```

### Error Handling

```rust
// Fallible allocation
let data = alloc.try_frame_alloc::<T>()?;

// Fallback strategy
let data = alloc.try_frame_alloc::<T>()
    .unwrap_or_else(|| alloc.pool_box(default_value()).into_frame_box());
```

### Debug Features

```rust
// Enable debug mode
#[cfg(feature = "debug")]
alloc.enable_behavior_filter();

// Check for issues
let report = alloc.behavior_report();
for issue in &report.issues {
    eprintln!("[{}] {}", issue.code, issue.message);
}
```

## Tips and Tricks

1. **Always call begin_frame/end_frame in pairs**
2. **Use batch allocation for many small objects**
3. **Tag allocations by subsystem for debugging**
4. **Pool frequently reused objects**
5. **Use TransferHandle for cross-thread data**
6. **Enable debug features during development**
7. **Profile before optimizing**
8. **Consider memory layout for cache performance**

Happy cooking with framealloc! 🚀
