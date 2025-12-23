//! Integration tests for framealloc.

use framealloc::{AllocConfig, SmartAlloc};
use std::sync::Arc;
use std::thread;

#[test]
fn test_basic_frame_allocation() {
    let alloc = SmartAlloc::new(AllocConfig::default());

    alloc.begin_frame();

    let ptr1 = alloc.frame_alloc::<u64>();
    assert!(!ptr1.is_null());

    let ptr2 = alloc.frame_alloc::<[f32; 16]>();
    assert!(!ptr2.is_null());

    // Write to verify memory is usable
    unsafe {
        *ptr1 = 42;
        (*ptr2)[0] = 3.14;
    }

    alloc.end_frame();
}

#[test]
fn test_frame_reset_reuses_memory() {
    let alloc = SmartAlloc::new(AllocConfig::default());

    alloc.begin_frame();
    let ptr1 = alloc.frame_alloc::<u64>();
    alloc.end_frame();

    alloc.begin_frame();
    let ptr2 = alloc.frame_alloc::<u64>();
    alloc.end_frame();

    // After reset, should get the same memory back
    assert_eq!(ptr1, ptr2);
}

#[test]
fn test_pool_alloc_and_free() {
    let alloc = SmartAlloc::new(AllocConfig::default());

    let ptr = alloc.pool_alloc::<u64>();
    assert!(!ptr.is_null());

    unsafe {
        *ptr = 12345;
        assert_eq!(*ptr, 12345);
        alloc.pool_free(ptr);
    }

    // Allocating again should return the same pointer (from free list)
    let ptr2 = alloc.pool_alloc::<u64>();
    assert_eq!(ptr, ptr2);
}

#[test]
fn test_heap_alloc_and_free() {
    let alloc = SmartAlloc::new(AllocConfig::default());

    let ptr = alloc.heap_alloc::<[u8; 8192]>();
    assert!(!ptr.is_null());

    unsafe {
        (*ptr)[0] = 0xAB;
        (*ptr)[8191] = 0xCD;
        alloc.heap_free(ptr);
    }
}

#[test]
fn test_frame_scope_guard() {
    let alloc = SmartAlloc::new(AllocConfig::default());

    alloc.begin_frame();

    let outer_ptr = alloc.frame_alloc::<u32>();

    {
        let scope = alloc.frame_scope();
        let inner_ptr = scope.alloc::<u32>();
        assert!(!inner_ptr.is_null());
        // inner_ptr is valid here
    }
    // inner_ptr's memory is now available for reuse

    let after_scope_ptr = alloc.frame_alloc::<u32>();
    // This allocation might reuse the scoped memory
    assert!(!after_scope_ptr.is_null());
    assert_ne!(outer_ptr, after_scope_ptr);

    alloc.end_frame();
}

#[test]
fn test_clone_allocator() {
    let alloc1 = SmartAlloc::new(AllocConfig::default());
    let alloc2 = alloc1.clone();

    // Both should work independently but share global state
    alloc1.begin_frame();
    alloc2.begin_frame();

    let ptr1 = alloc1.frame_alloc::<u64>();
    let ptr2 = alloc2.frame_alloc::<u64>();

    assert!(!ptr1.is_null());
    assert!(!ptr2.is_null());

    alloc1.end_frame();
    alloc2.end_frame();
}

#[test]
fn test_stats() {
    let alloc = SmartAlloc::new(AllocConfig::default());

    let stats_before = alloc.stats();

    // Do some allocations
    let ptr = alloc.heap_alloc::<[u8; 1024]>();
    assert!(!ptr.is_null());

    let stats_after = alloc.stats();

    // Stats should reflect the allocation
    assert!(stats_after.allocation_count >= stats_before.allocation_count);

    unsafe {
        alloc.heap_free(ptr);
    }
}

// ============ MULTI-THREADED TESTS ============

