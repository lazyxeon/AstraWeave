# AstraWeave Rendering System - Comprehensive Fix & Testing Implementation Plan

**Plan Version:** 1.0  
**Created:** 2025-11-12  
**Total Estimated Duration:** 14-16 days  
**Target Completion:** 2025-11-30

---

## Executive Summary

This plan provides a complete roadmap to:
1. Fix all 12 identified rendering issues from the analysis
2. Implement comprehensive testing infrastructure to prevent future regressions
3. Achieve 75%+ test coverage for the rendering system
4. Establish quality assurance processes

**Current State:**
- ✅ 323 tests, 63.62% coverage
- ❌ 4 critical bugs affecting visual quality
- ⚠️ No visual regression testing
- ⚠️ No shader validation in CI

**Target State:**
- ✅ All critical bugs fixed with tests
- ✅ Visual regression test suite (15+ golden images)
- ✅ Shader compilation validation
- ✅ GPU resource leak detection
- ✅ 75%+ test coverage
- ✅ Automated performance regression detection

---

## Phase 1: Critical Bug Fixes (Days 1-3)

### Objective
Fix all 4 critical rendering bugs that cause immediate visual failures.

---

### Task 1.1: Fix Depth Texture Resize Bug ⚠️ CRITICAL
**Priority:** P0 (Highest)  
**Duration:** 4 hours  
**Files:** `examples/unified_showcase/src/main_bevy_v2.rs`

#### Implementation Steps

1. **Read current resize implementation** (15 min)
   ```rust
   // Lines 2527-2541: Current resize handler
   ```

2. **Implement depth texture recreation** (1 hour)
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

3. **Write automated test** (2 hours)
   ```rust
   // File: examples/unified_showcase/tests/resize_depth_texture.rs
   
   #[test]
   fn test_depth_texture_recreated_on_resize() {
       // Create headless renderer
       let mut app = create_test_app(800, 600);
       
       // Initial render
       app.render_frame().expect("Initial render failed");
       
       // Resize multiple times
       for (width, height) in [(1024, 768), (1920, 1080), (640, 480)] {
           app.resize(PhysicalSize::new(width, height));
           
           // Verify depth texture matches new size
           assert_eq!(app.depth_texture_size(), (width, height));
           
           // Render should succeed without validation errors
           app.render_frame().expect("Render after resize failed");
       }
   }
   ```

4. **Manual testing** (30 min)
   - Launch unified_showcase
   - Resize window to various sizes
   - Monitor console for WebGPU validation errors
   - Verify depth testing works correctly

5. **Documentation** (30 min)
   - Add comments explaining depth texture lifecycle
   - Update CHANGELOG.md

#### Success Criteria
- ✅ No WebGPU validation errors on resize
- ✅ Depth testing works at all window sizes
- ✅ Automated test passes in CI
- ✅ Manual resize testing shows no visual artifacts

---

### Task 1.2: Fix Terrain Sampler (Tiling Issue) ⚠️ CRITICAL
**Priority:** P0  
**Duration:** 3 hours  
**Files:** `examples/unified_showcase/src/main_bevy_v2.rs`

#### Implementation Steps

1. **Create dedicated terrain sampler** (45 min)
   ```rust
   // After line 1271 (after atlas sampler creation)
   
   // Create terrain sampler with proper tiling support
   let terrain_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
       label: Some("Terrain Sampler"),
       address_mode_u: wgpu::AddressMode::Repeat,
       address_mode_v: wgpu::AddressMode::Repeat,
       address_mode_w: wgpu::AddressMode::Repeat,
       mag_filter: wgpu::FilterMode::Linear,
       min_filter: wgpu::FilterMode::Linear,
       mipmap_filter: wgpu::FilterMode::Linear,
       anisotropy_clamp: 16,  // High quality
       ..Default::default()
   });
   ```

2. **Update terrain bind group** (30 min)
   ```rust
   // Lines 1541-1553: Update terrain bind group
   let terrain_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
       label: Some("Terrain Bind Group"),
       layout: &terrain_bind_group_layout,
       entries: &[
           wgpu::BindGroupEntry {
               binding: 0,
               resource: wgpu::BindingResource::TextureView(&terrain_texture_array_view),
           },
           wgpu::BindGroupEntry {
               binding: 1,
               resource: wgpu::BindingResource::Sampler(&terrain_sampler), // FIXED
           },
       ],
   });
   ```

3. **Write visual validation test** (1 hour)
   ```rust
   // File: examples/unified_showcase/tests/terrain_tiling.rs
   
   #[test]
   fn test_terrain_textures_tile_correctly() {
       let mut app = create_test_app(512, 512);
       
       // Render terrain with 10x tiled UVs
       app.render_terrain_test_pattern();
       
       // Capture frame buffer
       let pixels = app.capture_framebuffer();
       
       // Verify tiling: center and edges should have consistent patterns
       // (not clamped edge colors)
       let center_color = get_pixel(&pixels, 256, 256);
       let edge_color = get_pixel(&pixels, 510, 510);
       
       // Should not be clamped to edge color
       assert_ne!(edge_color, [0, 0, 0, 255], "Texture appears clamped");
       
       // Pattern should repeat (check periodicity)
       assert_pattern_repeats(&pixels, 512, 512, 10);
   }
   ```

