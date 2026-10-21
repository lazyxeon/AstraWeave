# Workspace Lint Inventory — 2026-07

**Date:** 2026-07-24
**Status:** Accepted CLIP-1 enumeration; founding evidence for a future HEALTH campaign
**Campaign disposition:** Evidence only. This document does **not** start HEALTH or authorize any cleanup.

## 1. Scope and evidence basis

This inventory records the complete primary-package diagnostic stack exposed by Comprehensive CI's Code Quality Clippy tier after CLIP-1 retired the `astraweave-terrain` findings.

- **No-edit enumeration revision:** `89fbe97eb18b36cfe43172c03cdb5d1ad9259aa1`
- **Toolchain:** `rustc 1.89.0 (29483883e 2025-08-04)`
- **Clippy:** `clippy 0.1.89 (29483883ee 2025-08-04)`
- **Workspace members from `cargo metadata --no-deps`:** 133
- **Workflow exclusions:** 12
- **Primary packages enumerated:** 121
- **Primary packages passing:** 99
- **Packages with Rust diagnostics:** 21
- **Packages blocked before Rust diagnostics:** 1 (`astraweave-blend` on Windows)
- **Unique Rust diagnostics:** 156
  - **Mechanical:** 102
  - **Structural / compile / configuration / test-intent:** 54

Repeated diagnostics emitted while compiling both ordinary and test targets were counted once by diagnostic code/message and primary source location. Cargo summary lines such as `could not compile` were not counted as additional findings.

## 2. Reproducible invocation

The exact workflow command in `.github/workflows/ci.yml` was:

```bash
cargo clippy --workspace --locked $EXCLUDED_PACKAGES \
  --all-features --all-targets -- -D warnings
```

`EXCLUDED_PACKAGES` expanded to:

```text
--exclude astraweave-author
--exclude visual_3d
--exclude ui_controls_demo
--exclude npc_town_demo
--exclude rhai_authoring
--exclude cutscene_render_demo
--exclude weaving_playground
--exclude combat_physics_demo
--exclude navmesh_demo
--exclude physics_demo3d
--exclude debug_toolkit_demo
--exclude aw_editor
```

On Windows, the literal workspace command stopped before Rust diagnostics when `astraweave-blend --all-features` reached `tikv-jemalloc-sys`. Cargo is fail-fast at the package graph level, so the complete inventory used the following semantically equivalent primary-package loop. `--no-deps` prevents dependency warnings from being attributed to the selected package while preserving the workflow's locked, all-feature, all-target, warnings-as-errors tier:

```powershell
$excluded = @(
    'astraweave-author',
    'visual_3d',
    'ui_controls_demo',
    'npc_town_demo',
    'rhai_authoring',
    'cutscene_render_demo',
    'weaving_playground',
    'combat_physics_demo',
    'navmesh_demo',
    'physics_demo3d',
    'debug_toolkit_demo',
    'aw_editor'
)

$metadata = cargo metadata --no-deps --format-version 1 | ConvertFrom-Json
$workspaceMembers = @($metadata.workspace_members)
$packages = @(
    $metadata.packages |
        Where-Object {
            $workspaceMembers -contains $_.id -and
            $excluded -notcontains $_.name
        } |
        Sort-Object name
)

foreach ($package in $packages) {
    cargo clippy --locked -p $package.name `
        --all-features --all-targets --no-deps `
        --message-format short -- -D warnings
}
```

Packages whose compact output contained only Cargo summaries were rerun individually with the same flags and an isolated target directory to capture the complete compiler diagnostics.

## 3. Lint debt versus compile rot

The following are **NOT-BUILDING-UNDER-CI-FEATURES**, not ordinary lint debt:

| Package | Compile/build failures | Classification |
|---|---:|---|
| `astraweave-embeddings` | 16 | Feature-gated library compile rot: ORT API drift, missing feature dependencies/macros, and Candle API mismatches |
| `astraweave-llm` | 3 | Phi-3 feature compile rot: trait signature, removed configuration constructor, and model-forward arity |
| `astraweave-physics` | 1 | All-feature library compile failure: unresolved `serde_json` |
| `astraweave-core` | 1 | All-target bench compile failure: stale `Pose` initializer |
| `veilweaver_demo` | 2 | Binary compile failure: unresolved workspace crates |
| `hello_companion` | 3 | Binary compile failure: missing `Result::context` trait availability |
| `astraweave-blend` on Windows | 1 build boundary | `tikv-jemalloc-sys` cannot execute its configure command for MSVC; Rust diagnostics are not reached |

