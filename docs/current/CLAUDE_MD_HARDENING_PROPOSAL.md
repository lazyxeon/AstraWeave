# CLAUDE.md Hardening Proposal

**Date:** 2026-05-06
**Author:** Audit synthesis (AI-driven)
**Status:** Draft for human review — do not apply automatically.

---

## Executive Summary

I read CLAUDE.md, the two named audits (`EDITOR_BEHAVIORAL_CORRECTNESS_AUDIT.md`, `FIX27_UNIFIED_PIPELINE_CAMPAIGN.md`), the parallel-schedule audit chain, and ten additional `docs/audits/*` files spanning render divergence, terrain material flow, multi-tool architecture, panel registration, pointer-event dispatch, heightmap generation, test-suite remediation, and the production-readiness audit. The 37 editor fixes, the 7-phase Fix-27 campaign, and ~25 other audit findings collapse into **three dominant classes of misjudgment** that recur in 4+ audits each and that the current CLAUDE.md does **not** prevent: (1) silent `Result` discarding, (2) dual implementations of the same logical system, and (3) "completed" code that has zero production call sites or partial multi-surface registration.

**Proposed:** four amendments — one new short subsection ("Integration Completeness") and three in-place tightenings of existing language (Error Handling Policy, Scope Discipline, Key Lessons). **Net delta: +245 words.** No new top-level sections.

**Top-3 highest-leverage changes:**
1. **Integration Completeness checklist** (new subsection, +120 words) — collapses the "dormant infrastructure" + "multi-surface registration gap" + "zombie config field" classes into one pre-completion checklist. Targets the single most frequent audit-grade failure mode.
2. **Forbid silent `Result` discarding** (~+50 words under Error Handling Policy) — extends the existing `.unwrap()` rule to `let _ =` and `.ok()`. Directly addresses the C-1/C-2/C-4 cluster (12 CRITICAL + 18 HIGH editor findings).
3. **No-second-pipeline rule** (~+55 words under Scope Discipline) — encodes the Fix-27 lesson directly so the next agent does not silently rebuild a parallel renderer / vertex format / scheduler.

---

## Phase 1 Notes — Class extraction

Across 12 audits I identified **eight underlying classes**. Six recur in ≥3 audits; two are domain-specific (rendering math). Frequency table:

| # | Class | Audit hits | CLAUDE.md prevents today? |
|---|-------|-----------|---------------------------|
| 1 | Silent `Result` discarding (`let _ =`, `.ok()`, swallowed errors) | 6+ | No — only `.unwrap()` is named |
| 2 | Dual implementations of one logical system (FastPreview/EnginePBR, TerrainVertex 28B/80B, two schedulers, two tonemaps) | 5 | No |
| 3 | Dormant infrastructure: tested but zero production callers (`ParallelSchedule`, render-graph DAG, `AdvancedErosionSimulator`, `RegionalArchetypePanel`, splat-map pipeline) | 5 | No — "Production first" lesson is too vague |
| 4 | Multi-surface registration partial-application (panel needs 11 surfaces, F.5-paint touched 1; pointer-event dispatcher missing 3 sites) | 4 | No |
| 5 | Zombie config fields — UI-exposed, never read (`RenderMode::FastPreview`, `tonemap_operator`, `set_world_archetype`) | 3 | No |
| 6 | Documentation-ahead-of-implementation (campaign plan claims phases shipped; code says no) | 4 | Partial — "Maintain Context" rule exists but is generic |
| 7 | BRDF / energy-conservation / reference-renderer-parity errors | 3 | No, and **shouldn't** — too domain-specific |
| 8 | Numerical edge-case (epsilon placement, denominator order) | 2 | No, and shouldn't — domain-specific |

Classes 7–8 are deferred to render-domain docs and the `shader-wgsl-reviewer` agent (see Bucket C below). Class 6 is partial overlap with existing language; I propose tightening rather than adding. Classes 1–5 form the Bucket A core.

---

## Phase 3 — Bucket A Edits (true gaps)

### Edit 1: Forbid silent `Result` discarding

