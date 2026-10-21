# Phase 8-10: Game Engine Readiness — Comprehensive Implementation & Validation Plan

**Document Version**: 1.0  
**Date**: November 9, 2025  
**Scope**: 3-12 months (Phase 8: 3-4.5 months, Phase 9: 2-3 months, Phase 10: 4-6 months OPTIONAL)  
**Objective**: Transform AstraWeave from "production-ready infrastructure" to "world-class, mission-critical game engine"

**Mission Statement**: No guessing, no assuming. Validate everything. Reach for mission-critical standards. Ensure AstraWeave is fully professional-grade, adhering to best practices and industry standards.

---

## Executive Summary

### Current State Assessment (November 9, 2025)

**What We Have** ✅:
- **AI-Native Architecture**: 12,700+ agents @ 60 FPS, 100% deterministic, GOAP+LLM Hybrid Arbiter
- **Advanced Rendering**: PBR with IBL, BC7/BC5 textures, GPU skinning, mesh optimization
  - ✅ **Shadow System EXISTS**: CSM infrastructure, 2-cascade depth array, 3×3 PCF filtering
  - ✅ **Post-Processing EXISTS**: Bloom (5-mip pyramid), tonemapping, SSAO/SSR shader stubs
  - ✅ **Skybox EXISTS**: Cubemap pipeline, inverted cube geometry, view-centered rendering
- **Audio System EXISTS**: AudioEngine with 2 buses (music, SFX), spatial audio (SpatialSink), crossfading
- **Save/Load Foundation EXISTS**: `astraweave-persistence-ecs` with SaveManager, serialization stubs
- **Comprehensive Tooling**: Editor (14 panels), asset CLI, Tracy profiling
- **Week 5 Content**: 351/351 tests passing, 1850× performance targets (99.979% frame budget headroom)

**Critical Gaps** ⚠️:
1. **Phase 8 Rendering**: Shadow/bloom/skybox infrastructure exists but NOT ENABLED in main renderer
2. **Phase 8 Save/Load**: Persistence crate exists but ECS serialization NOT IMPLEMENTED
3. **Phase 8 Audio**: Basic mixer exists but NO dynamic music layers, occlusion, or reverb zones
4. **Phase 9 Build Pipeline**: NO asset packing, installers, platform SDK integration, telemetry
5. **Phase 10 Networking**: NO multiplayer support (optional but high-value)
6. **Foundation Robustness**: 50+ `.unwrap()` calls, 2 `unimplemented!()` in core systems

**Strategic Insight**: The roadmap underestimated existing systems! Shadow maps, post-FX, skybox, audio mixer, and save/load foundations already exist. This accelerates Phase 8 by **3-4 weeks** (from 12-16 weeks to 8-12 weeks).

### Validation-First Philosophy

**Core Principles**:
1. **Test Before Build**: Write tests first, implement to pass tests (TDD)
2. **Evidence-Based Progress**: No "should work", only "proven to work" with test evidence
3. **Incremental Milestones**: Small, measurable steps with acceptance criteria
4. **Regression Prevention**: CI gates enforce quality (no backsliding)
5. **Mission-Critical Standards**: 99.9% uptime, <1% crash rate, deterministic replay

**Validation Pyramid** (Bottom-Up):
```
          /\
         /  \
        /User\          Acceptance Tests (Veilweaver Demo Level)
       /------\
      /Integration\     Full System Tests (UI+Render+Audio+Save)
     /------------\
    / Performance  \    Stress Tests (1000 entities @ 60 FPS)
   /----------------\
  /   Unit Tests     \  Component Tests (API correctness)
 /____________________\
```

### Timeline Overview (8-12 Months)

**Phase 8: Core Game Loop** (8-12 weeks = 2-3 months)
- **Weeks 1-2**: Enable & validate existing rendering (shadows, bloom, skybox) — 2 weeks saved!
- **Weeks 3-4**: Complete rendering (dynamic lights, particles, volumetric fog)
- **Weeks 5-6**: Save/load system (ECS serialization, versioning, corruption recovery)
- **Weeks 7-8**: Production audio (dynamic music, occlusion, reverb zones)
- **Weeks 9-10**: Integration & stress testing (Veilweaver Demo Level)
- **Weeks 11-12**: Polish & acceptance testing (external playtesters)

**Phase 9: Distribution & Polish** (8-12 weeks = 2-3 months)
- **Weeks 13-16**: Build pipeline (asset packing, installers, platform SDKs)
- **Weeks 17-20**: Asset optimization (atlasing, retargeting, LOD, hot-reload)
- **Weeks 21-24**: Telemetry & profiling (production profiler, crash dumps, metrics)

**Phase 10: Multiplayer & Advanced** (16-24 weeks = 4-6 months, OPTIONAL)
- **Weeks 25-32**: Networking (client-server, replication, latency compensation)
- **Weeks 33-38**: Advanced rendering (GI, advanced post-FX, decals, weather)
- **Weeks 39-48**: Advanced AI & console ports (LLM improvements, swarm tactics, consoles)

**Total**: 8-48 weeks (2-12 months), Phase 8-9 are CRITICAL (4-6 months), Phase 10 is OPTIONAL

---

## Phase 8: Core Game Loop Essentials (8-12 Weeks)

### Objective

**Enable shipping complete single-player games** with professional-grade rendering, UI, save/load, and audio.

### Week-by-Week Breakdown

#### Week 1: Enable & Validate Shadow Mapping (Nov 10-16, 2025)

**Current State**:
- ✅ CSM infrastructure exists in `astraweave-render/src/renderer.rs`:
  - `shadow_tex`: `Depth32Float` 2048×2048 2-array texture
  - `shadow_pipeline`: Depth-only render pipeline
  - `cascade0`, `cascade1`: Light-space projection matrices
  - Shader: 2-cascade selection, 3×3 PCF filtering, bias correction
- ❌ **NOT ENABLED**: Shadow passes not called in main render loop

**Goals**:
1. Enable shadow depth passes in main renderer
2. Validate 2-cascade CSM with rotating directional light
3. Test shadow quality (peter-panning, acne, cascade transitions)
4. Benchmark performance (shadow pass time vs 60 FPS budget)

**Implementation Tasks**:
- [ ] **Task 1.1**: Add shadow pass dispatch before main pass
  - Location: `astraweave-render/src/renderer.rs::render()`
  - Code: `self.render_shadow_pass(encoder, &meshes, cascade_idx)`
  - Validation: Tracy profile shows shadow pass <2 ms @ 100 meshes
