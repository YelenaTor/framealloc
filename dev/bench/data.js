window.BENCHMARK_DATA = {
  "lastUpdate": 1766468722127,
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
      },
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
          "id": "1033f8c6bc78fa6365487a9215daa13842d6498c",
          "message": "Add Rapier 0.31 integration lints to cargo-fa (v0.10.0)\n\n- Add FA901-FA905 diagnostic codes for Rapier integration issues\n- Detect QueryFilter import from wrong module (FA901)\n- Detect deprecated BroadPhase usage (FA902)\n- Recommend step_with_events over step (FA903)\n- Detect ray casting without physics step (FA904)\n- Detect deprecated frame_alloc_slice usage (FA905)\n- Update cargo-fa version to 0.10.0\n- Add FA9xx documentation to README",
          "timestamp": "2025-12-23T07:23:55+02:00",
          "tree_id": "a21926ed283ead24f765314cd6a0aa87bdb50918",
          "url": "https://github.com/YelenaTor/framealloc/commit/1033f8c6bc78fa6365487a9215daa13842d6498c"
        },
        "date": 1766468274628,
        "tool": "cargo",
        "benches": [
          {
            "name": "single_alloc_64B/framealloc",
            "value": 4,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "single_alloc_64B/bumpalo",
            "value": 6,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "single_alloc_64B/system_malloc",
            "value": 13,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "single_alloc_64B/box_vec",
            "value": 23,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "single_alloc_256B/framealloc",
            "value": 4,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "single_alloc_256B/bumpalo",
            "value": 25,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "single_alloc_256B/system_malloc",
            "value": 13,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "single_alloc_1KB/framealloc",
            "value": 4,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "single_alloc_1KB/bumpalo",
            "value": 101,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "single_alloc_1KB/system_malloc",
            "value": 13,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "single_alloc_4KB/framealloc",
            "value": 4,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "single_alloc_4KB/bumpalo",
            "value": 418,
            "range": "± 26",
            "unit": "ns/iter"
          },
          {
            "name": "single_alloc_4KB/system_malloc",
            "value": 37,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "single_alloc_64KB/framealloc",
            "value": 5,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "single_alloc_64KB/bumpalo",
            "value": 5835,
            "range": "± 306",
            "unit": "ns/iter"
          },
          {
            "name": "single_alloc_64KB/system_malloc",
            "value": 40,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "batch_100x64B/framealloc",
            "value": 238,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "batch_100x64B/bumpalo",
            "value": 220,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "batch_100x64B/system_malloc",
            "value": 1891,
            "range": "± 78",
            "unit": "ns/iter"
          },
          {
            "name": "batch_1000x64B/framealloc",
            "value": 1792,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "batch_1000x64B/bumpalo",
            "value": 1820,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "batch_1000x64B/system_malloc",
            "value": 34813,
            "range": "± 200",
            "unit": "ns/iter"
          },
          {
            "name": "batch_10000x64B/framealloc",
            "value": 21777,
            "range": "± 415",
            "unit": "ns/iter"
          },
          {
            "name": "batch_10000x64B/bumpalo",
            "value": 18645,
            "range": "± 106",
            "unit": "ns/iter"
          },
          {
            "name": "batch_10000x64B/system_malloc",
            "value": 337916,
            "range": "± 4654",
            "unit": "ns/iter"
          },
          {
            "name": "workload_physics/framealloc",
            "value": 1484,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "workload_physics/bumpalo",
            "value": 3950,
            "range": "± 307",
            "unit": "ns/iter"
          },
          {
            "name": "workload_physics/system_malloc",
            "value": 27602,
            "range": "± 150",
            "unit": "ns/iter"
          },
          {
            "name": "workload_render/framealloc",
            "value": 12803,
            "range": "± 71",
            "unit": "ns/iter"
          },
          {
            "name": "workload_render/bumpalo",
            "value": 11662,
            "range": "± 1495",
            "unit": "ns/iter"
          },
          {
            "name": "workload_render/system_malloc",
            "value": 261312,
            "range": "± 2883",
            "unit": "ns/iter"
          },
          {
            "name": "workload_particles/framealloc",
            "value": 108833,
            "range": "± 1157",
            "unit": "ns/iter"
          },
          {
            "name": "workload_particles/bumpalo",
            "value": 81412,
            "range": "± 312",
            "unit": "ns/iter"
          },
          {
            "name": "workload_particles/system_malloc",
            "value": 1498601,
            "range": "± 31728",
            "unit": "ns/iter"
          },
          {
            "name": "stress_100k_tiny/framealloc",
            "value": 172995,
            "range": "± 2025",
            "unit": "ns/iter"
          },
          {
            "name": "stress_100k_tiny/bumpalo",
            "value": 104299,
            "range": "± 478",
            "unit": "ns/iter"
          },
          {
            "name": "stress_100k_tiny/system_malloc",
            "value": 2764099,
            "range": "± 10725",
            "unit": "ns/iter"
          },
          {
            "name": "frame_lifecycle/empty_frame",
            "value": 15,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "frame_lifecycle/light_frame_100",
            "value": 238,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "frame_lifecycle/heavy_frame_10000",
            "value": 21777,
            "range": "± 80",
            "unit": "ns/iter"
          },
          {
            "name": "frame_lifecycle/bumpalo_reset_light",
            "value": 155,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "frame_lifecycle/bumpalo_reset_heavy",
            "value": 18387,
            "range": "± 92",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_write_1000x256B/framealloc",
            "value": 5857,
            "range": "± 24",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_write_1000x256B/bumpalo",
            "value": 5469,
            "range": "± 37",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_write_1000x256B/system_malloc",
            "value": 34031,
            "range": "± 94",
            "unit": "ns/iter"
          },
          {
            "name": "multithreaded/framealloc/2",
            "value": 131195,
            "range": "± 2036",
            "unit": "ns/iter"
          },
          {
            "name": "multithreaded/system_malloc/2",
            "value": 371735,
            "range": "± 4860",
            "unit": "ns/iter"
          },
          {
            "name": "multithreaded/framealloc/4",
            "value": 203511,
            "range": "± 2581",
            "unit": "ns/iter"
          },
          {
            "name": "multithreaded/system_malloc/4",
            "value": 664854,
            "range": "± 4434",
            "unit": "ns/iter"
          },
          {
            "name": "multithreaded/framealloc/8",
            "value": 415803,
            "range": "± 9221",
            "unit": "ns/iter"
          },
          {
            "name": "multithreaded/system_malloc/8",
            "value": 1368800,
            "range": "± 79332",
            "unit": "ns/iter"
          },
          {
            "name": "alignment_stress/framealloc_mixed",
            "value": 6759,
            "range": "± 30",
            "unit": "ns/iter"
          },
          {
            "name": "alignment_stress/bumpalo_mixed",
            "value": 8792,
            "range": "± 19",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_write_1000x256B/framealloc_default",
            "value": 741845,
            "range": "± 11915",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_write_1000x256B/framealloc_minimal",
            "value": 742030,
            "range": "± 7204",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_write_1000x256B/framealloc_prefetch",
            "value": 741782,
            "range": "± 10206",
            "unit": "ns/iter"
          },
          {
            "name": "batch_vs_individual_1000x64B/individual_allocs",
            "value": 2508,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "batch_vs_individual_1000x64B/batch_alloc",
            "value": 19,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "batch_vs_individual_1000x64B/small_batch_8x125",
            "value": 220,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "frame_boundary/empty_frame_cycle",
            "value": 14,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "frame_boundary/light_frame_10_allocs",
            "value": 32,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "fallible_vs_regular/regular_alloc",
            "value": 16,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "fallible_vs_regular/fallible_alloc",
            "value": 16,
            "range": "± 0",
            "unit": "ns/iter"
          }
        ]
      },
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
          "id": "9047797415bd202d701240b061a3fd4b5af46969",
          "message": "Fix documentation ordering - arrange versions chronologically (0.6-0.10)\n\n- Reorder README.md sections to show versions in chronological order\n- Thread Coordination (v0.6.0)\n- IDE Integration (v0.7.0)\n- Tokio Integration (v0.8.0)\n- Performance Optimizations (v0.9.0)\n- Rapier Physics Integration (v0.10.0)\n\nCHANGELOG.md and TECHNICAL.md already had correct chronological order",
          "timestamp": "2025-12-23T07:29:20+02:00",
          "tree_id": "71aad850bbac2d739929c47242f5d44db1b34cd7",
          "url": "https://github.com/YelenaTor/framealloc/commit/9047797415bd202d701240b061a3fd4b5af46969"
        },
        "date": 1766468548667,
        "tool": "cargo",
        "benches": [
          {
            "name": "single_alloc_64B/framealloc",
            "value": 4,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "single_alloc_64B/bumpalo",
            "value": 6,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "single_alloc_64B/system_malloc",
            "value": 13,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "single_alloc_64B/box_vec",
            "value": 23,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "single_alloc_256B/framealloc",
            "value": 4,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "single_alloc_256B/bumpalo",
            "value": 26,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "single_alloc_256B/system_malloc",
            "value": 13,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "single_alloc_1KB/framealloc",
            "value": 4,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "single_alloc_1KB/bumpalo",
            "value": 104,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "single_alloc_1KB/system_malloc",
            "value": 13,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "single_alloc_4KB/framealloc",
            "value": 4,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "single_alloc_4KB/bumpalo",
            "value": 431,
            "range": "± 22",
            "unit": "ns/iter"
          },
          {
            "name": "single_alloc_4KB/system_malloc",
            "value": 40,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "single_alloc_64KB/framealloc",
            "value": 5,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "single_alloc_64KB/bumpalo",
            "value": 6354,
            "range": "± 207",
            "unit": "ns/iter"
          },
          {
            "name": "single_alloc_64KB/system_malloc",
            "value": 40,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "batch_100x64B/framealloc",
            "value": 238,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "batch_100x64B/bumpalo",
            "value": 209,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "batch_100x64B/system_malloc",
            "value": 1858,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "batch_1000x64B/framealloc",
            "value": 1791,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "batch_1000x64B/bumpalo",
            "value": 1776,
            "range": "± 723",
            "unit": "ns/iter"
          },
          {
            "name": "batch_1000x64B/system_malloc",
            "value": 34654,
            "range": "± 1044",
            "unit": "ns/iter"
          },
          {
            "name": "batch_10000x64B/framealloc",
            "value": 21790,
            "range": "± 253",
            "unit": "ns/iter"
          },
          {
            "name": "batch_10000x64B/bumpalo",
            "value": 18716,
            "range": "± 61",
            "unit": "ns/iter"
          },
          {
            "name": "batch_10000x64B/system_malloc",
            "value": 333398,
            "range": "± 5699",
            "unit": "ns/iter"
          },
          {
            "name": "workload_physics/framealloc",
            "value": 1485,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "workload_physics/bumpalo",
            "value": 3470,
            "range": "± 182",
            "unit": "ns/iter"
          },
          {
            "name": "workload_physics/system_malloc",
            "value": 26741,
            "range": "± 129",
            "unit": "ns/iter"
          },
          {
            "name": "workload_render/framealloc",
            "value": 12811,
            "range": "± 263",
            "unit": "ns/iter"
          },
          {
            "name": "workload_render/bumpalo",
            "value": 10932,
            "range": "± 500",
            "unit": "ns/iter"
          },
          {
            "name": "workload_render/system_malloc",
            "value": 260127,
            "range": "± 2238",
            "unit": "ns/iter"
          },
          {
            "name": "workload_particles/framealloc",
            "value": 108860,
            "range": "± 2643",
            "unit": "ns/iter"
          },
          {
            "name": "workload_particles/bumpalo",
            "value": 81668,
            "range": "± 261",
            "unit": "ns/iter"
          },
          {
            "name": "workload_particles/system_malloc",
            "value": 1418784,
            "range": "± 31316",
            "unit": "ns/iter"
          },
          {
            "name": "stress_100k_tiny/framealloc",
            "value": 173107,
            "range": "± 859",
            "unit": "ns/iter"
          },
          {
            "name": "stress_100k_tiny/bumpalo",
            "value": 103788,
            "range": "± 359",
            "unit": "ns/iter"
          },
          {
            "name": "stress_100k_tiny/system_malloc",
            "value": 2729848,
            "range": "± 83011",
            "unit": "ns/iter"
          },
          {
            "name": "frame_lifecycle/empty_frame",
            "value": 15,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "frame_lifecycle/light_frame_100",
            "value": 238,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "frame_lifecycle/heavy_frame_10000",
            "value": 21794,
            "range": "± 58",
            "unit": "ns/iter"
          },
          {
            "name": "frame_lifecycle/bumpalo_reset_light",
            "value": 155,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "frame_lifecycle/bumpalo_reset_heavy",
            "value": 18118,
            "range": "± 289",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_write_1000x256B/framealloc",
            "value": 5848,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_write_1000x256B/bumpalo",
            "value": 5323,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_write_1000x256B/system_malloc",
            "value": 33993,
            "range": "± 96",
            "unit": "ns/iter"
          },
          {
            "name": "multithreaded/framealloc/2",
            "value": 137238,
            "range": "± 2090",
            "unit": "ns/iter"
          },
          {
            "name": "multithreaded/system_malloc/2",
            "value": 379771,
            "range": "± 6238",
            "unit": "ns/iter"
          },
          {
            "name": "multithreaded/framealloc/4",
            "value": 216325,
            "range": "± 2004",
            "unit": "ns/iter"
          },
          {
            "name": "multithreaded/system_malloc/4",
            "value": 671217,
            "range": "± 5173",
            "unit": "ns/iter"
          },
          {
            "name": "multithreaded/framealloc/8",
            "value": 429242,
            "range": "± 9677",
            "unit": "ns/iter"
          },
          {
            "name": "multithreaded/system_malloc/8",
            "value": 1443703,
            "range": "± 27082",
            "unit": "ns/iter"
          },
          {
            "name": "alignment_stress/framealloc_mixed",
            "value": 6762,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "alignment_stress/bumpalo_mixed",
            "value": 8827,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_write_1000x256B/framealloc_default",
            "value": 741751,
            "range": "± 4918",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_write_1000x256B/framealloc_minimal",
            "value": 741955,
            "range": "± 1984",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_write_1000x256B/framealloc_prefetch",
            "value": 741624,
            "range": "± 1020",
            "unit": "ns/iter"
          },
          {
            "name": "batch_vs_individual_1000x64B/individual_allocs",
            "value": 2507,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "batch_vs_individual_1000x64B/batch_alloc",
            "value": 19,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "batch_vs_individual_1000x64B/small_batch_8x125",
            "value": 216,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "frame_boundary/empty_frame_cycle",
            "value": 14,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "frame_boundary/light_frame_10_allocs",
            "value": 32,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "fallible_vs_regular/regular_alloc",
            "value": 16,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "fallible_vs_regular/fallible_alloc",
            "value": 16,
            "range": "± 0",
            "unit": "ns/iter"
          }
        ]
      },
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
          "id": "9e07cdf0ac863a727b96d450d8439bffb806e93d",
          "message": "Update framealloc version to 0.10.0 for release",
          "timestamp": "2025-12-23T07:32:13+02:00",
          "tree_id": "f15e4e64c91ac65a563dfa78a7f8b239a6978fbd",
          "url": "https://github.com/YelenaTor/framealloc/commit/9e07cdf0ac863a727b96d450d8439bffb806e93d"
        },
        "date": 1766468721819,
        "tool": "cargo",
        "benches": [
          {
            "name": "single_alloc_64B/framealloc",
            "value": 4,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "single_alloc_64B/bumpalo",
            "value": 7,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "single_alloc_64B/system_malloc",
            "value": 13,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "single_alloc_64B/box_vec",
            "value": 23,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "single_alloc_256B/framealloc",
            "value": 4,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "single_alloc_256B/bumpalo",
            "value": 26,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "single_alloc_256B/system_malloc",
            "value": 13,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "single_alloc_1KB/framealloc",
            "value": 4,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "single_alloc_1KB/bumpalo",
            "value": 107,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "single_alloc_1KB/system_malloc",
            "value": 14,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "single_alloc_4KB/framealloc",
            "value": 4,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "single_alloc_4KB/bumpalo",
            "value": 432,
            "range": "± 26",
            "unit": "ns/iter"
          },
          {
            "name": "single_alloc_4KB/system_malloc",
            "value": 41,
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
            "value": 6355,
            "range": "± 391",
            "unit": "ns/iter"
          },
          {
            "name": "single_alloc_64KB/system_malloc",
            "value": 40,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "batch_100x64B/framealloc",
            "value": 193,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "batch_100x64B/bumpalo",
            "value": 210,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "batch_100x64B/system_malloc",
            "value": 1832,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "batch_1000x64B/framealloc",
            "value": 2198,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "batch_1000x64B/bumpalo",
            "value": 1986,
            "range": "± 117",
            "unit": "ns/iter"
          },
          {
            "name": "batch_1000x64B/system_malloc",
            "value": 34659,
            "range": "± 776",
            "unit": "ns/iter"
          },
          {
            "name": "batch_10000x64B/framealloc",
            "value": 21787,
            "range": "± 177",
            "unit": "ns/iter"
          },
          {
            "name": "batch_10000x64B/bumpalo",
            "value": 18538,
            "range": "± 54",
            "unit": "ns/iter"
          },
          {
            "name": "batch_10000x64B/system_malloc",
            "value": 340395,
            "range": "± 2126",
            "unit": "ns/iter"
          },
          {
            "name": "workload_physics/framealloc",
            "value": 1597,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "workload_physics/bumpalo",
            "value": 2299,
            "range": "± 58",
            "unit": "ns/iter"
          },
          {
            "name": "workload_physics/system_malloc",
            "value": 27738,
            "range": "± 69",
            "unit": "ns/iter"
          },
          {
            "name": "workload_render/framealloc",
            "value": 16084,
            "range": "± 57",
            "unit": "ns/iter"
          },
          {
            "name": "workload_render/bumpalo",
            "value": 10337,
            "range": "± 104",
            "unit": "ns/iter"
          },
          {
            "name": "workload_render/system_malloc",
            "value": 266377,
            "range": "± 2103",
            "unit": "ns/iter"
          },
          {
            "name": "workload_particles/framealloc",
            "value": 108863,
            "range": "± 3054",
            "unit": "ns/iter"
          },
          {
            "name": "workload_particles/bumpalo",
            "value": 77397,
            "range": "± 344",
            "unit": "ns/iter"
          },
          {
            "name": "workload_particles/system_malloc",
            "value": 1482864,
            "range": "± 40164",
            "unit": "ns/iter"
          },
          {
            "name": "stress_100k_tiny/framealloc",
            "value": 177058,
            "range": "± 848",
            "unit": "ns/iter"
          },
          {
            "name": "stress_100k_tiny/bumpalo",
            "value": 104109,
            "range": "± 931",
            "unit": "ns/iter"
          },
          {
            "name": "stress_100k_tiny/system_malloc",
            "value": 2733185,
            "range": "± 12254",
            "unit": "ns/iter"
          },
          {
            "name": "frame_lifecycle/empty_frame",
            "value": 16,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "frame_lifecycle/light_frame_100",
            "value": 193,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "frame_lifecycle/heavy_frame_10000",
            "value": 21786,
            "range": "± 67",
            "unit": "ns/iter"
          },
          {
            "name": "frame_lifecycle/bumpalo_reset_light",
            "value": 185,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "frame_lifecycle/bumpalo_reset_heavy",
            "value": 19928,
            "range": "± 102",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_write_1000x256B/framealloc",
            "value": 5639,
            "range": "± 75",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_write_1000x256B/bumpalo",
            "value": 5566,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_write_1000x256B/system_malloc",
            "value": 33528,
            "range": "± 223",
            "unit": "ns/iter"
          },
          {
            "name": "multithreaded/framealloc/2",
            "value": 133389,
            "range": "± 2315",
            "unit": "ns/iter"
          },
          {
            "name": "multithreaded/system_malloc/2",
            "value": 372470,
            "range": "± 5906",
            "unit": "ns/iter"
          },
          {
            "name": "multithreaded/framealloc/4",
            "value": 197987,
            "range": "± 2149",
            "unit": "ns/iter"
          },
          {
            "name": "multithreaded/system_malloc/4",
            "value": 660565,
            "range": "± 9617",
            "unit": "ns/iter"
          },
          {
            "name": "multithreaded/framealloc/8",
            "value": 421489,
            "range": "± 5375",
            "unit": "ns/iter"
          },
          {
            "name": "multithreaded/system_malloc/8",
            "value": 1396508,
            "range": "± 34524",
            "unit": "ns/iter"
          },
          {
            "name": "alignment_stress/framealloc_mixed",
            "value": 6846,
            "range": "± 26",
            "unit": "ns/iter"
          },
          {
            "name": "alignment_stress/bumpalo_mixed",
            "value": 8910,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_write_1000x256B/framealloc_default",
            "value": 740991,
            "range": "± 2858",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_write_1000x256B/framealloc_minimal",
            "value": 741057,
            "range": "± 3077",
            "unit": "ns/iter"
          },
          {
            "name": "alloc_write_1000x256B/framealloc_prefetch",
            "value": 740885,
            "range": "± 6658",
            "unit": "ns/iter"
          },
          {
            "name": "batch_vs_individual_1000x64B/individual_allocs",
            "value": 1581,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "batch_vs_individual_1000x64B/batch_alloc",
            "value": 20,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "batch_vs_individual_1000x64B/small_batch_8x125",
            "value": 216,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "frame_boundary/empty_frame_cycle",
            "value": 14,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "frame_boundary/light_frame_10_allocs",
            "value": 32,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "fallible_vs_regular/regular_alloc",
            "value": 16,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "fallible_vs_regular/fallible_alloc",
            "value": 16,
            "range": "± 0",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}