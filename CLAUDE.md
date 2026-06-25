# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

AstraWeave is a **scientific proof of concept**: a production-grade AI-native game engine built **iteratively by AI with zero human-written code**. It uses a deterministic ECS architecture where AI agents are first-class citizens. The workspace contains 130 crates (~51 production + examples + tools; `cargo metadata --no-deps`, verified 2026-06-10). Rust toolchain is pinned at 1.89.0.

### Mandate

1. **Zero Human Code**: Generate all code, bias heavily toward executing rather than asking, but confirm before making architectural changes
2. **Mission-Critical Standards**: Treat every line as spacecraft-grade. **90%+ confidence** before marking any task complete.
3. **Exhaustive Testing**: "It compiles" is insufficient. Verify through tests, benchmarks, and validation.
4. **Production Ready**: No toy code. All systems must be scalable, performant, and secure.
5. **Maintain Context**: Read `docs/current/PROJECT_STATUS.md` at the start of every session before doing any work.
6. **Pre-Existing Issues**: Never Ignore or defer pre-existing issues when found, fix all issues found to production ready standards.

### Scope Discipline

Make ONLY the changes requested. Do not refactor, rename, reorganize, or "improve" adjacent code unless specifically asked. Do not add features, abstractions, or "nice to haves" beyond the stated task.

**Never build a second implementation of a logical system that already exists** (rendering path, vertex format, material pipeline, scheduler, tonemap chain, scene serializer). Before adding any such system, run `rg 'struct <Name>\|trait <Name>'` workspace-wide; if a peer implementation exists, extend it or surface the conflict to the user. The Fix-27 campaign and the editor-render-divergence audit each took weeks to unwind a duplicate pipeline that was created without this check.

**Wrapped-Component Resource Identity (§7.7 structural axiom).** When component A wraps component B and both manage state of the same logical role (depth target, terrain chunk map, splat buffer, material library, input action queue, audio bus state, animation clip library), A's reads do not reflect B's writes unless the wrapper explicitly delegates. The Editor Multi-Tool Architecture Sub-phase 3 campaign confirmed this at four layers (depth target, mesh-data, texture-data attribute set, UI/renderer capacity) across eight diagnostic rounds (2026-05-04 → 2026-05-08). The trap recurs at non-rendering boundaries: editor `input_bindings_panel.rs` (2,511 LoC) reinventing astraweave-input without depending on it; editor `AudioPanel` UI knobs (10 of ~25 `AudioAction` variants) that no engine code reads; `astraweave-fluids/src/editor.rs` (5,823 LoC) forward-design surface not wired to `tools/aw_editor`. Before adding any wrapper-layer resource, check whether the wrapped layer holds the same logical resource — if so, the wrapper must delegate, not duplicate. See `docs/current/EDITOR_MULTI_TOOL_ARCHITECTURE_CAMPAIGN.md` §7.7 for the full forensic trail.

### Error Handling Policy

- **FIX ALL COMPILATION ERRORS** — zero tolerance. Never leave broken code.
- **Warnings may be deferred** — document for future cleanup.
- Run `cargo check -p <crate>` after **every** code change. This is mandatory.
- **Never discard `Result` on user-facing fallible operations** (asset I/O, GPU state, file ops, prefab/scene reload, watcher creation). Use `?.context("…")` or a named recovery function. `let _ =` and `.ok()` on such calls are forbidden — they produced 12 CRITICAL + 18 HIGH editor-audit findings where features silently no-op'd.

---

## Workflow & Process

### Chain of Thought

1. **Understand**: Analyze the request against mission-critical standards.
2. **Context**: Check `docs/current/` for latest state. **For any cross-crate work, read `docs/architecture/ARCHITECTURE_MAP.md` first** (dependency graph, integration seams, blast-radius analysis). **For subsystem-internal work, read the relevant `docs/architecture/<system>.md` trace first** (file map, conflict map, decision log, invariants, open questions). Read other reference files as needed.
3. **Plan**: Break down the task. Identify risks. Consult the Architecture Map for dependency direction and shared types before modifying any public API.
4. **Execute**: Generate code/docs. **Verify compilation immediately.**
5. **Validate**: Run tests/benchmarks. Ensure 90%+ confidence.
6. **Report**: Update master reports if thresholds are exceeded.

### Development Workflow

