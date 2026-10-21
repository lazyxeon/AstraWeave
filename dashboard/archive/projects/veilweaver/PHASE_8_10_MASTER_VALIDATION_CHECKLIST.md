# Phase 8-10: Master Validation Checklist

**Document Version**: 1.0  
**Date**: November 9, 2025  
**Purpose**: Comprehensive test matrix with explicit pass/fail criteria for Phase 8-10 validation

**Philosophy**: Test everything. Assume nothing works. Prove everything with evidence.

---

## Phase 8: Core Game Loop (Weeks 1-12)

### Week 1: Shadow Mapping (8 Tests)

| ID | Test | Description | Pass Criteria | Evidence | Status |
|----|------|-------------|---------------|----------|--------|
| S1.1 | Shadow visibility | Shadows visible under directional light | Screenshot shows shadows | ⏳ | ❌ |
| S1.2 | Cascade coverage | Close objects use cascade 0, far use cascade 1 | Debug visualization shows correct cascade | ⏳ | ❌ |
| S1.3 | Peter-panning fix | No floating shadows | Visual inspection, bias <0.01 | ⏳ | ❌ |
| S1.4 | Shadow acne fix | No shadow banding | Visual inspection, slope bias working | ⏳ | ❌ |
| S1.5 | PCF smoothness | Shadow edges smooth | 3×3 kernel, no jagged edges | ⏳ | ❌ |
| S1.6 | Performance | Shadow pass <2 ms @ 100 meshes | Tracy profile, `shadow_pass_100_meshes` benchmark | ⏳ | ❌ |
| S1.7 | Cascade transitions | No visible seam at cascade boundary | Visual inspection, smooth transition | ⏳ | ❌ |
| S1.8 | Dynamic lights | Moving light updates shadows correctly | Rotating light, shadows update every frame | ⏳ | ❌ |

**Week 1 Acceptance**: 8/8 tests passing, Tracy profile <2 ms, visual validation clean

---

### Week 2: Post-Processing (8 Tests)

| ID | Test | Description | Pass Criteria | Evidence | Status |
|----|------|-------------|---------------|----------|--------|
| P2.1 | Bloom visibility | Bright objects glow | Screenshot shows bloom | ⏳ | ❌ |
| P2.2 | Bloom radius | Glow extends 50-100 pixels | Measure bloom radius in pixels | ⏳ | ❌ |
| P2.3 | Bloom intensity | Adjustable 0-1 range | UI slider changes intensity, visual confirmation | ⏳ | ❌ |
| P2.4 | Bloom threshold | Only bright pixels (>1.0) glow | Dark objects (luminance <1.0) don't glow | ⏳ | ❌ |
| P2.5 | Tonemapping | No blown highlights, correct exposure | ACES curve applied, histogram shows no clipping | ⏳ | ❌ |
| P2.6 | Performance | Post-processing <3 ms @ 1920×1080 | Tracy profile, `bloom_composite` benchmark | ⏳ | ❌ |
| P2.7 | Mip chain | 5 mips generated correctly | Each mip is 50% resolution of previous | ⏳ | ❌ |
| P2.8 | UI responsiveness | Settings update <16 ms | UI slider changes take effect in <1 frame | ⏳ | ❌ |

**Week 2 Acceptance**: 8/8 tests passing, Tracy profile <3 ms, visual validation clean

---

### Week 3: Skybox (8 Tests)

| ID | Test | Description | Pass Criteria | Evidence | Status |
|----|------|-------------|---------------|----------|--------|
| K3.1 | Skybox visibility | Sky visible in all directions | Screenshot shows sky in all 6 directions | ⏳ | ❌ |
| K3.2 | Skybox orientation | Horizon horizontal, poles vertical | Visual inspection, correct orientation | ⏳ | ❌ |
| K3.3 | Skybox seams | No visible seams at cube edges | Visual inspection, seamless cubemap | ⏳ | ❌ |
| K3.4 | Day/night cycle | Smooth transition (0-1 parameter) | Lerp between day/night cubemaps, no pop | ⏳ | ❌ |
| K3.5 | Depth ordering | Sky behind all scene objects | Visual inspection, sky never occludes geometry | ⏳ | ❌ |
| K3.6 | Performance | Skybox pass <0.5 ms | Tracy profile, `skybox_render_pass` benchmark | ⏳ | ❌ |
| K3.7 | Camera movement | Skybox always centered on camera | Move camera, sky follows (no parallax) | ⏳ | ❌ |
| K3.8 | HDR compatibility | Skybox brightness matches scene exposure | Sky exposure consistent with scene lighting | ⏳ | ❌ |

