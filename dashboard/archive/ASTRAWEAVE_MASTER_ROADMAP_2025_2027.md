# AstraWeave Master Roadmap 2025-2027
## Comprehensive Long-Horizon Strategic Plan

**Document Version**: 2.0 (Enhanced Validation & Quality Gates)  
**Date**: October 16, 2025  
**Scope**: 24-month transformation roadmap  
**Objective**: Transform AstraWeave from "production-ready infrastructure" to "industry-leading AI-native game engine"

**Version 2.0 Enhancements**:
- ✅ Detailed performance regression testing strategy
- ✅ Per-phase integration testing gates
- ✅ Comprehensive quality gates checklists
- ✅ Phase 10 decision criteria (proceed/skip triggers)
- ✅ Realistic LLM success targets (70% vs 95%)
- ✅ Measurement & observability requirements
- ✅ Enhanced Phase 0/8/9 exit criteria

---

## Executive Summary

### Current State (October 2025)

**Phase 8.1 Week 4 Day 3 Status**: ✅ **72% Complete** (18 of 25 days)
- **UI Framework**: Menus complete, HUD 60% done, Week 4 Day 4 (minimap improvements) next
- **Zero-Warning Streak**: Day 18 (Oct 14-31, 2025)
- **LOC Delivered**: 3,573 lines across 18 days
- **Quality**: 0 errors, 0 warnings, 100% compilation success

**What We Have** ✅:
- **AI-Native Architecture**: 12,700+ agents @ 60 FPS, 100% deterministic
- **Hermes 2 Pro Integration**: GOAP+LLM Hybrid Arbiter (Phase 7 complete)
- **Advanced Rendering**: PBR with IBL, BC7/BC5 textures, GPU skinning, mesh optimization
- **Comprehensive Tooling**: Editor (14 panels), asset CLI, Tracy profiling
- **Phase 8 In Progress**: UI framework 72% done, rendering/save/audio planned

**Critical Gaps Identified** ⚠️:
1. **Foundation Robustness**: 50+ `.unwrap()` calls, 2 `unimplemented!()` in core systems
2. **Phase 8 Completion**: 28% UI remaining, rendering/save/audio not started
3. **Build Pipeline**: No asset packing, installers, or platform SDK integration
4. **Networking**: No multiplayer support (optional but high-value)
5. **Advanced Features**: Limited LLM success rate (40-50%), no RAG/context management
6. **Production Readiness**: Limited integration tests, missing performance baselines

### Transformation Vision (24 Months)

**From**: "Excellent infrastructure with gaps"  
**To**: "Industry-leading AI-native game engine where developers ship AAA-quality games"

**3-Track Parallel Strategy**:
1. **Track A - Foundation Hardening** (Months 1-6): Fix robustness issues, complete Phase 8
2. **Track B - Distribution & Scale** (Months 7-12): Build pipeline, optimization, multiplayer
3. **Track C - Advanced AI & Polish** (Months 13-24): LLM improvements, console support, GI rendering

---

## Roadmap Overview

### Phase Timeline (24 Months)

```
Month    1    2    3    4    5    6    7    8    9   10   11   12   13-18  19-24
         |----|----|----|----|----|----|----|----|----|----|----|----|------|------|
Phase 0  [===]                                        Foundation Hardening (CRITICAL)
Phase 8       [===================]                   Core Game Loop (UI/Render/Save/Audio)
Phase 9                          [===================] Distribution & Polish
Phase 10                                          [========================] Multiplayer & Advanced (OPTIONAL)
Phase 11                                                      [=======================] AI Excellence & Consoles

Legend: [===] Active Development
```

**Phased Approach**:
- **Phase 0** (Month 1): Foundation Hardening - Fix critical blockers
- **Phase 8** (Months 2-4): Core Game Loop - Complete UI, rendering, save/load, audio
- **Phase 9** (Months 5-7): Distribution & Polish - Build pipeline, asset optimization
- **Phase 10** (Months 8-12): Multiplayer & Advanced - Networking, GI, advanced rendering
- **Phase 11** (Months 13-24): AI Excellence & Consoles - LLM improvements, console ports

---

## Phase 0: Foundation Hardening (Month 1) 🔴 CRITICAL

**Timeline**: Weeks 1-4 (November 2025)  
**Dependencies**: NONE - Can start immediately  
**Priority**: CRITICAL (blocks all other work)  
**Effort**: 1-2 FTE

### Objective

**Eliminate critical blockers** that make the difference between "compiles cleanly" and "production-ready". These issues must be resolved before optimization or feature work.

### Critical Blockers Identified

#### CB-1: Pervasive `.unwrap()` Usage
**Severity**: 🔴 Critical  
**Scope**: 50+ instances across core, rendering, LLM, tools  
**Impact**: Production deployments will panic instead of degrading gracefully

**Current State**:
```rust
// Examples from unwrap_audit_report.csv:
astraweave-ecs/src/*.rs: 20 instances (core query operations)
astraweave-llm: 13 instances (test and production code)
astraweave-render: 8 instances (IBL, voxelization)
tools/aw_asset_cli: 11 instances (texture baking)
examples/unified_showcase: 9 instances (surface acquisition)
```

**Solution Path**:
- **Week 1**: Audit all `.unwrap()` calls, categorize by risk (P0/P1/P2)
- **Week 2-3**: Replace with `Result<>` in core systems (ECS, physics, nav)
- **Week 4**: Replace in rendering/LLM, add fallback strategies

**Acceptance Criteria**:
- ✅ Zero `.unwrap()` in core crates (ecs, ai, physics, nav)
- ✅ <5 `.unwrap()` in rendering (only in initialization, guarded by validation)
- ✅ Tools use `anyhow::Result` with user-friendly error messages

**Reference**: [IMMEDIATE_ACTIONS_IMPLEMENTATION_PLAN.md](../IMMEDIATE_ACTIONS_IMPLEMENTATION_PLAN.md) - Action 3

---

#### CB-2: Incomplete Feature Implementations
**Severity**: 🔴 Critical  
**Scope**: 2 confirmed `todo!()` / `unimplemented!()` in production crates  
**Impact**: Advertised features are non-functional

**Issues**:
1. **GPU Skinning Pipeline Descriptor** (`astraweave-render/src/skinning_gpu.rs:242`)
   - Status: `todo!("Pipeline descriptor creation - integrate with existing renderer pipelines")`
   - Impact: GPU skeletal animation non-functional
   - Blocker for: Character rendering, animation showcase

2. **Combat Attack Sweep** (`astraweave-gameplay/src/combat_physics.rs:43`)
   - Status: `unimplemented!("...due to rapier3d API changes")`
   - Impact: Melee combat system incomplete
   - Blocker for: Combat examples, gameplay validation

**Solution Path**:
- **Week 1**: GPU Skinning - Implement bind group layout, compute pipeline, test with animated mesh
- **Week 2**: Combat Physics - Migrate to Rapier3D 0.22 `ShapeCast` API, add unit tests

**Acceptance Criteria**:
- ✅ Zero `todo!()` or `unimplemented!()` in production crates
- ✅ Both features have passing unit tests
- ✅ Documentation updated to reflect actual capability

**Reference**: [IMMEDIATE_ACTIONS_IMPLEMENTATION_PLAN.md](../IMMEDIATE_ACTIONS_IMPLEMENTATION_PLAN.md) - Actions 1 & 2

---

#### CB-3: Skeletal Animation Integration Tests (0/4)
**Severity**: 🟡 High  
**Scope**: GPU vs CPU parity, determinism, scene graph, performance tests missing  
**Impact**: No validation that skeletal animation works correctly

**Missing Tests**:
1. CPU vs GPU Parity - Verify identical output
2. Determinism - Replay validation
3. Scene Graph Integration - Hierarchical transforms
4. Performance - 100+ animated characters @ 60 FPS

**Solution Path**:
- **Week 3**: Implement 4 integration tests after GPU skinning fix
- **Week 3**: Add benchmarks for animation performance

**Acceptance Criteria**:
- ✅ 4/4 integration tests passing
- ✅ Benchmark validates 100+ characters @ 60 FPS

**Reference**: [COMPREHENSIVE_STRATEGIC_ANALYSIS.md](../COMPREHENSIVE_STRATEGIC_ANALYSIS.md) - Section 2.1

---

#### CB-4: Performance Baselines Missing
**Severity**: 🟡 Medium  
**Scope**: No documented target metrics for AI planning, LLM inference, navmesh pathfinding  
**Impact**: Can't track regression or validate optimization

**Missing Benchmarks**:
- AI planning latency (perception → plan generation)
- LLM inference throughput (batch vs single)
- Navmesh pathfinding at scale (100+ agents)
- Material hot-reload performance
- Large-scale physics simulation (500+ bodies)

**Solution Path**:
- **Week 4**: Run existing benchmarks, document baselines
- **Week 4**: Create missing benchmarks for AI/LLM/navmesh

**Acceptance Criteria**:
- ✅ 10+ performance metrics documented in BASELINE_METRICS.md
- ✅ CI validates regression (>200% degradation fails build)

**Reference**: [IMMEDIATE_ACTIONS_IMPLEMENTATION_PLAN.md](../IMMEDIATE_ACTIONS_IMPLEMENTATION_PLAN.md) - Action 4

---

### Phase 0 Success Criteria