1. Make changes in one crate at a time
2. `cargo check -p <crate>` (mandatory after every change)
3. Fix all compilation errors immediately
4. `cargo test -p <crate>` (if tests exist)
5. `cargo fmt --all`
6. `cargo clippy -p <crate> --all-features -- -D warnings`
7. Run `hello_companion` or `unified_showcase` for integration validation

### Integration Completeness (before declaring work complete)

A feature is incomplete until it is wired end-to-end. Before marking any task done, verify all four:

1. **Production caller exists.** Every new public type, function, or module must have ≥1 non-test, non-feature-gated call site. Run `rg '<Name>::new\|<fn>\(' --type rust -g '!*test*' -g '!benches/*'`. Zero matches = dormant code (this is how `ParallelSchedule`, the render-graph DAG, `AdvancedErosionSimulator`, and `RegionalArchetypePanel` all shipped — and were later removed or rewritten).
2. **All registration surfaces touched.** When adding an enum variant, panel type, component, or system, list every match arm, registry, and initializer in a comment first. After editing, `cargo check` must pass without exhaustiveness warnings AND `rg <new_name>` must show every site updated. Panel registration alone has 11 surfaces — the F.5-paint diagnostic is the canonical failure.
3. **Every UI/API-exposed config field is read.** A field that is settable but never observed downstream is a bug. Confirm with a grep for the field name in the consuming subsystem before completion.
4. **Architecture trace is current.** If you modified code in a subsystem that has a trace under `docs/architecture/`, update the trace in the same commit (or note the deferred update in the commit message). At minimum touch §5 Active File Map, §8 Invariants if any changed, and §11 Open Questions if any closed or opened. Bump the doc version and add a revision history entry. Untraced subsystems: when adding non-trivial new surface, run the trace-generation prompt against the subsystem before declaring the feature complete.

### Architecture Trace Maintenance

The workspace has a per-subsystem architecture trace campaign under `docs/architecture/`. Each trace is the canonical forensic reference for one subsystem — authoritative pipeline, semantic vocabulary, file map, conflict map, decision log, invariants, open questions. **Trace docs are part of the production contract**: when you change code in a traced subsystem, you update the trace in the same commit.

**Existing traces** (table generated by `aw_trace_sync --write` from trace front-matter — do not hand-edit between the markers):

<!-- TRACE-TABLE:START -->
| Trace | Subsystem |
|---|---|
| `docs/architecture/ai_pipeline.md` | AI Pipeline (with 8 subsystem traces) |
| `docs/architecture/animation.md` | Animation System |
| `docs/architecture/asset.md` | Asset + Asset-Pipeline — loading, cell loader, Nanite preprocess, validator/texture/mesh |
| `docs/architecture/audio.md` | Audio |
| `docs/architecture/aw_editor.md` | aw_editor (Visual Editor) |
| `docs/architecture/camera.md` | Camera — freefly, projection, render-view, parity producer |
| `docs/architecture/cinematics.md` | Cinematics — timeline / sequencer |
| `docs/architecture/ecs_math_core_sdk_foundation.md` | ECS substrate + Math/Core/SDK |
| `docs/architecture/fluids.md` | Fluids |
| `docs/architecture/gameplay.md` | Gameplay — combat, crafting, Veilweaver slice, in-crate weave bridge |
| `docs/architecture/input.md` | Input |
| `docs/architecture/nav.md` | Navigation / Pathfinding — navmesh (resolves the two-`nav`-crate conflict) |
| `docs/architecture/net.md` | Net (snapshot-based game server) |
| `docs/architecture/net_ecs.md` | Net-ECS + standalone matchmaking |
| `docs/architecture/pcg.md` | PCG — WFC, layout, encounters, seeded RNG |
| `docs/architecture/persistence_ecs.md` | Persistence (aw-save + persistence-ecs) |
| `docs/architecture/physics.md` | Physics (Rapier3D wrapping + subsystems) |
| `docs/architecture/quests.md` | Quests — components, systems, LLM + terrain quests |
| `docs/architecture/render_pipeline_material_system_shader_infrastructure.md` | Render Pipeline + Material System + Shader Infrastructure |
| `docs/architecture/scene.md` | Scene — scene graph, world partition, cell streaming, GPU resource manager |
| `docs/architecture/security.md` | Security + Secrets — script sandbox, anticheat, signatures, path validation, keyring |
| `docs/architecture/terrain.md` | Terrain System — voxel meshing, biomes, noise, scatter, streaming (complements `terrain_materials.md`, which is the material-splat slice) |
| `docs/architecture/terrain_materials.md` | Terrain Material System (canonical reference example) |
| `docs/architecture/ui.md` | UI — HUD, menus, panels (egui) |
| `docs/architecture/water.md` | Water Successor — `WaterQuery` facade + render water surface + weave-response (part/freeze/raise/FreezeWater) + F.4 accents |
| `docs/architecture/weaving.md` | Weaving / Fate-Weaving — the Veilweaver mechanic (`astraweave-weaving`) |
<!-- TRACE-TABLE:END -->

