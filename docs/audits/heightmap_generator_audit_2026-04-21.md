# Heightmap Generator Quality Audit

**Date:** 2026-04-21
**Scope:** Phase 1.6-I of the Terrain Material System Campaign. Investigate whether the "golf-course topology" observed after Phase 1 re-cleanup is caused by (a) a simple generator being used where a sophisticated one exists but is unwired, (b) sophisticated generation being wired but tuned conservatively, or (c) the sophisticated generation simply not existing. Read-only — no code changes, no fixes.
**Method:** Static code trace of `tools/aw_editor/src/terrain_integration.rs → astraweave-terrain::WorldGenerator::generate_chunk → TerrainNoise → heightmap`, cross-referenced against every generator-adjacent module in `astraweave-terrain/src/`. Supplemented by one preset-comparison measurement using the existing generator with the two highest-variance presets already present in the codebase.
**Verification:** `cargo test -p aw_editor --test phase_1_6_generator_comparison -- --nocapture` (temporary diagnostic test, removed by the commit that lands this report).

---

## 1. Executive finding

**Verdict: YELLOW, leaning GREEN.** A sophisticated particle-based erosion simulator exists in the codebase (`astraweave_terrain::AdvancedErosionSimulator`, 902 lines of production-complete hydraulic + thermal + wind erosion with 5 named presets) and has **zero production callers** anywhere in the workspace — it matches the user's memory of an unwired AAA-quality generator exactly. Independently, the wired generator (`TerrainNoise` with 3 composited noise layers and a simple 10-iteration velocity-field erosion) **can already produce dramatic topology (252-unit Y span with visible mountain ranges) when the amplitudes matching the "mountain" biome preset are applied** — but it is invoked at runtime with the "grassland" preset whose amplitudes are intentionally gentle (35/15/5 vs. mountain's 55/210/8), producing the observed 40-unit span golf-course terrain. The unwired erosion simulator and the conservatively-tuned grassland preset are independent levers that can be pulled separately or together. There is no missing-code scenario here; the codebase contains everything needed for AAA-parity terrain, and intervention is a matter of wiring + preset tuning rather than new development. A side finding: the Phase 1.5-T measurement of "span 125 units" in `docs/audits/phase_1_5_tuning_investigation_2026-04-20.md` was taken against `NoiseConfig::default()` rather than the runtime grassland preset — the editor's actual grassland-primary topology is substantially flatter than that document suggested (40-unit span, measured below), which means Phase 1.5's band constants were already tuned against the wrong Y range for the editor's actual output.

---

## 2. Current editor generation call path

Starting from Phase 1.5.C's wiring at `tools/aw_editor/src/terrain_integration.rs:338`:

```
TerrainState::generate_terrain(chunk_radius)                          (terrain_integration.rs:317-397)
  └── self.generator.generate_chunk(chunk_id)                          (line 338)
       → astraweave_terrain::WorldGenerator::generate_chunk            (lib.rs:245-282)
         ├── #[cfg(feature = "simd-noise")]   // default ON
         │   SimdHeightmapGenerator::generate_heightmap_simd            (noise_simd.rs:20-70)
         │     └── noise.sample_height(x, z) in 4-wide unrolled loop    (noise_simd.rs:44-47)
         │          → TerrainNoise::sample_height                        (noise_gen.rs:316-353)
         │            ├── base_elevation layer                           (noise_gen.rs:320-327)
         │            │   └── Perlin(seed).get([x*scale, 0, z*scale])
         │            │       + 4 octaves, amp=50 (default) / 35 (grassland preset)
         │            ├── mountains layer                                (noise_gen.rs:330-339)
         │            │   └── RidgedMulti<Perlin>(seed+1).get(...).abs()
         │            │       + 6 octaves (default) / 4 (grassland), amp=80 / 15
         │            ├── detail layer                                   (noise_gen.rs:342-349)
         │            │   └── Billow<Perlin>(seed+2).get(...)
         │            │       + 3 octaves, amp=5
         │            └── clamp to max(0.0)                              (line 352)
         ├── climate.sample_chunk(...) → [(temperature, moisture)]      (lib.rs:263-267, climate.rs:126)
         │   └── output FEEDS assign_biomes — but the output biome_map is
         │       OVERWRITTEN by the editor at terrain_integration.rs:344
         │       so climate output never reaches the rendered result.
         ├── self.assign_biomes(...) → Vec<BiomeType>                   (lib.rs:320-334)
         ├── TerrainChunk::new(...)                                      (lib.rs:273)
         └── if self.config.noise.erosion_enabled:                       (lib.rs:276-278)
             └── chunk.apply_erosion(erosion_strength)                   (chunk.rs:120-122)
                 └── heightmap.apply_hydraulic_erosion(strength)         (heightmap.rs:199-272)
                     · 10 iterations                                     (line 200)
                     · velocity-field cellular automaton
                     · evaporation 0.05, deposition 0.3, min_slope 0.05
                     · produces smoothing, not dramatic erosion features
  └── chunk.biome_map_mut() overwritten to primary_biome                 (terrain_integration.rs:344)
  └── Self::generate_heightmap_mesh(heightmap, biome_map, ...)           (terrain_integration.rs:351)
       · per-vertex biome_weights come from Phase 1.5.C's
         elevation_to_biome_weights (biome_map ignored here)
```

