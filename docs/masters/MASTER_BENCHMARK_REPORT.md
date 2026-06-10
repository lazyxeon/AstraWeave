# AstraWeave Master Benchmark Report

> **Version**: 5.57 | **Date**: 2026-06-10 | **Grade**: A+ | **Framework**: Criterion.rs (statistical)

---

## Executive Summary

| Metric | Value |
|--------|-------|
| **Total Benchmarks** | ~2,830 across 76 sections |
| **Crates Benchmarked** | 40+ |
| **Framework** | Criterion.rs (statistical confidence intervals) |
| **Overall Grade** | A+ (Production Ready) |
| **Frame Time** | 2.70 ms @ 1,000 entities (84% headroom vs 60 FPS) |
| **Agent Capacity** | 12,700+ @ 60 FPS (18.8x over initial target) |
| **Validation Throughput** | 6.48M checks/sec |
| **Determinism** | 100% bit-identical across runs |

### mimalloc / fast-alloc Allocator Merge (v5.57)

Paired baseline-vs-mimalloc measurement (3 independent runs per cell, 2026-04-17; merged same day — recorded here per the >10%-change update rule, which this entry back-fills):

| Benchmark | Baseline | mimalloc | Delta |
|-----------|----------|----------|-------|
| `ai.goap.plan/actions_16` | 767.7 µs | 371.7 µs | **−51.6%** |
| `profiling_demo` avg FPS (1,000 entities / 1,000 frames) | 956.03 | 1,368.60 | **+43.2%** |
| `physics.step` | — | — | −5.2% (within noise) |
| `render.bin_lights_cpu` | — | — | −3.6% (within noise) |

- `ecs.schedule.run/systems_16` −23.1% (2.73 → 2.10 µs) is **HISTORICAL**: measured on the `astraweave-ecs` `alloc_measure` harness deleted 2026-04-18 with `ParallelSchedule` — not reproducible on current code.
- **Three** extant `alloc_measure` Criterion benches registered: `astraweave-ai`, `astraweave-physics`, `astraweave-render` (added 2026-04-17; the fourth, ecs, was deleted with ParallelSchedule).
- `fast-alloc` (via the `astraweave-alloc` crate) is **default-on** in `examples/profiling_demo`, `examples/hello_companion`, and `tools/aw_editor`; library crates keep it opt-in. Opt-out verified: 855 FPS `--no-default-features` vs 1,458 FPS default.
- Allocs/bytes/reallocs per frame identical across allocators (sanity check passed). Sources: `docs/audits/mimalloc_experiment_2026-04-17.md`, `docs/audits/allocation_measurement_plan_2026-04-17.md`.

### Qwen3-8B LLM Latency Breakthrough (v5.56)

- Qwen3-8B: **3.60× faster** than Hermes 2 Pro (blocking), **76.9% faster** than unoptimized baseline
- Steady-state TTFC: **160ms** (within 6% of theoretical hardware minimum)
- 4 optimization rounds: 15 changes accepted, 9 rejected with empirical evidence
- Software-level optimization space exhausted; performance floor reached

### Engine Validation (v5.55)

- 5,372/5,383 tests passing (99.8%)
- 17/17 core crates with zero errors/warnings
- RAG DashMap deadlock fixed (P0 critical)
- Frame time p99: 2.42 ms (85% under budget)

### ECS Regression Fix (v5.53)

BlobVec lazy initialization eliminated unnecessary HashMap allocations in `Archetype::new()`.

| Benchmark | Before (v5.52) | After (v5.53) | Improvement |
|-----------|----------------|---------------|-------------|
| entity_spawn/empty/10K | 1.34 ms | 645 µs | 52% faster |
| entity_despawn/empty/10K | +388% regressed | 287 µs | Fixed |
| component_iteration/10K | Degraded | 273 µs | 68-75% faster |

---

## Benchmark Hardware

All benchmarks in this report were performed on the following system. Results may vary on different hardware.

| Component | Specification |
|-----------|---------------|
| **System** | HP Pavilion Gaming Laptop 16-a0xxx |
| **CPU** | Intel Core i5-10300H @ 2.50 GHz (4 cores / 8 threads) |
| **RAM** | 32 GB DDR4 @ 3200 MHz (2x 16 GB) |
| **GPU (Discrete)** | NVIDIA GeForce GTX 1660 Ti Max-Q (4 GB VRAM) |
| **GPU (Integrated)** | Intel UHD Graphics |
| **Storage** | Intel Optane + 477 GB SSD |
| **OS** | Microsoft Windows 11 Home (Build 26200, 64-bit) |
| **Rust Toolchain** | 1.89.0 (stable, x86_64-pc-windows-msvc) |
| **Cargo** | 1.89.0 |
| **Benchmark Framework** | Criterion.rs (statistical, 100+ iterations per benchmark) |
| **Power Profile** | Plugged in, High Performance |

---

## Benchmark Execution

