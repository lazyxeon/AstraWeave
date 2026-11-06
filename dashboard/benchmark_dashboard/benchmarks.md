---
layout: default
title: Performance Benchmarks
---

# Performance Benchmarks

All benchmarks run using Criterion.rs. Values represent median latencies. **~2,830 benchmarks** across **40+ crates**. Overall grade: **A+**.

## Executive Summary

| Metric | Value |
|--------|-------|
| Frame time (1K entities) | 0.97 ms (system) / 0.71 ms (mimalloc) — 2.70 ms was the target |
| FPS (1K entities) | ~1,036 (system) / ~1,410 (mimalloc) |
| p50 frame time | 1.27 ms |
| p99 frame time | 2.42 ms (85% under budget) |
| Agent capacity @ 60 FPS | **12,700+** (18.8× over target) |
| Anti-cheat validation | 6.48M checks/sec |
| Determinism | 100% bit-identical |
| Budget headroom vs 60 FPS | ~94–96% |

## ECS (astraweave-ecs)

| Benchmark | Value |
|-----------|-------|
| Entity spawn (empty) | 50 ns |
| Entity despawn (empty) | 24 ns |
| Entity spawn/10K | 645 µs |
| Component iteration/10K | 273 µs |
| SparseSet insert/1K | 9.9 ns/entity (13× faster than BTreeMap) |
| SparseSet lookup/1K | 1.56 ns/entity (37× faster than BTreeMap) |
| BlobVec insert/1K | 10–24× faster than `Vec<Box<dyn Any>>` |
| WorldSnapshot clone (simple) | 449 ns |
| WorldSnapshot clone (100 enemies) | 14.9 µs (~149 ns/enemy) |
| World hash/1K entities | 14.5 ns/entity |
| ECS stress (1K entities) | 508.96 µs |

**60 FPS capacity**: 1K entities = ~85 µs (0.51%), 5K = ~529 µs (3.17%), 10K = ~1 ms (~6%)

## AI Planning (astraweave-ai)

| Benchmark | Value |
|-----------|-------|
| GOAP next_action (no enemies) | **3.46–3.56 ns** |
| GOAP next_action (close) | 4.68–5.11 ns |
| GOAP next_action (far) | 7.04–7.86 ns |
| GOAP planning (cache hit) | 9.8 ns |
| GOAP planning (cache miss) | 286 ns |
| Multi-agent/10 agents | 1.34–1.39 µs |
| AIArbiter GOAP control | 101.7 ns |
| AIArbiter LLM polling | 575 ns |
| AIArbiter mode transition | 221.9 ns |
| Full arbiter cycle | <2 µs |
| Capacity @ 10% budget | ~186,000 agents |

## Behavior Trees (astraweave-behavior)

| Benchmark | Value |
|-----------|-------|
| BT tick/1K nodes | **3.19 µs** (3.19 ns/node) |
| GOAP plan generation | Sub-µs |
| Utility evaluation | Sub-µs |

## NPC (astraweave-npc)

| Benchmark | Value |
|-----------|-------|
| BT evaluation/1K | 420–580 ns/eval |
| State transitions/5K | 6–11.6 ns/transition |
| Emotion blending/1K | 320–450 ns/blend |
| Threat assessment/500 | 5.6–6 ns/assess |
| Schedule lookup/10K | 17.5–19.5 ns/lookup |
| Vision cone/5K | 2.2–2.8 ms |
| Dialogue traversal/500 | 94–112 ns/traverse |
| Capacity | 10,000+ NPCs @ 60 FPS |

## Director (astraweave-director)

| Benchmark | Value |
|-----------|-------|
| Boss phase determination | 8.0–8.9 µs |
| Boss plan generation (3 phases) | 311–350 µs |
| Minion AI update/1K | 177–214 µs |
| LLM decision (simple) | 8.2–8.6 µs |
| Difficulty calculation | 4.9–5.9 µs |
| Minion capacity | **1.4M minions** |

## Coordination (astraweave-coordination)

| Benchmark | Value |
|-----------|-------|
| Squad formation/1K | 725 ps |
| Event filtering/1K | 1.10 ns |
| Consensus building/1K | 15.12 ns (660× faster than budget) |
| Social graph/1K agents | 465 ns |
| Capacity | 100,000+ agents/frame |

## Physics (astraweave-physics)

| Benchmark | Value |
|-----------|-------|
| Character move | 43.8–52.0 ns |
| Character controller tick | 6.5 µs |
| Rigid body transform lookup | 14.8–15.4 ns |
| Rigid body batch (100) | 47 µs |
| Raycast (empty scene) | 26.3–31.5 ns |
| Raycast throughput | 28M rays/sec |
| Spatial hash | 99.96% fewer collision checks |

## Rendering (astraweave-render)

