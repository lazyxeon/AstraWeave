# Phase 10A: Sequential Testing Strategy

**Date**: January 21, 2026  
**Status**: 🎯 IN PROGRESS - Scene test running (563 mutants)

---

## Current Approach: Sequential Single-Crate Testing

### Why Sequential (Not Parallel)?

**Disk Space Constraint Discovery**:
- nav test failed at 280/295 mutants with "disk full" error
- Root cause: 10GB workspace × 8 parallel jobs = 80GB temp space
- Even with --jobs 4: 4 threads × 10GB = 40GB temp space per test
- Running 2 tests in parallel: 80GB+ total temp space = exceeds system capacity

**Solution**: Test ONE crate at a time with --jobs 4 --copy-target=false

### Current Test Configuration

```powershell
cargo mutants --package <CRATE> --timeout 90 --jobs 4 --copy-target=false
```

**Settings Rationale**:
- `--timeout 90`: Allows slower tests (increased from 60s)
- `--jobs 4`: Balances parallelism vs disk space (down from default 8)
- `--copy-target=false`: Skips target/ directory in workspace copies (~30% size reduction)

### Estimated Times (563 mutants baseline)

**astraweave-scene**: 563 mutants
- Workspace copy: ~1 minute (5-6 GB)
- Per mutant: ~30-60 seconds average
- **Total estimated**: 5-9 hours (worst case: 563 × 60s / 4 jobs / 60 = 2.3h minimum, realistically 5-9h with build times)

---

## P0 Tier Execution Plan

### Completed (2/12) ✅

| # | Crate | Mutants | Score | Issues | Time | Status |
|---|-------|---------|-------|--------|------|--------|
| 1 | astraweave-math | 79 | 94.37% | 4 | 1h | ✅ Complete |
| 2 | astraweave-nav | 280/295 | 85.00% | 42 | 2.5h | ⚠️ Partial (disk full) |

**Cumulative**: 359 mutants tested, 89.69% average score, 46 issues documented

### Running (1/12) 🎯

| # | Crate | Mutants | Estimated Time | Terminal |
|---|-------|---------|----------------|----------|
| 3 | astraweave-scene | 563 | 5-9h | 4016a3a9-cd1f-43bf-a8aa-06956df3aa41 |

**Started**: January 21, 2026  
**Status**: Workspace copied (5.9GB in 57s), testing phase started

### Pending (9/12) ⏳

| # | Crate | Est. Mutants | Est. Time | Priority |
|---|-------|-------------|-----------|----------|
| 4 | astraweave-asset | 150-200 | 2-3h | Next |
| 5 | astraweave-core | 250-300 | 3-5h | High |
| 6 | astraweave-ecs | 300-400 | 4-6h | High |
| 7 | astraweave-gameplay | 150-200 | 2-3h | Medium |
| 8 | astraweave-ui | 200-250 | 2-4h | Medium |
| 9 | astraweave-terrain | 250-300 | 3-5h | Medium |
| 10 | astraweave-physics | 300-400 | 4-6h | High |
| 11 | astraweave-render | 400-500 | 6-10h | High |
| 12 | (audio retest) | 117 | 1-2h | Low |

**Estimated remaining**: 25-40 hours

---

## Monitoring & Documentation Process

### Real-Time Monitoring

**Check progress** (every 30-60 minutes):
```powershell
if (Test-Path "mutants.out/outcomes.json") {
    $json = Get-Content "mutants.out/outcomes.json" -Raw | ConvertFrom-Json
    $total = $json.Count
    Write-Host "Progress: $total mutants tested"
}
```

**Or use monitor script**:
```powershell
.\scripts\monitor_mutation_test.ps1 -Package astraweave-scene
```

### Post-Completion Documentation

**For each completed crate**:

1. **Parse Results**:
   ```powershell
   $json = Get-Content "mutants.out/outcomes.json" -Raw | ConvertFrom-Json
   $caught = ($json | Where-Object {$_.scenario.outcome -eq "Caught"}).Count
   $missed = ($json | Where-Object {$_.scenario.outcome -eq "Missed"}).Count
   # Calculate score, list survived mutants
   ```

2. **Create Completion Report**: `PHASE_10A_DAY_X_<CRATE>_COMPLETE.md`
   - Overall score with industry comparison
   - All survived mutants analyzed with severity
   - Critical patterns identified
   - Comparison with previous crates

3. **Update Master Issues Tracker**: Add all issues as Issue #N
   - File path and line number
   - Severity (P0/P1/P2/P3)
   - Mutation details
   - Root cause and impact
   - Recommended fix with code example

4. **Update Progress Tracker**: Mark crate complete, update statistics

---

## Risk Mitigation

### Disk Space Management

**Before each test**:
```powershell
Remove-Item "mutants.out" -Recurse -Force -ErrorAction SilentlyContinue
```

**Monitor during test**:
- Watch for "disk full" errors in terminal output
- If occurs: Clean temp files, reduce --jobs to 2, retry

### Test Interruptions

**If test interrupted**:
1. Check for partial results in mutants.out/outcomes.json
2. Calculate score from partial results (valid if >75% mutants tested)
3. Document as "PARTIAL" with caveat
4. Decide: Accept partial OR retry with lower --jobs

### Long-Running Tests

**For crates with 400+ mutants** (render, physics, ecs):
- Consider splitting test into smaller batches (not currently supported by cargo-mutants)
- OR run overnight with monitoring
- OR increase timeout to 120-180s for complex tests

---

## Success Criteria

### Per-Crate Goals
- ✅ **≥80% mutation score**: World-class quality
- ✅ **All survived mutants documented**: For systematic remediation
- ✅ **Severity classification**: P0 (critical) vs P1-P3
- ✅ **Pattern identification**: Repeated issue types across crates

### P0 Tier Goals (12 crates)
- ✅ **≥80% average score**: 89.69% currently (exceeds target)
- ✅ **100% issue tracking**: 46/46 documented so far (100%)
- ⏳ **100% crate completion**: 2/12 complete (16.7%)
- ⏳ **Systematic remediation prep**: After all 12 tested

---

## Next Steps

### Immediate (Current Session)
1. **Monitor scene test** (5-9h estimated, check every 1-2 hours)
2. **Parse scene results** when complete
3. **Document scene issues** in master tracker
4. **Start next crate** (asset or core)

### Short-Term (Next 3-5 Days)
5. **Complete P0 tier** (9 remaining crates, 25-40h)
6. **Aggregate P0 statistics** (12-crate average, pattern analysis)
7. **Create P0 completion report**

### Medium-Term (Next 1-2 Weeks)
8. **P1 tier testing** (5 crates: ai, cinematics, weaving, materials, editor)
9. **P2 tier testing** (8 crates: embeddings, memory, behavior, input, pcg, scripting, security, llm)
10. **Triage all issues** (prioritize by severity, ROI, safety impact)
11. **Systematic remediation** (fix P0/P1 issues, retest)

---

**Status**: 🎯 Scene test running (563 mutants, 5-9h estimated)  
**Next Update**: After scene test completes  
**Overall Progress**: 2/12 P0 crates (16.7%), 46 issues documented

