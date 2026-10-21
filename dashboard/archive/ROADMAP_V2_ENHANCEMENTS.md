# Master Roadmap v2.0 Enhancements
## Validation Rigor Improvements (October 16, 2025)

**Document**: `ASTRAWEAVE_MASTER_ROADMAP_2025_2027.md`  
**Version**: 1.0 → 2.0  
**Changes**: 7 major enhancements based on external analysis  
**Result**: From "85% excellent" to "production-grade project management"

---

## Summary of Changes

### 1. Enhanced Phase 0 Exit Criteria ✅

**Before**: Vague success criteria table with no validation details

**After**: Comprehensive validation framework with:
- **Code Quality**: Automated verification (cargo check, clippy --deny warnings, unwrap scan)
- **Performance Regression**: Specific thresholds vs BASELINE_METRICS.md (10% tolerance)
  - ECS tick <1.5 ns (currently 1 ns, 50% headroom)
  - GOAP planning <110 ns (currently 101.7 ns, 8% headroom)
  - Arbiter overhead <250 ns (currently 221.9 ns, 13% headroom)
- **Integration Testing**: 4 specific skeletal animation tests
  1. CPU vs GPU parity (output diff <0.01%)
  2. Determinism (100 frames, binary comparison)
  3. Scene graph integration (hierarchical transforms)
  4. Performance (100+ animated chars @ 60 FPS)
- **CI Quality Gates**: Automated enforcement (zero warnings, benchmark regression <200%)

**Impact**: Phase 0 validation now measurable and enforceable

---

### 2. Phase 8 Performance Validation ✅

**Before**: No frame budget allocation, vague performance targets

**After**: Detailed frame budget per system:
```
60 FPS = 16.67 ms total budget (10% headroom = 15 ms target)
├── UI update:     <2 ms (egui + events)
├── Rendering:     <8 ms (shadows + post-FX + particles)
├── Audio:         <1 ms (mixer + spatial)
├── Physics:       <3 ms (validated existing)
├── AI:            <2 ms (validated existing)
└── Total:        <16 ms (within budget)
```

**Stress Test Scenarios**:
- 1,000 entities with full Phase 8 features @ 60 FPS
- Veilweaver Demo: 5-10 min sustained, no frame drops >33 ms
- UI responsiveness: <16 ms input latency
- Save/load: <5s save, <10s load
- Asset hot-reload: <500 ms

**Benchmarking Requirements**:
- New `phase_8_integration_bench` - Full game loop
- CI fails if p95 frame time >20 ms
- Tracy profiles included in completion report

**Impact**: Phase 8 performance now quantified and testable

---

### 3. Phase 9 Exit Criteria (Distribution Validation) ✅

**Before**: "Shipped to itch.io" - vague, no validation details

**After**: Comprehensive validation framework:

**Build Pipeline Validation**:
- Windows .exe on clean Windows 10/11 VM (no dev tools)
- Linux AppImage on Ubuntu 20.04/22.04 LTS
- macOS .app on macOS 11+ (Intel and Apple Silicon)
- Asset packing: <500 MB, loads <30 seconds

**Platform Integration**:
- itch.io downloadable by public, auto-update works
- Steam (optional): achievements, cloud saves functional
- Telemetry: crash dumps from 10+ test users

**Quality Gates**:
- Crash rate <1% per session (measured via telemetry)
- Load time <30s HDD, <10s SSD
- Memory footprint <2 GB RAM

**User Acceptance Testing**:
- 10+ external playtesters complete demo
- Feedback collected (controls, performance, bugs)
- Critical bugs fixed before public release

**Success Metrics** (post-launch):
- 50+ downloads in first week
- Crash rate <5% of sessions
- Average session length >10 minutes
- Completion rate >50%

**Impact**: "Shipped" now has concrete definition with measurable outcomes

---

### 4. Phase 10 Decision Gate ✅

**Before**: "OPTIONAL" designation with no decision criteria

**After**: Clear proceed/skip decision framework:

**Proceed to Phase 10 if ANY of**:
1. Community demand: 50+ multiplayer requests
2. Revenue target: $10k from Veilweaver EA
3. Strategic partnership: Studio contract requiring multiplayer
4. Technical validation: 100% determinism, 60 FPS proven

**Skip Phase 10 if ANY of**:
1. Resource constraints: <1.5 FTE available
2. Timeline pressure: Phase 8-9 took >12 months
3. Market feedback: <30% interest in multiplayer
4. Technical blockers: Desyncs or <30 FPS

