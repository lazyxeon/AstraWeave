# F.1 Execution Report — FluidSystem Correctness Repair, Solver Consolidation, First Real Baselines

**Document version**: 1.1
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

**Revision history**

| Version | Date | Change |
|---|---|---|
| 1.0 | 2026-06-11 | F.1 execution report; branch `campaign/fluids-f1`, commits `e22e7bd0a..e4c98bb7f` + this report |
| 1.1 | 2026-06-11 | F.1.1 hotfix addendum: FluidRenderer had never constructed (2 pipeline/binding mismatches fixed), clean 12 s demo run captured, `gpu_renderer_smoke` closes the renderer coverage gap, v1.0 "should now actually run" inference corrected |