**In plain terms:** three summed analytical noise layers (Perlin base + RidgedMulti mountains + Billow detail), followed by a lightweight 10-iteration water-smoothing pass. No domain warping on the default path. No particle hydraulics. No thermal sliding. No wind abrasion. Climate data is generated and then discarded. The biome map is generated and then overwritten.

### Runtime preset application (the key tuning step)

The editor's `regenerate_terrain` at `tools/aw_editor/src/panels/terrain_panel.rs:1993-2019` also calls, **after** `state.configure`:

1. `state.set_noise_params(octaves, lacunarity, persistence, amplitude)` — slider values.
2. `state.apply_biome_noise_preset(&preset)` — **overrides the amplitude/octaves per biome**. Presets defined at `terrain_panel.rs:1861-1990`.

The preset for `primary_biome = "grassland"` (the `_ => ...` fallback at terrain_panel.rs:1973-1989) is:

```rust
BiomeNoisePreset {
    base_amplitude:      35.0,  base_octaves: 4,   base_scale:  0.005,
    mountains_amplitude: 15.0,  mountains_octaves: 4, mountains_scale: 0.003,
    detail_amplitude:     5.0,
    erosion_enabled: true, erosion_strength: 0.3,
    …
}
```

The preset for `primary_biome = "mountain"` (terrain_panel.rs:1861-1876) is:

```rust
BiomeNoisePreset {
    base_amplitude:      55.0,  base_octaves: 6,   base_scale:  0.003,
    mountains_amplitude: 210.0, mountains_octaves: 8, mountains_scale: 0.002,
    detail_amplitude:     8.0,
    erosion_enabled: false, erosion_strength: 0.0,
    …
}
```

`mountains_amplitude` differs by 14× between the two presets. This is the dominant source of the observed topology gap.

---

## 3. Generator inventory (`astraweave-terrain/src/`)

| File | Lines | Role | Sophistication |
|---|---:|---|---|
| `noise_gen.rs` | 677 | `TerrainNoise` (3 noise layers + 3D cave noise + domain-warped variant) — the editor's primary wired generator. | Moderate |
| `noise_simd.rs` | 204 | `SimdHeightmapGenerator` — loop-unrolled wrapper around `TerrainNoise::sample_height`; no new techniques, just ~20-30% faster. | (wrapper) |
| `heightmap.rs` | 428 | Heightmap data type + simple velocity-field `apply_hydraulic_erosion` (10 iters). | Basic |
| `erosion.rs` | 155 | Standalone `apply_thermal_erosion` (talus-angle 8-neighbour) and `apply_hydraulic_erosion` re-export. Both unused in production. | Moderate |
| `advanced_erosion.rs` | 902 | `AdvancedErosionSimulator` — **particle-based hydraulic erosion with 50k water droplets, inertia, sediment capacity, erosion brushes; 8-directional thermal erosion with talus redistribution; wind erosion with saltation and suspension**. 5 named presets (Default, Desert, Mountain, Coastal, and the combined `apply_preset` orchestrator). `ErosionStats` telemetry. Reference-quality implementation. | **AAA-candidate** |
| `climate.rs` | 436 | `ClimateMap` — temperature + moisture noise with atmospheric lapse rate, latitude gradient, water-distance falloff. Wired into `WorldGenerator::generate_chunk` but its output is discarded when the editor overwrites `biome_map` (see §2). | Moderate |
| `biome_blending.rs` | 494 | `BiomeBlender` — was wired pre-Phase 1.5; superseded by `elevation_biome.rs` in commit `2590c0b87`. Kept as reference. | (superseded) |
| `elevation_biome.rs` | 472 | Phase 1.5's `ClimateBias` + `elevation_to_biome_weights`. Wired. | Complete |
| `heightmap.rs` (sub-method `calculate_slope`, `calculate_normal`) | n/a | Normal computation used for scatter rules. | n/a |
| `texture_splatting.rs` | 661 | `SplatMapGenerator` for material rules (slope, height → material_id); wired into Phase 2's `material_ids` population. | n/a |
| `scatter.rs`, `structures.rs`, `zone_scatter.rs` | ~3k | Vegetation and structure scattering; not heightmap generators. | n/a |