4. **Visual inspection** (45 min)
   - Load terrain scene
   - Verify seamless tiling at all zoom levels
   - Check for edge artifacts
   - Test with different terrain materials

#### Success Criteria
- ✅ Terrain textures tile seamlessly
- ✅ No edge clamping artifacts
- ✅ Smooth filtering at all distances
- ✅ Visual test validates tiling behavior

---

### Task 1.3: Fix Roughness Channel Mismatch ⚠️ CRITICAL
**Priority:** P0  
**Duration:** 2 hours  
**Files:** `examples/unified_showcase/src/pbr_shader.wgsl`

#### Implementation Steps

1. **Update shader roughness sampling** (30 min)
   ```wgsl
   // Line 196-197: Fix channel selection
   
   // OLD (INCORRECT):
   // let roughness = textureSample(roughness_texture, material_sampler, input.uv).r;
   
   // NEW (CORRECT - MRA packing):
   let mra_sample = textureSample(roughness_texture, material_sampler, input.uv);
   let metallic = mra_sample.r;   // R = Metallic
   let roughness = mra_sample.g;  // G = Roughness (FIXED)
   let ao = mra_sample.b;         // B = Ambient Occlusion
   ```

2. **Apply AO to lighting** (30 min)
   ```wgsl
   // After PBR calculation (around line 200+)
   
   // Apply ambient occlusion to indirect lighting
   let ambient = vec3<f32>(0.03) * albedo * ao;  // Use AO channel
   ```

3. **Create MRA validation test** (45 min)
   ```rust
   // File: astraweave-render/tests/test_mra_channels.rs
   
   #[test]
   fn test_mra_texture_channels_correctly_mapped() {
       // Create test MRA texture: R=1.0, G=0.5, B=0.3
       let test_mra = create_test_texture_rgba(&[
           255, 128, 77, 255  // R=Metallic, G=Roughness, B=AO
       ]);
       
       // Render with shader
       let result = render_with_mra_material(test_mra);
       
       // Verify roughness is 0.5 (from green channel)
       let extracted_roughness = extract_roughness_from_specular(result);
       assert!((extracted_roughness - 0.5).abs() < 0.05, 
           "Roughness should be ~0.5 from green channel, got {}", 
           extracted_roughness);
   }
   ```

4. **Visual validation** (15 min)
   - Render reference PBR spheres with known roughness
   - Compare specular highlights against expected values
   - Verify metal/rough materials look correct

#### Success Criteria
- ✅ Roughness sampled from correct channel (green)
- ✅ Materials render with accurate PBR response
- ✅ Metal surfaces are appropriately reflective
- ✅ Rough surfaces are appropriately diffuse

---

### Task 1.4: Fix sRGB Swapchain Format ⚠️ CRITICAL
**Priority:** P0  
**Duration:** 2 hours  
**Files:** `examples/unified_showcase/src/main_bevy_v2.rs`

#### Implementation Steps

1. **Implement sRGB format selection** (1 hour)
   ```rust
   // Line 2582: Replace blind format selection
   
   let surface_caps = surface.get_capabilities(&adapter);
   
   // Prefer sRGB format for correct color space
   let surface_format = surface_caps
       .formats
       .iter()
       .copied()
       .find(|f| f.is_srgb())
       .unwrap_or_else(|| {
           log::warn!("No sRGB format available, using linear: {:?}", 
               surface_caps.formats[0]);
           surface_caps.formats[0]
       });
   
   log::info!("Selected surface format: {:?}", surface_format);
   
   let surface_config = wgpu::SurfaceConfiguration {
       usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
       format: surface_format,  // Use selected sRGB format
       width: size.width,
       height: size.height,
       present_mode: surface_caps.present_modes[0],
       alpha_mode: surface_caps.alpha_modes[0],
       view_formats: vec![],
   };
   ```

2. **Write color space validation test** (45 min)
   ```rust
   // File: examples/unified_showcase/tests/color_space_srgb.rs
   
   #[test]
   fn test_surface_uses_srgb_format() {
       let app = create_test_app(800, 600);
       
       // Verify surface format is sRGB
       let format = app.get_surface_format();
       assert!(format.is_srgb(), 
           "Surface should use sRGB format, got {:?}", format);
   }
   
   #[test]
   fn test_color_accuracy_srgb() {
       let mut app = create_test_app(256, 256);
       
       // Render pure red albedo (sRGB 255, 0, 0)
       app.render_solid_color_sphere([1.0, 0.0, 0.0, 1.0]);
       
       let pixels = app.capture_framebuffer();
       let center_pixel = get_pixel(&pixels, 128, 128);
       
       // Should be bright red in sRGB space (not washed out)
       assert!(center_pixel[0] > 200, "Red channel too dim: {}", center_pixel[0]);
       assert!(center_pixel[1] < 50, "Green channel too bright: {}", center_pixel[1]);
       assert!(center_pixel[2] < 50, "Blue channel too bright: {}", center_pixel[2]);
   }
   ```

3. **Multi-platform testing** (15 min)
   - Test on Windows (DXGI)
   - Test on Linux (Vulkan)
   - Verify sRGB format selection on all platforms

#### Success Criteria
- ✅ Swapchain uses sRGB format when available
- ✅ Colors match source textures (not washed out)
- ✅ Gamma-correct presentation
- ✅ Works on all platforms

---