```powershell
# Full workspace (30-60 min)
cargo bench --workspace --exclude astraweave-author --exclude visual_3d --exclude ui_controls_demo

# Core crates (5-10 min)
cargo bench -p astraweave-core -p astraweave-ecs -p astraweave-ai -p astraweave-physics

# Automated odyssey
./scripts/benchmark_odyssey.ps1
```

Per-crate: `cargo bench -p <crate-name>` (see individual sections below).

---

## 60 FPS Budget Analysis

**Frame Budget**: 16.67 ms (60 FPS target)

### Current Allocation (1,000 entities, validated)

| System | Budget | Current | Headroom | Status |
|--------|--------|---------|----------|--------|
| ECS | 2 ms (12%) | ~0.5 ms | +75% | EXCELLENT |
| AI | 5 ms (30%) | ~0.3 ms | +94% | EXCELLENT |
| Physics | 3 ms (18%) | ~2.0 ms | +33% | GOOD |
| Rendering | 6 ms (36%) | ~5-8 ms (est.) | ~3k draws | ESTIMATED |
| Misc | 0.67 ms (4%) | ~0.2 ms | +70% | GOOD |
| **TOTAL** | **16.67 ms** | **~2.70 ms** | **+84%** | **EXCELLENT** |

### Scalability Projections

| Scale | Frame Time | FPS | Status |
|-------|-----------|-----|--------|
| 1,000 entities | 2.70 ms | 370 | Validated |
| 10,000 entities | ~22 ms (est.) | ~45 | Playable (optimization path identified) |

10K @ 60 FPS achievable with: parallel ECS scheduling (-30%), LLM batch inference (-50%), GPU culling (-40%).

---

## 1. Core Engine

### 1.1 astraweave-ecs

ECS storage, entity lifecycle, and data structure performance.

| Benchmark | Time | Notes |
|-----------|------|-------|
| entity_spawn/empty | 50 ns | Industry-leading |
| entity_despawn/empty | 24 ns | Industry-leading |
| entity_spawn/10K | 645 µs (post-fix) | 52% faster after v5.53 |
| component_iteration/10K | 273 µs | 68-75% faster after v5.53 |
| SparseSet insert/1K | 9.9 ns/entity | 13x faster than BTreeMap |
| SparseSet lookup/1K | 1.6 ns/entity | 37x faster than BTreeMap, O(1) |
| BlobVec insert/1K | 10-24x faster than `Vec<Box<dyn Any>>` | |
| WorldSnapshot clone/simple | 449 ns | |
| WorldSnapshot clone/100 enemies | 14.9 µs | ~149 ns/enemy, linear |
| World hash/1K entities | 14.5 ns/entity | O(n) scaling |

**Adversarial stress tests**: Archetype explosion (90% fragmentation <2% impact), bullet hell (2.03 ms @ 10K entities), concurrent safety validated.

### 1.2 astraweave-core

| Benchmark | Time | Notes |
|-----------|------|-------|
| Full game loop/100 entities | 64.8 µs | |
| World creation | 25.8 ns | |
| Tick/1K entities | 1.39 ms | Stress test baseline |

### 1.3 astraweave-math

| Benchmark | Time | Notes |
|-----------|------|-------|
| SIMD movement/10K | 9.879 µs | 2.08x speedup vs scalar |
| quat_multiply | 797 ps | Sub-nanosecond |
| vec3_normalize (scalar) | 5x faster than SIMD wrapper | glam already auto-vectorizes |
| Infinity handling | 27-30 ns | IEEE-754 compliant |
| NaN propagation | 30-32 ns | Safe |
| Denormal operations | 23-27 ns | No slowdown |

**Key insight**: Scalar beats explicit SIMD wrappers because glam already uses SIMD auto-vectorization (80-85% of hand-written AVX2). Trust `glam`.

### 1.4 astraweave-physics

| Benchmark | Time | Notes |
|-----------|------|-------|
| Raycast throughput | 28M rays/sec | |
| Character controller tick | 6.5 µs | |
| Rigid body (post v5.48) | 10x faster | Breakthrough optimization |
| Spatial hash | 99.96% fewer collision checks | O(n log n), cascade 9-17% to all systems |

### 1.5 astraweave-input

| Benchmark | Time | Notes |
|-----------|------|-------|
| Binding creation | 4.67 ns | |
| is_down query | 978 ps | Sub-nanosecond |
| clear_frame/1K | 0.77-1.0 ns/op | Sub-nanosecond |
| Query all actions | 49-65 ns | |

**Adversarial**: Input storm causes ZERO performance issues.

---

## 2. AI Systems

### 2.1 astraweave-ai

| Benchmark | Time | Notes |
|-----------|------|-------|
| GOAP next_action | 3.46-3.56 ns | Sub-4ns (idle detection FREE) |
| Multi-agent/10 agents | 66-68% faster (v5.49) | |
| Tool validation | 65-68% faster (v5.49) | |
| Orchestrator tick | Sub-µs | |
| Arbiter cycle | <2 µs | 27,000x faster than Phase 3 target |

