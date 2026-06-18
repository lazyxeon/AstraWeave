# Technical Specification: AstraWeave Editor World-Class Transformation

## 1. Technical Context
- **Language**: Rust 1.89.0
- **UI Framework**: egui 0.32
- **Graphics API**: wgpu 25.0.2
- **Engine Core**: `astraweave-core`, `astraweave-render`, `astraweave-physics`, `astraweave-ai`
- **Current Status**: Functional prototype (82% mature), but with critical rendering gaps (cubes only) and architectural debt (God Object).

## 2. Implementation Approach

### Phase 1: Professional Rendering & Engine Integration
- **Objective**: Achieve visual parity between the editor viewport and the actual game engine.
- **Tasks**:
    - Fully integrate `astraweave-render`'s PBR pipeline into `ViewportRenderer`.
    - Implement `EngineRenderAdapter` to load and render real glTF/FBX meshes instead of cubes.
    - Add directional light, skybox textures (IBL), and basic shadow mapping to the viewport.
    - Synchronize the `MaterialInspector` with the viewport for real-time material editing.

### Phase 2: Architectural Refactoring (Decoupling the God Object)
- **Objective**: Break down `EditorApp` (2,700+ lines) into manageable domain services.
- **Tasks**:
    - Extract `SceneService`: Manages world state, loading, saving, and autosaves.
    - Extract `AssetService`: Handles asset database, imports, and thumbnail caching (with LRU eviction).
    - Extract `SelectionService`: Manages entity selection state and gizmo orchestration.
    - Extract `ProjectService`: Manages project-wide settings, recent files, and build targets.
    - Move `EditorApp` to `src/app.rs` and use it as a thin composition layer.

### Phase 3: Production Stability & Error Handling
- **Objective**: Eliminate all crash risks and implement professional error recovery.
- **Tasks**:
    - Replace all production `unwrap()` and `expect()` calls with proper `Result` propagation or safe fallbacks.
    - Fix Mutex poisoning in `telemetry.rs` and other modules.
    - Implement GPU device loss handling and resource recovery.
    - Add a `ToastManager` and `GlobalErrorDialog` for user-facing error reporting.

### Phase 4: Asset Pipeline & Workflow Automation
- **Objective**: Functional and efficient asset management.
- **Tasks**:
    - Fix the broken `AssetBrowser` action queue; implement model and texture importers.
    - Add background loading for assets with UI progress bars.
    - Implement a "Save Confirmation" dialog when quitting with unsaved changes.
    - Persist editor preferences (panel layout, camera position, snapping) to a TOML file.

## 3. Source Code Structure Changes
```
tools/aw_editor/src/
├── services/           # NEW: Decoupled domain logic
│   ├── scene.rs
│   ├── asset.rs
│   ├── selection.rs
│   ├── project.rs
│   └── mod.rs
├── panels/             # Refined: Only UI logic
├── viewport/           # Engine-integrated rendering
├── app.rs              # NEW: Refactored EditorApp
├── main.rs             # Entry point (minimal)
└── ...
```

## 4. Data Model & API Changes
- **EditorConfig**: New TOML-based persistence model for editor preferences.
- **AssetDatabase**: Enhanced to support metadata, compression settings, and async loading status.
- **Command Pattern**: Extend to support more complex operations like terrain editing and multi-entity property changes.

## 5. Verification Approach
- **Automated Testing**:
    - `cargo test --workspace`: Ensure zero regressions.
    - New unit tests for each decoupled Service.
    - Integration tests for mesh loading and scene synchronization.
- **Linting**:
    - `cargo clippy --workspace --all-features -- -D warnings`: Enforce zero-warning policy and `clippy::unwrap_used`.
- **Manual Verification**:
    - "Veilweaver Playability Test": Verify end-to-end workflow from asset import to Play-in-Editor.
    - Long-run stability test: 4 hours of active editing without memory leaks or crashes.