**Week 3 Acceptance**: 8/8 tests passing, Tracy profile <0.5 ms, visual validation clean

---

### Week 4: Dynamic Lights (8 Tests)

| ID | Test | Description | Pass Criteria | Evidence | Status |
|----|------|-------------|---------------|----------|--------|
| L4.1 | Point light | Spherical light falloff | Visual inspection, inverse-square attenuation | ⏳ | ❌ |
| L4.2 | Spot light | Cone-shaped light with falloff | Visual inspection, cone cutoff working | ⏳ | ❌ |
| L4.3 | Multiple lights | 16+ lights rendered correctly | Spawn 16 lights, all visible | ⏳ | ❌ |
| L4.4 | Light attenuation | Inverse-square falloff | Measure light intensity at 1m, 2m, 4m (1:0.25:0.0625) | ⏳ | ❌ |
| L4.5 | Omnidirectional shadows | Point light shadows in all directions | Cubemap shadows, object casts shadow in all 6 directions | ⏳ | ❌ |
| L4.6 | Performance | 16 lights <4 ms, 4 shadowed <8 ms | Tracy profile, benchmarks | ⏳ | ❌ |
| L4.7 | Dynamic updates | Lights move/change color in real-time | Move light, color changes immediately | ⏳ | ❌ |
| L4.8 | Light limit | Graceful degradation beyond 16 lights | 17th light culled or lowest priority replaced | ⏳ | ❌ |

**Week 4 Acceptance**: 8/8 tests passing, Tracy profile <8 ms for 4 shadowed, visual validation clean

---

### Week 5: GPU Particles (8 Tests)

| ID | Test | Description | Pass Criteria | Evidence | Status |
|----|------|-------------|---------------|----------|--------|
| T5.1 | Particle update | 10,000 particles @ 60 FPS | Frame time <16.67 ms with 10k particles | ⏳ | ❌ |
| T5.2 | Particle spawn | Emitters spawn particles correctly | Visual inspection, particles spawn from emitter | ⏳ | ❌ |
| T5.3 | Particle lifetime | Particles fade/die after lifetime | Visual inspection, particles disappear after N seconds | ⏳ | ❌ |
| T5.4 | Fire effect | Realistic fire with 50-100 particles | Visual inspection, fire looks realistic | ⏳ | ❌ |
| T5.5 | Smoke effect | Realistic smoke with 100-200 particles | Visual inspection, smoke rises and dissipates | ⏳ | ❌ |
| T5.6 | Explosion effect | Dramatic explosion with 500-1000 particles | Visual inspection, explosion is impactful | ⏳ | ❌ |
| T5.7 | Performance | <2 ms for 10,000 particles | Tracy profile, `particle_update_10k` + `particle_render_10k` | ⏳ | ❌ |
| T5.8 | Sorting | Particles sort correctly (back-to-front) | Visual inspection, no rendering artifacts | ⏳ | ❌ |

**Week 5 Acceptance**: 8/8 tests passing, Tracy profile <2 ms, visual validation clean

---

### Week 6: ECS Serialization (8 Tests)

| ID | Test | Description | Pass Criteria | Evidence | Status |
|----|------|-------------|---------------|----------|--------|
| V6.1 | Component serialization | All components serialize | `cargo test` all component tests pass | ⏳ | ❌ |
| V6.2 | World save | World saves to disk | File created, no errors | ⏳ | ❌ |
| V6.3 | World load | World loads from disk | Load succeeds, no errors | ⏳ | ❌ |
| V6.4 | Roundtrip | save → load → verify identical | All components match after roundtrip (deterministic equality) | ⏳ | ❌ |
| V6.5 | Large world | 10,000 entities save/load correctly | Save/load 10k entities, verify counts match | ⏳ | ❌ |
| V6.6 | Performance | Save <5s, load <10s @ 10k entities | Stopwatch measurement | ⏳ | ❌ |
| V6.7 | Error handling | Corrupted file detected, graceful error | Corrupt save file, verify error message | ⏳ | ❌ |
| V6.8 | File size | Save file <100 MB for 10k entities | Check file size on disk | ⏳ | ❌ |

**Week 6 Acceptance**: 8/8 tests passing, roundtrip deterministic, benchmarks within expectations

---

### Week 7: Save Slots & Versioning (8 Tests)

