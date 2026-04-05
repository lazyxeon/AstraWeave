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
