# Phase 2 CSM Next Steps - Decision Guide

**Date**: November 4, 2025  
**Current Status**: Visual demo running successfully  
**Time Invested**: 3 hours (Phase 2) + 4 hours (Phase 1) = 7 hours total  
**Budget Remaining**: 5-7 hours (Phase 2), ~60 hours (remaining phases)

---

## Current Achievement

✅ **Option A Complete**: shadow_csm_demo created and running!

**What Works**:
- 912-line standalone CSM demo
- 4-cascade shadow atlas (4096×4096)
- PCF filtering (5×5 kernel)
- Camera controls (WASD + mouse)
- Scene rendering (ground + 7 cubes)
- Zero compilation/runtime errors

**Critical Fixes Applied**:
- Solved bind group architecture issue (separate shadow shader + bind group)
- Fixed 11 wgpu 25 API compatibility issues
- Integrated with workspace dependencies

**Files Created**:
1. `examples/shadow_csm_demo/src/main.rs` (912 lines)
2. `astraweave-render/src/shadow_csm.rs` (+70 lines for bind groups)
3. `docs/journey/daily/PHASE_2_CSM_OPTION_A_VISUAL_DEMO_COMPLETE.md` (completion report)

---

## Next Step Options

### OPTION A: Manual Visual Validation (15 minutes)

**Test the demo to confirm AAA quality:**

1. **Run Demo**:
   ```powershell
   cargo run -p shadow_csm_demo --release
   ```

2. **Visual Checklist**:
   - ✅ Do shadows appear on ground plane?
   - ✅ Are shadow edges soft (not aliased/hard)?
   - ✅ Test camera movement (WASD + mouse drag)
   - ✅ Test zoom (does shadow quality degrade gracefully at distance?)
   - ⚠️ Check for shadow acne (shimmering/flickering on surfaces)
   - ⚠️ Check for peter-panning (shadows detached from objects)
   - ⚠️ Look for cascade seams (visible lines between shadow quality regions)

3. **Performance Check**:
   - Note FPS in window (should be >200 FPS on modern GPU)
   - Shadows should cost <2ms (9-12% of 60 FPS budget)

4. **Report Findings**:
   - Tell me what you observe
   - Identify any visual artifacts
   - Confirm if quality meets your standards

**Time**: 15 minutes  
**Next**: Based on findings, choose polish vs proceed

---

### OPTION B: Phase 2 Polish (1-2 hours)

**Optimize and perfect CSM before integration:**

#### B1: Cascade Optimization (30 minutes)
- **Current**: Simplified orthographic projection covers entire scene
- **Target**: Tight-fit frustum bounds (higher shadow resolution)
- **Benefit**: 2-4× effective resolution increase (sharper shadows)
- **Implementation**: Calculate frustum corners in view space, fit bounds to actual geometry

#### B2: Cascade Blending (20 minutes)
- **Current**: Hard transitions between cascades
- **Target**: Smooth crossfade in overlap regions
- **Benefit**: Eliminate visible cascade seams
- **Implementation**: Sample two adjacent cascades, blend based on distance

#### B3: Performance Profiling (15 minutes)
- **Current**: Estimated timings (~1.2ms shadow, ~0.5ms sampling)
- **Target**: Real GPU timings with Tracy or wgpu timestamps
- **Benefit**: Identify optimization opportunities
- **Implementation**: Add `#[profiling::function]` macros, run profiling_demo

#### B4: Documentation (25 minutes)
- **Current**: Code comments + integration example
- **Target**: Full architecture guide (docs/CSM_IMPLEMENTATION.md)
- **Benefit**: Help future contributors understand CSM design
- **Implementation**: Document atlas layout, cascade math, PCF algorithm, integration patterns

**Total Time**: 1.5 hours  
**Deliverables**: Optimized CSM ready for production use  
**Next**: Renderer integration (Option C) or Phase 3

---

### OPTION C: Renderer Integration (45 minutes)

**Merge CSM into main Renderer struct:**

#### C1: Renderer Structure (15 minutes)
```rust
// astraweave-render/src/lib.rs
pub struct Renderer {
    // ... existing fields ...
    csm: Option<CsmRenderer>,  // Shadow system
}

impl Renderer {
    pub fn enable_csm(&mut self, device: &wgpu::Device) {
        self.csm = Some(CsmRenderer::new(device).unwrap());
    }
}
```

