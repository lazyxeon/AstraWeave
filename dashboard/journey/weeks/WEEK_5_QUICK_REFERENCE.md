# Week 5 Quick Reference Card

**Crate**: astraweave-input | **Dates**: Oct 24-26 | **Target**: 60 tests, 80% coverage, 8h

---

## 📋 Daily Checklist

### Day 1: Baseline + Unit Tests (2.5h)
- [ ] Run baseline coverage: `cargo llvm-cov --lib -p astraweave-input --summary-only`
- [ ] Identify existing tests: `cargo test -p astraweave-input --lib -- --list`
- [ ] Create 15-20 unit tests (input mapping, bindings, gamepad)
- [ ] Document in `PHASE_5B_WEEK_5_DAY_1_BASELINE.md`

### Day 2: Stress + Edge Cases (3h)
- [ ] Create 15-20 stress tests (rapid input, many keys, large binding tables)
- [ ] Create 15-20 edge cases (invalid codes, conflicts, missing devices)
- [ ] Watch for coverage plateau (apply Week 4 pattern if needed)
- [ ] Document in `PHASE_5B_WEEK_5_DAY_2_COMPLETE.md`

### Day 3: Integration + Benchmarks (2.5h)
- [ ] Create 10-15 integration tests (input pipeline, multi-context)
- [ ] Create 5-10 benchmarks (event processing, binding lookup)
- [ ] Generate test fixtures if plateau detected (config files)
- [ ] Document in `PHASE_5B_WEEK_5_DAY_3_COMPLETE.md`

### Day 4: Documentation (1h)
- [ ] Create week summary: `PHASE_5B_WEEK_5_COMPLETE.md`
- [ ] Update `PHASE_5B_STATUS.md`
- [ ] Create Week 6 plan
- [ ] Celebrate A+ grade! 🎉

---

## 🎯 Success Criteria (A+ Grade)

| Criterion | Target |
|-----------|--------|
| Tests | ≥60 |
| Coverage | ≥80% |
| Pass Rate | 100% |
| Time | ≤8h |
| Innovation | 1+ pattern |

---

## 🔧 Key Commands

```powershell
# Baseline
cargo llvm-cov --lib -p astraweave-input --summary-only

# Tests
cargo test -p astraweave-input --lib

# Coverage gaps
cargo llvm-cov --html -p astraweave-input --open

# Benchmarks
cargo bench -p astraweave-input
```

---

## 🚨 Plateau Response (Week 4 Pattern)

If coverage stuck for 2+ days:
1. `cargo llvm-cov --show-missing -p astraweave-input`
2. Identify uncovered clusters (config parsing? device I/O?)
3. Generate test fixtures programmatically (Week 4 pattern)
4. Re-measure (expect +10-20%)

---

## 💡 Fixture Generation Examples

```rust
// Key binding config
fn generate_test_config(path: &Path) -> std::io::Result<()> {
    let config = r#"
[bindings]
move_forward = "W"
jump = "Space"
"#;
    std::fs::write(path, config)
}

// Mock gamepad
struct MockGamepad {
    buttons: [bool; 16],
    axes: [f32; 6],
}
```

---

## 📊 Expected Coverage Journey

- Day 1: ~50-60% (baseline + unit tests)
- Day 2: ~65-75% (stress + edge cases)
- Day 3: ~75-85% (integration + benchmarks)
- Day 3+: Breakthrough if plateau detected

---

## 🎯 Phase 5B Status

**After Week 4**:
- Tests: 452/555 (81%)
- Crates: 4/7 (57%)
- Time: 25.9h/45h (58%)
- A+ Rate: 4/4 (100%)

**After Week 5** (projected):
- Tests: 512/555 (92%)
- Crates: 5/7 (71%)
- Time: 33-34h/45h (73-76%)
- A+ Rate: 5/5 (100%)

---

## 🏆 Streak Status

✅ Week 1: A+ (astraweave-security, 90% coverage)
✅ Week 2: A+ (astraweave-nav, 89.7% coverage)
✅ Week 3: A+ (astraweave-ai, 94.89% coverage)
✅ Week 4: A+ (astraweave-audio, 92.34% coverage)
⏳ Week 5: ??? (astraweave-input, target 80%+)

**Goal**: Maintain 5/5 A+ streak!

---

## 📚 Documentation Targets

- Day 1: 5k words (baseline report)
- Day 2: 10k words (stress + edge report)
- Day 3: 8k words (integration + benchmarks)
- Day 4: 12k words (week summary)
- **Total**: 35k words (Week 4 standard)

---

## ⚡ Time Budget

| Day | Target | Stretch |
|-----|--------|---------|
| Day 1 | 2.5h | 2h |
| Day 2 | 3h | 2.5h |
| Day 3 | 2.5h | 2h |
| Day 4 | 1h | 0.5h |
| **Total** | **9h** | **7h** |

Buffer: 1-3 hours (10-30%)

---

## 🎉 Celebration Milestones

- [ ] Baseline measured
- [ ] 20 tests passing
- [ ] 40 tests passing
- [ ] 60 tests passing
- [ ] 75% coverage achieved
- [ ] 80% coverage achieved
- [ ] All tests executable (0 ignored)
- [ ] Week summary complete
- [ ] A+ grade earned! 🏆

---

**Quick Reference Version**: 1.0
**Date**: October 23, 2025
**Status**: Ready for Week 5 Day 1
