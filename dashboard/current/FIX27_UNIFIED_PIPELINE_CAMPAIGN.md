# Fix 27: Unified Rendering Pipeline Campaign Plan

**Date**: 2026-04-05
**Author**: Claude Opus 4.6 (AI-Orchestrated)
**Status**: PLAN — awaiting approval before execution
**Estimated Duration**: 7-9 weeks (32-41 working days)
**Prerequisite**: Fixes 1-26 complete (28 commits, 3894 tests green)

---

## Executive Summary

The AstraWeave editor viewport maintains two parallel rendering pipelines that have diverged across 12 dimensions (vertex format, materials, shadows, IBL, post-processing, etc.). This campaign eliminates the FastPreview path by routing ALL scene rendering through `astraweave_render::Renderer`, preserving editor-specific overlays (grid, gizmo, physics debug) via a formal overlay injection protocol.

**Outcome**: ~4,000 lines of duplicated rendering code deleted. Single source of truth for PBR, shadows, IBL, and post-processing. Visual parity between editor and runtime guaranteed by construction.

---

## Architecture: Current vs Target

### Current (Dual Pipeline)

```
ViewportRenderer
├── [FastPreview] ─── EntityRenderer (3609 LOC)
│   ├── Own PBR shader (entity.wgsl)
│   ├── Own 4-cascade CSM (shadow.wgsl)
│   ├── Own BRDF LUT (brdf_lut.wgsl)
│   ├── Own IBL (SH + cubemap)
│   ├── Own glTF loader + texture pipeline
│   ├── Own material uniforms
│   └── Own tonemap (tonemap.wgsl)
│
├── [EnginePBR] ─── EngineRenderAdapter (568 LOC)
│   ├── Wraps astraweave_render::Renderer
│   ├── Lossy terrain vertex conversion (96B → 36B)
│   └── Feature-gated (optional dependency)
│
├── GridRenderer (KEEP)
├── GizmoRenderer (KEEP)
├── PhysicsDebugRenderer (KEEP)
└── BlueprintOverlay (KEEP)
```

### Target (Unified Pipeline)

```
ViewportRenderer (simplified)
│
├── EngineRenderAdapter (EVOLVED — always active)
│   ├── astraweave_render::Renderer (single renderer)
│   ├── Entity data fed directly (no format conversion)
│   ├── Terrain uses engine vertex format natively
│   ├── Selection highlighting via engine hooks
│   └── Headless stub for CI
│
├── EditorOverlayHooks (NEW — formal injection protocol)
│   ├── render_scene_overlays() → HDR target, before post-FX
│   │   ├── GridRenderer
│   │   ├── PhysicsDebugRenderer
│   │   └── BlueprintOverlay
│   │
│   └── render_ldr_overlays() → LDR target, after post-FX
│       └── GizmoRenderer
│
└── DELETED:
    - EntityRenderer (3609 LOC)
    - entity.wgsl, shadow.wgsl, brdf_lut.wgsl, tonemap.wgsl, mipmap_blit.wgsl
    - MipmapGenerator
    - Editor material/texture loading
```

---

## Campaign Phases

### Phase 0: Foundation and Safety Net
**Duration**: 3-4 days (Week 1)

| Task | Description |
|------|-------------|
| Make astraweave-render non-optional | Remove `optional = true` from Cargo.toml |
| Remove all `#[cfg(feature)]` guards | Delete ~30 conditional compilation blocks across viewport code |
| Add `RenderBackend` enum | `{ Engine, LegacyPreview, Headless }` replaces `RenderMode` |
| Visual regression test harness | Screenshot comparison: render scene with both paths, compute RMSE |
| Delete stub adapter | Remove the empty `#[cfg(not(feature))]` impl |

**Key files**: `Cargo.toml`, `viewport/mod.rs`, `viewport/renderer.rs`, `viewport/engine_adapter.rs`
**Test gate**: All 3894 tests pass. Both render paths produce frames. RMSE baseline captured.
**Rollback**: Revert Cargo.toml to `optional = true`, restore `#[cfg]` guards.
**Risk**: Build time increase (~45s). Mitigate with incremental compilation.

---

### Phase 1: Entity Rendering Through Engine
**Duration**: 8-10 days (Weeks 2-3)
**CRITICAL PATH — highest risk, highest value**

| Sub-phase | Scope | Days |
|-----------|-------|------|
| 1a: Entity data feeding | Feed World entities → engine as named models | 3-4 |
| 1b: Selection highlighting | Outline/highlight selected entities via stencil or overlay | 2-3 |
| 1c: Shading modes | Wire Lit/Unlit/Wireframe to engine or overlay | 2 |
| 1d: Gate legacy path | `EntityRenderer` behind `#[cfg(feature = "legacy-preview")]` | 1 |

**Entity data flow (1a)**:
```
World entities → entity mesh map → engine_adapter feeds to Renderer:
  - For each entity with mesh: renderer.add_model(name, mesh, instances)
  - For entities without mesh: renderer.add_model(name, cube_mesh, instances)
  - glTF loading delegates to engine's mesh_gltf::load_gltf()
  - Skeleton/animation via engine's animation module
```