| ID | Test | Description | Pass Criteria | Evidence | Status |
|----|------|-------------|---------------|----------|--------|
| F7.1 | Player profile | Profile saves/loads | Profile file created, settings persist | ⏳ | ❌ |
| F7.2 | Save slots | 10 slots work independently | Save to 10 slots, load each, verify independent | ⏳ | ❌ |
| F7.3 | Save versioning | Version mismatch detected | Load old save, verify version error message | ⏳ | ❌ |
| F7.4 | Save migration | v1 → v2 migration works | Create v1 save, migrate to v2, verify correct | ⏳ | ❌ |
| F7.5 | Corruption detection | Corrupted file detected | Corrupt CRC32, verify error message | ⏳ | ❌ |
| F7.6 | Auto-backup | 3 backups created | Save 3 times, verify 3 backup files exist | ⏳ | ❌ |
| F7.7 | Backup restore | Restore from backup works | Corrupt save, restore from backup, verify correct | ⏳ | ❌ |
| F7.8 | UI integration | Save/load menu works | Click save/load buttons, verify actions | ⏳ | ❌ |

**Week 7 Acceptance**: 8/8 tests passing, UI functional, versioning+migration working

---

### Week 8: Production Audio (8 Tests)

| ID | Test | Description | Pass Criteria | Evidence | Status |
|----|------|-------------|---------------|----------|--------|
| A8.1 | Dynamic music | 4 layers crossfade smoothly | Play 4 music layers, verify no pops/clicks | ⏳ | ❌ |
| A8.2 | Audio occlusion | Sounds muffled behind walls | Raycast occlusion, sounds attenuated when blocked | ⏳ | ❌ |
| A8.3 | Reverb zones | 5 zone types work correctly | Cave, hall, outdoor, underwater, tunnel reverb working | ⏳ | ❌ |
| A8.4 | Mixer UI | Volume sliders work | Adjust sliders, verify immediate effect | ⏳ | ❌ |
| A8.5 | 50+ sounds | No clipping/distortion | Spawn 50 emitters, verify all sounds play correctly | ⏳ | ❌ |
| A8.6 | Performance | Audio update <1 ms | Tracy profile, `audio_update_50_sounds` benchmark | ⏳ | ❌ |
| A8.7 | Spatial audio | 3D positioning correct | Move listener, verify sounds pan correctly | ⏳ | ❌ |
| A8.8 | Music looping | Loops seamlessly (no gap) | Play music for 5 minutes, verify no gap at loop point | ⏳ | ❌ |

**Week 8 Acceptance**: 8/8 tests passing, Tracy profile <1 ms, audio quality clean

---

### Week 9-10: Integration Tests (8 Tests)

| ID | Test | Description | Pass Criteria | Evidence | Status |
|----|------|-------------|---------------|----------|--------|
| I9.1 | Full rendering | Shadows + bloom + skybox + particles | All rendering systems working together | ⏳ | ❌ |
| I9.2 | Full audio | Dynamic music + occlusion + reverb | All audio systems working together | ⏳ | ❌ |
| I9.3 | Save/load | Save → quit → load → verify identical | Roundtrip with full game state | ⏳ | ❌ |
| I9.4 | Veilweaver Demo | 5-10 min playthrough @ 60 FPS | Complete demo level, frame time <16.67 ms | ⏳ | ❌ |
| I9.5 | 1,000 entities | All systems @ 60 FPS | Spawn 1000 entities, frame time <16.67 ms | ⏳ | ❌ |
| I9.6 | Frame budget | <15 ms p95 frame time | Tracy profile, p95 <15 ms (10% headroom) | ⏳ | ❌ |
| I9.7 | Memory usage | <2 GB RAM | Task Manager/htop, verify <2 GB | ⏳ | ❌ |
| I9.8 | No regressions | All Week 5 tests still passing | Re-run Week 5 test suite, 351/351 passing | ⏳ | ❌ |

**Week 9-10 Acceptance**: 8/8 tests passing, Veilweaver Demo playable, no regressions

---

### Week 11-12: External Acceptance Tests (10 Tests)