**The 5-prompt trace toolkit lives at `docs/architecture/_meta/`:**

- `ARCHITECTURE_TRACE_TEMPLATE.md` — 12-section trace structure
- `TRACE_PROMPT_TEMPLATE.md` — generation prompt (new traces)
- `TRACE_VERIFICATION_PROMPT_TEMPLATE.md` — verification pass (resolve [INFERRED]/[NEEDS VERIFICATION] markers)
- `DEEP_TRACE_INVESTIGATION_TEMPLATE.md` — close factual Open Questions / enrich decisional ones
- `SUBSYSTEM_TRACE_EXPANSION_PROMPT_TEMPLATE.md` — additive subsystem expansion within a parent trace

**Automation (`aw_trace_sync`, v1 implemented):** each trace carries YAML front-matter (the single source of truth for `trace_id`/`primary_crate`/`owns`/status). The `tools/aw_trace_sync` tool regenerates the trace table above (between the `TRACE-TABLE` markers) and the `workspace_map.html` per-crate trace links from that front-matter; the CI `trace-sync.yml` gate (`--check`) blocks drift. Workflow: edit a trace's front-matter, run `cargo run -p aw_trace_sync -- --write`, commit. Other commands: `--validate-only`, `--list-untraced`, `--check`. Do NOT hand-edit between the `TRACE-TABLE` markers, nor the per-crate `trace`/`status`/`statusCategory`/`statusEvidence` fields or `runtime` edge flags in `workspace_map.html` (all tool-owned). **v1.1** also syncs the map node status (`lifecycle_status`/`integration_status`/`summary` → the owning trace's primary crate) and renders `runtime_edges` (asserted runtime wiring). Cargo-derived topology + `map_overlay.json` remain deferred (design: `TRACE_SYNC_PROPOSAL.md`).

**Workflow:**

1. **Before** modifying code: read the trace's §6 Conflict Map, §7 Decision Log, §8 Invariants. If your change would break an invariant or contradict a decision, surface that explicitly.
2. **After** modifying code: update §5 Active File Map, §8 Invariants, §11 Open Questions, and revision history in the same commit.
3. **For untraced subsystems:** when adding non-trivial new surface, run the trace-generation prompt against the subsystem in a separate doc-only commit before the feature commit lands.

The traces are the workspace's forensic counterweight to documentation drift — see Documentation Hazards below.

### Build Strategy

**DO:**
- Build incrementally (`-p` flag for single crates)
- Use the editor cargo aliases (`editor`, `editor-release`, `editor-dev`); for workspace-wide operations use the explicit commands below (no `check-all`-style aliases are defined in `.cargo/config.toml`)
- Use `--release` for examples
- Run `cargo check -p <crate>` after every modification

**DON'T:**
- Attempt full workspace builds without exclusions
- Cancel long-running builds (dependencies take time)
- Fix broken examples without checking API versions first
- Leave compilation errors unfixed

### Build & Development Commands

