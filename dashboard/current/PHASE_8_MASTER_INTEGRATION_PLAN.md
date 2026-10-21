# Phase 8 Master Integration Plan

**Document Version**: 1.0  
**Date**: October 14, 2025  
**Duration**: 12-16 weeks (3-4 months)  
**Objective**: Coordinate all 4 Phase 8 priorities to deliver a complete game engine

---

## Executive Summary

**Mission**: Transform AstraWeave from "in-progress infrastructure" to "ship a complete game" by completing in-game UI, rendering pipeline, save/load system, and production audio.

**4 Parallel Workstreams**:
1. **Phase 8.1**: In-Game UI Framework (5 weeks, Priority 1, CRITICAL)
2. **Phase 8.2**: Complete Rendering Pipeline (4-5 weeks, Priority 2)
3. **Phase 8.3**: Save/Load System (2-3 weeks, Priority 3)
4. **Phase 8.4**: Production Audio (2-3 weeks, Priority 4, depends on 8.1)

**Timeline**: 12-16 weeks total (Weeks 1-12 parallel work, Weeks 13-16 integration & polish)

**Success Criteria**: Ship Veilweaver Demo Level (5-10 min gameplay) with AAA-quality visuals, audio, UI, and save/load

---

## Phase 8 Gantt Chart (Week-by-Week)

```
Week    1    2    3    4    5    6    7    8    9   10   11   12   13   14   15   16
        |----|----|----|----|----|----|----|----|----|----|----|----|----|----|----|----|
8.1 UI  [===============================================]
        [Core][Sett][HUD ][Mini][Poli]
                                        
8.2 Ren      [===========================================]
             [Shad][Post][Sky ][Ligh][Part]
                                        
8.3 Save          [===================]
                  [ECS ][Slot][Versi]
                                        
8.4 Aud                     [==================]
                            [Mixr][Musc][Occl]
                                        
Integr                                                   [=======================]
                                                         [Veil][Test][Poli][Ship]
```

**Legend**:
- `[====]` = Active development
- Weeks 1-12: Parallel development
- Weeks 13-16: Integration, testing, polish

---

## Month 1 (Weeks 1-4): Foundations

### Week 1: UI Core + Rendering Shadows

**Phase 8.1 (UI)**: Week 1 - Core Infrastructure
- Tasks: egui-wgpu integration, main menu, pause menu
- Deliverable: Basic menu navigation working
- Team: 1 FTE

**Phase 8.2 (Rendering)**: Week 1 - Shadow Maps
- Tasks: Validate CSM, enable shadows, PCF filtering
- Deliverable: Directional shadows working
- Team: 1 FTE (can be same person, different crate)

**Dependencies**: NONE (both can start immediately in parallel)

**Integration Point**: None (independent work)

---

### Week 2: UI Settings + Rendering Post-FX

**Phase 8.1 (UI)**: Week 2 - Settings Menu
- Tasks: Audio settings, graphics settings, control settings
- Deliverable: Settings menu with volume sliders
- Team: 1 FTE

**Phase 8.2 (Rendering)**: Week 2 - Post-Processing
- Tasks: Enable post-FX, bloom, tonemapping (ACES)
- Deliverable: HDR → LDR pipeline working
- Team: 1 FTE

**Dependencies**: 
- Phase 8.1 Week 2 → Phase 8.4 Week 1 (mixer needs UI)

**Integration Point**: 
- UI settings menu needs placeholder for audio mixer (Phase 8.4)
- Create stub: "Audio settings (coming soon)"

---

### Week 3: UI HUD + Rendering Skybox + Save ECS

**Phase 8.1 (UI)**: Week 3 - HUD Foundation
- Tasks: Health bars, objectives, ammo counter
- Deliverable: Veilweaver HUD visible
- Team: 1 FTE

**Phase 8.2 (Rendering)**: Week 3 - Skybox
- Tasks: Cubemap rendering, atmospheric scattering
- Deliverable: Realistic sky with day/night cycle
- Team: 1 FTE