| ID | Test | Description | Pass Criteria | Evidence | Status |
|----|------|-------------|---------------|----------|--------|
| E11.1 | Installer (Windows) | Windows .exe installs cleanly | Test on clean Windows 10/11 VM, no errors | ⏳ | ❌ |
| E11.2 | Installer (Linux) | Linux AppImage runs cleanly | Test on Ubuntu 20.04/22.04 LTS, no errors | ⏳ | ❌ |
| E11.3 | Installer (macOS) | macOS .app runs cleanly | Test on macOS 11+ (Intel/Apple Silicon), no errors | ⏳ | ❌ |
| E11.4 | External playtesters | 10+ testers complete demo | >80% completion rate (8/10 finish) | ⏳ | ❌ |
| E11.5 | Session length | >5 minutes average | Telemetry data, average session >5 min | ⏳ | ❌ |
| E11.6 | Crash rate | <5% per session | Telemetry data, 0-1 crashes per 10 sessions | ⏳ | ❌ |
| E11.7 | Positive feedback | >70% positive | Feedback survey, >70% would recommend | ⏳ | ❌ |
| E11.8 | Bug reports | Critical bugs fixed | All P0 bugs fixed before release | ⏳ | ❌ |
| E11.9 | Performance validation | Frame time p95 <20 ms on target hardware | Telemetry data, representative hardware | ⏳ | ❌ |
| E11.10 | User satisfaction | At least 3 positive reviews | Playtesters write positive reviews | ⏳ | ❌ |

**Week 11-12 Acceptance**: 10/10 tests passing, external validation complete, ready for Phase 9

---

## Phase 9: Distribution & Polish (Weeks 13-24)

### Week 13-16: Build Pipeline (10 Tests)

| ID | Test | Description | Pass Criteria | Evidence | Status |
|----|------|-------------|---------------|----------|--------|
| B13.1 | Asset packing | Assets packed into .pak archive | .pak file created, <500 MB | ⏳ | ❌ |
| B13.2 | CI/CD (Windows) | Automated Windows build | GitHub Actions builds .exe | ⏳ | ❌ |
| B13.3 | CI/CD (Linux) | Automated Linux build | GitHub Actions builds AppImage | ⏳ | ❌ |
| B13.4 | CI/CD (macOS) | Automated macOS build | GitHub Actions builds .app | ⏳ | ❌ |
| B13.5 | NSIS installer | Windows installer | NSIS installer works on clean Windows VM | ⏳ | ❌ |
| B13.6 | AppImage | Linux portable app | AppImage runs on Ubuntu 20.04/22.04 | ⏳ | ❌ |
| B13.7 | DMG installer | macOS disk image | DMG mounts and installs on macOS 11+ | ⏳ | ❌ |
| B13.8 | Steamworks SDK | Steam integration | Steam achievements trigger, cloud saves work | ⏳ | ❌ |
| B13.9 | itch.io upload | Downloadable by public | Upload to itch.io, download works | ⏳ | ❌ |
| B13.10 | Asset load time | <30 seconds on SSD | Stopwatch measurement, <30s load time | ⏳ | ❌ |

**Week 13-16 Acceptance**: 10/10 tests passing, all platforms building, installers working

---

### Week 17-20: Asset Pipeline (8 Tests)

| ID | Test | Description | Pass Criteria | Evidence | Status |
|----|------|-------------|---------------|----------|--------|
| O17.1 | Texture atlasing | 50% draw call reduction | Benchmark draw calls before/after | ⏳ | ❌ |
| O17.2 | Animation retargeting | 3+ skeleton types | Retarget animation between 3 skeletons | ⏳ | ❌ |
| O17.3 | LOD generation | 3-5 LOD levels auto-generated | Check LOD assets, verify 3-5 levels | ⏳ | ❌ |
| O17.4 | Asset dependency tracking | Dependencies tracked correctly | Change texture, verify mesh reloads | ⏳ | ❌ |
| O17.5 | Hot-reload cascade | Cascade reloads work | Change material, verify all using meshes reload | ⏳ | ❌ |
| O17.6 | Hot-reload performance | <500 ms to reload | Stopwatch measurement, <500 ms | ⏳ | ❌ |
| O17.7 | UV remapping | Atlas UVs correct | Visual inspection, no texture distortion | ⏳ | ❌ |
| O17.8 | LOD transitions | Smooth LOD transitions | No pop when LOD changes | ⏳ | ❌ |

**Week 17-20 Acceptance**: 8/8 tests passing, asset pipeline optimized

---

### Week 21-24: Telemetry & Profiling (8 Tests)