- [ ] **Task 1.2**: Update light buffer with cascade matrices
  - Location: `astraweave-render/src/renderer.rs::update_lights()`
  - Code: Upload `cascade0`, `cascade1` to GPU buffer
  - Validation: Shader debug visualization shows correct cascade split
- [ ] **Task 1.3**: Enable shadow sampling in fragment shader
  - Location: Shader inline in `renderer.rs`, line 164-194
  - Current: Commented out or disabled
  - Fix: Uncomment/enable `textureSampleCompare()` calls
  - Validation: Shadows visible in rendered scene
- [ ] **Task 1.4**: Add shadow quality settings
  - Location: `astraweave-render/src/config.rs` (create if missing)
  - Options: Resolution (1024/2048/4096), PCF kernel (3×3/5×5/7×7), bias
  - Validation: UI setting changes shadow quality in real-time

**Testing Matrix**:

| Test | Description | Pass Criteria | Status |
|------|-------------|---------------|--------|
| T1.1 | Shadow visibility | Shadows visible under directional light | ⏳ |
| T1.2 | Cascade coverage | Close objects use cascade 0, far use cascade 1 | ⏳ |
| T1.3 | Peter-panning fix | No floating shadows (bias <0.01) | ⏳ |
| T1.4 | Shadow acne fix | No shadow banding (slope bias working) | ⏳ |
| T1.5 | PCF smoothness | Shadow edges smooth (3×3 kernel) | ⏳ |
| T1.6 | Performance | Shadow pass <2 ms @ 100 meshes | ⏳ |
| T1.7 | Cascade transitions | No visible seam at cascade boundary | ⏳ |
| T1.8 | Dynamic lights | Moving light updates shadows correctly | ⏳ |

**Benchmarks**:
- `shadow_pass_100_meshes`: <2 ms (12% of 16.67 ms budget)
- `shadow_pass_1000_meshes`: <8 ms (48% of budget)
- `cascade_selection`: <1 µs per fragment (negligible)

**Acceptance Criteria**:
- ✅ All 8 tests passing
- ✅ Benchmarks within budget
- ✅ Visual validation: No artifacts, smooth shadows, correct coverage
- ✅ Tracy profile: Shadow pass <2 ms @ 100 meshes

**Deliverable**: Working CSM shadow system, validated and benchmarked

---

#### Week 2: Enable & Validate Post-Processing (Nov 17-23, 2025)

**Current State**:
- ✅ Bloom pipeline infrastructure exists:
  - `bloom_threshold_pipeline`, `bloom_downsample_pipeline`, `bloom_upsample_pipeline`, `bloom_composite_pipeline`
  - `bloom_mip_textures`: 5-mip pyramid for downsample/upsample
  - `bloom_threshold_buf`, `bloom_intensity_buf`: Configurable parameters
- ✅ SSAO/SSR shader stubs exist (`WGSL_SSAO`, `WGSL_SSR`)
- ❌ **NOT ENABLED**: Bloom passes not called, tonemapping not applied

**Goals**:
1. Enable bloom extraction, downsample, upsample, composite passes
2. Add ACES tonemapping for HDR → LDR conversion
3. Test bloom quality (glow radius, intensity, threshold)
4. Benchmark performance (post-processing time vs 60 FPS budget)

**Implementation Tasks**:
- [ ] **Task 2.1**: Enable bloom threshold extraction
  - Location: `astraweave-render/src/post.rs` (create if missing)
  - Code: Render HDR scene to `hdr_texture`, extract bright pixels (>threshold)
  - Validation: Bright areas glow, dark areas unchanged
- [ ] **Task 2.2**: Implement 5-mip downsample pass
  - Code: Progressive 2× downscaling to create Gaussian pyramid
  - Validation: Each mip is 50% resolution of previous
- [ ] **Task 2.3**: Implement 5-mip upsample + blend pass
  - Code: Progressive 2× upscaling with additive blending
  - Validation: Bloom radius grows smoothly
- [ ] **Task 2.4**: Implement ACES tonemapping
  - Location: Post-processing shader
  - Code: Apply ACES curve to final composite
  - Validation: No blown-out highlights, correct exposure
- [ ] **Task 2.5**: Add bloom/tonemap settings UI
  - Location: `examples/unified_showcase` settings panel
  - Options: Bloom intensity (0-1), threshold (0-10), exposure (0.5-2.0)
  - Validation: UI sliders update bloom in real-time

**Testing Matrix**:

| Test | Description | Pass Criteria | Status |
|------|-------------|---------------|--------|
| T2.1 | Bloom visibility | Bright objects glow | ⏳ |
| T2.2 | Bloom radius | Glow extends 50-100 pixels | ⏳ |
| T2.3 | Bloom intensity | Adjustable 0-1 range | ⏳ |
| T2.4 | Bloom threshold | Only bright pixels (>1.0) glow | ⏳ |
| T2.5 | Tonemapping | No blown highlights, correct exposure | ⏳ |
| T2.6 | Performance | Post-processing <3 ms @ 1920×1080 | ⏳ |
| T2.7 | Mip chain | 5 mips generated correctly | ⏳ |
| T2.8 | UI responsiveness | Settings update <16 ms | ⏳ |

**Benchmarks**:
- `bloom_threshold_pass`: <0.5 ms
- `bloom_downsample_5_mips`: <1 ms
- `bloom_upsample_5_mips`: <1 ms
- `aces_tonemapping`: <0.5 ms
- **Total**: <3 ms (18% of 16.67 ms budget)

**Acceptance Criteria**:
- ✅ All 8 tests passing
- ✅ Benchmarks within budget
- ✅ Visual validation: Bloom glow, ACES tonemapping, no artifacts
- ✅ Tracy profile: Post-processing <3 ms @ 1920×1080

**Deliverable**: Working bloom + tonemapping, validated and benchmarked

---

#### Week 3: Enable & Validate Skybox (Nov 24-30, 2025)

**Current State**:
- ✅ Skybox pipeline exists in `astraweave-render/src/environment.rs`:
  - `skybox_pipeline`: Render pipeline with no culling, no depth write
  - `skybox_vertices`, `skybox_indices`: Inverted cube geometry (8 verts, 36 indices)
  - Shader: View-centered rendering (removes translation from view matrix)