```bash
# Setup
./scripts/bootstrap.sh

# Per-crate workflow (mandatory after every change)
cargo check -p <crate>              # Compile check — ALWAYS run after changes
cargo test -p <crate>               # Run tests for a crate
cargo fmt --all                     # Format all code
cargo clippy -p <crate> --all-features -- -D warnings  # Lint a crate

# Workspace-wide commands (no aliases defined — use the explicit forms)
cargo check --workspace             # Workspace check (130/130 members, 0 errors as of 2026-06-10)
cargo build -p astraweave-core -p astraweave-ecs -p astraweave-math -p astraweave-ai  # Core components
cargo test --workspace              # Workspace tests (long-running)
cargo clippy --workspace --all-features -- -D warnings  # Full linting

# Editor
cargo editor                        # Run editor (release-fast profile)
cargo editor-release                # Run editor (full release)
cargo editor-dev                    # Run editor (debug)

# Examples
cargo run -p hello_companion --release   # Flagship AI demo (6 modes)
cargo run -p unified_showcase --release  # Rendering showcase

# Benchmarks & coverage
cargo bench -p <crate>              # Run benchmarks (criterion)
cargo llvm-cov                      # LLVM source-based coverage

# Formal verification (for unsafe code)
cargo +nightly miri test -p <crate> --lib -- --test-threads=1
cargo kani --package <crate>
```

### Build Timings

- First build: 15-45 minutes (wgpu + dependencies)
- Core incremental: 8-15 seconds
- Full workspace check: 2-4 minutes (with exclusions)

### Master Report Maintenance

Three authoritative reports **MUST** be updated when thresholds are exceeded:

| Report | Update When |
|--------|-------------|
| `docs/current/MASTER_ROADMAP.md` | Completing phases, changing priorities, >4h work sessions |
| `docs/current/MASTER_BENCHMARK_REPORT.md` | Performance changes >10%, new benchmarks |
| `docs/current/MASTER_COVERAGE_REPORT.md` | Coverage ±5% per-crate or ±2% overall |

Increment version number and add revision history entry on every update.

### Response Guidelines

- Use markdown for clarity. End responses with questions to continue iteration.
- Handle incomplete features gracefully (feature flags).
- If stuck, try simpler solutions — never leave broken code.

---

## Code Patterns & Conventions

### Error Handling

```rust
use anyhow::{Context, Result};
fn do_work() -> Result<()> {
    something().context("Failed to do work")?;
    Ok(())
}
```

**NO `.unwrap()` in production code.** All existing `.unwrap()` calls are confined to `#[cfg(test)]` modules and test utilities — this is intentional and acceptable. Use `anyhow::Context` or `?` in production paths. Build/CLI tools (`aw_build`, `aw_demo_builder`) have a handful of low-risk `.unwrap()` calls in non-runtime paths.

### ECS Components & Systems

```rust
pub struct Position { pub x: f32, pub y: f32 }
// Any T: 'static + Send + Sync auto-implements Component

app.add_system(SystemStage::PERCEPTION, build_ai_snapshots);
app.add_system(SystemStage::AI_PLANNING, orchestrator_tick);
```

### WorldSnapshot API (Critical — get this right)

```rust
pub struct WorldSnapshot {
    pub t: f32, pub player: PlayerState,
    pub me: CompanionState,        // NOT "my_stats"
    pub enemies: Vec<EnemyState>,   // NOT "threats"
    pub pois: Vec<Poi>,             // NOT "obj_pos"
    pub obstacles: Vec<IVec2>,
    pub objective: Option<String>,
}
pub struct CompanionState {
    pub ammo: i32, pub cooldowns: BTreeMap<String, f32>,
    pub morale: f32, pub pos: IVec2,
}
pub struct PlanIntent { pub plan_id: String, pub steps: Vec<ActionStep> }
```

### BehaviorGraph API

```rust
use astraweave_behavior::{BehaviorGraph, BehaviorNode, BehaviorContext, BehaviorStatus};
let root = BehaviorNode::Selector(vec![
    BehaviorNode::Sequence(vec![
        BehaviorNode::Condition("check_threat".into()),
        BehaviorNode::Action("throw_smoke".into()),
    ]),
    BehaviorNode::Sequence(vec![BehaviorNode::Action("move_to_objective".into())]),
]);
let graph = BehaviorGraph::new(root);  // 1 arg: BehaviorNode
let status = graph.tick(&BehaviorContext::new(snap));
```

### GOAP+Qwen3 Hybrid Arbiter (Common Pattern)

```rust
use astraweave_ai::arbiter::{AIArbiter, AIControlMode};

// Dual-executor setup: strategic (thinking) + fast (non-thinking)
let arbiter = AIArbiter::new(strategic_executor, Some(fast_executor), goap, bt);

// Single-executor backward compat:
// let arbiter = AIArbiter::with_single_executor(llm_executor, goap, bt);

arbiter.update(&snap);
match arbiter.mode() {
    AIControlMode::GOAP => goap_orchestrator.plan(world, &snap),
    AIControlMode::ExecutingLLM { step_index } => execute_step(plan, step_index),
    AIControlMode::BehaviorTree => bt_orchestrator.plan(world, &snap),
}
```

