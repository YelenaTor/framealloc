# Async/Await with framealloc

Safe async/await patterns using framealloc's hybrid model.

## Table of Contents

1. [Overview](#overview)
2. [TaskAlloc](#taskalloc)
3. [AsyncPoolGuard](#asyncpoolguard)
4. [Common Patterns](#common-patterns)
5. [Integration with Tokio](#integration-with-tokio)
6. [Best Practices](#best-practices)

## Overview

framealloc provides special utilities for async code that maintain the frame allocation model while working with Rust's async/await:

- **TaskAlloc** - Task-scoped allocations that auto-cleanup
- **AsyncPoolGuard** - RAII guard for pool allocations
- **Hybrid model** - Frame allocations on main thread, pool/heap in tasks

### Key Principle

Frame allocations stay on the main thread. Async tasks use pool/heap allocations. This prevents frame data from crossing await points.

## TaskAlloc

### Basic Usage

```rust
use framealloc::SmartAlloc;
use framealloc::tokio::TaskAlloc;

async fn process_asset(alloc: &SmartAlloc, path: &str) -> Vec<u8> {
    let alloc_clone = alloc.clone();
    
    tokio::spawn(async move {
        // Create task-scoped allocator
        let mut task = TaskAlloc::new(&alloc_clone);
        
        // Load asset (pool allocation)
        let data = task.alloc_box(load_from_disk(path).await);
        
        // Process data
        let processed = process_data(&data).await;
        
        // All allocations automatically freed when task ends
    }).await.unwrap()
}
```

### Task-Scoped Allocations

```rust
use framealloc::tokio::TaskAlloc;

async fn handle_connection(mut task: TaskAlloc) {
    // All allocations in this task are tracked
    
    // Pool allocation for connection state
    let state = task.alloc_box(ConnectionState::new());
    
    // Frame allocation not available here (correctly!)
    // let frame_data = task.frame_alloc<u8>(); // Error!
    
    // Use pool for temporary data
    let buffer = task.alloc_vec::<u8>();
    
    // Process connection
    while !state.is_closed() {
        let data = receive_data().await;
        buffer.extend_from_slice(&data);
        process_buffer(&buffer).await;
        buffer.clear();
    }
    
    // All allocations freed when function returns
}
```

### Nested Tasks

```rust
async fn process_requests(alloc: &SmartAlloc, requests: Vec<Request>) {
    let mut handles = Vec::new();
    
    for request in requests {
        let alloc_clone = alloc.clone();
        let handle = tokio::spawn(async move {
            let mut task = TaskAlloc::new(&alloc_clone);
            process_single_request(&mut task, request).await
        });
        handles.push(handle);
    }
    
    // Wait for all tasks
    for handle in handles {
        handle.await.unwrap();
    }
}

async fn process_single_request(task: &mut TaskAlloc, request: Request) {
    // Use task allocations
    let data = task.alloc_box(request.data);
    let result = compute_result(&data).await;
    
    // Send result back
    send_result(result).await;
}
```

## AsyncPoolGuard

### RAII Pattern

```rust
use framealloc::tokio::AsyncPoolGuard;

async fn process_with_guard(alloc: &SmartAlloc) {
    // Guard ensures cleanup even on early return
    let _guard = AsyncPoolGuard::new(&alloc);
    
    // Pool allocations are automatically tracked
    let data1 = alloc.pool_alloc::<Data>();
    let data2 = alloc.pool_alloc::<MoreData>();
    
    // Complex async logic
    if some_condition().await {
        // Early return - guard still cleans up
        return;
    }
    
    // More processing
    process_data(data1, data2).await;
    
    // Guard drops here, cleaning up all pool allocations
}
```

### Scoped Resource Management

```rust
async fn database_transaction(alloc: &SmartAlloc) -> Result<(), Error> {
    let _guard = AsyncPoolGuard::new(&alloc);
    
    // Begin transaction
    let tx = begin_transaction().await?;
    
    // Allocate query results
    let results = alloc.pool_vec::<Row>();
    
    // Execute queries
    for query in queries {
        let rows = execute_query(&tx, query).await?;
        results.extend(rows);
    }
    
    // Commit or rollback
    if validate_results(&results).await {
        tx.commit().await?;
        Ok(())
    } else {
        tx.rollback().await?;
        Err(Error::ValidationFailed)
    }
    // Guard ensures all Row allocations are freed
}
```

## Common Patterns

### Main Thread Coordination

```rust
use framealloc::SmartAlloc;
use framealloc::tokio::TaskAlloc;

struct GameServer {
    alloc: SmartAlloc,
    runtime: tokio::Runtime,
}

impl GameServer {
    fn new() -> Self {
        Self {
            alloc: SmartAlloc::new(Default::default()),
            runtime: tokio::Runtime::new().unwrap(),
        }
    }
    
    fn run(&mut self) {
        loop {
            // Main thread frame allocation
            self.alloc.begin_frame();
            
            // Process network events (async)
            let network_events = self.runtime.block_on(async {
                self.process_network().await
            });
            
            // Process events with frame allocation
            for event in network_events {
                self.handle_event(event);
            }
            
            // Update game state
            self.update_game();
            
            self.alloc.end_frame();
        }
    }
    
    async fn process_network(&self) -> Vec<NetworkEvent> {
        let alloc_clone = self.alloc.clone();
        
        // Spawn async tasks for network I/O
        let mut handles = Vec::new();
        
        for connection in self.get_connections() {
            let alloc_clone = alloc_clone.clone();
            let handle = tokio::spawn(async move {
                let mut task = TaskAlloc::new(&alloc_clone);
                read_connection_events(&mut task, connection).await
            });
            handles.push(handle);
        }
        
        // Collect all events
        let mut all_events = Vec::new();
        for handle in handles {
            all_events.extend(handle.await.unwrap());
        }
        
        all_events
    }
}
```

### Asset Loading Pipeline

```rust
struct AssetLoader {
    alloc: SmartAlloc,
    loading_queue: Vec<LoadRequest>,
    loaded_assets: HashMap<String, Asset>,
}

impl AssetLoader {
    async fn load_assets(&mut self) {
        let alloc_clone = self.alloc.clone();
        
        // Process loading queue in parallel
        let futures: Vec<_> = self.loading_queue.drain(..)
            .map(|request| {
                let alloc_clone = alloc_clone.clone();
                tokio::spawn(async move {
                    let mut task = TaskAlloc::new(&alloc_clone);
                    load_asset_async(&mut task, request).await
                })
            })
            .collect();
        
        // Wait for all loads
        for future in futures {
            match future.await.unwrap() {
                Ok(asset) => {
                    self.loaded_assets.insert(asset.path.clone(), asset);
                }
                Err(e) => eprintln!("Failed to load asset: {}", e);
            }
        }
    }
}

async fn load_asset_async(task: &mut TaskAlloc, request: LoadRequest) -> Result<Asset, Error> {
    // Load file data
    let file_data = task.alloc_box(read_file(&request.path).await?);
    
    // Parse based on type
    match request.asset_type {
        AssetType::Texture => {
            let texture = parse_texture(&file_data).await?;
            Ok(Asset::Texture(texture))
        }
        AssetType::Mesh => {
            let mesh = parse_mesh(&file_data).await?;
            Ok(Asset::Mesh(mesh))
        }
        AssetType::Audio => {
            let audio = parse_audio(&file_data).await?;
            Ok(Asset::Audio(audio))
        }
    }
}
```

### Stream Processing

```rust
async fn process_stream(alloc: &SmartAlloc, mut stream: TcpStream) {
    let _guard = AsyncPoolGuard::new(&alloc);
    
    // Buffer for incoming data
    let buffer = alloc.pool_vec::<u8>();
    
    loop {
        // Read chunk
        let chunk = read_chunk(&mut stream).await?;
        if chunk.is_empty() {
            break;
        }
        
        // Process chunk
        let processed = process_chunk(&chunk).await;
        
        // Write back
        write_chunk(&mut stream, &processed).await?;
    }
    
    // Guard ensures buffer is freed
}

async fn process_chunk(chunk: &[u8]) -> Vec<u8> {
    // Simple transformation
    chunk.iter()
        .map(|&b| b.wrapping_add(1))
        .collect()
}
```

## Integration with Tokio

### Tokio Feature

```toml
# Cargo.toml
framealloc = { version = "0.10", features = ["tokio"] }
```

### Tokio Runtime Integration

```rust
use framealloc::SmartAlloc;
use framealloc::tokio::{TaskAlloc, AsyncPoolGuard};
use tokio::runtime::Runtime;

struct AsyncApplication {
    alloc: SmartAlloc,
    runtime: Runtime,
}

impl AsyncApplication {
    fn new() -> Self {
        Self {
            alloc: SmartAlloc::new(Default::default()),
            runtime: Runtime::new().unwrap(),
        }
    }
    
    fn run(&mut self) {
        self.runtime.block_on(async {
            self.main_loop().await;
        });
    }
    
    async fn main_loop(&self) {
        loop {
            // Async operations
            self.process_network().await;
            self.update_database().await;
            
            // Yield to other tasks
            tokio::task::yield_now().await;
        }
    }
}
```

### Async Channels

```rust
use tokio::sync::mpsc;

async fn worker_task(
    mut receiver: mpsc::Receiver<WorkItem>,
    alloc: SmartAlloc,
) {
    let mut task = TaskAlloc::new(&alloc);
    
    while let Some(work) = receiver.recv().await {
        // Process work item with task allocations
        let result = process_work_item(&mut task, work).await;
        
        // Send result back
        if let Some(sender) = result.sender {
            let _ = sender.send(result.data).await;
        }
    }
}

async fn dispatch_work(work_items: Vec<WorkItem>) -> Vec<WorkResult> {
    let alloc = SmartAlloc::new(Default::default());
    let (tx, rx) = mpsc::channel(100);
    
    // Spawn worker tasks
    for _ in 0..4 {
        let alloc_clone = alloc.clone();
        let rx_clone = rx.clone();
        tokio::spawn(async move {
            worker_task(rx_clone, alloc_clone).await;
        });
    }
    
    // Send work
    for item in work_items {
        let _ = tx.send(item).await;
    }
    
    // Collect results
    let mut results = Vec::new();
    // ... collect logic ...
    
    results
}
```

### Async Timers

```rust
use tokio::time::{interval, Duration};

async fn timed_operations(alloc: &SmartAlloc) {
    let mut interval = interval(Duration::from_secs(1));
    
    loop {
        interval.tick().await;
        
        // Use AsyncPoolGuard for cleanup
        let _guard = AsyncPoolGuard::new(&alloc);
        
        // Periodic operations
        let metrics = collect_metrics().await;
        let report = generate_report(&metrics).await;
        
        send_report(report).await;
        
        // All allocations cleaned up automatically
    }
}
```

## Best Practices

### Do's

- ✅ Use `TaskAlloc` for async task allocations
- ✅ Use `AsyncPoolGuard` for scoped cleanup
- ✅ Keep frame allocations on main thread
- ✅ Use pool/heap in async code
- ✅ Let TaskAlloc handle cleanup automatically

### Don'ts

- ❌ Use frame allocations across await points
- ❌ Store TaskAlloc across await points
- ❌ Mix frame and pool allocations confusingly
- ❌ Forget to enable tokio feature

### Performance Tips

1. **Batch operations** - Group related allocations
2. **Reuse TaskAlloc** - For long-running tasks
3. **Pool large data** - Use pool for big buffers
4. **Minimize awaits** - Reduce allocation boundaries

### Common Patterns

```rust
// Pattern 1: Task with cleanup
async fn with_cleanup(alloc: &SmartAlloc) {
    let _guard = AsyncPoolGuard::new(&alloc);
    // All pool allocations auto-cleaned
}

// Pattern 2: Spawned task
async fn spawn_task(alloc: &SmartAlloc) {
    let alloc_clone = alloc.clone();
    tokio::spawn(async move {
        let mut task = TaskAlloc::new(&alloc_clone);
        // Task-specific allocations
    });
}

// Pattern 3: Stream processing
async fn process_stream(alloc: &SmartAlloc, stream: TcpStream) {
    let _guard = AsyncPoolGuard::new(&alloc);
    // Stream-specific allocations
}
```

## Migration from Sync Code

### Before (Sync)

```rust
fn process_data(data: &[u8]) -> Vec<u8> {
    let alloc = SmartAlloc::new(Default::default());
    alloc.begin_frame();
    
    let result = alloc.frame_vec::<u8>();
    // Process data...
    
    alloc.end_frame();
    result.into_inner()
}
```

### After (Async)

```rust
async fn process_data_async(alloc: &SmartAlloc, data: &[u8]) -> Vec<u8> {
    let mut task = TaskAlloc::new(alloc);
    
    let result = task.alloc_vec::<u8>();
    // Process data asynchronously...
    
    result.into_inner()
}
```

## Troubleshooting

### "Frame allocation in async function"

```rust
// Error - cargo-fa FA701
async fn bad_function() {
    let alloc = SmartAlloc::new(Default::default());
    alloc.begin_frame();
    let data = alloc.frame_alloc::<u8>(); // Error!
    some_async_op().await;
    alloc.end_frame();
}

// Solution - use TaskAlloc
async fn good_function() {
    let alloc = SmartAlloc::new(Default::default());
    let mut task = TaskAlloc::new(&alloc);
    let data = task.alloc_box::<u8>();
    some_async_op().await;
}
```

### "Frame data crossing await point"

```rust
// Error - cargo-fa FA603
async fn crossing_await() {
    let alloc = SmartAlloc::new(Default::default());
    alloc.begin_frame();
    let data = alloc.frame_vec::<u8>();
    async_op().await; // Frame data crosses await!
    use_data(&data);
    alloc.end_frame();
}

// Solution - use pool allocation
async fn not_crossing() {
    let alloc = SmartAlloc::new(Default::default());
    let data = alloc.pool_vec::<u8>();
    async_op().await;
    use_data(&data);
}
```

## Further Reading

- [Getting Started](getting-started.md) - Basic concepts
- [Patterns Guide](patterns.md) - Common patterns
- [Tokio Documentation](https://tokio.rs) - Async runtime

Happy async programming! 🚀