| Metric | Current | Target | Status |
|--------|---------|--------|--------|
| `.unwrap()` in core crates | 20 | 0 | ⏳ Week 2-3 |
| `todo!()` / `unimplemented!()` | 2 | 0 | ⏳ Week 1-2 |
| Integration tests (skeletal anim) | 0/4 | 4/4 | ⏳ Week 3 |
| Performance baselines | 5 | 15+ | ⏳ Week 4 |
| CI quality gates | Basic | Comprehensive | ⏳ Week 4 |

### Phase 0 Exit Criteria (Detailed)

**Code Quality (Automated Verification):**
- ✅ Zero `.unwrap()` in production paths (verified by `cargo check -p astraweave-{ecs,ai,physics,nav}`)
- ✅ Zero `todo!()` / `unimplemented!()` in advertised features (automated scan with `scripts/audit_unwrap.ps1`)
- ✅ Clippy passes with `--deny warnings` on all core crates
- ✅ All examples compile without errors (Phase1-check task passes)

**Performance Regression (Validated Against BASELINE_METRICS.md):**
- ✅ All existing benchmarks within 10% of Phase 7 baselines
- ✅ ECS world tick: <1.5 ns/entity (currently 1 ns, 50% headroom)
- ✅ GOAP planning: <110 ns (currently 101.7 ns, 8% headroom)
- ✅ Arbiter overhead: <250 ns (currently 221.9 ns, 13% headroom)
- ✅ AI core loop: <2.5 µs (currently 2.10 µs, 19% headroom)
- ✅ Physics character move: <125 ns (currently 114 ns, 10% headroom)

**Integration Testing (Skeletal Animation - 4/4 Tests):**
1. **CPU vs GPU Parity**: Output diff <0.01% for 100 frames
2. **Determinism**: Identical replay (100 frames, binary comparison)
3. **Scene Graph Integration**: Hierarchical transforms (parent-child bone chains)
4. **Performance**: 100+ animated characters @ 60 FPS (16.67 ms budget)

**CI Quality Gates (Automated Enforcement):**
- ✅ All core crates compile with zero warnings (`cargo clippy --deny warnings`)
- ✅ Production paths: No unwraps, panics, or unsafe without justification comment
- ✅ Benchmark suite runs in CI, fails on >200% regression (Week 2 Action 11 threshold)
- ✅ Phase1-check task passes (excludes known broken crates)

**Deliverable**: "Foundation-Hardened AstraWeave" - production-ready core systems

---

## Phase 8: Core Game Loop Essentials (Months 2-4)

**Timeline**: Weeks 5-16 (December 2025 - February 2026)  
**Dependencies**: Phase 0 complete  
**Priority**: CRITICAL (enables shipping games)  
**Effort**: 1-2 FTE

### Current Progress (October 16, 2025)

**Phase 8.1 (UI Framework)**: 72% complete (18 of 25 days)
- ✅ Week 1: Core menu system (main menu, pause menu)
- ✅ Week 2: Settings UI (graphics, audio, controls, persistence)
- ✅ Week 3: HUD foundation (health bars, objectives, minimap, dialogue)
- ✅ Week 4 Day 1-3: Animations (health transitions, damage numbers, notifications)
- ⏳ Week 4 Day 4-5: Minimap improvements, Week 4 validation (next)
- ⏳ Week 5: Advanced HUD polish (remaining 7 days)

**Phase 8.2 (Rendering)**: NOT STARTED (validated: shadow/post-FX infrastructure exists)
**Phase 8.3 (Save/Load)**: NOT STARTED  
**Phase 8.4 (Audio)**: NOT STARTED (depends on 8.1 UI for mixer panel)

### Remaining Work (Weeks 5-16)

#### Phase 8.1 Completion (Weeks 5-6)
**Remaining**: 7 days (Week 4 Day 4-5, Week 5)
- Week 4 Day 4: Minimap improvements (zoom, fog of war, POI icons, click-to-ping)
- Week 4 Day 5: Week 4 validation (test plan, UAT, polish)
- Week 5: Advanced HUD polish (tooltip enhancements, combo system, particle effects)

**Deliverable**: Complete in-game UI framework (5-week plan fulfilled)

**Reference**: [PHASE_8_PRIORITY_1_UI_PLAN.md](../PHASE_8_PRIORITY_1_UI_PLAN.md)

---

#### Phase 8.2: Complete Rendering Pipeline (Weeks 7-11)
**Duration**: 4-5 weeks  
**Dependencies**: Phase 8.1 complete (UI needed for rendering settings)

**Key Finding**: Roadmap review revealed existing systems more advanced than expected:
- ✅ Shadow mapping EXISTS (CSM infrastructure in renderer.rs)
- ✅ Post-processing EXISTS (post_fx_shader with tonemapping/bloom)
- ❌ Missing: Skybox, dynamic lights, particle system, volumetric fog

**Revised Timeline** (3 weeks saved from original 4-6 week estimate):

**Week 7**: Validate & Complete Shadow Maps
- Tasks: Enable CSM, test directional shadows, add PCF filtering
- Deliverable: Production-quality shadows working

**Week 8**: Complete Post-Processing
- Tasks: Enable bloom, tonemapping (ACES), optional SSAO
- Deliverable: HDR → LDR pipeline working

**Week 9**: Skybox & Atmospheric Scattering
- Tasks: Cubemap rendering, day/night cycle, atmospheric fog
- Deliverable: Realistic sky working

**Week 10**: Dynamic Lights
- Tasks: Point/spot light shadows, 16+ light support
- Deliverable: Dynamic lighting working

**Week 11**: GPU Particle System
- Tasks: Compute shader particles, 10,000+ particles @ 60 FPS
- Deliverable: Particle effects working

**Deliverable**: Complete rendering pipeline (shadows, skybox, post-FX, lights, particles)

**Reference**: [PHASE_8_PRIORITY_2_RENDERING_PLAN.md](../PHASE_8_PRIORITY_2_RENDERING_PLAN.md)

---

#### Phase 8.3: Save/Load System (Weeks 12-14)
**Duration**: 2-3 weeks  
**Dependencies**: Phase 8.1 complete (UI needed for save/load menus)

**Week 12**: ECS World Serialization
- Tasks: Component derives, archetype serialization, world saves to disk
- Deliverable: World state persisted

**Week 13**: Player Profile & Save Slots
- Tasks: PlayerProfile struct, save slot management (3-10 slots), UI integration
- Deliverable: Save/load from menu

**Week 14**: Versioning, Migration, Deterministic Replay
- Tasks: Save versioning, migration system, corruption detection, auto-backups
- Deliverable: Production-ready save system

**Deliverable**: Complete save/load system with versioning and corruption recovery

**Reference**: [PHASE_8_PRIORITY_3_SAVE_LOAD_PLAN.md](../PHASE_8_PRIORITY_3_SAVE_LOAD_PLAN.md)

---

#### Phase 8.4: Production Audio (Weeks 15-16)
**Duration**: 2-3 weeks  
**Dependencies**: Phase 8.1 complete (UI needed for mixer panel)

**Key Finding**: Existing AudioEngine has 4-bus mixer + crossfading (more advanced than roadmap suggested)

**Week 15**: Refine Mixer + UI Integration
- Tasks: Mixer panel in editor, settings menu audio controls
- Deliverable: Audio mixer working in-game

**Week 16**: Dynamic Music Layers + Audio Occlusion
- Tasks: 4+ music layers with adaptive crossfading, raycast occlusion, reverb zones (5+ types)
- Deliverable: Production audio system

**Deliverable**: Complete production audio with mixer, dynamic music, occlusion

**Reference**: [PHASE_8_PRIORITY_4_AUDIO_PLAN.md](../PHASE_8_PRIORITY_4_AUDIO_PLAN.md)

---

### Phase 8 Success Criteria

| Feature | Current | Target | Timeline |
|---------|---------|--------|----------|
| In-Game UI | 72% | 100% | Weeks 5-6 |
| Rendering Pipeline | 40% | 100% | Weeks 7-11 |
| Save/Load System | 0% | 100% | Weeks 12-14 |
| Production Audio | 30% | 100% | Weeks 15-16 |
| Example Game | No | Yes | Week 16 (Veilweaver Demo) |

### Phase 8 Performance Validation

**Frame Budget (60 FPS = 16.67 ms target, 10% headroom = 15.00 ms):**
- **UI update**: <2 ms (egui rendering + event handling, measured with Tracy)
- **Rendering**: <8 ms (shadows + post-FX + particles + mesh skinning)
- **Audio**: <1 ms (mixer + spatial audio + streaming)
- **Physics**: <3 ms (existing validated: 6.52 µs full tick, extrapolated to 1k entities)
- **AI**: <2 ms (existing validated: arbiter + GOAP + perception)
- **Total**: <16 ms with 10% headroom (15 ms actual target)

**Stress Test Scenarios (All Must Pass @ 60 FPS):**
- ✅ 1,000 entities with full Phase 8 features (UI + rendering + physics + AI + audio)
- ✅ Veilweaver Demo Level: 5-10 min sustained 60 FPS (no frame drops >33 ms)
- ✅ UI responsiveness: <16 ms input latency (measured from key press to screen update)
- ✅ Save/load performance: <5 seconds to save, <10 seconds to load
- ✅ Asset hot-reload: <500 ms to reload texture/mesh, no frame stutter