For all 7 usage patterns, testing patterns, and benchmarking: see `docs/current/ARCHITECTURE_REFERENCE.md`.

### Other Key Patterns

```rust
// Combat physics (astraweave-gameplay/src/combat_physics.rs)
let hits = perform_attack_sweep(&phys, attacker_id, &attacker_pos, &targets,
    attack_range, &mut stats_map, &mut parry_map, &mut iframe_map);

// SIMD movement (astraweave-math/src/simd_movement.rs)
update_positions_simd(&mut positions[..], &velocities[..], dt);

// Asset loading (async pattern)
pub async fn load_cell_from_ron(path: &Path) -> Result<CellData> {
    let content = tokio::fs::read_to_string(path).await?;
    Ok(ron::from_str(&content)?)
}
```

---

## Architecture

### AI-First Loop (Core Pattern)

```
Perception → Reasoning → Planning → Action
    ↓           ↓            ↓          ↓
WorldSnapshot  AI Model   PlanIntent  Tool Validation
```

### ECS System Stages (60 Hz deterministic tick)

The ECS scheduler is **deterministic single-threaded** per tick. Systems within a stage execute in the order they were registered; stages execute in canonical order (below). A prior `ParallelSchedule` scheduler was removed 2026-04-18 after a soundness audit — see `docs/audits/parallel_schedule_removal_2026-04-18.md` and the safety audit it references. Parallelism in AstraWeave lives at the subsystem level (rayon for terrain meshing and fluids SPH; tokio for async I/O, LLM inference, asset streaming; GPU compute for rendering).

`App::new()` registers eight stages; the canonical execution order is:

1. **PRE_SIMULATION** — Setup, initialization
2. **PERCEPTION** — Build WorldSnapshots, update AI sensors
3. **SIMULATION** — Game logic, cooldowns, state updates
4. **SYNC** — ECS→legacy-World propagation (for `astraweave-core::ecs_adapter::build_app` consumers)
5. **AI_PLANNING** — Generate PlanIntents from orchestrators
6. **PHYSICS** — Apply forces, resolve collisions
7. **POST_SIMULATION** — Cleanup, constraint resolution
8. **PRESENTATION** — Rendering, audio, UI updates

### Key Crate Domains

- **Core**: `ecs`, `core`, `math`, `sdk` — foundation types, WorldSnapshot, SIMD math
- **AI**: `ai`, `behavior`, `llm`, `memory`, `director`, `npc`, `dialogue`, `coordination` — orchestration, GOAP, behavior trees, LLM integration
- **Rendering**: `render`, `materials`, `asset`, `asset-pipeline` — PBR, IBL, clustered lighting
- **Physics/World**: `physics`, `nav`, `terrain`, `fluids`, `scene` — Rapier3D, navmesh, procedural terrain
- **Gameplay**: `gameplay`, `quests`, `weaving`, `cinematics`, `pcg` — combat, crafting, quest systems
- **Networking**: `net`, `net-ecs`, `persistence-ecs` — snapshot networking, delta compression
- **Tools**: `tools/aw_editor` (9,425 test annotations, unified engine pipeline), `tools/aw_asset_cli`, `tools/aw_build`

All crate names are prefixed with `astraweave-`.

> **Agents**: For the full dependency graph, public API surface per crate, integration seams with risk levels, and known architectural anomalies (e.g. `terrain` → `gameplay` reverse dep, `render` → `aw_asset_cli` tool dep), see **`docs/architecture/ARCHITECTURE_MAP.md`**. Read it before any cross-crate modification, shared type change, or dependency analysis. For subsystem-internal work, see the relevant **`docs/architecture/<system>.md`** trace.

### Where to Look