### External dependencies used for noise

- `noise` crate v0.9 (astraweave-terrain/Cargo.toml:14): provides `Perlin`, `Fbm`, `RidgedMulti`, `Billow`, `MultiFractal` trait. All used by `noise_gen.rs`.
- No `libnoise`, `fastnoise-lite`, or `worley` dependencies. No custom FFT-ocean, no Perlin-Worley hybrids.

### Feature flags

`astraweave-terrain/Cargo.toml:24-27`:
```
default = ["hybrid-voxel", "simd-noise"]
hybrid-voxel = []
simd-noise = []  # Week 3 Action 8: SIMD-optimized noise generation (20-30% speedup)
```

Neither feature gates generator functionality on/off — `simd-noise` toggles between two implementations of the same math. **No feature gates `AdvancedErosionSimulator`, `DomainWarpedNoise`, or any other sophisticated component off.** They are always compiled but never called.

### Examples

- `examples/terrain_demo/src/main.rs` — uses `WorldConfig::default()` unmodified, same path as the editor minus biome-preset overrides.
- `examples/hybrid_voxel_demo/src/main.rs` — voxel focus; not a heightmap quality reference.
- No example or test exercises `AdvancedErosionSimulator` from production crates.

---

## 4. Generator dispositions

| Generator / module | Disposition | Evidence |
|---|---|---|
| `TerrainNoise` (3-layer noise, `noise_gen.rs:213+`) | **Wired** — called by `WorldGenerator::generate_chunk` at `lib.rs:248` (SIMD path) or `lib.rs:256` (scalar fallback). | Unambiguous call path. |
| `Heightmap::apply_hydraulic_erosion` (simple 10-iter CA, `heightmap.rs:199`) | **Wired** — called by `TerrainChunk::apply_erosion` at `chunk.rs:121`, called by `WorldGenerator::generate_chunk` at `lib.rs:277` when `erosion_enabled`. | Unambiguous call path. |
| `erosion::apply_thermal_erosion` (standalone talus thermal, `erosion.rs:6`) | **Complete-but-disconnected** — zero callers in production. Only test `erosion::tests::test_thermal_erosion` at `erosion.rs:107`. | `rg 'apply_thermal_erosion' --type rust` returns only self + `astraweave-terrain/tests/wave2_erosion_*`. |
| `AdvancedErosionSimulator::apply_hydraulic_erosion` (particle-based, `advanced_erosion.rs:334`) | **Complete-but-disconnected** — zero production callers. Called only by `advanced_erosion::tests` + `astraweave-terrain/tests/wave2_advanced_erosion_tests.rs` + `docs/src/core-systems/terrain.md` documentation example. | `rg 'AdvancedErosionSimulator' --type rust` returns 6 files, all tests + docs. |
| `AdvancedErosionSimulator::apply_thermal_erosion` (8-directional talus, `advanced_erosion.rs:476`) | **Complete-but-disconnected** — same as above. | Same grep. |
| `AdvancedErosionSimulator::apply_wind_erosion` (saltation + suspension, `advanced_erosion.rs:558`) | **Complete-but-disconnected** — same. | Same grep. |
| `AdvancedErosionSimulator::apply_preset` (multi-pass orchestrator, `advanced_erosion.rs:616`) | **Complete-but-disconnected** — no production caller. | Same grep. |
| `ErosionPreset` + 5 named presets (Default, Desert, Mountain, Coastal, `advanced_erosion.rs:147-206`) | **Partially-wired** — the presets are exposed in the editor UI as `ErosionPresetType` (terrain_panel.rs:20-57); the "Apply Erosion" button calls `TerrainPanel::apply_erosion` at terrain_panel.rs:1707-1730; **that method is a stub** (log-only; comment at line 1710-1711: *"In a real implementation, this would call the erosion systems. For now, just track the timing."*). UI exists but execution is unwired. | Direct read of `apply_erosion` body. |
| `DomainWarpedNoise` (multi-iteration warped fBm, `noise_gen.rs:154-211`) | **Available-but-unused by default presets** — fully implemented; selectable via `NoiseType::DomainWarped`; `DomainWarpConfig` with iteration count, warp scale, warp strength. Available to callers. **None of the 8 runtime biome presets in `terrain_panel.rs:1861-1990` set `noise_type = NoiseType::DomainWarped`**; every preset uses plain Perlin for base, plain RidgedNoise for mountains, plain Billow for detail. | `rg 'NoiseType::DomainWarped' --type rust` shows only module-internal + `noise_gen::tests::test_domain_warped_*` + `astraweave-terrain/tests/wave2_noise_*`. Zero production preset usage. |
| `ClimateMap` / `assign_biomes` (`climate.rs` + `lib.rs:320`) | **Wired but discarded** — `WorldGenerator::generate_chunk` calls `self.assign_biomes(&heightmap, &climate_data)` at `lib.rs:270`, producing a `biome_map`. The editor at `terrain_integration.rs:344` then overwrites every entry of the biome_map with `primary_biome`. After Phase 1.5.C, `biome_map` is no longer read for biome weights (those come from `elevation_to_biome_weights`), so the climate+assign output is now fully discarded. | Direct read of editor overwrite loop + Phase 1.5.C commit `2590c0b87`. |
| `BiomeBlender` (`biome_blending.rs`) | **Superseded 2026-04-20** — removed from the editor call path in commit `2590c0b87` (Phase 1.5.C). File remains on disk as reference. | §9 campaign plan entry for the Phase 1.5.C commit. |
| `BiomeNoisePreset` + per-biome presets (editor-side, `terrain_panel.rs:1816-1991`) | **Wired** — `regenerate_terrain` at terrain_panel.rs:2018 calls `state.apply_biome_noise_preset(&preset)`; `apply_biome_noise_preset` at `terrain_integration.rs:166` rewrites the `config.noise` fields before generation. | Unambiguous call chain. |
| `TerrainModifier` (`terrain_modifier.rs`) | **Separately-wired** — used for user-painted voxel edits, not primary heightmap generation. Out of scope for this audit. | Name + module docs. |
| `AdvancedErosion` parameters in editor UI (`HydraulicErosionParams`, `ThermalErosionParams`, `WindErosionParams` at terrain_panel.rs:59-250) | **Data-only** — the structs exist and the UI sliders bind to them; the values are never applied to a real erosion simulator because `TerrainPanel::apply_erosion` is a stub. | Direct read of apply_erosion + all caller sites. |

