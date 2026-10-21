# AstraWeave Game Engine Readiness Roadmap

**Document Version**: 1.0  
**Date**: January 2025  
**Status**: Phase 7 Complete, Planning Next Steps  
**Objective**: Transform AstraWeave from "production-ready infrastructure" to "ship a game on it"

---

## Executive Summary

AstraWeave has achieved remarkable technical milestones—a production-validated AI-native game engine with 12,700+ agent capacity, 100% deterministic simulation, and comprehensive tooling. **However, significant gaps remain before a game developer can create and ship a complete game.**

This roadmap identifies **8 critical gaps** organized into 3 phases over **6-12 months**, taking AstraWeave from its current state (excellent infrastructure, excellent AI, good authoring tools) to **"ship a game with zero external dependencies"**.

### Current State (January 2025)

**✅ Production-Ready**:
- AI-native architecture (Perception → Reasoning → Planning → Action)
- 12,700+ agents @ 60 FPS (validated)
- Deterministic ECS with 100% replay accuracy
- Real Phi-3 LLM integration (40-50% success rate)
- 37-tool vocabulary for AI planning
- Tracy profiling, SIMD optimization, spatial hash collision
- GPU skinning, mesh optimization, PBR rendering pipeline
- Editor with 14 panels (BT/Dialogue/Quest graphs, terrain painter, material editor)

**⚠️ Missing for "Ship a Game"**:
- **No complete rendering pipeline** (skybox, post-processing, shadows, lighting)
- **No production audio system** (3D spatial audio exists but needs integration)
- **No UI framework** for in-game menus/HUD
- **No save/load system** for player progress
- **No packaged build pipeline** for distribution
- **No networking** for multiplayer
- **No performance profiling in production** (Tracy is dev-only)
- **Limited asset pipeline** (no texture atlasing, no animation retargeting)

### Transformation Goal

**From**: "Excellent game engine infrastructure with AI focus"  
**To**: "Fully capable game engine where a developer can create, test, and ship a complete game"

---

## Gap Analysis: 8 Critical Missing Features

### 1. Complete Rendering Pipeline 🎨 **CRITICAL**

**Current State**:
- ✅ PBR materials with IBL
- ✅ BC7/BC5 texture compression
- ✅ GPU skinning
- ✅ Mesh optimization
- ❌ No skybox/skydome
- ❌ No post-processing (bloom, tonemapping, color grading)
- ❌ No shadow mapping (critical for 3D games)
- ❌ No dynamic lighting beyond IBL
- ❌ No particle systems
- ❌ No volumetric effects

**Impact on Game Development**:
- **Blocker**: 3D games look flat without shadows and dynamic lighting
- **Blocker**: No day/night cycle possible without skybox
- **High Impact**: AAA-quality visuals impossible without post-processing

**Required Work**:
1. Shadow Mapping (CSM for outdoor, omnidirectional for point lights)
2. Skybox/Atmosphere rendering
3. Post-processing stack (bloom, tonemapping, ACES, SSAO)
4. Dynamic point/spot lights with shadow casting
5. Particle system (GPU-accelerated, 10k+ particles)
6. Volumetric fog/lighting

**Estimated Effort**: 4-6 weeks

---

### 2. Production Audio System 🔊 **CRITICAL**

**Current State**:
- ✅ Spatial audio basics (`astraweave-audio`)
- ✅ Dialogue system with audio playback
- ❌ No audio mixer/bus system
- ❌ No dynamic music system (layering, crossfading)
- ❌ No audio occlusion/reverb zones
- ❌ No FMOD/Wwise integration (industry-standard)
- ❌ Audio editing only via TOML files (no in-editor tools)

**Impact on Game Development**:
- **Blocker**: No way to adjust audio levels during gameplay
- **High Impact**: Can't create immersive audio experiences
- **Medium Impact**: Hard to prototype audio without visual tools

