//! Diagnostic code explanations for `cargo fa explain`.

use colored::*;

/// Detailed explanation of a diagnostic code.
pub struct Explanation {
    pub code: &'static str,
    pub name: &'static str,
    pub category: &'static str,
    pub severity: &'static str,
    pub summary: &'static str,
    pub description: &'static str,
    pub example_bad: &'static str,
    pub example_good: &'static str,
    pub see_also: &'static [&'static str],
}

/// Get explanation for a diagnostic code
pub fn get_explanation(code: &str) -> Option<Explanation> {
    match code.to_uppercase().as_str() {
        "FA601" => Some(Explanation {
            code: "FA601",
            name: "frame-escape",
            category: "Lifetime",
            severity: "warning",
            summary: "Frame allocation may escape frame scope",
            description: r#"
Frame allocations are designed to be temporary - they live only until 
`end_frame()` is called. This diagnostic triggers when a frame allocation
appears to be stored in a structure that might outlive the current frame.

After `end_frame()`, all frame memory is invalidated. Any references to
frame-allocated data become dangling pointers, leading to undefined behavior.

This is a common mistake when:
- Storing frame data in persistent game state
- Passing frame allocations to callbacks that execute later
- Caching frame allocations across frames
"#,
            example_bad: r#"
struct GameState {
    cached_path: Option<FrameBox<NavPath>>,  // BAD: survives frame
}

fn update(alloc: &SmartAlloc, state: &mut GameState) {
    let path = alloc.frame_box(compute_path());
    state.cached_path = Some(path);  // FA601: escapes to persistent state
    alloc.end_frame();  // path is now invalid!
}
"#,
            example_good: r#"
struct GameState {
    cached_path: Option<PoolBox<NavPath>>,  // GOOD: persistent allocation
}

fn update(alloc: &SmartAlloc, state: &mut GameState) {
    let path = alloc.pool_box(compute_path());  // Use pool for persistent data
    state.cached_path = Some(path);
    alloc.end_frame();  // pool allocations survive
}
"#,
            see_also: &["FA605", "FA703"],
        }),
        
        "FA602" => Some(Explanation {
            code: "FA602",
            name: "loop-allocation", 
            category: "Lifetime",
            severity: "warning",
            summary: "Allocation in hot loop",
            description: r#"
Allocations inside tight loops can cause performance issues:
- Pool allocations may exhaust pool capacity
- Frame allocations accumulate until frame end
- Heap allocations cause allocator pressure

Even fast allocators have overhead. When you allocate thousands of times
per frame inside a physics or rendering loop, that overhead adds up.

Consider:
- Pre-allocating buffers before the loop
- Using frame_vec() to batch allocations
- Moving allocation outside the loop when possible
"#,
            example_bad: r#"
for entity in entities {  // 10,000 entities
    let transform = alloc.pool_alloc::<Transform>();  // FA602: 10K allocations!
    // ...
}
"#,
            example_good: r#"
// Pre-allocate once
let mut transforms = alloc.frame_vec::<Transform>();
transforms.reserve(entities.len());

for entity in entities {
    transforms.push(entity.transform);  // No allocation per iteration
}
"#,
            see_also: &["FA301"],
        }),
        
        "FA603" => Some(Explanation {
            code: "FA603",
            name: "missing-frame-boundary",
            category: "Lifetime", 
            severity: "warning",
            summary: "Frame-structured loop without frame lifecycle calls",
            description: r#"
This diagnostic triggers when cargo-fa detects a main loop pattern
(loop { }, while running { }, etc.) that doesn't call begin_frame()
or end_frame().

Without frame boundaries:
- Frame allocations accumulate indefinitely
- Memory usage grows until OOM
- Frame-based budgeting doesn't work

Every game loop should have explicit frame boundaries.
"#,
            example_bad: r#"
fn main() {
    let alloc = SmartAlloc::new(AllocConfig::default());
    
    loop {  // FA603: no frame boundaries
        let temp = alloc.frame_alloc::<TempData>();
        process(temp);
        // temp is never freed!
    }
}
"#,
            example_good: r#"
fn main() {
    let alloc = SmartAlloc::new(AllocConfig::default());
    
    loop {
        alloc.begin_frame();
        let temp = alloc.frame_alloc::<TempData>();
        process(temp);
        alloc.end_frame();  // temp is freed here
    }
}
"#,
            see_also: &["FA601"],
        }),
        
        "FA701" => Some(Explanation {
            code: "FA701",
            name: "async-frame",
            category: "Async Safety",
            severity: "error",
            summary: "Frame allocation in async function",
            description: r#"
Async functions can suspend at await points. When they resume, they might
be on a different thread or at a different point in the frame lifecycle.

Frame allocations in async code are dangerous because:
- The frame may have been reset while the task was suspended
- The allocation becomes a dangling pointer after end_frame()
- Async tasks may outlive many frames

This is marked as an ERROR because it almost always leads to bugs.
"#,
            example_bad: r#"
async fn load_asset(alloc: &SmartAlloc) {
    let buffer = alloc.frame_box(vec![0u8; 1024]);  // FA701: in async fn
    let data = fetch_data().await;  // May suspend across frames!
    buffer.copy_from_slice(&data);  // buffer may be invalid
}
"#,
            example_good: r#"
async fn load_asset(alloc: &SmartAlloc) {
    let buffer = alloc.heap_box(vec![0u8; 1024]);  // Heap survives
    let data = fetch_data().await;
    buffer.copy_from_slice(&data);  // Safe!
}
"#,
            see_also: &["FA702", "FA703"],
        }),
        
        "FA702" => Some(Explanation {
            code: "FA702",
            name: "await-crossing",
            category: "Async Safety",
            severity: "error", 
            summary: "Frame allocation used across await point",
            description: r#"
This is a more specific version of FA701. It triggers when:
1. A frame allocation is created
2. An await point occurs
3. The allocation is used after the await

The await point is the dangerous boundary - frames may reset during
the suspension, invalidating all frame allocations.
"#,
            example_bad: r#"
async fn process() {
    let data = alloc.frame_vec::<u8>();  // Created before await
    data.extend(initial_data());
    
    network_send().await;  // FA702: await crossing
    
    data.extend(more_data());  // data may be invalid!
}
"#,
            example_good: r#"
async fn process() {
    // Option 1: Complete frame work before await
    let data = alloc.frame_vec::<u8>();
    data.extend(initial_data());
    let result = process_data(&data);
    alloc.end_frame();  // Explicitly end frame
    
    network_send().await;  // Safe: no frame data crosses
    
    // Option 2: Use persistent allocation
    let data = alloc.pool_vec::<u8>();  // Survives await
}
"#,
            see_also: &["FA701", "FA703"],
        }),
        
        "FA703" => Some(Explanation {
            code: "FA703",
            name: "closure-capture",
            category: "Async Safety",
            severity: "error",
            summary: "FrameBox captured by closure or task",
            description: r#"
Closures and spawned tasks can outlive the current frame. When they
capture frame allocations, those allocations become invalid after
end_frame() but the closure/task may still try to use them.

This is especially dangerous with:
- move || closures
- tokio::spawn() / async_std::spawn()
- rayon parallel iterators
- Thread pool submissions
"#,
            example_bad: r#"
let data = alloc.frame_box(expensive_compute());

tokio::spawn(move || {  // FA703: frame data captured
    process(data);  // data invalid after end_frame!
});

alloc.end_frame();
"#,
            example_good: r#"
let data = alloc.heap_box(expensive_compute());

tokio::spawn(move || {
    process(data);  // heap data is valid
});

alloc.end_frame();
"#,
            see_also: &["FA701", "FA702", "FA201"],
        }),
        
        "FA801" => Some(Explanation {
            code: "FA801",
            name: "tag-mismatch",
            category: "Architecture",
            severity: "warning",
            summary: "Allocation tag mismatch for module",
            description: r#"
When you configure module-to-tag mappings in .fa.toml, this diagnostic
triggers when code allocates with a tag that doesn't match its module.

This helps enforce architectural boundaries:
- Physics code should use "physics" tag
- Rendering code should use "rendering" tag
- etc.

Tag mismatches often indicate:
- Architectural confusion
- Code in the wrong module
- Copy-paste errors
"#,
            example_bad: r#"
// In src/physics/collision.rs
alloc.with_tag("rendering", |a| {  // FA801: wrong tag for physics module
    let contacts = a.frame_vec();
});
"#,
            example_good: r#"
// In src/physics/collision.rs
alloc.with_tag("physics", |a| {  // Correct tag
    let contacts = a.frame_vec();
});
"#,
            see_also: &["FA802", "FA803"],
        }),
        
        "FA802" => Some(Explanation {
            code: "FA802",
            name: "unknown-tag",
            category: "Architecture",
            severity: "hint",
            summary: "Unknown allocation tag",
            description: r#"
This hint triggers when you use an allocation tag that isn't in your
project's known_tags list in .fa.toml.

This helps catch typos and maintain consistency:
- "physcs" instead of "physics"
- "rendr" instead of "rendering"

To fix, either:
- Add the tag to known_tags in .fa.toml
- Use an existing tag
"#,
            example_bad: r#"
alloc.with_tag("physcs", |a| {  // FA802: typo!
    // ...
});
"#,
            example_good: r#"
alloc.with_tag("physics", |a| {  // Correct spelling
    // ...
});
"#,
            see_also: &["FA801"],
        }),
        
        "FA201" => Some(Explanation {
            code: "FA201",
            name: "cross-thread-frame",
            category: "Threading",
            severity: "error",
            summary: "Cross-thread frame access without explicit transfer",
            description: r#"
Frame allocations are thread-local. Each thread has its own frame arena.
When you pass frame data to another thread without using TransferHandle,
you risk undefined behavior.

v0.6.0 introduces explicit transfers via TransferHandle. Use it to
declare cross-thread intent and make the cost visible.
"#,
            example_bad: r#"
let data = alloc.frame_box(compute());

std::thread::spawn(move || {  // FA201: implicit cross-thread
    process(data);  // Wrong arena!
});
"#,
            example_good: r#"
// Use explicit transfer
let handle = alloc.frame_box_for_transfer(compute());

std::thread::spawn(move || {
    let data = handle.receive();  // Explicit acceptance
    process(data);
});
"#,
            see_also: &["FA202", "FA703"],
        }),
        
        "FA202" => Some(Explanation {
            code: "FA202",
            name: "barrier-mismatch",
            category: "Threading",
            severity: "warning",
            summary: "Thread not in FrameBarrier but shares frame boundary",
            description: r#"
When multiple threads share frame boundaries (calling end_frame),
they should be coordinated via FrameBarrier to prevent races.

This warning triggers when a thread calls end_frame() but isn't
registered with the FrameBarrier that other threads are using.
"#,
            example_bad: r#"
let barrier = FrameBarrier::new(2);  // Main + worker1

// worker2 not in barrier but calls end_frame
worker2.spawn(|| {
    alloc.end_frame();  // FA202: not coordinated
});
"#,
            example_good: r#"
let barrier = FrameBarrier::new(3);  // Main + worker1 + worker2

// All threads coordinate
barrier.signal_frame_complete();
barrier.wait_all();
alloc.end_frame();
"#,
            see_also: &["FA201", "FA205"],
        }),
        
        "FA203" => Some(Explanation {
            code: "FA203",
            name: "budget-not-configured",
            category: "Threading",
            severity: "hint",
            summary: "Thread allocates without explicit budget configuration",
            description: r#"
Per-thread budgets help prevent unexpected memory growth and make
memory usage predictable. This hint suggests configuring explicit
budgets for threads that perform allocations.

Not always an error, but worth considering for production code.
"#,
            example_bad: r#"
// Thread allocates without budget
std::thread::spawn(|| {
    loop {
        let data = alloc.frame_alloc();  // FA203: no budget
    }
});
"#,
            example_good: r#"
// Configure budget before spawning
alloc.set_thread_frame_budget(thread_id, megabytes(8));

std::thread::spawn(|| {
    loop {
        if alloc.frame_remaining() > size {
            let data = alloc.frame_alloc();
        }
    }
});
"#,
            see_also: &["FA204"],
        }),
        
        "FA204" => Some(Explanation {
            code: "FA204",
            name: "deferred-overflow-risk",
            category: "Threading",
            severity: "warning",
            summary: "Pattern may overflow deferred free queue",
            description: r#"
Cross-thread frees go through a deferred queue. If this queue grows
unbounded, it can cause memory pressure or latency spikes when drained.

Configure a bounded queue with DeferredConfig to prevent this.
"#,
            example_bad: r#"
// High-frequency cross-thread frees
for _ in 0..10000 {
    let data = alloc.frame_box(x);
    other_thread.send(data);  // FA204: unbounded queue growth
}
"#,
            example_good: r#"
// Configure bounded queue
let config = DeferredConfig::bounded(1024);
alloc.set_deferred_config(config);

// Or use incremental processing
let config = DeferredConfig::incremental(16);
"#,
            see_also: &["FA203"],
        }),
        
        "FA205" => Some(Explanation {
            code: "FA205",
            name: "frame-sync-race",
            category: "Threading",
            severity: "error",
            summary: "end_frame() called without barrier synchronization",
            description: r#"
In multi-threaded contexts, calling end_frame() without barrier
synchronization can cause races where one thread resets frame memory
while another is still using it.

Use FrameBarrier to coordinate frame boundaries across threads.
"#,
            example_bad: r#"
// Thread 1
alloc.end_frame();  // FA205: not synchronized

// Thread 2 (concurrent)
let data = alloc.frame_alloc();  // May use reset memory!
"#,
            example_good: r#"
let barrier = FrameBarrier::new(2);

// Thread 1
barrier.signal_frame_complete();
barrier.wait_all();
alloc.end_frame();

// Thread 2
barrier.signal_frame_complete();
// Will wait until frame is safe
"#,
            see_also: &["FA202"],
        }),
        
        "FA301" => Some(Explanation {
            code: "FA301",
            name: "unbounded-allocation",
            category: "Budget",
            severity: "hint",
            summary: "Loop contains multiple allocation calls",
            description: r#"
This hint triggers when a loop contains multiple allocation calls,
which may indicate unbounded memory growth depending on iteration count.

While not always a bug, this pattern deserves attention:
- Is the loop iteration count bounded?
- Are you staying within budget?
- Could allocations be batched or pre-allocated?
"#,
            example_bad: r#"
for item in unbounded_iterator() {  // FA301: how many iterations?
    let a = alloc.pool_alloc();
    let b = alloc.frame_alloc();
}
"#,
            example_good: r#"
// Bounded iteration with budget check
for item in items.iter().take(MAX_ITEMS) {
    if alloc.remaining_budget() < ITEM_SIZE {
        break;
    }
    let a = alloc.pool_alloc();
}
"#,
            see_also: &["FA602"],
        }),
        
        "FA801" => Some(Explanation {
            code: "FA801",
            name: "staging-buffer-leak",
            category: "GPU",
            severity: "warning",
            summary: "Staging buffer not freed before frame end",
            description: r#"
Staging buffers are temporary CPU-side buffers used to transfer data to the GPU.
They should be freed or transferred before end_frame() to avoid memory leaks.

Unfreed staging buffers accumulate in GPU memory, leading to:
- Increased memory usage
- Potential out-of-memory errors
- Reduced performance due to memory fragmentation

This commonly happens when:
- Creating staging buffers but forgetting to transfer them
- Storing staging buffer references beyond the frame
- Missing end_frame() calls
"#,
            example_bad: r#"
fn upload_vertices(alloc: &mut UnifiedAllocator) {
    let staging = alloc.create_staging_buffer(1024).unwrap();
    // Fill staging buffer...
    // FA801: staging buffer not transferred or freed before frame end
    alloc.end_frame();  // staging buffer leaks!
}
"#,
            example_good: r#"
fn upload_vertices(alloc: &mut UnifiedAllocator) {
    let staging = alloc.create_staging_buffer(1024).unwrap();
    // Fill staging buffer...
    
    // Transfer to GPU (frees staging buffer)
    alloc.transfer_to_gpu(&mut staging).unwrap();
    alloc.end_frame();  // No leak
}
"#,
            see_also: &["FA802", "FA803"],
        }),
        
        "FA802" => Some(Explanation {
            code: "FA802",
            name: "missing-transfer-usage",
            category: "GPU",
            severity: "error",
            summary: "GPU buffer created without transfer usage flags",
            description: r#"
Device-local GPU buffers cannot be accessed directly by the CPU.
To transfer data to them, they must be created with TRANSFER_DST usage.

Without TRANSFER_DST:
- transfer_to_gpu() will fail at runtime
- CPU data cannot be copied to the buffer
- The buffer remains uninitialized

This is required for:
- Vertex buffers
- Index buffers
- Uniform buffers
- Storage buffers that receive CPU data
"#,
            example_bad: r#"
// FA802: Missing TRANSFER_DST usage
let gpu_buffer = alloc.create_gpu_buffer(
    1024,
    BufferUsage::VERTEX_BUFFER,  // Missing TRANSFER_DST
    MemoryType::DeviceLocal,
).unwrap();
"#,
            example_good: r#"
// Correct: Include TRANSFER_DST for CPU-GPU transfers
let gpu_buffer = alloc.create_gpu_buffer(
    1024,
    BufferUsage::VERTEX_BUFFER | BufferUsage::TRANSFER_DST,
    MemoryType::DeviceLocal,
).unwrap();
"#,
            see_also: &["FA801", "FA804"],
        }),
        
        "FA803" => Some(Explanation {
            code: "FA803",
            name: "missing-synchronization",
            category: "GPU",
            severity: "warning",
            summary: "CPU-GPU transfer without synchronization barrier",
            description: r#"
GPU operations execute asynchronously. Without proper synchronization,
you may access data before the GPU has finished writing to it.

Missing synchronization can cause:
- Data corruption
- Access violations
- Visual artifacts in graphics
- Crashes

Always ensure:
- GPU commands are submitted before reading back data
- Proper barriers are in place for read-after-write hazards
- Fence or semaphore synchronization when needed
"#,
            example_bad: r#"
let staging = alloc.create_staging_buffer(1024).unwrap();
alloc.transfer_to_gpu(&mut staging).unwrap();

// FA803: No synchronization - GPU might still be writing
unsafe { read_gpu_data(gpu_buffer); }  // Potential data corruption!
"#,
            example_good: r#"
let staging = alloc.create_staging_buffer(1024).unwrap();
alloc.transfer_to_gpu(&mut staging).unwrap();

// Wait for GPU to complete transfer
let barrier = CpuGpuBarrier::new();
barrier.wait_current_frame();

// Now safe to read
unsafe { read_gpu_data(gpu_buffer); }
"#,
            see_also: &["FA801", "FA802"],
        }),
        
        "FA804" => Some(Explanation {
            code: "FA804",
            name: "device-local-mapped",
            category: "GPU",
            severity: "error",
            summary: "Device-local buffer mapped for CPU access",
            description: r#"
Device-local memory is optimized for GPU access and cannot be mapped
for direct CPU access. Attempting to map it will fail at runtime.

Device-local memory characteristics:
- Fast GPU access
- No CPU access
- Cannot be mapped
- Requires staging buffers for data transfer

For CPU-accessible memory, use:
- MemoryType::HostVisible - CPU can map and read/write
- MemoryType::HostCoherent - CPU writes are automatically visible to GPU
"#,
            example_bad: r#"
// FA804: Device-local memory cannot be mapped
let gpu_buffer = alloc.create_gpu_buffer(
    1024,
    BufferUsage::VERTEX_BUFFER | BufferUsage::TRANSFER_DST,
    MemoryType::DeviceLocal,
).unwrap();

let ptr = gpu_buffer.map();  // Runtime error!
"#,
            example_good: r#"
// Use host-visible memory for CPU mapping
let cpu_buffer = alloc.create_gpu_buffer(
    1024,
    BufferUsage::VERTEX_BUFFER,
    MemoryType::HostVisible,
).unwrap();

let ptr = cpu_buffer.map();  // OK!
"#,
            see_also: &["FA802", "FA805"],
        }),
        
        "FA805" => Some(Explanation {
            code: "FA805",
            name: "staging-buffer-reuse",
            category: "GPU",
            severity: "warning",
            summary: "Staging buffer reused across frames without reset",
            description: r#"
Reusing staging buffers across frames without proper reset can lead to:
- Data corruption from previous frame
- Stale data being transferred
- Memory leaks if not properly managed

Best practices:
- Create fresh staging buffers each frame, OR
- Properly reset buffers with begin_frame()
- Never store staging buffer references across frames
"#,
            example_bad: r#"
struct Renderer {
    staging_buffer: Option<UnifiedBuffer>,  // FA805: Persists across frames
}

impl Renderer {
    fn upload(&mut self, data: &[u8]) {
        if self.staging_buffer.is_none() {
            self.staging_buffer = Some(create_staging_buffer());
        }
        
        // Buffer contains stale data from previous frame!
        let buffer = self.staging_buffer.as_mut().unwrap();
        buffer.cpu_slice_mut().unwrap().copy_from_slice(data);
    }
}
"#,
            example_good: r#"
fn upload_frame(alloc: &mut UnifiedAllocator, data: &[u8]) {
    // Fresh buffer each frame
    let staging = alloc.create_staging_buffer(data.len()).unwrap();
    staging.cpu_slice_mut().unwrap().copy_from_slice(data);
    alloc.transfer_to_gpu(&mut staging).unwrap();
    // Buffer automatically freed when dropped
}
"#,
            see_also: &["FA801", "FA804"],
        }),
        
        _ => None,
    }
}

