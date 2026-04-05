# Audit Remediation Campaign Progress

**Started**: 2026-04-04
**Audit Report**: `docs/current/EDITOR_BEHAVIORAL_CORRECTNESS_AUDIT.md`

## Tier 1: Immediate — COMPLETE
- [x] VC-1: GGX NDF epsilon — entity.wgsl:163 (dc95b220d)
- [x] VC-2: Diffuse ignores Fresnel — entity.wgsl:230 (dc95b220d)
- [x] C-3: Mutex poison cascade — widget.ws (x17) (3af1bf712)
- [x] C-1: Material preview let _ = — main.rs:7674 (0bf76f4d9)
- [x] C-2: Asset scan let _ = — main.rs:8046+8048 (0bf76f4d9)

## Tier 2: High Priority — IN PROGRESS (6/8 complete)
- [x] R8G2: Normal map blue=0 — entity_renderer.rs (3b23f71a2)
- [x] VC-3: BRDF LUT geometry model mismatch — brdf_lut.wgsl (faca629f0)
- [x] VC-4: Multi-scatter energy compensation — entity.wgsl (9561e3bda)
- [x] M-21: Scale gizmo UP-only — gizmo/scale.rs (3de4bc10d)
- [x] Surface lost recovery — N/A (eframe manages surfaces, not editor viewport)
- [x] Simulation crash recovery — runtime.rs (4857e55a2)
- [ ] EntityManager/World undo desync — multiple files (NEXT)
- [ ] C-5: 9 operations bypass undo stack — command.rs + main.rs

## Tier 3: Important
- [ ] Permanent mesh blacklist no retry — entity_renderer.rs:174
- [ ] Tonemap exposure stub — entity.wgsl / renderer
- [ ] VC-5: IBL Fresnel roughness-aware — entity.wgsl:320
- [ ] ScaleEntityCommand scalar not Vec3 — command.rs
- [ ] TerrainVertex 96-vs-36 byte alignment — terrain types
- [ ] Instance struct location alignment — vertex layout
- [ ] Autosave ring let _ = rename — autosave code
- [ ] Prefab hot-reload let _ = — main.rs:8798

## Tier 4: Deferred (SOTA Upgrades)
- [ ] Khronos PBR Neutral tonemapper
- [ ] 3-channel DFG LUT for cloth sheen
- [ ] Load glTF tangent attributes
- [ ] Align shadow cascades 1 -> 4
- [ ] IBL prefiltered cubemap
- [ ] Unify FastPreview/EnginePBR paths