**Phase 8.3 (Save/Load)**: Week 1 - ECS Serialization (START)
- Tasks: Component derives, archetype serialization
- Deliverable: World saves to disk
- Team: 1 FTE (NEW resource, or shared)

**Dependencies**: NONE (all independent)

**Integration Point**: None (parallel work)

---

### Week 4: UI Minimap + Rendering Lights + Save Slots

**Phase 8.1 (UI)**: Week 4 - Minimap & Subtitles
- Tasks: 2D minimap, dialogue subtitles
- Deliverable: Minimap shows player/enemies
- Team: 1 FTE

**Phase 8.2 (Rendering)**: Week 4 - Dynamic Lights
- Tasks: Point light shadows, spot light shadows
- Deliverable: 16+ lights render correctly
- Team: 1 FTE

**Phase 8.3 (Save/Load)**: Week 2 - Save Slots
- Tasks: PlayerProfile, save slot management
- Deliverable: Save/load from menu
- Team: 1 FTE

**Dependencies**:
- Phase 8.1 Week 4 → Phase 8.3 Week 2 (save menu needs UI)

**Integration Point**:
- UI needs save/load menu (integrate SaveManager API)
- Create placeholder: "Save/Load (in progress)"

---

## Month 2 (Weeks 5-8): Feature Completion

### Week 5: UI Polish + Rendering Particles + Save Versioning

**Phase 8.1 (UI)**: Week 5 - Polish & Accessibility
- Tasks: Animations, controller support, accessibility
- Deliverable: **Phase 8.1 COMPLETE**
- Team: 1 FTE

**Phase 8.2 (Rendering)**: Week 5 - Particle System
- Tasks: GPU simulation, billboard rendering
- Deliverable: Fire/smoke/magic particles working
- Team: 1 FTE

**Phase 8.3 (Save/Load)**: Week 3 - Versioning & Replay
- Tasks: Save versioning, migration, replay
- Deliverable: **Phase 8.3 COMPLETE**
- Team: 1 FTE

**Dependencies**:
- Phase 8.1 COMPLETE → Phase 8.4 can start

**Integration Point**:
- **CRITICAL**: Phase 8.1 UI framework now available for Phase 8.4 mixer panel
- Phase 8.2 particles need Phase 8.1 HUD overlay (fire effects in HUD)

---

### Week 6: Audio Mixer + Rendering Complete

**Phase 8.2 (Rendering)**: Week 5+ (Optional Volumetric)
- Tasks: Height fog, god rays (optional)
- Deliverable: **Phase 8.2 COMPLETE** (with or without volumetric)
- Team: 1 FTE

**Phase 8.4 (Audio)**: Week 1 - Audio Mixer (START)
- Tasks: Bus architecture, mixer UI, snapshots
- Deliverable: Mixer working in editor
- Team: 1 FTE (reuse Phase 8.3 resource)

**Dependencies**:
- Phase 8.4 BLOCKED until Phase 8.1 Week 2+ (UI settings menu exists)

**Integration Point**:
- Audio mixer panel added to editor (uses Phase 8.1 egui framework)
- In-game audio settings integrate with Phase 8.1 settings menu

---

### Week 7: Audio Music Layers

**Phase 8.4 (Audio)**: Week 2 - Dynamic Music
- Tasks: Music layers, adaptive music, stingers
- Deliverable: Music adapts to gameplay
- Team: 1 FTE

**Dependencies**: NONE

**Integration Point**:
- Music system needs game state events (combat start, enemy spotted)
- Create event system or use existing ECS events

---

### Week 8: Audio Occlusion & Reverb

**Phase 8.4 (Audio)**: Week 3 - Occlusion & Reverb
- Tasks: Raycast occlusion, reverb zones
- Deliverable: **Phase 8.4 COMPLETE**
- Team: 1 FTE

**Dependencies**: NONE