| Need | Location |
|------|----------|
| **Architecture Traces** | **`docs/architecture/<system>.md`** — per-subsystem forensic reference. See the generated trace table under **Architecture Trace Maintenance** above for the full list. Toolkit at `docs/architecture/_meta/`. |
| **Architecture Map** | **`docs/architecture/ARCHITECTURE_MAP.md`** — dependency graph, API surface, seams, data flows |
| AI Systems | `astraweave-ai/src/{orchestrator,tool_sandbox,core_loop}.rs` |
| ECS Internals | `astraweave-ecs/src/{archetype,system_param,events}.rs` |
| Rendering | `astraweave-render/src/{lib,material,skinning_gpu,vertex_compression}.rs` |
| Physics | `astraweave-physics/src/{lib,spatial_hash}.rs` — the character controller lives in `lib.rs:424-535`; there is no `character_controller.rs` file |
| Combat | `astraweave-gameplay/src/combat_physics.rs` |
| SIMD Math | `astraweave-math/src/{simd_vec,simd_mat,simd_quat,simd_movement}.rs` |
| Terrain | `astraweave-terrain/src/{voxel_mesh,biome_pack,biome,scatter}.rs` |
| Blend Import | `crates/astraweave-blend/src/{decomposer,texture_processor,importer}.rs` |
| Build Config | `.cargo/config.toml`, root `Cargo.toml` |

---

## Guardrails & Verification

### Formal Verification (Miri & Kani)

Any new or modified `unsafe` code **MUST** pass both verification pipelines:

1. **Miri** (UB detection): `cargo +nightly miri test -p <crate> --lib -- --test-threads=1`
   - Flags: `-Zmiri-symbolic-alignment-check -Zmiri-strict-provenance`
   - CI: `.github/workflows/miri.yml` (weekly, nightly toolchain)
   - Validated crates: `ecs`, `math`, `core`, `sdk` — 977 tests, ZERO undefined behavior

2. **Kani** (formal proof): `cargo kani --package <crate>`
   - CI: `.github/workflows/kani.yml`
   - Proofs: `astraweave-sdk/src/lib_kani.rs`, `astraweave-ecs/tests/mutation_resistant_comprehensive_tests.rs`
   - Validated crates: `ecs`, `math`, `sdk`

3. **Requirements for unsafe code**:
   - Must pass Miri locally before committing
   - Must have a corresponding Kani proof or Kani-mirror test
   - Must include a `// SAFETY:` comment explaining the invariant
   - Must be validated in CI before merge

### Known Build Issues

**All previously-listed build breakages are resolved** (live `cargo check` audit, 2026-06-10): `ui_controls_demo`, `debug_overlay` (former egui/winit drift), `astraweave-author`, `rhai_authoring` (former Rhai `Sync` trait errors), and `astraweave-llm` all compile clean, and `cargo check --workspace` passes 130/130 members with 0 errors. The root `Cargo.toml` `[workspace.metadata.ci-excludes]` problematic list is empty.

- **Residual warnings (deferred)**: 1 `dead_code` warning in `astraweave-ai`; 1 unused import in `tools/aw_editor/src/gizmo/mod.rs:32`; `nalgebra v0.26.2` future-incompat note from the dependency graph
- **`.unwrap()` in test code only**: All `.unwrap()` calls are inside `#[cfg(test)]` modules — justified for test assertions. Zero production-path unwraps in engine runtime crates.

### Documentation Hazards

**Aspirational documentation drift.** A 2025-09-08 commit (`28bc94f21`, "Create comprehensive bespoke wiki with 51-section documentation structure (#34)", authored by GitHub Copilot bot) added ~80 doc files under `docs/src/` referencing types that do not exist in the code. Affected surfaces include:

- `docs/src/core-systems/audio.md` — references `AudioConfig`, `AudioBackend`, `AudioListener`, `SpatialSound`, `AttenuationModel`, `ReverbZone`, `MusicManager`, `MusicLayer`, `SfxManager`, `SoundPool`, `AudioMixer` — **none of which exist in `astraweave-audio/src/lib.rs`'s re-exports**
- `docs/src/core-systems/input.md` — references `InputSystem`, `InputConfig`, `ActionMap`, `BindingRecorder`, `BindingProfile`, `ContextPriority`, `InputBuffer`, `InputPredictor`, `InputRecorder` — none of which exist in `astraweave-input/src/lib.rs`'s re-exports
- `docs/src/core-systems/networking.md` — claims QUIC via Quinn; actual implementation is WebSocket via tokio-tungstenite over TCP (architectural-class mismatch)