**Selection highlighting (1b)**:
- Option A (simple): Selected entity bounding boxes drawn via PhysicsDebugRenderer (colored wireframe)
- Option B (polished): Stencil-based outline pass in the engine (requires engine API addition)
- Recommend: Start with Option A, upgrade to B later

**Key files**: `engine_adapter.rs` (major expansion), `renderer.rs`, `entity_renderer.rs` (deprecate)
**Test gate**: Visual regression RMSE < 5% for entities. Selection works. Shading modes work.
**Rollback**: Toggle `RenderBackend::LegacyPreview` in UI toolbar.
**Risk**: HIGH. Largest behavioral change. Skeleton animation, per-entity textures, and mesh cache need careful migration.

---

### Phase 2: Shadow and IBL Unification
**Duration**: 3-4 days (Week 4)

| Task | Description |
|------|-------------|
| Delete editor shadow system | Remove shadow_texture, shadow_cascade_views, shadow_pipeline, etc. |
| Delete editor BRDF LUT | Remove brdf_lut_view, brdf_lut compute shader |
| Delete editor IBL uniforms | Remove IblParamsGpu, SH computation code |
| Forward lighting params to engine | set_sun/set_ambient/set_ibl route to engine's scene_environment |

**Deletions**: `shadow.wgsl`, `brdf_lut.wgsl`, ~800 LOC from entity_renderer.rs
**Key files**: `entity_renderer.rs`, `engine_adapter.rs`
**Test gate**: Shadows and IBL work through engine path. No editor-specific shadow textures allocated.
**Rollback**: Re-add shader files from git history.
**Risk**: LOW (cleanup after Phase 1).

---

### Phase 3: Tonemap and Post-Processing Unification
**Duration**: 4-5 days (Week 5)

| Task | Description |
|------|-------------|
| Fix double-tonemapping | Engine's draw_into() outputs HDR; editor tonemaps. Or: engine does full post. |
| Delete editor tonemap pass | Remove tonemap_pipeline, tonemap_bind_group, tonemap_params_buffer |
| Forward tonemap mode to engine | PBR Neutral / ACES / Reinhard selection via engine's HDR pipeline config |
| Route engine post-FX | GTAO, bloom, god rays, auto-exposure now available in editor |

**Deletions**: `tonemap.wgsl`, ~200 LOC from renderer.rs
**Key files**: `renderer.rs`, `engine_adapter.rs`, `astraweave-render/src/renderer.rs`
**Test gate**: Post-processing effects visible. Tonemap mode selector works.
**Rollback**: Restore editor tonemap pass.
**Risk**: MEDIUM. Overlay rendering order must be correct (see Phase 4).

---

### Phase 4: Overlay Injection Protocol
**Duration**: 4-5 days (Weeks 5-6)

```rust
/// Formal contract for editor overlays compositing with the engine.
pub trait EditorOverlayHooks {
    /// Called after scene rendering, before post-processing.
    /// Overlays render into the HDR target with the scene depth buffer.
    fn render_scene_overlays(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        hdr_view: &wgpu::TextureView,
        depth_view: &wgpu::TextureView,
    ) -> Result<()>;

    /// Called after post-processing, on the final LDR target.
    /// For crisp UI overlays (gizmos) that should not be bloom/tonemapped.
    fn render_ldr_overlays(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        ldr_view: &wgpu::TextureView,
        depth_view: &wgpu::TextureView,
    ) -> Result<()>;
}
```

| Task | Description |
|------|-------------|
| Define trait in astraweave-render | `EditorOverlayHooks` trait with HDR and LDR injection points |
| Add to draw_into_with_hooks() | Engine calls hooks at correct points in render pipeline |
| Implement in editor | ViewportRenderer implements trait: grid/physics → HDR, gizmos → LDR |
| Expose engine internals | HDR texture view and depth view must be accessible to hooks |

**Key files**: `astraweave-render/src/renderer.rs` (API addition), `viewport/renderer.rs` (implement)
**Test gate**: Grid renders correctly with depth. Gizmos are crisp (no bloom/tonemap applied).
**Rollback**: Overlays render to separate texture + alpha composite (wasteful but independent).
**Risk**: MEDIUM. Requires exposing engine internal textures. API design must be stable.

---

### Phase 5: Terrain Vertex Unification
**Duration**: 5-6 days (Weeks 6-7)
**Can run in parallel with Phases 2-4**

| Task | Description |
|------|-------------|
| Audit terrain vertex consumers | Find every file that uses `viewport::types::TerrainVertex` |
| Define shared vertex format | Either engine adopts biome weights or editor uses engine's simpler format |
| Update terrain_integration.rs | Generate terrain meshes in engine-compatible format directly |
| Remove adapter conversion | Delete the lossy 96B→36B conversion in engine_adapter.rs |

**Decision required**: Does the engine's terrain renderer need biome weights? If yes, expand engine's `TerrainVertex`. If no, the editor generates engine-compatible terrain and handles biome visuals via material IDs.