**Benchmarking Requirements (Added to CI):**
- ✅ Add `phase_8_integration_bench` - Full game loop with UI/rendering/audio
  - Measures frame time distribution (p50, p95, p99)
  - Validates 60 FPS sustained for 1,000 frames
- ✅ CI fails if frame time p95 >20 ms (20% regression threshold)
- ✅ Tracy profile captures included in Phase 8 completion report
- ✅ Performance comparison: Phase 8 vs Phase 7 baseline (Week 8: 2.70 ms)

**Deliverable**: "Ship-a-Game-Ready AstraWeave" - complete single-player game engine

**Reference**: [PHASE_8_MASTER_INTEGRATION_PLAN.md](../PHASE_8_MASTER_INTEGRATION_PLAN.md)

---

## Phase 9: Distribution & Polish (Months 5-7)

**Timeline**: Weeks 17-28 (March - May 2026)  
**Dependencies**: Phase 8 complete  
**Priority**: HIGH (enables shipping to players)  
**Effort**: 1-2 FTE

### Objective

**Enable shipping games to players** on multiple platforms with optimized assets, telemetry, and crash reporting.

### Deliverables

#### 9.1: Build & Packaging Pipeline (Weeks 17-20)
**Duration**: 3-4 weeks

**Week 17**: Asset Packing
- Tasks: Bundle assets into `.pak` archives, compressed/encrypted, indexed for fast loading
- Deliverable: Asset packing system

**Week 18**: Build Automation
- Tasks: CI/CD for release builds (Windows .exe, Linux AppImage, macOS .app), strip debug symbols
- Deliverable: Automated builds

**Week 19**: Installer Generation
- Tasks: Windows (NSIS/WiX), Linux (AppImage/Flatpak), macOS (DMG with code signing)
- Deliverable: Installers for all platforms

**Week 20**: Platform SDK Integration
- Tasks: Steamworks (achievements, cloud saves), Epic Online Services, itch.io API
- Deliverable: Platform integration

**Reference**: [GAME_ENGINE_READINESS_ROADMAP.md](../GAME_ENGINE_READINESS_ROADMAP.md) - Gap 5

---

#### 9.2: Enhanced Asset Pipeline (Weeks 21-24)
**Duration**: 3-4 weeks

**Week 21**: Texture Atlasing
- Tasks: Combine small textures into atlas, UV remapping, draw call optimization
- Deliverable: Texture atlasing system

**Week 22**: Animation Retargeting
- Tasks: Bone mapping between skeletons, IK/FK blending
- Deliverable: Animation retargeting

**Week 23**: Enhanced LOD Generation
- Tasks: Auto-decimation with quality targets, transition distances
- Deliverable: LOD system

**Week 24**: Asset Dependency Graph + Hot-Reload
- Tasks: Dependency tracking, hot-reload cascade, production hot-reload
- Deliverable: Production asset pipeline

**Reference**: [GAME_ENGINE_READINESS_ROADMAP.md](../GAME_ENGINE_READINESS_ROADMAP.md) - Gap 7

---

#### 9.3: Performance Profiling in Production (Weeks 25-28)
**Duration**: 2-3 weeks

**Week 25-26**: Lightweight Production Profiler
- Tasks: Frame time tracking, GPU timing, memory allocation tracking
- Deliverable: Production profiler

**Week 27**: Telemetry System
- Tasks: Anonymized metrics, crash dumps with stack traces, performance percentiles
- Deliverable: Telemetry system

**Week 28**: In-Game Performance Overlay
- Tasks: FPS counter, frame time graph, memory usage
- Deliverable: Performance overlay

**Reference**: [GAME_ENGINE_READINESS_ROADMAP.md](../GAME_ENGINE_READINESS_ROADMAP.md) - Gap 8

---

### Phase 9 Success Criteria

| Metric | Current | Target |
|--------|---------|--------|
| Build pipeline | 0/4 | 4/4 (packing, automation, installers, SDK) |
| Asset pipeline | 2/5 | 5/5 (atlasing, retargeting, LOD, dependency, hot-reload) |
| Profiling | 1/3 | 3/3 (production profiler, telemetry, crash reporting) |
| Shipped game | No | Yes (Veilweaver Early Access on itch.io) |

### Phase 9 Exit Criteria (Distribution Validation)

**Build Pipeline Validation (Clean Install Testing):**
- ✅ Windows .exe: Tested on clean Windows 10/11 VM (no dev tools, no Rust, no Git)
- ✅ Linux AppImage: Tested on Ubuntu 20.04/22.04 LTS (minimal install)
- ✅ macOS .app: Tested on macOS 11+ (Intel and Apple Silicon, no Xcode)
- ✅ Asset packing: <500 MB total download, loads in <30 seconds on SSD

**Platform Integration (Verified Functional):**
- ✅ itch.io upload: Downloadable by public, auto-update works (itch.io app)
- ✅ Steam (optional): Steamworks SDK integrated, achievements trigger, cloud saves work
- ✅ Telemetry: Crash dumps received from 10+ test users, anonymized metrics collected
- ✅ Installers: NSIS (Windows), AppImage (Linux), DMG (macOS) tested on clean OS

**Quality Gates (Production Readiness):**
- ✅ Crash rate: <1% per session (measured via telemetry from 10+ external playtesters)
- ✅ Load time: <30 seconds on HDD, <10 seconds on SSD (measured with stopwatch)
- ✅ Memory footprint: <2 GB RAM for Veilweaver Demo (measured with Task Manager/htop)
- ✅ Frame time: p95 <20 ms (measured via Tracy profiler on representative hardware)

**User Acceptance Testing (External Validation):**
- ✅ 10+ external playtesters complete Veilweaver Demo (5-10 min playthrough)
- ✅ Feedback collected: Controls (responsive?), performance (smooth?), bugs (showstoppers?)
- ✅ Critical bugs fixed before public release (crash on startup, save corruption, etc.)
- ✅ At least 3 positive reviews from playtesters (would recommend to friend)

**Success Metrics (Post-Launch Monitoring):**
- Download count: 50+ downloads in first week (validated via itch.io analytics)
- Crash rate: <5% of sessions (telemetry tracking)
- Average session length: >10 minutes (players engage with content)
- Completion rate: >50% of players finish demo (telemetry tracking)

**Deliverable**: "Shippable AstraWeave" - games can be distributed to players on Steam/Epic/itch.io

---

## Phase 10: Multiplayer & Advanced Rendering (Months 8-12)

**Timeline**: Weeks 29-48 (June - October 2026)  
**Dependencies**: Phase 8 + 9 complete  
**Priority**: OPTIONAL (expands market, not required for single-player)  
**Effort**: 2-3 FTE

### Objective

**Enable multiplayer games and advanced visuals** competitive with AAA engines.

### Deliverables

#### 10.1: Networking & Multiplayer (Weeks 29-36)
**Duration**: 6-8 weeks

**Week 29-30**: Networking Library Integration
- Tasks: Integrate `bevy_renet`, `laminar`, or `quinn` (QUIC), protocol design
- Deliverable: Networking layer

**Week 31-32**: Client-Server Architecture
- Tasks: Authoritative server, prediction + rollback client
- Deliverable: Client-server foundation

**Week 33-34**: Replication System
- Tasks: Delta compression, interest management (only sync nearby entities)
- Deliverable: Entity replication

**Week 35**: Matchmaking & Lobby System
- Tasks: Matchmaking logic, lobby UI
- Deliverable: Matchmaking

**Week 36**: Latency Compensation
- Tasks: Client-side prediction, server reconciliation, lag compensation
- Deliverable: Smooth multiplayer

**Reference**: [GAME_ENGINE_READINESS_ROADMAP.md](../GAME_ENGINE_READINESS_ROADMAP.md) - Gap 6

---

#### 10.2: Advanced Rendering (Weeks 37-42)
**Duration**: 4-6 weeks

**Week 37-38**: Global Illumination
- Tasks: Voxel GI or light probes, indirect lighting
- Deliverable: GI system

**Week 39-40**: Advanced Post-FX
- Tasks: DoF, motion blur, chromatic aberration
- Deliverable: Advanced post-processing

**Week 41**: Decal System
- Tasks: Projected decals, deferred rendering integration
- Deliverable: Decal system

**Week 42**: Weather Effects
- Tasks: Rain, snow, wind simulation
- Deliverable: Weather system

**Reference**: [GAME_ENGINE_READINESS_ROADMAP.md](../GAME_ENGINE_READINESS_ROADMAP.md) - Phase 10

---

#### 10.3: Advanced AI Features (Weeks 43-46)
**Duration**: 2-4 weeks

**Week 43-44**: Improved LLM Success Rates (40% → 80%+)
- Tasks: Use phi3:medium (14B), parameter defaulting, simplified Tier 2 tool set
- Deliverable: 80%+ LLM success

**Week 45**: Prompt Caching
- Tasks: 50× speedup via prompt caching
- Deliverable: Cached prompts

**Week 46**: Multi-Agent Coordination
- Tasks: Swarm tactics, cooperative strategies
- Deliverable: Squad AI

**Reference**: [GAME_ENGINE_READINESS_ROADMAP.md](../GAME_ENGINE_READINESS_ROADMAP.md) - Phase 10

---

#### 10.4: Console Support (Weeks 47-48) - OPTIONAL
**Duration**: 4-6 weeks (if pursued)

