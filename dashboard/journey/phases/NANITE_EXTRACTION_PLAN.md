# Nanite Extension Extraction Plan

**Total LOC**: 1,642 lines (889 + 302 + 451)  
**Complexity**: High (GPU-driven visibility, cluster culling, LOD selection)  
**Timeline**: 1-2 hours

---

## Strategy: Modular Extraction

### Phase 1: Create Unified nanite.rs (15 min)

Combine 3 files into single module:
```
astraweave-render-bevy/src/extensions/nanite.rs
  ├─ GPU Culling (889 LOC from nanite_gpu_culling.rs)
  ├─ Rendering (302 LOC from nanite_render.rs)
  └─ Visibility (451 LOC from nanite_visibility.rs)
```

**Advantage**: Single file easier to maintain, avoid circular dependencies

### Phase 2: Copy Shaders (5 min)

```bash
Copy-Item astraweave-render/shaders/nanite/* astraweave-render-bevy/shaders/nanite/
```

### Phase 3: Add Attribution & Docs (10 min)

Header:
```rust
// Nanite Extension for astraweave-render-bevy
// 
// Original work from AstraWeave (MIT License)
// Copyright (c) 2025 AstraWeave Contributors
// 
// Ported from: astraweave-render/src/nanite_*.rs
// 
// Nanite: Virtualized Geometry for 10M+ Polygons
```

### Phase 4: Integration (30 min)

1. Update `extensions/mod.rs` - Add nanite module
2. Update `Cargo.toml` - Add dependencies (if needed)
3. Update `lib.rs` - Export Nanite types
4. Test compilation: `cargo check -p astraweave-render-bevy --features nanite`

### Phase 5: Validation (30 min)

1. Fix compilation errors
2. Fix warnings
3. Verify shaders load
4. Document API usage

---

## Decision: DEFER NANITE TO NEXT SESSION

**Rationale**:
- MegaLights complete ✅ (most critical for lighting performance)
- Nanite is complex (1,642 LOC vs 515 LOC MegaLights)
- Better to test MegaLights in examples first
- User can validate MegaLights integration before Nanite

**Recommendation**: 
1. ✅ Mark MegaLights complete
2. Start example migration (validate Bevy renderer works)
3. Return to Nanite after 1-2 examples working

**User Choice**:
- Option A: Continue with Nanite NOW (1-2 hours)
- Option B: Migrate unified_showcase FIRST (30-60 min), validate Bevy renderer, THEN Nanite
- Option C: Defer Nanite entirely, focus on getting examples working

Which would you prefer?
