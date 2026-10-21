# AstraWeave Visual Editor – World-Class Delivery Plan

**Audience:** AstraWeave phase leads, editor subsystem maintainers, QA automation  
**Scope:** `tools/aw_editor` and supporting crates (scene serialization, prefab, runtime, plugin, build, asset pipelines)  
**Objective:** Close the gap between the current feature-prototype state and a verifiably world-class, production-grade editor that matches the claims in `EDITOR_ROADMAP_TO_WORLD_CLASS.md`.

---

## 1. North-Star Definition

| Dimension | World-Class Target | Verification Method |
|-----------|-------------------|---------------------|
| **Authoring Fidelity** | True 3D scene graph with hierarchical transforms, component editing, and prefab overrides; change latency < 50 ms | Golden scene round-trips, gizmo latency profiling |
| **Runtime Loop** | Deterministic play/pause/step using the actual AstraWeave ECS/physics/audio stack at ≥60 FPS with 1k entities | Automated replay harness + performance counters |
| **Tooling Ecosystem** | Reliable asset import, hot-reload, build/package, plugin loading, theming/layout persistence | Tool-specific regression suites + manual UX checklist |
| **Defect Containment** | ≥85 % automated coverage on editor crate, zero known P0/P1, <5 P2 | `cargo llvm-cov` + defect dashboard |
| **UX Polish** | Multi-viewport layouts, contextual inspectors, undo/redo for every action, responsive asset browser (<100 ms interactions) | Ergonomic scorecard + telemetry |

Success is achieved when every phase exit criteria (Section 3) is green and validated by CI pipelines + manual review.

---

## 2. Strategic Pillars

1. **Authoritative Scene Core** – unify all editing flows around `EditorSceneState`/`World` so panels, gizmos, serialization, prefabs, and renderer share one source of truth.
2. **Real Runtime & Validation** – embed the actual engine loop inside the editor (no stubs) and expose instrumentation to both the UI and automated tests.
3. **Content & Asset Flow** – make asset import, prefab nesting, and hot-reload deterministic, undoable, and performance-aware.
4. **Ecosystem Tooling** – deliver the promised plugin system, build manager, theme/layout persistence, and scripting hooks with production parity.
5. **Quality Harness** – enforce coverage, telemetry, nightly stress runs, and doc alignment so claims stay accurate.

---

## 3. Phased Roadmap

### Phase 0 – Stabilization & Fact-Finding (1 week)
* **Goals**
  * Baseline current behavior, gather metrics, freeze misleading docs.
  * Stand up editor-specific CI (check, fmt, clippy, tests, minimal headless smoke).
* **Key Tasks**
  * Instrument `aw_editor` with `cargo llvm-cov` config; add to CI.
  * Capture current undo coverage, runtime behavior, prefab feature list (facts vs claims).
  * Annotate `EDITOR_ROADMAP_TO_WORLD_CLASS.md` with “UNDER REVISION” banner.
* **Exit Criteria**
  * CI job `editor-ci` running on PRs with coverage artifact.
  * Documented "as-is" metrics stored in `/docs/current/EDITOR_BASELINE.md`.
  * **Update (2025-12-08):** Runtime loop now executes the real ECS schedule with Tracy instrumentation—see `EDITOR_RUNTIME_VALIDATION.md` for validation details.

### Phase 1 – Scene Authority & Gizmo Integrity (2–3 weeks)
* **Goals**
  * Remove duplicate `EntityManager`/TransformPanel caches; operate on `EditorSceneState`.
  * Extend pose/serialization to true Vec3 + rotation/scale persistence.
  * Ensure gizmo edits, clipboard, paste, duplicate, delete all flow through `UndoStack`.
* **Key Tasks**
  * Refactor asset application (`handle_asset_action`) to mutate `World` + re-sync caches.
  * Update renderer & serialization for Y axis + hierarchy support.
  * Add tests covering copy/paste/undo and gizmo commit/cancel logic.
* **Exit Criteria**
  * Single source of truth for entity transforms.
  * Undo/redo menu accurately reflects every edit type.
  * Scene save/load proven by automated round-trip tests with ≥10 entities, varied transforms.

### Phase 2 – Runtime & Performance Core (3–4 weeks)
* **Goals**
  * Replace stub runtime with AstraWeave ECS loop (physics, AI, audio) and deterministic snapshots.
  * Wire Tracy / wgpu timestamp instrumentation into Performance panel.
  * Enable multi-viewport support using egui dock + shared render backend.
* **Key Tasks**
  * Embed `astraweave-core` scheduler; expose hooks for per-frame instrumentation.
  * Update `PerformancePanel` to consume live metrics + enable historical graphs.
  * Support multiple `ViewportWidget` instances (perspective, top, side) with layout persistence.
* **Exit Criteria**
  * Play/Pause/Step manipulates actual simulation (verified by deterministic replay test).
  * Performance HUD displays true frame time/entity counts within ±5 % of Tracy metrics.
  * Multi-viewport layout saved/restored via `EditorPreferences`.

### Phase 3 – Prefab, Asset, and Hot-Reload Maturity (3 weeks)
* **Goals**
  * Implement hierarchical prefab serialization, overrides (transforms + component payloads), and undoable apply/revert.
  * Deliver real-time asset hot reload for materials, prefabs, models, and Rhai scripts.
  * Make asset browser async with caching (no blocking disk IO on UI thread).
