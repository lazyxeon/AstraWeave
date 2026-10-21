# Rendering System Fix - Quick Reference Guide

**Status:** Ready to Execute  
**Last Updated:** 2025-11-12

---

## 📋 Quick Summary

### Critical Issues (Fix First)
1. **Depth Texture Resize** - Causes crashes/errors on window resize
2. **Terrain Sampler** - Textures don't tile correctly
3. **Roughness Channel** - Wrong PBR channel read
4. **sRGB Format** - Color space mismatch

### High Priority (Week 1)
5. **Back-face Culling** - 50% performance loss
6. **Surface Errors** - Window minimize crashes
7. **Terrain Materials** - Missing per-material normals/roughness
8. **Terrain Mipmaps** - Distance aliasing

### Testing Infrastructure (Week 2-3)
9. **Visual Regression Tests** - Golden image comparison
10. **Shader Validation** - CI compilation checks
11. **GPU Leak Detection** - Resource tracking
12. **Performance Benchmarks** - Frame-time budgets

---

## 🎯 Execution Order

### Week 1: Critical Fixes
```
Day 1: Tasks 1-2 (Depth + Terrain)
Day 2: Tasks 3-4 (Roughness + sRGB)
Day 3: Task 5 (Culling)
Day 4: Tasks 6-7 (Surface + Terrain Materials)
Day 5: Task 8 (Mipmaps)
```

### Week 2: Testing Infrastructure Part 1
```
Day 6-8: Task 9 (Visual Regression)
Day 9: Task 10 (Shader Validation)
Day 10: Task 11 (Leak Detection) - Day 1
```

### Week 3: Testing Infrastructure Part 2 + Polish
```
Day 11: Task 11 (Leak Detection) - Day 2
Day 12: Task 12 (Performance)
Day 13-14: Task 13-15 (Medium Priority Fixes)
Day 15-16: Task 16 (Coverage Push)
```

---

## 🔧 Quick Fix Snippets

### Fix 1: Depth Texture Resize
**File:** `examples/unified_showcase/src/main_bevy_v2.rs:2527`

```rust
fn resize(&mut self, new_size: PhysicalSize<u32>) {
    if new_size.width > 0 && new_size.height > 0 {
        self.size = new_size;
        self.surface_config.width = new_size.width;
        self.surface_config.height = new_size.height;
        self.surface.configure(&self.device, &self.surface_config);
        self.camera.aspect = new_size.width as f32 / new_size.height as f32;
        
        // FIX: Recreate depth texture
        let depth_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size: wgpu::Extent3d {
                width: new_size.width,
                height: new_size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT 
                | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        
        self.depth_texture = depth_texture.create_view(
            &wgpu::TextureViewDescriptor::default()
        );
    }
}
```

### Fix 2: Terrain Sampler
**File:** `examples/unified_showcase/src/main_bevy_v2.rs:1271`

```rust
// Add after atlas sampler creation
let terrain_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
    label: Some("Terrain Sampler"),
    address_mode_u: wgpu::AddressMode::Repeat,
    address_mode_v: wgpu::AddressMode::Repeat,
    address_mode_w: wgpu::AddressMode::Repeat,
    mag_filter: wgpu::FilterMode::Linear,
    min_filter: wgpu::FilterMode::Linear,
    mipmap_filter: wgpu::FilterMode::Linear,
    anisotropy_clamp: 16,
    ..Default::default()
});

// Update terrain bind group (line 1549)
// Change: &sampler -> &terrain_sampler
```

### Fix 3: Roughness Channel
**File:** `examples/unified_showcase/src/pbr_shader.wgsl:196`

```wgsl
// OLD: let roughness = textureSample(roughness_texture, material_sampler, input.uv).r;
// NEW:
let mra_sample = textureSample(roughness_texture, material_sampler, input.uv);
let metallic = mra_sample.r;
let roughness = mra_sample.g;  // FIX: Use green channel
let ao = mra_sample.b;
```

### Fix 4: sRGB Format
**File:** `examples/unified_showcase/src/main_bevy_v2.rs:2582`

```rust
let surface_caps = surface.get_capabilities(&adapter);
let surface_format = surface_caps
    .formats
    .iter()
    .copied()
    .find(|f| f.is_srgb())
    .unwrap_or(surface_caps.formats[0]);

let surface_config = wgpu::SurfaceConfiguration {
    format: surface_format,  // Use sRGB
    // ... rest unchanged
};
```

### Fix 5: Enable Culling
**File:** `examples/unified_showcase/src/main_bevy_v2.rs:1592`

```rust
primitive: wgpu::PrimitiveState {
    cull_mode: Some(wgpu::Face::Back),  // Change from None
    // ... rest unchanged
},
```

### Fix 6: Surface Error Handling
**File:** `examples/unified_showcase/src/main_bevy_v2.rs:2342`

```rust
let frame = match surface.get_current_texture() {
    Ok(frame) => frame,
    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
        log::warn!("Surface lost/outdated, reconfiguring...");
        self.resize(self.size);
        return Ok(());
    }
    Err(wgpu::SurfaceError::Timeout) => {
        log::warn!("Surface timeout, skipping frame");
        return Ok(());
    }
    Err(wgpu::SurfaceError::OutOfMemory) => {
        log::error!("GPU out of memory!");
        return Err(wgpu::SurfaceError::OutOfMemory.into());
    }
};
```

---

## ✅ Testing Checklist

### After Each Fix
- [ ] Code compiles without warnings
- [ ] Existing tests still pass
- [ ] Manual testing shows fix works
- [ ] New automated test added
- [ ] Documentation updated
- [ ] Git commit with clear message

### Before Merge
- [ ] All TODO tasks marked completed
- [ ] Full test suite passes
- [ ] No performance regressions
- [ ] Code reviewed
- [ ] CHANGELOG.md updated

---

## 📊 Success Metrics

### Phase 1 Complete When:
- ✅ All 4 critical bugs fixed
- ✅ 4 new tests passing
- ✅ Zero WebGPU validation errors
- ✅ Visual quality improved

### Phase 2 Complete When:
- ✅ 30-50% performance improvement
- ✅ No crashes on resize/minimize
- ✅ Realistic terrain rendering
- ✅ No aliasing artifacts

### Phase 3 Complete When:
- ✅ 15+ visual regression tests
- ✅ All shaders validate in CI
- ✅ Zero GPU leaks detected
- ✅ Performance budgets enforced

### Phase 4 Complete When:
- ✅ Test coverage ≥ 75%
- ✅ All 12 issues resolved
- ✅ Documentation complete
- ✅ Production-ready quality

---

## 📚 Reference Documents

1. **RENDERING_SYSTEM_ANALYSIS.md** - Detailed issue analysis
2. **RENDERING_FIX_IMPLEMENTATION_PLAN.md** - Complete implementation plan
3. **RENDERING_QUICK_REFERENCE.md** - This document

---

## 🚀 Ready to Start?

**First Task:** Fix Depth Texture Resize Bug  
**File:** `examples/unified_showcase/src/main_bevy_v2.rs`  
**Line:** 2527-2541  
**Duration:** 4 hours  
**Priority:** P0 Critical

**Command to run tests after fix:**
```bash
cargo test --package astraweave-render -- --nocapture
cargo run --example unified_showcase --features=bevy
```

Good luck! 🎮✨
