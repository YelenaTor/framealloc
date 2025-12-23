# Migration Guide

Coming to framealloc from other allocators or memory management approaches.

## Table of Contents

1. [From Standard Library](#from-standard-library)
2. [From Custom Allocators](#from-custom-allocators)
3. [From Arena Allocators](#from-arena-allocators)
4. [From Pool Allocators](#from-pool-allocators)
5. [From Other Game Engines](#from-other-game-engines)

## From Standard Library

### Basic Allocation Patterns

```rust
// Before - Standard library
fn process_data() {
    let mut data = Vec::new();
    for i in 0..1000 {
        data.push(i);
    }
    // Process data...
    // data automatically dropped
}

// After - framealloc
fn process_data(alloc: &SmartAlloc) {
    alloc.begin_frame();
    let mut data = alloc.frame_vec::<i32>();
    for i in 0..1000 {
        data.push(i);
    }
    // Process data...
    alloc.end_frame(); // Everything freed
}
```

### Box Usage

```rust
// Before
fn create_large_object() -> Box<LargeObject> {
    Box::new(LargeObject::new())
}

// After - Use pool for persistence
fn create_large_object(alloc: &SmartAlloc) -> PoolBox<LargeObject> {
    alloc.pool_box(LargeObject::new())
}

// Or frame for temporary use
fn use_temporarily(alloc: &SmartAlloc) {
    alloc.begin_frame();
    let obj = alloc.frame_box(LargeObject::new());
    // Use object...
    alloc.end_frame(); // Automatically freed
}
```

### Rc/Arc Patterns

```rust
// Before
fn shared_data() -> Arc<Data> {
    Arc::new(Data::new())
}

// After - framealloc's pool arc
fn shared_data(alloc: &SmartAlloc) -> PoolArc<Data> {
    alloc.pool_arc(Data::new())
}
```

### String Allocation

```rust
// Before
fn build_string(parts: &[&str]) -> String {
    let mut result = String::new();
    for part in parts {
        result.push_str(part);
    }
    result
}

// After - Frame string for temporary use
fn build_string(alloc: &SmartAlloc, parts: &[&str]) -> FrameString {
    alloc.begin_frame();
    let mut result = alloc.frame_string();
    for part in parts {
        result.push_str(part);
    }
    alloc.end_frame();
    result // Note: Must be used within frame
}
```

## From Custom Allocators

### Global Allocator Replacement

```rust
// Before - Custom global allocator
#[global_allocator]
static MY_ALLOCATOR: MyAllocator = MyAllocator;

fn main() {
    let data = Box::new(42); // Uses MyAllocator
}

// After - framealloc for specific patterns
fn main() {
    let alloc = SmartAlloc::new(Default::default());
    
    // Use framealloc where appropriate
    alloc.begin_frame();
    let frame_data = alloc.frame_box(42);
    alloc.end_frame();
    
    // Still use global for persistent data
    let persistent_data = Box::new(42);
}
```

### Arena Allocator Migration

```rust
// Before - Custom arena
struct MyArena {
    memory: Vec<u8>,
    offset: usize,
}

impl MyArena {
    fn alloc<T>(&mut self) -> &mut T {
        // Custom allocation logic
    }
}

// After - framealloc provides this out of the box
fn use_frame_arena(alloc: &SmartAlloc) {
    alloc.begin_frame();
    
    // Direct equivalent
    let data = alloc.frame_alloc::<T>();
    
    // Or batch for many allocations
    let many = unsafe { alloc.frame_alloc_batch::<T>(1000) };
    
    alloc.end_frame(); // All freed
}
```

### Bump Allocator Migration

```rust
// Before - Bumpalo
use bumpalo::Bump;

fn process_with_bump() {
    let bump = Bump::new();
    
    // Allocate from bump arena
    let vec = bump.alloc_vec(|| {
        let mut v = Vec::new();
        v.push(1);
        v.push(2);
        v
    });
    
    // Reset when done
    bump.reset();
}

// After - framealloc
fn process_with_frame(alloc: &SmartAlloc) {
    alloc.begin_frame();
    
    // Frame allocation is similar to bump
    let vec = alloc.frame_vec();
    vec.push(1);
    vec.push(2);
    
    alloc.end_frame(); // Automatic reset
}
```

## From Arena Allocators

### Typed Arena Migration

```rust
// Before - TypedArena
use typed_arena::Arena;

fn process_with_typed_arena() {
    let arena = Arena::new();
    
    // All allocations same type
    let item1 = arena.alloc(MyItem::new());
    let item2 = arena.alloc(MyItem::new());
    
    // Arena drops all at once
}

// After - framealloc with type safety
fn process_with_frame(alloc: &SmartAlloc) {
    alloc.begin_frame();
    
    // Can allocate different types
    let item1 = alloc.frame_alloc::<MyItem>();
    let item2 = alloc.frame_alloc::<OtherItem>();
    
    alloc.end_frame();
}
```

### Generational Arena Migration

```rust
// Before - generational-arena
use generational_arena::Arena;

struct EntityWorld {
    entities: Arena<Entity>,
}

impl EntityWorld {
    fn spawn(&mut self) -> EntityId {
        self.entities.insert(Entity::new())
    }
    
    fn get(&self, id: EntityId) -> Option<&Entity> {
        self.entities.get(id)
    }
}

// After - framealloc with handles
struct EntityWorld {
    entities: Vec<Option<Entity>>,
    free_list: Vec<usize>,
}

impl EntityWorld {
    fn spawn(&mut self, alloc: &SmartAlloc) -> EntityHandle {
        // Use frame allocation for temporary entities
        if let Some(idx) = self.free_list.pop() {
            self.entities[idx] = Some(Entity::new());
            EntityHandle::new(idx)
        } else {
            self.entities.push(Some(Entity::new()));
            EntityHandle::new(self.entities.len() - 1)
        }
    }
}
```

## From Pool Allocators

### Object Pool Migration

```rust
// Before - Custom object pool
struct ObjectPool<T> {
    objects: Vec<T>,
    available: Vec<usize>,
}

impl<T: Default> ObjectPool<T> {
    fn get(&mut self) -> &mut T {
        if let Some(idx) = self.available.pop() {
            &mut self.objects[idx]
        } else {
            self.objects.push(T::default());
            &mut self.objects[self.objects.len() - 1]
        }
    }
    
    fn return_object(&mut self, _obj: &T) {
        // Manual tracking needed
    }
}

// After - framealloc's built-in pools
fn use_pools(alloc: &SmartAlloc) {
    // Pool allocation - automatic management
    let obj1 = alloc.pool_alloc::<MyObject>();
    let obj2 = alloc.pool_alloc::<MyObject>();
    
    // Automatically returned when dropped
}
```

### Slab Allocator Migration

```rust
// Before - slab
use slab::Slab;

struct ComponentManager {
    slab: Slab<Component>,
}

impl ComponentManager {
    fn add(&mut self, component: Component) -> usize {
        self.slab.insert(component)
    }
    
    fn get(&self, key: usize) -> Option<&Component> {
        self.slab.get(key)
    }
}

// After - framealloc with direct indexing
struct ComponentManager {
    components: Vec<Option<Component>>,
    free_list: Vec<usize>,
}

impl ComponentManager {
    fn add(&mut self, alloc: &SmartAlloc, component: Component) -> ComponentHandle {
        // Use frame allocation for batch operations
        alloc.begin_frame();
        
        let handle = if let Some(idx) = self.free_list.pop() {
            self.components[idx] = Some(component);
            ComponentHandle::new(idx)
        } else {
            self.components.push(Some(component));
            ComponentHandle::new(self.components.len() - 1)
        };
        
        alloc.end_frame();
        handle
    }
}
```

## From Other Game Engines

### Unity C# Migration

```csharp
// Unity C# pattern
public class GameManager : MonoBehaviour {
    void Update() {
        // Temporary arrays each frame
        Vector3[] positions = new Vector3[1000];
        // Process positions...
        // GC pressure from allocations
    }
}

// Rust with framealloc equivalent
impl GameManager {
    fn update(&mut self, alloc: &SmartAlloc) {
        alloc.begin_frame();
        
        // Frame allocation - no GC pressure
        let positions = alloc.frame_slice::<Vector3>(1000);
        
        // Process positions...
        
        alloc.end_frame(); // All freed
    }
}
```

### Unreal Engine Migration

```cpp
// Unreal C++ pattern
class AMyActor : public AActor {
    TArray<FVector> Positions;
    
    void BeginPlay() {
        Positions.Reserve(1000);
        // Manual memory management
    }
};

// Rust with framealloc
struct MyActor {
    positions: FrameBox<[Vector3]>,
}

impl MyActor {
    fn new(alloc: &SmartAlloc) -> Self {
        alloc.begin_frame();
        let positions = alloc.frame_slice::<Vector3>(1000);
        Self {
            positions: alloc.frame_box(positions),
        }
        // Note: See patterns.md for proper lifecycle management
    }
}
```

### Godot Migration

```gdscript
# GDScript pattern
extends Node

func _process(delta):
    var positions = []
    for i in range(1000):
        positions.append(Vector3())
    # GDScript handles GC automatically

# Rust with framealloc
impl Node {
    fn process(&mut self, alloc: &SmartAlloc, delta: f32) {
        alloc.begin_frame();
        
        let positions = alloc.frame_slice::<Vector3>(1000);
        for i in 0..1000 {
            positions[i] = Vector3::new(0.0, 0.0, 0.0);
        }
        
        alloc.end_frame();
    }
}
```

## Migration Checklist

### Step 1: Identify Allocation Patterns

- [ ] Find all `Box::new()` calls
- [ ] Find all `Vec::new()` and `Vec::with_capacity()`
- [ ] Find all `String::new()` allocations
- [ ] Find custom arena/pool usage
- [ ] Identify per-frame temporary data

### Step 2: Choose Allocation Strategy

| Pattern | framealloc Strategy |
|---------|-------------------|
| Per-frame temporary | Frame allocation |
| Small persistent objects | Pool allocation |
| Large persistent data | Heap allocation |
| Cross-thread data | TransferHandle |
| Bulk small items | Batch allocation |

### Step 3: Refactor Incrementally

```rust
// 1. Add allocator parameter
fn process_data(data: &[u8]) -> Vec<u8> {
    // Old implementation
}

fn process_data(data: &[u8], alloc: &SmartAlloc) -> FrameBox<Vec<u8>> {
    // New implementation
}

// 2. Update call sites
let result = process_data(&input); // Old
let result = process_data(&input, &alloc); // New

// 3. Add frame boundaries
alloc.begin_frame();
let result = process_data(&input, &alloc);
alloc.end_frame();
```

### Step 4: Optimize Hot Paths

- [ ] Profile allocation patterns
- [ ] Convert hot loops to batch allocation
- [ ] Add tags for debugging
- [ ] Set budgets if needed
- [ ] Enable minimal mode in production

### Step 5: Validate

- [ ] Run existing tests
- [ ] Add memory leak detection
- [ ] Verify performance improvements
- [ ] Check for use-after-frame issues
- [ ] Test with cargo-fa lints

## Common Migration Issues

### Issue: Storing Frame Allocations

```rust
// Problem - frame data dies at end_frame()
struct Component {
    data: FrameBox<Vec<f32>>, // Won't work!
}

// Solution - use pool for persistence
struct Component {
    data: PoolBox<Vec<f32>>,
}

// Or frame retention for specific cases
struct Component {
    data: FrameRetained<Vec<f32>>,
}
```

### Issue: Cross-Thread Frame Data

```rust
// Problem - frame allocations aren't Send
fn send_data(data: FrameBox<Vec<u8>>) {
    thread::spawn(move || {
        // Error: data can't cross thread boundary
    });
}

// Solution - use TransferHandle
fn send_data(data: FrameBox<Vec<u8>>) {
    let handle = alloc.frame_box_for_transfer(data);
    thread::spawn(move || {
        let data = handle.receive();
        // Works correctly
    });
}
```

### Issue: Mixed Allocation Types

```rust
// Problem - type confusion
let mut vec = alloc.frame_vec::<u32>();
vec.push(alloc.pool_alloc::<u32>()); // Type error!

// Solution - consistent types
let mut vec = alloc.frame_vec::<u32>();
vec.push(42); // Direct value
```

## Performance Comparison

### Before Migration

```rust
// Standard library - GC pressure
fn game_loop() {
    loop {
        let entities = Vec::new(); // Allocation
        let positions = Vec::new(); // Allocation
        let velocities = Vec::new(); // Allocation
        
        // Process...
        // All dropped to GC
    }
}
```

### After Migration

```rust
// framealloc - deterministic
fn game_loop(alloc: &SmartAlloc) {
    loop {
        alloc.begin_frame();
        
        let entities = alloc.frame_vec::<Entity>();
        let positions = alloc.frame_vec::<Vector3>();
        let velocities = alloc.frame_vec::<Vector3>();
        
        // Process...
        
        alloc.end_frame(); // All freed deterministically
    }
}
```

### Expected Improvements

- **Allocation speed**: 10-100x faster for frame data
- **Memory usage**: Predictable, no fragmentation
- **GC pressure**: Eliminated for frame data
- **Cache performance**: Better locality
- **Determinism**: No GC pauses

## Tools for Migration

### cargo-fa Lints

```bash
# Install
cargo install cargo-fa

# Check for issues
cargo fa --all

# Specific checks
cargo fa --dirtymem    # Frame escape issues
cargo fa --threading   # Thread safety
cargo fa --rapier      # Physics engine issues
```

### Debug Features

```toml
# Enable during migration
framealloc = { version = "0.10", features = ["debug"] }
```

### Migration Script

```rust
// Simple script to find allocation patterns
use std::fs;
use std::io::Read;

fn find_allocations(file: &str) {
    let mut content = String::new();
    fs::File::open(file).unwrap().read_to_string(&mut content).unwrap();
    
    // Look for patterns
    if content.contains("Box::new") {
        println!("Found Box::new in {}", file);
    }
    if content.contains("Vec::new") {
        println!("Found Vec::new in {}", file);
    }
    // ... more patterns
}
```

## Further Reading

- [Getting Started](getting-started.md) - Basic concepts
- [Patterns Guide](patterns.md) - Common usage patterns
- [Performance Guide](performance.md) - Optimization techniques

Happy migrating! 🚀