**Capacity**: ~186,000 agents @ 10% budget.

### 2.2 astraweave-behavior

| Benchmark | Time | Notes |
|-----------|------|-------|
| BT tick/1K nodes | 3.19 µs | 3.19 ns/node |
| GOAP plan generation | Sub-µs | |
| Utility evaluation | Sub-µs | |

### 2.3 astraweave-director

| Benchmark | Time | Notes |
|-----------|------|-------|
| Boss phase determination | 8.0-8.9 µs | Sub-10µs |
| Boss plan generation/3 phases | 311-350 µs | Linear scaling |
| Minion AI update/1K | 177-214 µs | **1.4M minion capacity @ 60 FPS** |
| LLM decision (simple) | 8.2-8.6 µs | Sub-10µs |
| Difficulty calculation | 4.9-5.9 µs | Near-instant |

### 2.4 astraweave-npc

| Benchmark | Time | Notes |
|-----------|------|-------|
| Behavior tree eval/1K | 420-580 ns/eval | |
| State transitions/5K | 6-11.6 ns/transition | Sub-12ns |
| Emotion blending/1K | 320-450 ns/blend | |
| Threat assessment/500 | 5.6-6 ns/assess | Essentially free |
| Schedule lookup/10K | 17.5-19.5 ns/lookup | |
| Vision cone/5K | 2.2-2.8 ms | |
| Dialogue traversal/500 | 94-112 ns/traverse | |

**Capacity**: 10,000+ NPCs with full behavior trees @ 60 FPS.

### 2.5 astraweave-coordination

| Benchmark | Time | Notes |
|-----------|------|-------|
| Squad formation/1K | 725 ps | Sub-nanosecond |
| Event filtering/1K | 1.10 ns | |
| Consensus building/1K | 15.12 ns | 660x faster than budget |
| Social graph/1K agents | 465 ns | |

**Capacity**: 100,000+ agents with complex coordination per frame.

---

## 3. AI Memory & LLM

### 3.1 astraweave-memory

| Benchmark | Time | Notes |
|-----------|------|-------|
| Memory creation | 246 ns | 67K/frame capacity |
| Storage/50 | 40.9 µs | |
| Retrieval by ID | 4.75 µs | |
| Importance update | 119 ns | 139K/frame capacity |

### 3.2 astraweave-context

| Benchmark | Time | Notes |
|-----------|------|-------|
| Message creation | 220 ns | 75K/frame |
| Context switching | 2.38 ns | 7.0M switches/frame capacity |
| Context window creation | 721 ns | |
| Window with stats | 44.9 ns | Sub-100ns token counting |
| Add 50 messages | 75 µs | 1.5 µs/message |
| Conversation creation | 1.19-1.23 µs | |
| Telemetry record | 38.9 ns | Zero-overhead observability |

### 3.3 astraweave-persona

| Benchmark | Time | Notes |
|-----------|------|-------|
| profile_verify | **544.68 ps** | **Fastest benchmark in AstraWeave** |
| Persona default | 15.7 ns | |
| Profile creation | 73.2 ns | |
| Profile comprehensive/140 items | 47.2 µs | |
| Profile serialize JSON | 5.1 µs | |
| Profile deserialize JSON | 25.7 µs | |

### 3.4 astraweave-prompts

| Benchmark | Time | Notes |
|-----------|------|-------|
| Engine creation | 7.29 ns | Zero-cost abstraction |
| Template render/simple | 999 ns | Sub-1µs |
| Template render/complex | 8.75 µs | |
| Batch render/100 | 113 µs | 1.13 µs/template |
| Template registration | ~195 µs/template | One-time setup |

### 3.5 astraweave-rag

| Benchmark | Time | Notes |
|-----------|------|-------|
| Retrieval engine creation | 2.18 ns | Zero-cost abstraction |
| Simple search | 8.22 µs | |
| Search scaling/1K | 123.8 µs | O(n) linear |
| Similarity calculation | 711 ns | |
| Context injection | 1.08-1.11 µs | Sub-1.2µs |
| Memory decay | 21-28 ns/memory | Essentially free |
| MMR sampling/100 | 182-192 µs | Diversity sampling |

### 3.6 astraweave-llm

#### Qwen3-8B vs Hermes 2 Pro — Head-to-Head Latency (Final, Post Round 4)

| Metric | Qwen3-8B | Hermes 2 Pro | Winner | Ratio |
|--------|----------|--------------|--------|-------|
| **Warmup (cold→hot)** | 4,692 ms | 13,846 ms | **Qwen3** | 2.95× |
| **Avg blocking** | 2,745 ms | 9,867 ms | **Qwen3** | 3.60× |
| **Best blocking** | 2,562 ms | 8,224 ms | **Qwen3** | 3.21× |
| **Avg streaming** | 2,707 ms | 8,862 ms | **Qwen3** | 3.27× |
| **Avg TTFC** | 274 ms | 184 ms | **Hermes** | 0.67× |
| **Steady-state TTFC** | 160 ms | 80 ms | **Hermes** | 0.50× |
| Avg response (chars) | 287 | 600 | n/a | — |

