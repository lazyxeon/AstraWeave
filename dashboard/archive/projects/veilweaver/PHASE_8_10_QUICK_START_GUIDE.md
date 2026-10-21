# Phase 8-10: Quick Start Guide

**Last Updated**: November 9, 2025  
**Purpose**: Fast-track guide to start Phase 8-10 implementation and validation

---

## 🚀 Getting Started (5 Minutes)

### Step 1: Read Core Documents (3 min)

**MUST READ** (in order):
1. ✅ **This document** (Quick Start Guide) — You're here!
2. ⏳ `PHASE_8_10_GAME_ENGINE_READINESS_COMPREHENSIVE_PLAN.md` (77 pages)
   - Executive Summary (2 pages)
   - Phase 8 Week-by-Week (30 pages)
   - Skip Phase 9-10 for now (come back later)
3. ⏳ `PHASE_8_10_MASTER_VALIDATION_CHECKLIST.md` (50 pages)
   - Week 1 Shadow Mapping tests (8 tests)
   - Skip other weeks for now

**Time**: 3 minutes to skim, 30 minutes for deep read

---

### Step 2: Set Up Environment (2 min)

**Prerequisites**:
- ✅ Rust 1.89.0+ (check: `rustc --version`)
- ✅ Tracy Profiler 0.11.1 (optional but recommended)
- ✅ Git (for version control)

**Clone Repository** (if not already):
```powershell
git clone https://github.com/lazyxeon/AstraWeave-AI-Native-Gaming-Engine.git
cd AstraWeave-AI-Native-Gaming-Engine
```

**Build Core Crates** (first time: 15-45 min):
```powershell
cargo build -p astraweave-render -p astraweave-audio -p astraweave-persistence-ecs --release
```

**Run Existing Example** (validate environment):
```powershell
cargo run -p unified_showcase --release
```

✅ **Success**: Window opens, renders scene  
❌ **Failure**: See `docs/supplemental/DEVELOPMENT_SETUP.md`

---

## 📋 Week 1: Shadow Mapping (Nov 10-16, 2025)

### Day 1: Enable Shadow Passes (2-3 hours)

**Goal**: Enable shadow depth passes in main renderer

**Location**: `astraweave-render/src/renderer.rs`

**Current Code** (lines ~500-550, approximate):
```rust
pub fn render(&mut self, encoder: &mut wgpu::CommandEncoder, ...) {
    // Main pass
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("main_pass"),
        // ... color/depth attachments
    });
    
    // Render meshes
    for mesh in meshes {
        self.render_mesh(&mut pass, mesh);
    }
}
```

**New Code** (add shadow passes BEFORE main pass):
```rust
pub fn render(&mut self, encoder: &mut wgpu::CommandEncoder, ...) {
    // NEW: Shadow passes (2 cascades)
    for cascade_idx in 0..2 {
        self.render_shadow_pass(encoder, meshes, cascade_idx);
    }
    
    // Existing main pass
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("main_pass"),
        // ... color/depth attachments
    });
    
    // Render meshes
    for mesh in meshes {
        self.render_mesh(&mut pass, mesh);
    }
}
```

**Add Method** (after `render()`):
```rust
fn render_shadow_pass(
    &mut self,
    encoder: &mut wgpu::CommandEncoder,
    meshes: &[Mesh],
    cascade_idx: u32,
) {
    let shadow_view = if cascade_idx == 0 {
        &self.shadow_layer0_view
    } else {
        &self.shadow_layer1_view
    };
    
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some(&format!("shadow_pass_cascade_{}", cascade_idx)),
        color_attachments: &[],
        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
            view: shadow_view,
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(1.0),
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
        }),
        // ... occlusion_query, timestamp_writes
    });
    
    pass.set_pipeline(&self.shadow_pipeline);
    pass.set_bind_group(0, &self.camera_bind_group, &[]);
    pass.set_bind_group(2, &self.light_bg_shadow, &[]); // Use shadow-only bind group
    
    for mesh in meshes {
        self.render_mesh_depth_only(&mut pass, mesh);
    }
}
```

**Test**:
```powershell
cargo check -p astraweave-render  # Should compile
cargo run -p unified_showcase --release  # Should render (no shadows yet)
```

---

### Day 2: Update Light Buffer (2-3 hours)

**Goal**: Upload cascade matrices to GPU

**Location**: `astraweave-render/src/renderer.rs::update_lights()`

**Current Code**:
```rust
pub fn update_lights(&mut self, queue: &wgpu::Queue, light_dir: Vec3) {
    let light_data = LightUniform {
        direction: light_dir.extend(0.0),
        color: Vec3::ONE.extend(1.0),
        // ... other fields
    };
    
    queue.write_buffer(&self.light_buffer, 0, bytemuck::bytes_of(&light_data));
}
```

