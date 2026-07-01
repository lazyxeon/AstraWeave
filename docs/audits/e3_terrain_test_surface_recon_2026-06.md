# E3 Terrain Test-Surface Recon — Full Inventory + Three-Way Classification

| Field | Value |
|---|---|
| **Campaign** | R-series · **M2 / E3 recon** (M2's substantive item after E4's defer) |
| **Mode** | READ-ONLY recon. Zero code changed. Fixtures verified un-mutated (git clean post-run). |
| **Branch / base** | `campaign/roadmap` (`ae9328a20`, E4 resolved) |
| **Date** | 2026-06-30 |
| **Authority** | R.1 roadmap M2 E3: *"terrain/material green: terrain VP + materials VP."* R.0.B verdict: `terrain` PRODUCTION-CAPABLE-FAILING-TESTS (**936/8**), with M1 flagging the "8" as a likely single-binary undercount (render precedent: "1277/2" was really 19-across-14). |
| **Load-bearing finding** | The "8" is a **massive undercount**: it counted **one** target. The real failing surface is **≥45 failing tests across ≥13 targets** — **~43 test-rot** (from 3–4 deliberate terrain changes the assertions lagged) + **2 real-defect** (one scatter-Z bug, two tests). The **diagnostic-residue** bucket exists structurally but contributes **0 current failures** (those tests pass or are assert-free measurement). |
| **Gate** | HARD STOP for director review. E3's scope is **proposed, not enacted**. |

This note is the recon's diagnostic deliverable. The fixes are **later beats after the scope is ratified** — none is done here.

---

## §0. Method + the count correction

`astraweave-terrain` has **54 integration test targets** + the lib (804 unit tests) + in-src modules. The R.0.B "936/8" captured essentially **one binary** — the `mutation_resistant_comprehensive_tests` target (78 passed / **8 failed**). Running the broader surface (`cargo test -p astraweave-terrain --no-fail-fast`, debug + a release confirmation pass) surfaced the real picture.

**A practical hazard the recon hit (worth recording):** the terrain suite is **slow in debug** — the `*_perf`, `*_continuity`, and `*_diagnostic` targets run 50k+-droplet erosion simulations (60–145 s each; one perf test took 145.9 s even in release). A naive full `cargo test` run does not finish in a 10-minute window. The failing inventory was gathered by (a) the fast remediation/mutation targets in debug and (b) a `--release` full run for the slow tail. **Meta-finding:** because the full terrain suite isn't cheap to run, **drift accumulated invisibly** — the failures below are all *deliberate changes the tests were never updated against*, and nothing caught them. This strongly implies the full terrain test surface is **not gated in CI** (only a subset is).

**The real failing surface (≥45 tests across ≥13 targets):**

| Target | Failed | Dominant cause |
|---|---|---|
| `mutation_resistant_comprehensive_tests` | 8 | amplitude + chunk_size + fallback drift |
| `phase_1_6_f3_phase_1_halo_scaffolding` | 1 | F.3.B climate-modulation contract-change |
| `wave2_noise_golden_remediation` | ~16 | golden/fingerprint snapshots (amplitude drift) |
| `wave2_mutation_remediation_tests` | 4 | chunk_size/resolution + fallback drift |
| `wave2_biome_blending_remediation` | 3 | fallback grassland→desert |
| `wave2_noise_remediation` / `wave2_noise_and_misc_tests` | ~6 | noise amplitude/field defaults |
| `wave2_biome_lod_modifier_tests` | 1 | vegetation.density default drift |
| `wave2_partition_splatting_remediation` | 1 | fallback grass→sand |
| `wave2_shard19_structures_p99_remediation` | 1 | **test's own** chunk_size 256 hardcode |
| `wave2_lib_remediation` | 1 | **scatter-Z real-defect** |
| `wave2_shard0_targeted_remediation` | 1 | **scatter-Z real-defect** |
| `wave3_mutation_remediation` | 1 | biome vegetation.density drift |