### Phase 1 Deliverables
- ✅ 4 critical bugs fixed
- ✅ 4 new automated tests
- ✅ All fixes validated manually and automatically
- ✅ Documentation updated

---

## Phase 2: High Priority Warnings (Days 4-6)

### Objective
Address performance and stability issues that impact production quality.

---

### Task 2.1: Enable Back-Face Culling
**Priority:** P1  
**Duration:** 2 hours  
**Files:** `examples/unified_showcase/src/main_bevy_v2.rs`

#### Implementation Steps

1. **Enable culling in pipeline** (15 min)
   ```rust
   // Line 1592: Enable back-face culling
   primitive: wgpu::PrimitiveState {
       topology: wgpu::PrimitiveTopology::TriangleList,
       strip_index_format: None,
       front_face: wgpu::FrontFace::Ccw,
       cull_mode: Some(wgpu::Face::Back),  // ENABLED
       polygon_mode: wgpu::PolygonMode::Fill,
       unclipped_depth: false,
       conservative: false,
   },
   ```

2. **Verify mesh winding orders** (1 hour)
   ```rust
   // Add validation for loaded meshes
   fn validate_mesh_winding(mesh: &Mesh) -> Result<()> {
       // Check that triangles are CCW for outward normals
       for triangle in mesh.triangles() {
           let normal = compute_face_normal(triangle);
           let vertex_normal_avg = average_vertex_normals(triangle);
           
           if normal.dot(vertex_normal_avg) < 0.0 {
               log::warn!("Triangle winding may be reversed");
           }
       }
       Ok(())
   }
   ```

3. **Performance benchmark** (30 min)
   ```rust
   // File: astraweave-render/benches/culling_performance.rs
   
   fn bench_culling_enabled(c: &mut Criterion) {
       let mut app = create_benchmark_scene();  // Complex scene
       
       c.bench_function("culling_enabled", |b| {
           b.iter(|| app.render_frame())
       });
   }
   
   fn bench_culling_disabled(c: &mut Criterion) {
       let mut app = create_benchmark_scene();
       app.disable_culling();
       
       c.bench_function("culling_disabled", |b| {
           b.iter(|| app.render_frame())
       });
   }
   
   // Expected: ~2x speedup with culling enabled for closed meshes
   ```

4. **Visual verification** (15 min)
   - Ensure no meshes disappear (wrong winding)
   - Verify performance improvement

#### Success Criteria
- ✅ Back-face culling enabled
- ✅ No visual artifacts (meshes disappearing)
- ✅ Measurable performance improvement (~30-50%)
- ✅ All meshes have correct winding order

---

### Task 2.2: Robust Surface Error Handling
**Priority:** P1  
**Duration:** 3 hours  
**Files:** `examples/unified_showcase/src/main_bevy_v2.rs`

#### Implementation Steps