These require dedicated feature/API/build restoration work. Removing them from the workflow, disabling their features/targets, or adding lint allowances would not fix the underlying failures.

`astraweave-profiling` additionally has two all-target integration-test compile failures from removed Tracy client methods. They are recorded as structural test-target failures, not mechanical lint debt.

## 4. Per-package summary

| Package | Unique findings | Mechanical | Structural | Disposition |
|---|---:|---:|---:|---|
| `adaptive_boss` | 1 | 1 | 0 | Lint debt |
| `astraweave-ai` | 38 | 30 | 8 | Mixed lint and deferred wiring/API debt |
| `astraweave-asset` | 2 | 2 | 0 | Test lint debt |
| `astraweave-blend` | 0 enumerated | 0 | 0 | Windows build boundary before Rust diagnostics |
| `astraweave-core` | 1 | 0 | 1 | **NOT-BUILDING-UNDER-CI-FEATURES** |
| `astraweave-dialogue` | 1 | 1 | 0 | Test lint debt |
| `astraweave-ecs` | 8 | 4 | 4 | Mixed mechanical and test-intent debt |
| `astraweave-embeddings` | 17 | 1 | 16 | **NOT-BUILDING-UNDER-CI-FEATURES** |
| `astraweave-gameplay` | 2 | 2 | 0 | Test lint debt |
| `astraweave-llm` | 8 | 3 | 5 | **NOT-BUILDING-UNDER-CI-FEATURES** plus feature configuration debt |
| `astraweave-memory` | 3 | 3 | 0 | Lint debt |
| `astraweave-net` | 2 | 1 | 1 | Mixed lint and feature configuration debt |
| `astraweave-physics` | 1 | 0 | 1 | **NOT-BUILDING-UNDER-CI-FEATURES** |
| `astraweave-profiling` | 3 | 0 | 3 | Test-target compile and assertion-quality debt |
| `astraweave-rag` | 3 | 3 | 0 | Test lint debt |
| `astraweave-render` | 40 | 31 | 9 | Mixed mechanical, API, and test-logic debt |
| `astraweave-scene` | 4 | 4 | 0 | Test lint debt |
| `astraweave-ui` | 2 | 2 | 0 | Test lint debt |
| `astraweave-weaving` | 4 | 4 | 0 | Test lint debt |
| `hello_companion` | 4 | 1 | 3 | **NOT-BUILDING-UNDER-CI-FEATURES** |
| `veilweaver_demo` | 3 | 0 | 3 | **NOT-BUILDING-UNDER-CI-FEATURES** |
| `veilweaver_slice_runtime` | 9 | 9 | 0 | Test lint debt |
| **Total Rust diagnostics** | **156** | **102** | **54** | 21 packages |

## 5. Complete finding detail

### `adaptive_boss` — 1 mechanical

- `examples/adaptive_boss/src/main.rs:2` — unused `BossDirector` import.

### `astraweave-ai` — 38 total: 30 mechanical, 8 structural

- `src/ai_arbiter.rs:209` — `fast_executor` never read. Director disposition: built-but-unwired subsystem; retain with a documented field-level allowance pending its dedicated wiring beat.
- `src/goap/actions.rs:14,53,109,175,217,262,315,369,417,470,511` — eleven `new_without_default` findings.
- `src/goap/adapter.rs:798` and `src/goap/goal_scheduler.rs:686` — unnecessary `mut`.
- `src/goap/config.rs:19` and `src/goap/goal.rs:19` — manually implemented `Default` can be derived.
- `src/goap/debug_tools.rs:196,197` and `src/goap/plan_stitcher.rs:118,119` — needless borrow followed by immediate dereference.
- `src/goap/goal_authoring.rs:192` — redundant closure.
- `src/goap/goal_scheduler.rs:191` — unused `calculate_urgency` method.
- `src/goap/goal_validator.rs:374` — simplifiable `map_or`.
- `src/goap/goal_validator.rs:393,526,542` and `src/goap/plan_visualizer.rs:324,378,410` — parameters used only in recursion.
- `src/goap/history.rs:70,89,147` — `or_insert_with(Default::default)` should be `or_default`.
- `src/goap/plan_stitcher.rs:176` — useless `vec!`.
- `src/goap/plan_stitcher.rs:203` — needless reference of the right operand.
- `src/goap/plan_visualizer.rs:159` — literal passed through an empty format slot.
- `src/goap/plan_visualizer.rs:866` — unused `closing_braces`.
- `src/goap/shadow_mode.rs:196,203` — explicit clone closures instead of `cloned`.

