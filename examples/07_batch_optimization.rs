//! Batch Optimization example
//! 
//! Demonstrates high-performance batch allocation patterns

use framealloc::SmartAlloc;
use std::time::Instant;

#[derive(Debug, Clone, Copy)]
struct Particle {
    position: [f32; 3],
    velocity: [f32; 3],
    color: [u8; 4],
    life: f32,
}

impl Particle {
    fn new() -> Self {
        Self {
            position: [0.0; 3],
            velocity: [
                (rand::random::<f32>() - 0.5) * 10.0,
                (rand::random::<f32>() - 0.5) * 10.0,
                (rand::random::<f32>() - 0.5) * 10.0,
            ],
            color: [255; 4],
            life: 1.0,
        }
    }
    
    fn update(&mut self, dt: f32) {
        self.position[0] += self.velocity[0] * dt;
        self.position[1] += self.velocity[1] * dt;
        self.position[2] += self.velocity[2] * dt;
        self.velocity[1] -= 9.81 * dt; // Gravity
        self.life -= dt;
    }
}

fn individual_allocation(count: usize) -> Vec<Particle> {
    let alloc = SmartAlloc::new(Default::default());
    
    alloc.begin_frame();
    
    let mut particles = Vec::with_capacity(count);
    for _ in 0..count {
        // Individual allocation - SLOW
        let particle = alloc.frame_alloc::<Particle>();
        particles.push(*particle);
    }
    
    alloc.end_frame();
    particles
}

fn batch_allocation(count: usize) -> Vec<Particle> {
    let alloc = SmartAlloc::new(Default::default());
    
    alloc.begin_frame();
    
    // Batch allocation - FAST
    let batch = unsafe {
        let batch = alloc.frame_alloc_batch::<Particle>(count);
        for i in 0..count {
            let particle = batch.add(i);
            std::ptr::write(particle, Particle::new());
        }
        batch
    };
    
    // Copy to owned vector for demonstration
    let mut particles = Vec::with_capacity(count);
    for i in 0..count {
        particles.push(unsafe { *batch.get(i) });
    }
    
    alloc.end_frame();
    particles
}

fn specialized_batch_sizes() {
    let alloc = SmartAlloc::new(Default::default());
    
    alloc.begin_frame();
    
    // Specialized sizes - ZERO overhead
    let [p1, p2] = alloc.frame_alloc_2::<Particle>();
    let [p3, p4, p5, p6] = alloc.frame_alloc_4::<Particle>();
    let batch8 = alloc.frame_alloc_8::<Particle>();
    
    // Initialize
    *p1 = Particle::new();
    *p2 = Particle::new();
    *p3 = Particle::new();
    *p4 = Particle::new();
    *p5 = Particle::new();
    *p6 = Particle::new();
    
    for i in 0..8 {
        batch8[i] = Particle::new();
    }
    
    println!("Specialized batches:");
    println!("  2 items: {:p}, {:p}", p1, p2);
    println!("  4 items: {:p}, {:p}, {:p}, {:p}", p3, p4, p5, p6);
    println!("  8 items: {:p}..{:p}", &batch8[0], &batch8[7]);
    
    alloc.end_frame();
}

fn particle_system_simulation() {
    let alloc = SmartAlloc::new(Default::default());
    const PARTICLE_COUNT: usize = 100_000;
    const FRAME_COUNT: usize = 60;
    
    println!("\n=== Particle System Simulation ===");
    println!("Simulating {} particles for {} frames", PARTICLE_COUNT, FRAME_COUNT);
    
    let mut total_time = Duration::new(0, 0);
    
    for frame in 0..FRAME_COUNT {
        let frame_start = Instant::now();
        
        alloc.begin_frame();
        
        // Batch allocate all particles
        let particles = unsafe {
            let batch = alloc.frame_alloc_batch::<Particle>(PARTICLE_COUNT);
            for i in 0..PARTICLE_COUNT {
                let particle = batch.add(i);
                if frame == 0 {
                    std::ptr::write(particle, Particle::new());
                }
            }
            batch
        };
        
        // Update all particles (SIMD-friendly)
        let dt = 1.0 / 60.0;
        for i in 0..PARTICLE_COUNT {
            let particle = unsafe { &mut *particles.get_mut(i) };
            particle.update(dt);
        }
        
        // Count alive particles
        let mut alive_count = 0;
        for i in 0..PARTICLE_COUNT {
            let particle = unsafe { particles.get(i) };
            if particle.life > 0.0 {
                alive_count += 1;
            }
        }
        
        alloc.end_frame();
        
        let frame_time = frame_start.elapsed();
        total_time += frame_time;
        
        if frame % 10 == 0 {
            println!("Frame {}: {} alive, {:?}", frame, alive_count, frame_time);
        }
    }
    
    let avg_frame_time = total_time / FRAME_COUNT as u32;
    println!("Average frame time: {:?}", avg_frame_time);
    println!("Particles per second: {:.0}", 
        PARTICLE_COUNT as f64 / avg_frame_time.as_secs_f64());
}

