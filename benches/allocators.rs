use criterion::{
    black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput,
};
use framealloc::{SmartAlloc, AllocConfig};
use bumpalo::Bump;
use std::alloc::{alloc, dealloc, Layout};

// =============================================================================
// SINGLE ALLOCATION BENCHMARKS (Various sizes)
// =============================================================================

fn bench_single_alloc_64(c: &mut Criterion) {
    let mut group = c.benchmark_group("single_alloc_64B");
    group.throughput(Throughput::Bytes(64));
    
    group.bench_function("framealloc", |b| {
        let alloc = SmartAlloc::new(AllocConfig::default());
        alloc.begin_frame();
        b.iter(|| {
            black_box(alloc.frame_alloc::<[u8; 64]>());
        });
        alloc.end_frame();
    });
    
    group.bench_function("bumpalo", |b| {
        let bump = Bump::with_capacity(64 * 1024 * 1024);
        b.iter(|| {
            black_box(bump.alloc([0u8; 64]));
        });
    });
    
    group.bench_function("system_malloc", |b| {
        b.iter(|| unsafe {
            let layout = Layout::from_size_align(64, 8).unwrap();
            let ptr = alloc(layout);
            black_box(ptr);
            dealloc(ptr, layout);
        });
    });
    
    group.bench_function("box_vec", |b| {
        b.iter(|| {
            let v: Box<[u8; 64]> = Box::new([0u8; 64]);
            black_box(v);
        });
    });
    
    group.finish();
}

fn bench_single_alloc_256(c: &mut Criterion) {
    let mut group = c.benchmark_group("single_alloc_256B");
    group.throughput(Throughput::Bytes(256));
    
    group.bench_function("framealloc", |b| {
        let alloc = SmartAlloc::new(AllocConfig::default());
        alloc.begin_frame();
        b.iter(|| {
            black_box(alloc.frame_alloc::<[u8; 256]>());
        });
        alloc.end_frame();
    });
    
    group.bench_function("bumpalo", |b| {
        let bump = Bump::with_capacity(64 * 1024 * 1024);
        b.iter(|| {
            black_box(bump.alloc([0u8; 256]));
        });
    });
    
    group.bench_function("system_malloc", |b| {
        b.iter(|| unsafe {
            let layout = Layout::from_size_align(256, 8).unwrap();
            let ptr = alloc(layout);
            black_box(ptr);
            dealloc(ptr, layout);
        });
    });
    
    group.finish();
}

fn bench_single_alloc_1kb(c: &mut Criterion) {
    let mut group = c.benchmark_group("single_alloc_1KB");
    group.throughput(Throughput::Bytes(1024));
    
    group.bench_function("framealloc", |b| {
        let alloc = SmartAlloc::new(AllocConfig::default());
        alloc.begin_frame();
        b.iter(|| {
            black_box(alloc.frame_alloc::<[u8; 1024]>());
        });
        alloc.end_frame();
    });
    
    group.bench_function("bumpalo", |b| {
        let bump = Bump::with_capacity(64 * 1024 * 1024);
        b.iter(|| {
            black_box(bump.alloc([0u8; 1024]));
        });
    });
    
    group.bench_function("system_malloc", |b| {
        b.iter(|| unsafe {
            let layout = Layout::from_size_align(1024, 8).unwrap();
            let ptr = alloc(layout);
            black_box(ptr);
            dealloc(ptr, layout);
        });
    });
    
    group.finish();
}

fn bench_single_alloc_4kb(c: &mut Criterion) {
    let mut group = c.benchmark_group("single_alloc_4KB");
    group.throughput(Throughput::Bytes(4096));
    
    group.bench_function("framealloc", |b| {
        let alloc = SmartAlloc::new(AllocConfig::default());
        alloc.begin_frame();
        b.iter(|| {
            black_box(alloc.frame_alloc::<[u8; 4096]>());
        });
        alloc.end_frame();
    });
    
    group.bench_function("bumpalo", |b| {
        let bump = Bump::with_capacity(64 * 1024 * 1024);
        b.iter(|| {
            black_box(bump.alloc([0u8; 4096]));
        });
    });
    
    group.bench_function("system_malloc", |b| {
        b.iter(|| unsafe {
            let layout = Layout::from_size_align(4096, 8).unwrap();
            let ptr = alloc(layout);
            black_box(ptr);
            dealloc(ptr, layout);
        });
    });
    
    group.finish();
}

