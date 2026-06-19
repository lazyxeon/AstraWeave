# AstraWeave Editor: Transformation Plan

This plan tracks the end-to-end transformation of `aw_editor` into a professional-grade game engine editor.

---

## 🛠 Phase 1: Critical Stability & Error Handling Hardening
**Goal**: Eliminate all crash risks and implement professional error reporting.

- [ ] **Task 1.1: Comprehensive Unwrap/Expect Audit**
  - Replace all production `unwrap()` and `expect()` with proper `Result` or fallbacks.
  - Fix Mutex poisoning in `telemetry.rs` (use safe lock recovery).
  - Enforce `clippy::unwrap_used` in workspace.
- [ ] **Task 1.2: GPU Resource & Device Loss Handling**
  - Implement explicit `Drop` for `ViewportRenderer`.
  - Add recovery path for GPU device lost events.
- [ ] **Task 1.3: User-Facing Error Reporting**
  - Implement `ToastManager` for non-blocking notifications.
  - Implement `GlobalErrorDialog` for critical failures.

## 🏗 Phase 2: Architectural Refactoring (Decoupling)
**Goal**: Resolve the God Object debt and move towards a service-oriented architecture.

- [ ] **Task 2.1: Extract Domain Services**
  - Create `SceneService`, `AssetService`, `SelectionService`.
  - Move logic from `EditorApp` into these services.
- [ ] **Task 2.2: Modularize UI Panels**
  - Decouple panels from direct `EditorApp` field access.
  - Implement a clean panel registry/manager.
- [ ] **Task 2.3: App Refactor**
  - Move `EditorApp` out of `main.rs` to `src/app.rs`.
  - Reduce `main.rs` to entry-point only.

## 🎨 Phase 3: Professional Rendering Integration
**Goal**: Full visual parity with the AstraWeave engine.

- [ ] **Task 3.1: Engine Render Adapter Implementation**
  - Fully wire `astraweave-render` PBR pipeline into the viewport.
  - Replace placeholder cubes with real mesh loading (`.glb`/`.fbx`).
- [ ] **Task 3.2: PBR & Lighting Enhancements**
  - Add directional light shadows to viewport.
  - Implement skybox/IBL texture support.
- [ ] **Task 3.3: Material Inspector Synchronization**
  - Ensure material edits immediately reflect in the 3D viewport.

## 📁 Phase 4: Asset Pipeline & Workflow Polish
**Goal**: Functional, efficient workflow for game development.

- [ ] **Task 4.1: Asset Browser & Importer Fixes**
  - Fix broken asset action queue.
  - Implement model and texture import pipelines (with compression).
- [ ] **Task 4.2: UX/Workflow Enhancements**
  - Add progress bars for long operations (Load/Save/Import).
  - Implement "Unsaved Changes" confirmation dialog.
  - Persist editor preferences (TOML).
- [ ] **Task 4.3: Keyboard Shortcut System**
  - Implement a robust, configurable shortcut manager.

## ✅ Phase 5: Final Validation & Certification
**Goal**: Verify production readiness.

- [ ] **Task 5.1: Performance & Stress Testing**
  - Validate 10k+ entities at 60 FPS.
  - Memory leak audit (long-running session).
- [ ] **Task 5.2: End-to-End Workflow Validation**
  - Build a mini-game level from scratch using only the editor.
  - One-click build and ship to a test package.
- [ ] **Task 5.3: Final Documentation & Cleanup**
  - Update all audit reports to "A+ / Production Ready" status.