**Bucket:** A
**Trigger audits:** `EDITOR_BEHAVIORAL_CORRECTNESS_AUDIT.md` C-1, C-2, C-4 (12 CRITICAL + 18 HIGH); `editor_viewport_render_divergence_2026-04-19.md`; `f5_paint_panel_registration_diagnostic_2026-05-03.md`.
**Class of mistake prevented:** Treating fallible operations as infallible by suppressing `Result` with `let _ =` or `.ok()`, leaving features broken with zero diagnostics.

**Location in CLAUDE.md:** existing `### Error Handling Policy` subsection.

**Proposed change:**

[BEFORE]
```
### Error Handling Policy

- **FIX ALL COMPILATION ERRORS** — zero tolerance. Never leave broken code.
- **Warnings may be deferred** — document for future cleanup.
- Run `cargo check -p <crate>` after **every** code change. This is mandatory.
```

[AFTER]
```
### Error Handling Policy

- **FIX ALL COMPILATION ERRORS** — zero tolerance. Never leave broken code.
- **Warnings may be deferred** — document for future cleanup.
- Run `cargo check -p <crate>` after **every** code change. This is mandatory.
- **Never discard `Result` on user-facing fallible operations** (asset I/O, GPU state, file ops, prefab/scene reload, watcher creation). Use `?.context("…")` or a named recovery function. `let _ =` and `.ok()` on such calls are forbidden — they produced 12 CRITICAL + 18 HIGH editor-audit findings where features silently no-op'd.
```

**Word count delta:** +52 words.
**Why this earns its space:** Highest-frequency class in the entire audit corpus. Trivially testable by `rg 'let _ =.*\?'` and `rg '\.ok\(\);'` in code review. Specific enough to change behavior in a single line of code review.

---

### Edit 2: No-second-pipeline rule (dual-path divergence)

**Bucket:** A
**Trigger audits:** `FIX27_UNIFIED_PIPELINE_CAMPAIGN.md` (entire 7-phase campaign); `editor_viewport_render_divergence_2026-04-19.md`; `terrain_material_flow_investigation_2026-04-19.md`; `tonemap_double_application_investigation_2026-04-19.md`.
**Class of mistake prevented:** Building a parallel implementation of a logical system (renderer, vertex format, scheduler, tonemap chain) and letting it drift from the original.

**Location in CLAUDE.md:** existing `### Scope Discipline` subsection.

**Proposed change:**

[BEFORE]
```
### Scope Discipline

Make ONLY the changes requested. Do not refactor, rename, reorganize, or "improve" adjacent code unless specifically asked. Do not add features, abstractions, or "nice to haves" beyond the stated task.
```

[AFTER]
```
### Scope Discipline

Make ONLY the changes requested. Do not refactor, rename, reorganize, or "improve" adjacent code unless specifically asked. Do not add features, abstractions, or "nice to haves" beyond the stated task.

**Never build a second implementation of a logical system that already exists** (rendering path, vertex format, material pipeline, scheduler, tonemap chain, scene serializer). Before adding any such system, run `rg 'struct <Name>\|trait <Name>'` workspace-wide; if a peer implementation exists, extend it or surface the conflict to the user. The Fix-27 campaign and the editor-render-divergence audit each took weeks to unwind a duplicate pipeline that was created without this check.
```

**Word count delta:** +78 words.
**Why this earns its space:** Fix-27 is the single largest cleanup campaign in the project's history; editor-render divergence is its sibling. Rule is specific (names the systems that recurred), testable (the `rg` invocation), and triggers exactly when it should — at the moment a new module/struct is being created.

---

### Edit 3: Integration Completeness checklist

**Bucket:** A
**Trigger audits:** `parallel_schedule_removal_2026-04-18.md` (ParallelSchedule + render-graph DAG had zero production callers); `f5_paint_panel_registration_diagnostic_2026-05-03.md` (1 of 11 registration surfaces touched); `g_pointer_events_diagnostic_2026-05-03.md`; `advanced_erosion_static_audit_2026-04-23.md` (902 LOC, 0 callers); `editor_viewport_render_divergence_2026-04-19.md` (`RenderMode::FastPreview` UI toggle never read).
**Class of mistake prevented:** Declaring work complete when (a) the new code has no production caller, (b) only some of the required registration surfaces were updated, or (c) a UI-exposed config field is set but never consumed.