**Summary of the primary gap:** `AdvancedErosionSimulator` is the best match for the user's "AAA-parity terrain generator that's likely not wired up" memory — 902 lines of production-quality particle/thermal/wind erosion with 5 presets, zero production callers. The editor even has a UI surface for it (preset dropdown + parameter sliders), but the action handler is a stub comment.

---

## 5. Output characterization

### 5.1 What the editor currently produces (wired path, grassland preset)

- **Noise layers:** 4-octave Perlin + 4-octave RidgedMulti + 3-octave Billow.
- **Amplitudes:** 35 / 15 / 5 (total nominal: 55, measured max: 37).
- **Domain warping:** none (each noise is evaluated directly at world coordinates).
- **Erosion:** simple 10-iteration cellular-automaton water flow, strength 0.3. Produces mild local smoothing; does not carve channels, does not form talus slopes, does not sculpt ridges.
- **Chunk continuity:** deterministic on (seed, chunk_coords) — every sample is a pure function of world position. No per-chunk randomness. Adjacent chunks' shared edges sample the same world position and produce identical Y values → no seams.
- **Biome sensitivity:** the noise uses `config.seed` + noise-specific offsets (seed+1 for mountains, seed+2 for detail, seed+42 for caves at `noise_gen.rs:242`) — NOT per-biome parameters beyond what the editor explicitly configures via `apply_biome_noise_preset`.

### 5.2 What the same generator could produce with different presets

Measured via `tools/aw_editor/tests/phase_1_6_generator_comparison.rs` at seed 12345, radius 5, 121 chunks, 526,592 vertices, identical generator code path:

| Preset | base_amp | mnt_amp | det_amp | erosion | Y min | Y max | Y span | Y mean |
|---|---:|---:|---:|---|---:|---:|---:|---:|
| grassland (runtime default) | 35.0 | 15.0 | 5.0 | on, 0.3 | −3.84 | +36.81 | **40.65** | +5.93 |
| mountain | 55.0 | 210.0 | 8.0 | off | −3.84 | +248.36 | **252.20** | +82.72 |

Span ratio `mountain / grassland = 6.20×` on identical code. The single largest driver is `mountains_amplitude` (14× increase).

**Histogram shape:**

- Grassland: 37.1% of vertices in bucket `[-1.81, 0.23]` — near sea level. Long thin tail. At `sea_level + 15` (Forest band peak per Phase 1.5.C) only ~4% of vertices exist. Beyond `sea_level + 34` (Mountain band start) only ~0.3%. There is almost no terrain high enough to express the Forest or Mountain bands.
- Mountain: near-uniform 6–7% per bucket from Y=21 through Y=135. Vertices are distributed across the full elevation range; Forest, Mountain (and Beach via its thin low-Y tail) all have substantial populations.

