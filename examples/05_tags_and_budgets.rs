//! Tags and Budgets example
//! 
//! Demonstrates tagged allocations and memory budgeting

use framealloc::SmartAlloc;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy)]
enum SystemTag {
    Physics,
    Rendering,
    Audio,
    AI,
    Network,
}

impl SystemTag {
    fn as_str(&self) -> &'static str {
        match self {
            SystemTag::Physics => "physics",
            SystemTag::Rendering => "rendering",
            SystemTag::Audio => "audio",
            SystemTag::AI => "ai",
            SystemTag::Network => "network",
        }
    }
}

#[derive(Debug, Default)]
struct SystemStats {
    allocations: usize,
    bytes_used: usize,
    peak_usage: usize,
}

impl SystemStats {
    fn record_allocation(&mut self, size: usize) {
        self.allocations += 1;
        self.bytes_used += size;
        self.peak_usage = self.peak_usage.max(self.bytes_used);
    }
}

struct BudgetManager {
    budgets: HashMap<String, usize>,
    current_usage: HashMap<String, usize>,
    stats: HashMap<String, SystemStats>,
}

impl BudgetManager {
    fn new() -> Self {
        let mut budgets = HashMap::new();
        budgets.insert("physics".to_string(), 1024 * 1024); // 1MB
        budgets.insert("rendering".to_string(), 10 * 1024 * 1024); // 10MB
        budgets.insert("audio".to_string(), 512 * 1024); // 512KB
        budgets.insert("ai".to_string(), 2 * 1024 * 1024); // 2MB
        budgets.insert("network".to_string(), 256 * 1024); // 256KB
        
        Self {
            budgets,
            current_usage: HashMap::new(),
            stats: HashMap::new(),
        }
    }
    
    fn check_budget(&mut self, tag: &str, size: usize) -> bool {
        let budget = *self.budgets.get(tag).unwrap_or(&0);
        let current = *self.current_usage.get(tag).unwrap_or(&0);
        
        if current + size <= budget {
            self.current_usage.insert(tag.to_string(), current + size);
            self.stats.entry(tag.to_string())
                .or_default()
                .record_allocation(size);
            true
        } else {
            println!("Budget exceeded for '{}': {} + {} > {}", 
                tag, current, size, budget);
            false
        }
    }
    
    fn reset_frame(&mut self) {
        for (_, usage) in self.current_usage.iter_mut() {
            *usage = 0;
        }
    }
    
    fn print_stats(&self) {
        println!("\n=== System Memory Stats ===");
        for (tag, stats) in &self.stats {
            let budget = self.budgets.get(tag).unwrap_or(&0);
            let usage_percent = if *budget > 0 {
                (*budget as f64 / stats.peak_usage as f64 * 100.0) as u32
            } else {
                0
            };
            
            println!("{}: {} allocations, {} bytes peak, {}% of budget", 
                tag, stats.allocations, stats.peak_usage, usage_percent);
        }
    }
}

fn simulate_physics(alloc: &SmartAlloc, budget: &mut BudgetManager) {
    alloc.with_tag("physics", |a| {
        // Contact manifold data
        if budget.check_budget("physics", 1024 * 100) {
            let contacts = a.frame_slice::<Contact>(100);
            for i in 0..100 {
                contacts[i] = Contact::new(i as f32);
            }
            println!("Physics: Processed {} contacts", contacts.len());
        }
        
        // Force accumulators
        if budget.check_budget("physics", 1024 * 50) {
            let forces = a.frame_slice::<Vector3>(50);
            for i in 0..50 {
                forces[i] = Vector3::new(0.0, -9.81, 0.0);
            }
            println!("Physics: Applied {} forces", forces.len());
        }
    });
}

fn simulate_rendering(alloc: &SmartAlloc, budget: &mut BudgetManager) {
    alloc.with_tag("rendering", |a| {
        // Vertex buffer
        if budget.check_budget("rendering", 1024 * 1024) {
            let vertices = a.frame_slice::<Vertex>(65536);
            for i in 0..65536 {
                vertices[i] = Vertex::new(
                    [i as f32, (i % 256) as f32, 0.0],
                    [255, 255, 255, 255],
                );
            }
            println!("Rendering: Generated {} vertices", vertices.len());
        }
        
        // Draw commands
        if budget.check_budget("rendering", 1024 * 10) {
            let commands = a.frame_vec::<DrawCommand>();
            for i in 0..1000 {
                commands.push(DrawCommand::DrawMesh {
                    mesh_id: i,
                    transform: Transform::identity(),
                });
            }
            println!("Rendering: {} draw commands", commands.len());
        }
    });
}

fn simulate_audio(alloc: &SmartAlloc, budget: &mut BudgetManager) {
    alloc.with_tag("audio", |a| {
        // Audio buffer
        if budget.check_budget("audio", 1024 * 4) {
            let buffer = a.frame_slice::<f32>(1024);
            for i in 0..1024 {
                buffer[i] = (i as f32 * 0.01).sin();
            }
            println!("Audio: Generated {} audio samples", buffer.len());
        }
        
        // Active sounds
        if budget.check_budget("audio", 1024) {
            let sounds = a.frame_vec::<ActiveSound>();
            for i in 0..10 {
                sounds.push(ActiveSound {
                    id: i,
                    volume: 0.5,
                    pitch: 1.0,
                });
            }
            println!("Audio: {} active sounds", sounds.len());
        }
    });
}