Treat `docs/src/` content as **historical/aspirational** unless cross-validated against actual `pub use` re-exports in the relevant crate's `lib.rs`. The architecture traces in `docs/architecture/` are the falsification mechanism.

**Doc-comment migration drift.** Doc-comments and CLAUDE.md frequently describe target state ahead of runtime wiring. Confirmed examples:

- **AI runtime model**: doc-comments and CLAUDE.md describe Qwen3-based hybrid; `astraweave-ai/src/orchestrator.rs:488-490` defaults `OLLAMA_MODEL` to `"phi3:medium"`. Set `OLLAMA_MODEL=qwen3:8b` to get the documented behavior, or update the default if Qwen3 is the canonical choice.
- **Networking signature (RESOLVED 2026-06-10, Net-Trio-Remediation W.1–W.5)**: the former mismatch (client computed XOR `sign16`, server verified HMAC-SHA256, so every verification failed and the server only warned) is fixed end-to-end — `aw-net-proto` exposes the canonical HMAC-SHA256 surface (`SigningKey`, `sign`/`verify`, `input_frame_sig_payload`), the `sign16` stub is deleted, the client signs via that surface, and the server verifies FIRST with `SignatureFailurePolicy::Kick` by default (WebSocket Close 1008). `net/README.md` was rewritten to match. Deliberate boundaries, not defects: no replay/freshness protection; server→client messages unsigned (asymmetric-trust design). See `docs/architecture/net_ecs.md` rev 1.3 and `docs/audits/net_trio_signature_remediation_findings_2026-06.md`.
- **HNSW vector index**: `astraweave-embeddings/src/lib.rs:9` advertises HNSW with `hnsw_rs` dependency declared and feature default-on; actual `VectorStore::search` is a linear scan over a DashMap.
- **SpatialHash broadphase**: `astraweave-physics/src/lib.rs:25-26` doc-comment advertises `SpatialHash` as broadphase ("99.96% pair reduction"); actual broadphase is Rapier's `DefaultBroadPhase`. The in-crate `SpatialHash` (1,038 LoC) is dormant.
- **Multiple "stub" surfaces**: `astraweave-llm/src/llm_adapter.rs::safe_llm_invoke` has zero workspace callers (the `MAX_PROMPT_LENGTH = 4096` invariant is unreachable code); `astraweave-persistence-ecs::auto_save_system` is a comment-only TODO; `compute_poses_stub` in scene; `process_destructible_hits` no-op stub in physics.

When a doc-comment describes desired behavior, treat it as a hypothesis to verify against the code path, not as ground truth.

### Key Lessons (Apply to All Future Work)

1. **Batching > Scattering**: ECS collect/writeback 3-5x faster than scattered `get_mut()`
2. **Only parallelize >5ms workloads** (Rayon overhead ~50-100 us)
3. **Trust glam auto-vectorization** (80-85% of hand-written AVX2)
4. **Cache locality cascades**: Spatial hash improved ALL systems 9-17%
5. **API verification first**: Read actual struct definitions AND `rg 'struct <Name>'` workspace-wide for parallel definitions before generating code (dual TerrainVertex / shadow-layout / FastPreview pipelines each cost multi-day cleanups)
6. **Case sensitivity matters**: snake_case vs PascalCase mismatch caused 100% false positives
7. **Silent failures cost weeks**: `let _ =` and `.ok()` on `Result` hide the bugs that produce the longest debugging sessions (12 CRITICAL editor findings traced here)
8. **Wired beats tested**: A subsystem with passing tests and zero production callers is dormant code, not a feature. The workspace currently carries ~200K LoC across this taxonomy:
   - **In-design-but-tested** — passes tests, zero workspace callers (Memory pipeline ~11K, Coordination crate ~5.3K, Advanced GOAP ~16.7K, LLM Production Hardening ~15K, RAG stack ~12.3K, Fluids ~84K, Dialogue LLM layer ~2.9K, NPC isolated subsystem)
   - **Dormant scaffolding** — module exists, body is TODO comments (`auto_save_system`, `replay_system` event application, `safe_llm_invoke`, `compute_poses_stub`)
   - **Orphan source** — file on disk, not declared as a module (`astraweave-net-ecs/src/lib_temp.rs`, `archive/temp_files/temp/temp_lib.rs`, `astraweave-ai/src/rag/`, `astraweave-ai/src/persona/`)
   - **Declared-but-unused Cargo deps** — listed in Cargo.toml, zero `use` statements (`astraweave-author` + `astraweave-observability` in aw_editor; `astraweave-llm` + `astraweave-embeddings` + `astraweave-rag` in astraweave-memory)
   - **Dormant feature flags** — gate zero `#[cfg(feature = "X")]` sites (`editor-graphs`, `editor-materials`, `editor-terrain`, `editor-nav`, `editor-sim`, `editor-full`)
   - **Aspirational-doc-only types** — referenced in `docs/src/` but not in any `pub use` (the 28bc94f21 wiki sweep, see Documentation Hazards)

   Verify via the Integration Completeness checklist. See `docs/architecture/` traces for per-subsystem inventory.
