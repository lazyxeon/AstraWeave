# Mutation Testing Wave 2 — Visual Crate Campaign Plan

**Version**: 1.1  
**Date**: February 20, 2026  
**Status**: IN PROGRESS — TERRAIN SWEEP RUNNING  
**Crates**: `astraweave-terrain`, `astraweave-render`, `aw_editor`

---

## 1. Executive Summary

Wave 2 extends the mission-critical mutation testing campaign to **3 visual-heavy P0 crates** totaling **34,519 mutants** — 6× the size of the physics crate alone. Pilot shards reveal raw kill rates ranging from **0% (binary entrypoints) to 57.5% (well-tested modules)**, with significant opportunities for classification-based exclusions that should bring adjusted rates into the 80-90%+ range.

### Key Challenges

| Challenge | Impact | Mitigation |
|-----------|--------|------------|
| **34,519 total mutants** (6× physics) | ~330+ hours of compute | Tiered priority, classification-first approach |
| **marching_cubes_tables.rs**: 1,636 lookup-table mutants | 35% of terrain pool | Classify as low-observability constants |
| **main.rs + bin/**: 624 binary entrypoint mutants | 0% kill rate (no lib tests) | Classify as integration-level |
| **tab_viewer.rs**: 2,658 mutants in single file | 10.4% of editor pool | Assess coverage, batch remediation |
| **Feature-gated render code**: ~481 mutants (11.1%) | Not tested under default features | Document, or test with `--all-features` |
| **GPU/wgpu-dependent code** | Inherently untestable in CI | Classify as integration-level |

### Aggregate Metrics

| Crate | Source Files | Lines | Tests | Mutants | Pilot Kill Rate | Est. Compute |
|-------|:-----------:|:-----:|:-----:|:-------:|:--------------:|:------------:|
| astraweave-terrain | 27 | 14,202 | 468 | 4,629 | 29.9% (S0) | ~14h |
| astraweave-render | 65 | 37,932 | 806 | 4,317 | 55.6% (S0) | ~80h |
| aw_editor | 115 | 114,949 | 3,592 | 25,573 | 0-57.5% | ~200h+ |
| **Totals** | **207** | **167,083** | **4,866** | **34,519** | — | **~300h** |

---

## 2. Research Findings

### 2.1 astraweave-terrain (4,629 mutants, 22 shards)

**Test Baseline**: 468 passing, 3 ignored  
**Build Time**: ~9s | **Test Time**: ~17s | **Per-Mutant**: ~26s

**Mutant Distribution (Top 10)**:

| File | Mutants | % Pool | Notes |
|------|:-------:|:------:|-------|
| marching_cubes_tables.rs | 1,636 | 35.3% | Const lookup arrays (hex/i8 values) |
| advanced_erosion.rs | 488 | 10.5% | Hydraulic/thermal erosion algorithms |
| heightmap.rs | 290 | 6.3% | Height generation |
| climate.rs | 202 | 4.4% | Climate zone calculations |
| texture_splatting.rs | 152 | 3.3% | Texture blending weights |
| voxel_data.rs | 147 | 3.2% | Voxel storage/operations |
| meshing.rs | 144 | 3.1% | Mesh generation |
| biome_blending.rs | 140 | 3.0% | Biome transition logic |
| background_loader.rs | 135 | 2.9% | Async chunk loading |
| chunk.rs | 131 | 2.8% | Chunk management |

**Pilot Shard S0/22** (211 mutants tested, 39 min):

| Outcome | Count | Details |
|---------|:-----:|---------|
| Caught | 63 | advanced_erosion.rs, lib.rs |
| Missed | 140 | advanced_erosion.rs (119), lib.rs (21) |
| Unviable | 8 | Compilation failures |
| **Raw Kill Rate** | **29.9%** | Low due to advanced_erosion.rs dominance |

**Key Insight**: `marching_cubes_tables.rs` contains 1,636 mutants in const lookup tables. Each mutant changes one hex constant in a marching cubes edge/tri table — these are effectively constant mutations that require running the full marching cubes pipeline to detect. Many should be classified as **low-observability** (individual table entry changes may produce valid-but-different geometry).

### 2.2 astraweave-render (4,317 mutants, 20 shards)

**Test Baseline**: 806 passing, 0 ignored  
**Build Time**: ~11s | **Per-Mutant**: ~66s

**Feature Gate Analysis**: ~481 mutants (11.1%) behind non-default feature gates:

| Feature | Files | Mutants | Notes |
|---------|-------|:-------:|-------|
| nanite | nanite_gpu_culling, nanite_visibility, nanite_render | 161 | GPU mesh LOD |
| ibl | ibl.rs | 66 | Image-based lighting |
| ssao | ssao.rs | 68 | Screen-space AO |
| decals | decals.rs | 34 | Decal rendering |
| advanced-post | advanced_post.rs | 41 | Post-processing |
| msaa | msaa.rs | 29 | Multi-sample AA |
| gpu-particles | gpu_particles.rs | 5 | Particle rendering |
| deferred | deferred.rs | 10 | Deferred shading |
| skinning-gpu | skinning_gpu.rs | 20 | GPU skinning |
| gltf-assets | mesh_gltf.rs | 5 | glTF loading |
| obj-assets | mesh_obj.rs | 36 | OBJ loading |
| megalights | clustered_megalights.rs | 6 | Many-light clustering |

**Mutant Distribution (Top 10)**:

| File | Mutants | % Pool | Notes |
|------|:-------:|:------:|-------|
| renderer.rs | 536 | 12.4% | Core pipeline (44 `#[cfg]` attrs) |
| environment.rs | 401 | 9.3% | Environment maps, skybox |
| clustered.rs | 251 | 5.8% | Clustered forward lighting |
| material_loader.rs | 234 | 5.4% | Material parsing |
| lod_generator.rs | 168 | 3.9% | LOD mesh generation |
| weather_system.rs | 162 | 3.8% | Weather effects |
| camera.rs | 155 | 3.6% | Camera matrices/projection |
| effects.rs | 153 | 3.5% | Visual effects |
| culling.rs | 142 | 3.3% | Frustum/occlusion culling |
| clustered_forward.rs | 134 | 3.1% | Forward+ rendering (22 cfg attrs) |

**Pilot Shard S0/20** (216 mutants tested, 4h):

| Outcome | Count | Source Files |
|---------|:-----:|-------------|
| Caught | 120 | camera.rs (71), clustered.rs (49) |
| Missed | 95 | camera.rs (83), clustered.rs (12) |
| Unviable | 1 | — |
| **Raw Kill Rate** | **55.6%** | camera.rs most polarized (71 caught / 83 missed) |

### 2.3 aw_editor (25,573 mutants, 121 shards)

**Test Baseline**: 3,592 passing, 5 ignored  
**Location**: `tools/aw_editor/`

**Test Distribution by Subdirectory**:

| Directory | Tests | Notes |
|-----------|:-----:|-------|
| panels/ (41 files) | 2,521 | Heaviest testing — UI panels |
| gizmo/ | 129 | 3D manipulation tools |
| top-level .rs | 711 | Core editor logic |
| ui/ | 97 | UI utilities |
| behavior_graph/ | 74 | BT visual editor |
| viewport/ | 32 | Viewport rendering |
| bin/ | 0 | Binary entrypoints (untestable via `--lib`) |

**Mutant Distribution (Top 15)**:

| File | Mutants | % Pool | Tests in File |
|------|:-------:|:------:|:------------:|
| tab_viewer.rs | 2,658 | 10.4% | ~6,383 lines |
| dialogue_editor_panel.rs | 1,102 | 4.3% | 100 |
| viewport/widget.rs | 790 | 3.1% | 32 (dir total) |
| particle_system_panel.rs | 690 | 2.7% | — |
| main.rs | 600 | 2.3% | 0 (binary) |
| animation_panel.rs | 558 | 2.2% | 109 |
| performance_panel.rs | 550 | 2.2% | — |
| cinematics_panel.rs | 514 | 2.0% | — |
| animation.rs | 484 | 1.9% | — |
| lighting_panel.rs | 478 | 1.9% | 117 |
| profiler_panel.rs | 464 | 1.8% | — |
| ui_editor_panel.rs | 456 | 1.8% | — |
| runtime.rs | 448 | 1.8% | 55 |
| entity_manager.rs | 444 | 1.7% | 64 |
| input_bindings_panel.rs | 426 | 1.7% | — |

**Pilot Results**:

| Shard | Mutants | Caught | Missed | Unviable | Kill Rate | Files Hit |
|:-----:|:-------:|:------:|:------:|:--------:|:---------:|-----------|
| S0/121 | 212 | 0 | 212 | 0 | **0.0%** | main.rs (188), bin/aw_game_runtime.rs (24) |
| S4/121 | 212 | 122 | 84 | 6 | **57.5%** | command.rs region |

**Key Insight**: S0 hit only binary entrypoints (guaranteed 0%). S4 hit well-tested code (command.rs, 33 tests) and achieved 57.5%. The editor's effective kill rate depends heavily on *which* files each shard hits.

---

## 3. Classification Framework

### Pre-Sweep Exclusion Estimates

| Category | Terrain | Render | Editor | Total | Action |
|----------|:-------:|:------:|:------:|:-----:|--------|
| **Const lookup tables** | 1,636 | 0 | 0 | 1,636 | Low-obs: test via integration |
| **Binary entrypoints** | 0 | 0 | ~624 | 624 | Integration-level: exclude from `--lib` |
| **Feature-gated code** | 0 | ~481 | TBD | 481+ | Document or test with features |
| **GPU-dependent code** | 0 | TBD | TBD | TBD | Integration-level |
| **Subtotal excluded** | **1,636** | **~481** | **~624** | **~2,741** | |
| **Testable pool** | **~2,993** | **~3,836** | **~24,949** | **~31,778** | |

### Adjusted Pool Size: ~31,778 actionable mutants

---

## 4. Execution Strategy

### Priority Order: Terrain → Render → Editor

**Rationale**: 
- Terrain is smallest (2,993 actionable), most mathematical, fastest per-mutant (~26s)
- Render is medium (3,836 actionable), has clear feature-gate boundaries
- Editor is largest (24,949 actionable), but has 3,592 existing tests already

### Phase 1: Full Sweeps (Establish Baselines)

#### 1A. Terrain Full Sweep (22 shards)

```
Estimated time: 22 shards × ~39 min = ~14 hours
Strategy: Sequential shards with disk cleanup between each
Command: cargo mutants -p astraweave-terrain --shard {N}/22 --timeout 300 -j 1 -o "C:\temp" -- --lib
```

**Expected outcomes**:
- marching_cubes_tables.rs: Likely ~1,636 misses (const tables) → classify low-obs
- advanced_erosion.rs: ~488 mutants, expect ~60-70% caught → remediate gaps
- heightmap.rs, climate.rs, biome_blending.rs: Math-heavy, good test targets

#### 1B. Render Full Sweep (20 shards)

```
Estimated time: 20 shards × ~4h = ~80 hours (can parallelize with -j 2)
Strategy: Run with default features first, classify feature-gated misses
Command: cargo mutants -p astraweave-render --shard {N}/20 --timeout 300 -j 1 -o "C:\temp" -- --lib
```

**Expected outcomes**:
- renderer.rs: 536 mutants, many behind cfg — expect moderate kill rate
- camera.rs: Pilot showed 71/154 caught (46%) — needs remediation
- environment.rs: 401 mutants — likely needs significant remediation
- Feature-gated: ~481 → auto-classify as feature-gated exclusion

#### 1C. Editor Prioritized Sweep (121 shards, tiered)

```
Estimated time: 121 shards × ~1-8h each = ~200+ hours
Strategy: TIERED — high-value files first, skip binary shards
```

**Editor Shard Tiers**:

| Tier | Shards | Description | Priority |
|------|:------:|-------------|:--------:|
| **Skip** | S0 (main.rs/bin/) | Binary entrypoints, 0% kill | N/A |
| **High** | S4-S30 (command, entity, runtime, panels) | Well-tested modules | 1st |
| **Medium** | S31-S80 (tab_viewer, dialog, animation) | Large files, good tests | 2nd |
| **Low** | S81-S120 (remaining panels, UI) | Smaller files | 3rd |

### Phase 2: Classification (After Each Crate Sweep)

For each completed sweep:
1. Parse `outcomes.json` — group misses by file and function
2. Classify each miss: genuine gap, feature-gated, low-obs, binary/integration, equivalent
3. Calculate adjusted kill rate
4. Identify remediation targets (files with >10 genuine misses)

### Phase 3: Remediation (Targeted Test Writing)

**Priority order for test writing**:
1. **Mathematical/algorithmic code** — erosion, heightmap, camera, culling (100% testable)
2. **State management** — entity_manager, command (undo/redo), runtime (already well-tested)
3. **Panel logic** — editor panels have extensive tests but may have gaps in mutation detection
4. **Data loading** — material_loader, mesh loading (parsers are highly testable)

### Phase 4: Report Update

Update `MUTATION_TESTING_VERIFICATION_REPORT.md` to v4.0:
- Add 3 new crate sections
- Update aggregate totals
- New campaign totals (original 9 P0 crates + 3 visual P0 crates)

---

## 5. Risk Mitigation

| Risk | Probability | Impact | Mitigation |
|------|:-----------:|:------:|------------|
| Editor sweep takes >200h | High | Schedule slip | Tiered approach — high-value shards first |
| Render shards timeout at 300s | Medium | False misses | Increase to `--timeout 600` if needed |
| marching_cubes constants flood misses | High | Inflated miss count | Pre-classify as low-obs, exclude from adjusted rate |
| GPU code reports as missed | High | False gap signal | Classify as integration-level |
| Source corruption (Windows) | Low | Code damage | Never use `--in-place`, verify git status between shards |
| Disk space exhaustion | Medium | Build failures | Mandatory cleanup between shards |

---

## 6. Success Criteria

| Metric | Target |
|--------|--------|
| **Kill rate (adjusted)** per crate | **80%+** |
| **Classification coverage** | 100% of missed mutants classified |
| **Remediation tests** | All genuine gaps with >5 misses addressed |
| **Report** | Updated to v4.0 with all 12 P0 crates |

---

## 7. Estimated Timeline

| Phase | Terrain | Render | Editor | Wall Clock |
|-------|:-------:|:------:|:------:|:----------:|
| Full sweep | 14h | 80h | 200h+ | — |
| Classification | 2h | 4h | 8h | — |
| Remediation | 4-8h | 8-16h | 16-32h | — |
| **Total per crate** | **~20-24h** | **~92-100h** | **~224-240h** | — |

**Recommended approach**: Run sweeps sequentially (terrain → render → editor) with remediation after each crate's full sweep. This allows incremental progress and avoids wasteful full-sweep-then-remediate for the enormous editor pool.

---

*Plan generated from pilot shard analysis: Terrain S0/22, Render S0/20, Editor S0/121, Editor S4/121.*

---

## 8. Execution Progress Log

### Remediation Test Writing (Pre-Sweep)

| Date | Commit | Tests Added | Files | Crates |
|------|--------|:-----------:|:-----:|--------|
| Feb 16 | `685fa4e0` | 570 terrain + 549 render + ~3,592 editor (pre-existing) | 7+7+18 | terrain, render, aw_editor |
| Feb 20 | `e2666954` | 533 editor panel tests | 9 new files | aw_editor |

**Current editor test total**: 8,793 tests (0 failures, 5 ignored)

### Sweep Status

| Crate | Shards | Status | Kill Rate | Notes |
|-------|:------:|--------|:---------:|-------|
| astraweave-terrain | 0/22 | 🟡 Running S0 | TBD | ~211 mutants/shard, ~39min/shard |
| astraweave-render | 0/20 | ⬜ Not started | TBD | Waiting on terrain completion |
| aw_editor | 0/121 | ⬜ Not started | TBD | Tiered approach planned |

### Sweep Scripts

- `scripts/run_terrain_sweep.ps1` — Automated 22-shard sequential sweep
- `scripts/run_render_sweep.ps1` — Automated 20-shard sequential sweep