fn simulate_ai(alloc: &SmartAlloc, budget: &mut BudgetManager) {
    alloc.with_tag("ai", |a| {
        // Pathfinding nodes
        if budget.check_budget("ai", 1024 * 100) {
            let nodes = a.frame_slice::<PathNode>(1000);
            for i in 0..1000 {
                nodes[i] = PathNode::new(i % 100, i / 100);
            }
            println!("AI: Processed {} path nodes", nodes.len());
        }
        
        // Behavior trees
        if budget.check_budget("ai", 1024 * 50) {
            let trees = a.frame_vec::<BehaviorTree>();
            for i in 0..50 {
                trees.push(BehaviorTree::new(i));
            }
            println!("AI: {} behavior trees", trees.len());
        }
    });
}

fn simulate_network(alloc: &SmartAlloc, budget: &mut BudgetManager) {
    alloc.with_tag("network", |a| {
        // Packet buffer
        if budget.check_budget("network", 1024 * 2) {
            let packets = a.frame_slice::<Packet>(100);
            for i in 0..100 {
                packets[i] = Packet::new(i, format!("Data {}", i));
            }
            println!("Network: Processed {} packets", packets.len());
        }
    });
}

fn main() {
    let alloc = SmartAlloc::new(Default::default());
    let mut budget = BudgetManager::new();
    
    println!("=== Tags and Budgets Demo ===\n");
    
    // Simulate 5 frames
    for frame in 0..5 {
        println!("--- Frame {} ---", frame + 1);
        
        alloc.begin_frame();
        
        // Simulate each system
        simulate_physics(&alloc, &mut budget);
        simulate_rendering(&alloc, &mut budget);
        simulate_audio(&alloc, &mut budget);
        simulate_ai(&alloc, &mut budget);
        simulate_network(&alloc, &mut budget);
        
        alloc.end_frame();
        
        // Reset for next frame
        budget.reset_frame();
        
        if frame < 4 {
            thread::sleep(Duration::from_millis(100));
        }
    }
    
    // Print final statistics
    budget.print_stats();
    
    // Demonstrate per-thread budgets
    println!("\n=== Per-Thread Budget Demo ===");
    demonstrate_per_thread_budgets();
    
    println!("\nAll demos completed!");
}

fn demonstrate_per_thread_budgets() {
    let alloc = SmartAlloc::new(Default::default());
    
    // Set thread-specific budget
    alloc.set_thread_frame_budget(512 * 1024); // 512KB per thread
    
    let mut handles = Vec::new();
    
    for thread_id in 0..4 {
        let alloc_clone = alloc.clone();
        
        let handle = thread::spawn(move || {
            alloc_clone.begin_frame();
            
            // Try to allocate within budget
            let data = alloc_clone.frame_vec::<u8>();
            let allocated = 256 * 1024; // 256KB
            
            for i in 0..allocated {
                data.push((i % 256) as u8);
            }
            
            println!("Thread {} allocated {} bytes", thread_id, allocated);
            
            alloc_clone.end_frame();
            
            allocated
        });
        
        handles.push(handle);
    }
    
    for handle in handles {
        let allocated = handle.join().unwrap();
        println!("Thread completed with {} bytes allocated", allocated);
    }
}

// Mock types for demonstration
#[derive(Debug)]
struct Contact {
    normal: [f32; 3],
    depth: f32,
}

impl Contact {
    fn new(id: f32) -> Self {
        Self {
            normal: [0.0, 1.0, 0.0],
            depth: id * 0.01,
        }
    }
}

#[derive(Debug)]
struct Vector3 {
    x: f32,
    y: f32,
    z: f32,
}

#[derive(Debug)]
struct Vertex {
    position: [f32; 3],
    color: [u8; 4],
}

impl Vertex {
    fn new(position: [f32; 3], color: [u8; 4]) -> Self {
        Self { position, color }
    }
}

#[derive(Debug)]
struct DrawCommand {
    mesh_id: usize,
    transform: Transform,
}

#[derive(Debug)]
struct Transform;

impl Transform {
    fn identity() -> Self {
        Self
    }
}

#[derive(Debug)]
struct ActiveSound {
    id: usize,
    volume: f32,
    pitch: f32,
}

#[derive(Debug)]
struct PathNode {
    x: usize,
    y: usize,
}

impl PathNode {
    fn new(x: usize, y: usize) -> Self {
        Self { x, y }
    }
}

#[derive(Debug)]
struct BehaviorTree {
    id: usize,
}

impl BehaviorTree {
    fn new(id: usize) -> Self {
        Self { id }
    }
}

#[derive(Debug)]
struct Packet {
    id: usize,
    data: String,
}

impl Packet {
    fn new(id: usize, data: String) -> Self {
        Self { id, data }
    }
}

use std::thread;
use std::time::Duration;