fn bench_single_alloc_64kb(c: &mut Criterion) {
    let mut group = c.benchmark_group("single_alloc_64KB");
    group.throughput(Throughput::Bytes(65536));
    
    group.bench_function("framealloc", |b| {
        let alloc = SmartAlloc::new(AllocConfig::default());
        alloc.begin_frame();
        b.iter(|| {
            black_box(alloc.frame_alloc::<[u8; 65536]>());
        });
        alloc.end_frame();
    });
    
    group.bench_function("bumpalo", |b| {
        let bump = Bump::with_capacity(128 * 1024 * 1024);
        b.iter(|| {
            black_box(bump.alloc([0u8; 65536]));
        });
    });
    
    group.bench_function("system_malloc", |b| {
        b.iter(|| unsafe {
            let layout = Layout::from_size_align(65536, 8).unwrap();
            let ptr = alloc(layout);
            black_box(ptr);
            dealloc(ptr, layout);
        });
    });
    
    group.finish();
}

// =============================================================================
// BATCH ALLOCATION BENCHMARKS
// =============================================================================

fn bench_batch_100(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_100x64B");
    group.throughput(Throughput::Bytes(100 * 64));
    
    group.bench_function("framealloc", |b| {
        let alloc = SmartAlloc::new(AllocConfig::default());
        b.iter(|| {
            alloc.begin_frame();
            for _ in 0..100 {
                black_box(alloc.frame_alloc::<[u8; 64]>());
            }
            alloc.end_frame();
        });
    });
    
    group.bench_function("bumpalo", |b| {
        b.iter(|| {
            let bump = Bump::with_capacity(100 * 64 + 4096);
            for _ in 0..100 {
                black_box(bump.alloc([0u8; 64]));
            }
            drop(bump);
        });
    });
    
    group.bench_function("system_malloc", |b| {
        b.iter(|| unsafe {
            let layout = Layout::from_size_align(64, 8).unwrap();
            let mut ptrs = Vec::with_capacity(100);
            for _ in 0..100 {
                ptrs.push(alloc(layout));
            }
            for ptr in &ptrs {
                black_box(*ptr);
            }
            for ptr in ptrs {
                dealloc(ptr, layout);
            }
        });
    });
    
    group.finish();
}

fn bench_batch_1000(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_1000x64B");
    group.throughput(Throughput::Bytes(1000 * 64));
    
    group.bench_function("framealloc", |b| {
        let alloc = SmartAlloc::new(AllocConfig::default());
        b.iter(|| {
            alloc.begin_frame();
            for _ in 0..1000 {
                black_box(alloc.frame_alloc::<[u8; 64]>());
            }
            alloc.end_frame();
        });
    });
    
    group.bench_function("bumpalo", |b| {
        b.iter(|| {
            let bump = Bump::with_capacity(1000 * 64 + 4096);
            for _ in 0..1000 {
                black_box(bump.alloc([0u8; 64]));
            }
            drop(bump);
        });
    });
    
    group.bench_function("system_malloc", |b| {
        b.iter(|| unsafe {
            let layout = Layout::from_size_align(64, 8).unwrap();
            let mut ptrs = Vec::with_capacity(1000);
            for _ in 0..1000 {
                ptrs.push(alloc(layout));
            }
            for ptr in &ptrs {
                black_box(*ptr);
            }
            for ptr in ptrs {
                dealloc(ptr, layout);
            }
        });
    });
    
    group.finish();
}

fn bench_batch_10000(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_10000x64B");
    group.throughput(Throughput::Bytes(10000 * 64));
    
    group.bench_function("framealloc", |b| {
        let alloc = SmartAlloc::new(AllocConfig::default());
        b.iter(|| {
            alloc.begin_frame();
            for _ in 0..10000 {
                black_box(alloc.frame_alloc::<[u8; 64]>());
            }
            alloc.end_frame();
        });
    });
    
    group.bench_function("bumpalo", |b| {
        b.iter(|| {
            let bump = Bump::with_capacity(10000 * 64 + 4096);
            for _ in 0..10000 {
                black_box(bump.alloc([0u8; 64]));
            }
            drop(bump);
        });
    });
    
    group.bench_function("system_malloc", |b| {
        b.iter(|| unsafe {
            let layout = Layout::from_size_align(64, 8).unwrap();
            let mut ptrs = Vec::with_capacity(10000);
            for _ in 0..10000 {
                ptrs.push(alloc(layout));
            }
            for ptr in &ptrs {
                black_box(*ptr);
            }
            for ptr in ptrs {
                dealloc(ptr, layout);
            }
        });
    });
    
    group.finish();
}

// =============================================================================
// REALISTIC GAME WORKLOAD PROFILES
// =============================================================================