1. **Implement error handling** (1.5 hours)
   ```rust
   // Line 2342: Replace simple error propagation
   
   let frame = match surface.get_current_texture() {
       Ok(frame) => frame,
       Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
           log::warn!("Surface lost/outdated, reconfiguring...");
           self.resize(self.size);
           return Ok(());  // Skip frame
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

2. **Write stress test** (1 hour)
   ```rust
   // File: examples/unified_showcase/tests/surface_resilience.rs
   
   #[test]
   fn test_window_minimize_restore() {
       let mut app = create_test_app(800, 600);
       
       // Render normally
       app.render_frame().expect("Initial render");
       
       // Simulate minimize (surface lost)
       app.simulate_surface_lost();
       
       // Should not crash, just skip frame
       assert!(app.render_frame().is_ok());
       
       // Simulate restore
       app.simulate_surface_restore();
       
       // Should render successfully again
       app.render_frame().expect("Render after restore");
   }
   ```

3. **Manual testing** (30 min)
   - Minimize and restore window repeatedly
   - Switch between fullscreen and windowed
   - Monitor for crashes or errors

#### Success Criteria
- ✅ No crashes on window minimize/restore
- ✅ Graceful handling of surface errors
- ✅ Application recovers automatically
- ✅ Stress test passes

---

### Task 2.3: Terrain Material-Specific Normals/Roughness
**Priority:** P1  
**Duration:** 6 hours  
**Files:** Multiple

#### Implementation Steps

1. **Design texture array approach** (1 hour)
   - Document array structure
   - Plan bind group layout changes
   - Estimate memory impact

2. **Implement normal texture array** (2 hours)
   ```rust
   // Create normal texture array for terrain
   let terrain_normal_paths = [
       "assets/terrain/grass_normal.png",
       "assets/terrain/rock_normal.png",
       "assets/terrain/dirt_normal.png",
       "assets/terrain/snow_normal.png",
   ];
   
   let terrain_normal_array = create_texture_array_with_mipmaps(
       &device,
       &queue,
       &terrain_normal_paths,
       wgpu::TextureFormat::Rgba8Unorm,  // Linear space for normals
       "Terrain Normals"
   );
   ```

3. **Implement roughness texture array** (1 hour)
   ```rust
   let terrain_roughness_paths = [
       "assets/terrain/grass_roughness.png",
       "assets/terrain/rock_roughness.png",
       // ...
   ];
   
   let terrain_roughness_array = create_texture_array_with_mipmaps(
       &device,
       &queue,
       &terrain_roughness_paths,
       wgpu::TextureFormat::Rgba8Unorm,
       "Terrain Roughness"
   );
   ```

4. **Update shader** (1.5 hours)
   ```wgsl
   // Add terrain normal/roughness arrays to bind group
   @group(2) @binding(2) var terrain_normal_array: texture_2d_array<f32>;
   @group(2) @binding(3) var terrain_roughness_array: texture_2d_array<f32>;
   
   // Sample based on material_id
   if (is_terrain) {
       let material_id = input.material_id;
       let normal_sample = textureSample(
           terrain_normal_array,
           terrain_sampler,
           terrain_uv,
           material_id
       );
       // Convert normal map to world space
       // ...
   }
   ```

5. **Visual validation** (30 min)
   - Verify different terrain materials have distinct surface properties
   - Check normal mapping quality

#### Success Criteria
- ✅ Each terrain material has unique normal/roughness
- ✅ Realistic terrain shading variation
- ✅ Performance remains acceptable
- ✅ Memory usage within budget

---

### Task 2.4: Terrain Mipmaps
**Priority:** P1  
**Duration:** 4 hours  
**Files:** `examples/unified_showcase/src/main_bevy_v2.rs`

#### Implementation Steps

1. **Implement mipmap generation utility** (2 hours)
   ```rust
   // File: examples/unified_showcase/src/mipmap_generator.rs
   
   fn generate_mipmaps_for_array(
       device: &wgpu::Device,
       queue: &wgpu::Queue,
       texture: &wgpu::Texture,
       layer_count: u32,
   ) {
       let mip_count = calculate_mip_levels(texture.size().width);
       
       for layer in 0..layer_count {
           for mip in 1..mip_count {
               // Use compute shader or render pass to generate each mip level
               generate_mip_level(device, queue, texture, layer, mip);
           }
       }
   }
   
   fn generate_mip_level(
       device: &wgpu::Device,
       queue: &wgpu::Queue,
       texture: &wgpu::Texture,
       layer: u32,
       mip: u32,
   ) {
       // Blit from previous mip level (mip-1) to current mip
       // Using bilinear filtering
   }
   ```

2. **Update texture array creation** (1.5 hours)
   ```rust
   let mip_count = calculate_mip_levels(TERRAIN_TEXTURE_SIZE);
   
   let terrain_texture_array = device.create_texture(&wgpu::TextureDescriptor {
       label: Some("Terrain Texture Array"),
       size: wgpu::Extent3d {
           width: TERRAIN_TEXTURE_SIZE,
           height: TERRAIN_TEXTURE_SIZE,
           depth_or_array_layers: terrain_count as u32,
       },
       mip_level_count: mip_count,  // ADDED
       sample_count: 1,
       dimension: wgpu::TextureDimension::D2,
       format: wgpu::TextureFormat::Rgba8UnormSrgb,
       usage: wgpu::TextureUsages::TEXTURE_BINDING 
           | wgpu::TextureUsages::COPY_DST 
           | wgpu::TextureUsages::RENDER_ATTACHMENT,  // For mipmap generation
       view_formats: &[],
   });
   
   // Copy base level, then generate mipmaps
   copy_textures_to_array(&queue, &terrain_texture_array, &source_textures);
   generate_mipmaps_for_array(&device, &queue, &terrain_texture_array, terrain_count);
   ```

3. **Visual validation** (30 min)
   - View terrain from various distances
   - Verify smooth LOD transitions
   - Check for aliasing/moiré patterns

#### Success Criteria
- ✅ Mipmaps present for all terrain textures
- ✅ No aliasing at distance
- ✅ Smooth mip transitions
- ✅ Performance improvement (better texture cache usage)

---

### Phase 2 Deliverables
- ✅ Back-face culling enabled (~40% performance gain)
- ✅ Robust surface error handling (no crashes)
- ✅ Terrain materials with proper normals/roughness
- ✅ Terrain mipmaps (better visual quality)
- ✅ 6 new tests + 1 benchmark

---

## Phase 3: Testing Infrastructure (Days 7-11)

### Objective
Build comprehensive testing infrastructure to prevent regressions and ensure quality.

---

### Task 3.1: Visual Regression Testing Suite
**Priority:** P0  
**Duration:** 3 days  
**Files:** New test infrastructure

#### Implementation Steps

##### Day 1: Framework Setup

1. **Create test utilities** (4 hours)
   ```rust
   // File: astraweave-render/tests/visual_regression/mod.rs
   
   pub struct VisualTestContext {
       device: wgpu::Device,
       queue: wgpu::Queue,
       width: u32,
       height: u32,
   }
   
   impl VisualTestContext {
       pub async fn new(width: u32, height: u32) -> Self {
           let (device, queue) = create_headless_device().await;
           Self { device, queue, width, height }
       }
       
       pub fn render_to_buffer(
           &self,
           render_fn: impl FnOnce(&wgpu::Device, &wgpu::Queue, &wgpu::TextureView),
       ) -> Vec<u8> {
           // Create render target
           let texture = self.device.create_texture(&wgpu::TextureDescriptor {
               label: Some("Visual Test Render Target"),
               size: wgpu::Extent3d {
                   width: self.width,
                   height: self.height,
                   depth_or_array_layers: 1,
               },
               mip_level_count: 1,
               sample_count: 1,
               dimension: wgpu::TextureDimension::D2,
               format: wgpu::TextureFormat::Rgba8UnormSrgb,
               usage: wgpu::TextureUsages::RENDER_ATTACHMENT 
                   | wgpu::TextureUsages::COPY_SRC,
               view_formats: &[],
           });
           
           let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
           
           // Render
           render_fn(&self.device, &self.queue, &view);
           
           // Copy to CPU buffer
           let buffer = copy_texture_to_buffer(&self.device, &self.queue, &texture);
           buffer
       }
       
       pub fn assert_image_matches(
           &self,
           actual: &[u8],
           golden_path: &str,
           tolerance: u8,
       ) {
           let golden = load_golden_image(golden_path);
           
           let (max_diff, avg_diff) = image_delta(actual, &golden);
           
           if max_diff > tolerance {
               // Save diff image for debugging
               save_diff_image(&actual, &golden, "test_failure_diff.png");
               
               panic!(
                   "Visual regression detected!\n\
                    Max pixel diff: {} (tolerance: {})\n\
                    Avg pixel diff: {:.2}\n\
                    Diff image saved to test_failure_diff.png",
                   max_diff, tolerance, avg_diff
               );
           }
       }
   }
   
   fn image_delta(a: &[u8], b: &[u8]) -> (u8, f32) {
       assert_eq!(a.len(), b.len());
       let mut max_diff = 0u8;
       let mut sum_diff = 0u32;
       
       for (pixel_a, pixel_b) in a.iter().zip(b.iter()) {
           let diff = (*pixel_a as i16 - *pixel_b as i16).abs() as u8;
           max_diff = max_diff.max(diff);
           sum_diff += diff as u32;
       }
       
       let avg_diff = sum_diff as f32 / a.len() as f32;
       (max_diff, avg_diff)
   }
   ```

2. **Create golden image storage** (30 min)
   ```bash
   mkdir -p astraweave-render/tests/visual_regression/golden_images
   ```

3. **Add image comparison dependency** (15 min)
   ```toml
   # astraweave-render/Cargo.toml [dev-dependencies]
   image = "0.25"
   png = "0.18"
   ```

##### Day 2: Golden Image Tests (8 hours)

4. **Test 1: PBR Material Sphere**
   ```rust
   #[test]
   fn test_pbr_metal_rough_sphere() {
       let ctx = pollster::block_on(VisualTestContext::new(512, 512));
       
       let pixels = ctx.render_to_buffer(|device, queue, target| {
           render_pbr_sphere(
               device,
               queue,
               target,
               PbrParams {
                   albedo: [1.0, 0.0, 0.0],  // Red
                   metallic: 1.0,
                   roughness: 0.2,
               },
           );
       });
       
       ctx.assert_image_matches(
           &pixels,
           "golden_images/pbr_metal_sphere.png",
           5  // tolerance
       );
   }
   ```

5. **Test 2-10: Additional golden tests** (6 hours)
   - PBR roughness sweep (rough to smooth)
   - Terrain blending (4 materials)
   - Shadow quality (CSM cascades)
   - Normal mapping
   - IBL lighting
   - Post-processing (bloom)
   - Skybox rendering
   - Animation pose
   - Transparency/alpha cutout
   - Color space accuracy

##### Day 3: Integration & Documentation (4 hours)

6. **CI Integration** (2 hours)
   ```yaml
   # .github/workflows/pbr-pipeline-ci.yml
   
   - name: Visual Regression Tests
     run: |
       cargo test --package astraweave-render \
         --test visual_regression \
         -- --nocapture
       
   - name: Upload Visual Diffs (on failure)
     if: failure()
     uses: actions/upload-artifact@v3
     with:
       name: visual-regression-diffs
       path: astraweave-render/tests/visual_regression/*.png
   ```

7. **Documentation** (2 hours)
   - Write guide for adding new visual tests
   - Document how to update golden images
   - Create troubleshooting guide

#### Success Criteria
- ✅ 10+ visual regression tests
- ✅ Golden images stored in repo
- ✅ Tests run in CI headless
- ✅ Diff images generated on failure
- ✅ Documentation complete

---

### Task 3.2: Shader Compilation Validation
**Priority:** P0  
**Duration:** 1 day  
**Files:** New test file + CI updates

#### Implementation Steps

1. **Create shader validation test** (3 hours)
   ```rust
   // File: astraweave-render/tests/shader_validation.rs
   
   use naga::front::wgsl;
   use std::path::PathBuf;
   
   fn get_all_shaders() -> Vec<PathBuf> {
       glob::glob("astraweave-render/shaders/**/*.wgsl")
           .unwrap()
           .chain(glob::glob("astraweave-render/src/shaders/**/*.wgsl").unwrap())
           .chain(glob::glob("astraweave-render-bevy/shaders/**/*.wgsl").unwrap())
           .chain(glob::glob("tools/aw_editor/src/viewport/shaders/**/*.wgsl").unwrap())
           .chain(glob::glob("examples/unified_showcase/src/**/*.wgsl").unwrap())
           .filter_map(Result::ok)
           .collect()
   }
   
   #[test]
   fn test_all_shaders_compile() {
       let shaders = get_all_shaders();
       assert!(!shaders.is_empty(), "No shaders found");
       
       let mut failures = Vec::new();
       
       for shader_path in shaders {
           let source = std::fs::read_to_string(&shader_path)
               .expect("Failed to read shader");
           
           match wgsl::parse_str(&source) {
               Ok(module) => {
                   // Validate module
                   let mut validator = naga::valid::Validator::new(
                       naga::valid::ValidationFlags::all(),
                       naga::valid::Capabilities::all(),
                   );
                   
                   if let Err(e) = validator.validate(&module) {
                       failures.push(format!(
                           "{}: Validation error: {}",
                           shader_path.display(),
                           e
                       ));
                   }
               }
               Err(e) => {
                   failures.push(format!(
                       "{}: Parse error: {}",
                       shader_path.display(),
                       e
                   ));
               }
           }
       }
       
       if !failures.is_empty() {
           panic!(
               "Shader compilation failures:\n{}",
               failures.join("\n")
           );
       }
       
       println!("✅ All {} shaders validated successfully", shaders.len());
   }
   
   #[test]
   fn test_shader_features_compatibility() {
       // Verify shaders don't use features unavailable on target platforms
       // e.g., no fragment shader outputs without location
   }
   ```

2. **Add to CI** (1 hour)
   ```yaml
   # .github/workflows/pbr-pipeline-ci.yml
   
   validate-shaders:
     name: Validate WGSL Shaders
     runs-on: ubuntu-latest
     steps:
       - uses: actions/checkout@v4
       - uses: dtolnay/rust-toolchain@stable
       
       - name: Run shader validation
         run: |
           cargo test --package astraweave-render \
             --test shader_validation \
             -- --nocapture
   ```

3. **Documentation** (30 min)
   - Document shader authoring guidelines
   - Add validation instructions to CONTRIBUTING.md

#### Success Criteria
- ✅ All shaders validate successfully
- ✅ CI catches shader errors before merge
- ✅ Clear error messages on validation failure
- ✅ Documentation updated

---

### Task 3.3: GPU Resource Leak Detection
**Priority:** P0  
**Duration:** 2 days  
**Files:** New test infrastructure

#### Implementation Steps

##### Day 1: Detection Framework (4 hours)

1. **Create resource tracking utility** (3 hours)
   ```rust
   // File: astraweave-render/tests/resource_leak_detection.rs
   
   use std::sync::atomic::{AtomicUsize, Ordering};
   use std::sync::Arc;
   
   struct ResourceTracker {
       buffers: AtomicUsize,
       textures: AtomicUsize,
       bind_groups: AtomicUsize,
       pipelines: AtomicUsize,
   }
   
   impl ResourceTracker {
       fn new() -> Self {
           Self {
               buffers: AtomicUsize::new(0),
               textures: AtomicUsize::new(0),
               bind_groups: AtomicUsize::new(0),
               pipelines: AtomicUsize::new(0),
           }
       }
       
       fn track_buffer(&self) { self.buffers.fetch_add(1, Ordering::Relaxed); }
       fn untrack_buffer(&self) { self.buffers.fetch_sub(1, Ordering::Relaxed); }
       
       fn snapshot(&self) -> ResourceSnapshot {
           ResourceSnapshot {
               buffers: self.buffers.load(Ordering::Relaxed),
               textures: self.textures.load(Ordering::Relaxed),
               bind_groups: self.bind_groups.load(Ordering::Relaxed),
               pipelines: self.pipelines.load(Ordering::Relaxed),
           }
       }
   }
   
   #[derive(Debug, Clone, PartialEq)]
   struct ResourceSnapshot {
       buffers: usize,
       textures: usize,
       bind_groups: usize,
       pipelines: usize,
   }
   
   impl ResourceSnapshot {
       fn assert_no_leaks(&self, after: &ResourceSnapshot, label: &str) {
           if self != after {
               panic!(
                   "Resource leak detected in {}:\n\
                    Before: {:?}\n\
                    After:  {:?}\n\
                    Leaked: buffers={}, textures={}, bind_groups={}, pipelines={}",
                   label,
                   self,
                   after,
                   after.buffers.saturating_sub(self.buffers),
                   after.textures.saturating_sub(self.textures),
                   after.bind_groups.saturating_sub(self.bind_groups),
                   after.pipelines.saturating_sub(self.pipelines),
               );
           }
       }
   }
   ```

2. **Documentation** (1 hour)
   - Write guide for using resource tracker
   - Document common leak patterns

##### Day 2: Leak Tests (4 hours)

3. **Test: Renderer lifecycle**
   ```rust
   #[test]
   fn test_renderer_no_resource_leaks() {
       let tracker = Arc::new(ResourceTracker::new());
       let (device, queue) = pollster::block_on(create_headless_device());
       
       let before = tracker.snapshot();
       
       // Create and destroy renderer 10 times
       for _ in 0..10 {
           let mut renderer = Renderer::new(&device, &queue, 800, 600);
           
           // Render a few frames
           for _ in 0..5 {
               renderer.render_frame();
           }
           
           drop(renderer);
           device.poll(wgpu::Maintain::Wait);  // Force cleanup
       }
       
       let after = tracker.snapshot();
       before.assert_no_leaks(&after, "Renderer lifecycle");
   }
   ```

4. **Test: Texture loading**
   ```rust
   #[test]
   fn test_texture_loading_no_leaks() {
       let (device, queue) = pollster::block_on(create_headless_device());
       let tracker = Arc::new(ResourceTracker::new());
       
       let before = tracker.snapshot();
       
       // Load and unload textures repeatedly
       for _ in 0..100 {
           let texture = load_texture(&device, &queue, "test_texture.png");
           drop(texture);
       }
       
       device.poll(wgpu::Maintain::Wait);
       let after = tracker.snapshot();
       before.assert_no_leaks(&after, "Texture loading");
   }
   ```

5. **Test: Resize operations**
   ```rust
   #[test]
   fn test_resize_no_leaks() {
       // Verify depth texture recreation doesn't leak
       // (Should pass after Task 1.1 fix)
   }
   ```

#### Success Criteria
- ✅ Resource tracking framework working
- ✅ 5+ leak detection tests
- ✅ All tests pass (no leaks detected)
- ✅ CI integration complete

---

### Task 3.4: Performance Regression Detection
**Priority:** P1  
**Duration:** 1.5 days  
**Files:** Benchmark infrastructure + CI

#### Implementation Steps

1. **Create frame-time benchmark** (4 hours)
   ```rust
   // File: astraweave-render/benches/frame_time_budget.rs
   
   use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
   use std::time::Duration;
   
   fn bench_full_frame(c: &mut Criterion) {
       let mut group = c.benchmark_group("frame_time");
       group.measurement_time(Duration::from_secs(10));
       
       // Budget: 16.67ms for 60 FPS
       group.bench_function("full_frame", |b| {
           let mut renderer = create_complex_scene();
           b.iter(|| {
               black_box(renderer.render_frame());
           });
       });
       
       // Verify budget
       let result = group.bench_function("full_frame_budget", |b| {
           // ...
       });
       
       // Assert frame time < 16.67ms
       group.finish();
   }
   
   fn bench_shadow_pass(c: &mut Criterion) {
       // Budget: 2ms for shadow rendering
   }
   
   fn bench_post_processing(c: &mut Criterion) {
       // Budget: 2ms for bloom + tonemapping
   }
   
   criterion_group!(benches, bench_full_frame, bench_shadow_pass, bench_post_processing);
   criterion_main!(benches);
   ```

2. **GPU timestamp queries** (3 hours)
   ```rust
   // Add GPU timing infrastructure
   
   struct GpuTimer {
       query_set: wgpu::QuerySet,
       resolve_buffer: wgpu::Buffer,
       read_buffer: wgpu::Buffer,
   }
   
   impl GpuTimer {
       fn begin_pass(&self, encoder: &mut wgpu::CommandEncoder, label: &str) {
           // Write timestamp
       }
       
       fn end_pass(&self, encoder: &mut wgpu::CommandEncoder) {
           // Write timestamp and resolve
       }
       
       fn read_elapsed_ms(&self) -> f32 {
           // Read back and compute elapsed time
       }
   }
   ```

3. **CI integration** (2 hours)
   ```yaml
   # .github/workflows/ci.yml
   
   performance-benchmarks:
     name: Performance Regression Check
     runs-on: ubuntu-latest
     steps:
       - uses: actions/checkout@v4
       
       - name: Run benchmarks
         run: |
           cargo bench --package astraweave-render \
             --bench frame_time_budget \
             -- --save-baseline main
       
       - name: Compare with baseline
         run: |
           cargo bench --package astraweave-render \
             --bench frame_time_budget \
             -- --baseline main
   ```

#### Success Criteria
- ✅ Frame-time benchmarks implemented
- ✅ GPU timestamp queries working
- ✅ Performance budgets defined
- ✅ CI detects regressions

---

### Phase 3 Deliverables
- ✅ Visual regression test suite (10+ golden images)
- ✅ Shader validation in CI
- ✅ GPU resource leak detection
- ✅ Performance regression detection
- ✅ Comprehensive test documentation

---

## Phase 4: Coverage Improvements & Medium Priority (Days 12-16)

### Objective
Address remaining medium-priority issues and push test coverage to 75%.

---

### Task 4.1: Fix Atlas Normal/Roughness Fallback
**Duration:** 4 hours

Implement per-material normal/roughness atlasing or bindings.

---

### Task 4.2: Implement Skybox HDRI Switching
**Duration:** 3 hours

Add resource recreation on F1-F3 key presses.

---

### Task 4.3: Verify Uniform Buffer Alignment
**Duration:** 2 hours

Use bytemuck or explicit alignment verification.

---

### Task 4.4: Add Transparency Support
**Duration:** 6 hours

Create alpha-tested/blended pipelines for foliage and glass.

---

### Task 4.5: Coverage Push to 75%
**Duration:** 2 days

Add tests for uncovered modules:
- IBL module (+5-10%)
- Shadow CSM module (+5-8%)
- Nanite module (+3-5%)

---

### Phase 4 Deliverables
- ✅ All medium-priority issues resolved
- ✅ Test coverage ≥ 75%
- ✅ 20+ new tests added
- ✅ Full system integration tested

---

## Testing Strategy Summary

### Test Pyramid

```
        /\
       /  \      E2E (5%)        - Full application runs
      /----\     
     /      \    Integration (25%) - Multi-component interactions
    /--------\   
   /          \  Unit (70%)       - Individual functions/modules
  /____________\ 