- ❌ **NOT ENABLED**: Skybox pass not called, cubemap texture not loaded

**Goals**:
1. Enable skybox rendering with cubemap texture
2. Add day/night cycle (lerp between day/night cubemaps)
3. Test skybox quality (seams, orientation, brightness)
4. Benchmark performance (skybox pass time vs 60 FPS budget)

**Implementation Tasks**:
- [ ] **Task 3.1**: Load cubemap texture from HDRI
  - Location: `astraweave-render/src/environment.rs::init_skybox()`
  - Code: Load 6-face cubemap (or equirectangular → cubemap conversion)
  - Validation: Skybox visible with correct orientation
- [ ] **Task 3.2**: Enable skybox pass before main scene
  - Location: `astraweave-render/src/renderer.rs::render()`
  - Code: `self.render_skybox(encoder, &camera)`
  - Validation: Skybox visible behind all scene objects
- [ ] **Task 3.3**: Implement day/night cycle
  - Location: `astraweave-render/src/environment.rs`
  - Code: Lerp between 2 cubemaps based on time-of-day parameter
  - Validation: Sky smoothly transitions from day (blue) to night (black/stars)
- [ ] **Task 3.4**: Add atmospheric scattering (optional)
  - Code: Rayleigh + Mie scattering for realistic atmosphere
  - Validation: Sun/moon glow, horizon color gradient
  - Note: Defer if >1 ms performance cost

**Testing Matrix**:

| Test | Description | Pass Criteria | Status |
|------|-------------|---------------|--------|
| T3.1 | Skybox visibility | Sky visible in all directions | ⏳ |
| T3.2 | Skybox orientation | Horizon horizontal, poles vertical | ⏳ |
| T3.3 | Skybox seams | No visible seams at cube edges | ⏳ |
| T3.4 | Day/night cycle | Smooth transition (0-1 parameter) | ⏳ |
| T3.5 | Depth ordering | Sky behind all scene objects | ⏳ |
| T3.6 | Performance | Skybox pass <0.5 ms | ⏳ |
| T3.7 | Camera movement | Skybox always centered on camera | ⏳ |
| T3.8 | HDR compatibility | Skybox brightness matches scene exposure | ⏳ |

**Benchmarks**:
- `skybox_render_pass`: <0.5 ms (3% of 16.67 ms budget)
- `cubemap_sampling`: <1 µs per fragment (negligible)

**Acceptance Criteria**:
- ✅ All 8 tests passing
- ✅ Benchmarks within budget
- ✅ Visual validation: Seamless cubemap, correct orientation, day/night cycle
- ✅ Tracy profile: Skybox pass <0.5 ms

**Deliverable**: Working skybox with day/night cycle, validated and benchmarked

---

#### Week 4: Dynamic Lights & Point/Spot Shadows (Dec 1-7, 2025)

**Current State**:
- ✅ Directional light exists (single light with CSM shadows)
- ❌ **MISSING**: Point lights, spot lights, dynamic light updates

**Goals**:
1. Add point light support (16+ lights with attenuation)
2. Add spot light support (cone angle, falloff)
3. Implement omnidirectional shadow maps for point lights
4. Test light quality (attenuation, falloff, shadow quality)
5. Benchmark performance (light pass time vs 60 FPS budget)

**Implementation Tasks**:
- [ ] **Task 4.1**: Extend light buffer to support multiple lights
  - Location: `astraweave-render/src/renderer.rs`
  - Code: Change `LightUniform` to array `LightUniform[16]`
  - Validation: 16+ lights rendered correctly
- [ ] **Task 4.2**: Implement point light attenuation
  - Code: `intensity / (1 + d² / r²)` falloff in shader
  - Validation: Light intensity decreases with distance
- [ ] **Task 4.3**: Implement spot light cone
  - Code: `dot(light_dir, -to_pixel) > cos(cone_angle)` cutoff
  - Validation: Light forms cone shape, smooth falloff
- [ ] **Task 4.4**: Implement omnidirectional shadow maps
  - Code: Cubemap depth texture, 6-pass rendering for point lights
  - Validation: Point light casts shadows in all directions
  - Note: Expensive (6× shadow passes), limit to 2-4 shadowed point lights
- [ ] **Task 4.5**: Add light management API
  - Location: `astraweave-render/src/light_manager.rs` (create)
  - Code: `add_point_light()`, `add_spot_light()`, `remove_light()`
  - Validation: Lights can be added/removed at runtime

**Testing Matrix**:

| Test | Description | Pass Criteria | Status |
|------|-------------|---------------|--------|
| T4.1 | Point light | Spherical light falloff | ⏳ |
| T4.2 | Spot light | Cone-shaped light with falloff | ⏳ |
| T4.3 | Multiple lights | 16+ lights rendered correctly | ⏳ |
| T4.4 | Light attenuation | Inverse-square falloff | ⏳ |
| T4.5 | Omnidirectional shadows | Point light shadows in all directions | ⏳ |
| T4.6 | Performance | 16 lights <4 ms, 4 shadowed <8 ms | ⏳ |
| T4.7 | Dynamic updates | Lights move/change color in real-time | ⏳ |
| T4.8 | Light limit | Graceful degradation beyond 16 lights | ⏳ |

**Benchmarks**:
- `point_lights_16_no_shadow`: <4 ms (24% of budget)
- `point_lights_4_with_shadow`: <8 ms (48% of budget)
- `spot_lights_16_no_shadow`: <4 ms
- **Note**: Limit shadowed lights to 4 to stay within budget

**Acceptance Criteria**:
- ✅ All 8 tests passing
- ✅ Benchmarks within budget
- ✅ Visual validation: Point/spot lights work, shadows correct, attenuation correct
- ✅ Tracy profile: 16 lights <4 ms, 4 shadowed <8 ms

**Deliverable**: Working point/spot lights with omnidirectional shadows, validated and benchmarked

---

#### Week 5: GPU Particle System (Dec 8-14, 2025)

**Current State**:
- ❌ **MISSING**: No particle system exists

**Goals**:
1. Implement GPU compute shader particle system
2. Support 10,000+ particles @ 60 FPS
3. Add particle emitters (point, cone, sphere)
4. Add particle effects (fire, smoke, explosions, trails)
5. Benchmark performance (particle update + render time vs 60 FPS budget)