* **Key Tasks**
  * Redesign `PrefabData::collect_entity_recursive` to walk child graphs (using ECS relationships).
  * Implement prefab commands (`PrefabSpawn`, `PrefabApply`, `PrefabRevert`) with real deletes and coverage tests.
  * Introduce async thumbnail pipeline + progress indicators.
  * Expand `FileWatcher` to watch configured directories; dispatch reloads to appropriate subsystems.
* **Exit Criteria**
  * Prefab workflow end-to-end test suite enabled (currently commented) and passing.
  * Dragged prefabs update in-scene instantly when source files change.
  * Asset browser responsiveness <100 ms for directories ≤1k entries (profiling data logged).
  * **Update (2025-12-09):** Prefab capture now records hierarchy snapshots via PrefabHierarchySnapshot; see docs/current/EDITOR_PREFAB_HIERARCHY_VALIDATION.md for validation details and new prefab workflow tests.

### Phase 4 – Ecosystem Tools & UX Polish (3 weeks)
* **Goals**
  * Productionize Build Manager (invoke cargo/packaging, parse output, support platform presets).
  * Deliver plugin loader that discovers binaries/scripts in `plugins/`, enforces versioning, and sandboxes execution.
  * Complete theme/layout system (preset export/import, per-project overrides, accessible UI).
* **Key Tasks**
  * Implement build pipelines (Rust target selection, asset bundling, error surfacing, cancelation).
  * Plugin discovery + manifest format (toml/json), dynamic load/unload, panel registration.
  * Persist layout to project config; add multi-theme preview and validation.
* **Exit Criteria**
  * “Build & Run” produces executable for at least Win/Linux; logs show real compiler output.
  * Plugin sample demonstrates panel injection + event handling; integration tests cover lifecycle.
  * Layout/theme presets accessible from UI, stored per project, and validated on restart.

### Phase 5 – Quality Envelope & Release (2 weeks)
* **Goals**
  * Raise editor crate coverage ≥85 %, zero P0/P1 open, doc alignment with actual capabilities.
  * Run large-scene stress tests (10k entities, multiple viewports) to prove stability/perf claims.
  * Publish updated roadmap + marketing-ready feature checklist.
* **Key Tasks**
  * Expand smoke/integration tests (runtime, prefabs, build manager, plugins, hot reload).
  * Add telemetry dashboards (undo usage, fps, errors) to flag regressions.
  * Update docs (`EDITOR_ROADMAP_TO_WORLD_CLASS.md`, release notes) with evidence + links to tests/metrics.
* **Exit Criteria**
  * CI gate enforcing coverage + lint + scenario suites.
  * Stress scenario report checked into `docs/current/EDITOR_STRESS_REPORT.md`.
  * Executive sign-off on doc accuracy.

---

## 4. Cross-Cutting Enablers

1. **Testing & Automation**
   * Adopt `cargo nextest` with feature-gated GPU tests.
   * Introduce headless UI harness (winit event replay) for gizmo/viewport flows.
   * Nightly “editor-e2e” pipeline: import assets → edit → prefab → save → build → load.

2. **Telemetry & Observability**
   * Emit tracing spans for every command execution + undo/redo event.
   * Log asset browser metrics (scan time, thumbnail hits).
   * Add editor-specific dashboards to `astraweave-observability`.

3. **Documentation & Training**
   * Maintain `docs/current/EDITOR_USER_GUIDE.md` updated per phase exit.
   * Provide SOPs for plugin authors, build packaging, prefab workflows.

---

## 5. Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Legacy UI caches resist unification | Bugs/duplication persist | Incrementally delete unused structures; gate merges on single source-of-truth tests |
| Engine runtime integration destabilizes editor | Frequent crashes during play mode | Introduce feature flag, run engine loop in background thread with panic boundaries until stable |
| Hot-reload watchers cause performance issues | UI hitching, dropped events | Debounce with task queues, add benchmarks + telemetry to tune thresholds |
| Plugin execution compromises security | User scripts crash editor | Sandbox plugin context, require explicit allowlist, run in separate thread/process where possible |

---

## 6. Deliverable Tracking

| Phase | Artifact | Owner | Due |
|-------|----------|-------|-----|
| 0 | `EDITOR_BASELINE.md` | Editor Lead | Week 1 |
| 1 | `SCENE_CORE_REFAC.md`, new gizmo tests | Scene Pod | Week 4 |
| 2 | `EDITOR_RUNTIME_VALIDATION.md`, multi-viewport demo | Runtime Pod | Week 7 |
| 3 | `PREFAB_SYSTEM_SPEC.md`, hot-reload matrix | Content Pod | Week 10 |
| 4 | `PLUGIN_SDK.md`, Build manager SOPs | Tooling Pod | Week 13 |
| 5 | `EDITOR_STRESS_REPORT.md`, updated roadmap | QA + PM | Week 15 |

Progress reviews occur at the end of each phase with sign-off from AI lead + PM. No roadmap status moves to “Complete” without linked evidence (tests, docs, demos).

---

## 7. Next Actions

1. Approve Phase 0 scope and assign pod owners.
2. Spin up `editor-ci` pipeline and capture baseline coverage/perf data.
3. Schedule weekly war-room to track roadmap execution and unblock cross-crate changes.

Once Phase 0 is green, proceed sequentially—each phase builds on the previous pillars to ensure the final editor is not only feature-complete but also validated, performant, and trustworthy.***
