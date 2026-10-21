# Bevy Shadow System Integration Plan

**Date**: November 5, 2025  
**Status**: PROPOSED  
**Estimated Time**: 2-3 hours (vs 8+ hours debugging from scratch)

---

## Problem Statement

After 8+ hours debugging our custom CSM implementation:
- ✅ Shadow atlas renders correctly (Mode 5 proof)
- ❌ Shadow sampling broken (coordinate transform issues)
- ❌ Stuck on low-level WGSL debugging

**USER DIRECTIVE**: "can you fork and clone bevy's shadow system... instead just solve integration"

This is the RIGHT engineering decision: **integrate proven tech** instead of reinventing.

---

## Why Bevy?

### Advantages
1. **Battle-tested**: Used in hundreds of production games
2. **Active maintenance**: Latest commit 18 hours ago (Nov 4, 2025)
3. **wgpu-based**: Same backend as AstraWeave (wgpu 25.0)
4. **MIT/Apache-2.0**: License compatible
5. **Comprehensive**: CSM + PCF + soft shadows + all the bells and whistles
6. **Well-documented**: Extensive shader comments and examples

### Why NOT rend3?
- **ARCHIVED** June 7, 2025 (read-only, maintenance mode)
- No future updates or bug fixes

---

## Integration Strategy

### Phase 1: Extract Bevy Shadow Components (30 min)

**Files to extract**:
```
bevy/crates/bevy_pbr/src/render/
├── shadows.wgsl           # Main shadow rendering shader
├── shadow_sampling.wgsl   # PCF sampling + soft shadows
├── light.rs               # Cascade calculation (Rust)
└── mesh_view_bindings.wgsl # Shadow atlas bindings
```

**What we'll copy**:
1. `shadows.wgsl` → `astraweave-render/shaders/bevy_shadows.wgsl`
2. `shadow_sampling.wgsl` → `astraweave-render/shaders/bevy_shadow_sampling.wgsl`
3. Cascade calculation code from `light.rs` → `shadow_csm.rs::update_cascades()`
4. Uniform structs from `mesh_view_types.wgsl` → Our shader uniforms

### Phase 2: Adapt to AstraWeave Interfaces (60 min)

**Keep our high-level API**:
```rust
// astraweave-render/src/shadow_csm.rs (PUBLIC API - NO CHANGES)
impl CsmRenderer {
    pub fn new(device: &wgpu::Device, ...) -> Self { ... }
    pub fn update_cascades(&mut self, camera_pos: Vec3, ...) { ... }
    pub fn render(&mut self, ...) { ... }
}
```

**Replace low-level implementation**:
```rust
// BEFORE (our broken custom impl):
fn world_to_shadow_uv(...) { /* coordinate transform bugs */ }

// AFTER (Bevy's proven impl):
#include "bevy_shadows.wgsl"  // Just use their working code!
```

**Shader integration**:
```wgsl
// examples/shadow_csm_demo/src/main.rs shader
// REPLACE our custom shadow sampling with:
#import bevy_pbr::shadow_sampling

fn sample_shadow(...) -> f32 {
    return bevy_pbr::sample_directional_cascade(...);
}
```

### Phase 3: Test with shadow_csm_demo (30 min)

**Test matrix**:
1. **Mode 0**: Normal rendering → Should show shadows!
2. **Mode 1-4**: Debug modes → Keep for validation
3. **Mode 5**: Atlas view → Should still work

**Success criteria**:
- ✅ Shadows appear under cubes
- ✅ Cascades transition smoothly
- ✅ No visual artifacts
- ✅ Performance ≥60 FPS

### Phase 4: Cleanup & Documentation (30 min)

1. Remove old broken shader code
2. Add LICENSE attribution to Bevy
3. Update RENDERER_MASTER_IMPLEMENTATION_PLAN.md
4. Create completion report

---

## File Structure After Integration

```
astraweave-render/
├── shaders/
│   ├── bevy_shadows.wgsl              # ← COPIED from Bevy
│   ├── bevy_shadow_sampling.wgsl      # ← COPIED from Bevy
│   ├── shadow_csm.wgsl                # ← DELETED (broken)
│   └── ...
├── src/
│   ├── shadow_csm.rs                  # ← UPDATED (use Bevy cascade calc)
│   └── ...
└── LICENSE_BEVY_SHADOWS.md            # ← NEW (attribution)

examples/shadow_csm_demo/
└── src/
    └── main.rs                        # ← UPDATED (import Bevy shaders)
```

---

## License Compliance

Bevy is dual-licensed **MIT OR Apache-2.0**:
- ✅ Compatible with AstraWeave's MIT license
- ✅ Can copy code directly with attribution
- ✅ No viral licensing issues

**Required attribution**:
```rust
// astraweave-render/shaders/bevy_shadows.wgsl
// Adapted from Bevy Engine (https://github.com/bevyengine/bevy)
// Copyright (c) 2020 Carter Anderson
// Licensed under MIT OR Apache-2.0
// Original: bevy/crates/bevy_pbr/src/render/shadows.wgsl
```

---

## Risks & Mitigations

| Risk | Mitigation |
|------|-----------|
| Bevy API differences | Extract only WGSL shaders (minimal Rust changes) |
| Uniform struct mismatches | Adapt Bevy structs to our layout (1:1 mapping) |
| Performance regression | Bevy is highly optimized (likely FASTER than ours) |
| Breaking our API | Keep public `CsmRenderer` API unchanged |

---

## Timeline

| Phase | Task | Time | Status |
|-------|------|------|--------|
| 1 | Extract Bevy files | 30 min | 🔲 Not started |
| 2 | Adapt interfaces | 60 min | 🔲 Not started |
| 3 | Test demo | 30 min | 🔲 Not started |
| 4 | Cleanup & docs | 30 min | 🔲 Not started |
| **TOTAL** | **Full integration** | **2.5 hours** | **vs 8+ hours debugging!** |

---

## Alternative Considered: three-d

**Pros**:
- Simple, focused on wgpu
- MIT licensed

**Cons**:
- Smaller community (fewer eyes on code)
- Less comprehensive shadow features
- Less active development than Bevy

**Decision**: **Bevy** is the clear winner (battle-tested + active + feature-rich).

---

## Next Steps

**IMMEDIATE**:
1. User approval of this plan
2. Extract Bevy shadow shaders
3. Integrate into AstraWeave
4. Test with demo
5. Ship working shadows in 2-3 hours!

**AFTER INTEGRATION**:
- Resume renderer roadmap (MegaLights validation, post-processing, etc.)
- Mark Phase 2 CSM as COMPLETE
- Move to Phase 3: Post-Processing Stack

---

## Success Metrics

**Before** (current state):
- ❌ 8+ hours debugging
- ❌ No working shadows
- ❌ Coordinate transform bugs
- ❌ Developer frustration

**After** (with Bevy integration):
- ✅ 2-3 hours total time
- ✅ Production-quality shadows
- ✅ Proven, battle-tested code
- ✅ Move forward on roadmap

---

**USER QUOTE**: "can you fork and clone bevy's shadow system... instead just solve integration"

**RESPONSE**: YES! This is the right engineering call. Let's do it.