**Required Work**:
1. Audio mixer system (master, music, SFX, voice buses)
2. Dynamic music system with layers and crossfades
3. Audio occlusion (raycast-based or zone-based)
4. Reverb zones for environmental audio
5. In-editor audio preview and bus controls
6. Optional: FMOD/Wwise bridge for pro audio workflows

**Estimated Effort**: 3-4 weeks

---

### 3. In-Game UI Framework 🖼️ **CRITICAL**

**Current State**:
- ✅ Editor UI (egui for authoring tools)
- ❌ No in-game UI system
- ❌ No HUD rendering
- ❌ No menu system (main menu, pause, settings)
- ❌ No dialog boxes / prompts
- ❌ No UI layout system (anchors, responsive design)

**Impact on Game Development**:
- **BLOCKER**: Can't create playable games without menus/HUD
- **BLOCKER**: No way to show player health, ammo, objectives
- **High Impact**: No settings menu for players

**Required Work**:
1. In-game UI rendering layer (separate from editor egui)
   - Option A: Integrate egui for in-game (simpler, less control)
   - Option B: Custom immediate-mode UI (more work, full control)
   - Option C: Integrate existing Rust UI lib (e.g., kayak_ui, iced)
2. HUD system (health bars, ammo counters, minimaps)
3. Menu system (main menu, pause, settings with save/load)
4. Dialog/prompt system (confirmation boxes, text input)
5. UI layout engine (anchors, aspect ratio handling)
6. UI animation support (tweening, transitions)

**Estimated Effort**: 4-5 weeks

---

### 4. Save/Load System 💾 **HIGH**

**Current State**:
- ✅ Editor can save levels (TOML/JSON)
- ✅ Deterministic ECS (perfect for save states)
- ❌ No player save system (progress, inventory, unlocks)
- ❌ No save slot management
- ❌ No cloud save integration
- ❌ No save corruption detection/recovery

**Impact on Game Development**:
- **Blocker**: Can't create games with progression
- **High Impact**: No way to resume gameplay
- **Medium Impact**: No cross-platform save sync

**Required Work**:
1. Save system architecture:
   - Serialize ECS world state (entities + components)
   - Player profile data (settings, unlocks, stats)
   - Campaign progress (quest states, dialogue choices)
2. Save slot management (multiple saves, auto-save)
3. Save versioning (handle updates without breaking saves)
4. Corruption detection (checksums, validation)
5. Optional: Cloud save (Steam Cloud, platform SDKs)

**Estimated Effort**: 2-3 weeks

---

### 5. Build & Packaging Pipeline 📦 **HIGH**

**Current State**:
- ✅ Compiles on Windows/Linux/macOS
- ❌ No asset packing (loose files in `assets/`)
- ❌ No build pipeline for distribution
- ❌ No installer generation
- ❌ No platform-specific optimizations
- ❌ No Steam/Epic/itch.io integration

**Impact on Game Development**:
- **Blocker**: Can't distribute games to players
- **High Impact**: Large download sizes (uncompressed assets)
- **Medium Impact**: No platform achievements/leaderboards

**Required Work**:
1. Asset packing:
   - Bundle assets into `.pak` or `.zip` archives
   - Compressed, encrypted, indexed for fast loading
2. Build automation:
   - CI/CD for release builds (Windows .exe, Linux AppImage, macOS .app)
   - Strip debug symbols, optimize for size
3. Installer generation:
   - Windows: NSIS or WiX installer
   - Linux: AppImage or Flatpak
   - macOS: DMG with code signing
4. Platform SDK integration:
   - Steamworks (achievements, cloud saves, multiplayer)
   - Epic Online Services
   - itch.io API

**Estimated Effort**: 3-4 weeks

---

### 6. Networking & Multiplayer 🌐 **MEDIUM** (Optional for many games)