fn bench_physics_frame(c: &mut Criterion) {
    // Simulates: 500 contacts (96B), 100 bodies (256B), 200 transforms (64B)
    let mut group = c.benchmark_group("workload_physics");
    let total = 500 * 96 + 100 * 256 + 200 * 64;
    group.throughput(Throughput::Bytes(total as u64));
    
    group.bench_function("framealloc", |b| {
        let alloc = SmartAlloc::new(AllocConfig::default());
        b.iter(|| {
            alloc.begin_frame();
            for _ in 0..500 { black_box(alloc.frame_alloc::<[u8; 96]>()); }
            for _ in 0..100 { black_box(alloc.frame_alloc::<[u8; 256]>()); }
            for _ in 0..200 { black_box(alloc.frame_alloc::<[u8; 64]>()); }
            alloc.end_frame();
        });
    });
    
    group.bench_function("bumpalo", |b| {
        b.iter(|| {
            let bump = Bump::with_capacity(total + 4096);
            for _ in 0..500 { black_box(bump.alloc([0u8; 96])); }
            for _ in 0..100 { black_box(bump.alloc([0u8; 256])); }
            for _ in 0..200 { black_box(bump.alloc([0u8; 64])); }
            drop(bump);
        });
    });
    
    group.bench_function("system_malloc", |b| {
        b.iter(|| unsafe {
            let mut ptrs = Vec::with_capacity(800);
            let l96 = Layout::from_size_align(96, 8).unwrap();
            let l256 = Layout::from_size_align(256, 8).unwrap();
            let l64 = Layout::from_size_align(64, 8).unwrap();
            for _ in 0..500 { ptrs.push((alloc(l96), l96)); }
            for _ in 0..100 { ptrs.push((alloc(l256), l256)); }
            for _ in 0..200 { ptrs.push((alloc(l64), l64)); }
            for (ptr, _) in &ptrs { black_box(*ptr); }
            for (ptr, layout) in ptrs { dealloc(ptr, layout); }
        });
    });
    
    group.finish();
}

fn bench_render_frame(c: &mut Criterion) {
    // Simulates: 2000 commands (24B), 5000 vertices (32B), 500 mesh refs (32B)
    let mut group = c.benchmark_group("workload_render");
    let total = 2000 * 24 + 5000 * 32 + 500 * 32;
    group.throughput(Throughput::Bytes(total as u64));
    
    group.bench_function("framealloc", |b| {
        let alloc = SmartAlloc::new(AllocConfig::default());
        b.iter(|| {
            alloc.begin_frame();
            for _ in 0..2000 { black_box(alloc.frame_alloc::<[u8; 24]>()); }
            for _ in 0..5000 { black_box(alloc.frame_alloc::<[u8; 32]>()); }
            for _ in 0..500 { black_box(alloc.frame_alloc::<[u8; 32]>()); }
            alloc.end_frame();
        });
    });
    
    group.bench_function("bumpalo", |b| {
        b.iter(|| {
            let bump = Bump::with_capacity(total + 4096);
            for _ in 0..2000 { black_box(bump.alloc([0u8; 24])); }
            for _ in 0..5000 { black_box(bump.alloc([0u8; 32])); }
            for _ in 0..500 { black_box(bump.alloc([0u8; 32])); }
            drop(bump);
        });
    });
    
    group.bench_function("system_malloc", |b| {
        b.iter(|| unsafe {
            let mut ptrs = Vec::with_capacity(7500);
            let l24 = Layout::from_size_align(24, 8).unwrap();
            let l32 = Layout::from_size_align(32, 8).unwrap();
            for _ in 0..2000 { ptrs.push((alloc(l24), l24)); }
            for _ in 0..5500 { ptrs.push((alloc(l32), l32)); }
            for (ptr, _) in &ptrs { black_box(*ptr); }
            for (ptr, layout) in ptrs { dealloc(ptr, layout); }
        });
    });
    
    group.finish();
}