/// Print explanation to terminal
pub fn print_explanation(explanation: &Explanation) {
    println!();
    println!("{}", format!("━━━ {} ━━━", explanation.code).cyan().bold());
    println!();
    
    println!("{}: {}", "Name".bold(), explanation.name);
    println!("{}: {}", "Category".bold(), explanation.category);
    println!("{}: {}", "Severity".bold(), match explanation.severity {
        "error" => explanation.severity.red().to_string(),
        "warning" => explanation.severity.yellow().to_string(),
        _ => explanation.severity.cyan().to_string(),
    });
    println!();
    
    println!("{}", "Summary".bold().underline());
    println!("{}", explanation.summary);
    println!();
    
    println!("{}", "Description".bold().underline());
    for line in explanation.description.trim().lines() {
        println!("{}", line);
    }
    println!();
    
    println!("{}", "Example (incorrect)".red().bold());
    println!("```rust");
    for line in explanation.example_bad.trim().lines() {
        println!("{}", line);
    }
    println!("```");
    println!();
    
    println!("{}", "Example (correct)".green().bold());
    println!("```rust");
    for line in explanation.example_good.trim().lines() {
        println!("{}", line);
    }
    println!("```");
    println!();
    
    if !explanation.see_also.is_empty() {
        println!("{}: {}", "See also".bold(), explanation.see_also.join(", "));
    }
    
    println!();
    println!(
        "{}: {}",
        "Documentation".dimmed(),
        format!("https://docs.rs/framealloc/diagnostics#{}", explanation.code)
    );
    println!();
}