**Integration Point**:
- Reverb zones need Phase 8.2 rendering (visualize zones in editor)
- Occlusion needs Phase 8.2 physics (raycast against geometry)

---

## Month 3 (Weeks 9-12): Integration Testing

### Week 9-10: Veilweaver Demo Level Integration

**Goal**: Integrate all Phase 8 features into Veilweaver Demo Level

**Tasks**:
1. **UI Integration** (2 days):
   - Main menu → New Game / Load Game / Settings / Quit
   - In-game HUD: Health, ammo, objectives, minimap
   - Pause menu: Resume / Save / Settings / Quit
   - Validate: Full UI workflow (start → play → save → quit → load)

2. **Rendering Integration** (2 days):
   - Shadows: Directional + point lights in all scenes
   - Post-FX: Bloom on magic effects, tonemapping for HDR
   - Skybox: Day/night cycle based on time
   - Particles: Fire, smoke, magic sparkles
   - Validate: AAA visual quality

3. **Save/Load Integration** (2 days):
   - Save slots: 3 slots with screenshots
   - Auto-save: On checkpoint, level transition
   - Load: From main menu and pause menu
   - Validate: Full game state preserved

4. **Audio Integration** (2 days):
   - Mixer: Master/Music/SFX/Voice/Ambient buses
   - Music: Exploration theme with combat layers
   - SFX: Footsteps, combat, UI sounds with occlusion
   - Reverb: Cave zones, hall zones
   - Validate: Immersive audio experience

5. **Cross-Feature Integration** (2 days):
   - Settings persist: Save audio/graphics settings to PlayerProfile
   - UI shows state: Health bar updates from game state
   - Music responds: Combat music when enemies nearby
   - Particles in HUD: Magic effects overlay on HUD
   - Validate: All systems work together

**Team**: 2 FTE (1 gameplay, 1 integration testing)

**Deliverable**: Playable Veilweaver Demo Level with all Phase 8 features

---

### Week 11-12: Testing & Bug Fixes

**Goal**: Comprehensive testing and bug fixing

**Testing Categories**:

1. **Functional Testing** (3 days):
   - Test: All UI menus (navigation, settings, save/load)
   - Test: All rendering features (shadows, post-FX, skybox, particles)
   - Test: Save/load (save mid-combat, load, verify state)
   - Test: Audio (mixer, music, occlusion, reverb)
   - Validate: No crashes, no data loss

2. **Performance Testing** (2 days):
   - Tracy profiling: Measure all systems (UI, rendering, audio, save/load)
   - Target: 60 FPS @ 1080p with all features enabled
   - Budget: Rendering <8ms, UI <2ms, Audio <3ms, Gameplay <3ms
   - Optimize: Hot spots identified by Tracy

3. **Stress Testing** (2 days):
   - Test: 100 save/load cycles (memory leak detection)
   - Test: 16 point lights + shadows (rendering stress)
   - Test: 100 audio emitters with occlusion (audio stress)
   - Test: Complex UI interactions (spam pause/unpause)
   - Validate: No leaks, no crashes

4. **Regression Testing** (2 days):
   - Test: All examples still work (hello_companion, unified_showcase)
   - Test: Phase 7 AI still works (LLM integration, behavior trees)
   - Test: Previous features not broken (ECS, physics, nav)
   - Validate: No regressions

5. **Bug Fixing** (1 day buffer):
   - Fix: Critical bugs (crashes, data loss)
   - Fix: High priority bugs (visual glitches, audio pops)
   - Defer: Low priority bugs to Phase 9

**Team**: 2 FTE (1 testing, 1 fixing)

**Deliverable**: Bug-free Veilweaver Demo Level

---

## Month 4 (Weeks 13-16): Polish & Robustness (OPTIONAL)

**Note**: Weeks 13-16 are OPTIONAL quality improvements. Can ship after Week 12 if time-constrained.

### Week 13: Code Quality & Robustness

