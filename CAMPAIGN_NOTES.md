# Audit Remediation Campaign Notes

Running log of surprises, adjacent issues, and decisions.

---

## 2026-04-04 — Campaign Start

Campaign initiated from EDITOR_BEHAVIORAL_CORRECTNESS_AUDIT.md findings.
27 fixes across 4 tiers. Executing in strict tier order.

## 2026-04-04 — Tier 1 Complete

All 5 Tier 1 fixes committed across 3 commits:
1. **VC-1 + VC-2** (entity.wgsl): GGX NDF epsilon fixed to `max(PI * denom^2, 1e-7)`, diffuse now reduced by Fresnel `(1-F)*(1-metallic)`. Specular is now computed before diffuse so F is available for kD.
2. **C-3** (widget.rs): 17 `if let Ok` mutex lock patterns replaced with `with_renderer()` helper that recovers from poison via `into_inner()`. Also refactored the large entity-state block to pre-compute data outside the lock.
3. **C-1 + C-2** (main.rs): Two `let _ =` patterns replaced with `if let Err(e) = ...` + `tracing::error!`.

**Surprises**: Found 17 mutex lock instances, not 16 as the audit reported. The entity-state-update block (lines 546-621) required restructuring because `self` was simultaneously borrowed by `with_renderer` and the closure body — resolved by pre-computing all data before acquiring the lock.

**Pre-existing issues noted (not fixed — out of scope)**:
- Integration tests (prefab_workflow, delete_command_tests, etc.) have pre-existing compilation errors from `cmd.undo()` signature changes that weren't updated in test files. These are NOT caused by Tier 1 changes.
- astraweave-render has 13 pre-existing warnings (dead_code fields).

**Gate results**: 3,893 lib tests pass, 0 failures. `cargo check -p aw_editor` clean.

## 2026-04-05 — Tier 2 In Progress

### Fix 10 (Surface Lost Recovery): RECLASSIFIED AS N/A

The audit reported "no SurfaceError::Lost handling" in viewport/renderer.rs. Investigation reveals the editor viewport **never uses wgpu surfaces directly** — it renders to offscreen textures (Rgba16Float HDR target + Depth32Float), then copies pixels to an egui texture handle via CPU readback. The wgpu window surface is managed entirely by eframe/egui_wgpu, which has its own surface recovery logic. No code change needed.

Confirmed via grep: zero matches for `get_current_texture`, `SurfaceError`, `surface.configure`, or `SurfaceTexture` in the entire `tools/aw_editor/src/viewport/` directory.

### Fixes 6-9, 11 Complete

- **Fix 6 (R8G2)**: Reconstructed Z from XY in R8G8 normal map expansion. Used `sqrt(1 - x^2 - y^2)` on the CPU during texture load. Only applied when `srgb=false`.
- **Fix 7 (VC-3)**: Replaced separable Schlick-GGX in BRDF LUT with height-correlated Smith-GGX V-term matching entity.wgsl. Updated g_vis integrand formula for visibility form.
- **Fix 8 (VC-4)**: Added Turquin 2019 multi-scatter energy compensation. Threading `energy_comp` parameter through `disney_brdf_directional` and `disney_brdf_point`. Applied to both analytical and IBL specular paths. Initial attempt incorrectly multiplied entire `color` (including diffuse) — caught and corrected.
- **Fix 9 (M-21)**: Changed `mouse_delta.length()` to `mouse_delta.x` for signed direction. Added test for downscaling.
- **Fix 11**: Wrapped `app.run_fixed()` in `catch_unwind`. Uses Option<App> shuttle pattern to handle move semantics. On panic: logs error, auto-pauses, returns Err.

### Remaining Tier 2 Work

Fixes 12 and 13 are tightly coupled (both modify the undo system in command.rs + main.rs event handlers):
- **Fix 12**: EntityManager/World undo desync — the event handlers have explicit comments acknowledging the issue ("NOTE: EntityManager not part of command; undo won't revert it"). Fix requires making every EditorCommand's execute/undo update BOTH World and EntityManager.
- **Fix 13 (C-5)**: 9 operations bypass undo entirely. The commands exist (SpawnEntitiesCommand, DuplicateEntitiesCommand) but are not wired to the event handlers.

These should be done together in a focused session since they touch the same code paths. Estimated: 4-6 hours combined.
