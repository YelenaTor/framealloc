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
            summary: "Frame allocation in spawned thread context",
            description: r#"
Frame allocations are thread-local. Each thread has its own frame arena.
When you spawn a new thread, it gets a fresh arena - it cannot access
frame allocations from the parent thread.

This diagnostic catches patterns where frame data appears to be
passed to spawned threads, which would cause undefined behavior.
"#,
            example_bad: r#"
let data = alloc.frame_box(compute());

std::thread::spawn(move || {  // FA201: frame data to new thread
    process(data);  // Wrong arena!
});
"#,
            example_good: r#"
let data = alloc.heap_box(compute());

std::thread::spawn(move || {
    process(data);  // Heap is thread-safe
});
"#,
            see_also: &["FA703"],
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
        ("FA201", "Threading", "Cross-thread frame access"),
        ("FA301", "Budget", "Unbounded allocation in loop"),
        ("FA601", "Lifetime", "Frame allocation escapes scope"),
        ("FA602", "Lifetime", "Allocation in hot loop"),
        ("FA603", "Lifetime", "Missing frame boundaries"),
        ("FA604", "Lifetime", "Retention policy mismatch"),
        ("FA605", "Lifetime", "Discard policy stored beyond frame"),
        ("FA701", "Async", "Frame allocation in async function"),
        ("FA702", "Async", "Frame allocation crosses await"),
        ("FA703", "Async", "FrameBox captured by closure"),
        ("FA801", "Architecture", "Tag mismatch"),
        ("FA802", "Architecture", "Unknown tag"),
        ("FA803", "Architecture", "Cross-module allocation"),
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