**Location in CLAUDE.md:** new short subsection in `## Workflow & Process`, immediately after `### Development Workflow`. Justified by frequency: this single check would have prevented at least five major audit-grade campaigns.

**Proposed change:**

[BEFORE]
*(new)*

[AFTER]
```
### Integration Completeness (before declaring work complete)

A feature is incomplete until it is wired end-to-end. Before marking any task done, verify all three:

1. **Production caller exists.** Every new public type, function, or module must have ≥1 non-test, non-feature-gated call site. Run `rg '<Name>::new\|<fn>\(' --type rust -g '!*test*' -g '!benches/*'`. Zero matches = dormant code (this is how `ParallelSchedule`, the render-graph DAG, `AdvancedErosionSimulator`, and `RegionalArchetypePanel` all shipped — and were later removed or rewritten).
2. **All registration surfaces touched.** When adding an enum variant, panel type, component, or system, list every match arm, registry, and initializer in a comment first. After editing, `cargo check` must pass without exhaustiveness warnings AND `rg <new_name>` must show every site updated. Panel registration alone has 11 surfaces — the F.5-paint diagnostic is the canonical failure.
3. **Every UI/API-exposed config field is read.** A field that is settable but never observed downstream is a bug. Confirm with a grep for the field name in the consuming subsystem before completion.
```

**Word count delta:** +160 words.
**Why this earns its space:** Bundles three distinct recurring classes (dormant infrastructure, multi-surface registration, zombie fields) into one checklist with one trigger ("before declaring complete"). Each item is grep-testable. This is the single highest-leverage addition in this proposal — at least five named audit campaigns would never have happened if a fresh agent had executed this checklist.

---

## Phase 3 — Bucket B Edits (strengthen existing language)

### Edit 4: Tighten "Key Lessons" — replace two weakest items

**Bucket:** B
**Trigger audits:** Same as Edits 1 + 3.
**Class of mistake prevented:** Two of the existing lessons ("Production first: Working demo over 100% test coverage" and "Debug early") are too vague to trigger behavior change. The first is actually misleading given that the dominant failure mode is "tested but never wired." Replace with sharpened versions of the same intent.

**Location in CLAUDE.md:** existing `### Key Lessons (Apply to All Future Work)` list.

**Proposed change:**

[BEFORE]
```
5. **API verification first**: Always read actual struct definitions before generating code
6. **Case sensitivity matters**: snake_case vs PascalCase mismatch caused 100% false positives
7. **Debug early**: One debug log revealed a critical validation bug
8. **Production first**: Working demo over 100% test coverage
```

[AFTER]
```
5. **API verification first**: Read actual struct definitions AND `rg 'struct <Name>'` workspace-wide for parallel definitions before generating code (dual TerrainVertex / shadow-layout / FastPreview pipelines each cost multi-day cleanups)
6. **Case sensitivity matters**: snake_case vs PascalCase mismatch caused 100% false positives
7. **Silent failures cost weeks**: `let _ =` and `.ok()` on `Result` hide the bugs that produce the longest debugging sessions (12 CRITICAL editor findings traced here)
8. **Wired beats tested**: A subsystem with passing tests and zero production callers is dormant code, not a feature — verify via the Integration Completeness checklist
```

**Word count delta:** +47 words (lessons 5 and 8 expanded; lesson 7 replaced).
**Why this earns its space:** Cost is small; reuses an existing list slot rather than adding new structure. Sharpens three lessons whose current wording either misleads (lesson 8) or fails to trigger (lessons 5, 7). Each new phrasing names a concrete grep or symptom.

---

## Phase 3 — Summary of Word Budget