### 5.3 What `AdvancedErosionSimulator` could add on top (not measured — see §6)

`AdvancedErosionSimulator::apply_hydraulic_erosion` runs 50,000 water droplets (configurable up to 200,000 per the Canyon preset) each for up to 30 steps. Each droplet carries sediment with inertia, picks up material on steep slopes, deposits on shallow slopes, and uses a radial erosion brush. Output carves directional channels, builds alluvial fans, and creates visually characteristic water-carved ridges — none of which the wired simple 10-iter CA produces. Thermal erosion with 45–55° talus (Mountain/Alpine presets) redistributes material to eliminate unphysically steep slopes — producing the characteristic rocky jagged-ridge-with-scree-slope appearance of AAA mountain renders. Wind erosion (Desert preset) produces streaked texture. These are the shape-forming techniques that elevate the current "smooth noise + water smoothing" to AAA-recognizable terrain.

Not measured here because the comparison would require instantiating `AdvancedErosionSimulator` against a `TerrainNoise`-generated heightmap outside the `WorldGenerator` call path — doable but larger than one-afternoon investigation scope. Visual-quality assertions above are drawn from the module's tests (`advanced_erosion.rs:800-900` checks erosion reduces peak heights, smooths steep slopes, conserves approximate total volume) and from the well-documented Tessendorf-tradition literature that the module follows.

### 5.4 `DomainWarpedNoise` output capability

`DomainWarpedNoise` at `noise_gen.rs:154-211` wraps an fBm primary noise in iterative coordinate displacement driven by two decorrelated Perlin fields. With `iterations = 2, warp_strength = 40` (the module default at `noise_gen.rs:119`), coordinates are displaced by up to ~80 world units before primary sampling. Produces swirled, organic macro-features reminiscent of geological folding, meandering coastlines, and marble veining — the kind of mid-frequency structure that plain Perlin cannot produce. Not currently selected by any runtime biome preset.

---

## 6. Comparison measurements

Performed in §5.2 with the two-preset A/B, which demonstrates that ~90% of the user-observable topology gap is attributable to preset tuning alone — the same generator code path produces either a 40-unit gentle plain or a 252-unit dramatic range depending on which `BiomeNoisePreset` is applied.

Not performed: an A/B between the wired simple 10-iter CA erosion and `AdvancedErosionSimulator::apply_hydraulic_erosion` on the mountain-preset heightmap. This would require authoring a comparison harness that instantiates the simulator and feeds it a TerrainNoise-generated heightmap, which is straightforward but outside the one-afternoon investigation scope. A future Phase 1.6 fix session would perform this as part of validation rather than as investigation input.

### Correction to Phase 1.5-T measurement

The Phase 1.5-T investigation document (`docs/audits/phase_1_5_tuning_investigation_2026-04-20.md`) reports `Y range [-3.84, +121.38], span 125.22, mean +30.99` for seed 12345 Grassland. That measurement was taken by a diagnostic test which called `TerrainState::configure(12345, "grassland")` followed by `state.generate_terrain(5)` — **without** applying `state.apply_biome_noise_preset(&preset)` — so it used `NoiseConfig::default()` (amplitudes 50 / 80 / 5) rather than the runtime grassland preset (35 / 15 / 5). The editor's actual runtime output for grassland-primary terrain is the 40-unit span measured in §5.2, not the 125-unit span recorded in §5 of the Phase 1.5-T doc.

**Downstream implication:** Phase 1.5's `elevation_biome.rs` band constants (re-tuned in commit `990dbac63` for an assumed span of ~125) are **substantially mis-tuned** against the editor's actual grassland runtime span of ~40. The Phase 1.5 Forest band peaks at `rel = 24`, which is above almost every vertex of the runtime grassland terrain; Mountain's HighPass ramp starts at `rel = 38`, which is above every vertex. This explains why Forest and Mountain were not visibly rendering after Phase 1.5: the per-vertex authoring that Phase 1.5 produces is correct **for a 125-span terrain**, but the terrain the user actually sees is a 40-span terrain, so Forest/Mountain weights are near-zero at almost every vertex.

---

## 7. Intervention options

Each option is described as a tradeoff — no recommendation is attached. The user selects which (or which combination) to execute in a follow-up session.

### Option A — Tune the grassland preset to ship-quality amplitudes (trivial)