**Week 47-48**: Console Ports (if SDK licensing secured)
- Tasks: Xbox Series X/S (GDKX), PlayStation 5 (PS5 SDK), Nintendo Switch (NintendoSDK)
- Deliverable: Console builds

**Reference**: [GAME_ENGINE_READINESS_ROADMAP.md](../GAME_ENGINE_READINESS_ROADMAP.md) - Phase 10

---

### Phase 10 Success Criteria

| Metric | Current | Target |
|--------|---------|--------|
| Networking | 0/5 | 5/5 (client-server, replication, matchmaking, latency comp) |
| Advanced rendering | 0/4 | 4/4 (GI, advanced post-FX, decals, weather) |
| LLM success rate | 40-50% | 80%+ (with phi3:medium) |
| Console support | 0/3 | 1-3/3 (Xbox, PlayStation, Switch) - OPTIONAL |
| Shipped multiplayer game | No | Yes (Veilweaver 1.0 with co-op mode) |

**Deliverable**: "AAA-Ready AstraWeave" - multiplayer games with advanced graphics

---

## Phase 10 Decision Gate (Month 7 - After Phase 9)

**Timeline**: End of Week 28 (May 2026)  
**Purpose**: Decide whether to invest 5 months in multiplayer + advanced rendering  
**Resources Required**: 2-3 FTE for Phase 10 (vs 1-2 FTE for Phase 11)

### Proceed to Phase 10 if ANY of:

1. **Community Demand**: 50+ requests for multiplayer in feedback/Discord/forums
   - Evidence: Survey results, feature request upvotes, community discussions
   
2. **Revenue Target**: Veilweaver Early Access hits $10k revenue (proves market viability)
   - Evidence: itch.io/Steam sales data, demonstrates willingness to pay
   
3. **Strategic Partnership**: Studio partnership opportunity requiring multiplayer
   - Evidence: Signed LOI or contract with game studio, multiplayer is deal requirement
   
4. **Technical Validation**: Deterministic sim proves ready for networking
   - Evidence: 100% replay validation, zero desync in local multiplayer tests

### Skip Phase 10 if ANY of:

1. **Resource Constraints**: <1.5 FTE available (networking needs 2-3 FTE minimum)
   - Networking is MASSIVE (6-8 weeks baseline, likely 10-12 with delays)
   
2. **Timeline Pressure**: Phase 8-9 took >12 months (focus on polish/AI instead)
   - If foundation work exceeded estimates, multiplayer will too
   
3. **Market Feedback**: Players prioritize AI/content/polish over multiplayer
   - Evidence: Surveys show <30% interest in multiplayer, >70% want better AI/content
   
4. **Technical Blockers**: Determinism breaks or performance degrades below 30 FPS
   - Multiplayer requires rock-solid determinism + high performance

### Fallback Strategy (If Skipping Phase 10):

**Option A: Focus on Phase 11 (AI Excellence) - RECOMMENDED**
- Invest 12 months in LLM improvements, advanced AI, console support
- Differentiate on "best AI-native single-player engine" vs "me-too multiplayer"

**Option B: Community-Driven Networking**
- Open-source networking implementation, accept community contributions
- Provide architecture guidance, review PRs, integrate when ready
- Timeline: 18-24 months (slower but sustainable)

**Option C: Partner with Middleware**
- License Photon, Mirror, or Netcode for GameObjects
- Integrate via FFI (C API already exists from Phase 1)
- Cost: $1k-5k/year, saves 6-8 months development

**Option D: Defer to Post-1.0**
- Ship AstraWeave 1.0 as single-player engine (Phase 0+8+9+11)
- Add multiplayer in 2.0 based on market demand
- Focus on stability, content creation tools, community growth

### Decision Criteria Summary

| Criterion | Proceed to Phase 10 | Skip Phase 10 |
|-----------|---------------------|---------------|
| Community demand | 50+ multiplayer requests | <30 requests |
| Revenue | $10k+ from Veilweaver EA | <$5k revenue |
| Resources | 2-3 FTE available | <1.5 FTE available |
| Timeline | Phase 8-9 took <10 months | >12 months |
| Market feedback | >50% want multiplayer | <30% want multiplayer |
| Technical readiness | 100% determinism, 60 FPS | Desyncs or <30 FPS |

**Recommended Decision Process**:
1. Collect data at end of Week 28 (Phase 9 complete)
2. Survey community (Discord, itch.io, Steam forums)
3. Review revenue metrics (itch.io/Steam analytics)
4. Assess resource availability (FTE commitments for next 5 months)
5. Make go/no-go decision within 1 week
6. Communicate decision to community with rationale

---

## Phase 11: AI Excellence & Production Maturity (Months 13-24)

**Timeline**: Months 13-24 (November 2026 - October 2027)  
**Dependencies**: Phase 10 complete  
**Priority**: MEDIUM (competitive differentiation)  
**Effort**: 2-3 FTE

### Objective

**Establish AstraWeave as the industry-leading AI-native game engine** with production-grade LLM integration, advanced AI features, and full console support.

### Focus Areas (12 months)

#### 11.1: LLM Production Readiness (Months 13-15)
**Duration**: 3 months

**Month 13**: Context Management & RAG
- Tasks: Implement RAG system for long-term memory, context window management
- Deliverable: LLM with memory

**Month 14**: Evaluation Harness
- Tasks: Automated quality regression testing, LLM output validation
- Deliverable: Quality baselines

**Month 15**: Multi-Tier LLM Fallback
- Tasks: Fast tier (Phi-3 mini, 500ms) → Deep tier (Hermes 2 Pro, 13-21s) → GOAP fallback
- Deliverable: 95% fast tier success

**Reference**: [COMPREHENSIVE_STRATEGIC_ANALYSIS.md](../COMPREHENSIVE_STRATEGIC_ANALYSIS.md) - Section 1.2

---

#### 11.2: Advanced AI Features (Months 16-18)
**Duration**: 3 months

**Month 16**: Plan Caching & Streaming
- Tasks: Cache LLM plans by situation hash (50-70% hit rate), stream plans incrementally
- Deliverable: 50% latency reduction

**Month 17**: GPU GOAP Orchestration
- Tasks: Move GOAP to GPU compute shaders, batch 10,000 agents in single dispatch
- Deliverable: <100 µs for 10k agents

**Month 18**: Self-Improving AI
- Tasks: Collect telemetry on plan outcomes, fine-tune LLM prompts based on success rate
- Deliverable: 10-20% success rate improvement

**Reference**: [PHASE_7_ARBITER_PHASE_7_COMPLETE.md](../PHASE_7_ARBITER_PHASE_7_COMPLETE.md) - Future Improvements

---

#### 11.3: Console Optimization & Porting (Months 19-21)
**Duration**: 3 months (if console support pursued)

**Month 19**: Xbox Series X/S Optimization
- Tasks: GDKX integration, performance profiling, certification prep
- Deliverable: Xbox build

**Month 20**: PlayStation 5 Optimization
- Tasks: PS5 SDK integration, DualSense controller, certification prep
- Deliverable: PS5 build

**Month 21**: Nintendo Switch Optimization
- Tasks: NintendoSDK integration, performance optimization for mobile GPU
- Deliverable: Switch build

**Reference**: [GAME_ENGINE_READINESS_ROADMAP.md](../GAME_ENGINE_READINESS_ROADMAP.md) - Phase 10

---

#### 11.4: Production Maturity (Months 22-24)
**Duration**: 3 months

**Month 22**: Integration Testing
- Tasks: Cross-system validation (rendering + physics + AI), stress testing
- Deliverable: Comprehensive integration tests

**Month 23**: Content Pipeline Validation
- Tasks: Asset hot-reload stress testing, editor stability, production workflows
- Deliverable: Stable content pipeline

**Month 24**: Observability & Metrics
- Tasks: Comprehensive metrics, profiling tools, debugging infrastructure
- Deliverable: Production observability

**Reference**: [COMPREHENSIVE_STRATEGIC_ANALYSIS.md](../COMPREHENSIVE_STRATEGIC_ANALYSIS.md) - Strategic Priority Assessment

---

### Phase 11 Success Criteria (Revised - Realistic Targets)

| Metric | Current | Target | Notes |
|--------|---------|--------|-------|
| LLM parse success | 100% | 100% | Already achieved (Hermes 2 Pro) |
| LLM validation success | ~60-70% | 85%+ | Tool sandbox passes |
| LLM goal achievement | ~40-50% | 70%+ | Plan completes objective |
| System reliability | 100% | 100% | With GOAP fallback |
| Context management | None | RAG system | Long-term memory |
| GPU GOAP performance | N/A | <100 µs | 10k agents |
| Console builds | 0/3 | 1-3/3 | Xbox, PS5, Switch - OPTIONAL |
| Integration test coverage | 10% | 70%+ | Automated tests |
| Production observability | Basic | Comprehensive | Telemetry + profiling |

### Phase 11 LLM Success Metrics (Detailed Breakdown)

**Tier 1: Parse Success** (JSON Generation Quality)
- **Current**: 100% (Hermes 2 Pro validated in Phase 7)
- **Target**: Maintain 100%
- **Validation**: All LLM responses parse as valid JSON with correct schema

**Tier 2: Validation Success** (Tool Sandbox Passes)
- **Current**: ~60-70% (estimated, needs measurement in Phase 11.1)
- **Target**: 85%+ (with parameter defaulting, simplified Tier 2 tool set)
- **Validation**: Generated plan passes tool sandbox validation checks (valid tool IDs, parameter types, cooldown respect)