**New Code** (add cascade matrices):
```rust
pub fn update_lights(&mut self, queue: &wgpu::Queue, light_dir: Vec3, camera_pos: Vec3, camera_target: Vec3) {
    // Calculate cascade splits (logarithmic)
    let near = 0.1;
    let far = 100.0;
    let split0 = near + (far - near) * 0.1; // 10% split
    let split1 = near + (far - near) * 0.3; // 30% split
    
    // Calculate cascade 0 (close range: 0.1 - split0)
    self.cascade0 = Self::calculate_cascade_matrix(light_dir, camera_pos, camera_target, near, split0);
    
    // Calculate cascade 1 (far range: split0 - split1)
    self.cascade1 = Self::calculate_cascade_matrix(light_dir, camera_pos, camera_target, split0, split1);
    
    let light_data = LightUniform {
        direction: light_dir.extend(0.0),
        color: Vec3::ONE.extend(1.0),
        cascade_matrix0: self.cascade0,
        cascade_matrix1: self.cascade1,
        // ... other fields
    };
    
    queue.write_buffer(&self.light_buffer, 0, bytemuck::bytes_of(&light_data));
}

fn calculate_cascade_matrix(light_dir: Vec3, camera_pos: Vec3, camera_target: Vec3, near: f32, far: f32) -> Mat4 {
    // Frustum corners in world space
    let frustum_corners = Self::calculate_frustum_corners(camera_pos, camera_target, near, far);
    
    // Fit orthographic projection to frustum
    let light_view = Mat4::look_at_rh(
        frustum_corners.center() - light_dir * 100.0,
        frustum_corners.center(),
        Vec3::Y,
    );
    
    let (min_x, max_x, min_y, max_y, min_z, max_z) = Self::calculate_bounds(&frustum_corners, light_view);
    
    let light_proj = Mat4::orthographic_rh(min_x, max_x, min_y, max_y, min_z, max_z);
    
    light_proj * light_view
}
```

**Test**:
```powershell
cargo check -p astraweave-render  # Should compile
cargo run -p unified_showcase --release  # Should render (cascade matrices calculated)
```

---

### Day 3: Enable Shadow Sampling (2-3 hours)

**Goal**: Uncomment shadow sampling in fragment shader

**Location**: `astraweave-render/src/renderer.rs` (shader inline, lines 164-194)

**Current Code** (shadow sampling commented out or disabled):
```wgsl
// Shadow sampling
// Cascaded shadow mapping (2 cascades)
// let lp = uLight.cascade_matrix0 * vec4<f32>(world_pos, 1.0);
// ... (rest commented out)
```

**New Code** (enable shadow sampling):
```wgsl
// Shadow sampling
// Cascaded shadow mapping (2 cascades)
let lp = uLight.cascade_matrix0 * vec4<f32>(world_pos, 1.0);
let ndc_shadow = lp.xyz / lp.w;
let uv = ndc_shadow.xy * 0.5 + vec2<f32>(0.5, 0.5);
let depth = ndc_shadow.z;

// Cascade selection (distance-based)
let cam_dist = length(world_pos - uCamera.position.xyz);
let layer = select(0u, 1u, cam_dist > 10.0);

var shadow: f32 = 1.0;
if (uv.x >= 0.0 && uv.x <= 1.0 && uv.y >= 0.0 && uv.y <= 1.0 && depth >= 0.0 && depth <= 1.0) {
    // 3x3 PCF filtering
    let dims = vec2<f32>(textureDimensions(shadow_tex).xy);
    let texel_size = 1.0 / dims;
    let bias = 0.005;
    var sum: f32 = 0.0;
    for (var y: i32 = -1; y <= 1; y = y + 1) {
        for (var x: i32 = -1; x <= 1; x = x + 1) {
            let o = vec2<f32>(f32(x), f32(y)) * texel_size;
            sum = sum + textureSampleCompare(shadow_tex, shadow_sampler, uv + o, layer, depth - bias);
        }
    }
    shadow = sum / 9.0;
}

// Apply shadow to lighting
var lit_color = (diffuse + specular) * radiance * NdotL * shadow + base_color * 0.08;
```

**Test**:
```powershell
cargo check -p astraweave-render  # Should compile
cargo run -p unified_showcase --release  # SHADOWS SHOULD BE VISIBLE!
```

✅ **Success**: Shadows visible under objects  
❌ **Failure**: Check shader compilation errors, verify light direction

---

### Day 4-5: Testing & Benchmarking (4-6 hours)

**Goal**: Run all 8 shadow tests, capture evidence

**Test Checklist** (from `PHASE_8_10_MASTER_VALIDATION_CHECKLIST.md`):
- [ ] S1.1: Shadow visibility (screenshot)
- [ ] S1.2: Cascade coverage (debug visualization)
- [ ] S1.3: Peter-panning fix (visual inspection)
- [ ] S1.4: Shadow acne fix (visual inspection)
- [ ] S1.5: PCF smoothness (visual inspection)
- [ ] S1.6: Performance (<2 ms @ 100 meshes)
- [ ] S1.7: Cascade transitions (visual inspection)
- [ ] S1.8: Dynamic lights (rotating light test)