**Implementation Tasks**:
- [ ] **Task 5.1**: Create compute shader particle update
  - Location: `astraweave-render/src/particles.rs` (create)
  - Code: Compute shader updates position, velocity, lifetime
  - Validation: 10,000 particles update in <1 ms
- [ ] **Task 5.2**: Implement particle emitters
  - Code: Point emitter, cone emitter, sphere emitter
  - Validation: Particles spawn from emitter origin with correct velocity
- [ ] **Task 5.3**: Implement particle rendering
  - Code: Instanced quad rendering with additive blending
  - Validation: Particles render as billboards, correct sorting
- [ ] **Task 5.4**: Add particle effects library
  - Effects: Fire (50-100 particles), smoke (100-200), explosion (500-1000), trail (10-20)
  - Validation: Effects look correct, performance within budget
- [ ] **Task 5.5**: Add particle API
  - Location: `astraweave-render/src/particle_manager.rs` (create)
  - Code: `spawn_effect(effect_type, position)`, `update()`, `render()`
  - Validation: API easy to use, effects spawn correctly

**Testing Matrix**:

| Test | Description | Pass Criteria | Status |
|------|-------------|---------------|--------|
| T5.1 | Particle update | 10,000 particles @ 60 FPS | ⏳ |
| T5.2 | Particle spawn | Emitters spawn particles correctly | ⏳ |
| T5.3 | Particle lifetime | Particles fade/die after lifetime | ⏳ |
| T5.4 | Fire effect | Realistic fire with 50-100 particles | ⏳ |
| T5.5 | Smoke effect | Realistic smoke with 100-200 particles | ⏳ |
| T5.6 | Explosion effect | Dramatic explosion with 500-1000 particles | ⏳ |
| T5.7 | Performance | <2 ms for 10,000 particles | ⏳ |
| T5.8 | Sorting | Particles sort correctly (back-to-front) | ⏳ |

**Benchmarks**:
- `particle_update_10k`: <1 ms (compute shader)
- `particle_render_10k`: <1 ms (instanced rendering)
- **Total**: <2 ms (12% of 16.67 ms budget)

**Acceptance Criteria**:
- ✅ All 8 tests passing
- ✅ Benchmarks within budget
- ✅ Visual validation: Fire, smoke, explosion effects look correct
- ✅ Tracy profile: <2 ms for 10,000 particles

**Deliverable**: Working GPU particle system with effects library, validated and benchmarked

---

#### Week 6: ECS World Serialization (Dec 15-21, 2025)

**Current State**:
- ✅ Persistence crate exists: `astraweave-persistence-ecs`
  - SaveManager struct, SaveMetadata struct
  - ECS plugin registration (`install_persistence_systems`)
- ❌ **NOT IMPLEMENTED**: ECS serialization, component derives, archetype save/load

**Goals**:
1. Implement ECS world serialization (all components)
2. Add `#[derive(Serialize, Deserialize)]` to all components
3. Save/load world state to disk (RON format)
4. Test serialization roundtrip (save → load → verify identical)
5. Benchmark performance (save/load time vs user expectations)

**Implementation Tasks**:
- [ ] **Task 6.1**: Add `Serialize`/`Deserialize` derives to all components
  - Location: All crates with components (ecs, physics, nav, ai, etc.)
  - Code: Add `#[derive(Serialize, Deserialize)]` to structs
  - Validation: Components serialize correctly (no errors)
- [ ] **Task 6.2**: Implement world serialization
  - Location: `astraweave-persistence-ecs/src/world_saver.rs` (create)
  - Code: Iterate archetypes, serialize components to RON
  - Validation: World saves to disk without errors
- [ ] **Task 6.3**: Implement world deserialization
  - Location: `astraweave-persistence-ecs/src/world_loader.rs` (create)
  - Code: Parse RON, recreate archetypes and entities
  - Validation: World loads from disk correctly
- [ ] **Task 6.4**: Test serialization roundtrip
  - Test: Create world → save → load → compare (bit-identical or deterministic equality)
  - Validation: All components match after roundtrip
- [ ] **Task 6.5**: Add save file format documentation
  - Location: `docs/current/SAVE_FILE_FORMAT.md` (create)
  - Content: RON schema, versioning, migration strategy
  - Validation: Developers can read save files manually

**Testing Matrix**:

| Test | Description | Pass Criteria | Status |
|------|-------------|---------------|--------|
| T6.1 | Component serialization | All components serialize | ⏳ |
| T6.2 | World save | World saves to disk | ⏳ |
| T6.3 | World load | World loads from disk | ⏳ |
| T6.4 | Roundtrip | save → load → verify identical | ⏳ |
| T6.5 | Large world | 10,000 entities save/load correctly | ⏳ |
| T6.6 | Performance | Save <5s, load <10s @ 10k entities | ⏳ |
| T6.7 | Error handling | Corrupted file detected, graceful error | ⏳ |
| T6.8 | File size | Save file <100 MB for 10k entities | ⏳ |

**Benchmarks**:
- `save_world_10k_entities`: <5 seconds (user expectation: <10s)
- `load_world_10k_entities`: <10 seconds (user expectation: <15s)

**Acceptance Criteria**:
- ✅ All 8 tests passing
- ✅ Benchmarks within expectations
- ✅ Roundtrip validation: Deterministic equality
- ✅ Documentation: Save file format documented

**Deliverable**: Working ECS serialization, validated and benchmarked

---

#### Week 7: Save Slots, Versioning, Corruption Recovery (Dec 22-28, 2025)

**Current State**:
- ✅ Basic world serialization (Week 6)
- ❌ **MISSING**: Player profile, save slots, versioning, corruption recovery

**Goals**:
1. Implement player profile (settings, unlocks, stats)
2. Add save slot management (3-10 slots)
3. Implement save versioning with migration
4. Add corruption detection and auto-backups
5. Test all save/load edge cases

**Implementation Tasks**:
- [ ] **Task 7.1**: Create PlayerProfile struct
  - Location: `astraweave-persistence-ecs/src/player_profile.rs` (create)
  - Fields: settings, unlocks, playtime, stats
  - Validation: Profile saves/loads correctly
- [ ] **Task 7.2**: Implement save slot manager
  - Location: `astraweave-persistence-ecs/src/save_slot_manager.rs` (create)
  - Code: `list_saves()`, `load_save(slot)`, `save_to_slot(slot)`, `delete_save(slot)`
  - Validation: 10 save slots work correctly
