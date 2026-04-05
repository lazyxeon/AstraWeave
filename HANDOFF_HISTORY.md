# Session Handoff History

Chronological log of all session handoffs for AstraWeave.

---
## 2026-04-05 (Extended session checkpoint)
**Mandate**: Behavioral correctness audit + remediation campaign for `tools/aw_editor`

**Accomplished**:
- Full 8-phase behavioral correctness audit (14 CRITICAL, 18 HIGH, 28 MEDIUM, 10 LOW findings)
- 26 tracked fixes across 30 commits; 3,894 lib tests green throughout
- Tier 1 (5/5): GGX NDF epsilon, Fresnel energy conservation, mutex poison recovery, error surfacing (material preview, asset scan)
- Tier 2 (8/8): R8G8 normal Z reconstruction, BRDF LUT height-correlated Smith-GGX, Turquin 2019 multi-scatter, scale gizmo sign fix, simulation panic recovery, undo desync (6 duplicate updates removed), entity creation undo partial wiring
- Deferred→resolved (3/3): Per-axis Pose.scale added to astraweave_core, terrain adapter documented, instance layout verified correct
- Tier 3 (5 fixed): Mesh retry cache, exposure HDR path, IBL roughness Fresnel, autosave/prefab error surfacing
- Tier 4 SOTA (6/6): Khronos PBR Neutral tonemapper, 3-channel DFG LUT cloth sheen, glTF MikkTSpace TBN tangents, 4-cascade CSM shadows with frustum fitting, IBL prefiltered cubemap + IblManager bake
- Fix 27 campaign plan written (7-phase, 7-9 week architectural refactor)

**Left in progress**:
- Fix 27 Phase 0 not started — plan committed at `docs/current/FIX27_UNIFIED_PIPELINE_CAMPAIGN.md`, analysis complete, zero execution begun
- C-5 partial: 5 of 9 entity creation ops still bypass undo stack

**Failed approaches**:
- Fix 8 multi-scatter: First pass multiplied entire `color` (diffuse + specular combined) — corrected to specular-only within same session
- Fix 10 surface lost recovery: Audit finding was invalid; editor viewport uses offscreen textures, never wgpu surfaces directly — reclassified N/A

**Next session should**:
1. Start Fix 27 Phase 0: remove `optional = true` from `tools/aw_editor/Cargo.toml`, delete ~70 cfg guards across 7 files, add `RenderBackend` enum
2. Create feature branch `fix-27/unified-pipeline` per commit strategy in plan
3. Fix 27 Phase 1 (CRITICAL PATH): entity rendering through `astraweave_render::Renderer`
4. Wire remaining 5 C-5 entity creation ops to undo stack (1-2 days, can be parallelized with Fix 27)

---