The structural count is the deferred executor field, the unused method, and six recursion-only parameter findings. The remaining 30 are mechanical candidates.

### `astraweave-asset` — 2 mechanical

- `tests/gltf_loading_tests.rs:450` — needless `return`.
- `tests/texture_loading_tests.rs:422` — needless `return`.

### `astraweave-blend` — Windows build boundary

- `tikv-jemalloc-sys v0.6.1+5.3.0-1-ge13ca993e8ccb9ba9847cc330696e02839f328f7` warns that MSVC support is untested, then its build script fails with `failed to execute command: program not found`.
- No Rust lint count is assigned because package compilation never reaches the Rust target.

### `astraweave-core` — 1 structural compile failure

- `benches/full_game_loop.rs:220` — `E0063`: `Pose` initializer lacks `float_x`, `float_z`, `scale_y`, and two other fields.

### `astraweave-dialogue` — 1 mechanical

- `tests/mutation_resistant_comprehensive_tests.rs:7` — unused `RunnerState` import.

### `astraweave-ecs` — 8 total: 4 mechanical, 4 structural/test-intent

- `src/mutation_resistance_tests.rs:19` — duplicated attribute.
- `src/mutation_resistance_tests.rs:1250,1251` — each comparison emits both `unused_comparisons` and an extreme-comparison diagnostic; the comparisons are invariant under the operand types.
- `tests/concurrency_tests.rs:190,195` — explicit copy closure instead of `copied`.
- `tests/concurrency_tests.rs:391` — unused `alive`.

### `astraweave-embeddings` — 17 total: 1 mechanical, 16 compile failures

All findings are in `src/client.rs`.

- `:159` — `ort::Session` type not found.
- `:160,173,274,290` — unresolved `tokenizers` crate/module.
- `:168` — `ort::Session` constructor not found.
- `:169` — `ort::GraphOptimizationLevel` not found.
- `:174,192,291,329,459` — `anyhow!` macro unavailable.
- `:206` — unresolved `ort::Tensor` and `ort::Value` imports.
- `:282` — unused `Tensor` import; the sole mechanical lint.
- `:294` — unresolved `hf_hub` crate/module.
- `:308` — Candle `VarBuilder` type mismatch.
- `:340` — method now requires three arguments but two were supplied.

### `astraweave-gameplay` — 2 mechanical

- `src/mutation_tests.rs:3230` — empty line after doc comment.
- `src/mutation_tests.rs:3532` — useless `vec!`.

### `astraweave-llm` — 8 total: 3 mechanical, 2 configuration, 3 compile failures

All findings are in `src/phi3.rs`.

- `:36` — unused `DType` import.
- `:155` — `E0599`: `candle_transformers::models::phi3::Config::v3_mini` no longer exists.
- `:212` — unexpected undeclared `cuda` feature condition.
- `:218` — unexpected undeclared `metal` feature condition.
- `:317` — `E0061`: Phi-3 `forward` now requires a `seqlen_offset: usize`.
- `:411` — unused `rand::Rng` import.
- `:412` — deprecated `gen_range`; renamed to `random_range`.
- `:440` — `E0195`: `LlmClient::complete` implementation takes `String`, while the trait requires `&str`, producing incompatible async-trait lifetime bounds.

The three compiler errors require LLM-domain/API restoration and were explicitly STOP-scoped out of CLIP-1.

### `astraweave-memory` — 3 mechanical

- `src/components.rs:428,496,550` — manual clamp patterns.

### `astraweave-net` — 2 total: 1 mechanical, 1 configuration

- `src/tls.rs:160` — unexpected undeclared `dangerous-testing` feature condition.
- `src/tls.rs:244` — unused `result`.

### `astraweave-physics` — 1 structural compile failure

- `src/async_scheduler.rs:297` — `E0433`: unresolved `serde_json` crate/module.

### `astraweave-profiling` — 3 structural/test-target findings

- `tests/profiling_tests.rs:9` — `assert!(true)` is vacuous.
- `tests/mutation_resistant_comprehensive_tests.rs:143` — `E0599`: Tracy `Client::alloc` method not found.
- `tests/mutation_resistant_comprehensive_tests.rs:150` — `E0599`: Tracy `Client::free` method not found.

### `astraweave-rag` — 3 mechanical