#[test]
fn test_multithread_independent_frame_arenas() {
    let alloc = Arc::new(SmartAlloc::new(AllocConfig::default()));
    let num_threads = 4;
    let allocations_per_thread = 100;

    let handles: Vec<_> = (0..num_threads)
        .map(|thread_id| {
            let alloc = alloc.clone();
            thread::spawn(move || {
                for iteration in 0..10 {
                    alloc.begin_frame();

                    let mut ptrs = Vec::new();
                    for i in 0..allocations_per_thread {
                        let ptr = alloc.frame_alloc::<u64>();
                        assert!(!ptr.is_null(), "Thread {} iter {} alloc {} failed", thread_id, iteration, i);
                        unsafe {
                            *ptr = (thread_id * 1000 + i) as u64;
                        }
                        ptrs.push(ptr);
                    }

                    // Verify all values are correct
                    for (i, &ptr) in ptrs.iter().enumerate() {
                        unsafe {
                            let expected = (thread_id * 1000 + i) as u64;
                            assert_eq!(*ptr, expected, "Thread {} value mismatch at {}", thread_id, i);
                        }
                    }

                    alloc.end_frame();
                }
            })
        })
        .collect();

    for handle in handles {
        handle.join().expect("Thread panicked");
    }
}

#[test]
fn test_multithread_pool_allocation() {
    let alloc = Arc::new(SmartAlloc::new(AllocConfig::default()));
    let num_threads = 4;

    let handles: Vec<_> = (0..num_threads)
        .map(|thread_id| {
            let alloc = alloc.clone();
            thread::spawn(move || {
                let mut ptrs = Vec::new();

                // Allocate
                for i in 0..50 {
                    let ptr = alloc.pool_alloc::<u64>();
                    assert!(!ptr.is_null(), "Thread {} alloc {} failed", thread_id, i);
                    unsafe {
                        *ptr = (thread_id * 100 + i) as u64;
                    }
                    ptrs.push(ptr);
                }

                // Free half
                for ptr in ptrs.drain(..25) {
                    unsafe {
                        alloc.pool_free(ptr);
                    }
                }

                // Allocate more (should reuse freed)
                for i in 0..25 {
                    let ptr = alloc.pool_alloc::<u64>();
                    assert!(!ptr.is_null());
                    unsafe {
                        *ptr = (thread_id * 1000 + i) as u64;
                    }
                    ptrs.push(ptr);
                }

                // Cleanup
                for ptr in ptrs {
                    unsafe {
                        alloc.pool_free(ptr);
                    }
                }
            })
        })
        .collect();

    for handle in handles {
        handle.join().expect("Thread panicked");
    }
}

#[test]
fn test_multithread_mixed_allocation() {
    let alloc = Arc::new(SmartAlloc::new(AllocConfig::default()));
    let num_threads = 4;

    let handles: Vec<_> = (0..num_threads)
        .map(|_thread_id| {
            let alloc = alloc.clone();
            thread::spawn(move || {
                for _ in 0..5 {
                    alloc.begin_frame();

                    // Mix of frame and pool allocations
                    let frame_ptrs: Vec<_> = (0..20)
                        .map(|_| alloc.frame_alloc::<[u8; 64]>())
                        .collect();

                    let pool_ptrs: Vec<_> = (0..10)
                        .map(|_| alloc.pool_alloc::<[u8; 32]>())
                        .collect();

                    // Verify frame allocations
                    for ptr in &frame_ptrs {
                        assert!(!ptr.is_null());
                    }

                    // Free pool allocations
                    for ptr in pool_ptrs {
                        assert!(!ptr.is_null());
                        unsafe {
                            alloc.pool_free(ptr);
                        }
                    }

                    alloc.end_frame();
                }
            })
        })
        .collect();

    for handle in handles {
        handle.join().expect("Thread panicked");
    }
}

#[test]
fn test_stress_rapid_frame_cycling() {
    let alloc = SmartAlloc::new(AllocConfig::default());

    for _ in 0..1000 {
        alloc.begin_frame();

        for _ in 0..10 {
            let ptr = alloc.frame_alloc::<u64>();
            assert!(!ptr.is_null());
        }

        alloc.end_frame();
    }
}

#[test]
fn test_config_minimal() {
    let alloc = SmartAlloc::new(AllocConfig::minimal());

    alloc.begin_frame();
    let ptr = alloc.frame_alloc::<u64>();
    assert!(!ptr.is_null());
    alloc.end_frame();
}

#[test]
fn test_config_high_performance() {
    let alloc = SmartAlloc::new(AllocConfig::high_performance());

    alloc.begin_frame();

    // High-performance config has larger arena, should handle more
    for _ in 0..10000 {
        let ptr = alloc.frame_alloc::<[u8; 128]>();
        assert!(!ptr.is_null());
    }

    alloc.end_frame();
}