| ID | Test | Description | Pass Criteria | Evidence | Status |
|----|------|-------------|---------------|----------|--------|
| M21.1 | Production profiler | <0.1 ms overhead | Benchmark profiler overhead, <0.1 ms | ⏳ | ❌ |
| M21.2 | Telemetry collection | Metrics received from 10+ users | Telemetry backend receives data | ⏳ | ❌ |
| M21.3 | Crash dumps | Stack traces uploaded | Crash dump server receives traces | ⏳ | ❌ |
| M21.4 | Performance overlay | FPS counter working | Press F3, verify FPS display | ⏳ | ❌ |
| M21.5 | Frame graph | Frame time graph accurate | Verify graph matches Tracy profile | ⏳ | ❌ |
| M21.6 | Memory tracking | Memory usage tracked | Verify memory graph matches Task Manager | ⏳ | ❌ |
| M21.7 | Crash reporting | Logs attached to crash dumps | Verify crash dump includes logs | ⏳ | ❌ |
| M21.8 | Anonymization | Personal data not collected | Audit telemetry data, no PII | ⏳ | ❌ |

**Week 21-24 Acceptance**: 8/8 tests passing, telemetry operational, privacy compliant

---

## Phase 10: Multiplayer & Advanced (Weeks 25-48, OPTIONAL)

### Week 25-32: Networking (10 Tests)

| ID | Test | Description | Pass Criteria | Evidence | Status |
|----|------|-------------|---------------|----------|--------|
| N25.1 | UDP transport | Network packets sent/received | Packet capture, UDP packets visible | ⏳ | ❌ |
| N25.2 | Client-server | Authoritative server | Client predictions overridden by server | ⏳ | ❌ |
| N25.3 | Entity replication | 1,000 entities @ 20 Hz | Network bandwidth <50 Kbps per client | ⏳ | ❌ |
| N25.4 | Delta compression | Bandwidth reduced | Compare full vs delta, verify compression | ⏳ | ❌ |
| N25.5 | Interest management | Only nearby entities replicated | Client only receives entities within 100m | ⏳ | ❌ |
| N25.6 | Matchmaking | 10+ players in lobby | Spawn 10 clients, verify all see each other | ⏳ | ❌ |
| N25.7 | Prediction | <50 ms perceived latency @ 100 ms ping | Latency test, user perception acceptable | ⏳ | ❌ |
| N25.8 | Rollback | Rollback on misprediction | Force misprediction, verify rollback | ⏳ | ❌ |
| N25.9 | Lag compensation | Hit detection works @ 100 ms ping | Shoot moving target, verify hits register | ⏳ | ❌ |
| N25.10 | Disconnect handling | Graceful disconnect | Disconnect client, verify no server crash | ⏳ | ❌ |

**Week 25-32 Acceptance**: 10/10 tests passing, multiplayer functional @ 20 Hz

---

### Week 33-38: Advanced Rendering (8 Tests)

| ID | Test | Description | Pass Criteria | Evidence | Status |
|----|------|-------------|---------------|----------|--------|
| R33.1 | Global Illumination | Indirect lighting visible | Visual inspection, bounce light working | ⏳ | ❌ |
| R33.2 | GI performance | <5 ms overhead | Tracy profile, GI pass <5 ms | ⏳ | ❌ |
| R33.3 | Depth of Field | Bokeh blur working | Visual inspection, out-of-focus blur | ⏳ | ❌ |
| R33.4 | Motion blur | Moving objects blurred | Visual inspection, motion trails | ⏳ | ❌ |
| R33.5 | Chromatic aberration | Color fringing at edges | Visual inspection, RGB separation | ⏳ | ❌ |
| R33.6 | Decal system | 100+ decals @ 60 FPS | Spawn 100 decals, frame time <16.67 ms | ⏳ | ❌ |
| R33.7 | Rain effect | Realistic rain | Visual inspection, rain particles + puddles | ⏳ | ❌ |
| R33.8 | Snow effect | Realistic snow | Visual inspection, snow particles + accumulation | ⏳ | ❌ |

**Week 33-38 Acceptance**: 8/8 tests passing, advanced rendering working

---

### Week 39-48: Advanced AI (6 Tests)

| ID | Test | Description | Pass Criteria | Evidence | Status |
|----|------|-------------|---------------|----------|--------|
| H39.1 | LLM success rate | 80%+ success rate | Run 100 LLM calls, >80 succeed | ⏳ | ❌ |
| H39.2 | Prompt caching | 50× speedup | Measure latency before/after caching (3.5s → 70ms) | ⏳ | ❌ |
| H39.3 | Squad AI | 10+ agents coordinate | Spawn 10 agents, verify cooperative tactics | ⏳ | ❌ |
| H39.4 | Parameter defaulting | Missing params defaulted | LLM omits param, system defaults correctly | ⏳ | ❌ |
| H39.5 | Simplified tool set | Tier 2 tools work | Test simplified 10-tool set vs 37-tool set | ⏳ | ❌ |
| H39.6 | phi3:medium | 14B model works | Load phi3:medium, verify inference works | ⏳ | ❌ |