**Current State**:
- ✅ Deterministic simulation (perfect for lockstep)
- ✅ Authoritative validation (anti-cheat ready)
- ❌ No networking layer
- ❌ No client-server architecture
- ❌ No matchmaking
- ❌ No latency compensation

**Impact on Game Development**:
- **Optional**: Not needed for single-player games
- **Blocker**: Can't create multiplayer games
- **High Impact**: Huge market opportunity (online games)

**Required Work** (if targeting multiplayer):
1. Networking library integration (e.g., `bevy_renet`, `laminar`, `quinn` for QUIC)
2. Client-server architecture:
   - Server: Authoritative simulation + validation
   - Client: Prediction + rollback for smooth input
3. Replication system:
   - Delta compression for entity state
   - Interest management (only sync nearby entities)
4. Matchmaking & lobby system
5. Latency compensation:
   - Client-side prediction
   - Server reconciliation
   - Lag compensation for hit detection

**Estimated Effort**: 6-8 weeks (full multiplayer support)

---

### 7. Enhanced Asset Pipeline 🎨 **MEDIUM**

**Current State**:
- ✅ `aw_asset_cli` for texture/model/audio processing
- ✅ BC7/BC5 compression
- ❌ No texture atlasing (draw call optimization)
- ❌ No animation retargeting
- ❌ No LOD auto-generation (basic LOD exists)
- ❌ No asset dependency tracking
- ❌ No hot-reload in production builds

**Impact on Game Development**:
- **Medium Impact**: Inefficient draw calls without atlasing
- **Medium Impact**: Hard to reuse animations across characters
- **Low Impact**: Manual LOD creation tedious but possible

**Required Work**:
1. Texture Atlasing:
   - Combine small textures into atlas for sprite batching
   - UV coordinate remapping
2. Animation retargeting:
   - Bone mapping between skeletons
   - IK/FK blending for different proportions
3. Enhanced LOD generation:
   - Auto-decimation with quality targets
   - Transition distances based on screen size
4. Asset dependency graph:
   - Track which assets reference others
   - Hot-reload cascade (material → textures → shaders)
5. Production hot-reload:
   - File watcher in release builds (optional)
   - Asset versioning for incremental updates

**Estimated Effort**: 3-4 weeks

---

### 8. Performance Profiling in Production 📊 **LOW** (Tracy is dev-only)

**Current State**:
- ✅ Tracy profiling integrated (Week 8)
- ❌ Tracy doesn't work in release builds (debug overhead)
- ❌ No production telemetry
- ❌ No crash reporting
- ❌ No frame time graphs for players

**Impact on Game Development**:
- **Low Impact**: Can debug in dev builds
- **Medium Impact**: Can't diagnose player performance issues
- **High Impact**: No crash logs from players

**Required Work**:
1. Lightweight production profiler:
   - Low-overhead frame time tracking
   - GPU timing queries
   - Memory allocation tracking
2. Telemetry system:
   - Send anonymized metrics to server
   - Crash dumps with stack traces
   - Performance percentiles (p50, p95, p99)
3. In-game performance overlay:
   - FPS counter
   - Frame time graph
   - Memory usage

**Estimated Effort**: 2-3 weeks

---

## Roadmap: 3 Phases Over 6-12 Months

### Phase 8: Core Game Loop Essentials (Months 1-3) **PRIORITY: CRITICAL**

**Goal**: Enable creation of complete single-player games  
**Timeline**: 10-14 weeks  
**Dependencies**: None (can start immediately)

**Deliverables**:

1. **Complete Rendering Pipeline** (4-6 weeks)
   - Shadow mapping (CSM + omnidirectional)
   - Skybox/atmosphere
   - Post-processing stack (bloom, tonemapping, SSAO)
   - Dynamic lighting (point/spot/directional)
   - Particle system (GPU-accelerated)
   - Volumetric fog/lighting

2. **In-Game UI Framework** (4-5 weeks)
   - In-game UI rendering (egui or custom)
   - HUD system (health, ammo, objectives)
   - Menu system (main, pause, settings)
   - Dialog/prompt system
   - UI layout engine (anchors, responsive)
   - UI animations