**Score**: Qwen3 4/5 | Hermes 1/5. **Verdict**: Qwen3-8B is the faster model.

#### Optimization Progression (4 Rounds, 15 Changes)

| Metric | Baseline | Round 1 | Round 2 | Round 3 | Round 4 | Total |
|--------|----------|---------|---------|---------|---------|-------|
| Avg blocking | 11,880 ms | 9,574 ms | 5,611 ms | 3,075 ms | 2,745 ms | **-76.9%** |
| Avg streaming | 11,157 ms | 9,034 ms | 5,570 ms | 2,808 ms | 2,707 ms | **-75.7%** |
| TTFC (steady) | 515 ms | 319 ms | 275 ms | 210 ms | 160 ms | **-69.0%** |
| vs Hermes | 1.84× slower | 1.54× slower | 1.32× faster | 3.94× faster | 3.60× faster | Inverted |

**Key optimizations**: sampler pruning, context window reduction (32K→2048), KV prefix caching (`num_keep=-1`),
model persistence (`keep_alive=-1`), enhanced preload with KV cache warming, FAST_SYSTEM_PROMPT (~15 tokens).

**Rejected parameters** (empirically tested, no benefit or regression): `mlock`, `num_thread`, `num_batch=2048`,
`OLLAMA_KV_CACHE_TYPE` (q8_0/q4_0), `temperature=0`, `format:"json"`, minimal system prompt without schema.

#### TTFC Theoretical Floor Analysis

| Component | Measured | Notes |
|-----------|----------|-------|
| Ollama scheduler | ~80-100 ms | Irreducible (request routing + graph setup) |
| Prompt eval (cached) | ~25 ms | Near-zero with `num_keep=-1` |
| First token generation | ~30 ms | GPU compute bound (~30 ms/tok) |
| HTTP transport | ~5-10 ms | Local loopback |
| **Theoretical minimum** | **~135-170 ms** | |
| **Measured steady-state** | **160 ms** | **Within 6% of floor** |

#### Infrastructure Benchmarks

| Benchmark | Time | Notes |
|-----------|------|-------|
| Streaming (time-to-first-chunk) | 44.3x faster | Validated |
| Resilience chaos engineering | 4.28-6.74 µs | Circuit breaker validated |
| Cache hit speedup | 90,751x vs miss | |
| Preload (KV cache warming) | 4,692 ms | One-time; eliminates cold-start penalty |

**Hardware**: Intel i5-10300H, GTX 1660 Ti Max-Q (6GB), Ollama (FLASH_ATTENTION=1).
**Models**: `qwen3:8b` (5.2GB Q4_K_M), `adrienbrault/nous-hermes2pro:Q4_K_M` (4.4GB).
**Full report**: `docs/current/QWEN3_LATENCY_OPTIMIZATION_REPORT.md` (v3.0.0).

### 3.7 astraweave-llm-eval

| Benchmark | Time | Notes |
|-----------|------|-------|
| Prompt creation/100 | 128-137 µs | 1.3 µs/prompt |
| Score responses/1K | 149-160 ns/score | |
| Full pipeline/100 | 233-258 µs | 2.3-2.6 µs/item |
| Metric aggregation/100K | 5.7-6.6 ns/score | Sub-10ns at scale |

---

## 4. Rendering

The rendering subsystem spans 20+ benchmark phases with ~700 benchmarks total. Key results per phase:

### 4.1 Render Core & Mesh

| Benchmark | Time | Notes |
|-----------|------|-------|
| Mesh memory size | 816 ps | Sub-nanosecond |
| Vertex compression encode | 21 ns | |
| Render graph execution | O(n) scaling | |
| Material batch upload | Sub-linear | |

### 4.2 Post-Processing (~50 benchmarks)

SSAO, Bloom, CSM, TAA — all validated at production quality.

| Benchmark | Time | Notes |
|-----------|------|-------|
| SSAO sample | Sub-µs baseline | |
| Bloom downsample | Sub-µs | |
| CSM cascade | Sub-µs | |
| TAA resolve | Sub-µs | |

### 4.3 IBL & Deferred (~58 benchmarks)

| Key Result | Notes |
|------------|-------|
| SH16 coefficient count | Optimal for quality/performance |
| BRDF LUT | 64x64 optimal |
| G-buffer pack/unpack | Sub-µs |
| GGX importance sampling | Validated |

### 4.4 GPU Particles & Water (~43 benchmarks)

| Benchmark | Time | Notes |
|-----------|------|-------|
| Fresnel calculation | 1.63 ns | |
| Particle update/emit/sort | Sub-µs per batch | |

### 4.5 SSR, Decals & Weather (~52 benchmarks)

| Key Result | Notes |
|------------|-------|
| Decal system @ 500 | 0.05% frame budget |
| Weather particles | TERAELEM/s throughput |