| Edit | Bucket | Net words |
|------|--------|-----------|
| 1. Silent `Result` discarding | A | +52 |
| 2. No-second-pipeline | A | +78 |
| 3. Integration Completeness | A | +160 |
| 4. Tighten Key Lessons | B | +47 |
| **Total net additions** | | **+337** |

Within the ~400-word budget. No top-level sections added; one new subsection (`### Integration Completeness`) under an existing `##` heading.

---

## Bucket C — Out-of-scope deferrals (do NOT add to CLAUDE.md)

These are real lessons but belong elsewhere:

| Lesson | Recommended home |
|--------|------------------|
| Energy-conservation / Fresnel / multi-scatter compensation rules for BRDFs | `docs/lessons/render_brdf_correctness.md` (new) and the `shader-wgsl-reviewer` agent system prompt |
| Reference-renderer parity testing (Filament/Godot per-pixel RMSE) | `docs/current/ARCHITECTURE_REFERENCE.md` rendering section |
| Numerical edge cases (epsilon placement, NoV→0, roughness→0) | Same as above |
| Nested-HashMap concurrency model audit (`World::resources` → `Events::queues`) | `docs/audits/parallel_schedule_safety_audit_2026-04-18.md` already documents this; the `ParallelSchedule` is removed so the rule has low future-trigger frequency |
| Pre-measurement benchmark validation ("can this test workload actually parallelize?") | `docs/current/MASTER_BENCHMARK_REPORT.md` methodology section |
| Poisoned-lock recovery helper for `Renderer`/`Surface`/`Physics` locks | `astraweave-render` and `astraweave-physics` module-level docs |
| Documentation-ahead-of-implementation (campaign plans treated as state) | Strengthen `docs/current/MASTER_ROADMAP.md` header to mark "PLAN — awaiting approval" entries non-authoritatively; not a CLAUDE.md concern |
| Miri/Kani-at-same-commit-as-unsafe enforcement | `.github/copilot-instructions.md` already covers; CI workflow `kani.yml` is the enforcement point |
| Architecture-Map currency (claims about deleted code) | Update protocol for `ARCHITECTURE_MAP.md` itself |

Routing recommendation: Andrew should consider a single-page `docs/lessons/AGENT_FAILURE_CLASSES.md` that aggregates Bucket C with audit citations, so future agents can pull it on-demand without bloating the always-loaded CLAUDE.md.

---

## Phase 4 — Anti-Patterns (rejected edits)

I considered and rejected the following proposed edits during Phase 4 self-review:

| Proposed edit | Reason rejected |
|---------------|-----------------|
| "Validate all BRDF formulas against Filament reference" | Domain-specific — fails the 30%-of-sessions trigger bar; belongs in render docs |
| "Always run `cargo +nightly miri test` before committing unsafe code" | Already covered in §5 Formal Verification; would be duplicative |
| "When changing public API, update ARCHITECTURE_MAP.md" | Already implied by existing "ARCHITECTURE_MAP.md is authoritative for cross-crate work" line; adding it would be cargo-cult repetition |
| "Run `rg 'let _ =' --type rust` workspace-wide every session" | Too process-heavy; the Edit-1 rule triggers at the right moment (when an agent is about to write `let _ =`) |
| "Never use `if let Ok(mut lock)` without an else branch" | Real lesson but very render/lock-specific; belongs in module-level docs for `Renderer`/`Surface` |
| "Add a panic-injection test for every poisonable lock" | Same — too narrow, low frequency |
| "Document all parallel-schedule lessons in CLAUDE.md" | The scheduler is removed; rule frequency = ~0 |
| "Require a roadmap entry for every feature-gated module" | Genuinely useful but redundant with Edit-3 item 1 (production-caller check); would inflate without leverage |
| "When reading a campaign plan, run `git log --stat` to verify shipped state" | Too process-heavy; the symptom is cleaner reframed as Edit-3 item 1 ("zero callers = not shipped, regardless of what the plan says") |
| "Always compare two renderers via screenshot diff before merging" | Domain-specific; the more general no-second-pipeline rule (Edit 2) prevents the situation upstream |
| "Add a `// NOTE: registered in N places: …` comment for every multi-site change" | Captured inside Edit-3 item 2 — adding it as a separate top-level rule would bloat |
| "Strengthen `Maintain Context` rule with explicit doc-trust caveat" | Borderline; Edit-3 item 1 covers the failure mode by symptom rather than process |
| "Add a section: 'Concurrency Discipline' summarizing parallel-schedule lessons" | The scheduler was removed; rule has near-zero forward trigger rate; would dilute the constitution |

