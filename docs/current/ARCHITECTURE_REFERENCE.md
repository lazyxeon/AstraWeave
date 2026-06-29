# AstraWeave Architecture Reference

> **Extended reference** for code patterns, API deep dives, performance data, and testing patterns.
> The condensed versions live in `.github/copilot-instructions.md`. Read this file when you need full detail.

---

## Table of Contents

1. [AI-First Architecture](#ai-first-architecture)
2. [ECS System Stages](#ecs-system-stages)
3. [Rendering & Materials](#rendering--materials)
4. [Performance Optimization](#performance-optimization)
5. [API Deep Dives](#api-deep-dives)
6. [Testing Patterns](#testing-patterns)
7. [Workspace Structure (Detailed)](#workspace-structure-detailed)
8. [Blend Import Pipeline](#blend-import-pipeline)
9. [Blueprint Zone System](#blueprint-zone-system)
10. [Performance Baselines](#performance-baselines)
11. [Formal Verification (Miri & Kani)](#formal-verification-miri--kani)

---

## AI-First Architecture

### Core Loop

```
Perception → Reasoning → Planning → Action
    ↓           ↓            ↓          ↓
WorldSnapshot  AI Model   PlanIntent  Tool Validation
```

**Key Concepts**:

- `WorldSnapshot`: Filtered world state for AI perception (see `astraweave-ai/src/core_loop.rs`)
- `PlanIntent` + `ActionStep`: AI decisions as validated action sequences
- `Orchestrator` trait: Abstracts AI planning (rule-based vs LLM)
- **Tool Sandbox**: All AI actions validated by engine (no cheating possible)

### GOAP+LLM Hybrid Arbiter (Full Patterns)

```rust
// See astraweave-ai/src/arbiter.rs
use astraweave_ai::arbiter::{AIArbiter, AIControlMode};
use astraweave_ai::llm_executor::LlmExecutor;

// Pattern 1: Basic agent with arbiter
struct Agent {
    arbiter: AIArbiter,
    llm_executor: Arc<LlmExecutor>,
}

impl Agent {
    fn new(llm_executor: Arc<LlmExecutor>) -> Self {
        Self {
            arbiter: AIArbiter::new(llm_executor.clone()),
            llm_executor,
        }
    }

    fn update(&mut self, world: &mut World, snap: WorldSnapshot) -> Result<PlanIntent> {
        self.arbiter.update(world, &snap)?;
        match self.arbiter.mode() {
            AIControlMode::GOAP => {
                let plan = goap_orchestrator.plan(world, &snap)?;
                Ok(plan)
            }
            AIControlMode::ExecutingLLM { step_index } => {
                let llm_plan = self.arbiter.current_llm_plan().unwrap();
                Ok(execute_step(&llm_plan, step_index))
            }
            AIControlMode::BehaviorTree => {
                let plan = bt_orchestrator.plan(world, &snap)?;
                Ok(plan)
            }
        }
    }
}

// Pattern 2: Shared LLM executor (efficient for many agents)
let llm_executor = Arc::new(LlmExecutor::new(
    qwen_client,  // Qwen via Ollama
    tool_registry,
));
let agents: Vec<Agent> = (0..100)
    .map(|_| Agent::new(llm_executor.clone()))
    .collect();

// Pattern 3: Custom cooldown (adjust LLM request frequency)
let arbiter = AIArbiter::new(llm_executor)
    .with_llm_cooldown(Duration::from_secs(5));   // Aggressive (high LLM usage)
    // .with_llm_cooldown(Duration::from_secs(30)); // Passive
    // .with_llm_cooldown(Duration::ZERO);           // Immediate (testing only)

// Pattern 4: Metrics monitoring
let metrics = arbiter.metrics();
let success_rate = metrics.llm_successes as f32
    / (metrics.llm_successes + metrics.llm_failures) as f32;

// Pattern 5: Manual mode transitions (advanced usage)
if emergency_situation {
    arbiter.transition_to_goap();
}
```

**Performance Characteristics**:
- **GOAP Control**: 101.7 ns per update (982× faster than target)
- **LLM Polling**: 575.3 ns per update
- **Mode Transitions**: 221.9 ns
- **Full Cycle**: 313.7 ns (GOAP update + LLM poll + metrics)
- **Scalability**: 1,000 agents @ 60 FPS = 0.6% frame budget, 10,000 agents = 6.1%

### Testing Patterns for Arbiter

```rust
use astraweave_ai::test_utils::MockLlmOrch;

#[tokio::test]
async fn test_arbiter_with_mock() {
    let mock_llm = Arc::new(MockLlmOrch::new_with_delay(
        Duration::from_millis(100),
        Some(mock_plan()),
    ));
    let llm_executor = Arc::new(LlmExecutor::new(mock_llm, tool_registry));
    let mut arbiter = AIArbiter::new(llm_executor);

    arbiter.update(&world, &snap)?;
    tokio::time::sleep(Duration::from_millis(150)).await;
    arbiter.update(&world, &snap)?;

    assert!(matches!(arbiter.mode(), AIControlMode::ExecutingLLM { .. }));
}

// Benchmarking with criterion
fn bench_arbiter_goap_control(c: &mut Criterion) {
    let arbiter = setup_arbiter();
    c.bench_function("arbiter_goap_control", |b| {
        b.iter(|| arbiter.update(black_box(&mut world), black_box(&snap)))
    });
}
```

📚 **Docs**: `docs/archive/completion_reports/ARBITER_IMPLEMENTATION.md`, `ARBITER_QUICK_REFERENCE.md`

---

## ECS System Stages

Deterministic, ordered execution at **fixed 60Hz tick** with deterministic RNG:

| Order | Stage | Purpose |
|-------|-------|---------|
| 1 | `PRE_SIMULATION` | Setup, initialization |
| 2 | `PERCEPTION` | Build WorldSnapshots, update AI sensors |
| 3 | `SIMULATION` | Game logic, cooldowns, state updates |
| 4 | `AI_PLANNING` | Generate PlanIntents from orchestrators |
| 5 | `PHYSICS` | Apply forces, resolve collisions |
| 6 | `POST_SIMULATION` | Cleanup, constraint resolution |
| 7 | `PRESENTATION` | Rendering, audio, UI updates |

---

## Rendering & Materials

- **wgpu 25.0.2** backend (Vulkan/DX12/Metal)
- **Material System**: TOML → GPU D2 array textures with stable indices
  - Pattern: `assets/materials/<biome>/{materials.toml, arrays.toml}`
  - WGSL bindings (group=1): albedo (0), sampler (1), normal (2), linear sampler (3), MRA (4)
- **Shared Utilities**: `MaterialManager`, `IblManager`, `MeshRegistry`
- **Feature Flags**: `textures`, `assets` gate loaders
- **GPU Skinning**: pipeline with dual bone influence
  - `astraweave-render/src/skinning_gpu.rs`
- **GPU Mesh Optimization**:
  - `vertex_compression.rs` (octahedral normals, half-float UVs, 37.5% memory reduction)
  - `lod_generator.rs` (quadric error metrics, 3-5 LOD levels)
  - `instancing.rs` (GPU batching, 10-100× draw call reduction)

---

## Performance Optimization

### Tracy Profiling (Week 8)

Tracy 0.11.1 integrated for zero-overhead profiling. See `examples/profiling_demo/`.

### Spatial Hash Collision

O(n log n) grid-based spatial partitioning in `astraweave-physics/src/spatial_hash.rs` (1,038 LoC), benchmarked at 99.96% collision check reduction with cache-locality cascade benefits. Dormant: the physics broadphase actually runs Rapier's `DefaultBroadPhase`; this `SpatialHash` has no production caller in the physics step (used only by terrain LOD blending, the profiling example, and tests).

### SIMD Movement

Batch processing for 2.08× speedup in `astraweave-math/src/simd_movement.rs`. `BATCH_SIZE=4` loop unrolling, glam auto-vectorization. ECS batching pattern: `collect() → SIMD → writeback` (3-5× faster than scattered `get_mut()`).

### Key Lessons Learned

1. **Amdahl's Law**: Only 0.15-22.4% parallelizable work → max 1.24× speedup (59% ECS overhead is sequential)
2. **Batching > Scattering**: ECS collect/writeback 3-5× faster than scattered `get_mut()`
3. **Overhead Threshold**: Only parallelize >5 ms workloads (Rayon overhead ~50-100 µs)
4. **SIMD Auto-Vec**: glam achieves 80-85% of hand-written AVX2, trust auto-vectorization
5. **Cache Locality Cascades**: Spatial hash improved ALL systems 9-17%, not just collision
6. **API Verification First**: Always read actual struct definitions before generating code
7. **Comprehensive Fixing**: Creating one corrected file vs piecemeal edits is more reliable
8. **Case Sensitivity Matters**: snake_case vs PascalCase mismatch caused 100% false positives
9. **Debug Early**: One debug logging statement revealed critical validation bug
10. **Production First**: Focus on working demo over 100% test coverage

---

## API Deep Dives

### WorldSnapshot API

```rust
// CORRECT API (from astraweave-core/src/schema.rs):
pub struct WorldSnapshot {
    pub t: f32,
    pub player: PlayerState,
    pub me: CompanionState,
    pub enemies: Vec<EnemyState>,
    pub pois: Vec<Poi>,
    pub obstacles: Vec<IVec2>,
    pub objective: Option<String>,
}

pub struct CompanionState {
    pub ammo: i32,
    pub cooldowns: BTreeMap<String, f32>,
    pub morale: f32,
    pub pos: IVec2,
}

pub struct PlanIntent {
    pub plan_id: String,
    pub steps: Vec<ActionStep>,
}

// Usage:
let enemy_pos = snap.enemies[0].pos;           // ✅
let my_pos = snap.me.pos;                      // ✅
let my_ammo = snap.me.ammo;                    // ✅
let cooldown = snap.me.cooldowns.get("attack"); // ✅
let poi = snap.pois.first().map(|p| p.pos);    // ✅
```

### BehaviorGraph API

```rust
use astraweave_behavior::{BehaviorGraph, BehaviorNode, BehaviorContext, BehaviorStatus};

let combat_seq = BehaviorNode::Sequence(vec![
    BehaviorNode::Condition("check_threat".into()),
    BehaviorNode::Action("throw_smoke".into()),
]);
let move_seq = BehaviorNode::Sequence(vec![
    BehaviorNode::Action("move_to_objective".into()),
]);
let root = BehaviorNode::Selector(vec![combat_seq, move_seq]);
let graph = BehaviorGraph::new(root);

let context = BehaviorContext::new(snap);
let status = graph.tick(&context);
```

### hello_companion Performance (Phase 6)

| Mode | Latency |
|------|---------|
| Classical | 0.20 ms |
| BehaviorTree | 0.17 ms |
| Utility | 0.46 ms |
| LLM (Qwen) | 3,462 ms |
| Hybrid | 2,155 ms |
| Ensemble | 2,355 ms |

---

## Workspace Structure (Detailed)

### Core Engine Crates

| Crate | Purpose |
|-------|---------|
| `astraweave-ecs` | Archetype-based ECS, system stages, events |
| `astraweave-ai` | AI orchestrator, core loop, tool sandbox |
| `astraweave-sdk` | C ABI, header generation (SDK exports) |
| `astraweave-render` | wgpu 25 renderer, materials, IBL, GPU skinning, mesh optimization |
| `astraweave-physics` | Rapier3D wrapper, character controller, spatial hash |
| `astraweave-gameplay` | Combat physics, attack sweep |
| `astraweave-nav` | Navmesh, A*, portal graphs |
| `astraweave-audio` | Spatial audio, rodio backend |
| `astraweave-scene` | World partition, async cell streaming |
| `astraweave-terrain` | Voxel/polygon hybrid, marching cubes, BiomePack import bridge |
| `astraweave-cinematics` | Timeline, sequencer, camera/audio/FX tracks |
| `astraweave-math` | SIMD vector/matrix ops (glam-based), movement optimization |

### Gameplay & Tools

| Crate | Purpose |
|-------|---------|
| `astraweave-behavior` | Behavior trees, utility AI |
| `astraweave-weaving` | Fate-weaving system (Veilweaver game mechanic) |
| `astraweave-pcg` | Procedural content generation |
| `tools/aw_editor` | Level/encounter editor (GUI) |
| `tools/aw_asset_cli` | Asset pipeline tooling |
| `crates/astraweave-blend` | Blender .blend import: scene decomposition, texture processing |

### Examples Status

- ✅ Working: `hello_companion`, `unified_showcase`, `core_loop_bt_demo`, `core_loop_goap_demo`, `weaving_pcg_demo`, `profiling_demo`
- ⚠️ API Drift: `ui_controls_demo`, `debug_overlay` (egui/winit version mismatches)
- ❌ Broken: `astraweave-author`, `rhai_authoring` (rhai sync trait issues)

---

## Blend Import Pipeline

Full pipeline for importing Blender `.blend` scene files as biome vegetation/scatter profiles.

### Pipeline Flow

```
.blend file → Scene Decomposition → manifest.json → BiomePack → BiomeConfig + ScatterConfig
    ↓                ↓                    ↓              ↓              ↓
astraweave-blend   Python subprocess   JSON schema   Bridge format   Terrain system
```

### Crate Responsibilities

| Crate | Role |
|-------|------|
| `astraweave-blend` | Scene decomposition (Python subprocess), texture processing (HDR→PNG, thumbnails) |
| `astraweave-asset` | `BlendImportSystem` + optional `blend` feature flag |
| `astraweave-terrain` | `BiomePack` bridge format, `BiomeConfig`/`ScatterConfig` conversion |
| `aw_editor` | `BlendImportPanel` UI — file selection, asset review, biome pack generation |

### BiomePack API

```rust
use astraweave_terrain::{BiomePack, biome::BiomeType, scatter::ScatterConfig};

// Parse manifest.json from decomposition output
let pack = BiomePack::from_manifest(&manifest_path)?;

// Convert to terrain system configs
let biome_config = pack.to_biome_config(BiomeType::Desert);
let scatter_config = pack.to_scatter_config();

// Save/load for editor workflows
pack.save(&path)?;
let loaded = BiomePack::load(&path)?;
```

### Asset Classification & Scatter Weights

Assets are classified by category (`vegetation`, `rock`, `terrain`, `prop`, `billboard`) and sized by dimensions:

| Category | Size | Weight | Scatter Behavior |
|----------|------|--------|------------------|
| vegetation/Large | >5m | 0.05 | Sparse (trees) |
| vegetation/Medium | 1-5m | 0.30 | Moderate (bushes) |
| vegetation/Small | <1m | 2.00 | Dense (flowers/grass) |
| rock/Large | >5m | 0.02 | Very rare (cliffs) |
| rock/Medium | 1-5m | 0.15 | Moderate (boulders) |
| rock/Small | <1m | 0.50 | Common (stones) |
| terrain | any | 0.00 | Never scattered |

### Editor Integration

`BlendImportPanel` (panel type: `BlendImport`, category: Content) provides:
- File browser with `.blend` file selection
- Decomposition progress tracking (5 phases)
- Asset list with category filtering and toggle selection
- Texture processing settings (HDR conversion, thumbnails, resolution limits)
- Scatter/biome settings (Poisson disk, density, slope, biome type selection)
- BiomePack generation with name/description

### Test Coverage

| Test Suite | Count | Status |
|------------|-------|--------|
| `astraweave-blend` unit tests | 63 | ✅ |
| `astraweave-terrain::biome_pack` unit tests | 9 | ✅ |
| `astraweave-terrain::blend_pipeline_e2e` integration tests | 12 | ✅ |
| `aw_editor::blend_import_panel` unit tests | 13 | ✅ |

---

## Blueprint Zone System

Polygon-based zone editor with zone-scoped vegetation generation, heightmap injection, and 3D viewport overlay. Extends the Blend Import Pipeline with spatial placement control.

### Pipeline Flow

```
Editor Canvas → BlueprintZone → ZoneScatterGenerator → ZoneGenerationResult
     ↓               ↓                ↓                      ↓
Polygon drawing   ZoneRegistry    Replica/Inspired      placements + patches
                  (save/load)      mode dispatch         ↓
                                                    apply_heightmap_patches()
                                                         ↓
                                                    TerrainChunk updates
```

### Crate Responsibilities

| Crate | Role |
|-------|------|
| `astraweave-terrain` | `BlueprintZone`, `ZoneRegistry`, `ZoneScatterGenerator`, heightmap patching, `AdaptiveScaleParams` |
| `astraweave-blend` | `heightmap_raster` — rasterizes terrain meshes → heightmaps + fixed placements |
| `aw_editor` | `BlueprintPanel` (2D canvas editor), `BlueprintOverlay` (3D viewport), `BlendAssetScanner`, system wiring |

### Zone Data Model

```rust
use astraweave_terrain::blueprint_zone::*;

let zone = BlueprintZone {
    id: ZoneId(1),
    name: "Forest Clearing".into(),
    vertices: vec![[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]],
    source: ZoneSource::BlendScene {
        pack_path: "assets/pine_forest.biomepack".into(),
        placement_mode: PlacementMode::Replica,
    },
    priority: 0,
    enabled: true,
};

// ZoneRegistry — CRUD, spatial queries, persistence
let mut registry = ZoneRegistry::new();
registry.add_zone(zone);
let zones = registry.zones_containing_point(50.0, 50.0);
registry.save(&Path::new("zones.json"))?;
```

### Placement Modes

| Mode | Behavior |
|------|----------|
| `Replica` | 1:1 reproduction — fixed positions from `.blend` scene, scaled by `AdaptiveScaleParams` |
| `Inspired` | Procedural scatter using `ScatterConfig` derived from `BiomePack`, respects zone polygon |
| `BiomePreset` | Pure biome-driven scatter (Grassland, Forest, Desert, etc.) without `.blend` data |

### ZoneScatterGenerator API

```rust
use astraweave_terrain::zone_scatter::*;

let gen = ZoneScatterGenerator::new(256.0, 128); // chunk_size, heightmap_resolution
let result: ZoneGenerationResult = gen.generate_zone_scatter(&zone, &biome_pack)?;

// Result contains:
// - result.placements: Vec<VegetationInstance>  (position, rotation, scale, model_path)
// - result.heightmap_patches: Vec<HeightmapPatch> (per-chunk height modifications)

// Apply patches to terrain chunks
let results = vec![result];
apply_heightmap_patches(&mut chunk_map, &results);
```

### Adaptive Scaling

When zone area differs from source scene footprint, `AdaptiveScaleParams` adjusts density and scale:

```rust
let params = AdaptiveScaleParams::compute(reference_area, zone_area);
// params.density_multiplier = sqrt(zone_area / reference_area)
// params.scale_multiplier  = (zone_area / reference_area)^0.25
// params.position_scale    = sqrt(zone_area / reference_area)
```

### Boundary Blending

`apply_boundary_blending()` uses smoothstep falloff at zone edges to prevent hard cutoffs:
- Vegetation near edges: density fades via `BlendMask::sample(x, z)` → 0.0–1.0
- Heightmap patches: height delta scaled by mask value at each sample point
- Editor: `BrushMode::ZoneBlend` for manual blend-weight painting

### Heightmap Rasterization (Blend Crate)

```rust
use astraweave_blend::heightmap_raster::*;

// Rasterize terrain meshes from .blend decomposition output
let heightmap = rasterize_terrain_meshes(&terrain_meshes, resolution)?;
let height = heightmap.sample_bilinear(u, v); // Normalized [0,1] coords
let area = heightmap.footprint_area();         // World-space area in m²
```

Ray-triangle intersection with seam averaging and hole filling for multi-tile terrains.

### Editor Integration

`BlueprintPanel` (panel type: `Blueprint`, category: Content) provides:
- 2D canvas with pan/zoom for polygon drawing
- Tools: Select, DrawPolygon, MoveVertex, DeleteZone
- Zone inspector with name, source (biome preset or blend scene), placement mode
- Undo/redo via `BlueprintCommand` stack
- Save/Load zones as `.zones.json`

`BlueprintOverlay` projects zone polygons into the 3D viewport as `DebugLine` wireframes,
integrated alongside component gizmos and brush cursors in the physics renderer pass.

### System Wiring (main.rs)

```
Update loop → process_blueprint_actions()
  ├── GenerateZone  → handle_generate_zone()  → ZoneScatterGenerator
  ├── GenerateAll   → handle_generate_zone() for each zone
  ├── ClearGeneration → clear generation results
  ├── SaveZones     → handle_save_zones()    → ZoneRegistry::save()
  └── LoadZones     → handle_load_zones()    → ZoneRegistry::load() + panel sync
       └── sync_zone_overlay() → BlueprintOverlay::generate_lines() → renderer
```

### Test Coverage

| Test Suite | Count | Status |
|------------|-------|--------|
| `astraweave-terrain::blueprint_zone` unit tests | 24 | ✅ |
| `astraweave-terrain::zone_scatter` unit tests | 16 | ✅ |
| `astraweave-terrain::zone_scatter_e2e` integration tests | 11 | ✅ |
| `astraweave-blend::heightmap_raster` unit tests | 11 | ✅ |
| `astraweave-blend::heightmap_raster_e2e` integration tests | 10 | ✅ |
| `astraweave-terrain::biome_pack` unit tests (extended) | 9 | ✅ |
| `aw_editor::blueprint_panel` unit tests | 17 | ✅ |
| `aw_editor::blueprint_overlay` unit tests | 7 | ✅ |
| `aw_editor::blend_scanner` unit tests | 8 | ✅ |

---

## Performance Baselines

| System | Metric | Notes |
|--------|--------|-------|
| ECS | 25.8 ns world creation, 420 ns/entity spawn, <1 ns/entity tick | |
| AI Core Loop | 184 ns – 2.10 µs | 2500× faster than 5 ms target |
| GOAP | 1.01 µs cache hit (97.9% faster), 47.2 µs cache miss | |
| Behavior Trees | 57–253 ns | 66,000 agents @ 60 FPS possible |
| Terrain | 15.06 ms world chunk | 60 FPS budget achieved |
| Input | 4.67 ns binding creation | |
| Physics | 114 ns char move, 6.52 µs full tick, 2.97 µs rigid body step | |
| GPU Mesh | 21 ns vertex compression, 37.5% memory reduction | |
| SIMD Math | 2.08× speedup (20.588 µs → 9.879 µs @ 10k entities) | |
| Frame Time | 0.965 ms @ 1,000 entities (~1,036 FPS) System alloc; 0.709 ms (~1,410 FPS) mimalloc | 2.70 ms was the Week-8 target |
| AI-Native | 12,700+ agents @ 60 FPS, 6.48M checks/sec | 100% determinism |

**See also**: `docs/current/MASTER_BENCHMARK_REPORT.md`

---

## Formal Verification (Miri & Kani)

### Miri UB Detection

**Status**: All unsafe code validated — 1,059 tests, **ZERO undefined behavior**.

**Validated crates** (4):
- `astraweave-ecs` (419 tests): BlobVec, SparseSet, EntityAllocator, SystemParam
- `astraweave-math` (109 tests): SIMD vec/mat/quat, SSE2 scalar fallback
- `astraweave-core` (503 tests): Entity::from_raw, capture/replay
- `astraweave-sdk` (28 tests): C ABI FFI, raw pointer handling

**CI**: `.github/workflows/miri.yml` (weekly, nightly toolchain)

**Running locally**:
```bash
cargo +nightly miri test -p astraweave-ecs --lib -- --test-threads=1
# Flags: -Zmiri-symbolic-alignment-check -Zmiri-strict-provenance
```

**Report**: `docs/current/MIRI_VALIDATION_REPORT.md`

### Kani Formal Verification

**Purpose**: Model-checks critical properties for ALL possible inputs (unlike tests which check specific inputs).

**Validated crates** (3): `astraweave-ecs`, `astraweave-math`, `astraweave-sdk`

**Proof locations**:
- `astraweave-ecs/tests/mutation_resistant_comprehensive_tests.rs` — BlobVec, Entity bit encoding, EntityAllocator proof mirrors
- `astraweave-sdk/src/lib_kani.rs` — C ABI function proofs

**CI**: `.github/workflows/kani.yml` (uses `model-checking/kani-github-action@v1`)

**Running locally**:
```bash
cargo kani --package astraweave-sdk
cargo kani --package astraweave-ecs
```

### Unsafe Code Requirements

Any new `unsafe` code MUST:
1. Pass Miri (`cargo +nightly miri test -p <crate>`)
2. Have a corresponding Kani proof or Kani-mirror test
3. Include a `// SAFETY:` comment explaining the invariant
4. Be validated in CI before merge

---

## Key Files to Check

| Need | File(s) |
|------|---------|
| Public APIs | Each crate's `src/lib.rs` |
| Workspace deps | Root `Cargo.toml` |
| Build config | `.cargo/config.toml` (aliases, profiles, sccache) |
| CI tasks | `.vscode/tasks.json` |
| Miri validation | `docs/current/MIRI_VALIDATION_REPORT.md` |
| Kani proofs | `astraweave-sdk/src/lib_kani.rs`, `astraweave-ecs/tests/mutation_resistant_comprehensive_tests.rs` |

---

*Last updated: February 8, 2026*