fn bench_particle_heavy(c: &mut Criterion) {
    // Simulates: 50000 particles (48B)
    let mut group = c.benchmark_group("workload_particles");
    let total = 50000 * 48;
    group.throughput(Throughput::Bytes(total as u64));
    
    group.bench_function("framealloc", |b| {
        let alloc = SmartAlloc::new(AllocConfig::default());
        b.iter(|| {
            alloc.begin_frame();
            for _ in 0..50000 { black_box(alloc.frame_alloc::<[u8; 48]>()); }
            alloc.end_frame();
        });
    });
    
    group.bench_function("bumpalo", |b| {
        b.iter(|| {
            let bump = Bump::with_capacity(total + 4096);
            for _ in 0..50000 { black_box(bump.alloc([0u8; 48])); }
            drop(bump);
        });
    });
    
    group.bench_function("system_malloc", |b| {
        b.iter(|| unsafe {
            let mut ptrs = Vec::with_capacity(50000);
            let l = Layout::from_size_align(48, 8).unwrap();
            for _ in 0..50000 { ptrs.push(alloc(l)); }
            for ptr in &ptrs { black_box(*ptr); }
            for ptr in ptrs { dealloc(ptr, l); }
        });
    });
    
    group.finish();
}

fn bench_stress_tiny(c: &mut Criterion) {
    // Stress test: 100000 tiny allocations (16B)
    let mut group = c.benchmark_group("stress_100k_tiny");
    group.throughput(Throughput::Bytes(100000 * 16));
    
    group.bench_function("framealloc", |b| {
        let alloc = SmartAlloc::new(AllocConfig::default());
        b.iter(|| {
            alloc.begin_frame();
            for _ in 0..100000 { black_box(alloc.frame_alloc::<[u8; 16]>()); }
            alloc.end_frame();
        });
    });
    
    group.bench_function("bumpalo", |b| {
        b.iter(|| {
            let bump = Bump::with_capacity(100000 * 16 + 4096);
            for _ in 0..100000 { black_box(bump.alloc([0u8; 16])); }
            drop(bump);
        });
    });
    
    group.bench_function("system_malloc", |b| {
        b.iter(|| unsafe {
            let mut ptrs = Vec::with_capacity(100000);
            let l = Layout::from_size_align(16, 8).unwrap();
            for _ in 0..100000 { ptrs.push(alloc(l)); }
            for ptr in &ptrs { black_box(*ptr); }
            for ptr in ptrs { dealloc(ptr, l); }
        });
    });
    
    group.finish();
}

// =============================================================================
// FRAME LIFECYCLE OVERHEAD
// =============================================================================

fn bench_frame_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("frame_lifecycle");
    
    group.bench_function("empty_frame", |b| {
        let alloc = SmartAlloc::new(AllocConfig::default());
        b.iter(|| {
            alloc.begin_frame();
            alloc.end_frame();
        });
    });
    
    group.bench_function("light_frame_100", |b| {
        let alloc = SmartAlloc::new(AllocConfig::default());
        b.iter(|| {
            alloc.begin_frame();
            for _ in 0..100 { black_box(alloc.frame_alloc::<[u8; 64]>()); }
            alloc.end_frame();
        });
    });
    
    group.bench_function("heavy_frame_10000", |b| {
        let alloc = SmartAlloc::new(AllocConfig::default());
        b.iter(|| {
            alloc.begin_frame();
            for _ in 0..10000 { black_box(alloc.frame_alloc::<[u8; 64]>()); }
            alloc.end_frame();
        });
    });
    
    // Bumpalo comparison (reset)
    group.bench_function("bumpalo_reset_light", |b| {
        let mut bump = Bump::with_capacity(100 * 64 + 4096);
        b.iter(|| {
            for _ in 0..100 { black_box(bump.alloc([0u8; 64])); }
            bump.reset();
        });
    });
    
    group.bench_function("bumpalo_reset_heavy", |b| {
        let mut bump = Bump::with_capacity(10000 * 64 + 4096);
        b.iter(|| {
            for _ in 0..10000 { black_box(bump.alloc([0u8; 64])); }
            bump.reset();
        });
    });
    
    group.finish();
}

// =============================================================================
// ALLOCATION + WRITE (Cache behavior test)
// =============================================================================

fn bench_alloc_and_write(c: &mut Criterion) {
    let mut group = c.benchmark_group("alloc_write_1000x256B");
    group.throughput(Throughput::Bytes(1000 * 256));
    
    group.bench_function("framealloc", |b| {
        let alloc = SmartAlloc::new(AllocConfig::default());
        b.iter(|| {
            alloc.begin_frame();
            for i in 0..1000 {
                let ptr = alloc.frame_alloc::<[u8; 256]>();
                unsafe { std::ptr::write_bytes(ptr as *mut u8, (i & 0xFF) as u8, 256); }
                black_box(ptr);
            }
            alloc.end_frame();
        });
    });
    
    group.bench_function("bumpalo", |b| {
        b.iter(|| {
            let bump = Bump::with_capacity(1000 * 256 + 4096);
            for i in 0..1000 {
                let slice = bump.alloc_slice_fill_copy(256, (i & 0xFF) as u8);
                black_box(slice);
            }
            drop(bump);
        });
    });
    
    group.bench_function("system_malloc", |b| {
        b.iter(|| unsafe {
            let layout = Layout::from_size_align(256, 8).unwrap();
            let mut ptrs = Vec::with_capacity(1000);
            for i in 0..1000 {
                let ptr = alloc(layout);
                std::ptr::write_bytes(ptr, (i & 0xFF) as u8, 256);
                ptrs.push(ptr);
            }
            for ptr in &ptrs { black_box(*ptr); }
            for ptr in ptrs { dealloc(ptr, layout); }
        });
    });
    
    group.finish();
}

