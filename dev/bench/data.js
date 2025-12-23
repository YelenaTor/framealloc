window.BENCHMARK_DATA = {
  "lastUpdate": 1766467112601,
  "repoUrl": "https://github.com/YelenaTor/framealloc",
  "entries": {
    "framealloc Benchmarks": [
      {
        "commit": {
          "author": {
            "email": "dngentle@gmail.com",
            "name": "YoruSprites",
            "username": "YoruSprites"
          },
          "committer": {
            "email": "dngentle@gmail.com",
            "name": "YoruSprites",
            "username": "YoruSprites"
          },
          "distinct": true,
          "id": "ec23265d85d82e4b7dd5529a7c62cb2afb8a8aac",
          "message": "v0.10.0: Rapier Physics Integration (v0.31)\n\n- Add Rapier physics engine integration with framealloc support\n- Update to Rapier v0.31 API (BroadPhaseBvh, QueryFilter changes)\n- Implement frame-allocated contact/proximity events\n- Add ray casting with frame-allocated results\n- Remove Kira and RealTimeAlloc traces completely\n- Fix all compilation errors for Rapier integration\n- Add comprehensive tests for 2D and 3D physics\n- Update documentation for Rapier integration",
          "timestamp": "2025-12-23T07:05:15+02:00",
          "tree_id": "d6df695601489ea8c7c7ae1e440a21fa542e2843",
          "url": "https://github.com/YelenaTor/framealloc/commit/ec23265d85d82e4b7dd5529a7c62cb2afb8a8aac"
        },
        "date": 1766467112206,
        "tool": "cargo",
        "benches": [
          {
            "name": "single_alloc_64B/framealloc",
            "value": 3,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "single_alloc_64B/bumpalo",
            "value": 14,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "single_alloc_64B/system_malloc",
            "value": 10,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "single_alloc_64B/box_vec",
            "value": 36,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "single_alloc_256B/framealloc",
            "value": 3,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "single_alloc_256B/bumpalo",
            "value": 55,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "single_alloc_256B/system_malloc",
            "value": 10,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "single_alloc_1KB/framealloc",
            "value": 3,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "single_alloc_1KB/bumpalo",
            "value": 221,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "single_alloc_1KB/system_malloc",
            "value": 10,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "single_alloc_4KB/framealloc",
            "value": 3,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "single_alloc_4KB/bumpalo",
            "value": 867,
            "range": "± 65",
            "unit": "ns/iter"
          },
          {
            "name": "single_alloc_4KB/system_malloc",
            "value": 50,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "single_alloc_64KB/framealloc",
            "value": 4,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "single_alloc_64KB/bumpalo",
            "value": 13557,
            "range": "± 1084",
            "unit": "ns/iter"
          },
          {
            "name": "single_alloc_64KB/system_malloc",
            "value": 51,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "batch_100x64B/framealloc",
            "value": 226,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "batch_100x64B/bumpalo",
            "value": 356,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "batch_100x64B/system_malloc",
            "value": 2770,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "batch_1000x64B/framealloc",
            "value": 1877,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "batch_1000x64B/bumpalo",
            "value": 2749,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "batch_1000x64B/system_malloc",
            "value": 38770,
            "range": "± 1920",
            "unit": "ns/iter"
          },
          {
            "name": "batch_10000x64B/framealloc",
            "value": 20274,
            "range": "± 2495",
            "unit": "ns/iter"
          },
          {
            "name": "batch_10000x64B/bumpalo",
            "value": 29148,
            "range": "± 60",
            "unit": "ns/iter"
          },
          {
            "name": "batch_10000x64B/system_malloc",
            "value": 387893,
            "range": "± 729",
            "unit": "ns/iter"
          },
          {
            "name": "workload_physics/framealloc",
            "value": 1540,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "workload_physics/bumpalo",
            "value": 2902,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "workload_physics/system_malloc",
            "value": 31165,
            "range": "± 130",
            "unit": "ns/iter"
          },
          {
            "name": "workload_render/framealloc",
            "value": 13836,
            "range": "± 102",
            "unit": "ns/iter"
          },
          {
            "name": "workload_render/bumpalo",
            "value": 18462,
            "range": "± 36",
            "unit": "ns/iter"
          },
          {
            "name": "workload_render/system_malloc",
            "value": 290690,
            "range": "± 3630",
            "unit": "ns/iter"
          },
          {
            "name": "workload_particles/framealloc",
            "value": 100831,
            "range": "± 773",
            "unit": "ns/iter"
          },
          {
            "name": "workload_particles/bumpalo",
            "value": 138302,
            "range": "± 446",
            "unit": "ns/iter"
          },
          {
            "name": "workload_particles/system_malloc",
            "value": 1848025,
            "range": "± 19000",
            "unit": "ns/iter"
          },
          {
            "name": "stress_100k_tiny/framealloc",
            "value": 206063,
            "range": "± 2156",
            "unit": "ns/iter"
          },
          {
            "name": "stress_100k_tiny/bumpalo",
            "value": 241571,
            "range": "± 174",
            "unit": "ns/iter"
          },
          {
            "name": "stress_100k_tiny/system_malloc",
            "value": 3610488,
            "range": "± 20459",
            "unit": "ns/iter"
          },
          {
            "name": "frame_lifecycle/empty_frame",
            "value": 21,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "frame_lifecycle/light_frame_100",
            "value": 228,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "frame_lifecycle/heavy_frame_10000",
            "value": 20214,
            "range": "± 63",
            "unit": "ns/iter"
          },
          {
            "name": "frame_lifecycle/bumpalo_reset_light",
            "value": 302,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "frame_lifecycle/bumpalo_reset_heavy",
            "value": 25819,
            "range": "± 38",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_write_1000x256B/framealloc",
            "value": 8441,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_write_1000x256B/bumpalo",
            "value": 6997,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_write_1000x256B/system_malloc",
            "value": 44996,
            "range": "± 956",
            "unit": "ns/iter"
          },
          {
            "name": "multithreaded/framealloc/2",
            "value": 98710,
            "range": "± 3609",
            "unit": "ns/iter"
          },
          {
            "name": "multithreaded/system_malloc/2",
            "value": 476211,
            "range": "± 9792",
            "unit": "ns/iter"
          },
          {
            "name": "multithreaded/framealloc/4",
            "value": 166085,
            "range": "± 1555",
            "unit": "ns/iter"
          },
          {
            "name": "multithreaded/system_malloc/4",
            "value": 824990,
            "range": "± 4000",
            "unit": "ns/iter"
          },
          {
            "name": "multithreaded/framealloc/8",
            "value": 343686,
            "range": "± 4273",
            "unit": "ns/iter"
          },
          {
            "name": "multithreaded/system_malloc/8",
            "value": 1692932,
            "range": "± 40386",
            "unit": "ns/iter"
          },
          {
            "name": "alignment_stress/framealloc_mixed",
            "value": 8282,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "alignment_stress/bumpalo_mixed",
            "value": 9768,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_write_1000x256B/framealloc_default",
            "value": 984934,
            "range": "± 858",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_write_1000x256B/framealloc_minimal",
            "value": 985136,
            "range": "± 646",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_write_1000x256B/framealloc_prefetch",
            "value": 985133,
            "range": "± 712",
            "unit": "ns/iter"
          },
          {
            "name": "batch_vs_individual_1000x64B/individual_allocs",
            "value": 2327,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "batch_vs_individual_1000x64B/batch_alloc",
            "value": 24,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "batch_vs_individual_1000x64B/small_batch_8x125",
            "value": 265,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "frame_boundary/empty_frame_cycle",
            "value": 21,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "frame_boundary/light_frame_10_allocs",
            "value": 34,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "fallible_vs_regular/regular_alloc",
            "value": 22,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "fallible_vs_regular/fallible_alloc",
            "value": 23,
            "range": "± 0",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}