**Fallback Strategies**:
- **Option A**: Focus on Phase 11 (AI excellence) - RECOMMENDED
- **Option B**: Community-driven networking (18-24 months)
- **Option C**: Partner with middleware (Photon, Mirror, $1k-5k/year)
- **Option D**: Defer to post-1.0 (ship single-player first)

**Decision Criteria Table**:
| Criterion | Proceed | Skip |
|-----------|---------|------|
| Community demand | 50+ requests | <30 requests |
| Revenue | $10k+ | <$5k |
| Resources | 2-3 FTE | <1.5 FTE |
| Timeline | <10 months (Phase 8-9) | >12 months |
| Market feedback | >50% want MP | <30% want MP |
| Technical readiness | 100% determinism, 60 FPS | Desyncs or <30 FPS |

**Impact**: Phase 10 decision now data-driven with clear triggers

---

### 5. Revised Phase 11 LLM Metrics ✅

**Before**: Unrealistic 95%+ LLM success rate target

**After**: Tiered success metrics with realistic targets:

**Tier 1: Parse Success** (JSON Quality)
- Current: 100% (Hermes 2 Pro)
- Target: Maintain 100%

**Tier 2: Validation Success** (Tool Sandbox)
- Current: ~60-70% (estimated)
- Target: 85%+ (parameter defaulting, simplified tools)

**Tier 3: Goal Achievement** (Plan Completes Objective)
- Current: ~40-50% (from validation reports)
- Target: **70%+** (was 95%+)
- Rationale: Even GPT-4 doesn't hit 95%, 70% is excellent for Hermes 2 Pro (7B)

**Overall System Reliability** (With Fallbacks)
- Current: 100% (GOAP guarantees action)
- Target: Maintain 100%
- GOAP fallback means 70% LLM success is acceptable

