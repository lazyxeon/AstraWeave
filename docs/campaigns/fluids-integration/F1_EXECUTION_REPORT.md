# F.1 Execution Report — FluidSystem Correctness Repair, Solver Consolidation, First Real Baselines

**Document version**: 1.3
**Execution date**: 2026-06-11
**Branch**: `campaign/fluids-f1` (base: `8e1505dd8`)
**Commit range** (actual hashes, in order):

| Commit | Content |
|---|---|
| `e22e7bd0a` | F.0 audit report committed (branch baseline housekeeping) |
| `7fb4e17b0` | WI-1/2/3/5: FluidSystem correctness repair + first GPU-execution tests + 5 pre-existing SDF defect fixes |
| `c3f19e31e` | WI-4: solver consolidation (UnifiedSolver deleted, DFSPH/IISPH variants removed, `experimental` feature, validation honesty) + WI-7.1 serde fix |
| `e4c98bb7f` | WI-6/WI-7: GPU timestamp instrumentation, first recorded baselines, demo dead-UI removal, doc-correction batch |
| *(this file's commit)* | F.1 execution report |

**Headline**: all Must-Fix items in F.1 scope are closed; **the F.1 GPU tests proved `FluidSystem::step` had never successfully executed on any device** — beyond the three audited defects, five additional blocking SDF/bind-group defects were discovered and fixed the moment the first execution test ran. The crate is net **−614 LoC** (2,033+/2,647−) and now carries its first GPU-execution tests, first physical-invariant envelopes, and first recorded production benchmarks.

---

## Pre-Flight (recorded before any change)

**HEAD check**: `git log --oneline -5` →

```
8e1505dd8 Engine-Health-Audit.2026-06-10: full health audit + workspace-wide doc reconciliation
18af1519d add additional Bash and PowerShell commands for cargo operations and web fetching
017ada12c Net-Trio-Remediation.W.4: docs closeout
c05aa0251 Net-Trio-Remediation.W.3.3-fmt: rustfmt normalization of family3 client-binary test
eb9977b88 Net-Trio-Remediation.W.5.3: document server->client asymmetric-trust design (closes §5 finding 5)
```

HEAD was still `8e1505dd8` — **identical to the F.0 audit commit**; `git log 8e1505dd8..HEAD -- astraweave-fluids/ examples/fluids_demo/` was empty. Every audit claim held by construction. Working tree clean except the untracked F.0 report.

**Defect re-verification** (each confirmed by direct read at this commit):
1. `fluid.wgsl:57` — `particles_dst` declared `// Reserved for full state copy if needed`, written by **no kernel**; `lib.rs:700-729` bind groups swap buffers; `lib.rs:1044-1048` per-frame alternation; `lib.rs:404-412` buffer 1 created empty. CONFIRMED.
2. Zero `flags` matches in `fluid.wgsl`; `particle_flags` (`lib.rs:311, 659-663, 854, 906, 989`) in no bind group. CONFIRMED.
3. `lib.rs:1215` `map_async` issued inside `step()` before the caller submits; result feeds `self.iterations` (`:1207`). CONFIRMED.

**Builds/tests (verbatim)**: `cargo check -p astraweave-fluids` → `Finished 'dev' profile ... in 32.18s` (PASS, zero warnings); `--features parallel` → `Finished ... in 6.78s` (PASS); `cargo test -p astraweave-fluids` → `2480 passed; 0 failed` (lib) + `99 passed` (integration) + `6 ignored` (doc-tests). All green as expected.

**Deviation #0 — missing campaign plan**: `docs/campaigns/fluids-integration/CAMPAIGN_PLAN.md` does not exist (only the F.0 audit was present). The F.1 brief itself embeds the gate decisions (Path B; Q1 carve-out accepted; Q3 consolidation authorized; WI-1 Option 1 pre-endorsed; Q7 dev-machine labeling), so the brief was treated as the authoritative gate record. The determinism carve-out wording in `docs/architecture/fluids.md` §0 was authored from the F.0 WS4 posture-3 text; **reconcile it against the canonical wording when CAMPAIGN_PLAN.md is created** (handed to F.2 as an open item).

---

## WI-1 — Ping-pong repair (Must-Fix #1) ✅

**Design chosen: Option 1 (delete the ping-pong), as pre-endorsed.** No counter-evidence emerged: the seven kernels are coherently designed as in-place mutation with dispatch boundaries as barriers, and no kernel requires pre-step state that in-place mutation corrupts. Implementation: single `particle_buffer` (the empty second buffer deleted), single `particles_bind_group`, `frame_index % 2` bind-group alternation removed (the field remains for staging-buffer alternation and as the step counter), `particles_dst` binding deleted from `fluid.wgsl`, `get_particle_buffer()` returns the one true buffer with its aspirational comment replaced by the real contract, spawn/reset write the single buffer. The vacated group-1 binding-1 slot was reused for WI-2's flags buffer.

**Recorded, not fixed (per brief; the carve-out covers it):** `integrate` reads neighbor `position`/`predicted_position` from the same buffer it writes mid-dispatch (`fluid.wgsl` integrate kernel, vorticity/XSPH neighbor loops) — an unsynchronized intra-dispatch read/write race, defined-but-indeterminate in WGSL, contributing to the documented non-determinism. Any future deterministic-GPU effort must double-buffer `integrate` specifically.

## WI-2 — Despawn made real (Must-Fix #2) ✅

**Design chosen: (a) flags bound + per-kernel early-out**, with one addition: despawn also **parks** the particle at y=−10,000 (`DESPAWN_PARK_Y`) with zeroed velocity/color, so naive instanced renders of the full buffer don't draw a frozen ghost. **Design (b) compaction was rejected** for exactly the reason the brief anticipated: the swap-source position would come from the documented-stale CPU cache, making compaction unsound during simulation. `particle_flags` is bound at group 1 binding 1 (read-only storage); every per-particle kernel early-outs on flag==0; `build_grid` never inserts inactive particles into the neighbor grid (so they are invisible to all neighbor searches, not merely frozen). The stale-CPU-cache caveat on `despawn_region` is now precisely documented: region *membership* is approximate (exact only at spawn/reset), but once despawned the effect is exact.

## WI-3 — Readback ordering and defined iteration semantics (Must-Fix #3) ✅

**Design: deferred readback via an explicit per-buffer state machine** (`StagingState`: `Idle → CopyRecorded → MapRequested → Idle`), pumped at the start of every `step()`. `map_async` is issued **only** for a buffer whose copy was recorded in a *previous* step — i.e., strictly after submission under the now-documented contract ("the encoder passed to each `step()` must be submitted before the next `step()` call"). Map-completion is signaled via an `Arc<AtomicU8>` callback (OK/ERR/pending); a buffer still in flight simply skips that frame's copy (feedback gets one frame staler — defined behavior).

**Defined semantics (documented on `step()`)**: `self.iterations` for frame N derives from the smoothed density error of the most recently harvested frame — **normally N−2** (a two-frame lag rather than the brief's sketched one-frame: map is requested at the start of step N+1, harvested at the start of step N+2; this is the race-free variant requiring no new API/hook). `step_with_budget`'s wall-clock blending remains, documented as presentation-side LOD. The deeper truth is also documented: the error value itself is computed from non-deterministic GPU state, so iteration counts may differ between identical runs — tolerable under the gate-Q1 carve-out because the mechanism is now well-defined and race-free.

## Unplanned: five pre-existing SDF/dispatch defects (discovered by WI-5, fixed under mandate #6)

The first-ever execution test immediately produced a cascade of hard validation errors and a field-poisoning bug proving **`FluidSystem::step` could never have run** — on any device, including the demo's (the demo had evidently not been launched since these landed; "demo-validated" in F.0 was true only of the crate's history, not its present):

1. **WGSL `JfaParams` was 32 bytes vs a 16-byte host buffer** (`padding: vec3<u32>` has 16-byte alignment in uniform space; host struct is `u32 + [u32;3]` = 16) → validation failure on every JFA dispatch. Fixed: three scalar `u32` pads.
2. **Every `step()` pass bound only a subset of the 4 pipeline-layout bind groups** (Predict bound 0/1/3; ClearGrid only 0; mix_dye 0/1/2) → "expects a BindGroup at index N" on every dispatch. Fixed: all four groups bound in every pass.
3. **JFA ping-pong inverted**: init seeds texture B, but the first JFA step read texture A (uninitialized zeros, whose texels decode as "valid seed at the origin"), destroying the seed; finalize then wrote its result into B while the fluid shader samples A. Fixed: `data_in_b` tracking; finalize reads wherever the data landed and a B→A blit covers the odd-step-count case (textures gained `COPY_DST`).
4. **SDF init voxelized all 128 fixed-size buffer entries**: zeroed entries have a zero inverse-transform, making `sd_box(0,0) = 0` ⇒ *every voxel* seeded as "inside an object". Fixed: `SdfConfig.object_count` (replacing the dead `padding` field) + `SdfSystem::set_object_count`, plumbed from `FluidSystem::update_objects`, which also now feeds `SimParams.object_count` (replacing the audited hardcoded 0 — dynamic-object collision in `compute_delta_pos` is now reachable as designed).
5. **SDF dispatch covered half the volume in z**: `(res/8)³` workgroups against a `(8,8,4)` workgroup size left voxels with z ≥ res/2 (world z > 0) permanently unwritten. Fixed: `res/4` z-workgroups at all three dispatch sites.

Empirical before/after: pre-fix, frame 0 slammed the entire particle field into the world corner at ~2,900 m/s (max speed² 8.3×10⁶); post-fix, frame 0 max speed² = **0.025** and the fluid settles normally.

Also fixed/documented along the way: `FluidSystem::new`'s previously undocumented device requirement (`TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES`, which the demo requests) is now in the constructor docs; the constructor defaults (`target_density: 12.0`, `viscosity: 10.0`) were measured to be an unvalidated, violently jittering configuration (mean speed² ≈ 220 at frame 300) — documented in the test helper; tests pin the demo-canonical parameter set instead (**deviation, justified**: it is the only production-exercised configuration).

## WI-5 — First GPU-execution and physical-invariant tests (Must-Fix #9) ✅

`astraweave-fluids/tests/gpu_execution_tests.rs` (5 tests, all passing on the dev GPU):

1. **`gpu_smoke_construct_and_step`** — first test ever to call `FluidSystem::new` + step on a real device; asserts no validation panic + finite readback.
2. **`gpu_containment_invariant_120_frames`** — every particle inside the shader's world box, every float finite, after 120 fixed-dt frames.
3. **`gpu_settling_envelope_300_frames`** — gravity acted (mean height drops) + no explosion. **Empirical threshold**: mean speed² < 100.0; observed quasi-steady ≈ 20–27 across runs (GTX 1660 Ti, demo params, 4,096 particles — the pile keeps sloshing; XSPH at 0.01 damps slowly); explosion classes measured during debugging were 10⁵–10⁷, so 100 gives ~4× headroom above steady state and 3+ orders below the failure class.
4. **`gpu_despawn_removes_particles_from_simulation`** — active_count drops; parked particles remain bit-stationary across 30 further frames (proving every kernel skips them — gravity would move them otherwise); parked count equals despawned count.
5. **`gpu_visible_state_advances_every_frame`** — the Must-Fix #1 regression guard: a tracked free-falling particle's height decreases across consecutive single-step readbacks (under the old ping-pong, the visible buffer alternated between two half-rate states and this fails).

**Loud-skip**: skips print `SKIPPED (...): <test> did NOT exercise the GPU` to stderr (they still count as passes — the harness offers no clean alternative without nightly custom test frameworks; the loud message is the visibility mechanism, as the brief allowed). **Serialization**: the five tests share a `static Mutex` — five simultaneous wgpu devices on the Max-Q adapter distort the timing-coupled adaptive iterations enough to trip the settling envelope (one flake observed under 5-way contention, none serialized; documented in-file).

## WI-4 — Solver consolidation per gate Q3 ✅

**Deleted (vapor):** `unified_solver.rs` wholesale (982 LoC — `UnifiedSolver::step` was `self.frame_count += 1;` under a 10-step comment; verified self-contained, zero in-crate or workspace consumers; root re-export removed). `SolverType::DFSPH`/`IISPH` variants (research.rs) — quality tiers High/Ultra/Research remapped to PCISPH with in-code provenance comments; match arms in `pcisph_system.rs`/`warm_start.rs` and ~12 tests updated; the tested CPU helpers in `simd_ops` kept per brief. The orphan `src/shaders/viscosity_morris.wgsl` (644 LoC, no `include_str!` consumer) deleted; the phantom `viscosity_implicit.wgsl` reference and all phantom `ResearchFluidSystem` doc references corrected (the research.rs header now states plainly that no such type exists).

**Gated behind new `experimental` feature:** `pcisph_system`, `multi_phase`, `warm_start`, `particle_shifting`, `turbulence`, `viscosity_gpu` — tests preserved and passing under `--features experimental` (2,448 lib tests). `PcisphSystem`'s header now states the fixed-iteration reality (an "HONEST STATUS" block: no convergence check exists; `IterationState` never written; the "<0.1% density error after convergence" claim removed) and the empirical δ `sum_term = -0.5` is documented as a known approximation (T4 work).

**Honesty fixes (`validation.rs`):** `ReferenceData::load_csv` now actually parses CSV (`t,x,y,z[,vx,vy,vz[,density]]`, comment/header tolerant, real error reporting) — the brief allowed an `Err(unimplemented)` stub but the struct was simple enough for the real thing; 2 tests added (roundtrip + missing-file-errors, the latter pinning that fake-success can't return). Divergence metrics: **NaN, not 0.0** (deviation from the brief's `Option` suggestion — NaN preserves the `Copy + Default + bytemuck`-friendly layout while making "perfect by omission" impossible; documented on the fields + a pinning test).

## WI-6 — Instrumentation and first recorded baselines ✅

**Instrumentation**: `FluidSystem::enable_gpu_timing(device, queue)` (requires `Features::TIMESTAMP_QUERY`; returns false otherwise) + `read_gpu_timings(device)` (blocking; diagnostics/bench use, documented). Seven spans (`GPU_TIMING_SPANS`): `sdf` (bracketing init→finalize via the new `SdfSystem::generate_timed`), `predict`, `clear_grid`, `build_grid`, `pbd_iterations` (whole lambda/delta loop), `integrate`, `mix_dye`. One wgpu lesson encoded: `timestamp_writes` with neither edge set is a validation error — interior passes of a multi-pass span pass `None`.

**New bench `benches/fluid_baselines.rs`** — measures *production* code only (explicitly contrasted with `fluids_adversarial`'s mocks). Results recorded in **`docs/masters/MASTER_BENCHMARK_REPORT.md` v5.58** (the first fluids entries ever; machine context: i5-10300H, GTX 1660 Ti Max-Q, 31.8 GB, Win 11, labeled min-spec-class per gate Q7):

| Measurement | 10k | 20k | 50k |
|---|---|---|---|
| `step` wall (encode+submit+wait) | 5.62 ms | 8.19 ms | 20.72 ms |
| GPU total (per-pass sum, median) | 4.889 ms | 7.046 ms | 19.082 ms |
| └ `pbd_iterations` (×8, adaptive ceiling) | 3.315 | 4.946 | 14.521 |
| └ `sdf` (full JFA regen/frame) | 0.886 | 0.910 | 0.968 |
| └ `integrate` / `mix_dye` | 0.440 / 0.216 | 0.798 / 0.350 | 2.443 / 1.081 |

| `WaterVolumeGrid::simulate` (half-full basin) | 32³ | 64³ | 128³ |
|---|---|---|---|
| time/tick | 0.551 ms | 13.83 ms | 206.1 ms |

**Decision-grade findings** (also in the master report):
1. PBD iterations dominate (68–76%), partially refuting F.0's SDF-dominance inference — but the flat ~0.9 ms/frame SDF regen alone consumes ~45% of the proposed 2 ms budget; **SDF caching when no colliders moved is the leading optimization candidate (NOT implemented in F.1, per brief)**.
2. **The F.0-proposed 2 ms GPU budget is unachievable at 8 iterations on min-spec class** (10k ≈ 4.9 ms GPU); iteration count is the lever. The roadmap's "50–100k @ 60 FPS PBD" tier claim is refuted on this hardware (50k ≈ 19 ms GPU).
3. **Voxel sparsity is mandatory for F.3** beyond small volumes (64³ dense = 13.8 ms/tick vs the 1 ms CPU budget; 32³ fits at 0.55 ms).

## WI-7 — Hygiene and doc-correction batch ✅

1. **serde feature** — removed entirely; serde is an unconditional dependency (the 9 identical `cfg_attr` derive sites converted to plain derives; the feature gated zero `cfg` sites while 5 modules used serde unconditionally). All five matrices compile, **including `--no-default-features` for the first time**. Features are now `parallel` + `experimental`, both default-off (note: `default = ["serde"]` → `default = []` is a behavior-visible change for any future consumer that assumed serde was a feature; none exist today).
2. **`docs/architecture/fluids.md` v1.3** — new §0 "F.1 Revision Notice" (trace-error corrections D1/D3/D2-class incl. phantom `ResearchFluidSystem` and the 9th orphan shader; F.1 code deltas; the **binding determinism carve-out policy**, gate Q1); invariants 21–23 rewritten (the old "2-entry ping-pong" invariant had canonized a defect); §11 closures (production-wiring → Path B decided; consolidation/naming-collision questions → resolved); revision history added. Full body re-verification queued post-campaign (noted in the doc).
3. **`docs/architecture/net.md`** — fluids determinism carve-out note (particle state excluded from `world_hash`/replay/replication; 8 MB/tick infeasibility math).
4. **CLAUDE.md** — phantom `character_controller.rs` path corrected (controller is `astraweave-physics/src/lib.rs:424-535`).
5. **Render trace** — the phantom "astraweave-fluids → water.rs" upstream row corrected with provenance.
6. **`docs/src/api/fluids.md`** — rewritten honestly (was 261 lines documenting ≥13 nonexistent types from the `28bc94f21` wiki sweep; now documents the real F.1-state API with a correction banner).
7. **`FLUIDS_MUTATION_TESTING_REPORT.md`** — staleness banner (pre-refactor module list; the 45 excluded "GPU-dependent" mutants turned out to shield five blocking defects; re-run queued — NOT re-run in F.1, per brief).
8. **Demo cleanup** — orphan `scenarios/{splash,waterfall}.rs` deleted (were never declared in `mod.rs`); dead UI removed: "Drag Force" slider, "Show Foam" checkbox, quality-preset buttons + `target_particle_count` (settable, never applied), right-drag help text, the `mouse_right_pressed` handler, and `quality_preset` (its only reads were the deleted buttons — slight scope extension, same dead-UI class). Viscosity slider relabeled `Vorticity ("viscosity")` with an honest hover-text (audit H6). One audit nuance: H6 described a `pressure_multiplier` *slider*; in fact it was config writes only — the writes were removed along with the **dead `SimParams.pressure_multiplier` uniform itself** (Rust + WGSL, replaced by a pad; struct stays 64 B; the demo/lab writes dropped). The same-named fields in the *separate* editor/serialization/research/pcisph type families were deliberately left alone (pcisph genuinely reads its own; scope discipline).
9. **`FluidRenderer`** — four production-path `DEBUG:` `println!`s stripped.

---

## Verification Gate (all items, verbatim results)

| Item | Result |
|---|---|
| `cargo check -p astraweave-fluids` (default) | `Finished 'dev' profile ... in 2.35s` ✅ |
| `--features parallel` | `Finished ... in 5.56s` ✅ |
| `--features experimental` | `Finished ... in 5.75s` ✅ |
| `--no-default-features` | `Finished ... in 2.98s` ✅ (could not compile pre-F.1) |
| `--all-features` | `Finished ... in 3.08s` ✅ |
| `cargo test` (default) | `2259 passed` (lib) + `5 passed` (GPU) + `99 passed` (integration) + 6 doc ignored — **0 failed** ✅ |
| `cargo test --features experimental` | `2448 passed` + `5 passed` + `99 passed` — **0 failed** ✅ |
| `cargo clippy -p astraweave-fluids --all-features -- -D warnings` | `Finished ... in 7.63s` (zero warnings) ✅ |
| `cargo build -p fluids_demo --release` | exit 0 ✅ — **owner visual sanity check requested** (the agent cannot view the window; post-WI-1 every particle integrates every frame, so the fluid should look *more* coherent, and post-SDF-fix the demo should now actually run at all) |
| `cargo check --workspace` | `Finished ... in 11.55s` ✅ (only pre-existing deferred warnings: 1 aw_editor warning, nalgebra future-incompat note; the UnifiedSolver deletion stranded zero consumers) |
| Benchmarks recorded | MASTER_BENCHMARK_REPORT v5.58 ✅ (first fluids entries, machine context included) |
| Scope wall (`git diff --stat main..HEAD`) | Only `astraweave-fluids/`, `examples/fluids_demo/`, and the named documentation files. **Zero source edits** to physics/render/ecs/editor. ✅ |

Test-count delta accounting: pre-F.1 lib 2,480 → default 2,259 / experimental 2,448. Δ = −221 default: −189 gated experimental-module tests (recoverable via the feature), −35 deleted with `unified_solver.rs`, −2 DFSPH/IISPH-specific assertions folded, +5 validation tests added; experimental −32 vs old total = deleted UnifiedSolver tests + folded variants, +additions. No test was silently lost: every removal is attributable to a deleted module or deleted enum variant.

## Deviations / scope calls (none silent)

1. **CAMPAIGN_PLAN.md missing** → F.1 brief treated as the gate record (see Pre-Flight; carve-out wording to be reconciled).
2. **Five unplanned SDF/dispatch fixes** → mandated by CLAUDE.md #6 (pre-existing issues) and prerequisite to every other WI: the solver was unrunnable. All within the crate.
3. **GPU tests pin demo-canonical params**, not constructor defaults (the defaults are an unvalidated, violently jittering configuration — measured and documented). The defaults themselves were left untouched (changing them is a behavior decision for the owner; flagged as an open item).
4. **GPU tests serialized** (static mutex) after one observed contention flake.
5. **Two-frame (not one-frame) readback lag** in WI-3 — the no-new-API, provably race-free variant.
6. **NaN instead of `Option`** for the divergence metrics (layout-preserving, with doc + pinning test).
7. **`quality_preset` demo field also removed** (same dead-UI class as the enumerated items; its only readers were the deleted buttons).
8. **`load_csv` fully implemented** rather than stubbed-with-error (both allowed; the real parser was ~40 lines).

## Open items handed to F.2+

| Item | Owner phase | Notes |
|---|---|---|
| Physics water reconciliation (flat-plane buoyancy / `EnvironmentManager::WaterVolume` / `add_water_aabb` stub → facade) | F.2 | Untouched per scope wall |
| Facade design + `CFluidVolume` + ECS/host-driven GPU pattern | F.2 | Per Path B plan |
| **Q7 budget re-ratification with real data** | F.2 gate | 2 ms GPU is unachievable at 8 iterations on min-spec; decide iteration policy (cap at 2–4?) or budget |
| SDF caching when no colliders moved | F-later | Flat ~0.9 ms/frame measured; leading optimization candidate |
| **Voxel sparsity (`active_cells`)** | F.3 (gating) | 64³ dense = 13.8 ms/tick; 32³ = 0.55 ms fits budget |
| `WaterGate`/`CellFlags` dead flags (Must-Fix #6) | F.3 | Deliberately untouched per brief |
| Mutation-testing re-run (post-refactor + post-F.1) | future | Staleness banner placed |
| `integrate` in-place neighbor read/write race | T4 / deterministic-GPU work | Documented in WI-1 section + carve-out covers |
| PCISPH real convergence readback + analytic δ | T4 | Header now states honest status |
| Constructor-default sim params are unvalidated | owner call | Consider making demo params the defaults |
| CAMPAIGN_PLAN.md creation/backfill + carve-out wording reconciliation | owner / F.2 | |
| Demo visual sanity check | **owner, now** | Binary builds; agent cannot verify visuals |
| fluids.md full body re-verification pass (post-campaign) | trace maintenance | §0 corrections bridge until then |

---

## F.1.1 Hotfix Addendum — FluidRenderer Has Never Run (2026-06-11)

**Trigger**: the owner's post-F.1 demo launch (the visual sanity check v1.0 requested) hit a fatal startup panic: `SSFR Depth Pipeline ... Shader global ResourceBinding { group: 0, binding: 0 } is not available in the pipeline layout / Visibility flags don't include the shader stage`.

**Honest correction of v1.0.** Version 1.0 inferred the demo "should now actually run" after the compute-path repairs. It could not have: `FluidRenderer::new` panics at pipeline creation, *before* any compute work — meaning **this render-side panic, not the five SDF/compute defects, was the actual first blocker on every demo launch**, and `FluidRenderer` (like `FluidSystem` before F.1) had never successfully constructed on any device. The gap existed for the same reason twice over: F.1's verification gate used *built-not-run* evidence for the demo (`cargo build` exit 0), and the new GPU test suite covered the solver but added **no renderer test** — WI-5's brief specified solver tests and the agent did not generalize the lesson to the sibling subsystem in the same crate. The F.0 audit's "demo-validated" framing for `FluidSystem` compounded this: nothing in the crate's GPU surface had ever actually been validated by execution. F.1.1 closes both the defects and the coverage gap.

### H-1 — Full pipeline × binding mismatch audit

All four `FluidRenderer` pipelines (SSFR depth, smooth compute, shade, secondary) were cross-checked: per-stage WGSL resource usage vs `BindGroupLayout` visibility and binding types; WGSL struct sizes vs buffer sizes (`CameraUniform` 304 B ✓ both sides, `SmoothParams` 32 B ✓); texture formats/usages vs declarations; vertex layouts vs shader inputs; dispatch vs workgroup size. **Two mismatches found, both fixed**:

| # | Pipeline | Binding | Defect | Fix |
|---|---|---|---|---|
| 1 | SSFR Depth (and Secondary, which shares the layout) | group 0 binding 0 (camera uniform) | Layout visibility `VERTEX` only, but `ssfr_depth.wgsl::fs_main` reads `camera` (view_inv/cam_pos/view_proj) for sphere-surface depth reconstruction → pipeline creation panic (the reported error) | `visibility: VERTEX \| FRAGMENT` (`renderer.rs`). Safe superset for the shared secondary pipeline, whose fragment ignores the uniform |
| 2 | SSFR Shade | group 0 bindings 4+5 pairing | `ssfr_shade.wgsl:104` sampled `scene_depth` (`texture_depth_2d`) with `default_sampler` (a **Filtering** sampler); wgpu statically rejects depth-texture × filtering-sampler pairs → this would have been the *next* startup panic after fix #1 | Sample with `nearest_sampler` (NonFiltering) in the shader |

Verified clean in the same audit (no fix needed): smooth pass (params buffer 32 B = WGSL struct; depth/storage texture types; 16×16 dispatch), shade bindings 1/2/3/6 (filterable/non-filterable sample types consistent with their samplers and the R32Float source), secondary vertex layout (48 B stride, 3 attrs), depth-pass `targets: &[]` + `frag_depth`, all texture usage flags, both pass-order texture-usage transitions.

### H-2 — Clean run (verbatim)

`cargo run -p fluids_demo --release` equivalent (release binary launched, killed after 12 s of runtime):

```
RESULT: still running after 12s -> killed (no startup crash)
--- stderr ---
(empty)
```

**Zero wgpu validation errors, zero panics** on the first post-fix run; no iteration was required. The Steam Vulkan layer JSON loader errors the owner saw interactively did not reproduce in the redirected-stderr capture — they are machine-environment noise (Steam overlay layer manifests), not project output, and are out of scope.

### H-3 — Coverage gap closed

`gpu_renderer_smoke` added to `tests/gpu_execution_tests.rs` (same loud-skip + serialization mutex pattern; 6 GPU tests total now): constructs `FluidRenderer` **headless against an offscreen Rgba8UnormSrgb target** (no surface), builds a well-formed `CameraUniform` from glam matrices, and renders one full frame (depth → smooth → shade → secondary) against a live, stepped `FluidSystem` — with construction and render each wrapped in explicit `push_error_scope(Validation)`/`pop_error_scope` assertions, so validation errors fail the test even if the global panic hook changes. The test fails on pre-F.1.1 code and passes post-fix. **Structural finding for F.4**: `FluidRenderer` required *no* restructuring to render offscreen — it never touches a surface — so the brief's contingency (offscreen-blocking API would threaten the `draw_into` integration) did not materialize.

### F.1.1 verification gate

| Item | Result |
|---|---|
| All pipelines create + 12 s demo run with empty stderr | ✅ (capture above) |
| `gpu_renderer_smoke` passing on dev GPU | ✅ (6/6 GPU tests) |
| `cargo test -p astraweave-fluids` default / `--features experimental` | ✅ 2,259+6+99 / 2,448+6+99, 0 failed |
| `cargo clippy --all-features -- -D warnings` | ✅ clean |
| Scope wall (`git status`) | ✅ exactly `renderer.rs`, `ssfr_shade.wgsl`, `tests/gpu_execution_tests.rs` + this addendum |

**Owner's visual sanity check remains the final step** — the sim now provably runs without errors, but only eyes can confirm the water looks like water.

---

## F.1.2 Hotfix Addendum — Demo Runtime Defects (2026-06-11)

**Trigger**: owner's post-F.1.1 session — resize/scenario crash (captured), dead left-click, and the visual report "smooth playdough … perfect spheres or perfect oblong spheres … no rendered surface."

### Root causes (every owner symptom traced to a specific never-worked defect)

| Symptom | Root cause | Fix |
|---|---|---|
| Resize/maximize crash (the captured 800×600-vs-1920×991 panic) | `State::resize` recreated `depth_texture` but **not `depth_view`** (still viewing the startup-sized texture) and refreshed `scene_view` from a **never-recreated `scene_texture`** | Both recreated on resize; debug assertions at frame start name the class at its source (H-1) |
| Scenario-switch crash | **Distinct second defect**: `OceanRenderer`'s `Uniforms` packed to 144 B vs the WGSL's 160 B (`vec2` 8-byte alignment gap + 16-byte struct round-up — the JfaParams family again). Every ocean draw failed validation: **the ocean scenario had never rendered a single frame** | Explicit-pad host mirror with documented offsets (H-2). The owner's crash was the resize defect reached *through* the ocean path; with H-1 fixed, the exercise run immediately exposed this one underneath |
| Abort cascade after the panic (`SurfaceSemaphores` + `STATUS_STACK_BUFFER_OVERRUN`) | Panic unwound through the live `SurfaceTexture`, whose teardown assert panicked again during unwind → abort | `catch_unwind` around frame encode; controlled `SurfaceTexture` drop; **one** clean `FATAL: frame encode panicked: …` + exit (H-4). Verified by a REAL panic (the ocean uniform, run 1: single message, exit 1, no cascade) — no synthetic injection needed |
| Left-click never spawns | **Never-could-work**: the input wire existed, but `spawn_particles` only draws from the despawn free-list and `reset_particles` marks ALL particles active — the free-list was empty for the demo's entire life | Lab init places a 2,000-particle reserve block in a far corner and despawns it immediately, populating the pool. Verified: `Spawned 50/50 … free pool now 1950`. Cap-adjacent spawns safe by construction (`min(requested, free)`) (H-3) |
| "Playdough" (matte, no surface to judge) | Two compounding never-rendered defects: (1) the background (clear + skybox) was drawn **only into `scene_texture`** — nothing ever drew a background on the swapchain, so the screen was zero-black around the fluid; (2) **the skybox itself had never rendered anywhere**: its sphere radius was 1500 vs the camera's zfar = 100 — every triangle far-plane-clipped since the day it was written. The SSFR refraction input was therefore flat gray and the visible composite "pearls in a void" | Background now renders to the swapchain and is copied into `scene_texture` as the true refraction source (`scene_texture` += COPY_DST); skybox sphere radius 50 (camera-centered, depth-write-off). Post-fix captures show sky-lit glassy droplets with live refraction/Fresnel (H-6b **defect fixed**) |
| "Perfect OBLONG spheres" | Billboard expansion used the **right-axis projected radius for both NDC axes**; NDC x/y scale differently by the aspect ratio, so impostors stretched horizontally by `aspect` on any non-square window | Project an up-axis edge too; per-axis NDC radii (`ssfr_depth.wgsl`). Capture-verified circular at 4:3, 16:9, and 11:7 (H-6c **defect fixed**) |
| "Spheres, not a fluid" | H-6a hypothesis (smooth output unconsumed) **REJECTED**: the shade pass binds the smoothed texture (code-verified F.1.1) and post-fix captures show adjacent particles fusing into smooth merged blobs. The real blockers were **sim-state**: lab init shipped `viscosity = 40` (a ×4 vorticity-confinement gain — F.1's measured permanently-jittering regime: the dam exploded into a gas) and `target_density = 10` against an achievable spawn density of ~3.85 (constraint maximally violated forever → permanent attraction churn), into a 22-unit-tall pillar scattering 20k particles across the whole 60×60 floor | Demo-side (in scope): `viscosity 40 → 0.5`, `target_density 10 → 4.2` (near spawn-packing equilibrium), dam reshaped wide-and-low (32×18×31). Result: dense fused foam with smoothly blended multi-particle surfaces (f0270/f0600) — fusion proven |

### The honest remainder (out of scope, reported verbatim)

A *calm glassy pooled basin* still does not form: the fluid settles into a lively fused foam, not a flat surface. The blocker is **solver-side and explicitly out of F.1.2 scope** ("no solver changes"): the XSPH viscosity blend is **hardcoded at 0.01 in `fluid.wgsl`** (the demo's "viscosity" slider drives only vorticity confinement), so the sim has almost no energy sink — F.1's own envelope measurements (quasi-steady mean speed² ≈ 20–27 ⇒ ~4.5 m/s perpetual agitation) predicted exactly this. Per the brief's stop rule, this is handed to F.4 rather than fixed here.

### DEFECTS (fixed) / TUNING (F.4) ledger

**DEFECTS fixed in F.1.2** — resize targets (H-1), ocean uniform layout (H-2), teardown cascade (H-4), never-populated spawn pool (H-3), missing background composite + never-rendered skybox (H-6b), aspect-stretched impostors (H-6c), gas-regime lab defaults (H-6a, demo-side). Capture evidence: before = run-1/run-3 sets; after = current `captures/` set.

**TUNING / F.4 ledger** (observed, deliberately not acted on):
1. **Expose the XSPH viscosity coefficient** (hardcoded 0.01, `fluid.wgsl` integrate) as a `SimParams` field + slider — the single highest-leverage step toward a calm pooled surface (solver change). Revisit `ε = 100` constraint softening at the same time.
2. Smooth-pass strength (radius 5 px, hardcoded `SmoothParams`, no setter) — adequate for the current foam look; revisit for large calm surfaces.
3. Depth-pass render radius hardcoded 0.5 (decoupled from `smoothing_radius`).
4. "No variation between particles": uniform fluid is not wrong; foam/spray/size/color variation are dormant features (secondary-particle system runs but spawns conservatively).
5. Ground/floor visual: there is no floor geometry; the pool sits on an invisible plane — a ground quad would anchor the scene.
6. Pre-existing clippy warnings in `astraweave-terrain` (7) and `astraweave-render` (1) surfaced while linting the demo — out of scope, noted for their owners. (Also: `cargo clippy -p <bin> -- -D warnings` propagates the deny to path dependencies — use crate-scoped lint runs.)
7. Default-scenario camera starts inside the droplet cloud; pulling the start distance back would read better.

### H-6d — Surface-tension pair (the slider works)

`--surface-tension=0.0` vs `0.9`, settled frame 600 (`captures/st_0_0_f0600.png`, `captures/st_0_9_f0600.png`): at 0.0, a fine dispersion of small uniform droplets; at 0.9, markedly larger coalesced blobs dominate. The slider's cohesion term is alive and legible — not broken; further shaping is F.4 sim-tuning.

### Capture infrastructure (permanent — F.4 will be evaluated with it)

- **F12**: capture next frame. **`--capture-frames N,M,…`**: capture by frame index. PNGs → `examples/fluids_demo/captures/` (gitignored).
- **`--exercise`**: scripted gate driver — resize 800×600→1600×900 (frame 80), →ocean (140), →lab (200), resize →1100×700 (230), center click-spawn (260), captures (30/120/170/270/320/400/600), clean exit 0 (640).
- **`--surface-tension=X`**: startup override for comparison pairs.
- Implementation: swapchain `COPY_SRC`, padded `copy_texture_to_buffer`, blocking map, BGRA→RGBA swizzle, `image` crate PNG.

### F.1.2 verification gate

| Item | Result |
|---|---|
| Maximize/resize/minimize-restore, scenario switch both ways incl. post-resize | ✅ exercise runs 2–5: exit 0, zero validation errors (run-1 baseline caught the ocean defect) |
| Left-click spawn at clicked location; cap-safe | ✅ `Spawned 50/50 … free pool now 1950`; clamped to sim domain |
| Clean exit 0; single clean message on mid-frame panic | ✅ exit 0 every post-fix run; H-4 verified by the real ocean panic (one FATAL line, exit 1, no cascade) |
| H-6a/b/c resolved with evidence | ✅ a=rejected-as-render-defect→sim-state fixed (captures); b=fixed (skybox+composite, captures); c=fixed (circular at 3 aspect ratios, captures) |
| Fusion capture | ✅ f0270/f0600: smooth merged multi-particle surfaces; calm-pool remainder reported out-of-scope (solver) |
| Surface-tension pair | ✅ `st_0_0_f0600.png` / `st_0_9_f0600.png` |
| `cargo test -p astraweave-fluids` default/experimental | ✅ 2,259+6+99 / 2,448+6+99, 0 failed |
| `clippy -p astraweave-fluids --all-features -- -D warnings` | ✅ clean; demo's own 4 warnings fixed (deps' pre-existing warnings ledgered) |
| Scope wall | ✅ `ssfr_depth.wgsl` + `examples/fluids_demo/**` + this addendum only |

### Captures for the owner's second visual pass (each with the one question it answers)

| Capture | The one question |
|---|---|
| `f0030_800x600.png` | Are impostors circular at 4:3, sky background present? |
| `f0120_1600x900.png` | Still circular at 16:9 (the oblong defect's old worst case)? Sky-lit glassy droplets instead of matte pearls-on-black? |
| `f0170_1600x900.png` | Does the ocean scenario — rendering for the first time ever — look like an animated water plane under sky? |
| `f0270_1100x700.png` | Mid-click-spawn: do adjacent particles fuse into smooth merged blobs (the SSFR chain working)? |
| `f0600_1100x700.png` | The settled state: fused foam — is this acceptable until F.4 exposes the real viscosity control? |
| `st_0_0_f0600.png` vs `st_0_9_f0600.png` | Can you see your surface-tension slider working (fine droplets vs large coalesced blobs)? |

---

## F.1.3 Hotfix Addendum — Spawn Visibility, Ocean Defect Pass (2026-06-11)

### Evidence-discipline correction (record first)

v1.2 closed H-3 (click-spawn) on a CPU log line — `Spawned 50/50 … free pool now 1950` is free-list bookkeeping, not proof anything simulated or appeared on screen, and the owner-facing capture list contained no click capture. The owner's "still nothing visible" was correct *simultaneously* with the log being correct, which is precisely why the rule now stands: **a log counter is never sufficient evidence for an interactive or visual feature — captures or GPU-readback assertions only.** F.1.3 was verified under that rule.

### H-1 — Respawn hypotheses, worked in order

| # | Hypothesis | Verdict | Evidence |
|---|---|---|---|
| 1 | Flag never flips back on reuse | **REJECTED** | `spawn_particles` writes flag=1 per slot to the GPU buffer (code) and the new `gpu_respawn_reactivates_particles` test **passed against pre-fix code**: spawn→despawn→respawn→30 frames→readback shows all 64 sentinel particles unparked, finite, and fallen under gravity |
| 2 | Render count stale / instance range misses scattered slots | **REJECTED** | Draw covers `0..particle_count` instances (all slots); reused indices are inside the range by construction |
| 3 | Position write misses / loses to the park | **REJECTED** | Per-slot offsets verified; readback shows respawned positions live |
| 4 | Scenario coverage undefined | **CONFIRMED (and the broader demo-UX root cause)** | See below |

**The actual root cause was demo-side visibility, proven by capture**: a click-spawn at y=5 materializes *inside* the existing 18,000-particle foam — and `Particle.color` **is never sampled by the SSFR pipeline** (the depth pass reads position only; the shade pass reconstructs one uniform water material from depth), so even a sentinel-orange burst is pixel-identical to its neighbors. The CPU spawned, the GPU simulated, the renderer drew — and nothing was distinguishable. The brief's framing finding lands twice over: the F.1.2 log line was true *and* meaningless.

**Demo fixes (all capture-verified)**: bursts now spawn **+10 above the foam line** and fall in visibly; sky-aimed clicks that miss the y=5 plane get a 25-unit-along-ray fallback (a click ALWAYS produces a visible response); spawn distance capped at 30 so the clump reads at screen scale; burst color orange (correct if/when SSFR becomes color-aware — ledgered); **ocean-mode clicks show an on-screen notice** ("Particle spawning is Laboratory-only…") instead of silently draining the shared pool — the ocean scenario does not render the particle system at all; pool-exhausted clicks also notice. Help text now says "(Laboratory only)". New crate test: `gpu_respawn_reactivates_particles` (7 GPU tests total) — it could not fail pre-fix because the crate path was never broken; it now pins reactivation against regression, which no test did before.

### H-2 — Ocean defect pass (its first-ever evaluation)

| Check | Verdict | Detail |
|---|---|---|
| 1. "Neon" → color space | **DEFECT, fixed** | The albedo/depth constants are Godot-port **sRGB-authored values** written raw into the sRGB-encoded swapchain — wgpu treats fragment output as linear and encodes again; double-encoding produced exactly the neon cyan. `srgb_to_linear()` now converts the four authored constants (`ocean.wgsl`) |
| 2. "Gelatin" → intended features live? | **One DEFECT fixed, one hypothesis corrected** | (a) The generated "normal maps" were **degenerate by construction**: constant `(128,128,noise)` — zero x/y tilt, noisy z — decoding to flat/sign-flipped normals; the entire normal-perturbation path was inert. Now a real tangent-space normal map derived from height-field finite differences (seamless-wrapping). (b) Initial read suspected the Fresnel view vector (`ocean_pos − world_pos`) — corrected on closer inspection: `ocean_pos` is assigned the camera position each frame (it double-duties as scroll center), so Fresnel was already view-dependent. Fragile pattern, noted in ledger |
| 3. Blend/depth/draw order | **CLEAN** | Loads color+depth, draws after the skybox, opaque output through ALPHA_BLENDING with α=1; depth-tests correctly. No fix needed |

**After** (`captures/ocean_after_f0170.png`): deep saturated ocean blue with visible wave mottling under a sky horizon. **Before** = the owner's own "neon sky blue gelatin" sighting (and F.1.2's f0170, regenerated each run — the owner is the before-witness; capture sets are per-run).

### TUNING ledger additions (F.4)

- **L8 — `Particle.color` is dead data in the SSFR pipeline** — the shade pass renders one uniform water material. Per-particle color/dye rendering (the demo sets colors everywhere; `mix_dye` heat exists) is a renderer feature decision.
- **L9 — Ocean dead uniforms**: 7 uploaded but never read by the shader (`beers_law`, `depth_offset`, `edge_scale`, `metallic`, `roughness`, `near`, `far`) — the Godot port dropped the features that used them (Beer's-law depth, edge foam). Implement or remove at F.4.
- **L10 — Ocean structure/taste**: no sun specular and no environment-reflection input declared (the Fresnel mixes two albedos — by design of the port); `ocean_pos` double-duties as camera position; `normal2_texture` shares `normal_texture`'s seed (identical maps, differently scrolled).
- **L11 — Exercise-driver click timing is scene-dependent** (early-collapse frames fill the frustum with foam); late sky-aimed clicks are the stable evidence pattern.

### F.1.3 verification gate

| Item | Result |
|---|---|
| Click-spawn capture pairs, two window sizes | ✅ `f0381/f0440` (1100×700) and `f0481/f0540` (800×600): tight 150-sphere clump at the aimed sky point one frame after click; gone (fallen/merged) ~60 frames later. Logs corroborate (`150/150`, pool 1850→1700) but are no longer the evidence |
| `gpu_respawn_reactivates_particles` | ✅ present and passing; **passed pre-fix too** — documented honestly: the crate path was never broken, so the brief's "must fail pre-fix" expectation is replaced by the test's regression-pinning role + the capture pairs as the fix evidence |
| Ocean three checks | ✅ 2 defects fixed (color space, degenerate normals) + 1 clean (blend/depth) + 1 hypothesis self-corrected (Fresnel) |
| Ocean-mode click behavior | ✅ visible notice; no pool drain |
| Tests default/experimental | ✅ 2,259+**7**+99 / 2,448+**7**+99, 0 failed |
| Clippy `-D warnings` (crate) / demo own warnings | ✅ clean / none |
| Scope wall | ✅ `tests/gpu_execution_tests.rs`, `examples/fluids_demo/**` only |

### Captures for the owner (each with its one question)

| Capture | The one question |
|---|---|
| `f0381_1100x700.png` → `f0440_1100x700.png` | Click at frame 380: does a tight clump appear high at the aimed point, then fall and merge by +60? |
| `f0481_800x600.png` → `f0540_800x600.png` | Same, after resizing to 800×600 — click-spawn works at both sizes? |
| `ocean_after_f0170.png` | Is this an ocean now (deep blue, wave mottling, horizon) rather than neon gelatin? |
| `f0620_800x600.png` | Settled basin sanity after all F.1.3 changes |
| Live check (no capture): click during the Ocean scenario | Does the yellow "Laboratory-only" notice appear instead of silence? |

---

**Revision history**

| Version | Date | Change |
|---|---|---|
| 1.0 | 2026-06-11 | F.1 execution report; branch `campaign/fluids-f1`, commits `e22e7bd0a..e4c98bb7f` + this report |
| 1.1 | 2026-06-11 | F.1.1 hotfix addendum: FluidRenderer had never constructed (2 pipeline/binding mismatches fixed), clean 12 s demo run captured, `gpu_renderer_smoke` closes the renderer coverage gap, v1.0 "should now actually run" inference corrected |
| 1.2 | 2026-06-11 | F.1.2 hotfix addendum: resize/scenario crashes (2 distinct defects), teardown cascade, never-wired click-spawn, never-rendered skybox + missing background composite, oblong impostors, gas-regime demo defaults; permanent capture infrastructure; DEFECTS/TUNING ledger for F.4 |
| 1.3 | 2026-06-11 | F.1.3 hotfix addendum: evidence-discipline correction (log lines ≠ visual evidence); respawn proven crate-correct by GPU readback, root cause = demo-side invisibility (`Particle.color` unused by SSFR — ledgered); spawn UX (sky-drop bursts, fallback, ocean-mode notice); ocean defect pass (sRGB double-encode + degenerate normal maps fixed, blend clean) |