---

## Phase 5 — Final Amended CLAUDE.md (full text, with all proposed changes applied)

> The following is CLAUDE.md as it would read after Edits 1–4 are accepted. Diff this against the current file to apply.

````markdown
# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

AstraWeave is a **scientific proof of concept**: an AI-native game engine built **iteratively by AI with zero human-written code**. It uses a deterministic ECS architecture where AI agents are first-class citizens. The workspace contains 130 crates (~51 production + examples + tools). <!-- Source: CLAIMS_REGISTRY.md#workspace-members --> Rust toolchain is pinned at 1.89.0.

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

### Error Handling Policy

- **FIX ALL COMPILATION ERRORS** — zero tolerance. Never leave broken code.
- **Warnings may be deferred** — document for future cleanup.
- Run `cargo check -p <crate>` after **every** code change. This is mandatory.
- **Never discard `Result` on user-facing fallible operations** (asset I/O, GPU state, file ops, prefab/scene reload, watcher creation). Use `?.context("…")` or a named recovery function. `let _ =` and `.ok()` on such calls are forbidden — they produced 12 CRITICAL + 18 HIGH editor-audit findings where features silently no-op'd.

---

## Workflow & Process

### Chain of Thought

1. **Understand**: Analyze the request against mission-critical standards.
2. **Context**: Check `docs/current/` for latest state. Read reference files when needed. **For any cross-crate work, read `docs/architecture/ARCHITECTURE_MAP.md` first** — it contains the full dependency graph, integration seams, data flow paths, and blast-radius analysis.
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

A feature is incomplete until it is wired end-to-end. Before marking any task done, verify all three:

1. **Production caller exists.** Every new public type, function, or module must have ≥1 non-test, non-feature-gated call site. Run `rg '<Name>::new\|<fn>\(' --type rust -g '!*test*' -g '!benches/*'`. Zero matches = dormant code (this is how `ParallelSchedule`, the render-graph DAG, `AdvancedErosionSimulator`, and `RegionalArchetypePanel` all shipped — and were later removed or rewritten).
2. **All registration surfaces touched.** When adding an enum variant, panel type, component, or system, list every match arm, registry, and initializer in a comment first. After editing, `cargo check` must pass without exhaustiveness warnings AND `rg <new_name>` must show every site updated. Panel registration alone has 11 surfaces — the F.5-paint diagnostic is the canonical failure.
3. **Every UI/API-exposed config field is read.** A field that is settable but never observed downstream is a bug. Confirm with a grep for the field name in the consuming subsystem before completion.

### Build Strategy

**DO:**
- Build incrementally (`-p` flag for single crates)
- Use the editor cargo aliases (`editor`, `editor-release`, `editor-dev`); for workspace ops use explicit cargo commands
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

# Workspace-wide (no aliases defined — use the explicit forms)
cargo check --workspace                                   # Workspace check
cargo build -p astraweave-core -p astraweave-ecs -p astraweave-math -p astraweave-ai  # Core components
cargo test --workspace                                    # Workspace tests
cargo clippy --workspace --all-features -- -D warnings    # Full linting

# Editor
cargo editor                        # Run editor (release-fast profile)
cargo editor-release                # Run editor (full release)
cargo editor-dev                    # Run editor (debug)

# Examples
cargo run -p hello_companion --release   # Flagship AI demo (7 modes) <!-- Source: CLAIMS_REGISTRY.md#ai-modes -->
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
- **Tools**: `tools/aw_editor` (~9,427 test annotations, unified engine pipeline), `tools/aw_asset_cli`, `tools/aw_build` <!-- Source: CLAIMS_REGISTRY.md#editor-test-markers -->