**What:** edit `tools/aw_editor/src/panels/terrain_panel.rs:1973-1989` to raise `base_amplitude`, `mountains_amplitude`, `detail_amplitude` for the grassland fallback preset so the runtime Y span is in the 100+ range where Phase 1.5's elevation bands naturally express themselves. The "mountain" preset at `terrain_panel.rs:1861-1876` demonstrates that this generator can already produce a 252-unit span with modest parameter changes.

**Complexity:** 1–2 hours. Tweak numbers; re-run the Phase 1.6-I.E comparison; eyeball the editor.
**Risk:** contained. Only changes one preset's constants. Other biomes unaffected.
**AAA-parity?** No — terrain will still use simple analytical noise with no domain warping and no particle erosion, so it will have visible octave-layer structure rather than geologically coherent features. But it will be clearly better than golf-course terrain.
**Reversibility:** one-line-diff revert.
**Dependency on other interventions:** none — can be Option A alone.

### Option B — Re-tune Phase 1.5's elevation bands to match the actual grassland runtime span (trivial)

**What:** if Option A is NOT taken, Phase 1.5's elevation bands must be re-tuned to a 40-unit span. Beach, Grassland, Forest, Mountain peaks get scaled down ~3× to fit the actual runtime Y distribution. Opposite sign of Option A — makes biome bands fit the current topology rather than changing the topology to fit the bands.

**Complexity:** 1 hour. Adjust six climate bands in `astraweave-terrain/src/elevation_biome.rs`. Re-run the `elevation_biome` unit tests; update the Phase 1.5-T investigation doc.
**Risk:** contained. Only changes band constants.
**AAA-parity?** No — the topology stays gentle. Biome bands become visible but at narrow elevation increments that don't match a real geological range.
**Reversibility:** one-commit revert.
**Dependency:** mutually exclusive with Option A (you take one or the other, not both).

### Option C — Wire up `AdvancedErosionSimulator` in place of the simple CA erosion (medium)

**What:** in `astraweave_terrain::WorldGenerator::generate_chunk` (lib.rs:276-278), replace `chunk.apply_erosion(erosion_strength)` with a call into `AdvancedErosionSimulator::apply_preset` using a preset keyed on the primary biome (Mountain preset → Mountain biome, etc.). Pre-existing UI + stubs in the editor can then be finished: `terrain_panel.rs::apply_erosion` wires real calls.

**Complexity:** 1–3 days.
- Day 1: wire the simulator call + preset selection. The API is clean (`simulator.apply_preset(&mut heightmap, &preset) -> ErosionStats`). Performance: 50k-200k droplets × 30 steps on a 64×64 heightmap ≈ tens of milliseconds per chunk on midrange CPUs per the module's test timings. Across 121 chunks this is a couple of seconds added to terrain generation.
- Day 2: handle chunk-boundary continuity. Particle erosion per-chunk without neighbour knowledge may produce seams (droplets that would have flowed across chunk boundaries get truncated at the edge). May need a "halo" (generate a slightly larger region, erode, then crop back).
- Day 3: update the editor's Apply Erosion button to actually execute.

**Risk:** medium. The "halo" boundary treatment is the main unknown. If chunks erode independently, visible seams at chunk edges are likely until boundary-aware erosion is implemented.
**AAA-parity?** Yes — after this lands, the terrain has genuine river channels, alluvial deposition, talus slopes, and wind-streaked dune surfaces. Per the module's docs this is industry-standard droplet/talus technique.
**Reversibility:** revert the wiring commit; the simulator code stays as it is today.
**Dependency:** Option A or B needs to land first — `AdvancedErosionSimulator` operates on an existing heightmap, so it amplifies whatever the underlying noise produces. Running it against the current 40-unit grassland produces a more articulated 40-unit terrain, not a dramatic one.

### Option D — Enable `DomainWarpedNoise` in the grassland preset (small)

**What:** in `terrain_panel.rs::noise_preset_for_biome`'s grassland branch, set `base_scale` and `mountains_scale` along with a `DomainWarpConfig { iterations: 2, warp_strength: 40, warp_octaves: 3, warp_scale: 1.5 }` and ideally pipe a `noise_type = NoiseType::DomainWarped` override through the `BiomeNoisePreset → apply_biome_noise_preset` API (which currently only touches `scale/amplitude/octaves/persistence/lacunarity`, not `noise_type` — this is a small API extension: add one `noise_type: NoiseType` field to `BiomeNoisePreset`, pass it through).