- [ ] **Task 7.3**: Add save versioning
  - Code: Add `schema_version: u32` field to save header
  - Validation: Old saves rejected or migrated
- [ ] **Task 7.4**: Implement save migration
  - Code: `migrate_v1_to_v2()` functions for each schema change
  - Validation: Old saves migrate correctly
- [ ] **Task 7.5**: Add corruption detection
  - Code: CRC32 checksum in save header
  - Validation: Corrupted saves detected, error message shown
- [ ] **Task 7.6**: Implement auto-backup system
  - Code: Copy save file before overwrite (keep last 3 backups)
  - Validation: Backups created, can restore from backup

**Testing Matrix**:

| Test | Description | Pass Criteria | Status |
|------|-------------|---------------|--------|
| T7.1 | Player profile | Profile saves/loads | ⏳ |
| T7.2 | Save slots | 10 slots work independently | ⏳ |
| T7.3 | Save versioning | Version mismatch detected | ⏳ |
| T7.4 | Save migration | v1 → v2 migration works | ⏳ |
| T7.5 | Corruption detection | Corrupted file detected | ⏳ |
| T7.6 | Auto-backup | 3 backups created | ⏳ |
| T7.7 | Backup restore | Restore from backup works | ⏳ |
| T7.8 | UI integration | Save/load menu works | ⏳ |

**Acceptance Criteria**:
- ✅ All 8 tests passing
- ✅ Player profile + save slots working
- ✅ Versioning + migration working
- ✅ Corruption detection + auto-backup working
- ✅ UI integration: Save/load menu functional

**Deliverable**: Complete save/load system, production-ready

---

#### Week 8: Dynamic Music & Audio Occlusion (Dec 29-Jan 4, 2026)

**Current State**:
- ✅ Basic audio engine exists: 2 buses (music, SFX), spatial audio
- ❌ **MISSING**: Dynamic music layers, audio occlusion, reverb zones

**Goals**:
1. Implement dynamic music system (4+ simultaneous layers)
2. Add audio occlusion (raycast-based)
3. Add reverb zones (5+ types: cave, hall, outdoor, etc.)
4. Test audio quality (crossfading, occlusion, reverb)
5. Benchmark performance (audio update time vs 60 FPS budget)

**Implementation Tasks**:
- [ ] **Task 8.1**: Implement dynamic music layers
  - Location: `astraweave-audio/src/music_layers.rs` (create)
  - Code: Play 4+ music tracks simultaneously, adjust volume based on game state
  - Validation: Smooth crossfading between layers (no pops/clicks)
- [ ] **Task 8.2**: Implement audio occlusion
  - Location: `astraweave-audio/src/occlusion.rs` (create)
  - Code: Raycast from listener to emitter, attenuate if blocked
  - Validation: Sounds muffled behind walls
- [ ] **Task 8.3**: Implement reverb zones
  - Location: `astraweave-audio/src/reverb.rs` (create)
  - Types: Cave (long reverb), hall (medium), outdoor (short), underwater (filtered), tunnel
  - Validation: Sounds have correct reverb effect in zones
- [ ] **Task 8.4**: Add audio mixer UI
  - Location: Phase 8.1 UI (egui panel)
  - Controls: Master, music, SFX, voice volume sliders
  - Validation: UI changes take effect immediately
- [ ] **Task 8.5**: Test audio with 50+ simultaneous sounds
  - Test: Spawn 50 emitters, play sounds, verify no clipping/distortion
  - Validation: All sounds play correctly, no performance drop

**Testing Matrix**:

| Test | Description | Pass Criteria | Status |
|------|-------------|---------------|--------|
| T8.1 | Dynamic music | 4 layers crossfade smoothly | ⏳ |
| T8.2 | Audio occlusion | Sounds muffled behind walls | ⏳ |
| T8.3 | Reverb zones | 5 zone types work correctly | ⏳ |
| T8.4 | Mixer UI | Volume sliders work | ⏳ |
| T8.5 | 50+ sounds | No clipping/distortion | ⏳ |
| T8.6 | Performance | Audio update <1 ms | ⏳ |
| T8.7 | Spatial audio | 3D positioning correct | ⏳ |
| T8.8 | Music looping | Loops seamlessly (no gap) | ⏳ |

**Benchmarks**:
- `audio_update_50_sounds`: <1 ms (6% of 16.67 ms budget)
- `music_layers_4_tracks`: <0.5 ms
- `raycast_occlusion_10_sounds`: <0.2 ms

**Acceptance Criteria**:
- ✅ All 8 tests passing
- ✅ Benchmarks within budget
- ✅ Visual/audio validation: Dynamic music, occlusion, reverb working
- ✅ Tracy profile: Audio update <1 ms

**Deliverable**: Production audio system, validated and benchmarked

---

#### Week 9-10: Integration & Stress Testing (Jan 5-18, 2026)

**Goals**:
1. Integrate all Phase 8 systems (rendering, UI, save/load, audio)
2. Create Veilweaver Demo Level (5-10 min gameplay)
3. Run stress tests (1,000 entities @ 60 FPS)
4. Fix integration bugs and performance regressions
5. Document Phase 8 completion

**Integration Test Matrix**:

| Test | Description | Pass Criteria | Status |
|------|-------------|---------------|--------|
| I1 | Full rendering | Shadows + bloom + skybox + particles | ⏳ |
| I2 | Full audio | Dynamic music + occlusion + reverb | ⏳ |
| I3 | Save/load | Save → quit → load → verify identical | ⏳ |
| I4 | Veilweaver Demo | 5-10 min playthrough @ 60 FPS | ⏳ |
| I5 | 1,000 entities | All systems @ 60 FPS | ⏳ |
| I6 | Frame budget | <15 ms p95 frame time | ⏳ |
| I7 | Memory usage | <2 GB RAM | ⏳ |
| I8 | No regressions | All Week 5 tests still passing | ⏳ |

**Stress Test Scenarios**:
- **Scenario 1**: 1,000 entities with AI, physics, rendering
- **Scenario 2**: 10,000 particles with shadows + bloom
- **Scenario 3**: 50 simultaneous sounds with occlusion
- **Scenario 4**: Save/load 10,000 entities <10 seconds
- **Scenario 5**: 30-minute gameplay session (no memory leaks)