All crate names are prefixed with `astraweave-`.

> **Agents**: For the full dependency graph, public API surface per crate, integration seams with risk levels, and known architectural anomalies (e.g. `terrain` → `gameplay` reverse dep, `render` → `aw_asset_cli` tool dep), see **`docs/architecture/ARCHITECTURE_MAP.md`**. Read it before any cross-crate modification, shared type change, or dependency analysis.

### Where to Look

| Need | Location |
|------|----------|
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
   - Validated crates: `ecs`, `math`, `core`, `sdk` — 1,059 tests, ZERO undefined behavior

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

- **All previously-listed build breakages are resolved**: `ui_controls_demo`, `debug_overlay` (former egui/winit drift), `astraweave-author`, `rhai_authoring` (former Rhai `Sync` trait errors), and `astraweave-llm` all compile clean. `cargo check --workspace` passes 130/130 members with 0 errors; the root `Cargo.toml` `[workspace.metadata.ci-excludes]` problematic list is empty.
- **`.unwrap()` in test code only**: All `.unwrap()` calls are inside `#[cfg(test)]` modules — justified for test assertions. Zero production-path unwraps in engine runtime crates.

### Key Lessons (Apply to All Future Work)

1. **Batching > Scattering**: ECS collect/writeback 3-5x faster than scattered `get_mut()`
2. **Only parallelize >5ms workloads** (Rayon overhead ~50-100 us)
3. **Trust glam auto-vectorization** (80-85% of hand-written AVX2)
4. **Cache locality cascades**: Spatial hash improved ALL systems 9-17%
5. **API verification first**: Read actual struct definitions AND `rg 'struct <Name>'` workspace-wide for parallel definitions before generating code (dual TerrainVertex / shadow-layout / FastPreview pipelines each cost multi-day cleanups)
6. **Case sensitivity matters**: snake_case vs PascalCase mismatch caused 100% false positives
7. **Silent failures cost weeks**: `let _ =` and `.ok()` on `Result` hide the bugs that produce the longest debugging sessions (12 CRITICAL editor findings traced here)
8. **Wired beats tested**: A subsystem with passing tests and zero production callers is dormant code, not a feature — verify via the Integration Completeness checklist

### Documentation Organization

All new documents must be categorized before creation:

- **Current/ongoing work** → `docs/current/`
- **Completed phases/weeks/days** → `docs/journey/{phases,weeks,daily}/`
- **Lessons & patterns** → `docs/lessons/`
- **Setup & reference** → `docs/supplemental/`

**Never create files in root `docs/`.** Preserve git history with `git mv`.

### Platform Notes

- Windows builds set a 16MB stack size (configured in `.cargo/config.toml`) due to large State structs.

---

## Reference Files

Read these when you need deeper context. **Do not ask the user for information that exists in these documents.**

| File | Contains |
|------|----------|
| `docs/architecture/ARCHITECTURE_MAP.md` | **START HERE for cross-crate work.** Full dependency graph, public API surface, integration seams, editor viewport pipeline (unified post-Fix-27), data flow paths, unsafe code inventory. |
| `docs/current/EDITOR_BEHAVIORAL_CORRECTNESS_AUDIT.md` | 37-fix behavioral correctness audit: visual math, data pipeline, undo system, silent failures, integration seams. Completed 2026-04-05. |
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
````

---

## Recommendation

Apply Edits 1–4 as a single revision (bump CLAUDE.md to v0.10.1). Net cost is +337 words for four high-leverage rules, each tied to specific multi-week audit campaigns. If word budget pressure emerges later, Edit 4 (Key Lessons tightening) is the easiest to revert without losing core protection — Edits 1–3 are the load-bearing changes.

If Andrew prefers a more conservative pass, accept Edits 1 and 3 only (+212 words). These two cover the highest-frequency classes (silent failures + dormant infrastructure / partial registration) and would have prevented the majority of audit-grade rework on their own.