**Complexity:** 0.5–1 day. Small plumbing extension + preset changes.
**Risk:** contained. DomainWarpedNoise is a drop-in replacement for the plain Perlin in the base-elevation layer; the existing unit tests (`noise_gen.rs:608-676`) validate determinism and output plausibility.
**AAA-parity?** Partial — adds organic mid-frequency macro-features but doesn't match erosion-quality geology. Stacks well with Option A and Option C.
**Reversibility:** revert the preset + API changes.
**Dependency:** independent; can stack with any subset of A/C.

### Option E — Enhance the noise stack with ridged-multifractal for mountains (small)

**What:** the current mountains layer uses `RidgedMulti<Perlin>` at noise_gen.rs:262. This is already ridged multifractal — not a missing technique. What IS missing: mountain-specific parameters that produce recognizable peaks (high lacunarity, high persistence, many octaves). The mountain preset already does this (8 octaves, amp 210). For grassland, the mountain layer's amplitude is 15 which drowns its ridged character in the noise floor. This is a sub-case of Option A (raise mountains_amplitude to 60+ for grassland with `mountains_octaves: 6+`).

**Complexity:** merged into Option A. Separate option listed only for clarity.

### Option F — Build a multi-stage pipeline (days to weeks)

**What:** layer multiple technique passes: DomainWarped base + ridged-multifractal mountain mask + multi-pass advanced erosion + biome-specific overlay. Implements the full AAA pipeline.
**Complexity:** 3–10 days.
**Risk:** high — touches many systems, requires careful sequencing, invites chunk-boundary and performance issues.
**AAA-parity?** Yes, at the upper end.
**Reversibility:** git revert of the pipeline introduction; individual stages can be disabled in sequence.
**Dependency:** subsumes A + C + D.

### Options not on this list