**Performance Targets** (60 FPS = 16.67 ms budget):

| System | Budget | Measured | Status |
|--------|--------|----------|--------|
| Rendering (shadows + bloom + skybox + particles) | 8 ms | ⏳ | ⏳ |
| UI (egui update + render) | 2 ms | ⏳ | ⏳ |
| Physics (1,000 entities) | 3 ms | ⏳ | ⏳ |
| AI (1,000 agents) | 2 ms | ⏳ | ⏳ |
| Audio (50 sounds) | 1 ms | ⏳ | ⏳ |
| **Total** | **16 ms** | **⏳** | **⏳** |

**Acceptance Criteria**:
- ✅ All 8 integration tests passing
- ✅ All 5 stress tests passing
- ✅ Performance targets met (<15 ms p95)
- ✅ Veilweaver Demo Level playable (5-10 min @ 60 FPS)
- ✅ No regressions vs Week 5 baseline

**Deliverable**: Integrated Phase 8 systems, validated with Veilweaver Demo Level

---

#### Week 11-12: Polish & External Acceptance Testing (Jan 19-Feb 1, 2026)

**Goals**:
1. Fix bugs found in integration testing
2. Polish visuals, audio, UI
3. Run external acceptance tests (10+ playtesters)
4. Document Phase 8 completion
5. Prepare for Phase 9 (build pipeline)

**External Acceptance Test Plan**:
- **Testers**: 10+ external playtesters (not developers)
- **Test**: Complete Veilweaver Demo Level (5-10 min)
- **Metrics**:
  - Completion rate: >80% (8/10 finish demo)
  - Session length: >5 minutes average
  - Crash rate: <5% (0-1 crashes per 10 sessions)
  - Feedback: >70% positive (would recommend)

**Polish Checklist**:
- [ ] Visual polish: No missing textures, correct lighting, smooth animations
- [ ] Audio polish: No pops/clicks, correct volume levels, smooth crossfades
- [ ] UI polish: No clipping, correct layout, responsive controls
- [ ] Performance polish: No frame drops, smooth 60 FPS, no stuttering
- [ ] Bug fixes: All critical bugs fixed (crashes, saves not loading, etc.)

**Deliverable**: Phase 8 complete, ready for Phase 9 (build pipeline)

---

## Phase 9: Distribution & Polish (8-12 Weeks)

### Objective

**Enable shipping games to players** on multiple platforms with optimized assets, telemetry, and crash reporting.

### Week 13-16: Build & Packaging Pipeline

**Goals**:
1. Asset packing (bundle assets into `.pak` archives)
2. Build automation (CI/CD for Windows/Linux/macOS)
3. Installer generation (NSIS, AppImage, DMG)
4. Platform SDK integration (Steam, Epic, itch.io)

**Key Deliverables**:
- `.pak` asset archive format with compression + encryption
- Automated CI/CD builds for all platforms
- Installers for Windows (NSIS), Linux (AppImage), macOS (DMG)
- Steamworks SDK integration (achievements, cloud saves)

**Acceptance Criteria**:
- ✅ Asset packing: <500 MB download, loads <30s on SSD
- ✅ CI/CD: Builds on all platforms without errors
- ✅ Installers: Tested on clean OS, install without errors
- ✅ Platform SDKs: Steam achievements trigger, cloud saves work

---

### Week 17-20: Enhanced Asset Pipeline

**Goals**:
1. Texture atlasing (combine small textures)
2. Animation retargeting (bone mapping)
3. Enhanced LOD generation (auto-decimation)
4. Asset dependency tracking + hot-reload
5. Production hot-reload (reload assets without restart)

**Key Deliverables**:
- Texture atlasing system (UV remapping, draw call optimization)
- Animation retargeting (IK/FK blending)
- Enhanced LOD system (quality targets, transition distances)
- Asset dependency graph (hot-reload cascade)

**Acceptance Criteria**:
- ✅ Texture atlasing: 50% draw call reduction
- ✅ Animation retargeting: 3+ skeleton types supported
- ✅ LOD generation: 3-5 LOD levels auto-generated
- ✅ Hot-reload: <500 ms to reload texture/mesh

---

### Week 21-24: Telemetry & Production Profiling

**Goals**:
1. Lightweight production profiler (frame time, GPU timing, memory)
2. Telemetry system (anonymized metrics, crash dumps)
3. In-game performance overlay (FPS counter, frame graph)
4. Crash reporting (stack traces, log upload)

**Key Deliverables**:
- Production profiler (zero overhead, always-on)
- Telemetry backend (collects metrics from players)
- In-game overlay (press F3 for FPS, frame time, memory)
- Crash dump uploader (sends crash reports to server)

**Acceptance Criteria**:
- ✅ Production profiler: <0.1 ms overhead
- ✅ Telemetry: Metrics received from 10+ test users
- ✅ Crash reporting: Stack traces uploaded, logs attached
- ✅ Performance overlay: FPS counter working, frame graph accurate

---

## Phase 10: Multiplayer & Advanced Features (16-24 Weeks, OPTIONAL)

### Objective

**Enable multiplayer games and advanced visuals** competitive with AAA engines.

### Week 25-32: Networking & Multiplayer

**Goals**:
1. Networking library integration (bevy_renet, laminar, or quinn)
2. Client-server architecture (authoritative server)
3. Replication system (delta compression, interest management)
4. Matchmaking & lobby system
5. Latency compensation (prediction, rollback, lag compensation)

**Key Deliverables**:
- Networking layer (UDP/QUIC transport)
- Client-server foundation (authoritative server, prediction client)
- Entity replication (delta compression, interest management)
- Matchmaking system (lobby UI, server browser)
- Latency compensation (smooth multiplayer @ 100 ms ping)

**Acceptance Criteria**:
- ✅ Networking: 20 Hz tick rate, <50 Kbps per client
- ✅ Replication: 1,000 entities replicated @ 20 Hz
- ✅ Prediction: <50 ms perceived latency @ 100 ms ping
- ✅ Matchmaking: 10+ concurrent players in lobby

---

### Week 33-38: Advanced Rendering

**Goals**:
1. Global Illumination (voxel GI or light probes)
2. Advanced post-FX (DoF, motion blur, chromatic aberration)
3. Decal system (projected decals, deferred integration)
4. Weather effects (rain, snow, wind simulation)