| Benchmark | Value |
|-----------|-------|
| Frustum AABB inside | 889–915 ps |
| AABB contains point | 951 ps–1.01 ns |
| Camera view matrix | 4.42–5.36 ns |
| Camera toggle mode | 1.72–2.29 ns |
| Vertex compression encode | 21 ns |
| Instancing savings calc | 1.43–1.52 ns |
| Weather particle update | 1.95–2.04 ns |
| Weather light attenuation | 730–783 ps |
| Mesh memory size | 816 ps |
| Fresnel calculation | 1.63 ns |
| Mipmap level 3 | 390–451 ps |
| Tween update | 22.1 ns |
| Spring update | 14.2 ns |
| Full culling/200K instances | 60 FPS, 6.7 ns/instance |
| VSM sampling | 30× faster than PCF |
| GPU budget check | 890 ps–1.05 ns |

## SIMD Math (astraweave-math)

| Benchmark | Scalar | SIMD | Speedup |
|-----------|--------|------|---------|
| Batch movement/10K | 20.6 µs | 9.879 µs | 2.08× |
| quat_multiply | — | 797 ps | Sub-ns |
| Mat4 multiply | — | 4.28 ns | 2.5× |
| Quat slerp | — | 2.10 ns | 1.75× |

## Fluids (astraweave-fluids)

| Benchmark | Value |
|-----------|-------|
| GPU position buffer/10K | 0.9–1.15 ns (sub-ns) |
| Particle ops (1K–10K) | 5.3–110 µs (100–322 Melem/s) |
| SPH kernels/100K | 171–223 µs |
| Density/pressure (2–5K) | 3.5–10.5 ms |
| Single step/1K particles | 1.8–3.0 ms (12% frame budget) |
| Multi-step sim/500×10 | 450–500 µs (45–57% faster) |
| Adaptive timestep/1K | 3.3–4.0 µs |

## Navigation (astraweave-nav)

| Benchmark | Value |
|-----------|-------|
| A* short (2–5 hops) | 2.44 µs |
| A* medium (10–20 hops) | 54.5 µs |
| A* long (50–100 hops) | 17.0 µs |
| Navmesh bake/100 tri | 55.9 µs |
| Navmesh bake/10K tri | 473 ms (must be async) |
| Throughput/100 tri | 142K QPS |
| Sliver triangle handling | 99–104 ps/tri (sub-ns) |
| Snake maze/50 turns | 101–108 µs |
| Impossible paths (fast-fail) | 3.7–24.9 µs |

## Terrain (astraweave-terrain)

| Benchmark | Value |
|-----------|-------|
| World chunk generation | 48.6 ms (async required) |
| Climate sampling | 782 ns |
| Chunk under 60 FPS budget | 15.06 ms |

## Gameplay & Combat

| Benchmark | Value |
|-----------|-------|
| Single attack sweep | 81.3 ns |
| Rapid 100 hits | 3.3–4.0 ns/hit |
| Capacity | 73K combatants @ 60 FPS |

## Other Systems

| System | Benchmark | Value |
|--------|-----------|-------|
| **Core** | Full game loop/100 entities | 64.8 µs |
| **Core** | World creation | 25.8 ns |
| **Core** | Tick/1K entities | 1.39 ms |
| **Input** | Binding creation | 4.67 ns |
| **Input** | is_down query | 978 ps (sub-ns) |
| **Audio** | Pan mode switching | 418 ps |
| **UI** | Settings navigation | 696 ps |
| **UI** | Full pipeline | <10 µs (0.06% budget) |
| **Persistence** | Full snapshot/10K | 4.7–5.9 ms |
| **Persistence** | Checksum verify/100KB | ~40 GB/s |
| **Save** | CRC32 throughput | 13 GB/s |
| **Save** | Save operation | 19.31 ms |
| **Save** | Load operation | 376.63 µs |
| **Security** | Operation counting/100K | 0.45–0.53 ns/op |
| **Secrets** | Encryption scaling | 1.15–1.90 ns O(1) |
| **Profiling** | Zone creation | 2.1 ns/zone |
| **Context** | Context switching | 2.38 ns (7M switches/frame) |
| **Prompts** | Template render (simple) | 999 ns |
| **Net-ECS** | 16-client multiplayer | ~0.4 ms (2.4% budget) |
| **LLM** | Streaming TTFC | 44.3× faster |
| **LLM** | Cache hit speedup | 90,751× vs miss |
| **Materials** | Shader cache hit | 15–28 ns |
| **Editor** | Gizmo state transitions | 342–536 ps |
| **Persona** | profile_verify | 544.68 ps (fastest in engine) |

## 60 FPS Budget Analysis

At 60 FPS, each frame has 16.67 ms of budget:

| System | Time | Budget % |
|--------|------|----------|
| Physics (full tick) | 6.52 µs | 0.04% |
| AI (12,700 agents) | ~4 ms | 24% |
| Rendering (1K entities) | 2.70 ms (target — rendering-only cost not separately measured) | 16% |
| **Total** | **~6.7 ms** | **40%** |
| **Remaining headroom** | **~10 ms** | **60%** |

[← Back to Home](index.html)