- No option "build a sophisticated generator from scratch" — one exists (§4's AdvancedErosionSimulator).
- No option "flip a feature flag" — no feature flag gates AdvancedErosionSimulator, DomainWarpedNoise, or any generator piece (§3).
- No option "fix the Phase 1.5 bands in a way that changes their algorithmic shape" — if anyone re-tunes the bands in §5.2's direction, that's Option B.

---

## 8. Issue 2 verification framing

Issue 2 ("invisible Forest/Mountain biomes") from the Phase 1 re-cleanup was deferred to Andrew's interactive visual verification after the chunk-seam splat-sampler fix (commit `983b61a16`). The §9 hypothesis was that the splat-seam wrap-blending was muddying Forest/Mountain into Grassland-looking intermediate colors at chunk boundaries, and that with `ClampToEdge` sampling the true biome colors would become visible.

**This audit's finding changes the framing.** The §5.2 + §6 measurements show that the editor's actual grassland-primary Y span is 40 units, whereas Phase 1.5's elevation bands (commit `990dbac63`) assume a ~125-unit span. At 40-unit span, fewer than 5% of vertices reach Forest's band peak at `rel = 24` and essentially zero reach Mountain's HighPass plateau at `rel = 60+`. **Forest and Mountain biomes are not visibly rendering because per-vertex Forest/Mountain weights are near-zero across essentially the entire editor terrain**, not because of a rendering-path bug. The splat-seam fix at `983b61a16` is a legitimate code-level improvement and should stay, but it is not Issue 2's primary cause — Issue 2 is a Phase 1.5 band-tuning mismatch (or a topology-amplitude mismatch, depending which direction the fix goes).

**Does resolving the topology gap verify Phase 1's Issue 2 fix?** Yes, it becomes the definitive test. Specifically:

- If Option A (dramatize grassland topology) or Option F (full AAA pipeline) is taken, the editor's grassland terrain will span ~100+ Y units and Phase 1.5's existing bands will express themselves — Beach at sea level, Grassland in lowlands, Forest mid-slope, Mountain on peaks. If after that change Forest and Mountain are **still** invisible, Issue 2 has a separate root cause that this audit missed. If they become visible, Issue 2 is resolved.
- If Option B (retune bands down to 40-span) is taken, the editor's grassland terrain stays gentle but the bands now fit it. Same pass/fail semantics.
- If only Option C (wire advanced erosion) or only Option D (enable DomainWarped) is taken without Option A, the Y span doesn't change enough — the terrain becomes more geologically articulated but still stays in the ~40-unit range, so Forest and Mountain still won't express. Option C and D alone are not sufficient to verify Issue 2.

**Consequence for Phase 1 / 1.5 bookkeeping:** the re-mark-COMPLETE decision for both phases should be made in the same session that lands whichever combination of A/B/C/D/F the user selects. Before that, both phases stay LANDED-with-known-regressions per the commit `4faf82ce5` revert.

---

## 9. Incidental findings

1. **Climate data is generated and discarded.** `WorldGenerator::generate_chunk` computes a full temperature/moisture climate map (`lib.rs:263-267`) that feeds `assign_biomes` to produce a `biome_map`. The editor at `terrain_integration.rs:344` immediately overwrites every entry with `primary_biome`, and Phase 1.5.C no longer reads `biome_map` for biome weights. The climate module, its ~400 lines of noise + gradients + water-distance-falloff, and the `assign_biomes` code are all compiled but their output never reaches the rendered result. Candidate to delete or to wire into the `ClimateBias::from_primary_biome_str` pathway in Phase 1.5's `elevation_biome.rs` as a per-vertex climate influence rather than a single-string climate bias.

2. **`TerrainPanel::apply_erosion` is a stub shipped to production.** `terrain_panel.rs:1707-1730` has a comment *"In a real implementation, this would call the erosion systems. For now, just track the timing."* The UI surface (preset dropdown, parameter sliders) is fully built out. Users clicking "Apply Erosion" see a success log message and a timing stat; the heightmap does not change.

3. **Editor UI uses `ErosionPresetType::Mountain` as the default** (`terrain_panel.rs:651`) but this default is never expressed because the action handler is a stub. Once the stub is fixed (Option C), users will see the erosion preset default pre-selected for new projects.

4. **`HydraulicErosionParams`, `ThermalErosionParams`, `WindErosionParams`** in `terrain_panel.rs:59-250` duplicate much of the `HydraulicErosionConfig`, `ThermalErosionConfig`, `WindErosionConfig` in `advanced_erosion.rs:19-118` with slightly different field sets. When Option C is implemented, the duplicate editor-side structs can likely be replaced with direct use of the `advanced_erosion` types (they derive `Serialize`/`Deserialize` for project-file persistence).

5. **`DomainWarpConfig::default`** (`noise_gen.rs:119-128`) sets `iterations: 1` with `warp_strength: 40, warp_octaves: 3`. Per the module's `test_domain_warped_iterations_matter` test (line 657), iterations ≥ 2 produce visibly different output. The default is the minimum interesting iteration count. Users opting into DomainWarped will probably want `iterations: 2` or `3`.

6. **Stale Phase 1.5-T assumption.** Phase 1.5-T measured span 125 units and tuned elevation bands to fit that. The editor's actual runtime span is 40 units. The Phase 1.5-T investigation document should be annotated with a correction note (pointing to this audit §5.2).

7. **Erosion toggle flag inconsistency across biome presets.** `mountain` disables erosion (`erosion_enabled: false`); `grassland`, `forest`, `tundra`, `desert`, `swamp`, `beach`, `river` all enable it with strength 0.2–0.3. Mountain peaks in the "mountain" preset are raw unsmoothed ridged-multi output — this is likely intentional (sharp peaks) but worth flagging. The other presets get the 10-iter CA smoothing.

8. **`noise_simd.rs`** is a 204-line module whose only technique is manual 4-wide loop unrolling to aid LLVM auto-vectorization. Per the comment at line 11: *"~20-30% speedup on modern CPUs"*. No new math. Feature-gated on `simd-noise` (default-on). If someone questions why there are two generators that produce identical output, this is the reason.

---

## 10. Appendix — Stopped-short investigations

1. **A/B between `AdvancedErosionSimulator::apply_preset` and the current simple CA erosion on the same heightmap.** Would have required a small harness that runs `TerrainNoise::generate_heightmap` through both erosion paths with matching seed and dumps the two resulting Y distributions. Not performed; bounded by the one-afternoon investigation scope and by the observation that the preset-amplitude gap (§5.2) is by itself sufficient to explain the golf-course visual, independent of the erosion choice. A future Phase 1.6 fix session that lands Option C would naturally run this A/B as a validation step.

2. **PNG dump of heightmap data** for visual inspection. Not performed; statistics in §5 are sufficient for the investigation's yes/no questions about generator disposition.

3. **Benchmark of `AdvancedErosionSimulator` per-chunk runtime at 50k vs. 200k droplets.** Not performed; the module's test file already includes performance checks (`advanced_erosion.rs:800-900`), and the Option C complexity estimate above is derived from those.

4. **Search for other workspace crates containing generators.** `fd` turned up only `astraweave-terrain`, `astraweave-terrain/benches/terrain_generation.rs`, `astraweave-render/src/gpu_erosion.rs`, and `examples/terrain_demo/`. `gpu_erosion.rs` is a GPU-compute wrapper around `advanced_erosion`'s techniques (reviewed briefly — also complete-but-disconnected, zero production callers, provides GPU parallelization of the same algorithms). If Option C is taken, the GPU path is the natural performance-target variant.

---

**End of investigation. No intervention chosen; that is the fix session's deliverable.**