3. **Save/Load System** (2-3 weeks)
   - Serialize ECS world state
   - Player profile (settings, unlocks)
   - Save slot management
   - Save versioning
   - Corruption detection

4. **Production Audio** (3-4 weeks)
   - Audio mixer (master, music, SFX, voice)
   - Dynamic music (layers, crossfades)
   - Audio occlusion/reverb
   - In-editor audio tools

**Success Criteria**:
- ✅ Can create a 3D game with shadows, lighting, skybox, and particles
- ✅ Can create in-game menus, HUD, and dialog boxes
- ✅ Can save/load player progress
- ✅ Can mix audio levels and create dynamic music
- ✅ Example game: "Veilweaver Demo Level" (5-10 min gameplay loop)

**Estimated Total**: **13-18 weeks** (3-4.5 months)

---

### Phase 9: Distribution & Polish (Months 4-6) **PRIORITY: HIGH**

**Goal**: Enable shipping games to players  
**Timeline**: 6-10 weeks  
**Dependencies**: Phase 8 complete

**Deliverables**:

1. **Build & Packaging Pipeline** (3-4 weeks)
   - Asset packing (`.pak` archives)
   - Build automation (CI/CD for releases)
   - Installer generation (Windows, Linux, macOS)
   - Platform SDK integration (Steam, Epic, itch.io)

2. **Enhanced Asset Pipeline** (3-4 weeks)
   - Texture atlasing
   - Animation retargeting
   - Enhanced LOD generation
   - Asset dependency graph
   - Production hot-reload

3. **Performance Profiling** (2-3 weeks)
   - Lightweight production profiler
   - Telemetry system
   - Crash reporting
   - In-game perf overlay

**Success Criteria**:
- ✅ Can package a game for distribution (Windows, Linux, macOS)
- ✅ Can publish to Steam/Epic/itch.io
- ✅ Can track player performance and crashes
- ✅ Assets are optimized for shipping (atlasing, compression)
- ✅ Example: "Veilweaver Early Access" release candidate

**Estimated Total**: **8-11 weeks** (2-2.75 months)

---

### Phase 10: Multiplayer & Advanced Features (Months 7-12) **PRIORITY: OPTIONAL**

**Goal**: Enable multiplayer games and advanced visuals  
**Timeline**: 12-20 weeks  
**Dependencies**: Phase 8 + 9 complete

**Deliverables**:

1. **Networking & Multiplayer** (6-8 weeks)
   - Networking library integration
   - Client-server architecture
   - Replication system
   - Matchmaking & lobbies
   - Latency compensation

2. **Advanced Rendering** (4-6 weeks)
   - GI (global illumination) - voxel GI or light probes
   - Advanced post-FX (DoF, motion blur, chromatic aberration)
   - Decal system
   - Weather effects (rain, snow, wind)

3. **Advanced AI Features** (2-4 weeks)
   - Improved LLM success rates (40% → 80%+)
     - Use phi3:medium (14B) instead of phi3:game (2.2GB)
     - Parameter defaulting for missing fields
     - Simplified Tier 2 tool set (8 uniform tools)
   - Prompt caching for 50× speedup
   - Multi-agent coordination (swarm tactics)
   - AI director system (dynamic difficulty)

4. **Console Support** (4-6 weeks) - OPTIONAL
   - Xbox Series X/S (via GDKX)
   - PlayStation 5 (via PS5 SDK)
   - Nintendo Switch (via NintendoSDK)
   - Controller input abstraction

**Success Criteria**:
- ✅ Can create 2-16 player multiplayer games
- ✅ Can ship games with AAA-level graphics
- ✅ LLM planning success rate >80%
- ✅ (Optional) Can ship to consoles
- ✅ Example: "Veilweaver 1.0" full release with multiplayer mode

