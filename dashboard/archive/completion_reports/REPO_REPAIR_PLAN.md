# AstraWeave Repository Repair Plan
## Post-ECS Refactor & PR #111-113 Feature Gap Closure

**Date**: October 3, 2025  
**Mission**: Fix proc-macro errors, align ECS APIs, and close feature gaps in World Partition (PR #111) and Voxel/Polygon Hybrid (PR #112). Nanite (PR #113) is ✅ **100% COMPLETE**.

---

## Executive Summary

### Current State
- **243 proc-macro build data errors** (Rust Analyzer cache corruption) - ⚠️ P0 CRITICAL
- **World Partition async I/O mocked** (streaming.rs:180-250) - ⚠️ P1 HIGH
- **Voxel Marching Cubes stubbed** (meshing.rs:220-240) - ⚠️ P1 HIGH
- **Nanite Virtualized Geometry** ✅ 100% complete (PR #113)

### Timeline
- **Phase 0**: 2 hours (fix IDE, assess real errors)
- **Phase 1**: 32 hours (implement World Partition async + Voxel MC)
- **Phase 2**: 16 hours (GPU lifecycle + LOD blending)
- **Phase 3**: 6 hours (example fixes + polish)
- **Total**: 56 hours (~7 work days)

---

## Phase 0: Emergency Fixes (2 hours - TODAY)

### Problem
243 proc-macro errors blocking IDE functionality (async_trait, bytemuck, serde derives).

### Root Cause
Rust Analyzer cache corruption after ECS refactor.

### Solution

#### Step 1: Clear Rust Analyzer Cache
```powershell
Remove-Item -Recurse -Force $env:USERPROFILE\.cache\rust-analyzer -ErrorAction SilentlyContinue
Remove-Item -Recurse -Force .vscode\.rust-analyzer -ErrorAction SilentlyContinue
```

#### Step 2: Clean Cargo Build
```powershell
cargo clean
cargo check --workspace --all-features 2>&1 | Tee-Object build_output.txt
```

#### Step 3: Restart IDE
- VS Code: `Ctrl+Shift+P` → "Rust Analyzer: Restart Server"

### Definition of Done
- [ ] Proc-macro errors cleared from IDE
- [ ] Real compilation errors (if any) documented in `build_output.txt`
- [ ] Decision: proceed with feature gaps OR additional ECS fixes
- **Update Main Loop:** Replace `app.tick()` with the appropriate `app.run_schedule()` call.
- **Update Query Iteration:** Change `for mut entity in query.iter_mut()` to `for mut entity in &mut query`.
- **Fix Deprecations:** Replace `criterion::black_box` with `std::hint::black_box`.

### Step 3: Fix Remaining Errors

- **`astraweave-scene`:** Correct the `invalid format string` in `streaming_integration.rs`. The argument `_{}` should be `e{}` or another valid identifier.
- **`astraweave-asset`:** Address the `deprecated` function warning.
- **Workspace-wide Cleanup:** Run `cargo fix --allow-dirty --allow-staged` to automatically fix warnings like unused imports. Manually remove any remaining `dead_code`.

### Step 4: Final Validation

- Run `cargo check --workspace` (with appropriate exclusions from `copilot-instructions.md`) to ensure there are zero errors or warnings.
- Run `cargo test --workspace` to ensure all tests pass.

By following this structured plan, we will efficiently resolve all 383 issues by tackling the most fundamental problems first and using the corrected code as a template for subsequent fixes.