9. **Phase numbering is local, not global**: References to "Phase 1.1", "Phase 4.2", "Phase 5.3 T7 stage 3a", "Phase 7 Arbiter", "Phase PBR-E", etc. reflect parallel campaigns, not a unified timeline. Cross-reference against the relevant `docs/current/PHASE_*.md` or campaign-specific doc; don't try to linearize them.

### Documentation Organization

All new documents must be categorized before creation:

- **Current/ongoing work** → `docs/current/`
- **Completed phases/weeks/days** → `docs/journey/{phases,weeks,daily}/`
- **Lessons & patterns** → `docs/lessons/`
- **Setup & reference** → `docs/supplemental/`
- **Subsystem architecture traces** → `docs/architecture/`
- **Trace toolkit prompts** → `docs/architecture/_meta/`

**Never create files in root `docs/`.** Preserve git history with `git mv`.

### Platform Notes

- Windows builds set a 16MB stack size (configured in `.cargo/config.toml`) due to large State structs.

---

## Reference Files

Read these when you need deeper context. **Do not ask the user for information that exists in these documents.**

| File | Contains |
|------|----------|
| `docs/architecture/<system>.md` | **Per-subsystem architecture traces** — forensic reference for traced subsystems (full list in the generated trace table under **Architecture Trace Maintenance**). Read the relevant trace before modifying a traced subsystem; update it in the same commit when you do. Toolkit at `docs/architecture/_meta/`. |
| `docs/architecture/ARCHITECTURE_MAP.md` | **START HERE for cross-crate work.** Full dependency graph, public API surface, integration seams, editor viewport pipeline (unified post-Fix-27), data flow paths, unsafe code inventory. |
| `docs/current/EDITOR_BEHAVIORAL_CORRECTNESS_AUDIT.md` | 37-fix behavioral correctness audit: visual math, data pipeline, undo system, silent failures, integration seams. Completed 2026-04-05. |
| `docs/current/EDITOR_MULTI_TOOL_ARCHITECTURE_CAMPAIGN.md` | 8-round diagnostic narrowing that elevated the §7.7 wrapped-component resource identity trap to structural axiom. Sub-phases 3 (incl. Real-Fix.A–E) and 4 complete; Sub-phase 5 in flight (5.A/5.B landed 2026-06-06). |
| `docs/current/FIX27_UNIFIED_PIPELINE_CAMPAIGN.md` | 7-phase campaign plan that eliminated the dual FastPreview/EnginePBR rendering pipeline. |
| `docs/current/PROJECT_STATUS.md` | Current state, active work, recently completed milestones |
| `docs/current/ARCHITECTURE_REFERENCE.md` | Full API patterns (7 arbiter patterns, testing, benchmarking), performance data, formal verification details |
| `docs/current/DOCUMENTATION_INDEX.md` | Master navigation for all project documentation |
| `docs/current/MASTER_ROADMAP.md` | Strategic planning, prioritized action items |
| `docs/current/MASTER_BENCHMARK_REPORT.md` | Performance baselines per crate |
| `docs/current/MASTER_COVERAGE_REPORT.md` | Test coverage by priority tier |
| `docs/current/MIRI_VALIDATION_REPORT.md` | Miri validation details (977 tests, 0 UB) |
| `docs/current/BULLETPROOF_VALIDATION_PLAN.md` | Miri + Kani + mutation testing plan |
| `.github/copilot-instructions.md` | Detailed behavioral directives and code patterns |

---

**Version**: 0.10.1 | **Rust**: 1.89.0 | **License**: MIT | **Status**: Miri + Kani Validated