**Estimated Total**: **16-24 weeks** (4-6 months)

---

## Summary Roadmap

| Phase | Focus | Duration | Critical? |
|-------|-------|----------|-----------|
| **Phase 8** | Core game loop (rendering, UI, save/load, audio) | 3-4.5 months | ✅ CRITICAL |
| **Phase 9** | Distribution (packaging, asset pipeline, profiling) | 2-2.75 months | ✅ HIGH |
| **Phase 10** | Multiplayer & advanced features (networking, GI, consoles) | 4-6 months | ⚠️ OPTIONAL |
| **TOTAL** | Single-player game engine → Full-featured | **6-12 months** | |

---

## Prioritization Matrix

### Must-Have (Blockers for "Ship a Game")

1. **Rendering Pipeline** - 3D games need shadows and lighting
2. **In-Game UI** - Every game needs menus and HUD
3. **Save/Load** - Games with progression need save states
4. **Audio System** - Immersive games need spatial audio
5. **Build Pipeline** - Must distribute to players

### Should-Have (Quality & Developer Experience)

6. **Asset Pipeline** - Improves performance and workflow
7. **Profiling** - Needed for optimization and debugging

### Nice-to-Have (Expands Market)

8. **Networking** - Opens multiplayer market (large but optional)
9. **Advanced Rendering** - Competes with AAA engines
10. **Console Support** - Console market is huge but requires SDK licensing

---

## Recommended Action Plan

### Immediate Next Steps (Month 1-2)

**Week 1-2**: Shadow Mapping + Dynamic Lighting
- Implement CSM (Cascaded Shadow Maps) for directional light
- Add omnidirectional shadow maps for point lights
- Integrate with existing PBR pipeline

**Week 3-4**: Skybox + Post-Processing
- Skybox/skydome rendering
- Bloom pass
- Tonemapping (ACES)
- SSAO (Screen-Space Ambient Occlusion)

**Week 5-6**: In-Game UI Framework
- Choose UI library (egui for consistency, or custom)
- Implement HUD rendering layer
- Create menu system (main menu, pause, settings)

**Week 7-8**: Save/Load System
- Design save format (RON or bincode for ECS)
- Implement save slot management
- Add save versioning

### Month 3-4: Audio + Packaging

**Week 9-10**: Audio Mixer + Dynamic Music
- Audio bus system
- Dynamic music layers
- Reverb zones

**Week 11-12**: Build Pipeline
- Asset packing (`.pak` archives)
- CI/CD for release builds
- Installer generation

### Month 5-6: Polish + Ship Example Game

**Week 13-16**: Veilweaver Demo Level
- Complete 5-10 minute gameplay loop
- Integrate all Phase 8 features
- Polish and optimize

**Week 17-20**: Distribution Prep
- Steam integration
- Performance profiling
- Crash reporting

### Month 7-12: Optional Multiplayer & Advanced Features

**Only pursue if**:
- Phase 8 + 9 complete
- Community demand for multiplayer
- Resources available for 4-6 month commitment

---

## Success Metrics

### Phase 8 Success (End of Month 4)

| Metric | Current | Target |
|--------|---------|--------|
| Rendering features | 4/10 | 10/10 (shadows, skybox, post-FX, particles) |
| In-game UI | 0/5 | 5/5 (menus, HUD, dialogs, layout, animations) |
| Save/Load | 0/4 | 4/4 (save slots, versioning, corruption detection) |
| Audio | 2/6 | 6/6 (mixer, music, occlusion, reverb, editor tools) |
| Example game playable | No | Yes (5-10 min demo level) |

### Phase 9 Success (End of Month 6)

| Metric | Current | Target |
|--------|---------|--------|
| Build pipeline | 0/4 | 4/4 (packing, automation, installers, SDK) |
| Asset pipeline | 2/5 | 5/5 (atlasing, retargeting, LOD, dependency, hot-reload) |
| Profiling | 1/3 | 3/3 (production profiler, telemetry, crash reporting) |
| Shipped game | No | Yes (Veilweaver Early Access on itch.io) |

