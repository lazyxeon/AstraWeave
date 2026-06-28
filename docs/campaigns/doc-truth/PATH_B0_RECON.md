# Path-B.0 — Broken Test-Target Enumeration + Fix Scoping · RECON (read-only)

**Status:** RECON COMPLETE — **NOT STARTED on source.** · **Branch:** `campaign/doc-truth-resume` @ `5004741f5` (clean) · **Mode:** read-only diagnosis. · **Authority:** the Session-A coverage CODE-FINDING (`cargo llvm-cov --workspace` blocked by pre-existing broken test targets that `cargo check --workspace` masks) + the D.2.B.A gate report.

## Why this exists
The coverage registry row (`coverage-weighted`) is the last genuinely-blocked PENDING-D2 claim. It is blocked because `cargo llvm-cov --workspace` must **build test targets**, and several are pre-existing-broken from library API drift — a failure mode `cargo check --workspace` cannot see (check does not build test targets). The blocker surfaced incidentally during the Session-A coverage attempt, which named only an approximate "≥3". This recon runs the true enumeration before any source fix, so Path-B.1 operates on the **complete** broken surface, not a sample.

## Method
- `cargo build --workspace --tests --keep-going` (the `--keep-going` continues past each failure to surface the full set, not just the first). Exit 101.
- `cargo check --workspace` run separately → exit 0 (mask confirmation).
- Every error cross-verified against the current library source (symbol's actual location/signature).
- Tree confirmed clean afterward (build creates only gitignored `target/` artifacts; `git status` empty, HEAD unchanged).

---

## Deliverable 1 — The complete broken-test-target enumeration

**Count: 8 broken test targets** (7 in `astraweave-render`, 1 in `astraweave-blend`), **31 compile errors total.** This resolves the gate's "≥3": the real number is **8**.

| # | Crate | Test target (actual name) | Errors | Error codes |
|---|---|---|---|---|
| 1 | astraweave-blend | `property_tests` | 2 | E0063 ×2 |
| 2 | astraweave-render | `wave2_culling_anim_misc_tests` | 1 | E0432 |
| 3 | astraweave-render | `wave2_primitives_mesh_ssao_error_tests` | 4 | E0063 ×4 |
| 4 | astraweave-render | `wave2_material_biome_advpost_error` | 1 | E0603 |
| 5 | astraweave-render | `mutation_resistant_comprehensive_tests` | 4 | E0432 ×1, E0433 ×3 |
| 6 | astraweave-render | `wave2_material_post_streaming_remediation` | 2 | E0432 ×1, E0603 ×1 |
| 7 | astraweave-render | `memory_safety_tests` | 1 | E0063 |
| 8 | astraweave-render | `coverage_booster_render` | 16 | E0061 ×5, E0063 ×5, E0432 ×1, E0603 ×3, E0609 ×1, E0308 ×1 |

**Error-code tally (cross-check):** E0061 ×5, E0063 ×12, E0308 ×1, E0432 ×4, E0433 ×3, E0603 ×5, E0609 ×1 = **31**. Per-target sum (2+1+4+1+4+2+1+16) = **31**. ✔ reconciled.

**The mask, documented:** `cargo check --workspace` → **exit 0, 0 errors** (only a benign unused-import warning in `aw_editor` and the known `nalgebra v0.26.2` future-incompat note) while these 8 test targets fail. `check` compiles lib + bin targets, **not** `tests/*.rs` integration targets — so the workspace reads "green" (130/130) while coverage (which *does* build test targets) is blocked. This is exactly the mask the CODE-FINDING described.

**Scope note:** enumeration covers integration/unit **test targets** (`--tests`), matching what `llvm-cov` builds. It does **not** cover doctests (see Deliverable 4 residual risk). No bench targets were built (`llvm-cov` does not build benches by default).

**Gate attribution corrections** (the gate's three names were approximate / scrambled):
- Gate said `memory_safety_tests` failed on `post::BloomConfig` (E0432). **Actually** it fails on `CpuMesh` E0063; its BloomConfig attribution was misplaced.
- Gate said "a wave2 culling target, E0063 CpuMesh". **Actually** `wave2_culling_anim_misc_tests` fails on `post::BloomConfig` (E0432), not CpuMesh. The CpuMesh failures are in `wave2_primitives_mesh_ssao_error_tests`, `memory_safety_tests`, `coverage_booster_render`.
- Gate missed a **whole drift family** (`TaaConfig`, E0603/E0433) and the deeper `coverage_booster_render` drift (E0061/E0609/E0308) entirely.

---

## Deliverable 2 — Per-target diagnosis (each verified against current source)

All breakages are **API drift**: the library moved on, the test target did not follow. Five distinct drift families.

### Family A — `BloomConfig` import-path drift (E0432 ×4)
- **Tests do:** `use astraweave_render::post::BloomConfig;`
- **Truth:** `BloomConfig` lives in `astraweave-render/src/bloom.rs:23`. It is **not** in `post` and **not** re-exported top-level (lib.rs re-exports every *sibling* post-fx config — `advanced_post`, `taa`, `atmosphere`, `god_rays`, … — but `bloom` is the lone omission). The only valid path is `astraweave_render::bloom::BloomConfig`. **The compiler explicitly suggests this exact fix** (`post::BloomConfig` → `bloom::BloomConfig`).
- **Targets:** wave2_culling_anim_misc_tests, mutation_resistant_comprehensive_tests, wave2_material_post_streaming_remediation, coverage_booster_render.
- **Why `bloom_integration` / `wave3_mutation_remediation` did NOT fail** (they also reference top-level `BloomConfig`): both are `#[cfg(feature = "bloom")]`-gated and the `bloom` feature is off by default → those bodies compile to empty. Not in the broken set; out of scope. (Note: this means top-level `astraweave_render::BloomConfig` would *also* fail if that feature were enabled — see the optional library-polish note in Deliverable 3.)
- **Classification:** test-only (symbol exists, wrong path). **Not** removed functionality.

### Family B — `TaaConfig` import-path drift (E0603 ×5, E0433 ×3)
- **Tests do:** `use astraweave_render::advanced_post::{… TaaConfig};` (E0603 "private struct") or reference `TaaConfig` with no import (E0433 "undeclared").
- **Truth:** `TaaConfig` is defined in `taa.rs` and re-exported top-level at lib.rs:247 (`pub use taa::TaaConfig;`). Inside `advanced_post.rs:8` it is a **private** `use crate::taa::TaaConfig;` (not `pub use`), so `advanced_post::TaaConfig` is private. Valid path: `astraweave_render::TaaConfig` (or `taa::TaaConfig`). **The compiler suggests `astraweave_render::taa::TaaConfig`.**
- **Targets:** wave2_material_biome_advpost_error (E0603), wave2_material_post_streaming_remediation (E0603), coverage_booster_render (E0603 ×3), mutation_resistant_comprehensive_tests (E0433 ×3 — fix is to *add* a top-level import).
- **Classification:** test-only (symbol exists publicly top-level, test used the private sub-module path / no import).

### Family C — `CpuMesh` new required fields (E0063 ×10)
- **Tests do:** `CpuMesh { vertices, indices }` (exhaustive, 2 fields).
- **Truth:** `mesh.rs:64` `CpuMesh` gained `albedo_image: Option<CpuImage>` and `texture_source_hint: Option<String>` (both optional; `CpuMesh` derives `Default`). Fields are real and documented (scatter-uploader texture discovery).
- **Targets:** wave2_primitives_mesh_ssao_error_tests (×4), memory_safety_tests (×1), coverage_booster_render (×5).
- **Classification:** test-only (add the two fields, or `..Default::default()`).

### Family D — `astraweave-blend` options new required fields (E0063 ×2)
- **Tests do:** in `property_tests.rs`, `process_options_strategy()` builds `ProcessOptions { … }` (omits `parallel_workers`); `conversion_options_strategy()` builds `ConversionOptions { … }` (omits `decomposition`).
- **Truth:** `options.rs` — `ProcessOptions.parallel_workers: u32` (line 582, default 0); `ConversionOptions.decomposition: SceneDecompositionOptions` (line 32). `ConversionOptions` derives `Default` and `SceneDecompositionOptions` has a manual `impl Default` (line 122), so `SceneDecompositionOptions::default()` is available.
- **Classification:** test-only (add `parallel_workers: 0` and `decomposition: SceneDecompositionOptions::default()`).

### Family E — deeper drift, `coverage_booster_render` only (E0061 ×5, E0609 ×1, E0308 ×1)
- **E0061 ×5 — method signatures gained params:**
  - `weather_fx.update(&queue, 0.016)` → `effects.rs:143 update(&mut self, queue, dt, camera_pos: Vec3)` — needs a `Vec3` camera-pos arg.
  - `batch.update_buffer(&device)` ×3 → `instancing.rs:126 update_buffer(&mut self, device, queue)` — needs `&queue`.
  - `manager.update_buffers(&device)` → `instancing.rs:217 update_buffers(&mut self, device, queue)` — needs `&queue`.
- **E0609 ×1 — `GBufferFormats.position` removed:** `formats.position` at line 2588 (inside a `println!`). Available fields now: `albedo, normal, emissive, velocity, depth`. Position is reconstructed from depth (deliberate deferred-rendering choice) — the separate position target was removed.
- **E0308 ×1 — array size grew:** `layers: [TerrainLayerGpu::default(); 8]` (line 11930) → struct now expects `[TerrainLayerGpu; 32]` (terrain layer cap grew 8 → 32).
- **Classification:** all test-only API-sync. The E0609 site is the single one touching *removed* functionality, but it is one diagnostic `println!`, not a behavioral assertion — adapt to `formats.depth` or drop the line (the only place a "fix-vs-delete" micro-decision arises, and it is one line, not a whole test).

---

## Deliverable 3 — Fix-scope proposal

**Every fix is test-target-only. Zero library changes required.** Each error is the test lagging the library; the library API is the truth and the tests follow it. (The drift families were independently re-confirmed: BloomConfig reachable at `bloom::`, TaaConfig public top-level, CpuMesh/options gained fields, methods gained params, GBuffer dropped a field, terrain cap grew.)

| Family | Fix | Mechanical? | Targets |
|---|---|---|---|
| A BloomConfig | `post::BloomConfig` → `bloom::BloomConfig` (compiler-suggested) | Mechanical | 4 |
| B TaaConfig | `advanced_post::TaaConfig` / undeclared → `astraweave_render::TaaConfig` (top-level) | Mechanical | 4 (5 E0603 + 3 E0433) |
| C CpuMesh | add `albedo_image: None, texture_source_hint: None` (or `..Default::default()`) | Mechanical | 3 (10 sites) |
| D blend options | add `parallel_workers: 0` + `decomposition: SceneDecompositionOptions::default()` | Mechanical | 1 (2 sites) |
| E coverage_booster deep | add method args (Vec3 cam-pos; `&queue` ×4); `formats.position` → `.depth` or drop; `[…; 8]` → `[…; 32]` | Mechanical (1 micro fix-vs-drop: the position `println!`) | 1 (7 sites) |

**Blast radius:** confirmed **test-target-only** for all 8. No `astraweave-render` / `astraweave-blend` library/source change is implied by any fix. If any proposed fix *did* require a library edit, it would be flagged out-of-scope — none do.

**Judgment calls for the director:**
1. **CpuMesh / blend field style** — explicit `None` / `::default()` vs `..Default::default()`. (Recommendation: explicit values in the blend *strategy* literals to keep proptest coverage honest; `..Default::default()` is fine for the CpuMesh AABB/registry tests where the new fields are irrelevant.)
2. **The single `GBufferFormats.position` `println!`** — adapt to `.depth` vs delete the line. (Recommendation: switch to `.depth` — it preserves the diagnostic intent and the test's structure; deletion loses nothing but is equally valid.)
3. **Optional library polish (out of strict test-only scope, explicitly NOT proposed for Path-B.1):** add `pub use bloom::{BloomConfig, BloomPipeline};` to lib.rs so bloom matches the top-level re-export convention every sibling post-fx config follows (and so the `#[cfg(feature="bloom")]`-gated `bloom_integration` would resolve if that feature were enabled). This is a *library* change → it belongs to a render-trace beat, not Path B. Flagged for the record; **the test-only `bloom::BloomConfig` path fully unblocks coverage without it.**

**Trace routing (for trace-maintenance record):**
- Families A, B, C, E (astraweave-render) → `docs/architecture/render_pipeline_material_system_shader_infrastructure.md` (the render trace). The drift inventory (BloomConfig not top-level-re-exported; TaaConfig private in advanced_post; GBuffer position removed; terrain layer cap 8→32; effects/instancing signature changes) is material for its §6 Conflict Map / §8 Invariants.
- Family D (astraweave-blend) → **no dedicated trace exists** (blend is untraced; nearest is `docs/architecture/asset.md`). Route the finding to asset.md as the nearest owner, or note blend as a trace-generation candidate.

---

## Deliverable 4 — Coverage-rebaseline readiness

**After the 8 targets are fixed, will `cargo llvm-cov --workspace --summary-only` run clean?**
- **Compile blockers:** these 8 are the complete `--tests` compile-blocker set. Fixing them clears the compile barrier the Session-A attempt hit.
- **Residual risk 1 — doctests:** `--tests` does not build doctests; `llvm-cov --workspace` runs them unless `--no-doctests`. A broken doctest could still block. *Mitigation:* run the re-baseline with `--no-doctests` first (isolates the integration/unit coverage number), or do a `cargo test --workspace --doc --no-run` pass in Path-B.1 to enumerate any doctest drift before the full run. **Unknown until checked — flagged, not yet measured.**
- **Residual risk 2 — runtime (GPU/headless):** many render test targets compile fine but need a GPU adapter at runtime. The D.2.B.0 hazard-2 (headless GPU blocking) was **refuted on this headed machine** (real adapter present), so GPU-init is not expected to block. A hanging/long-running test remains a generic possibility; `--summary-only` does not change runtime behavior. *Mitigation:* run under a timeout and watch for stalls.
- **Recommendation for Path-B.2:** re-baseline with `cargo llvm-cov --workspace --summary-only --no-doctests` first for a clean comparable number; then optionally add doctests once any doctest drift is resolved.

**Methodology decision (already ratified, restated):** the re-baseline targets **true `--workspace` coverage** (Path B was chosen over re-baselining the curated 29-crate subset). The result will be a **different — likely lower — number than the asserted 59.3%**, because the denominator is the whole workspace, not a subset. That lower number is the **honest workspace coverage** and is comparable across the whole workspace, not a curated slice. The registry `coverage-weighted` row stays **PENDING-D2** until Path-B.2 produces it.

---

## Headline questions — answered

1. **How many targets are actually broken — 3 or more?** → **8** (7 render + 1 blend), 31 errors. The gate's "≥3" was a low incidental sample; two of its three attributions were scrambled and it missed two whole drift families.
2. **Are all fixes mechanical test-only API-syncs, or do any require fix-vs-delete?** → **All test-only; zero library changes.** The lone judgment micro-call is one `println!` referencing the removed `GBufferFormats.position` (adapt to `.depth` or drop) — not a whole-test deletion. No obsolete-test removal is warranted; every referenced symbol still exists (relocated, made-public-elsewhere, or signature-extended).
3. **Will fixing them actually unblock `--workspace` coverage?** → Clears the **compile** barrier (the known blocker). Two residual non-compile unknowns remain: **doctests** (untested by `--tests`; mitigate with `--no-doctests` first) and generic **runtime stalls** (GPU-init not expected to block — hazard-2 refuted on this headed machine).

## Gate
Path-B.0 stops here. Director ratifies before Path-B.1 touches source:
- the complete broken-target set (8, resolving "≥3");
- the per-target diagnoses + drift-family classification;
- the fix-scope proposal (all test-only) and the three judgment calls (field style; the `position` println adapt-vs-drop; the optional `bloom` library re-export — deferred);
- the coverage-rebaseline readiness (compile cleared; doctest + runtime residuals flagged), and the `--no-doctests`-first methodology.

**Not started:** any source edit (Path-B.1) and the coverage re-baseline (Path-B.2).