### 4.6 Animation & Skinning (37 benchmarks)

| Key Result | Notes |
|------------|-------|
| 400+ characters @ 60 FPS | Validated |
| FK chain evaluation | Sub-µs |
| Blend tree | Sub-µs |
| Spring animation | 1.6x faster than Tween |

### 4.7 GPU Culling & LOD (49 benchmarks)

| Benchmark | Time | Notes |
|-----------|------|-------|
| Frustum AABB test | Sub-ns | |
| Full culling pipeline/200K instances | 60 FPS | O(1) per instance at 6.7 ns |
| Edge collapse (mesh simplify) | Sub-µs | |
| Quadric error metric | Sub-µs | |

### 4.8 Nanite GPU Culling & Shadow CSM (49 benchmarks)

| Benchmark | Time | Notes |
|-----------|------|-------|
| Hi-Z pyramid | Sub-µs | |
| Meshlet culling batch | Sub-µs | |
| VSM sampling | 30x faster than PCF | Key finding |
| Cascade shadow matrix | Sub-µs | |

### 4.9 Texture Streaming & VXGI (51 benchmarks)

| Benchmark | Time | Notes |
|-----------|------|-------|
| LRU cache operations | Sub-µs | |
| Cone tracing | 1.27 µs | |
| Voxelization | Sub-ms | VXGI must run on GPU |

### 4.10 Clustered MegaLights & Residency (54 benchmarks)

| Benchmark | Time | Notes |
|-----------|------|-------|
| Light-cluster intersection | Sub-5 ns | |
| CPU light binning | Validated, but GPU 68x faster | GPU compute MANDATORY for >100 lights |
| Prefix sum | Sub-µs | |

### 4.11 GPU Memory, Terrain Materials, Skinning, Depth, Overlay, Post-Proc (~79 benchmarks)

Sub-nanosecond operations discovered: 998 ps, 999 ps, 929 ps. DOF focus plane optimized.

### 4.12 Scene, World Partition, Streaming (~70 benchmarks)

6 sub-nanosecond operations: manhattan_distance (969 ps), contains_point (951 ps), intersects_separate (914 ps), frustum_aabb_inside (889 ps), can_allocate (983 ps / 890 ps).

### 4.13 Transparency, Environment, MSAA, Camera, Primitives, Instancing (~131 benchmarks)

