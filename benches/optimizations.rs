//! Comprehensive benchmark for framealloc optimizations

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};

fn bench_alloc_write(c: &mut Criterion) {
    let mut group = c.benchmark_group("alloc_write_1000x256B");
    
    group.throughput(Throughput::Elements(1000));
    
    // Test prefetch optimization
    group.bench_function("framealloc_default", |b| {
        let alloc = framealloc::SmartAlloc::new(framealloc::AllocConfig::default());
        b.iter(|| {
            alloc.begin_frame();
            for _ in 0..1000 {
                let ptr = alloc.frame_alloc::<[u8; 256]>();
                unsafe {
                    // Write to allocated memory
                    std::ptr::write_bytes(ptr, 0xAA, 256);
                }
            }
            alloc.end_frame();
        });
    });
    
    group.bench_function("framealloc_minimal", |b| {
        let alloc = framealloc::SmartAlloc::new(framealloc::AllocConfig::default());
        b.iter(|| {
            alloc.begin_frame();
            for _ in 0..1000 {
                let ptr = alloc.frame_alloc::<[u8; 256]>();
                unsafe {
                    std::ptr::write_bytes(ptr, 0xAA, 256);
                }
            }
            alloc.end_frame();
        });
    });
    
    #[cfg(feature = "prefetch")]
    group.bench_function("framealloc_prefetch", |b| {
        let alloc = framealloc::SmartAlloc::new(framealloc::AllocConfig::default());
        b.iter(|| {
            alloc.begin_frame();
            for _ in 0..1000 {
                let ptr = alloc.frame_alloc::<[u8; 256]>();
                unsafe {
                    std::ptr::write_bytes(ptr, 0xAA, 256);
                }
            }
            alloc.end_frame();
        });
    });
    
    group.finish();
}

fn bench_batch_vs_individual(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_vs_individual_1000x64B");
    group.throughput(Throughput::Elements(1000));
    
    // Individual allocations
    group.bench_function("individual_allocs", |b| {
        let alloc = framealloc::SmartAlloc::new(framealloc::AllocConfig::default());
        b.iter(|| {
            alloc.begin_frame();
            for _ in 0..1000 {
                black_box(alloc.frame_alloc::<[u8; 64]>());
            }
            alloc.end_frame();
        });
    });
    
    // Batch allocation
    group.bench_function("batch_alloc", |b| {
        let alloc = framealloc::SmartAlloc::new(framealloc::AllocConfig::default());
        b.iter(|| {
            alloc.begin_frame();
            black_box(alloc.frame_alloc_batch::<[u8; 64]>(1000));
            alloc.end_frame();
        });
    });
    
    // Small-batch specialization
    group.bench_function("small_batch_8x125", |b| {
        let alloc = framealloc::SmartAlloc::new(framealloc::AllocConfig::default());
        b.iter(|| {
            alloc.begin_frame();
            for _ in 0..125 {
                black_box(alloc.frame_alloc_8::<[u8; 64]>());
            }
            alloc.end_frame();
        });
    });
    
    group.finish();
}

fn bench_frame_boundary(c: &mut Criterion) {
    let mut group = c.benchmark_group("frame_boundary");
    
    group.bench_function("empty_frame_cycle", |b| {
        let alloc = framealloc::SmartAlloc::new(framealloc::AllocConfig::default());
        b.iter(|| {
            alloc.begin_frame();
            alloc.end_frame();
        });
    });
    
    group.bench_function("light_frame_10_allocs", |b| {
        let alloc = framealloc::SmartAlloc::new(framealloc::AllocConfig::default());
        b.iter(|| {
            alloc.begin_frame();
            for _ in 0..10 {
                black_box(alloc.frame_alloc::<[u8; 64]>());
            }
            alloc.end_frame();
        });
    });
    
    group.finish();
}

fn bench_fallible_alloc(c: &mut Criterion) {
    let mut group = c.benchmark_group("fallible_vs_regular");
    
    group.bench_function("regular_alloc", |b| {
        let alloc = framealloc::SmartAlloc::new(framealloc::AllocConfig::default());
        b.iter(|| {
            alloc.begin_frame();
            black_box(alloc.frame_alloc::<[u8; 64]>());
            alloc.end_frame();
        });
    });
    
    group.bench_function("fallible_alloc", |b| {
        let alloc = framealloc::SmartAlloc::new(framealloc::AllocConfig::default());
        b.iter(|| {
            alloc.begin_frame();
            black_box(alloc.try_frame_alloc::<[u8; 64]>());
            alloc.end_frame();
        });
    });
    
    group.finish();
}

criterion_group!(
    benches,
    bench_alloc_write,
    bench_batch_vs_individual,
    bench_frame_boundary,
    bench_fallible_alloc
);
criterion_main!(benches);