**Key files**: `viewport/types.rs`, `terrain_integration.rs`, `engine_adapter.rs`, `astraweave-render/src/terrain.rs`
**Test gate**: Terrain renders correctly without data loss. Biome transitions visible.
**Rollback**: Keep adapter conversion (it works, just lossy).
**Risk**: MEDIUM-HIGH. Wide ripple across panel code that references TerrainVertex.

---

### Phase 6: Legacy Code Deletion
**Duration**: 3-4 days (Weeks 7-8)

**DELETE**:
| File | LOC | Purpose (now dead) |
|------|-----|--------------------|
| `entity_renderer.rs` | 3609 | FastPreview entity rendering |
| `mipmap_generator.rs` | ~300 | CPU/GPU mipmap generation |
| `entity.wgsl` | ~650 | Editor PBR shader |
| `mipmap_blit.wgsl` | ~30 | GPU mipmap blit shader |
| **Total** | **~4589** | |

(shadow.wgsl, brdf_lut.wgsl, tonemap.wgsl already deleted in Phases 2-3)

**Also remove**:
- `RenderBackend::LegacyPreview` enum variant
- All `entity_renderer` field and delegation methods in `ViewportRenderer`
- The `entity_renderer` module declaration in `viewport/mod.rs`
- Render mode selector UI in panels

**Test gate**: All 3894+ tests pass. `cargo clippy -p aw_editor -- -D warnings` clean.
**Risk**: LOW. Pure deletion — compiler enforces no remaining references.

---

### Phase 7: Headless/CI Fallback
**Duration**: 2-3 days (Week 8)

| Task | Description |
|------|-------------|
| Headless renderer | Use engine's `Renderer::new_headless()` with wgpu software backend |
| Null renderer | Skip all GPU work, return blank texture (for unit tests) |
| CI configuration | Ensure GitHub Actions workflows use Headless for render tests, Null for unit tests |
| Test matrix | Verify all 3894 tests pass in Null mode |

**Key files**: `viewport/renderer.rs`, `viewport/engine_adapter.rs`
**Risk**: LOW. Engine's headless already exists.

---

## Dependency Graph

```
Phase 0 (Foundation) ─────────────────────────► GATE
    │
    ▼
Phase 1 (Entity Unification) ◄── CRITICAL PATH  ► GATE
    │
    ├───────────────────────┐
    ▼                       ▼
Phase 2 (Shadow/IBL)    Phase 5 (Terrain)
    │                       │     [PARALLEL]
    ▼                       │
Phase 3 (Tonemap/Post)     │
    │                       │
    ▼                       │
Phase 4 (Overlay Hooks)    │
    │                       │
    ├───────────────────────┘
    ▼
Phase 6 (Deletion) ─────────────────────────────► GATE
    │
    ▼
Phase 7 (Headless/CI)
```

---

## Timeline

| Week | Phase | Milestone |
|------|-------|-----------|
| 1 | Phase 0 | Safety net in place, visual regression tests passing |
| 2-3 | Phase 1 | Entity rendering through engine, selection working |
| 4 | Phase 2 | Shadow/IBL unified, editor shaders deleted |
| 5 | Phases 3+4 | Post-processing unified, overlay hooks implemented |
| 6-7 | Phases 5+6 | Terrain unified, legacy code deleted |
| 8 | Phase 7 | Headless/CI, final validation |

---

## Risk Register

| Risk | Severity | Phase | Mitigation |
|------|----------|-------|------------|
| Entity rendering regression (visual) | CRITICAL | 1 | Visual regression test harness, LegacyPreview fallback |
| Build time increase (45s) | HIGH | 0 | Incremental compilation, feature-gate heavy subsystems |
| Overlay rendering order (bloom on gizmos) | HIGH | 3-4 | EditorOverlayHooks trait with HDR/LDR injection points |
| Engine API insufficient for editor needs | HIGH | 1,4 | Add minimal API extensions to astraweave-render |
| Skeleton/animation migration complexity | MEDIUM | 1 | Start with CPU skinning fallback, upgrade later |
| Terrain data loss (biome weights) | MEDIUM | 5 | Keep adapter conversion as fallback |
| Test suite breakage | MEDIUM | All | `RenderBackend::Null` for unit tests, `Headless` for render tests |
| Merge conflicts (other teams) | LOW | All | Feature branches, daily rebases |

---

## Success Criteria

1. **Zero visual regression**: RMSE < 2% for all reference scenes
2. **All tests green**: 3894+ tests pass (Null mode for unit, Headless for render)
3. **Code reduction**: ~4,000+ LOC deleted
4. **Single renderer**: No duplicate PBR, shadow, IBL, or post-processing code
5. **Build parity**: Editor builds in < 2 minutes incremental
6. **Overlay compositing**: Grid has correct depth, gizmos are crisp (no bloom/tonemap)
7. **Runtime switching**: No more render mode selector needed

---

## Commit Strategy

- Feature branch: `fix-27/unified-pipeline`
- One commit per sub-task within each phase
- Phase gate PRs: one PR per phase, squash-merged to main after visual regression passes
- Each phase tagged: `fix27-phase-N-complete`

---

**Version**: 1.0.0
**Classification**: Campaign Plan (requires approval before execution)