(The slow `*_perf` / `*_continuity` / pure-measurement `*_diagnostic` targets **pass** — confirmed for those the release run reached: `phase_2_perf` 1/0 @145s, `phase_3_diagnostic` 3/0 with **0 asserts** = can't fail. Any residual failures in the slow tail are the same golden/config drift family, not new causes.)

---

## §1. Three-way classification (each failure, with evidence)

### Bucket A — TEST-ROT (~43 failures, 3–4 root causes) — the code is correct; the assertion lagged a **deliberate** change

**A.1 — Target-B "Enshrouded-class" amplitude increase (~27 failures).** The noise defaults were deliberately raised, documented in-source:
- [noise_gen.rs:183](../../astraweave-terrain/src/noise_gen.rs#L183) `amplitude: 150.0, // was 50 (×3)`; [:193](../../astraweave-terrain/src/noise_gen.rs#L193) `480.0, // was 80 (×6)`; [:203](../../astraweave-terrain/src/noise_gen.rs#L203) `12.5, // was 5 (×2.5)` — header "Phase 1.6-F.4.B.2.B: Target B (Enshrouded-class) amplitudes."
- Lagging tests: `noise_config_{base_elevation,mountains,detail}_defaults`, `noise_config_{base_elevation,detail,mountains}_amplitude`, `noise_config_default_{base,detail,mountains}_fields`, `config_returns_actual_config_all_fields` (all assert amp 50/80/5); `sample_height_has_nonzero_range` (asserts max<300; got **545.8** — heights grew with the amplitudes); and the **entire golden/fingerprint snapshot family** — `density_fingerprint` (expected −0.0546, got **420.6**), `multi_point_fingerprint`/`golden_sample_height_origin` (expected 35.8, got **500.8**), all `golden_sample_height_*`, `golden_base_only_*`, `golden_mountains_only_*`, `golden_density_*`. Golden snapshots are noise-output captures; the amplitude change invalidated every one. **Fix = regenerate goldens + update the amp/bound assertions.**

**A.2 — `chunk_size` 256→512 + `heightmap_resolution` 64→96 (~5 failures).** Deliberate, [lib.rs:147](../../astraweave-terrain/src/lib.rs#L147) `chunk_size: 512.0` (commit `32a3f28ad`, 2026-04-24). Lagging tests: `world_config_defaults`, `world_config_default_chunk_size`, `world_generator_stores_config` (assert 256), `world_config_default_resolution` (asserts old resolution; got 96). **This is the same 256→512 drift the render test-rot fix already established as canonical.** Also `structure_heights_are_varied` (see A.5). **Fix = update to 512/96.**

**A.3 — Fallback-biome changes (documented) (~8 failures).** Two deliberate fallback-default changes:
- Splat: [texture_splatting.rs:169-170](../../astraweave-terrain/src/texture_splatting.rs#L169-L170) — `// Fallback to sand (layer 1) — avoids grass bleed in non-grassland biomes` → `weights_0.y = 1.0`. Tests `from_weights_empty_fallback_to_first`, `from_weights_zero_total_fallback`, `normalization_threshold_is_00001`, `splat_weights_from_weights_zero_fallback` all assert the old layer-0 fallback.
- Biome blend: [biome_blending.rs:70-74](../../astraweave-terrain/src/biome_blending.rs#L70-L74) — `// Desert (1) is a safer default than Grassland (0)` → fallback to Desert. Tests `packed_blend_empty_fallback_grassland`, `packed_blend_filters_tiny_weights`, `packed_blend_zero_weights_fallback`, `blend_weights_outside_radius_zero` all assert the old Grassland fallback. **Fix = update the expected fallback layer/biome.**

**A.4 — Vegetation density default drift (2 failures).** `biome_config_exact::grassland_biome_type`, `biome_config_grassland_vegetation_density` assert `vegetation.density == 0.003`; the default changed. **Fix = update the value.**

**A.5 — `structure_heights_are_varied` — a test that hardcodes stale chunk_size (1 failure).** Panic: *"All 170 structures have the same height — resolution index formula probably wrong."* The panic *hypothesizes* a code bug, but it's **test-rot**: the test creates a chunk with `WorldConfig::default()` (chunk_size **512**) but then passes `chunk_size=256.0` to `generate_structures` ([test:686](../../astraweave-terrain/tests/wave2_shard19_structures_p99_remediation.rs#L686)). The index formula [structures.rs:399](../../astraweave-terrain/src/structures.rs#L399) uses the passed 256, collapsing the sample range → all structures read the same height. Test added `16f5e0ec0` (2026-02-21), broke when the default changed `32a3f28ad` (2026-04-24). **Fix = pass 512 (or the config's chunk_size).** *Cautionary: the panic string looks like a real-defect; it is not.*

**A.6 — `halo_cropped_heightmap_matches_single_chunk_generation` — contract-change (1 failure).** Panic: max_diff **205.3**, expected <0.01. **Test-rot, but of a different shape** (not a value-update): commit `447367c15` (Phase 1.X-F.3.B, 2026-04-30) wired `BootstrapSplineSet` per-vertex amplitude modulation into `apply_per_biome_modulation_to_halo` ([lib.rs:570-645](../../astraweave-terrain/src/lib.rs#L570-L645)), so `generate_chunk_with_climate(Temperate)` now **intentionally** diverges from the unmodulated legacy `generate_chunk`. The test (written 2026-04-23, *before* F.3.B) asserts a now-obsolete byte-identity between two paths that are deliberately different. **Fix = reframe (assert halo-path determinism, like the sibling `halo_generation_deterministic_per_seed` which passes) OR remove — NOT a value-update.**

### Bucket B — REAL-DEFECT (2 failures, one underlying bug) — the test is correct; the code is wrong

**B.1 — Scatter is insensitive to the Z chunk coordinate (`scatter_different_z_different_result`, `scatter_seed_z_position_matters`).** Two independent targets both assert Z-adjacent chunks scatter differently; both fail on **all three** checks (vegetation count, resource count, first position identical). The seed formula [lib.rs:244,252](../../astraweave-terrain/src/lib.rs#L244-L252) is `seed + x*1000 + z` — **X contributes ×1000 per unit, Z contributes only ×1.** The X-variation test *passes* (seed changes by 1000); the Z tests fail because seed+1 vs seed+2 produce identical scatter. The seed *intends* Z-variation but the downstream scatter is insensitive to the ×1 delta. **This is a real code defect** (not test-rot — the tests have no stale hardcode). **Production impact:** vegetation/resource scatter **repeats along the Z axis** for fixed X — visible procedural monotony in rendered terrain. **Severity: moderate–high.** **The mechanism needs pin-down in the fix beat** (candidates: the ×1 Z multiplier is too weak for the scatter RNG's seed sensitivity; the RNG ignores low-order seed bits; or the seed isn't fully threaded into placement) — the fix is *not* "re-add Z" (Z is present); it is making Z materially affect the output. Introduction point (fresh drift vs long-standing) is a git-blame question for the fix beat.

### Bucket C — DIAGNOSTIC-RESIDUE (0 current failures, but the bucket is real)

The `wave2/wave3/phase_1_6` campaign history created a class of **investigative/measurement** tests. Characterization: `phase_1_6_f3_phase_3_diagnostic` is **pure measurement** — 3 tests, **0 asserts**, 46 `println!`s, header *"No behavior change — pure measurement. Output feeds `docs/audits/terrain_erosion_seamless_diagnostic_2026-04-24.md`"* — it **cannot fail** (and passes in 32.7 s release). Others are mixed (`phase_4_diagnostic` 4 asserts, `f4_b_3_d_3_diagnostic` 7 asserts). **None of these produced a failure in the runs reached.** So the third bucket is **structurally present but not a source of the current failing surface** — the failures are Buckets A and B. *Disposition note for later:* the assert-free measurement tests are arguably mis-shaped as `#[test]`s (they gate nothing and cost 30 s+); converting them to explicit benches/tools is a hygiene option, not an E3 requirement.

---

## §2. Prior-campaign linkage (Step 3) — all FRESH post-campaign drift, not logged known-debt

The `wave2_*`/`wave3_*`/`phase_1_6_*` targets came from prior terrain remediation campaigns that **closed green**. The failures are **later deliberate changes the tests were never re-run against**, with datable causes:
- **Amplitudes** (Target-B, Phase 1.6-F.4.B.2.B) — in-source "// was 50 (×3)" markers.
- **chunk_size/resolution** — commit `32a3f28ad` (2026-04-24), after the structure test's `16f5e0ec0` (2026-02-21).
- **Climate modulation** — commit `447367c15` (F.3.B, 2026-04-30), after the halo test's `2de78f3e1` (2026-04-23).
- **Fallback biomes** — documented in-source.

None is a known-failing item deferred by a prior campaign with a logged rationale; all are **drift** (green-at-close, lagged a later change). The scatter real-defect (B.1) is the exception whose introduction point is undetermined (git-blame in the fix beat). **Consequence:** the sheer volume (45+) confirms the terrain full suite isn't CI-gated — the recon's count correction is not just "8 was low," it's "the suite hasn't been run whole in a while."

---

## §3. E3's materials half (Step 4) — GREEN; terrain↔material path wired

- **`astraweave-materials` = VERIFIED-PRODUCTION** ([ROADMAP_R0B_STATE_MAP.md:168](../current/ROADMAP_R0B_STATE_MAP.md#L168)): live (consumed by `astraweave-render` `Cargo.toml:46`, `renderer.rs:16`), **250/0 tests**, non-stub (Node/Graph/MaterialPackage/`compile_to_wgsl`/BRDF). No E3 work needed on materials.
- **Terrain↔material path is wired+working:** authoring (editor `terrain_panel.rs` splat params, `TerrainMaterial` presets) → per-vertex `material_ids`/`material_weights` (`terrain_integration.rs`) → 32-layer splat shader (`terrain_material_manager.rs`, `pbr_terrain.wgsl`, `TerrainMaterialGpu`).
- **Consistent with the E4 finding:** terrain materials flow through the **same raw PNG→RGBA8 path** (`img.to_rgba8()`, 1024² upload, no GPU-compressed format). **Terrain-material authoring has no cook dependency** — v1.0 authors on raw textures. Cooking remains the post-v1.0 optimization E4 deferred.
- **Gaps beyond tests (not E3 blockers):** no dedicated editor UI to assign material slots per biome/chunk (assignment is preset-driven); no end-to-end editor→vertex→splat→shader integration test. The wiring itself is complete; these are ergonomics/coverage, not broken paths.

---

## §4. Proposed E3 scope (Step 5) — tiered, for ratification

E3 = "terrain VP + materials VP." Materials is already VP (§3). So E3 = **get `terrain` to green**, which the inventory shows is **overwhelmingly cheap test-rot + one real bug**. Proposed tiers (mirroring M1.4's a/b/defect split):

- **E3.a — Test-rot batch (bulk, mechanical, ~40 tests).** Update the stale assertions to canonical values: amplitudes (150/480/12.5), chunk_size/resolution (512/96), fallback biome/layer (sand / Desert), vegetation.density, and **regenerate the golden/fingerprint snapshots** against current noise output. Confirm each *canonical value is correct, not merely current* (the "is 512 canonical?" check — it is, per the render precedent; the goldens must be regenerated from verified-correct output, not blindly re-baked). The structure test (A.5) gets the chunk_size 256→512 fix. Largest tier by count, lowest risk.
- **E3.b — Contract-change disposition (1 test: halo A.6).** Not a value-update — **reframe** (assert halo-path determinism) **or remove** (legacy-identity no longer holds post-F.3.B). A small per-test judgment; director-ratifiable which way.
- **E3.c — Real-defect (1 bug, 2 tests: scatter-Z B.1).** The genuine E3 work: make the Z chunk coordinate materially affect scatter (pin the mechanism first — weak ×1 multiplier vs RNG seed-sensitivity — then fix; do **not** paper over by weakening the test). This is the one "surface, don't paper over" item. Scope: 1 localized fix + verify both tests + a visual/property check that Z-variation is real.
- **E3.d — (optional hygiene, not required for VP) Diagnostic-residue disposition.** Convert the assert-free measurement `#[test]`s to benches/tools so the terrain suite is cheap enough to CI-gate — which is what would have caught this drift. Propose deferring unless the director wants the CI-gating story closed as part of E3.

**Why terrain reaches VP cheaply:** 43 of 45 failures are mechanical test-rot from deliberate, in-source-documented changes; 1 is a localized real-defect. No systemic terrain-code breakage. The dominant risk is **volume** (regenerating ~16 goldens + ~27 assertion updates), not difficulty.

**Open question for ratification:** E3.a's golden regeneration bakes current noise output as the new truth — that is correct **iff** the Target-B amplitudes are the intended final tuning (they are documented as such). Confirm no further amplitude retune is pending before regenerating goldens (else they rot again).

---

## §5. Evidence ledger

| Claim | Evidence |
|---|---|
| Real count ≫8 | ≥45 failing tests across ≥13 targets (debug run + release confirmation); "8" = the `mutation_resistant` target alone |
| Amplitude drift = test-rot | `noise_gen.rs:183/193/203` ("was 50/80/5" markers); golden panics (expected −0.05/35.8, got 420/500) |
| chunk_size drift = test-rot | `lib.rs:147` = 512; `structures.rs:399` index formula; test hardcodes 256 |
| Fallback drift = test-rot | `texture_splatting.rs:169-170` (sand), `biome_blending.rs:70-74` (Desert) — both documented |
| halo = contract-change test-rot | `lib.rs:570-645` F.3.B modulation; commit `447367c15` post-dates test `2de78f3e1` |
| scatter = real-defect | `lib.rs:244,252` seed `+ x*1000 + z` (Z ×1 too weak); 2 tests fail all 3 checks; no stale hardcode in test |
| structure = test-rot (not the bug its panic implies) | test:686 passes chunk_size=256 for a 512 chunk |
| diagnostic-residue = 0 failures | `phase_3_diagnostic` 0 asserts/pure measurement, passes |
| materials = VP, path wired | `ROADMAP_R0B_STATE_MAP.md:168`; render `Cargo.toml:46`/`renderer.rs:16`; `pbr_terrain.wgsl` |
| fixtures un-mutated | `git status` clean post-run |

*Recon complete. E3 scope (esp. the E3.b halo disposition and the E3.c scatter real-defect) awaits director ratification. No code changed.*