```

### Test Categories

| Category | Count Target | Current | Gap | Priority |
|----------|--------------|---------|-----|----------|
| **Unit Tests** | 400 | 323 | 77 | Medium |
| **Integration Tests** | 25 | 15 | 10 | High |
| **Visual Regression** | 15 | 0 | 15 | **Critical** |
| **Performance Tests** | 8 | 3 | 5 | Medium |
| **Leak Detection** | 5 | 1 | 4 | High |
| **Shader Validation** | 1 | 0 | 1 | **Critical** |
| **TOTAL** | **454** | **342** | **112** | - |

---

## Success Metrics

### Phase Completion Criteria

#### Phase 1 (Critical Fixes)
- ✅ All 4 critical bugs fixed
- ✅ 4 new automated tests passing
- ✅ Manual validation complete
- ✅ Zero regression in existing tests

#### Phase 2 (High Priority)
- ✅ Performance improved by 30-50% (culling)
- ✅ Zero crashes on window minimize/restore
- ✅ Terrain materials with full PBR data
- ✅ Zero aliasing on distant terrain

#### Phase 3 (Testing Infrastructure)
- ✅ 15+ visual regression tests
- ✅ All shaders validate in CI
- ✅ Zero GPU resource leaks detected
- ✅ Performance budgets enforced

#### Phase 4 (Polish)
- ✅ Test coverage ≥ 75%
- ✅ All medium-priority issues resolved
- ✅ Documentation complete

---

## Risk Management

### High Risks

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| **Visual tests flaky in CI** | Medium | High | Use tolerance thresholds, save diffs on failure |
| **Performance regressions not caught** | Low | High | Multiple benchmark runs, statistical analysis |
| **Shader validation too strict** | Low | Medium | Incremental rollout, feature flags |
| **Texture array memory budget** | Medium | Medium | Profile memory usage, add limits |

### Medium Risks

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| **Winding order issues** | Low | Medium | Comprehensive mesh validation |
| **Platform-specific color issues** | Medium | Low | Test on all platforms |
| **Mipmap generation performance** | Low | Low | Async generation, caching |

---

## Timeline & Milestones

### Week 1 (Days 1-5)
- **Mon-Wed:** Phase 1 (Critical fixes)
- **Thu-Fri:** Phase 2 (High priority) - partial

**Milestone 1:** All critical bugs fixed, basic stability achieved

### Week 2 (Days 6-10)
- **Mon:** Phase 2 completion
- **Tue-Fri:** Phase 3 (Testing infrastructure)

**Milestone 2:** Comprehensive testing infrastructure operational

### Week 3 (Days 11-16)
- **Mon-Wed:** Phase 3 completion
- **Thu-Sat:** Phase 4 (Coverage & polish)

**Milestone 3:** 75% coverage, all issues resolved

---

## Deliverables Checklist

### Code Deliverables
- [ ] 12 bug fixes (4 critical, 4 high, 4 medium)
- [ ] 112 new tests (unit, integration, visual, performance)
- [ ] Visual regression framework
- [ ] Shader validation framework
- [ ] GPU resource leak detection
- [ ] Performance benchmarking suite

### Documentation Deliverables
- [ ] Updated RENDERING_SYSTEM_ANALYSIS.md
- [ ] Test authoring guidelines
- [ ] Visual regression test guide
- [ ] Performance benchmarking guide
- [ ] Resource leak detection guide
- [ ] CHANGELOG.md updates
- [ ] API documentation updates

### Quality Deliverables
- [ ] 75%+ test coverage
- [ ] Zero critical bugs
- [ ] Zero GPU resource leaks
- [ ] Performance budgets met
- [ ] All CI checks passing

---

## Post-Implementation

### Ongoing Maintenance
1. **Weekly:** Review test results, update golden images if needed
2. **Per PR:** Run full test suite, review coverage diff
3. **Per release:** Full visual regression audit
4. **Monthly:** Performance benchmark review

### Continuous Improvement
1. Add more golden image tests as features are added
2. Expand performance benchmarks
3. Improve test execution speed
4. Add more platforms to CI matrix

---

## Appendix: Test File Structure

```
astraweave-render/
├── tests/
│   ├── visual_regression/
│   │   ├── mod.rs                    # Test framework
│   │   ├── test_pbr_materials.rs
│   │   ├── test_terrain_rendering.rs
│   │   ├── test_shadows.rs
│   │   ├── test_post_processing.rs
│   │   └── golden_images/            # Reference images
│   │       ├── pbr_metal_sphere.png
│   │       ├── terrain_blend.png
│   │       └── ...
│   ├── shader_validation.rs          # Shader compilation tests
│   ├── resource_leak_detection.rs    # GPU leak tests
│   ├── test_utils.rs                 # Shared utilities
│   ├── test_pbr_brdf.rs             # Existing
│   └── ...
├── benches/
│   ├── frame_time_budget.rs         # Performance benchmarks
│   ├── mesh_optimization.rs         # Existing
│   └── ...

examples/unified_showcase/
├── tests/
│   ├── resize_depth_texture.rs
│   ├── terrain_tiling.rs
│   ├── color_space_srgb.rs
│   ├── surface_resilience.rs
│   └── ...
```

---

**Plan Status:** Ready for Implementation  
**Next Action:** Begin Phase 1, Task 1.1 (Depth Texture Resize Fix)  
**Estimated Completion:** 2025-11-30
