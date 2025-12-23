//! Benchmark to test minimal mode performance improvements

use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_batch_minimal_mode(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_1000x64B");
    
    group.bench_function("framealloc_with_stats", |b| {
        let alloc = framealloc::SmartAlloc::new(framealloc::AllocConfig::default());
        b.iter(|| {
            alloc.begin_frame();
            for _ in 0..1000 {
                black_box(alloc.frame_alloc::<[u8; 64]>());
            }
            alloc.end_frame();
        });
    });
    
    #[cfg(feature = "minimal")]
    group.bench_function("framealloc_minimal", |b| {
        let alloc = framealloc::SmartAlloc::new(framealloc::AllocConfig::default());
        b.iter(|| {
            alloc.begin_frame();
            for _ in 0..1000 {
                black_box(alloc.frame_alloc::<[u8; 64]>());
            }
            alloc.end_frame();
        });
    });
    
    group.finish();
}

criterion_group!(benches, bench_batch_minimal_mode);
criterion_main!(benches);