**Goal**: Address technical debt identified in strategic analysis

**Tasks**:
1. **Fix `.unwrap()` Calls** (2 days):
   - Audit: 50+ unwraps in production code (from COMPREHENSIVE_STRATEGIC_ANALYSIS.md)
   - Fix: Replace with proper error handling (Result, Option, anyhow)
   - Priority: astraweave-ecs (20 unwraps), astraweave-render (8 unwraps)
   - Validate: Zero unwraps in production paths

2. **Complete `todo!()` Items** (1 day):
   - Find: 2 todos in advertised features
   - Complete: skinning_gpu.rs:242, combat physics (if blocking)
   - Validate: Zero todos in Phase 8 code

3. **Expand Test Coverage** (2 days):
   - Current: ~30% coverage (from strategic analysis)
   - Target: 50%+ coverage (defer 70%+ to Phase 9)
   - Focus: UI tests, rendering tests, save/load tests, audio tests
   - Validate: Critical paths covered

**Team**: 1 FTE

**Deliverable**: Codebase with zero unwraps, zero todos, 50%+ coverage

---

### Week 14: Performance Profiling & Optimization

**Goal**: Establish performance baselines for Phase 9 optimization

**Tasks**:
1. **Tracy Profiling** (2 days):
   - Profile: All Phase 8 systems (UI, rendering, audio, save/load)
   - Capture: 1,000 frames of gameplay (identify patterns)
   - Analyze: Hot spots, frame spikes, memory allocations
   - Document: Baseline metrics in PHASE_8_PERFORMANCE.md

2. **Stress Testing** (2 days):
   - Test: 10,000 entities with all Phase 8 features
   - Test: 100 save/load cycles (leak detection)
   - Test: 1 hour gameplay session (stability)
   - Validate: No leaks, no crashes, stable frame rate

3. **Optimization Pass** (1 day):
   - Fix: Only critical hot spots (>5ms)
   - Defer: Minor optimizations to Phase 9
   - Validate: 60 FPS maintained

**Team**: 1 FTE

**Deliverable**: Performance baselines established, critical hot spots fixed

---

### Week 15: Documentation & Examples

**Goal**: Comprehensive documentation for Phase 8 features

**Tasks**:
1. **API Documentation** (2 days):
   - Document: All Phase 8 APIs (UI, rendering, save/load, audio)
   - Files: SAVE_LOAD_API.md, AUDIO_MIXER_API.md, etc.
   - Code docs: Rustdoc for all public APIs
   - Validate: 100% public API documented

2. **Tutorial Examples** (2 days):
   - Create: save_load_demo (standalone save/load example)
   - Create: audio_mixer_demo (standalone mixer example)
   - Enhance: unified_showcase (all Phase 8 features)
   - Validate: Examples compile and run