fn benchmark_comparison() {
    const COUNTS: &[usize] = &[100, 1000, 10000, 100000];
    
    println!("\n=== Performance Comparison ===");
    println!("Testing individual vs batch allocation:");
    
    for &count in COUNTS {
        // Individual allocation
        let start = Instant::now();
        let _individual = individual_allocation(count);
        let individual_time = start.elapsed();
        
        // Batch allocation
        let start = Instant::now();
        let _batch = batch_allocation(count);
        let batch_time = start.elapsed();
        
        let speedup = individual_time.as_nanos() as f64 / batch_time.as_nanos() as f64;
        
        println!("  {}: individual={:?}, batch={:?}, {:.1}x speedup", 
            count, individual_time, batch_time, speedup);
    }
}

fn memory_layout_optimization() {
    println!("\n=== Memory Layout Optimization ===");
    
    let alloc = SmartAlloc::new(Default::default());
    
    alloc.begin_frame();
    
    // Structure of Arrays (SoA) - better cache locality
    struct ParticleSystemSoA {
        positions: FrameBox<[[f32; 3]>>,
        velocities: FrameBox<[[f32; 3]>>,
        colors: FrameBox<[[u8; 4]>>,
        lives: FrameBox<[f32]>,
    }
    
    impl ParticleSystemSoA {
        fn new(count: usize, alloc: &SmartAlloc) -> Self {
            Self {
                positions: alloc.frame_box([[0.0; 3]; count]),
                velocities: alloc.frame_box([[0.0; 3]; count]),
                colors: alloc.frame_box([[255; 4]; count]),
                lives: alloc.frame_box([0.0; count]),
            }
        }
        
        fn update(&mut self, dt: f32) {
            // SIMD-friendly update
            for i in 0..self.lives.len() {
                // Update positions
                self.positions[i][0] += self.velocities[i][0] * dt;
                self.positions[i][1] += self.velocities[i][1] * dt;
                self.positions[i][2] += self.velocities[i][2] * dt;
                
                // Update velocities
                self.velocities[i][1] -= 9.81 * dt;
                
                // Update lives
                self.lives[i] -= dt;
            }
        }
    }
    
    let mut soa_system = ParticleSystemSoA::new(10000, &alloc);
    
    // Update simulation
    for frame in 0..60 {
        soa_system.update(1.0 / 60.0);
        
        if frame % 20 == 0 {
            let alive = soa_system.lives.iter().filter(|&&l| l > 0.0).count();
            println!("  Frame {}: {} particles alive", frame, alive);
        }
    }
    
    alloc.end_frame();
}

fn zero_copy_buffer_sharing() {
    println!("\n=== Zero-Copy Buffer Sharing ===");
    
    let alloc = SmartAlloc::new(Default::default());
    
    alloc.begin_frame();
    
    // Create a large buffer
    let buffer = alloc.frame_slice::<f32>(10000);
    for i in 0..10000 {
        buffer[i] = i as f32;
    }
    
    // Share without copying
    struct BufferView<'a> {
        data: &'a [f32],
        offset: usize,
        len: usize,
    }
    
    impl<'a> BufferView<'a> {
        fn new(buffer: &'a [f32], offset: usize, len: usize) -> Self {
            Self {
                data: &buffer[offset..offset + len],
                offset,
                len,
            }
        }
        
        fn sum(&self) -> f64 {
            self.data.iter().map(|&x| x as f64).sum()
        }
    }
    
    // Create multiple views without copying
    let view1 = BufferView::new(buffer, 0, 2500);
    let view2 = BufferView::new(buffer, 2500, 2500);
    let view3 = BufferView::new(buffer, 5000, 2500);
    let view4 = BufferView::new(buffer, 7500, 2500);
    
    println!("  Buffer views (zero-copy):");
    println!("    View 1 sum: {:.0}", view1.sum());
    println!("    View 2 sum: {:.0}", view2.sum());
    println!("    View 3 sum: {:.0}", view3.sum());
    println!("    View 4 sum: {:.0}", view4.sum());
    
    alloc.end_frame();
}

fn main() {
    println!("=== Batch Optimization Demo ===\n");
    
    // Run all demonstrations
    specialized_batch_sizes();
    benchmark_comparison();
    particle_system_simulation();
    memory_layout_optimization();
    zero_copy_buffer_sharing();
    
    println!("\nAll batch optimization demos completed!");
    
    // Performance tips
    println!("\n=== Performance Tips ===");
    println!("1. Use batch allocation for >100 items");
    println!("2. Use specialized sizes for known small counts");
    println!("3. Structure of Arrays improves SIMD performance");
    println!("4. Zero-copy views reduce memory bandwidth");
    println!("5. Always profile with realistic data sizes");
}

use std::time::Duration;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_batch_vs_individual() {
        const COUNT: usize = 1000;
        
        let individual = individual_allocation(COUNT);
        let batch = batch_allocation(COUNT);
        
        assert_eq!(individual.len(), COUNT);
        assert_eq!(batch.len(), COUNT);
    }
    
    #[test]
    fn test_specialized_sizes() {
        let alloc = SmartAlloc::new(Default::default());
        
        alloc.begin_frame();
        let [a, b] = alloc.frame_alloc_2::<u32>();
        let [c, d, e, f] = alloc.frame_alloc_4::<u32>();
        
        *a = 1;
        *b = 2;
        *c = 3;
        *d = 4;
        *e = 5;
        *f = 6;
        
        assert_eq!(*a, 1);
        assert_eq!(*b, 2);
        assert_eq!(*c, 3);
        assert_eq!(*d, 4);
        assert_eq!(*e, 5);
        assert_eq!(*f, 6);
        
        alloc.end_frame();
    }
}