**Why 70% vs 95%?**
- Hermes 2 Pro (7B) is smaller model, 70% is realistic
- 95% would require 70B+ model or ensemble (costly)
- GOAP fallback makes 70% sufficient (players won't notice)
- Prevents setting unrealistic expectations

**Impact**: LLM targets now achievable and aligned with model capabilities

---

### 6. Integration Testing Strategy ✅

**Before**: No integration testing until Phase 11 (Month 22) - risky

**After**: Per-phase integration gates to catch issues early:

**Phase 8 Integration Test (Week 16)**:
- Full game loop: UI → Rendering → Physics → AI → Audio @ 60 FPS
- Cross-system: HUD updates, audio responds to events
- Save/load: Determinism, state preservation
- Stress: 30 min continuous, zero crashes

**Phase 9 Integration Test (Week 28)**:
- Asset pipeline: Hot-reload all types (<500 ms)
- Build pipeline: Clean install on 3 platforms
- Telemetry: Crash dumps from 10+ users

**Phase 10 Integration Test (Week 48)**:
- Networking: 4-player, 2-hour session, <5% desync
- Advanced rendering: GI + shadows + post-FX @ 60 FPS
- Multiplayer stress: 10+ playtesters, smooth experience

**Phase 11 Integration Test (Month 22)**:
- Cross-system: All systems active simultaneously
- Soak test: 10-hour session, zero crashes
- Console builds: Xbox/PS5/Switch functional

**Impact**: Integration issues caught early, reduces late-stage rework

---

### 7. New Sections Added ✅

#### 7a. Performance Regression Testing (Continuous Validation)

**Automated Benchmarking (CI)**:
- Run full suite on every PR
- Fail if >20% regression (existing 200% threshold)
- Post results as PR comment
- Require approval for 10-20% regressions

**Performance Baselines** (updated per phase):
- Phase 0 (Week 4): Foundation hardening
- Phase 8 (Week 16): Full game loop
- Phase 9 (Week 28): Optimized build
- Phase 10 (Week 48): Networking + advanced

**Regression Alerts**:
- Slack/Discord notifications
- Auto-create GitHub issue with Tracy profile

**Tracy Integration**:
- Always-on in debug builds (zero overhead)
- F-key toggle in release builds (<1% overhead)
- Automated profile capture (CI, weekly, on regression)

---

#### 7b. Quality Gates Checklist (Per-Phase Exit)

**Format**: Checkbox checklists for manual validation

**Phase 0 Exit Gate**:
- [ ] Zero unwraps in core (automated scan)
- [ ] Zero todos in production
- [ ] Clippy clean --deny warnings
- [ ] All benchmarks within 10% baseline
- [ ] 4/4 integration tests passing

**Phase 8 Exit Gate**:
- [ ] UI/rendering/save/audio 100% complete
- [ ] Veilweaver Demo @ 60 FPS (30 min stress)
- [ ] Frame time p95 <20 ms
- [ ] 5+ external playtesters (UAT)

**Phase 9 Exit Gate**:
- [ ] Clean install works on 3 platforms
- [ ] itch.io upload downloadable
- [ ] Crash rate <1% (telemetry)
- [ ] 10+ external playtesters

**Phase 10 Exit Gate**:
- [ ] 4-player stress test (<5% desync)
- [ ] Advanced rendering @ 60 FPS
- [ ] 20+ multiplayer sessions no crash

**Phase 11 Exit Gate**:
- [ ] LLM Tier 3 success 70%+
- [ ] Console builds functional
- [ ] Integration test coverage 70%+

**Impact**: Exit criteria now actionable with clear checklists

---

#### 7c. Measurement & Observability Requirements

**Production Telemetry** (Phase 9.3):
- Frame time: p50, p95, p99 per system
- Crash dumps: Stack trace, system info, logs
- LLM success: Parse, validation, goal achievement
- Player behavior: Session length, completion rate

**Tracy Integration** (Phase 9.3):
- Always-on in debug (zero overhead)
- Optional in release (F-key toggle, <1%)
- Automated captures (CI, weekly, regression)

**Quality Metrics Dashboard** (Phase 11.4):
- CI: Test pass rate, benchmarks, coverage
- Community: Crash rate, feedback, downloads
- Development: LOC, velocity, bug count

**Impact**: Comprehensive observability for production debugging and improvement

---

## Validation Improvements Summary

| Area | Before | After | Impact |
|------|--------|-------|--------|
| **Phase 0 Exit** | Vague table | Detailed criteria + thresholds | ✅ Measurable |
| **Phase 8 Perf** | No budget | Frame budget per system | ✅ Testable |
| **Phase 9 Shipped** | "Upload to itch.io" | Build validation + UAT + metrics | ✅ Concrete |
| **Phase 10 Decision** | "OPTIONAL" | Proceed/skip criteria + fallbacks | ✅ Data-driven |
| **Phase 11 LLM** | 95%+ unrealistic | 70%+ realistic tiered metrics | ✅ Achievable |
| **Integration Tests** | Phase 11 only | Per-phase gates (8, 9, 10, 11) | ✅ Early detection |
| **Perf Regression** | Ad-hoc | Automated CI + alerts + baselines | ✅ Continuous |
| **Quality Gates** | Implied | Explicit checklists per phase | ✅ Enforceable |
| **Observability** | Basic | Comprehensive telemetry + Tracy + dashboard | ✅ Production-ready |

---

## Analysis Verdict: From 85% → 100%

**Original Assessment**: "85% excellent, 15% gap in validation rigor"

**After v2.0 Enhancements**: ✅ **Production-grade project management**

**What Was Missing (Now Fixed)**:
- ⚠️ Performance validation → ✅ Frame budgets, regression tests
- ⚠️ Integration testing → ✅ Per-phase integration gates
- ⚠️ Quality gates → ✅ Measurable exit criteria with checklists
- ⚠️ Phase 10 decision → ✅ Clear proceed/skip triggers
- ⚠️ Realistic targets → ✅ LLM 70% (was 95%), achievable

**Bottom Line**: 
This roadmap is now **production-grade**, with:
- Measurable success criteria at every phase
- Automated validation (CI, benchmarks, tests)
- Clear decision points (Phase 10 gate)
- Realistic targets (LLM 70%, not 95%)
- Comprehensive observability (telemetry, Tracy, dashboards)

---

## Next Actions

1. **Review v2.0 Enhancements**: Validate that changes align with project goals
2. **Approve Roadmap**: Sign off on v2.0 as authoritative plan
3. **Begin Phase 0**: Start foundation hardening (Week 1: unwrap audit + feature fixes)
4. **Set Up CI Gates**: Implement automated quality gates (clippy, benchmarks, unwrap scan)
5. **Baseline Tracking**: Update BASELINE_METRICS.md with Phase 0 targets

---

**Document Status**: Ready for Approval  
**Version**: 2.0 (Enhanced Validation & Quality Gates)  
**Last Updated**: October 16, 2025  
**Maintainer**: AI Development Team  
**Next Review**: November 16, 2025 (after Phase 0 completion)