3. **User Guide** (1 day):
   - Create: VEILWEAVER_USER_GUIDE.md (how to play demo)
   - Create: PHASE_8_FEATURES.md (what's new)
   - Create: KNOWN_ISSUES.md (limitations, workarounds)
   - Validate: Clear and actionable

**Team**: 1 FTE

**Deliverable**: Comprehensive documentation

---

### Week 16: Final Polish & Release Prep

**Goal**: Prepare for Veilweaver Demo Level release

**Tasks**:
1. **Visual Polish** (2 days):
   - Fix: Any visual glitches (z-fighting, texture seams)
   - Polish: UI animations, transitions
   - Polish: Particle effects, lighting
   - Screenshot: Marketing screenshots

2. **Audio Polish** (1 day):
   - Fix: Audio pops, clicks, glitches
   - Polish: Music transitions, reverb blending
   - Polish: Sound mix (balance volumes)
   - Validate: Professional audio quality

3. **Build & Packaging** (1 day):
   - Build: Release build with all optimizations
   - Package: Zip file with executable + assets
   - Test: Fresh install on clean Windows machine
   - Validate: Runs out-of-box

4. **Release Notes** (1 day):
   - Write: VEILWEAVER_RELEASE_NOTES.md
   - Write: CHANGELOG.md update
   - Write: README.md update (Phase 8 complete)
   - Validate: Clear and professional

**Team**: 1 FTE

**Deliverable**: Veilweaver Demo Level ready for release

---

## Dependencies & Critical Path

### Dependency Graph

```
Phase 8.1 (UI)
  └─> Phase 8.4 Week 1 (Audio mixer needs UI)
  └─> Phase 8.3 Week 2 (Save menu needs UI)

Phase 8.2 (Rendering)
  └─> Phase 8.4 Week 3 (Reverb zones need editor visualization)
  
Phase 8.3 (Save/Load)
  └─> Integration Week 9-10 (Save/load needed for demo)

Phase 8.4 (Audio)
  └─> Integration Week 9-10 (Audio needed for demo)
```

### Critical Path (Longest Chain)

```
Week 1-5: Phase 8.1 (UI) [5 weeks]
  ↓
Week 6-8: Phase 8.4 (Audio) [3 weeks] (blocks on UI complete)
  ↓
Week 9-10: Integration [2 weeks] (blocks on all 4 phases)
  ↓
Week 11-12: Testing [2 weeks]
  ↓
Total: 12 weeks minimum
```

**Critical Path Duration**: 12 weeks (3 months)

**If Optional Weeks 13-16**: 16 weeks (4 months)

---

## Resource Allocation

### Team Size Options

**Option 1: 1 FTE (Sequential)**
- Phase 8.1: 5 weeks
- Phase 8.2: 5 weeks (after 8.1)
- Phase 8.3: 3 weeks (after 8.2)
- Phase 8.4: 3 weeks (after 8.3, but needs 8.1 UI)
- Integration: 4 weeks
- **Total**: 20 weeks (5 months) ← TOO SLOW

**Option 2: 2 FTE (Parallel)**
- FTE 1: Phase 8.1 (5 weeks) → Phase 8.4 (3 weeks) → Integration (2 weeks)
- FTE 2: Phase 8.2 (5 weeks) → Phase 8.3 (3 weeks) → Testing (2 weeks)
- **Total**: 12 weeks (3 months) ← RECOMMENDED

**Option 3: 1 FTE (AI Copilot, Iterative)**
- Weeks 1-5: Focus on Phase 8.1 (UI)
- Weeks 3-7: Start Phase 8.2 (Rendering) in parallel (different crate)
- Weeks 3-5: Start Phase 8.3 (Save/Load) in parallel (different crate)
- Weeks 6-8: Focus on Phase 8.4 (Audio) after UI done
- Weeks 9-12: Integration & testing
- **Total**: 12 weeks with careful task switching ← CURRENT REALITY

---

## Risk Management

### High Risks

**Risk 1: UI Framework Delays** (CRITICAL PATH)
- **Impact**: Blocks Phase 8.4 (audio mixer), delays integration
- **Mitigation**: Phase 8.1 is Priority 1, start immediately
- **Contingency**: Defer UI polish (Week 5) to integration phase

**Risk 2: Feature Scope Creep**
- **Impact**: Each phase could expand beyond estimate
- **Mitigation**: Strict scope control, defer optional features to Phase 9
- **Contingency**: Week 16 is buffer for overruns

**Risk 3: Integration Issues**
- **Impact**: Systems don't work together (e.g., UI + audio + save/load)
- **Mitigation**: Early integration testing (Week 9-10)
- **Contingency**: Week 11-12 dedicated to bug fixing

### Medium Risks

**Risk 4: Performance Degradation**
- **Impact**: All Phase 8 features together cause <60 FPS
- **Mitigation**: Tracy profiling, performance budgets
- **Contingency**: Week 14 for optimization

**Risk 5: Save/Load Complexity**
- **Impact**: Serialization harder than expected (non-serializable components)
- **Mitigation**: Use `#[serde(skip)]`, rebuild transient state
- **Contingency**: Simplify to save only critical state (not full ECS)

---

## Success Metrics

### Phase 8 Success Criteria (Minimum Viable)

**Functional**:
- ✅ Can create in-game menus (main, pause, settings)
- ✅ Can create HUD (health, ammo, objectives, minimap)
- ✅ Can render AAA-quality visuals (shadows, post-FX, skybox, particles)
- ✅ Can save/load game state (player profile, world state, 3+ save slots)
- ✅ Can mix audio (4+ buses, dynamic music, occlusion, reverb)

**Performance**:
- ✅ 60 FPS @ 1080p with all features enabled
- ✅ Save/load <500ms total
- ✅ Zero crashes, zero data loss

**Quality**:
- ✅ Zero `.unwrap()` in production paths
- ✅ Zero `todo!()` in Phase 8 code
- ✅ 50%+ test coverage (defer 70%+ to Phase 9)

**Deliverable**:
- ✅ **Veilweaver Demo Level** (5-10 min gameplay loop)
  - Start: Main menu → New Game
  - Play: Combat encounter with AI companions
  - Save: Mid-combat save to slot 1
  - Quit: To main menu
  - Load: Load from slot 1, resume combat
  - Victory: Defeat enemies, see victory screen
  - Quit: Return to main menu

---

## Phase 8 Success Criteria (Stretch Goals)

**Functional**:
- ✅ Volumetric fog + god rays (Phase 8.2 optional)
- ✅ SSAO (Phase 8.2 optional, defer if >3ms)
- ✅ Cloud saves (Phase 8.3 optional, defer to Phase 9)
- ✅ Full reverb with DSP (Phase 8.4 optional, simple echo acceptable)

**Performance**:
- ✅ 120 FPS @ 1080p (high-end hardware)
- ✅ 60 FPS @ 1440p (medium-high hardware)

**Quality**:
- ✅ Zero warnings in production code
- ✅ 70%+ test coverage
- ✅ 100% API documentation

---

## Deliverables Checklist

### Code (All Phases)

- [ ] Phase 8.1: In-Game UI Framework (5 weeks)
  - [ ] egui-wgpu integration
  - [ ] Main menu, pause menu, settings menu
  - [ ] HUD (health, ammo, objectives, minimap, subtitles)
  - [ ] Animations, controller support, accessibility

- [ ] Phase 8.2: Complete Rendering Pipeline (4-5 weeks)
  - [ ] Shadow mapping (CSM, point, spot)
  - [ ] Post-processing (bloom, tonemapping, optional SSAO)
  - [ ] Skybox (cubemap, atmospheric scattering, day/night)
  - [ ] Dynamic lights (16 point + 8 spot)
  - [ ] Particle system (GPU-accelerated, 10,000+ particles)
  - [ ] Optional: Volumetric fog + god rays

- [ ] Phase 8.3: Save/Load System (2-3 weeks)
  - [ ] ECS world serialization (all components)
  - [ ] Player profile (settings, stats, unlocks, inventory)
  - [ ] Save slot management (3-10 slots, metadata, thumbnails)
  - [ ] Save versioning & migration
  - [ ] Corruption recovery & backups
  - [ ] Replay system (deterministic)

- [ ] Phase 8.4: Production Audio (2-3 weeks)
  - [ ] Audio mixer (4+ buses, hierarchy)
  - [ ] Mixer UI (editor panel, in-game settings)
  - [ ] Dynamic music (4+ layers, adaptive)
  - [ ] Audio occlusion (raycast-based)
  - [ ] Reverb zones (5+ types)
  - [ ] In-editor audio tools

### Documentation

- [ ] Individual phase plans (8.1, 8.2, 8.3, 8.4) ✅ COMPLETE
- [ ] Master integration plan ✅ THIS DOCUMENT
- [ ] API documentation (save/load, audio mixer, rendering)
- [ ] User guide (Veilweaver Demo Level)
- [ ] Known issues & limitations
- [ ] Performance baselines (PHASE_8_PERFORMANCE.md)

### Examples & Demos

- [ ] hello_companion: All Phase 8 features integrated
- [ ] unified_showcase: All Phase 8 features demonstrated
- [ ] Veilweaver Demo Level: 5-10 min playable game
- [ ] save_load_demo: Standalone save/load example
- [ ] audio_mixer_demo: Standalone mixer example

### Tests

- [ ] Unit tests: All Phase 8 systems (50%+ coverage)
- [ ] Integration tests: Cross-system validation
- [ ] Performance tests: Tracy profiling baselines
- [ ] Stress tests: 10,000 entities, 100 saves, 1 hour gameplay
- [ ] Regression tests: Previous features still work

---

## Timeline Summary Table

| Phase | Weeks | Start | End | Blocking Dependency |
|-------|-------|-------|-----|---------------------|
| 8.1 UI | 5 | Week 1 | Week 5 | NONE (start immediately) |
| 8.2 Rendering | 5 | Week 1 | Week 6 | NONE (parallel with 8.1) |
| 8.3 Save/Load | 3 | Week 3 | Week 5 | 8.1 Week 2 (UI for save menu) |
| 8.4 Audio | 3 | Week 6 | Week 8 | 8.1 Week 2 (UI for mixer) |
| Integration | 2 | Week 9 | Week 10 | ALL phases complete |
| Testing | 2 | Week 11 | Week 12 | Integration complete |
| **OPTIONAL** | | | | |
| Robustness | 1 | Week 13 | Week 13 | Testing complete |
| Profiling | 1 | Week 14 | Week 14 | Robustness complete |
| Documentation | 1 | Week 15 | Week 15 | Profiling complete |
| Polish | 1 | Week 16 | Week 16 | Documentation complete |

**Minimum Timeline**: 12 weeks (Weeks 1-12)  
**Recommended Timeline**: 14 weeks (Weeks 1-14, includes robustness + profiling)  
**Full Timeline**: 16 weeks (Weeks 1-16, includes all optional polish)

---

## Next Steps (Immediate Actions)

1. ✅ **Read all Phase 8 plans**: Understand scope, timeline, dependencies
2. ✅ **Update copilot-instructions.md**: Add all Phase 8 plan references
3. ✅ **Create todo list**: Track all Phase 8 tasks (7 major tasks identified)
4. 🎯 **Begin Phase 8.1 Week 1**: Create `astraweave-ui` crate, start UI framework
5. 📅 **Schedule check-ins**: Weekly progress reviews (Weeks 1, 2, 3, etc.)
6. 📊 **Setup Tracy profiling**: Enable profiling from start (baseline metrics)

---

## Conclusion

Phase 8 is **achievable in 12-16 weeks** with careful planning and parallel execution. The critical path is:

1. **Phase 8.1 (UI)**: 5 weeks ← MUST START IMMEDIATELY
2. **Phase 8.4 (Audio)**: 3 weeks ← Blocked by UI
3. **Integration**: 2 weeks ← Blocked by all phases
4. **Testing**: 2 weeks ← Blocked by integration
5. **Total**: 12 weeks minimum, 16 weeks with polish

**Success depends on**:
- Starting Phase 8.1 immediately (critical path)
- Running Phase 8.2 and 8.3 in parallel (different crates)
- Strict scope control (defer optional features)
- Early integration testing (Week 9-10)

**Recommendation**: Aim for 14-week timeline (includes robustness + profiling, skip documentation/polish if time-constrained). This delivers a Veilweaver Demo Level in **3.5 months**.

**After Phase 8**: Proceed to Phase 9 (Distribution & Optimization) for asset pipeline, build packaging, and final polish before public release.

---

**Document Status**: Master integration plan complete, ready for execution  
**Last Updated**: October 14, 2025  
**Next Action**: Update copilot-instructions.md, then begin Phase 8.1 implementation