/// List all diagnostic codes
pub fn list_all_codes(category_filter: Option<&str>) {
    let codes = [
        ("FA201", "Threading", "Cross-thread frame access without transfer"),
        ("FA202", "Threading", "Thread not in FrameBarrier"),
        ("FA203", "Threading", "Thread budget not configured"),
        ("FA204", "Threading", "Deferred queue overflow risk"),
        ("FA205", "Threading", "Frame sync race (end_frame without barrier)"),
        ("FA301", "Budget", "Unbounded allocation in loop"),
        ("FA601", "Lifetime", "Frame allocation escapes scope"),
        ("FA602", "Lifetime", "Allocation in hot loop"),
        ("FA603", "Lifetime", "Missing frame boundaries"),
        ("FA604", "Lifetime", "Retention policy mismatch"),
        ("FA605", "Lifetime", "Discard policy stored beyond frame"),
        ("FA701", "Async", "Frame allocation in async function"),
        ("FA702", "Async", "Frame allocation crosses await"),
        ("FA703", "Async", "FrameBox captured by closure"),
        ("FA801", "GPU", "Staging buffer not freed before frame end"),
        ("FA802", "GPU", "GPU buffer created without transfer usage flags"),
        ("FA803", "GPU", "CPU-GPU transfer without synchronization barrier"),
        ("FA804", "GPU", "Device-local buffer mapped for CPU access"),
        ("FA805", "GPU", "Staging buffer reused across frames without reset"),
        ("FA901", "Architecture", "Tag mismatch"),
        ("FA902", "Architecture", "Unknown tag"),
        ("FA903", "Architecture", "Cross-module allocation"),
    ];
    
    println!();
    println!("{}", "Available Diagnostic Codes".bold().underline());
    println!();
    
    let mut current_category = "";
    
    for (code, category, description) in codes {
        // Filter by category if specified
        if let Some(filter) = category_filter {
            if !category.to_lowercase().contains(&filter.to_lowercase()) {
                continue;
            }
        }
        
        // Print category header
        if category != current_category {
            if !current_category.is_empty() {
                println!();
            }
            println!("{}", format!("  {}", category).cyan().bold());
            current_category = category;
        }
        
        println!("    {} - {}", code.yellow(), description);
    }
    
    println!();
    println!("{}", "Run `cargo fa explain <CODE>` for detailed information".dimmed());
    println!();
}