**Key Deliverables**:
- GI system (indirect lighting, bounce light)
- Advanced post-processing (cinematic effects)
- Decal rendering (bullet holes, blood splatters)
- Weather system (procedural rain/snow)

**Acceptance Criteria**:
- ✅ GI: Realistic indirect lighting, <5 ms overhead
- ✅ Post-FX: DoF, motion blur, chromatic aberration working
- ✅ Decals: 100+ decals @ 60 FPS
- ✅ Weather: Rain/snow effects, <2 ms overhead

---

### Week 39-48: Advanced AI & Console Ports

**Goals**:
1. Improved LLM success rates (40% → 80%+)
2. Prompt caching (50× speedup)
3. Multi-agent coordination (swarm tactics)
4. Console port foundations (Xbox, PlayStation, Switch)

**Key Deliverables**:
- LLM improvements (phi3:medium 14B, parameter defaulting)
- Prompt caching system
- Squad AI (cooperative strategies)
- Console SDK integration (optional)

**Acceptance Criteria**:
- ✅ LLM: 80%+ success rate (vs 40-50% current)
- ✅ Prompt caching: 50× speedup (3.5s → 70ms)
- ✅ Squad AI: 10+ agents coordinate tactics
- ✅ Console ports: Dev kits integrated (if applicable)

---

## Quality Gates & CI Enforcement

### Phase 8 Quality Gates

**Every Week**:
- ✅ Zero compilation errors (all crates build)
- ✅ Zero warnings in core crates (clippy passes)
- ✅ All unit tests passing (100%)
- ✅ All integration tests passing (100%)
- ✅ Benchmarks within budget (no regressions >10%)
- ✅ Tracy profile captured (frame time <16 ms)

**Every 2 Weeks**:
- ✅ Stress tests passing (1,000 entities @ 60 FPS)
- ✅ Memory leak check (30-min session <100 MB leak)
- ✅ Save/load roundtrip (deterministic equality)

**Phase End**:
- ✅ External acceptance tests (10+ playtesters)
- ✅ Veilweaver Demo Level playable (5-10 min @ 60 FPS)
- ✅ Completion report (30+ pages, all metrics documented)

---

## Master Test Matrix

### Unit Tests (Component-Level)

| Crate | Tests | Current | Target | Status |
|-------|-------|---------|--------|--------|
| astraweave-render | Shadow, bloom, skybox, particles | 10 | 30 | ⏳ |
| astraweave-audio | Music layers, occlusion, reverb | 5 | 20 | ⏳ |
| astraweave-persistence-ecs | Serialization, versioning, corruption | 5 | 25 | ⏳ |
| **Total** | **20** | **75** | **⏳** |

### Integration Tests (System-Level)

| Test | Description | Target | Status |
|------|-------------|--------|--------|
| Full rendering | Shadows + bloom + skybox + particles @ 60 FPS | Week 5 | ⏳ |
| Full audio | Dynamic music + occlusion + reverb @ 50 sounds | Week 8 | ⏳ |
| Save/load | 10,000 entities <10 seconds | Week 7 | ⏳ |
| Veilweaver Demo | 5-10 min playthrough @ 60 FPS | Week 10 | ⏳ |

### Stress Tests (Performance)

| Test | Description | Target | Status |
|------|-------------|--------|--------|
| 1,000 entities | All systems @ 60 FPS | <15 ms p95 | ⏳ |
| 10,000 particles | Particles + shadows + bloom @ 60 FPS | <16 ms | ⏳ |
| 50 sounds | Audio + occlusion + reverb | <1 ms | ⏳ |
| 30-min session | No memory leaks | <100 MB leak | ⏳ |

### Acceptance Tests (User-Level)

| Test | Description | Target | Status |
|------|-------------|--------|--------|
| Veilweaver Demo | 10+ external playtesters complete demo | >80% completion | ⏳ |
| Crash rate | Crash rate across 10 sessions | <5% | ⏳ |
| Feedback | Positive feedback rate | >70% | ⏳ |

---

## Risk Assessment & Mitigation

### High-Risk Items

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| Shadow artifacts | Visual quality | Medium | Add PCF filtering, bias correction, cascade debug visualization |
| Save corruption | Data loss | High | CRC32 checksums, auto-backups (keep last 3), corruption detection |
| Performance regression | Frame rate drops | Medium | CI benchmarks, Tracy profiling, weekly performance reviews |
| LLM success rate | Gameplay quality | Medium | Fallback to GOAP, simplified tool set, prompt engineering |
| External tester availability | Timeline delay | Medium | Recruit testers early, offer incentives (Steam keys, credits) |

### Low-Risk Items

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| Bloom artifacts | Visual quality | Low | Mip chain validation, threshold tuning |
| Audio clipping | Audio quality | Low | Volume normalization, clipping detection |
| Skybox seams | Visual quality | Low | Cubemap validation, edge padding |

---

## Success Metrics

### Phase 8 Success Metrics

| Metric | Current | Target | Status |
|--------|---------|--------|--------|
| Rendering complete | 40% | 100% | ⏳ |
| Save/load complete | 10% | 100% | ⏳ |
| Audio complete | 30% | 100% | ⏳ |
| Veilweaver Demo playable | No | Yes | ⏳ |
| External playtesters | 0 | 10+ | ⏳ |
| Frame time (p95) | ? | <15 ms | ⏳ |
| Crash rate | ? | <5% | ⏳ |

### Phase 9 Success Metrics

| Metric | Current | Target | Status |
|--------|---------|--------|--------|
| Asset packing | No | Yes | ⏳ |
| CI/CD builds | No | Yes | ⏳ |
| Installers | No | Yes | ⏳ |
| Platform SDKs | No | Yes | ⏳ |
| Telemetry | No | Yes | ⏳ |
| Public release | No | Yes | ⏳ |

### Phase 10 Success Metrics (OPTIONAL)

| Metric | Current | Target | Status |
|--------|---------|--------|--------|
| Networking | No | Yes | ⏳ |
| Multiplayer demo | No | Yes | ⏳ |
| GI rendering | No | Yes | ⏳ |
| LLM success rate | 40-50% | 80%+ | ⏳ |

---

## Documentation Requirements

### Per-Week Documentation

- **Daily logs**: What was done, what works, what's blocked (1-2 paragraphs)
- **Weekly summary**: Achievements, metrics, tests passing, next week plan (5-10 pages)
- **Code documentation**: All new functions have docstrings, public APIs documented

