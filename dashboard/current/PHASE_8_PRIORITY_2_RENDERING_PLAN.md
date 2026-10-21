# Phase 8.2: Complete Rendering Pipeline Implementation Plan

**Document Version**: 1.0  
**Date**: October 14, 2025  
**Duration**: 4-5 weeks  
**Dependencies**: None (can run in parallel with Phase 8.1)

---

## Executive Summary

**Mission**: Transform AstraWeave's rendering from "prototype visuals" to "production-quality graphics" by completing shadow mapping, post-processing, skybox, particle system, and volumetric effects.

**Current State** (From Roadmap Review):
- ✅ **Shadow Mapping**: CSM infrastructure EXISTS (shadow_tex, shadow_pipeline, cascade matrices)
- ✅ **Post-Processing**: Pipeline EXISTS with feature flag (#[cfg(feature = "postfx")])
- ✅ **PBR Materials**: Complete with IBL, BC7/BC5 textures, GPU skinning
- ✅ **HDR Rendering**: Rgba16Float textures, hdr_view, hdr_sampler
- ⚠️ **Skybox**: Module EXISTS but needs atmosphere scattering
- ❌ **Particle System**: Not started
- ❌ **Volumetric Effects**: Not started

**Target State** (Phase 8 Complete):
- ✅ Shadow mapping enabled and validated in production
- ✅ Post-processing with bloom, tonemapping (ACES), SSAO
- ✅ Skybox with atmospheric scattering and day/night cycle
- ✅ Dynamic point/spot lights with omnidirectional shadows
- ✅ GPU-accelerated particle system with instancing
- ✅ Volumetric fog and god rays (optional, time permitting)

**Timeline**: 4-5 weeks (20-25 working days)

**Success Criteria**: Veilweaver Demo Level has AAA-quality visuals matching modern indie games

---

## Week 1: Validate & Complete Existing Shadow Maps

**Goal**: Enable CSM shadow mapping in production and integrate with main rendering pipeline

### Day 1-2: Shadow Map Infrastructure Validation

**Tasks**:
1. **Audit Existing Code**:
   - File: `astraweave-render/src/renderer.rs`
   - Lines: 1266+ (shadow_tex, shadow_pipeline, cascade matrices)
   - Verify: CSM implementation, 2-layer array texture, 1024x1024 resolution
   - Document: API surface, feature flags, known limitations

2. **Enable Shadow Feature**:
   - Check: Is shadow mapping gated by feature flag?
   - If yes: Add `shadows` feature to workspace Cargo.toml
   - If no: Verify why not enabled by default
   - Test: `cargo check -p astraweave-render --features shadows`

3. **Integration Testing**:
   - Create test scene: 1 directional light + 10 cubes at varying depths
   - Render with shadows enabled
   - Validate: Shadow cascades visible, no Peter Panning, smooth transitions
   - Debug: Use RenderDoc/Tracy to inspect shadow map contents

**Deliverables**:
- Shadow mapping feature flag enabled (if applicable)
- Test scene with validated shadow quality
- Documentation: `SHADOW_MAPPING_API.md` with usage examples

**Success Criteria**:
- ✅ CSM renders correctly with 2-4 cascades
- ✅ No visual artifacts (Peter Panning, shadow acne, cascade seams)
- ✅ Performance: <0.5ms per cascade @ 1024x1024 (2ms total for 4 cascades)

---

### Day 3-4: Shadow Quality Improvements

**Tasks**:
1. **PCF (Percentage Closer Filtering)**:
   - File: `astraweave-render/src/shaders/shadow.wgsl` (or equivalent)
   - Implement: 5x5 Poisson disk sampling for soft shadows
   - Optimize: Use textureSampleCompareLevel for hardware PCF
   - Validate: Shadows have soft penumbra, no aliasing

2. **Cascade Selection Optimization**:
   - Implement: Smooth cascade blending (fade between cascades)
   - Optimize: Per-fragment cascade selection (not per-object)
   - Validate: No visible cascade boundaries
   - Performance: <0.1ms overhead for blending

3. **Shadow Bias Tuning**:
   - Fix: Shadow acne (bias too low) vs Peter Panning (bias too high)
   - Implement: Normal offset bias + constant bias
   - Tune: Per-cascade bias values (closer cascades need less bias)
   - Validate: No self-shadowing artifacts, minimal Peter Panning

**Deliverables**:
- PCF shadow filtering implemented
- Smooth cascade transitions
- Tuned bias parameters

**Success Criteria**:
- ✅ Soft shadows with realistic penumbra
- ✅ No visible cascade boundaries
- ✅ No shadow acne or Peter Panning on flat/curved surfaces

---

### Day 5: Documentation & Example Integration

**Tasks**:
1. **API Documentation**:
   - Document: `ShadowMapConfig` struct (cascade count, resolution, bias)
   - Document: How to enable shadows per-light
   - Document: Performance tuning guidelines
   - Examples: Code snippets for common use cases

2. **Integrate with `unified_showcase`**:
   - File: `examples/unified_showcase/src/main.rs`
   - Add: Directional light with shadows
   - Validate: Biome scenes render with realistic shadows
   - Screenshot: Save for documentation

3. **Integrate with `hello_companion`**:
   - File: `examples/hello_companion/src/main.rs`
   - Add: Shadows for companion AI demo
   - Validate: Dynamic objects cast shadows correctly
   - Performance: Measure frame time impact

**Deliverables**:
- `SHADOW_MAPPING_API.md` documentation
- `unified_showcase` with shadow demo
- `hello_companion` with shadow integration

**Success Criteria**:
- ✅ Examples compile and run with shadows enabled
- ✅ Visual quality matches modern game standards
- ✅ Frame time impact <3ms for typical scenes

---

## Week 2: Complete Post-Processing Pipeline

**Goal**: Enable and polish post-processing effects (bloom, tonemapping, SSAO)

### Day 6-7: Enable Existing Post-FX Pipeline

**Tasks**:
1. **Audit Existing Code**:
   - File: `astraweave-render/src/renderer.rs` (post_pipeline, post_fx_shader)
   - Lines: 662+ (tonemapping ACES-like, bloom structure)
   - Verify: HDR → LDR pipeline, feature flag status
   - Document: What's implemented vs stubbed out

2. **Enable Post-FX Feature**:
   - Feature flag: `#[cfg(feature = "postfx")]`
   - Add to workspace: `postfx` feature in astraweave-render
   - Test: `cargo check -p astraweave-render --features postfx`
   - Validate: Compiles without errors

3. **Tonemapping Implementation**:
   - Shader: Verify ACES tonemapping curve (industry standard)
   - Alternative: Implement Reinhard, Uncharted 2, Filmic options
   - Exposure: Add auto-exposure or manual exposure control
   - Validate: HDR scenes map correctly to LDR (no clipping)

**Deliverables**:
- Post-FX feature enabled
- Tonemapping validated (ACES curve)
- Exposure control implemented

**Success Criteria**:
- ✅ HDR scenes render with realistic brightness
- ✅ No white clipping in bright areas
- ✅ No black crush in dark areas

---

### Day 8-9: Bloom Implementation

**Tasks**:
1. **Bloom Downsampling**:
   - Implement: 5-pass downsample (1/2, 1/4, 1/8, 1/16, 1/32 resolution)
   - Filter: Karis average for HDR firefly reduction
   - Optimize: Use bilinear filtering for free 2x2 box blur
   - Validate: Bright areas create smooth glow

2. **Bloom Upsampling**:
   - Implement: 5-pass upsample with tent filter (9-tap)
   - Blending: Additive blend each level (for smooth falloff)
   - Optimize: Use half-resolution intermediate buffers
   - Validate: Bloom has realistic radius and intensity

3. **Bloom Threshold & Intensity**:
   - Threshold: Extract pixels >1.0 luminance (HDR only)
   - Intensity: Configurable multiplier (0.1-0.3 typical)
   - Dirt mask: Optional lens dirt texture (AAA polish)
   - Validate: Bloom enhances bright lights, doesn't wash out scene

**Deliverables**:
- 5-pass bloom downsample/upsample
- Configurable bloom threshold and intensity
- Validated visual quality

**Success Criteria**:
- ✅ Bright lights have realistic glow
- ✅ No over-bloom (scene not washed out)
- ✅ Performance: <2ms @ 1080p for full bloom pipeline

---

### Day 10: SSAO (Screen-Space Ambient Occlusion)

**Tasks**:
1. **SSAO Implementation** (If time permits, else defer to Phase 9):
   - Algorithm: Scalable SSAO (GTAO preferred, fallback to basic SSAO)
   - Kernel: 16-32 samples in hemisphere
   - Noise: 4x4 rotation texture for sample distribution
   - Blur: 7x7 bilateral blur for noise removal
   - Validate: Contact shadows in corners, crevices

2. **Integration with Lighting**:
   - Apply: Multiply ambient term by SSAO factor
   - Intensity: Configurable (0.5-1.0 typical)
   - Performance: <2ms @ 1080p (defer if >3ms)

**Deliverables**:
- SSAO implementation (basic or GTAO)
- Integrated with ambient lighting

**Success Criteria** (Optional, defer if time-constrained):
- ✅ Realistic contact shadows in corners
- ✅ Performance: <2ms @ 1080p
- ⚠️ If >3ms, defer to Phase 9 and use baked AO instead

---

## Week 3: Skybox & Atmospheric Scattering

**Goal**: Implement realistic sky rendering with day/night cycle

### Day 11-12: Skybox Rendering

**Tasks**:
1. **Cubemap Skybox**:
   - Asset: 6-face cubemap or equirectangular HDR panorama
   - Shader: Sample cubemap based on view direction
   - Rendering: Full-screen quad or inverted cube (optimize with early-Z)
   - Validate: Sky renders behind all geometry

2. **HDR Sky Integration**:
   - IBL: Use same HDR sky for skybox and IBL (consistency)
   - Exposure: Apply tonemapping to skybox (match scene)
   - Sun disk: Render procedural sun at light direction
   - Validate: Sky brightness matches IBL contribution

3. **Performance Optimization**:
   - Early-Z: Render skybox after opaque geometry (depth test only)
   - Mip-mapping: Use mip levels for blurred horizon
   - LOD: Lower resolution for distant sky (not visible in detail)

**Deliverables**:
- Cubemap skybox rendering
- HDR sky integrated with IBL
- Procedural sun disk

**Success Criteria**:
- ✅ Realistic sky rendering with HDR
- ✅ Sun disk matches directional light position
- ✅ Performance: <0.5ms for skybox rendering

---

### Day 13-14: Atmospheric Scattering (Simplified)

**Tasks**:
1. **Rayleigh Scattering** (Sky Color):
   - Algorithm: Simplified Nishita model (not full Bruneton)
   - Implementation: Precompute scattering LUT (512x128 texture)
   - Inputs: Sun angle, view direction, atmosphere density
   - Validate: Blue sky at zenith, orange/red at horizon

2. **Mie Scattering** (Sun Glow):
   - Algorithm: Henyey-Greenstein phase function
   - Implementation: Add sun glow around sun disk
   - Intensity: Stronger at sunrise/sunset
   - Validate: Realistic sun halo

3. **Day/Night Cycle**:
   - Parameter: Time of day (0-24 hours)
   - Sun position: Compute from time (simple trigonometry)
   - Sky color: Interpolate scattering LUT based on sun angle
   - Validate: Smooth transition from day → dusk → night → dawn

**Deliverables**:
- Atmospheric scattering LUT
- Day/night cycle system
- Integrated with skybox and lighting

**Success Criteria**:
- ✅ Realistic blue sky during day, orange/red at sunset
- ✅ Smooth day/night transitions
- ✅ Performance: <1ms for scattering evaluation (LUT lookup)

---

### Day 15: Night Sky & Stars

**Tasks**:
1. **Starfield Rendering**:
   - Technique: Procedural stars or texture-based
   - Visibility: Fade in at night (based on sun angle)
   - Twinkle: Optional noise animation for realism
   - Validate: Stars visible at night, invisible during day

2. **Moon Rendering** (Optional):
   - Model: Sphere or billboard
   - Texture: Moon albedo texture with phases
   - Position: Opposite sun (simplified, not astronomical)
   - Validate: Moon provides subtle ambient light at night

**Deliverables**:
- Starfield rendering
- Optional moon rendering

**Success Criteria**:
- ✅ Stars visible at night, blend smoothly at dusk/dawn
- ✅ Optional: Moon visible with correct phase

---

## Week 4: Dynamic Point/Spot Lights

**Goal**: Extend shadow mapping to point and spot lights

### Day 16-17: Point Light Shadows (Omnidirectional)

**Tasks**:
1. **Cubemap Shadow Maps**:
   - Technique: Render 6 faces for each point light (expensive!)
   - Optimization: Use geometry shader to render all 6 faces in 1 pass
   - Resolution: 512x512 per face (3MB per light, limit to 4-8 lights)
   - Validate: Point lights cast shadows in all directions

2. **Dual-Paraboloid Alternative** (If cubemap too slow):
   - Technique: Render 2 hemispheres instead of 6 faces
   - Performance: 3× faster than cubemap
   - Quality: Slight distortion at equator (acceptable for games)
   - Validate: Shadows render with minimal artifacts

3. **Shadow Filtering**:
   - PCF: Apply 5x5 Poisson disk sampling (same as CSM)
   - Distance: Use linear depth for comparison
   - Bias: Tune per-light (larger bias for larger radius)

**Deliverables**:
- Omnidirectional shadow maps for point lights
- Optimized rendering (geometry shader or dual-paraboloid)

**Success Criteria**:
- ✅ Point lights cast realistic shadows in all directions
- ✅ Performance: <3ms per point light (limit to 4-8 lights)
- ✅ Visual quality acceptable (soft shadows, no major artifacts)

---

### Day 18-19: Spot Light Shadows

**Tasks**:
1. **Spot Light Shadow Maps**:
   - Technique: Standard perspective shadow map (simpler than point lights)
   - Atlas: Pack multiple spot lights into single 2048x2048 atlas
   - Culling: Only render shadows for visible spot lights
   - Validate: Spot lights cast cone-shaped shadows

2. **Cone Attenuation**:
   - Inner/Outer angle: Define cone shape (cos(angle) comparison)
   - Smooth falloff: Interpolate between inner and outer cone
   - Shadow masking: No shadows outside cone
   - Validate: Realistic flashlight/spotlight behavior

3. **Performance Optimization**:
   - Shadow atlas: 4-8 spot lights per 2048x2048 texture
   - Frustum culling: Skip shadow rendering for off-screen lights
   - LOD: Reduce shadow resolution for distant lights

**Deliverables**:
- Spot light shadow maps with atlas packing
- Cone attenuation with smooth falloff

**Success Criteria**:
- ✅ Spot lights cast realistic cone-shaped shadows
- ✅ Performance: <1ms per spot light (up to 8 lights)
- ✅ Smooth falloff at cone edges

---

### Day 20: Light Culling & Optimization

**Tasks**:
1. **Tiled/Clustered Forward Rendering** (Optional, defer if complex):
   - Technique: Divide screen into tiles, cull lights per-tile
   - Implementation: Compute shader to build light lists
   - Performance: O(lights × tiles) instead of O(lights × pixels)
   - Fallback: Simple distance culling (good enough for 8-16 lights)

2. **Light Limit Enforcement**:
   - Max lights: 16 point + 8 spot + 1 directional (reasonable for forward rendering)
   - Sorting: Prioritize by distance, intensity (render brightest first)
   - Fallback: Disable shadows for distant lights (still render lighting)

**Deliverables**:
- Light culling system (tiled or distance-based)
- Light limit enforcement

**Success Criteria**:
- ✅ 16+ point lights render at 60 FPS
- ✅ Distant lights disabled gracefully
- ✅ No visual popping when lights cull/uncull

---

## Week 5: Particle System (GPU-Accelerated)

**Goal**: Implement production-quality particle system for effects (fire, smoke, magic)

### Day 21-22: Particle Emitter & Simulation

**Tasks**:
1. **Particle Data Structure**:
   - Per-particle: Position (vec3), velocity (vec3), lifetime (f32), size (f32), color (vec4)
   - Buffer: GPU buffer with 10,000+ particles (400KB+)
   - Allocator: Free-list allocator for particle spawn/death
   - Validate: Particles spawn and die correctly

2. **GPU Compute Shader Simulation**:
   - Update: Position += velocity × dt
   - Forces: Gravity, wind, drag
   - Lifetime: Decrement each frame, recycle when dead
   - Performance: <1ms for 10,000 particles

3. **Emitter Shapes**:
   - Point: Spawn at single point
   - Sphere: Spawn on sphere surface
   - Cone: Spawn in cone direction (useful for flames)
   - Box: Spawn in box volume

**Deliverables**:
- GPU particle simulation system
- 4 emitter shapes (point, sphere, cone, box)

**Success Criteria**:
- ✅ 10,000+ particles simulate at 60 FPS
- ✅ Realistic physics (gravity, drag)
- ✅ Particles spawn/die without leaks

---

### Day 23-24: Particle Rendering & Effects

**Tasks**:
1. **Billboard Rendering**:
   - Technique: Quad facing camera (2 triangles per particle)
   - Instancing: Use GPU instancing for 10,000+ particles
   - Sorting: Back-to-front sort for alpha blending (expensive, consider OIT)
   - Validate: Particles render correctly from all angles

2. **Texturing & Animation**:
   - Atlas: 4x4 or 8x8 sprite sheet for flipbook animation
   - UV: Animate UV coordinates based on lifetime
   - Color: Interpolate color over lifetime (fade out)
   - Validate: Smoke/fire has realistic animation

3. **Blending Modes**:
   - Additive: For fire, explosions, magic (no sorting needed!)
   - Alpha blend: For smoke, dust (requires sorting)
   - Multiply: For shadows, fog (rare)
   - Validate: Each mode renders correctly

**Deliverables**:
- Billboard particle rendering with instancing
- Texture atlas animation
- 3 blending modes (additive, alpha, multiply)

**Success Criteria**:
- ✅ 10,000+ particles render at 60 FPS
- ✅ Realistic fire/smoke effects
- ✅ Smooth flipbook animation

---

### Day 25: Particle System Polish & Integration

**Tasks**:
1. **Particle Editor** (Basic):
   - UI: egui panel in `aw_editor` for particle editing
   - Parameters: Lifetime, size, color curve, velocity, forces
   - Preview: Real-time preview of particle effect
   - Save/Load: Serialize to TOML or JSON

2. **Integrate with `unified_showcase`**:
   - Effect: Add fire particles to campfire
   - Effect: Add smoke particles to chimneys
   - Effect: Add magic sparkles (additive blending)
   - Validate: Visual quality matches commercial games

3. **Performance Profiling**:
   - Tracy: Profile particle simulation and rendering
   - Optimize: Reduce overdraw (z-prepass for opaque geometry)
   - Validate: <2ms total for 10,000 particles

**Deliverables**:
- Basic particle editor in `aw_editor`
- Particle effects in `unified_showcase`
- Performance profiling results

**Success Criteria**:
- ✅ Artists can create/edit particle effects
- ✅ Veilweaver Demo Level has polished effects
- ✅ Performance: <2ms for typical scenes (1,000-5,000 particles)

---

## Optional: Week 5+ (Volumetric Effects)

**If Time Permits** (Otherwise defer to Phase 9)

### Volumetric Fog

**Tasks**:
1. **Height Fog**:
   - Technique: Exponential height fog (simple, fast)
   - Parameters: Density, height, color
   - Integration: Apply in post-processing (depth-based)
   - Validate: Realistic fog density increasing with distance

2. **Volumetric Lighting (God Rays)**:
   - Technique: Raymarched lighting in screen space
   - Steps: 32-64 samples along view ray
   - Performance: Expensive (defer if >5ms)
   - Validate: Light shafts visible through fog/smoke

**Deliverables** (Optional):
- Height fog implementation
- Optional: Volumetric lighting

**Success Criteria** (Optional):
- ✅ Realistic fog with depth
- ✅ Optional: God rays visible
- ⚠️ If >5ms, defer to Phase 9

---

## Integration & Testing

### Testing Strategy

**Unit Tests**:
- Shadow map allocation/deallocation
- Particle free-list allocator
- Post-FX pipeline creation

**Integration Tests**:
- Full rendering pipeline (PBR + shadows + post-FX + particles)
- Multi-light scenes (16 point + 8 spot + shadows)
- Particle spawn/death over 10,000 frames (no leaks)

**Visual Validation**:
- RenderDoc captures for each feature
- Side-by-side comparisons with reference images
- Screenshot regression tests (automated comparison)

**Performance Validation**:
- Tracy profiling for all rendering passes
- Frame time budget: <16.67ms total (60 FPS)
- Rendering budget: <8ms (physics, AI, gameplay get other 8ms)

---

## Success Criteria (Phase 8.2 Complete)

### Visual Quality

- ✅ **Shadows**: Soft, realistic shadows with CSM for directional lights, omnidirectional for point lights
- ✅ **Post-Processing**: HDR → LDR with ACES tonemapping, realistic bloom on bright lights
- ✅ **Skybox**: Realistic sky with atmospheric scattering, day/night cycle, stars at night
- ✅ **Lighting**: 16+ point lights + 8 spot lights + 1 directional light at 60 FPS
- ✅ **Particles**: 10,000+ particles with GPU simulation, realistic fire/smoke/magic effects
- ✅ **Optional**: Volumetric fog and god rays (defer if time-constrained)

### Performance

- ✅ **Frame Time**: <8ms for rendering (target 60 FPS with 8ms headroom)
- ✅ **Shadows**: <5ms total (CSM + point lights + spot lights)
- ✅ **Post-FX**: <3ms total (bloom + tonemapping + optional SSAO)
- ✅ **Particles**: <2ms for 1,000-5,000 particles (typical scenes)
- ✅ **Skybox**: <1ms total (scattering + rendering)

### Code Quality

- ✅ **Zero `.unwrap()`**: All rendering code uses proper error handling
- ✅ **Zero `todo!()`**: All advertised features complete
- ✅ **50%+ test coverage**: Unit + integration tests for critical paths
- ✅ **Documentation**: API docs for all public rendering APIs

### Integration

- ✅ **`unified_showcase`**: Demonstrates all rendering features (shadows, particles, sky)
- ✅ **`hello_companion`**: AI demo has production-quality visuals
- ✅ **Veilweaver Demo Level**: Ready for rendering integration (UI + gameplay)

---

## Dependencies & Risks

### Dependencies

**Upstream** (Blocks this work):
- None (can run in parallel with Phase 8.1 UI)

**Downstream** (Blocked by this work):
- Veilweaver Demo Level (needs visuals for marketing)
- Phase 9 optimization (needs rendering baseline)

### Risks

**High Risk**:
1. **Particle Sorting Performance**: Alpha-blended particles require CPU sorting (expensive)
   - Mitigation: Use additive blending (no sorting needed)
   - Fallback: Limit alpha-blended particles to 1,000

2. **Omnidirectional Shadows**: 6 faces per point light is expensive
   - Mitigation: Use dual-paraboloid (3× faster)
   - Fallback: Limit to 4 point lights with shadows

**Medium Risk**:
3. **Atmospheric Scattering Complexity**: Full Bruneton model is PhD-level implementation
   - Mitigation: Use simplified Nishita model (good enough)
   - Fallback: Use pre-baked cubemap sky (no dynamic time of day)

4. **SSAO Performance**: Can easily exceed 5ms @ 1080p
   - Mitigation: Use GTAO (faster) or lower sample count
   - Fallback: Defer to Phase 9, use baked AO for demo

**Low Risk**:
5. **Bloom Overdraw**: High-res bloom can cause overdraw issues
   - Mitigation: Use half-res intermediate buffers
   - Optimization: Early-Z pass for opaque geometry

---

## Deliverables Checklist

### Code

- [ ] Shadow mapping: CSM validation + PCF + cascade blending
- [ ] Post-processing: Bloom + tonemapping (ACES) + optional SSAO
- [ ] Skybox: Cubemap rendering + atmospheric scattering + day/night cycle
- [ ] Dynamic lights: Point lights (omnidirectional shadows) + spot lights (cone shadows)
- [ ] Particle system: GPU simulation + billboard rendering + 3 blending modes
- [ ] Optional: Volumetric fog + god rays (defer if time-constrained)

### Documentation

- [ ] `SHADOW_MAPPING_API.md`: API docs + usage examples
- [ ] `POST_PROCESSING_API.md`: Bloom, tonemapping, SSAO configuration
- [ ] `PARTICLE_SYSTEM_API.md`: Emitter API + custom effects guide
- [ ] `RENDERING_PERFORMANCE.md`: Profiling results + optimization tips

### Examples

- [ ] `unified_showcase`: All rendering features demonstrated
- [ ] `hello_companion`: Production-quality visuals
- [ ] `particle_demo`: Particle system showcase (fire, smoke, magic)

### Tests

- [ ] Unit tests: Shadow allocation, particle allocator, post-FX pipeline
- [ ] Integration tests: Full rendering pipeline, multi-light scenes
- [ ] Visual regression: Screenshot comparisons for each feature
- [ ] Performance tests: Tracy profiling for all rendering passes

---

## Timeline Summary

| Week | Days | Focus | Deliverables |
|------|------|-------|--------------|
| 1 | 1-5 | Shadow Mapping | CSM validation, PCF, cascade blending, docs |
| 2 | 6-10 | Post-Processing | Bloom, tonemapping, optional SSAO |
| 3 | 11-15 | Skybox & Atmosphere | Cubemap, scattering, day/night, stars |
| 4 | 16-20 | Dynamic Lights | Point shadows, spot shadows, light culling |
| 5 | 21-25 | Particle System | GPU simulation, rendering, editor integration |
| 5+ | 26-30 | Optional: Volumetric | Height fog, god rays (defer if needed) |

**Total Duration**: 4-5 weeks (20-30 days)

**Estimated Effort**: 160-200 hours (1 FTE)

---

## Next Steps

1. **Read this plan**: Understand scope, timeline, risks
2. **Create Phase 8.3 & 8.4 plans**: Save/Load and Audio implementation plans
3. **Create Master Integration Plan**: Coordinate all 4 Phase 8 priorities
4. **Begin Phase 8.1**: UI framework (can run in parallel)
5. **Begin Phase 8.2 Week 1**: Shadow mapping validation

---

**Document Status**: Implementation plan ready for execution  
**Last Updated**: October 14, 2025  
**Next Document**: PHASE_8_PRIORITY_3_SAVE_LOAD_PLAN.md