#### C2: Frame Loop Integration (20 minutes)
```rust
pub fn render(&mut self, ...) {
    // 1. Update shadow cascades
    if let Some(csm) = &mut self.csm {
        csm.update_cascades(cam_pos, view, proj, light_dir, near, far);
        csm.upload_to_gpu(&self.queue, &self.device);
    }
    
    // 2. Render shadow maps (before main pass)
    let mut encoder = device.create_command_encoder(...);
    if let Some(csm) = &self.csm {
        csm.render_shadow_maps(&mut encoder, vbuf, ibuf, count);
    }
    
    // 3. Main pass (sample shadows in fragment shader)
    // ... existing main render pass ...
}
```

#### C3: PBR Shader Update (10 minutes)
```wgsl
// astraweave-render/shaders/pbr.wgsl
@group(3) @binding(0) var shadow_atlas: texture_depth_2d;
@group(3) @binding(1) var shadow_sampler: sampler_comparison;
@group(3) @binding(2) var<uniform> cascades: array<ShadowCascade, 4>;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // ... existing PBR lighting ...
    
    let shadow_factor = sample_shadow_csm(in.world_pos, in.view_depth, in.normal);
    let lit_color = direct_light * shadow_factor;
    
    return vec4(lit_color, 1.0);
}
```

**Total Time**: 45 minutes  
**Deliverables**: Main renderer has functional CSM shadows  
**Next**: Phase 3 or different phase

---

### OPTION D: Phase 3 Advanced Shadows (8-12 hours)

**Implement next-gen shadow techniques:**

#### D1: PCSS - Percentage Closer Soft Shadows (3-4 hours)
- **Feature**: Variable penumbra (shadows softer at distance)
- **Algorithm**: Blocker search + adaptive kernel size
- **Benefit**: Physically accurate soft shadows
- **AAA Standard**: Used in UE5, Unity HDRP

#### D2: Contact Hardening (2-3 hours)
- **Feature**: Shadows sharper near occluder, softer far away
- **Algorithm**: Blocker distance modulates PCF kernel
- **Benefit**: Realistic shadow falloff
- **Example**: Character feet have sharp shadows, head has soft shadows

#### D3: Ray-Traced Contact Shadows (3-4 hours)
- **Feature**: Screen-space ray marching for fine detail
- **Algorithm**: DDA raycasting in depth buffer
- **Benefit**: Sub-pixel shadow accuracy
- **Use Case**: Self-shadowing, contact points

#### D4: Shadow Atlas Optimization (2-3 hours)
- **Feature**: Variable cascade resolutions (4K/2K/1K/512)
- **Benefit**: Better near-field quality, lower far-field cost
- **Implementation**: Non-uniform atlas subdivision

**Total Time**: 10-14 hours  
**Deliverables**: Industry-leading shadow system  
**Next**: Phase 4 (SSAO/SSGI) or Phase 5 (Volumetrics)

---

### OPTION E: Different Phase (Deferred)

**If CSM is "good enough", move to other systems:**

- **Phase 3**: SSAO/SSGI (ambient occlusion, global illumination)
- **Phase 4**: IBL Refinement (better environment lighting)
- **Phase 5**: Volumetrics (fog, god rays, clouds)
- **Phase 6**: Water Rendering (ocean, rivers, reflections)
- **Phase 7**: Terrain Rendering (heightmaps, splatting)
- **Phase 8**: Particle Systems (GPU-driven)
- **Phase 9**: Post-Processing (bloom, DOF, motion blur)
- **Phase 10**: HDR + Tonemapping (filmic, ACES)
- **Phase 11**: TAA (temporal anti-aliasing)
- **Phase 12**: Dynamic GI (probe-based or SDF)

---

## Recommendation

**My Suggestion**: **OPTION A → OPTION B → OPTION C** (sequential)

**Reasoning**:
1. **Option A (15 min)**: Validate visual quality meets standards
2. **Option B (1.5h)**: Polish CSM to AAA quality (tight-fit + blending)
3. **Option C (45 min)**: Integrate into main renderer

**Total**: 2.5 hours to complete Phase 2 (within 3-hour remaining budget)

**Then**: Move to Phase 3 (Advanced Shadows) or Phase 4 (SSAO/SSGI) with confidence that shadows are production-ready.

---

## Your Decision

**Please choose**:

- **"Test the demo"** → I'll guide you through Option A manual validation
- **"Polish CSM"** → I'll implement Option B (tight-fit + blending + profiling + docs)
- **"Integrate now"** → I'll implement Option C (merge into main Renderer)
- **"Go advanced"** → I'll start Phase 3 (PCSS, contact hardening, ray-traced)
- **"Switch phase"** → Tell me which phase (3-12) to tackle next
- **"Something else"** → Describe what you want

**Context**: You have a running CSM demo with 0 errors. The foundation is solid. The question is: perfect it now, or integrate and iterate?

---

**Awaiting your input...**