### Per-Phase Documentation

- **Completion report**: 30+ pages, comprehensive metrics, all tests documented
- **API reference**: All new APIs documented (Rust docs + examples)
- **User guide**: How to use new features (markdown tutorials)

---

## Appendix A: Existing System Inventory

### Rendering (`astraweave-render`)

**Existing Infrastructure**:
- ✅ **Shadow System**: CSM, 2-cascade depth array, 3×3 PCF filtering, bias correction
  - Location: `renderer.rs` lines 366-383 (shadow resources), 164-194 (shader code)
  - Status: Infrastructure exists, NOT ENABLED
- ✅ **Post-Processing**: Bloom (5-mip pyramid), tonemapping stubs, SSAO/SSR shader stubs
  - Location: `renderer.rs` lines 329-364 (bloom pipelines), `shaders/post_fx.wgsl`
  - Status: Pipelines exist, NOT ENABLED
- ✅ **Skybox**: Cubemap rendering, inverted cube geometry, view-centered shader
  - Location: `environment.rs` lines 182-560 (skybox pipeline)
  - Status: Pipeline exists, NOT ENABLED
- ❌ **Dynamic Lights**: No point/spot light support, no omnidirectional shadows
- ❌ **Particles**: No particle system

**Estimated Effort Savings**: 3-4 weeks (shadows, bloom, skybox already exist)

### Audio (`astraweave-audio`)

**Existing Infrastructure**:
- ✅ **AudioEngine**: 2 buses (music, SFX), spatial audio (SpatialSink), volume control
  - Location: `engine.rs` lines 133-290
  - Status: Basic mixer exists
- ❌ **Dynamic Music**: No layer system, no crossfading logic
- ❌ **Occlusion**: No raycast-based occlusion
- ❌ **Reverb Zones**: No reverb effects

**Estimated Effort Savings**: 1 week (basic mixer exists)

### Save/Load (`astraweave-persistence-ecs`)

**Existing Infrastructure**:
- ✅ **Persistence Crate**: SaveManager, SaveMetadata, ECS plugin registration
  - Location: `lib.rs` lines 1-73
  - Status: Crate exists, serialization NOT IMPLEMENTED
- ❌ **ECS Serialization**: No component derives, no world save/load
- ❌ **Save Slots**: No slot management
- ❌ **Versioning**: No migration system

**Estimated Effort Savings**: 0 weeks (only stubs exist)

---

## Appendix B: Performance Budget Breakdown

### 60 FPS Target (16.67 ms per frame)

**Budget Allocation** (with 10% headroom = 15.00 ms target):

| System | Budget | Justification |
|--------|--------|---------------|
| Rendering | 8 ms | Shadows (2ms) + bloom (3ms) + skybox (0.5ms) + particles (2ms) + mesh (0.5ms) |
| UI | 2 ms | egui update (1ms) + render (1ms) |
| Physics | 3 ms | 1,000 entities @ 6.52 µs (validated Week 2) extrapolated |
| AI | 2 ms | 1,000 agents @ 2.10 µs (validated Week 8) extrapolated |
| Audio | 1 ms | 50 sounds @ 0.02 ms each |
| **Total** | **16 ms** | **With 10% headroom = 15 ms actual target** |

**Validation Strategy**:
- Tracy profiling every week (capture frame time distribution)
- CI benchmarks enforce no regressions >10%
- Stress tests validate 1,000 entities @ 60 FPS

---

## Appendix C: CI/CD Quality Gates

### Pre-Merge Checks (GitHub Actions)

**Every Pull Request**:
```yaml
- name: Compile check
  run: cargo check --all-features
  
- name: Clippy (zero warnings)
  run: cargo clippy --all-features -- -D warnings
  
- name: Unit tests
  run: cargo test --all-features
  
- name: Benchmarks (regression check)
  run: cargo bench --all-features -- --save-baseline current
  
- name: Benchmark comparison
  run: ./scripts/check_benchmark_regression.sh
  # Fails if >10% regression vs baseline
```

**Post-Merge** (main branch):
```yaml
- name: Integration tests
  run: cargo test --test integration_*
  
- name: Stress tests
  run: cargo test --test stress_* --release
  
- name: Tracy profile capture
  run: ./scripts/capture_tracy_profile.sh
  # Saves profile for analysis
```

---

## Appendix D: External Playtester Protocol

### Recruitment (Week 11)

**Target**: 10+ external playtesters (not developers)

**Criteria**:
- Experience with PC games
- Willing to provide feedback
- Available for 1-2 hour session

**Incentives**:
- Early access to Veilweaver
- Credits in game
- Optional: Steam key for full release

### Test Protocol (Week 12)

**Pre-Test**:
- Send installer (Windows/Linux/macOS)
- Install instructions
- Pre-test survey (PC specs, gaming experience)

**Test Session** (1-2 hours):
- Play Veilweaver Demo Level (5-10 min)
- Optional: Continue playing after demo
- Screen recording (with consent)
- Performance monitoring (frame time, crashes)

**Post-Test**:
- Post-test survey (feedback, bugs, suggestions)
- Crash logs uploaded (if any)
- Telemetry data collected (anonymized)

### Feedback Analysis

**Metrics**:
- Completion rate: % who finish demo
- Session length: Average playtime
- Crash rate: % of sessions with crash
- Positive feedback: % who would recommend

**Bugs**:
- Critical: Crashes, save corruption, game-breaking bugs
- High: Visual glitches, audio issues, performance problems
- Medium: UI issues, minor bugs
- Low: Cosmetic issues, feature requests

---

## Next Steps

**IMMEDIATE (Week 1 — Nov 10-16, 2025)**:
1. ✅ Read Phase 8-10 plan (this document)
2. ⏳ Create Week 1 task board (shadow mapping tasks)
3. ⏳ Enable shadow depth passes in renderer
4. ⏳ Validate CSM with rotating directional light
5. ⏳ Benchmark shadow performance (<2 ms @ 100 meshes)

**Week 2-8**:
- Follow week-by-week plan above
- Update task board every Monday
- Weekly summary report every Friday
- Tracy profile captured every week

**Week 9-12**:
- Integration testing
- External acceptance testing
- Phase 8 completion report
- Prepare for Phase 9

---

**Let's make AstraWeave world-class! 🚀**