- `tests/mutation_resistant_comprehensive_tests.rs:1048,1049` — unindented doc-list items.
- `tests/mutation_resistant_comprehensive_tests.rs:1279` — unused `retained`.

### `astraweave-render` — 40 total: 31 mechanical, 9 structural/API/test-logic

- `src/brdf_lut.rs:248` — manual bit rotation.
- `src/brdf_lut.rs:260,261,262` — manual range containment.
- `src/disney_material.rs:184,211` — expressions always evaluate to false.
- `src/disney_material.rs:234` — field assignment after `Default::default`.
- `src/disney_material.rs:277` — unused `r_plain`.
- `src/grass_blade.rs:510` — manual inclusive-range containment.
- `src/hiz_pyramid.rs:259,265` — `min` operations that can have no effect.
- `src/impostor_bake.rs:568` — manual `div_ceil`.
- `src/impostor_pass.rs:137` — eight function arguments exceed the configured seven-argument threshold.
- `src/ltc_area_lights.rs:662` — identity operation.
- `src/mutation_tests.rs:2356` — unused `pos`.
- `src/particle_forces.rs:421,440` — manual inclusive-range containment.
- `src/particle_sort.rs:265` — useless `vec!`.
- `src/particle_sort.rs:266` — unused `camera`.
- `src/shadow_quality.rs:232` — range loop used only to index `splits`.
- `src/snow_accumulation.rs:366,373` — field assignment after `Default::default`.
- `src/subgroup_ops.rs:189,190,191` — expressions always evaluate to false.
- `src/taa.rs:545,546` — manual inclusive-range containment.
- `src/temporal_upscale.rs:668` — field assignment after `Default::default`.
- `src/terrain_material_manager.rs:40` — duplicated attribute.
- `src/terrain_material_manager.rs:165,166,167,168,169,170` — unindented doc-list items.
- `src/vegetation_gpu.rs:880` — manual inclusive-range containment.
- `src/vegetation_interaction.rs:359` — `assert!(true)` is vacuous.
- `src/virtual_texture.rs:634` — field assignment after `Default::default`.
- `src/water.rs:1376,1377` — `assert!(true)` is vacuous.

The nine structural/API/test-logic findings are five always-false expressions, the eight-argument API, and three vacuous assertions. The remaining 31 are mechanical candidates.

### `astraweave-scene` — 4 mechanical

- `src/mutation_tests.rs:2169,2173` — casts from `i32` to `i32`.
- `src/mutation_tests.rs:3122,3124` — unused `exp_s` and `got_s`.

### `astraweave-ui` — 2 mechanical

- `tests/mutation_hardening_tests.rs:9` — unused `HudManager` import.
- `tests/mutation_hardening_tests.rs:553` — unused `NotificationType` import.

### `astraweave-weaving` — 4 mechanical

- `tests/mutation_tests.rs:45,64,78` — field assignment after `Default::default`.
- `tests/mutation_tests.rs:2415` — multiplication by negative one instead of unary negation.

### `hello_companion` — 4 total: 1 mechanical, 3 compile failures

- `examples/hello_companion/src/main.rs:1342,1345,1354` — `E0599`: `Result::context` method unavailable at all three call sites.
- `examples/hello_companion/src/visual_demo.rs:168` — unused `color`.

### `veilweaver_demo` — 3 structural

- `examples/veilweaver_demo/src/main.rs:19` — `E0433`: unresolved `astraweave_gameplay`.
- `examples/veilweaver_demo/src/main.rs:443` — unreachable expression.
- `examples/veilweaver_demo/src/main.rs:599` — `E0433`: unresolved `astraweave_scene`.

### `veilweaver_slice_runtime` — 9 mechanical

- `tests/display_serde_polish_tests.rs:180` — approximate PI constant.
- `tests/e2e_presentation_pipeline.rs:17` — unused `WalkthroughEvent` import.
- `tests/mutation_tests.rs:956,975,3614` — needless borrows of formatted values passed to generic arguments.
- `tests/mutation_tests.rs:1039` — two approximate PI constants at distinct columns.
- `tests/mutation_tests.rs:3135,3380` — manual inclusive-range containment.

## 6. Disposition

This inventory is the accepted founding evidence document for a future HEALTH campaign. It does not authorize HEALTH planning, implementation, suppression, workflow exclusions, feature removal, target removal, or provider/API refactors.

CLIP-1 changed only the separately authorized `AIArbiter.fast_executor` field disposition after this no-edit enumeration. Every other item above remains an owned future finding until deliberately scoped by the director.