5 sub-nanosecond operations. Weather particles achieve TERAELEM/s throughput. Mipmap level 3 at 390-451 ps (#2 fastest in engine).

### 4.14 astraweave-materials (25 benchmarks)

| Benchmark | Time | Notes |
|-----------|------|-------|
| Trig evaluation | 39-56% improved | |
| Parameter updates | 36-56% improved | |
| Shader cache hit | 15-28 ns | Sub-30ns |
| Parameter lookup | 8.5-11.5 ns | |
| Edge connection | 29-39 ns | Graph building essentially free |

---

## 5. Gameplay & World

### 5.1 astraweave-gameplay

| Benchmark | Time | Notes |
|-----------|------|-------|
| Single attack sweep | 81.3 ns | |
| Massive damage | 3.9-8.0 ns | Sub-10ns extreme handling |
| Rapid 100 hits | 3.3-4.0 ns/hit | Burst damage free |
| Defense mitigation | 296-337 ns | |
| **Capacity** | 73K combatants | @ 60 FPS |

### 5.2 astraweave-weaving

| Benchmark | Time | Notes |
|-----------|------|-------|
| Adjudication | Sub-picosecond | |
| Pattern classification/1K | 2.4-2.8 ns/pattern | Sub-3ns |
| Empty patterns | 12.2-19 ns | |
| Strength boundaries | 120-133 ns | |

### 5.3 astraweave-cinematics

| Benchmark | Time | Notes |
|-----------|------|-------|
| Timeline creation empty | 166 ns | |
| Timeline creation/100 tracks | 39.5 µs | Per-track cost decreases at scale |
| 1K tracks step | 4.7-5.0 µs | Sub-5µs |
| Zero duration timeline | 15.6-16.4 ns | Edge case optimized |
| Hour-long timeline | 346-369 ns | Duration-independent |
| Playback/10s sequence | 708 ns/frame | |

### 5.4 astraweave-audio

| Benchmark | Time | Notes |
|-----------|------|-------|
| Tick (O(1)) | Constant time | |
| Pan mode switching | 418 ps | #1 fastest in engine (at time of benchmark) |
| Spatial audio | Sub-µs | |
| Voice beep generation | 367 ns | |
| 1,000 simultaneous sounds | Validated, no dropouts | |

### 5.5 astraweave-nav

| Benchmark | Time | Notes |
|-----------|------|-------|
| A* short path (2-5 hops) | 2.44 µs | |
| A* medium path (10-20 hops) | 54.5 µs | |
| A* long path (50-100 hops) | 17.0 µs | Optimized heuristics |
| Navmesh bake/100 tri | 55.9 µs | |
| Navmesh bake/10K tri | 473 ms | **Must be async** |
| Throughput/100 tri | 142K QPS | |
| Sliver triangles/100 | 99-104 ps/tri | Sub-nanosecond degenerate handling |
| 50 disconnected islands | 3.7-4.1 µs | Fast failure |
| Snake maze/50 turns | 101-108 µs | |

37-54% improvement over previous baselines. Pathfinding scales sub-linearly.

### 5.6 astraweave-terrain

| Benchmark | Time | Notes |
|-----------|------|-------|
| World chunk generation | 48.6 ms | Async required |
| Climate sampling | 782 ns | |
| Chunk/under 60 FPS budget | 15.06 ms | |

### 5.7 astraweave-pcg

| Benchmark | Time | Notes |
|-----------|------|-------|
| Room overlap check | Sub-600 ps | v5.47: 20-37% faster |
| Huge dungeon/100 rooms | 277.5 µs | |
| Encounter generation | Sub-µs | |

### 5.8 astraweave-fluids

| Benchmark | Time | Notes |
|-----------|------|-------|
| GPU position buffer/10K | 0.9-1.15 ns | Sub-nanosecond |
| Multi-step sim/500x10 | 450-500 µs | 45-57% faster than baseline |
| Adaptive timestep/1K | 3.3-4.0 µs | |
| SPH kernels | 28-39% improved | poly6/spiky/viscosity |
| Spatial hash rebuild | 38-62% improved | |
| Single step/1K particles | 1.8-3.0 ms | 12% frame budget |
| Particle throughput | 100-322 Melem/s | |

---

## 6. Infrastructure & Networking

### 6.1 astraweave-sdk

| Benchmark | Time | Notes |
|-----------|------|-------|
| FFI handle creation/10K | 0.43-0.49 ns/handle | |
| Vec3 round-trip throughput | 28-39 Melem/s | |
| Transform round-trip/1K | 1.2-1.3 Gelem/s | Giga-elements/sec |
| Callback invocation/1K | 131-135 Melem/s | |
| Vec3 length/10K | 904-924 Melem/s | |
| Error propagation chain | 336-344 ns | Sub-µs |
| CString marshal/1K | 113-144 µs | |
| JSON world lifecycle | Sub-µs | |

### 6.2 astraweave-net-ecs

| Benchmark | Time | Notes |
|-----------|------|-------|
| Entity serialize/10K | 470-550 ns/entity | |
| Interest management/1K entities/16 clients | 23-25 µs/client | |
| Delta compression/1K | 9.7-12 µs | |
| Transform interpolation/100 | 47-54 ns/transform | |
| Packet validation | 1.2-1.4 µs | |

**60 FPS impact**: 16-client multiplayer = ~0.4 ms (2.4% frame budget).

### 6.3 astraweave-persistence-ecs

| Benchmark | Time | Notes |
|-----------|------|-------|
| Entity serialize/10K | 0.96-1.17 µs/entity | |
| Full snapshot/10K | 4.7-5.9 ms | 59 ns/entity |
| Incremental delta/1% change | 3.3-3.5 µs | |
| Compression/1MB | 330-357 MiB/s | |
| Decompression/1MB | 476-526 MiB/s | |
| Component deserialize (transform) | 24-27 ns | Sub-30ns |
| Checksum verify/100KB | ~40 GB/s | |

### 6.4 aw-save

| Benchmark | Time | Notes |
|-----------|------|-------|
| CRC32 throughput | 13 GB/s | |
| LZ4 compress/decompress | Sub-ms typical | |
| Save operation | 19.31 ms | |
| Load operation | 376.63 µs | |
| Postcard serialize | 30.17 ns | |

### 6.5 astraweave-ipc

| Benchmark | Time | Notes |
|-----------|------|-------|
| Message type detection/1K | 7.2-12 ns/message | Sub-12ns |
| Binary deserialize/1K | 28-38 ns/entity | |
| Position quantization | 76-104 ns | |
| Rate limiting | 120-180 ns/check | Flow control free |

### 6.6 astraweave-scripting

| Benchmark | Time | Notes |
|-----------|------|-------|
| Empty script compile | 102-119 ns | |
| Complex script compile | 3.5-3.8 µs | |
| Security limit checks | 13-17 ns | Security is free |
| Command parsing | 57-61 ns | |
| Callback/10 | 95-105 ns/callback | |
| Hot reload (full) | 58-67 ms | Async recommended |

### 6.7 astraweave-security

| Benchmark | Time | Notes |
|-----------|------|-------|
| Operation counting/100K | 0.45-0.53 ns/op | Sub-nanosecond (security = zero overhead) |
| Numeric validation/50K | 2.2-2.8 ns/validate | |
| RBAC check/50K | 54-62 ns/check | |
| Movement anti-cheat/20K | 37-45 ns/validate | |
| Injection detection/10K | 350-420 ns/check | |

### 6.8 astraweave-secrets

| Benchmark | Time | Notes |
|-----------|------|-------|
| Encryption scaling (any key size) | 1.15-1.90 ns | O(1) constant time |
| Key generation | 426 ns/key | 2.3M keys/sec |
| Secret retrieval | 620 ns/secret | |
| Audit filtering | 12-19 ns/op | |
| Time range queries | 12 ns/query | |

### 6.9 astraweave-steam

| Benchmark | Time | Notes |
|-----------|------|-------|
| Get user ID | 90-104 ns | Sub-110ns |
| Cloud upload throughput | 116-322 MB/s | |
| Achievement batch/1K | 214-257 ns/unlock | |
| Statistics batch/1K | 168-186 ns/stat | |

---

## 7. Observability & Profiling

### 7.1 astraweave-profiling

| Benchmark | Time | Notes |
|-----------|------|-------|
| Zone creation | 2.1 ns/zone | Profiling essentially free |
| Frame timing | 3.0-3.3 µs | Full start+end |
| Message logging/1K | 33-38 ns/message | Batched |
| Allocation tracking | 260-370 ns/alloc | |
| Lock profiling | 89-97 ns/lock | |

### 7.2 astraweave-observability

| Benchmark | Time | Notes |
|-----------|------|-------|
| Gauge updates | 37-44% improved | |
| Histogram observations | 35-42% improved | |
| Span attributes | 37-46% improved | |
| Correlation ID | 45-68 ns | |
| Frame timing capture | 85-120 ns | Sub-120ns |
| Log filtering/50K | 24-36 ns/filter | |

---

## 8. UI & Editor

### 8.1 astraweave-ui

| Benchmark | Time | Notes |
|-----------|------|-------|
| Settings navigation | 696 ps | Sub-nanosecond |
| State changes/100 | 0.96 ns/change | 1B state changes/sec capacity |
| Quest lookup/100 | 0.98 ns/quest | |
| Arc motion physics | 1.15 ns | |
| Damage spawn burst/50 | 1.26 ns/damage | 789M spawns/sec |
| Health bar update | 2.1 ns | |
| Graphics validation | 2.45 ns | |
| Equality checks/1K | 1.18 ns | O(1) |

**60 FPS impact**: Full UI pipeline <10 µs (0.06% frame budget).

### 8.2 Editor Gizmos

| Benchmark | Time | Notes |
|-----------|------|-------|
| State transitions | 342-536 ps | Sub-nanosecond |
| Scale cube rendering | 96 ns | |
| Projection matrix | 1.83 ns | |

---

## 9. Stress Tests

| Benchmark | Time | Notes |
|-----------|------|-------|
| ECS stress (1K entities) | 508.96 µs | |
| Network stress | 265.57 µs | |
| Persistence stress | 1.25 ms | |
| Bullet hell (10K entities) | 2.03 ms | |

---

## Performance Targets

### P0: Critical (Production)

| System | Target | Current | Status |
|--------|--------|---------|--------|
| ECS world tick | <2 ms | ~0.5 ms | MET |
| AI planning | <5 ms | ~0.3 ms | MET |
| Physics full tick | <3 ms | ~2.0 ms | MET |
| Frame time total | <16.67 ms | ~2.7 ms | MET |

### P1: Important

| System | Target | Current | Status |
|--------|--------|---------|--------|
| LLM avg blocking (Qwen3) | <5,000 ms | 2,745 ms | **MET** |
| LLM TTFC (steady-state) | <500 ms | 160 ms | **MET (exceeds)** |
| LLM avg streaming (Qwen3) | <5,000 ms | 2,707 ms | **MET** |
| LLM warmup (cold→hot) | <10,000 ms | 4,692 ms | **MET** |
| LLM p95 latency | <5,000 ms | ~3,080 ms (est.) | **MET** |
| Terrain chunk gen | <16 ms | 15.06 ms | MET |
| A* pathfinding | <10 ms | 20-54 µs | MET (exceeds) |

---

## AI-Native Validation

| Metric | Target | Achieved |
|--------|--------|----------|
| Agent capacity @ 60 FPS | 700+ | **12,700+** |
| Validation throughput | 400K/sec | **6.48M/sec** |
| Determinism | 100% | **100%** |
| Anti-cheat detection | 100% | **100%** |

28 tests validated: Perception (9), Planner (10), Integration (9) — all passing.

---

## Integration Validation

800+ integration tests across 106 files validate correctness across 10 major integration paths (ECS-AI-Physics-Nav, Combat-Physics-Damage, LLM-Hermes2Pro-Plan, etc.). Unit benchmarks measure performance at appropriate granularity; integration tests validate cross-system correctness.

**Performance SLA tests**: 60 FPS @ 676 agents, 12,700+ agent capacity, 1,000+ simultaneous sounds, 100-frame determinism — all passing.

---

## Key Discoveries & Lessons

1. **Batching > Scattering**: ECS collect/writeback 3-5x faster than scattered `get_mut()`
2. **Parallelize only >5 ms workloads**: Rayon overhead ~50-100 µs
3. **Trust glam auto-vectorization**: 80-85% of hand-written AVX2
4. **Cache locality cascades**: Spatial hash improved ALL systems 9-17%
5. **Sub-nanosecond operations are real**: 20+ benchmarks in 500 ps - 1 ns range (verified by Criterion.rs)
6. **VSM 30x faster than PCF** for shadow sampling
7. **GPU compute mandatory** for MegaLights >100 lights (68x faster than CPU)
8. **SparseSet 37x faster** than BTreeMap for ECS lookups

---

## Known Limitations

- **Rendering**: Frame render time unknown (GPU benchmarks not yet integrated into Criterion)
- **LLM latency**: Blocking 2,745 ms avg (Qwen3-8B, optimized); further reduction requires hardware upgrade or custom Ollama build; TTFC at theoretical floor (160ms, within 6%)
- **Terrain 10K triangles**: 473 ms navmesh bake — must be async/precomputed
- **Graphics examples**: `ui_controls_demo`, `debug_overlay` won't compile (egui/winit version drift)
- **Rhai crates**: `astraweave-author`, `rhai_authoring` have Sync trait errors
- **LLM crates**: `astraweave-llm`, `llm_toolcall` excluded from standard builds
- **criterion 0.7**: `llm-eval` needs version alignment

---

## Week 8 Performance Sprint (Oct 9-12, 2025)

Frame time optimization: **-12.6%** (3.09 ms to 2.70 ms, +47 FPS to 370 FPS).

- Tracy 0.11.1 integrated (zero-overhead instrumentation)
- Spatial hash: 99.96% fewer collision checks, 9-17% cascade improvement
- SIMD movement: 2.08x speedup (BATCH_SIZE=4, glam auto-vectorization)
- 84% headroom vs 60 FPS budget validated

---

## Revision History

| Version | Date | Type | Summary | Impact |
|:-------:|:----:|:----:|:--------|:------:|
| 5.57 | 2026-06-10 | Update | mimalloc/fast-alloc merge recorded (back-fill of 2026-04-17 results): ai.goap.plan −52%, profiling_demo FPS +43%; 3 alloc_measure benches registered; ecs harness figure marked historical (deleted with ParallelSchedule) | Significant |
| 5.56 | 2026-02-28 | Major | Qwen3-8B LLM latency: 4 rounds, 3.60× faster than Hermes, TTFC 160ms | Significant |
| 5.55 | 2026-01-13 | Audit | Engine validation: 5,372 tests, RAG deadlock fixed | Critical |
| 5.54 | 2026-01-13 | Audit | Production audit: 99 bench files, Grade A- (91/100) | Significant |
| 5.53 | 2026-01-12 | Hotfix | ECS FIXED: BlobVec lazy init, 50-68% faster | Critical |
| 5.52 | 2026-01-11 | Regression | ECS REGRESSION: 47-333% slower, A+ to B- | Critical |
| 5.49 | 2026-01-08 | Major | AI breakthrough: Multi-agent 66-68% faster, sub-4ns GOAP | Significant |
| 5.48 | 2026-01-07 | Major | Physics breakthrough: Rigid body 10x faster | Significant |
| 5.42 | 2026-01-01 | Major | SIMD math: Scalar beats SIMD wrappers 6-36x | Significant |
| 5.20 | 2025-12-11 | Minor | Coordination/NPC/Security adversarial benchmarks | Significant |
| 5.15 | 2025-12-06 | Major | SDK/Director/RAG/Scripting/Steam/Profiling adversarial | Significant |
| 5.0 | 2025-12-19 | Major | Professional restructure: 575 to 485 benchmarks (corrected) | Significant |
| 4.0 | 2025-11-13 | Minor | Reality sync: 129 Criterion outputs validated | Incremental |
| 3.0 | 2025-10-31 | Minor | Tier 2 partial: Physics + render | Incremental |
| 2.0 | 2025-10-30 | Major | Tier 1 complete: All priority crates | Significant |
| 1.0 | 2025-10-21 | Major | Initial release: 33+ files consolidated | Significant |

**56 total versions** (2025-10-21 to 2026-06-10). Benchmark count: 155 (v1.0) to 2,830+ (v5.56), +1,725% growth.

<details>
<summary>Critical Version Details</summary>

**v5.53 (2026-01-12) — ECS Regression Fixed**: BlobVec lazy initialization. `Archetype::new()` allocated HashMaps even in legacy Box storage mode. Changed to `Option<HashMap>` with lazy init. Grade: B- back to A+.

**v5.52 (2026-01-11) — ECS Regression Discovered**: Entity operations 47-195% slower, component operations 86-333% slower. Root cause: HashMap allocation in `Archetype::new()`.

**v5.49 (2026-01-08) — AI Breakthrough**: GOAP next_action 3.46-3.56ns (sub-4ns). Multi-agent 66-68% faster. Capacity: ~186,000 agents @ 10% budget.

</details>

---

**Next Review**: 2026-07-10 (monthly cadence)