// =============================================================================
// MULTI-THREADED BENCHMARKS
// =============================================================================

fn bench_multithreaded(c: &mut Criterion) {
    let mut group = c.benchmark_group("multithreaded");
    
    for num_threads in [2, 4, 8] {
        let allocs_per_thread = 10000usize;
        
        group.bench_with_input(
            BenchmarkId::new("framealloc", num_threads),
            &num_threads,
            |b, &num_threads| {
                let alloc = std::sync::Arc::new(SmartAlloc::new(AllocConfig::default()));
                b.iter(|| {
                    let handles: Vec<_> = (0..num_threads)
                        .map(|_| {
                            let alloc = alloc.clone();
                            std::thread::spawn(move || {
                                alloc.begin_frame();
                                for _ in 0..allocs_per_thread {
                                    black_box(alloc.frame_alloc::<[u8; 64]>());
                                }
                                alloc.end_frame();
                            })
                        })
                        .collect();
                    for h in handles { h.join().unwrap(); }
                });
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("system_malloc", num_threads),
            &num_threads,
            |b, &num_threads| {
                b.iter(|| {
                    let handles: Vec<_> = (0..num_threads)
                        .map(|_| {
                            std::thread::spawn(move || unsafe {
                                let layout = Layout::from_size_align(64, 8).unwrap();
                                let mut ptrs = Vec::with_capacity(allocs_per_thread);
                                for _ in 0..allocs_per_thread { ptrs.push(alloc(layout)); }
                                for ptr in &ptrs { black_box(*ptr); }
                                for ptr in ptrs { dealloc(ptr, layout); }
                            })
                        })
                        .collect();
                    for h in handles { h.join().unwrap(); }
                });
            },
        );
    }
    
    group.finish();
}

// =============================================================================
// MIXED ALIGNMENT STRESS
// =============================================================================

fn bench_alignment_stress(c: &mut Criterion) {
    let mut group = c.benchmark_group("alignment_stress");
    
    // Alternating small/large alignments cause padding waste
    group.bench_function("framealloc_mixed", |b| {
        let alloc = SmartAlloc::new(AllocConfig::default());
        b.iter(|| {
            alloc.begin_frame();
            for _ in 0..1000 {
                black_box(alloc.frame_alloc::<[u8; 7]>());
                black_box(alloc.frame_alloc::<[u8; 16]>());
                black_box(alloc.frame_alloc::<[u8; 3]>());
                black_box(alloc.frame_alloc::<[u8; 64]>());
            }
            alloc.end_frame();
        });
    });
    
    group.bench_function("bumpalo_mixed", |b| {
        b.iter(|| {
            let bump = Bump::with_capacity(1024 * 1024);
            for _ in 0..1000 {
                black_box(bump.alloc([0u8; 7]));
                black_box(bump.alloc([0u8; 16]));
                black_box(bump.alloc([0u8; 3]));
                black_box(bump.alloc([0u8; 64]));
            }
            drop(bump);
        });
    });
    
    group.finish();
}

// =============================================================================
// CRITERION CONFIGURATION
// =============================================================================

criterion_group!(
    single_alloc,
    bench_single_alloc_64,
    bench_single_alloc_256,
    bench_single_alloc_1kb,
    bench_single_alloc_4kb,
    bench_single_alloc_64kb,
);

criterion_group!(
    batch_alloc,
    bench_batch_100,
    bench_batch_1000,
    bench_batch_10000,
);

criterion_group!(
    workloads,
    bench_physics_frame,
    bench_render_frame,
    bench_particle_heavy,
    bench_stress_tiny,
);

criterion_group!(
    overhead,
    bench_frame_overhead,
    bench_alloc_and_write,
);

criterion_group!(
    advanced,
    bench_multithreaded,
    bench_alignment_stress,
);

criterion_main!(single_alloc, batch_alloc, workloads, overhead, advanced);
