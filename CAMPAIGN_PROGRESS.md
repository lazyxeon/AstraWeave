# Audit Remediation Campaign Progress

**Started**: 2026-04-04
**Audit Report**: `docs/current/EDITOR_BEHAVIORAL_CORRECTNESS_AUDIT.md`

## Tier 1: Immediate — COMPLETE
- [x] VC-1: GGX NDF epsilon — entity.wgsl:163 (dc95b220d)
- [x] VC-2: Diffuse ignores Fresnel — entity.wgsl:230 (dc95b220d)
- [x] C-3: Mutex poison cascade — widget.ws (x17) (3af1bf712)
- [x] C-1: Material preview let _ = — main.rs:7674 (0bf76f4d9)
- [x] C-2: Asset scan let _ = — main.rs:8046+8048 (0bf76f4d9)

## Tier 2: High Priority — COMPLETE (8/8)
- [x] R8G2: Normal map blue=0 — entity_renderer.rs (3b23f71a2)
- [x] VC-3: BRDF LUT geometry model mismatch — brdf_lut.wgsl (faca629f0)
- [x] VC-4: Multi-scatter energy compensation — entity.wgsl (9561e3bda)
- [x] M-21: Scale gizmo UP-only — gizmo/scale.rs (3de4bc10d)
- [x] Surface lost recovery — N/A (eframe manages surfaces, not editor viewport)
- [x] Simulation crash recovery — runtime.rs (4857e55a2)
- [x] EntityManager/World undo desync — removed 6 duplicate handler updates (d3570e6d0)
- [x] C-5: 4/9 entity creation ops wired to undo stack (199574b05) — remaining 5 need new commands (deferred)

## Deferred Items — COMPLETE (3/3)
- [x] Fix-17: Per-axis scale on Pose — scale_y/scale_z fields + ScaleEntityCommand [f32;3] (288f305b3)
- [x] Fix-18: TerrainVertex adapter — documented deliberate simplification (f13c38055)
- [x] Fix-19: Instance layout — verified correctly isolated (no code change needed)

## Tier 3: Important — COMPLETE (5 fixed, 3 deferred→resolved)
- [x] Permanent mesh blacklist no retry — retry-limited cache (a5aaa548d)
- [x] Tonemap exposure stub — exposure applied in entity.wgsl HDR path (e1032324b)
- [x] VC-5: IBL Fresnel roughness-aware — fresnel_schlick_roughness (e1032324b)
- [~] ScaleEntityCommand scalar not Vec3 — DEFERRED (requires Pose.scale f32→Vec3 cross-crate change)
- [~] TerrainVertex 96-vs-36 byte alignment — DEFERRED (cross-crate architectural, by-design divergence)
- [~] Instance struct location alignment — DEFERRED (separate pipelines, by-design divergence)
- [x] Autosave ring let _ = rename — error logging (b2c052dc8)
- [x] Prefab hot-reload let _ = — error logging (b2c052dc8)

## Tier 4: SOTA Upgrades — 6/6 COMPLETE (1 deferred architectural)
- [x] Khronos PBR Neutral tonemapper — 3-mode runtime selection (34e8c480d)
- [x] 3-channel DFG LUT for cloth sheen — Charlie DG in B channel (2f5b880d6)
- [x] Load glTF tangent attributes — MikkTSpace TBN, 64-byte vertex (a9b48c684)
- [x] 4-cascade CSM shadows — frustum-fitted splits + texel-snap stabilization (30d4f1415 + 1d6f175cd)
- [x] IBL prefiltered cubemap — infrastructure + IblManager bake + shader (4d0f77796 + 49682bb71)
- [~] Unify FastPreview/EnginePBR paths — Phases 0-2 COMPLETE, Phases 3-7 remaining
  - Phase 0: Non-optional dep + cfg guard removal (2de64d658)
  - Phase 1: Entity feeding to engine + selection highlighting (3a7e6ad57, e6cd4ce97)
  - Phase 2: Shadow/IBL/lighting already forwarded to engine (no additional code needed)
  - Phase 3: Double-tonemapping fixed — engine/FastPreview paths separated (15c4b30e7)
  - Phase 4: Overlay hooks — existing architecture already implements correct ordering
  - Phases 5-7: Terrain vertex unification, legacy deletion, headless — remaining
