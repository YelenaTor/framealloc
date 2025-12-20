//! Benchmarks for framealloc.
//!
//! Run with: cargo bench

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use framealloc::{SmartAlloc, AllocConfig, StreamPriority};

fn bench_frame_alloc(c: &mut Criterion) {
    let alloc = SmartAlloc::new(AllocConfig::default());

    let mut group = c.benchmark_group("frame_allocation");

    group.bench_function("frame_alloc_u64_1000x", |b| {
        b.iter(|| {
            alloc.begin_frame();
            for _ in 0..1000 {
                let ptr = alloc.frame_alloc::<u64>();
                black_box(ptr);
            }
            alloc.end_frame();
        })
    });

    group.bench_function("frame_alloc_1kb_100x", |b| {
        b.iter(|| {
            alloc.begin_frame();
            for _ in 0..100 {
                let ptr = alloc.frame_alloc::<[u8; 1024]>();
                black_box(ptr);
            }
            alloc.end_frame();
        })
    });

    // Safe wrapper benchmark
    group.bench_function("frame_box_u64_1000x", |b| {
        b.iter(|| {
            alloc.begin_frame();
            for i in 0..1000u64 {
                let boxed = alloc.frame_box(i);
                black_box(boxed);
            }
            alloc.end_frame();
        })
    });

    group.finish();
}

fn bench_pool_alloc(c: &mut Criterion) {
    let alloc = SmartAlloc::new(AllocConfig::default());

    let mut group = c.benchmark_group("pool_allocation");

    group.bench_function("pool_alloc_free_u64", |b| {
        b.iter(|| {
            let ptr = alloc.pool_alloc::<u64>();
            black_box(ptr);
            unsafe { alloc.pool_free(ptr); }
        })
    });

    // Safe wrapper benchmark
    group.bench_function("pool_box_u64", |b| {
        b.iter(|| {
            let boxed = alloc.pool_box(42u64);
            black_box(boxed);
            // Automatically freed on drop
        })
    });

    // Batch allocation
    group.bench_function("pool_alloc_100x_then_free", |b| {
        b.iter(|| {
            let mut ptrs = Vec::with_capacity(100);
            for _ in 0..100 {
                ptrs.push(alloc.pool_alloc::<u64>());
            }
            for ptr in ptrs {
                unsafe { alloc.pool_free(ptr); }
            }
        })
    });

    group.finish();
}

fn bench_heap_alloc(c: &mut Criterion) {
    let alloc = SmartAlloc::new(AllocConfig::default());

    let mut group = c.benchmark_group("heap_allocation");

    group.bench_function("heap_alloc_free_8kb", |b| {
        b.iter(|| {
            let ptr = alloc.heap_alloc::<[u8; 8192]>();
            black_box(ptr);
            unsafe { alloc.heap_free(ptr); }
        })
    });

    // Safe wrapper benchmark
    group.bench_function("heap_box_8kb", |b| {
        b.iter(|| {
            let boxed = alloc.heap_box([0u8; 8192]);
            black_box(boxed);
        })
    });

    group.finish();
}

fn bench_handle_alloc(c: &mut Criterion) {
    let alloc = SmartAlloc::new(AllocConfig::default());
    let handles = alloc.handles();

    let mut group = c.benchmark_group("handle_allocation");

    group.bench_function("handle_alloc_resolve_free", |b| {
        b.iter(|| {
            let handle = handles.alloc::<u64>().unwrap();
            let ptr = handles.resolve_mut(handle);
            black_box(ptr);
            handles.free(handle);
        })
    });

    group.bench_function("handle_alloc_100x", |b| {
        b.iter(|| {
            let mut hs = Vec::with_capacity(100);
            for _ in 0..100 {
                hs.push(handles.alloc::<u64>().unwrap());
            }
            for h in hs {
                handles.free(h);
            }
        })
    });

    group.finish();
}

fn bench_streaming(c: &mut Criterion) {
    let alloc = SmartAlloc::new(AllocConfig::default());
    let streaming = alloc.streaming();

    let mut group = c.benchmark_group("streaming_allocation");

    group.bench_function("streaming_reserve_free_1mb", |b| {
        b.iter(|| {
            let id = streaming.reserve(1024 * 1024, StreamPriority::Normal).unwrap();
            black_box(id);
            streaming.free(id);
        })
    });

    group.finish();
}

fn bench_groups(c: &mut Criterion) {
    let alloc = SmartAlloc::new(AllocConfig::default());
    let groups = alloc.groups();

    let mut group = c.benchmark_group("group_allocation");

    group.bench_function("group_alloc_100x_free_all", |b| {
        b.iter(|| {
            let gid = groups.create_group("bench");
            for _ in 0..100 {
                groups.alloc::<u64>(gid);
            }
            groups.free_group(gid);
        })
    });

    group.finish();
}

fn bench_comparison(c: &mut Criterion) {
    let alloc = SmartAlloc::new(AllocConfig::default());

    let mut group = c.benchmark_group("comparison_vs_std");
    group.throughput(Throughput::Elements(1000));

    // framealloc frame allocation
    group.bench_function("framealloc_frame_1000x", |b| {
        b.iter(|| {
            alloc.begin_frame();
            for _ in 0..1000 {
                black_box(alloc.frame_alloc::<u64>());
            }
            alloc.end_frame();
        })
    });

    // std::alloc Box
    group.bench_function("std_box_1000x", |b| {
        b.iter(|| {
            let mut boxes: Vec<Box<u64>> = Vec::with_capacity(1000);
            for i in 0..1000u64 {
                boxes.push(Box::new(i));
            }
            black_box(boxes);
        })
    });

    // Vec allocation
    group.bench_function("std_vec_push_1000x", |b| {
        b.iter(|| {
            let mut vec = Vec::with_capacity(1000);
            for i in 0..1000u64 {
                vec.push(i);
            }
            black_box(vec);
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_frame_alloc,
    bench_pool_alloc,
    bench_heap_alloc,
    bench_handle_alloc,
    bench_streaming,
    bench_groups,
    bench_comparison
);
criterion_main!(benches);