**Tier 3: Goal Achievement** (Plan Completes Objective)
- **Current**: ~40-50% (from Hermes 2 Pro validation reports)
- **Target**: 70%+ (with multi-tier fallback, RAG context management, prompt fine-tuning)
- **Validation**: Encounter success rate (wins combat, completes puzzle, achieves quest objective)

**Overall System Reliability** (Including Fallbacks)
- **Current**: 100% (GOAP arbiter guarantees valid action even if LLM fails)
- **Target**: Maintain 100% (multi-tier fallback prevents "AI froze" scenarios)
- **Validation**: Zero "AI unresponsive" incidents in 100 Veilweaver playthroughs

**Why 70% vs 95%?**
- Even GPT-4 doesn't achieve 95% on complex tasks without iteration
- Hermes 2 Pro (7B) is smaller model, 70% is excellent for its size
- GOAP fallback means 70% is acceptable (players won't notice failed LLM plans)
- 95% would require much larger model (70B+) or ensemble approach (costly)

**Success Criteria Adjusted:**
- ✅ LLM Tier 1 (parse): 100% (maintained from Phase 7)
- ✅ LLM Tier 2 (validation): 85%+ (up from ~60-70%)
- ✅ LLM Tier 3 (goal achievement): 70%+ (up from ~40-50%, realistic target)
- ✅ Overall system reliability: 100% (with multi-tier fallback)
- ✅ Player experience: "AI always responsive and tactically intelligent"

**Deliverable**: "Industry-Leading AstraWeave" - production-grade AI-native game engine

---

## Integration Testing Strategy (Per-Phase Gates)

**Purpose**: Validate that systems work together before moving to next phase  
**Risk Mitigation**: Catch integration issues early, avoid expensive late-stage rework  
**Timeline**: Add 1 week per phase for integration testing

### Phase 8 Integration Test (Week 16 - Before Phase 9)

**Full Game Loop Validation:**
- ✅ UI → Rendering → Physics → AI → Audio (all systems active @ 60 FPS)
- ✅ Cross-system interactions: HUD updates from game state, audio responds to events
- ✅ Save/load: Persist all Phase 8 systems, reload maintains exact state (determinism)
- ✅ Stress test: Veilweaver Demo for 30 minutes continuous play, zero crashes

**Performance Validation:**
- ✅ Frame time p95 <20 ms (60 FPS with 20% headroom)
- ✅ Memory stable (no leaks over 30 min session)
- ✅ Tracy profile shows balanced frame budget (no single system >50% frame time)

**Test Scenarios:**
1. **Gameplay Loop**: Start demo → play 5-10 min → save → load → continue → finish
2. **UI Stress**: Rapidly open/close menus, spam settings changes, verify no crashes
3. **Combat Stress**: 10+ simultaneous combatants, verify physics + AI + rendering stable
4. **Save/Load Stress**: Save every 10 seconds for 5 minutes, verify all saves valid

**Acceptance Criteria:**
- 10+ playthroughs without crash
- All saves loadable without corruption
- Performance metrics within budget
- Zero regressions from Phase 7 baseline

---

### Phase 9 Integration Test (Week 28 - Before Phase 10)

**Asset Pipeline Validation:**
- ✅ Hot-reload works for all asset types (textures, meshes, materials, audio)
- ✅ No frame stutter during hot-reload (<500 ms reload time)
- ✅ Dependency graph correctly rebuilds dependent assets

**Build Pipeline Validation:**
- ✅ Fresh install on 3 platforms (Windows 10/11, Ubuntu 22.04, macOS 11+)
- ✅ Installer launches correctly (no DLL errors, no permissions issues)
- ✅ Game runs without development tools (no Rust, no Git, no admin rights)

**Telemetry Validation:**
- ✅ Crash dumps collected from test users (10+ users submit at least 1 crash dump)
- ✅ Performance metrics recorded (frame time, memory, session length)
- ✅ Anonymization verified (no PII in telemetry data)

**Test Scenarios:**
1. **Clean Install**: Wipe VM → install from .exe/AppImage/DMG → launch → play demo
2. **Asset Hot-Reload**: Edit texture in GIMP → save → verify reload in <500 ms
3. **Telemetry**: Trigger crash → verify dump uploaded → verify stack trace visible
4. **Platform Matrix**: Test on Windows (Intel/AMD), Linux (Nvidia/AMD), macOS (Intel/M1)

**Acceptance Criteria:**
- All 3 platforms install and run without errors
- Hot-reload works for 100% of asset types
- Telemetry captures crashes from external testers
- Zero showstopper bugs in external playtest

---

### Phase 10 Integration Test (Week 48 - Before Phase 11)

**Networking Validation:**
- ✅ 4-player stress test: 100+ entities, 2-hour session, <5% desync rate
- ✅ Latency compensation: Smooth gameplay @ 100 ms ping
- ✅ Matchmaking: Find game in <30 seconds, no lobby crashes

**Advanced Rendering Validation:**
- ✅ GI + shadows + post-FX active simultaneously @ 60 FPS
- ✅ Dynamic lights: 16+ lights, 8+ casting shadows, no frame drops
- ✅ Weather effects: Rain + wind simulation, no performance degradation

**Multiplayer Stress Scenarios:**
1. **Desync Test**: 4 players, 100 entities, 1-hour session, compare final state hashes
2. **Latency Test**: Simulated 50-200 ms ping, verify smooth prediction/rollback
3. **Stress Test**: 16 players (if supported), 500+ entities, verify stable networking

**Acceptance Criteria:**
- Desync rate <5% over 10 sessions
- No networking crashes in 20+ multiplayer sessions
- Advanced rendering maintains 60 FPS target
- External playtest reports "smooth multiplayer"

---

### Phase 11 Integration Test (Month 22 - Production Maturity)

**Cross-System Validation:**
- ✅ Rendering + Physics + AI + Networking (if applicable) all active
- ✅ LLM integration: Concurrent LLM requests from 10+ agents, no deadlocks
- ✅ Console builds: Xbox/PS5/Switch (if applicable) run full game loop

**Stress Testing:**
- ✅ 1,000+ entities with full feature set @ 60 FPS
- ✅ 10-hour continuous session (memory leak detection)
- ✅ Worst-case scenarios: 100 LLM requests/sec, 500 physics bodies, 50 dynamic lights

**Test Scenarios:**
1. **Soak Test**: Run Veilweaver Demo on loop for 10 hours, monitor memory/CPU
2. **LLM Stress**: 100 agents all request LLM plans simultaneously, verify queue handling
3. **Console Validation**: Run full test suite on Xbox/PS5/Switch devkits

**Acceptance Criteria:**
- Zero crashes in 10-hour soak test
- Integration test coverage 70%+ (automated test suite)
- Console builds pass certification requirements (if applicable)

---

## Dependency Graph & Critical Path

### Critical Path Analysis

**Sequential Dependencies** (cannot parallelize):
1. **Phase 0 (Month 1)** → Phase 8 (Months 2-4)
   - Foundation must be solid before building features
2. **Phase 8.1 (UI)** → Phase 8.3 (Save/Load), Phase 8.4 (Audio)
   - Save/load menus need UI, audio mixer needs UI panel
3. **Phase 8 (Months 2-4)** → Phase 9 (Months 5-7)
   - Can't ship games without complete game loop
4. **Phase 9 (Months 5-7)** → Phase 10 (Months 8-12)
   - Multiplayer needs build pipeline and distribution

**Parallelizable Work**:
- Phase 8.2 (Rendering) can run parallel to Phase 8.3 (Save/Load)
- Phase 10.1 (Networking) can run parallel to Phase 10.2 (Advanced Rendering)
- Phase 11 work can be split across multiple teams

### Risk-Adjusted Timeline

**Best Case** (1-2 FTE, no blockers):
- Phase 0: 4 weeks
- Phase 8: 12 weeks
- Phase 9: 12 weeks
- Phase 10: 20 weeks (if multiplayer pursued)
- Phase 11: 12 months (polish & AI excellence)
- **Total**: 24 months

**Realistic Case** (1 FTE, minor blockers):
- Phase 0: 6 weeks (50% buffer)
- Phase 8: 16 weeks (33% buffer)
- Phase 9: 14 weeks (17% buffer)
- Phase 10: 24 weeks (20% buffer)
- Phase 11: 14 months (17% buffer)
- **Total**: 28 months

**Worst Case** (0.5 FTE, major blockers):
- Phase 0: 8 weeks (100% buffer)
- Phase 8: 20 weeks (67% buffer)
- Phase 9: 18 weeks (50% buffer)
- Phase 10: 30 weeks (50% buffer)
- Phase 11: 18 months (50% buffer)
- **Total**: 36 months

**Recommendation**: Plan for **28-month realistic timeline** with contingency for delays.

---

## Resource Allocation Strategy

### Team Size Recommendations

**Minimum Viable** (0.5-1 FTE):
- Phase 0: 1 FTE (foundation critical)
- Phase 8: 1 FTE (sequential work)
- Phase 9: 0.5 FTE (can be part-time)
- Phase 10: SKIP or defer to Phase 11
- Phase 11: 0.5 FTE (polish)
- **Timeline**: 36 months

**Recommended** (1-2 FTE):
- Phase 0: 1 FTE
- Phase 8: 2 FTE (UI + Rendering parallel)
- Phase 9: 1 FTE
- Phase 10: 2 FTE (Networking + Advanced Rendering parallel)
- Phase 11: 2 FTE (AI + Console parallel)
- **Timeline**: 24 months

**Optimal** (2-3 FTE):
- Phase 0: 2 FTE (parallel unwrap + feature work)
- Phase 8: 3 FTE (UI, Rendering, Save/Load parallel)
- Phase 9: 2 FTE (Build + Asset pipeline parallel)
- Phase 10: 3 FTE (Networking, Rendering, AI parallel)
- Phase 11: 3 FTE (AI, Console, Production parallel)
- **Timeline**: 18 months

---

## Performance Regression Testing (Continuous Validation)

**Purpose**: Catch performance degradation before it reaches production  
**Infrastructure**: Leverage existing Tracy, benchmark suite, CI infrastructure from Week 8  
**Timeline**: Ongoing throughout all phases

### Automated Benchmarking (CI Integration)

**PR-Level Validation:**
- ✅ Run full benchmark suite on every PR (cargo bench --workspace)
- ✅ Fail CI if any benchmark regresses >20% (existing 200% threshold from Week 2 Action 11)
- ✅ Post benchmark results as PR comment (criterion-compare integration)
- ✅ Require maintainer approval for 10-20% regressions (performance review)

**Benchmark Suite Coverage:**
- ECS core (archetype iteration, query performance, event dispatch)
- AI planning (GOAP, behavior trees, arbiter overhead)
- Physics (character controller, raycast, rigid body step, spatial hash)
- Rendering (mesh optimization, vertex compression, material binding)
- SIMD math (vector/matrix operations, movement batching)
- Full integration (Phase 8+: UI + rendering + physics + AI + audio)

**CI Infrastructure:**
- Dedicated benchmark runner (consistent hardware, no background processes)
- Historical baseline tracking (BASELINE_METRICS.md updated per phase)
- Slack/Discord notifications on regressions (team awareness)

---

### Performance Baselines (Updated Per Phase)

**Phase 0 Baseline** (Week 4 - Foundation Hardening Complete):
- ECS world tick: 1 ns/entity
- GOAP planning: 101.7 ns (cache hit), 47.2 µs (cache miss)
- Arbiter overhead: 221.9 ns (GOAP control), 575.3 ns (LLM polling)
- Physics character move: 114 ns
- SIMD movement: 9.879 µs (10k entities)

**Phase 8 Baseline** (Week 16 - Full Game Loop):
- Frame time: <15 ms (target), p95 <20 ms (with headroom)
- UI update: <2 ms
- Rendering: <8 ms
- Physics: <3 ms
- AI: <2 ms
- Audio: <1 ms

**Phase 9 Baseline** (Week 28 - Optimized Build):
- Asset loading: <30 seconds (full game), <5 seconds (level)
- Hot-reload: <500 ms (texture/mesh), <1 second (material)
- Memory footprint: <2 GB RAM (Veilweaver Demo)
- Build time: <5 minutes (release build)

**Phase 10 Baseline** (Week 48 - Networking + Advanced):
- Networking: <50 ms latency (local), <5% desync rate
- Advanced rendering: GI + shadows + post-FX @ 60 FPS
- GPU particles: 10,000+ particles <1 ms GPU time

---

### Regression Alerts (Automated Monitoring)

**Alert Triggers:**
- >20% regression in any benchmark (CI fails, requires investigation)
- >10% regression in critical path (ECS tick, GOAP planning, frame time)
- >5% regression in user-facing metrics (load time, input latency)

**Alert Channels:**
- GitHub Actions: Fail CI check, post results as PR comment
- Slack/Discord: Automated notification with benchmark comparison link
- GitHub Issues: Auto-create issue for >20% regressions (assigned to author)

**Response Process:**
1. CI fails → Author investigates regression
2. If intentional (new feature): Document why, get approval
3. If unintentional: Fix before merge or revert PR
4. If unavoidable: Update baseline after team review

---

### Tracy Profiling Integration

**Always-On Profiling (Debug Builds):**
- Tracy instrumentation in all debug builds (zero-overhead when not connected)
- Developers can connect Tracy profiler anytime for live profiling
- Automated profile capture on benchmark runs (saved as artifacts)

**Selective Profiling (Release Builds):**
- F-key toggle (e.g., F9) to enable Tracy in release builds
- Useful for production debugging or player-reported performance issues
- Minimal overhead when disabled (<1% frame time)

**Profile Capture Process:**
- Weekly: Capture Tracy profile of Veilweaver Demo (30 min session)
- Per-Phase: Capture baseline profile at phase completion (archival)
- On Regression: Capture before/after profiles for comparison (root cause analysis)

**Profile Storage:**
- GitHub Actions artifacts: Automated captures from CI
- Cloud storage (Google Drive/Dropbox): Weekly manual captures
- BASELINE_METRICS.md: Link to representative profiles per phase

---

## Quality Gates Checklist (Per-Phase Exit Criteria)

**Purpose**: Ensure each phase meets quality standards before proceeding  
**Format**: Checkbox checklists for manual validation  
**Enforcement**: Required for phase completion sign-off

### Phase 0 Exit Gate (Foundation Hardening)

**Code Quality:**
- [ ] Zero `.unwrap()` in core crates (automated scan passes)
- [ ] Zero `todo!()` / `unimplemented!()` in production paths (automated scan)
- [ ] Clippy clean with `--deny warnings` on all core crates
- [ ] All examples compile without errors (Phase1-check task)

**Performance:**
- [ ] All benchmarks within 10% of Phase 7 baseline (CI passes)
- [ ] ECS tick <1.5 ns/entity
- [ ] GOAP planning <110 ns
- [ ] Arbiter overhead <250 ns

**Testing:**
- [ ] 4/4 skeletal animation integration tests passing
- [ ] GPU skinning pipeline functional (automated test)
- [ ] Combat physics attack sweep functional (unit tests)

**Documentation:**
- [ ] BASELINE_METRICS.md updated with Phase 0 numbers
- [ ] unwrap_audit_report.csv shows zero P0-Critical issues

---

### Phase 8 Exit Gate (Core Game Loop)

**Features:**
- [ ] UI framework 100% complete (Week 5 validation)
- [ ] Rendering pipeline 100% (shadows, skybox, post-FX, lights, particles)
- [ ] Save/load system 100% (ECS serialization, save slots, versioning)
- [ ] Production audio 100% (mixer, dynamic music, occlusion)

**Performance:**
- [ ] Veilweaver Demo @ 60 FPS sustained (30 min stress test)
- [ ] Frame time p95 <20 ms (Tracy validated)
- [ ] Save time <5 seconds, load time <10 seconds
- [ ] Hot-reload <500 ms (asset pipeline)

**Integration:**
- [ ] Full game loop integration test passes (Week 16)
- [ ] UI → Rendering → Physics → AI → Audio (all working together)
- [ ] 10+ playthroughs without crash (manual testing)

**User Acceptance:**
- [ ] 5+ external playtesters complete demo (UAT)
- [ ] Feedback collected and critical bugs fixed
- [ ] At least 3 positive reviews ("would recommend")

---

### Phase 9 Exit Gate (Distribution & Polish)

**Build Pipeline:**
- [ ] Windows .exe works on clean Windows 10/11 VM
- [ ] Linux AppImage works on Ubuntu 20.04/22.04
- [ ] macOS .app works on macOS 11+ (Intel and Apple Silicon)
- [ ] Asset packing <500 MB total

**Platform Integration:**
- [ ] itch.io upload downloadable by public
- [ ] Steam integration working (optional: achievements, cloud saves)
- [ ] Telemetry captures crashes from 10+ test users

**Quality:**
- [ ] Crash rate <1% per session (telemetry validated)
- [ ] Load time <30 seconds HDD, <10 seconds SSD
- [ ] Memory footprint <2 GB RAM

**User Acceptance:**
- [ ] 10+ external playtesters complete demo
- [ ] 50+ downloads in first week (itch.io analytics)
- [ ] Completion rate >50% (telemetry tracking)

---

### Phase 10 Exit Gate (Multiplayer & Advanced)

**Networking:**
- [ ] 4-player stress test passes (2-hour session, <5% desync)
- [ ] Matchmaking finds game in <30 seconds
- [ ] Latency compensation smooth @ 100 ms ping

**Advanced Rendering:**
- [ ] GI + shadows + post-FX @ 60 FPS
- [ ] 16+ dynamic lights, 8+ casting shadows
- [ ] 10,000+ GPU particles <1 ms

**Integration:**
- [ ] Multiplayer integration test passes (Week 48)
- [ ] 20+ multiplayer sessions without crash
- [ ] External playtest reports "smooth multiplayer"

---

### Phase 11 Exit Gate (AI Excellence & Maturity)

**LLM Quality:**
- [ ] Tier 1 parse success 100% (maintained)
- [ ] Tier 2 validation success 85%+ (measured)
- [ ] Tier 3 goal achievement 70%+ (measured)
- [ ] Overall system reliability 100% (fallback works)

**Production Maturity:**
- [ ] Integration test coverage 70%+ (automated tests)
- [ ] Console builds functional (1+ platform, if applicable)
- [ ] 10-hour soak test passes (zero crashes)

**Observability:**
- [ ] Comprehensive telemetry deployed (crash dumps, metrics)
- [ ] Tracy profiling integrated (production debugging)
- [ ] Quality metrics dashboard operational

---

## Measurement & Observability Requirements

**Purpose**: Instrument for production debugging, telemetry, and continuous improvement  
**Timeline**: Phase 9+ (production telemetry), Phase 11 (comprehensive observability)

### Production Telemetry (Phase 9.3 - Week 27)

**Frame Time Metrics:**
- p50 (median), p95 (95th percentile), p99 (99th percentile)
- Per-system breakdown (UI, rendering, physics, AI, audio)
- GPU time (if available via wgpu query)

**Crash Dump Collection:**
- Stack trace with symbols (release builds with debug symbols)
- System info (OS, CPU, GPU, RAM, driver versions)
- Last 100 log entries (context for crash)
- Anonymized (no PII: username, IP, etc.)

**LLM Success Tracking:**
- Parse success rate (JSON valid?)
- Validation success rate (tool sandbox passes?)
- Goal achievement rate (encounter success?)
- Fallback trigger rate (GOAP vs LLM usage)

**Player Behavior Metrics:**
- Session length (how long do players play?)
- Completion rate (% finish demo/level)
- Death locations (where do players struggle?)
- Settings preferences (graphics quality, difficulty, etc.)

---

### Tracy Integration (Phase 9.3 - Week 28)

**Always-On Profiling (Debug Builds):**
- Tracy instrumentation compiled in (tracy 0.11.1 from Week 8)
- Zero overhead when profiler not connected
- Developers can connect anytime for live profiling

**Optional Profiling (Release Builds):**
- F-key toggle (e.g., F9) enables Tracy capture
- Minimal overhead when disabled (<1% frame time)
- Useful for production debugging or player-reported issues

**Automated Profile Capture:**
- CI: Capture profile on benchmark runs (saved as artifacts)
- Weekly: Capture 30 min Veilweaver Demo session (archival)
- On Regression: Capture before/after profiles (root cause analysis)

---

### Quality Metrics Dashboard (Phase 11.4 - Month 24)

**CI Dashboard (GitHub Actions):**
- Test pass rate (unit, integration, benchmarks)
- Benchmark trends (ECS, AI, physics, rendering)
- Code coverage (70%+ target for Phase 11)
- Clippy warnings (zero tolerance for core crates)

**Community Dashboard (Public-Facing):**
- Crash rate (per platform, per build)
- Player feedback (average rating, reviews)
- Download/install count (itch.io, Steam)
- Active users (DAU, MAU from telemetry)

**Development Dashboard (Internal):**
- LOC (lines of code) per crate
- Velocity (story points per week, if using agile)
- Bug count (open, in-progress, closed)
- Technical debt (unwraps, todos, clippy warnings)

---

## Success Metrics & Validation

### Phase-Level Success Criteria

| Phase | Duration | Success Metric | Validation |
|-------|----------|----------------|------------|
| **Phase 0** | Month 1 | 0 unwraps in core, 0 unimplemented, 4/4 tests | CI passes, manual audit |
| **Phase 8** | Months 2-4 | Ship Veilweaver Demo (5-10 min gameplay) | Playable demo, UAT |
| **Phase 9** | Months 5-7 | Ship to itch.io/Steam | Public release |
| **Phase 10** | Months 8-12 | Multiplayer demo (2-16 players) | Network testing |
| **Phase 11** | Months 13-24 | 95%+ LLM success, console builds | Quality metrics, cert |

### Long-Term Vision Metrics (24 Months)

| Metric | Current (Oct 2025) | Target (Oct 2027) | Validation |
|--------|-------------------|-------------------|------------|
| **Compilation Success** | 90% (some broken crates) | 100% (all crates) | CI passes Phase1-check |
| **Error Handling** | 50+ unwraps in core | 0 unwraps in core | Automated scan clean |
| **Test Coverage** | 30% (limited integration) | 70%+ (comprehensive) | Coverage report |
| **LLM Parse Success** | 100% (Hermes 2 Pro) | 100% (maintained) | JSON validation |
| **LLM Goal Achievement** | 40-50% | 70%+ | Encounter success rate |
| **System Reliability** | 100% (GOAP fallback) | 100% (multi-tier fallback) | Zero "AI froze" |
| **Platform Support** | Windows/Linux/macOS | +Xbox/PS5/Switch | Console builds |
| **Example Games** | 1 (unified_showcase) | 5+ (various genres) | Shipped games |
| **Community Adoption** | Internal only | 100+ external developers | Discord/forum users |
| **Shipped Games** | 0 | 10+ (community + showcase) | itch.io/Steam releases |

---

## Risk Assessment & Mitigation

### High-Risk Areas

#### Risk 1: Phase 8 Rendering Complexity
**Probability**: Medium  
**Impact**: High (can delay by 4-8 weeks)

**Mitigation**:
- Start with existing CSM/post-FX infrastructure (validated to exist)
- Defer GI to Phase 10 if timeline slips
- Use Unity/Unreal tutorials as reference implementation

**Fallback**: Ship Phase 8 without volumetric fog/advanced post-FX (defer to Phase 10)

---

#### Risk 2: Networking is Massive
**Probability**: High  
**Impact**: Critical (can consume 6+ months)

**Mitigation**:
- Make Phase 10 entirely OPTIONAL (single-player is viable market)
- Partner with networking specialists or license middleware
- Focus on deterministic lockstep (simpler than client-server)

**Fallback**: Skip multiplayer, focus on single-player excellence + AI

---

#### Risk 3: LLM Quality Plateau
**Probability**: Medium  
**Impact**: Medium (40-50% → 95% may be unrealistic)

**Mitigation**:
- Implement multi-tier fallback (fast → deep → GOAP) for reliability
- Focus on use cases where 40-50% success is acceptable (narrative, not combat)
- Fine-tune prompts based on telemetry

**Fallback**: Position LLM as "optional advanced feature", not core selling point

---

#### Risk 4: Resource Constraints (0.5-1 FTE Only)
**Probability**: Medium  
**Impact**: High (timeline extends to 36 months)

**Mitigation**:
- Prioritize Phase 0 + Phase 8 (foundation + game loop)
- Defer Phase 10 (multiplayer) to Phase 11 or beyond
- Community contributions for non-critical features

**Fallback**: Ship single-player engine in 12 months (Phase 0 + 8 + 9), defer advanced features

---

### Medium-Risk Areas

#### Risk 5: Asset Pipeline Fragmentation
**Probability**: Medium  
**Impact**: Medium (can slow content creation)

**Mitigation**:
- Implement incrementally (atlas → retarget → LOD → hot-reload)
- Test each feature before moving to next
- Fallback to manual asset optimization if needed

---

#### Risk 6: Console SDK Licensing Delays
**Probability**: High (if pursued)  
**Impact**: Medium (delays Phase 11 by 3-6 months)

**Mitigation**:
- Start SDK application process in Phase 9 (early)
- Focus on PC/mobile first (itch.io, Steam, web)
- Console support is OPTIONAL, not critical path

---

## Alternative Strategies

### Strategy A: AI-Native Plugin for Unity/Unreal

**Timeline**: 6 months (vs 24 months for full engine)  
**Scope**: Export AstraWeave AI system as plugin, partner with Unity/Unreal for rendering/UI/audio

**Pros**:
- Faster to market (6 vs 24 months)
- Leverage mature ecosystems (Unity/Unreal)
- Focus on core differentiator (AI-native architecture)

**Cons**:
- Less control over full stack
- Dependent on partner engine updates
- Harder to innovate on rendering/UI

**Recommendation**: Consider if timeline is too aggressive or resources are limited

---

### Strategy B: Hybrid Approach (AstraWeave + Godot/Bevy)

**Timeline**: 12 months (vs 24 months for full engine)  
**Scope**: Use AstraWeave for AI + simulation, Godot/Bevy for rendering + UI

**Pros**:
- Faster than building from scratch
- Open-source integration (no licensing)
- Bridge via FFI (C API already exists)

**Cons**:
- Integration complexity (two engines)
- Potential performance overhead at boundaries
- Limited control over rendering pipeline

**Recommendation**: Viable middle ground if Phase 8 proves too ambitious

---

### Strategy C: Community-Driven Development

**Timeline**: 36+ months (slower but sustainable)  
**Scope**: Open-source missing features, accept community contributions

**Pros**:
- Sustainable long-term development
- Builds community ownership
- Distributes workload

**Cons**:
- Slower progress (coordination overhead)
- Quality control challenges
- Requires active community management

**Recommendation**: Consider for Phase 10-11 (non-critical features like console support)

---

## Recommended Action Plan

### Immediate Next Steps (Week 1 - November 2025)

**Priority 1**: Start Phase 0 Foundation Hardening
1. **Day 1-2**: Audit all `.unwrap()` calls (use `unwrap_audit_report.csv` as starting point)
2. **Day 3-5**: Fix GPU skinning pipeline descriptor (`astraweave-render/src/skinning_gpu.rs:242`)
3. **Day 6-7**: Fix combat physics attack sweep (`astraweave-gameplay/src/combat_physics.rs:43`)

**Priority 2**: Complete Phase 8.1 Week 4
1. **Day 4**: Minimap improvements (zoom, fog of war, POI icons)
2. **Day 5**: Week 4 validation (test plan, UAT, polish)

**Priority 3**: Plan Phase 8.2-8.4
1. Review existing shadow/post-FX infrastructure (validate findings)
2. Create detailed task breakdowns for rendering/save/audio
3. Set up tracking for Phase 8 progress

---

### Monthly Milestones (Months 1-6)

**Month 1 (November 2025)**: Foundation Hardening Complete
- ✅ Zero `.unwrap()` in core crates
- ✅ Zero `todo!()` / `unimplemented!()`
- ✅ 4/4 skeletal animation integration tests
- ✅ 15+ performance baselines documented

**Month 2 (December 2025)**: Phase 8.1 Complete, Phase 8.2 Started
- ✅ UI framework 100% complete (5-week plan fulfilled)
- ✅ Shadow mapping validated and enabled
- ⏳ Post-processing in progress

**Month 3 (January 2026)**: Phase 8.2 Complete, Phase 8.3 Started
- ✅ Complete rendering pipeline (shadows, skybox, post-FX, lights, particles)
- ✅ ECS world serialization working
- ⏳ Save slot management in progress

**Month 4 (February 2026)**: Phase 8 Complete
- ✅ Save/load system complete
- ✅ Production audio complete
- ✅ Veilweaver Demo Level playable (5-10 min gameplay)

**Month 5-7 (March-May 2026)**: Phase 9 Complete
- ✅ Build pipeline working (asset packing, automation, installers)
- ✅ Enhanced asset pipeline (atlasing, retargeting, LOD)
- ✅ Performance profiling in production
- ✅ Veilweaver Early Access on itch.io

---

### Quarterly Reviews (Every 3 Months)

**Q1 2026 (Jan-Mar)**: Phase 8 Completion Review
- Validate: Can ship complete single-player game?
- Metrics: All Phase 8 success criteria met?
- Decision: Proceed to Phase 9 or iterate?

**Q2 2026 (Apr-Jun)**: Phase 9 Completion Review
- Validate: Can distribute game to players?
- Metrics: All Phase 9 success criteria met?
- Decision: Proceed to Phase 10 or focus on polish?

**Q3 2026 (Jul-Sep)**: Phase 10 Mid-Point Review
- Validate: Is multiplayer feasible with current resources?
- Metrics: Networking progress, LLM improvements?
- Decision: Continue Phase 10 or pivot to Phase 11?

**Q4 2026 (Oct-Dec)**: Phase 10 Completion Review
- Validate: Multiplayer working, advanced rendering complete?
- Metrics: All Phase 10 success criteria met?
- Decision: Proceed to Phase 11 or ship 1.0?

---

## Conclusion

### Summary of Transformation

**From** (October 2025):
- Excellent infrastructure with gaps
- 72% UI complete, rendering/save/audio not started
- 50+ unwraps in core, 2 unimplemented features
- No build pipeline, no multiplayer, 40-50% LLM success

**To** (October 2027):
- Industry-leading AI-native game engine
- Complete single-player + multiplayer support
- 0 unwraps in core, 100% feature complete
- Build pipeline, console support, 95%+ LLM success
- 10+ shipped games, 100+ external developers

### Total Investment

**Timeline**: 24 months (November 2025 - October 2027)  
**Resources**: 1-2 FTE recommended (0.5 FTE minimum, 3 FTE optimal)  
**Phases**: 5 phases (0, 8, 9, 10, 11)  
**Cost**: ~$200k-400k (assuming $100k/year per FTE × 2 FTE × 2 years)

### Return on Investment

**Market Opportunity**:
- **AI-Native Game Engine**: Unique differentiator (no direct competitors)
- **Indie Game Market**: $5B+ (growing 20% YoY)
- **Licensing Model**: $99-499/year per developer (Unity/Unreal model)
- **Community Adoption**: 100+ developers × $249/year = $25k annual recurring revenue

**Strategic Value**:
- **Technical Leadership**: First production-ready AI-native engine
- **Research Platform**: Validate LLM + game development thesis
- **Community**: Open-source contributors + showcase games
- **IP**: Patentable AI orchestration architecture

### Recommended Path Forward

**Option 1: Full Commitment** (1-2 FTE, 24 months, $200-400k)
- Execute all phases (0 through 11)
- Target: Industry-leading engine with multiplayer + console support
- Risk: High investment, long timeline

**Option 2: Focused Execution** (1 FTE, 12 months, $100-150k)
- Execute Phase 0, 8, 9 only (foundation + single-player + distribution)
- Target: Ship-a-game-ready engine for itch.io/Steam
- Risk: Lower, but no multiplayer or advanced features

**Option 3: Hybrid Strategy** (0.5-1 FTE, 6-12 months, $50-100k)
- Execute Phase 0 + 8, partner with Unity/Unreal for rendering/UI
- Target: AI plugin for existing engines
- Risk: Lowest investment, fastest to market, but less control

**Recommendation**: **Option 2 (Focused Execution)** for first 12 months, then re-evaluate based on community traction. If demand for multiplayer emerges, pursue Phase 10-11 in Year 2.

---

## Next Actions

1. **Approve Roadmap** - Review and approve this master plan
2. **Allocate Resources** - Commit 1-2 FTE for Phase 0 (Month 1)
3. **Start Phase 0** - Begin foundation hardening (Week 1: unwrap audit + feature fixes)
4. **Track Progress** - Weekly check-ins, monthly milestone reviews
5. **Adapt as Needed** - Quarterly strategic reviews to adjust plan

---

**Document Status**: Ready for Approval  
**Last Updated**: October 16, 2025  
**Maintainer**: AI Development Team  
**Next Review**: November 16, 2025 (after Phase 0 completion)

---

## Appendix: Related Documentation

### Strategic Plans (Reference, Don't Duplicate)
- [GAME_ENGINE_READINESS_ROADMAP.md](../GAME_ENGINE_READINESS_ROADMAP.md) - Phase 8-10 gap analysis
- [COMPREHENSIVE_STRATEGIC_ANALYSIS.md](../COMPREHENSIVE_STRATEGIC_ANALYSIS.md) - 50+ page codebase analysis
- [LONG_HORIZON_STRATEGIC_PLAN.md](../docs/root-archive/LONG_HORIZON_STRATEGIC_PLAN.md) - 12-month plan (Phases A, B, C)
- [IMPLEMENTATION_PLANS_INDEX.md](../IMPLEMENTATION_PLANS_INDEX.md) - Navigation for all plans

### Phase 8 Implementation Plans
- [PHASE_8_MASTER_INTEGRATION_PLAN.md](../PHASE_8_MASTER_INTEGRATION_PLAN.md) - Coordination of 4 priorities
- [PHASE_8_PRIORITY_1_UI_PLAN.md](../PHASE_8_PRIORITY_1_UI_PLAN.md) - 5-week UI implementation
- [PHASE_8_PRIORITY_2_RENDERING_PLAN.md](../PHASE_8_PRIORITY_2_RENDERING_PLAN.md) - 4-5 week rendering
- [PHASE_8_PRIORITY_3_SAVE_LOAD_PLAN.md](../PHASE_8_PRIORITY_3_SAVE_LOAD_PLAN.md) - 2-3 week save/load
- [PHASE_8_PRIORITY_4_AUDIO_PLAN.md](../PHASE_8_PRIORITY_4_AUDIO_PLAN.md) - 2-3 week audio

### Phase 8 Progress Reports (Current State)
- [PHASE_8_1_WEEK_4_DAY_3_COMPLETE.md](../PHASE_8_1_WEEK_4_DAY_3_COMPLETE.md) - Latest progress (Oct 15, 2025)
- [PHASE_8_1_WEEK_3_COMPLETE.md](../PHASE_8_1_WEEK_3_COMPLETE.md) - Week 3 summary
- [PHASE_8_1_WEEK_2_COMPLETE.md](../PHASE_8_1_WEEK_2_COMPLETE.md) - Week 2 summary
- [PHASE_8_1_WEEK_1_COMPLETE.md](../PHASE_8_1_WEEK_1_COMPLETE.md) - Week 1 summary

### Foundation Hardening (Phase 0)
- [IMMEDIATE_ACTIONS_IMPLEMENTATION_PLAN.md](../IMMEDIATE_ACTIONS_IMPLEMENTATION_PLAN.md) - Week 1 critical fixes
- [UNWRAP_AUDIT_ANALYSIS.md](../UNWRAP_AUDIT_ANALYSIS.md) - 637 unwrap calls cataloged
- unwrap_audit_report.csv - Automated audit results

### AI & Arbiter (Phase 7 Complete)
- [PHASE_7_ARBITER_PHASE_7_COMPLETE.md](../PHASE_7_ARBITER_PHASE_7_COMPLETE.md) - Arbiter completion (Jan 15, 2025)
- [docs/ARBITER_IMPLEMENTATION.md](../docs/ARBITER_IMPLEMENTATION.md) - 8,000 word implementation guide
- [docs/ARBITER_QUICK_REFERENCE.md](../docs/ARBITER_QUICK_REFERENCE.md) - 5-minute quick reference

### Validation & Metrics
- [AI_NATIVE_VALIDATION_REPORT.md](../AI_NATIVE_VALIDATION_REPORT.md) - 12,700+ agent capacity proven
- [BASELINE_METRICS.md](../BASELINE_METRICS.md) - Performance baselines
- [WEEK_8_FINAL_SUMMARY.md](../WEEK_8_FINAL_SUMMARY.md) - Week 8 performance sprint

---

**END OF MASTER ROADMAP**