### Phase 10 Success (End of Month 12)

| Metric | Current | Target |
|--------|---------|--------|
| Networking | 0/5 | 5/5 (client-server, replication, matchmaking) |
| Advanced rendering | 0/4 | 4/4 (GI, advanced post-FX, decals, weather) |
| LLM success rate | 40-50% | 80%+ (with phi3:medium) |
| Console support | 0/3 | 1-3/3 (Xbox, PlayStation, Switch) |
| Shipped multiplayer game | No | Yes (Veilweaver 1.0 with co-op mode) |

---

## Risk Assessment

### High Risk

1. **Rendering Pipeline Complexity**
   - Risk: Shadow mapping + GI can take 8+ weeks alone
   - Mitigation: Start with CSM only, defer GI to Phase 10
   - Fallback: Use existing Unity/Unreal tutorials for reference

2. **UI Framework Choice**
   - Risk: Wrong choice locks in for years
   - Mitigation: Prototype with egui first (quick), evaluate later
   - Fallback: Custom immediate-mode UI (more work but full control)

3. **Networking is Massive**
   - Risk: Can easily consume 6+ months
   - Mitigation: Make Phase 10 entirely optional
   - Fallback: Partner with networking specialists or license middleware

### Medium Risk

4. **Asset Pipeline Fragmentation**
   - Risk: Many moving parts (atlasing, retargeting, LOD, hot-reload)
   - Mitigation: Implement incrementally, test each feature
   - Fallback: Manual asset optimization (slower but works)

5. **Platform SDK Integration**
   - Risk: Steamworks, Epic, itch.io APIs change frequently
   - Mitigation: Use community-maintained bindings (e.g., `steamworks-rs`)
   - Fallback: Basic builds without platform features

### Low Risk

6. **Save/Load Corruption**
   - Risk: Save corruption destroys player progress
   - Mitigation: Checksums, versioning, auto-backup
   - Fallback: Players can always start new save

7. **Profiling Overhead**
   - Risk: Production profiler slows down game
   - Mitigation: Make telemetry opt-in, use sampling
   - Fallback: Disable profiling in final builds

---

## Alternative Approach: Partner with Existing Engines

**If timeline is too aggressive**, consider:

1. **Focus on AI-Native Features Only**
   - Partner with Unity/Unreal for rendering/UI/audio
   - Export AstraWeave AI system as plugin
   - Faster to market (3-6 months vs 12 months)

2. **Hybrid Approach**
   - Use AstraWeave for AI + simulation
   - Use Godot/Bevy for rendering + UI
   - Bridge via FFI (C API already exists)

3. **Community-Driven Development**
   - Open-source missing features
   - Accept contributions for rendering/networking
   - Slower but sustainable

---

## Conclusion

**AstraWeave is 60-70% complete** for shipping a full game:

✅ **Excellent**: AI-native architecture, deterministic ECS, performance validation  
✅ **Good**: Authoring tools, GPU rendering basics, asset pipeline  
⚠️ **Needs Work**: Complete rendering, in-game UI, save/load, audio mixer  
❌ **Missing**: Build pipeline, networking (optional)

**Recommended Path**:
1. **Commit to Phase 8** (3-4.5 months) - Core game loop essentials
2. **Evaluate market fit** - Does demo resonate?
3. **Proceed to Phase 9** (2-2.75 months) - Distribution readiness
4. **Defer Phase 10** unless multiplayer is critical

**Total to "Ship a Single-Player Game"**: **6-7 months** (Phases 8 + 9)

---

**Next Action**: Approve Phase 8 roadmap and begin **Shadow Mapping** implementation (Week 1-2)

---

**Document Status**: Ready for review  
**Last Updated**: January 2025  
**Maintainer**: AI Development Team