**Week 39-48 Acceptance**: 6/6 tests passing, LLM success 80%+, squad AI working

---

## Summary Statistics

### Total Tests by Phase

| Phase | Unit | Integration | Stress | Acceptance | Total |
|-------|------|-------------|--------|------------|-------|
| Phase 8 (Weeks 1-12) | 64 | 8 | 5 | 10 | **87** |
| Phase 9 (Weeks 13-24) | 16 | 8 | 3 | 5 | **32** |
| Phase 10 (Weeks 25-48) | 24 | 10 | 5 | 5 | **44** |
| **Total** | **104** | **26** | **13** | **20** | **163** |

### Pass Rate Targets

| Week | Tests | Pass Target | Current | Status |
|------|-------|-------------|---------|--------|
| Week 1 | 8 | 8/8 (100%) | 0/8 (0%) | ⏳ |
| Week 2 | 8 | 8/8 (100%) | 0/8 (0%) | ⏳ |
| Week 3 | 8 | 8/8 (100%) | 0/8 (0%) | ⏳ |
| Week 4 | 8 | 8/8 (100%) | 0/8 (0%) | ⏳ |
| Week 5 | 8 | 8/8 (100%) | 0/8 (0%) | ⏳ |
| Week 6 | 8 | 8/8 (100%) | 0/8 (0%) | ⏳ |
| Week 7 | 8 | 8/8 (100%) | 0/8 (0%) | ⏳ |
| Week 8 | 8 | 8/8 (100%) | 0/8 (0%) | ⏳ |
| Week 9-10 | 8 | 8/8 (100%) | 0/8 (0%) | ⏳ |
| Week 11-12 | 10 | 10/10 (100%) | 0/10 (0%) | ⏳ |
| **Phase 8 Total** | **87** | **87/87 (100%)** | **0/87 (0%)** | **⏳** |

---

## Usage Instructions

### For Developers

1. **Before Starting Work**: Review test matrix for current week
2. **During Development**: Run tests frequently, mark passing tests ✅
3. **End of Week**: All tests must pass before proceeding to next week
4. **Evidence Required**: Screenshots, Tracy profiles, benchmark results, crash logs

### For Reviewers

1. **Verify Evidence**: Check screenshots, profiles, benchmarks
2. **Run Tests Yourself**: Don't trust developer reports, verify independently
3. **Document Failures**: If test fails, document why and how to fix
4. **No Pass Without Evidence**: Never mark test passing without proof

### For QA

1. **Weekly Validation**: Re-run all passing tests from previous weeks
2. **Regression Detection**: If test fails after passing, file bug immediately
3. **External Testing**: Recruit playtesters for acceptance tests (Week 11-12)
4. **Telemetry Analysis**: Monitor crash rate, frame time, completion rate

---

## CI/CD Integration

### Automated Tests (GitHub Actions)

```yaml
# .github/workflows/phase8_validation.yml
name: Phase 8 Validation

on: [push, pull_request]

jobs:
  unit-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - run: cargo test --all-features
      
  integration-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - run: cargo test --test integration_* --release
      
  benchmarks:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - run: cargo bench --all-features -- --save-baseline ci
      - run: ./scripts/check_benchmark_regression.sh
      # Fails if >10% regression
      
  stress-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - run: cargo test --test stress_* --release -- --nocapture
```

### Manual Tests (Weekly)

**Every Friday**:
1. Run manual test suite (visual validation, UI testing, audio testing)
2. Capture Tracy profile (save to `profiles/week_N.tracy`)
3. Generate weekly report (test results, metrics, blockers)
4. Update master checklist (mark passing tests ✅)

---

## Next Steps

**Week 1 (Nov 10-16, 2025)**:
1. Review Shadow Mapping test matrix (8 tests)
2. Enable shadow depth passes in renderer
3. Run tests S1.1 - S1.8
4. Document evidence (screenshots, Tracy profile)
5. Weekly report (Friday, Nov 15)

**Week 2-12**:
- Follow week-by-week test matrix
- All tests must pass before proceeding
- No exceptions, no shortcuts
- Prove everything with evidence

---

**Let's validate everything! 🧪**