**Benchmarking**:
```powershell
# Add benchmark to astraweave-render/benches/shadow_benchmarks.rs
cargo bench -p astraweave-render --bench shadow_benchmarks
```

**Tracy Profiling**:
```powershell
# Run with Tracy enabled
cargo run -p unified_showcase --release --features tracy
# Capture profile, save to profiles/week_1_shadows.tracy
```

**Evidence Collection**:
1. Screenshots: `assets/screenshots/week_1_shadows_*.png`
2. Tracy profile: `profiles/week_1_shadows.tracy`
3. Benchmark results: `target/criterion/shadow_pass_100_meshes/report/index.html`

---

## 📊 Progress Tracking

### Daily Updates

**Template** (copy to daily log):
```markdown
## Day N - [Date]

### Completed:
- [x] Task 1 description
- [x] Task 2 description

### Blocked:
- [ ] Issue description (blocker, needs help)

### Next Day:
- [ ] Task 3 description
- [ ] Task 4 description

### Evidence:
- Screenshot: `assets/screenshots/day_N_*.png`
- Tracy: `profiles/day_N.tracy`
- Benchmark: `target/criterion/*/report/index.html`
```

---

### Weekly Summary

**Template** (copy to weekly report):
```markdown
## Week N - [Date Range]

### Achievements:
- ✅ Feature 1 complete
- ✅ Feature 2 complete

### Tests Passing:
- [x] Test 1 (evidence: screenshot)
- [x] Test 2 (evidence: benchmark)

### Metrics:
- Frame time: X ms (target: <16.67 ms)
- Shadow pass: X ms (target: <2 ms)
- Tests passing: X/8 (target: 8/8)

### Blockers:
- None

### Next Week:
- [ ] Week N+1 tasks
```

---

## 🛠️ Tools & Resources

### Essential Tools

**Tracy Profiler**:
- Download: https://github.com/wolfpld/tracy/releases
- Tutorial: `docs/supplemental/TRACY_PROFILING_GUIDE.md` (if exists)
- Hotkeys: Space (pause), F1 (statistics), F2 (memory)

**Rust Analyzer**:
- VSCode extension: `rust-lang.rust-analyzer`
- Go to definition: F12
- Find all references: Shift+F12

**Git**:
```powershell
# Create feature branch
git checkout -b phase8-week1-shadows

# Commit frequently
git add .
git commit -m "feat(render): enable shadow depth passes"

# Push to remote
git push origin phase8-week1-shadows
```

---

### Documentation

**Key Documents**:
1. `PHASE_8_10_GAME_ENGINE_READINESS_COMPREHENSIVE_PLAN.md` - Full roadmap
2. `PHASE_8_10_MASTER_VALIDATION_CHECKLIST.md` - Test matrix
3. `docs/ASTRAWEAVE_MASTER_ROADMAP_2025_2027.md` - Strategic plan
4. `docs/current/MASTER_ROADMAP.md` - Current status

**API Reference**:
- `cargo doc --open -p astraweave-render` (Rust docs)
- wgpu docs: https://docs.rs/wgpu/latest/wgpu/

---

## 🎯 Success Criteria

### Week 1 (Shadow Mapping)

**Required**:
- ✅ 8/8 tests passing
- ✅ Tracy profile <2 ms for shadow pass
- ✅ Visual validation clean (no artifacts)
- ✅ Benchmark results documented

**Optional**:
- ⭐ Shadow quality settings UI
- ⭐ Cascade debug visualization
- ⭐ Performance optimization (<1 ms)

---

## ❓ FAQ

### Q: What if a test fails?

**A**: Document why it failed, fix the issue, re-run test. Do NOT proceed to next test until current test passes.

### Q: Can I skip a test?

**A**: NO. All tests must pass. No exceptions.

### Q: What if I'm blocked?

**A**: Document blocker in daily log, ask for help in team chat/Discord, try alternative approach.

### Q: How do I know if my code is correct?

**A**: Run tests. If tests pass, code is correct. If tests fail, code is incorrect.

### Q: What if the plan is wrong?

**A**: Update the plan! These documents are living documents. If you find a better approach, document it and update the plan.

---

## 🚦 Next Steps

**IMMEDIATE (Today)**:
1. ✅ Read this Quick Start Guide (you just did!)
2. ⏳ Set up environment (Step 2 above)
3. ⏳ Read comprehensive plan (Executive Summary at minimum)
4. ⏳ Read validation checklist (Week 1 section)

**Week 1 (Nov 10-16)**:
1. ⏳ Day 1: Enable shadow passes (Day 1 guide above)
2. ⏳ Day 2: Update light buffer (Day 2 guide above)
3. ⏳ Day 3: Enable shadow sampling (Day 3 guide above)
4. ⏳ Day 4-5: Testing & benchmarking (Day 4-5 guide above)
5. ⏳ Friday: Weekly summary report

**Week 2+ (Nov 17+)**:
- Follow comprehensive plan week-by-week
- All tests must pass before proceeding
- Update progress daily, report weekly

---

**Let's build a world-class engine! 🚀**
