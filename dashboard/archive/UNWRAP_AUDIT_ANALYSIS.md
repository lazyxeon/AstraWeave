# .unwrap() Usage Audit - Analysis & Remediation Plan

**Date**: October 9, 2025  
**Status**: ✅ Audit Complete - Remediation Planning  
**Tool**: `scripts/audit_unwrap.ps1`  
**Report**: `unwrap_audit_report.csv`  

---

## Executive Summary

Comprehensive audit of `.unwrap()` usage across 354 Rust files identified **637 total instances** requiring review. Critical findings show **342 P0 cases** in production code and **116 P1 cases** in core engine systems that need immediate remediation to prevent runtime panics.

### Risk Distribution
```
🔴 P0-Critical:  342 (54%)  - Production code, immediate action required
🟠 P1-High:      116 (18%)  - Core engine systems, high priority
🟡 P2-Medium:      5 (1%)   - Gameplay code with error messages
🟢 P3-Low:       174 (27%)  - Test/example code, acceptable
────────────────────────────────────────────────────────────────
    TOTAL:       637 (100%)
```

### Top Risk Crates (P0 + P1)
1. **astraweave-render**: 59 instances (Nanite, materials, GPU resources)
2. **astraweave-scene**: 47 instances (World streaming, async loading)
3. **astraweave-llm**: 38 instances (LLM integration, token handling)
4. **astraweave-context**: 34 instances (Context windows, token budgets)
5. **astraweave-core**: 28 instances (ECS, fundamental systems)
6. **astraweave-ecs**: 24 instances (Archetype management, queries)
7. **astraweave-terrain**: 23 instances (Voxel meshing, marching cubes)
8. **astraweave-embeddings**: 22 instances (Vector operations)
9. **astraweave-memory**: 20 instances (Entity memory, embeddings)
10. **astraweave-ai**: 18 instances (Core AI loop, GOAP planning)

---

## Critical P0 Cases (Top 20)

### 🔴 **Immediate Remediation Required**

#### 1. **astraweave-ai/src/core_loop.rs**
**Lines**: 337, 371, 380  
**Issue**: Core AI loop planning results unwrapped without fallback  
**Impact**: AI system crashes if planning fails (LLM timeout, invalid state)  
**Fix**:
```rust
// Before:
let plan = result.unwrap();

// After:
let plan = result.map_err(|e| {
    error!("AI planning failed: {}", e);
    PlanIntent::default()  // Fallback to idle/defensive plan
})?;
```

#### 2. **astraweave-asset/src/nanite_preprocess.rs**
**Lines**: 623, 829, 874, 926, 982, 983  
**Issue**: Nanite mesh processing pipeline unwraps without error handling  
**Impact**: Asset loading crashes on malformed meshes, corrupt data  
**Fix**:
```rust
// Before:
let meshlets = generate_meshlets(&positions, &normals, &tangents, &uvs, &indices).unwrap();

// After:
let meshlets = generate_meshlets(&positions, &normals, &tangents, &uvs, &indices)
    .context("Failed to generate meshlets for asset")?;
```

#### 3. **astraweave-context/src/history.rs**
**Lines**: 546, 550, 552, 569, 573, 577, 605, 623, 627  
**Issue**: Async context operations unwrap without propagating errors  
**Impact**: Context window crashes on database/storage failures  
**Fix**:
```rust
// Before:
let context = history.get_context(1000).await.unwrap();

// After:
let context = history.get_context(1000).await
    .context("Failed to retrieve conversation history")?;
```

#### 4. **astraweave-context/src/token_counter.rs**
**Lines**: 383, 395, 398, 411, 413, 423, 428, 455, 459, 471, 477  
**Issue**: Token counting/budgeting unwraps without handling overflow/errors  
**Impact**: LLM integration crashes on token limit violations  
**Fix**:
```rust
// Before:
let count = counter.count_tokens("Hello, world!").unwrap();

// After:
let count = counter.count_tokens("Hello, world!")
    .unwrap_or_else(|e| {
        warn!("Token counting failed: {}, using estimate", e);
        text.len() / 4  // Rough estimate: 1 token ≈ 4 chars
    });
```

#### 5. **astraweave-behavior/src/goap.rs**
**Lines**: 381, 408, 433, 434, 435, 469  
**Issue**: GOAP planner unwraps planning results in production paths  
**Impact**: NPC behavior crashes if no valid plan exists  
**Fix**:
```rust
// Before:
let plan = planner.plan(&current_state, &goal, &actions).unwrap();

// After:
let plan = planner.plan(&current_state, &goal, &actions)
    .unwrap_or_else(|e| {
        warn!("GOAP planning failed for goal {:?}: {}", goal, e);
        Vec::new()  // Fallback to idle/defensive actions
    });
```

#### 6. **astraweave-cinematics/src/lib.rs**
**Lines**: 168, 170, 172, 174, 194, 195  
**Issue**: Timeline serialization/stepping unwraps  
**Impact**: Cutscenes crash on invalid timelines or step errors  
**Fix**:
```rust
// Before:
let evs = seq.step(0.5, &tl).unwrap();

// After:
let evs = seq.step(0.5, &tl)
    .map_err(|e| {
        error!("Cinematics sequencer error at t=0.5: {}", e);
        e
    })?;
```

---

## Remediation Strategy

### Phase 1: Critical Path Protection (P0 - Week 1)
**Target**: Fix 50 most critical unwraps in hot paths  
**Focus**:
- ✅ AI core loop (astraweave-ai)
- ✅ Asset loading pipeline (astraweave-asset)
- ✅ Context management (astraweave-context)
- ✅ GOAP planning (astraweave-behavior)

**Approach**:
1. **Replace with `?` operator** where error propagation is appropriate
2. **Add fallback logic** for non-critical failures (token counting, planning)
3. **Add logging** to capture error context before fallback
4. **Add tests** to verify error paths work correctly

**Time Estimate**: 8-12 hours (50 fixes @ 10-15 min each)

### Phase 2: Core Engine Hardening (P1 - Week 2)
**Target**: Fix 116 P1 unwraps in core systems  
**Focus**:
- ✅ Rendering pipeline (astraweave-render)
- ✅ Scene streaming (astraweave-scene)
- ✅ ECS internals (astraweave-ecs, astraweave-core)
- ✅ Terrain generation (astraweave-terrain)

**Approach**:
1. **Audit error paths**: Ensure all `Result` types can be handled gracefully
2. **Add retry logic**: For transient failures (file I/O, async operations)
3. **Improve error messages**: Add context to help debugging
4. **Add integration tests**: Verify system stability under error conditions

**Time Estimate**: 12-16 hours

### Phase 3: Cleanup & Documentation (P2/P3 - Week 3)
**Target**: Review remaining 179 cases  
**Focus**:
- ✅ Categorize P2 cases (medium risk)
- ✅ Document P3 cases (acceptable in tests/examples)
- ✅ Update coding guidelines to prevent new unwraps
- ✅ Add clippy lint rules (`unwrap_used`, `expect_used`)

**Approach**:
1. **Add `.cargo/config.toml` lints**:
```toml
[target.'cfg(not(test))']
rustflags = [
    "-Wclippy::unwrap_used",
    "-Wclippy::expect_used",
]
```
2. **Update CONTRIBUTING.md** with error handling best practices
3. **Create PR template** requiring unwrap justification

**Time Estimate**: 4-6 hours

---

## Recommended Fixes (Code Patterns)

### Pattern 1: Simple Error Propagation
```rust
// ❌ Before:
let value = risky_operation().unwrap();

// ✅ After:
let value = risky_operation()
    .context("Failed during risky operation")?;
```

### Pattern 2: Fallback for Non-Critical
```rust
// ❌ Before:
let count = token_counter.count_tokens(text).unwrap();

// ✅ After:
let count = token_counter.count_tokens(text)
    .unwrap_or_else(|e| {
        warn!("Token counting failed: {}, using estimate", e);
        text.len() / 4  // Rough heuristic
    });
```

### Pattern 3: Default for Optional Data
```rust
// ❌ Before:
let last = inputs.last().unwrap();

// ✅ After:
let last = inputs.last().unwrap_or(&DEFAULT_INPUT);
// Or if data is critical:
let last = inputs.last()
    .context("Input list unexpectedly empty")?;
```

### Pattern 4: Retry Logic for Async
```rust
// ❌ Before:
let context = history.get_context(1000).await.unwrap();

// ✅ After:
let context = retry_with_backoff(3, || {
    history.get_context(1000)
}).await.context("Failed to load context after 3 retries")?;
```

---

## Tooling & Automation

### 1. Audit Script (`scripts/audit_unwrap.ps1`)
**Features**:
- ✅ Scans 354 Rust files in ~2 seconds
- ✅ Categorizes by risk level (P0-P3)
- ✅ Generates CSV report with file/line/context
- ✅ Color-coded console output
- ✅ Top 10 crates by unwrap count

**Usage**:
```powershell
.\scripts\audit_unwrap.ps1                  # Default output
.\scripts\audit_unwrap.ps1 -OutputPath custom.csv
```

### 2. GitHub Issue Templates
**Recommended**: Create issues for each P0/P1 crate  
**Template**:
```markdown
Title: [Unwrap Audit] Fix critical unwraps in {crate_name}

**Risk Level**: P0/P1  
**Crate**: {crate_name}  
**Count**: {num_unwraps}  
**Files**: {list_of_files}  

**Description**:
Audit identified {num_unwraps} critical `.unwrap()` calls in {crate_name}.
These can cause runtime panics and should be replaced with proper error handling.

**Acceptance Criteria**:
- [ ] All P0 unwraps replaced with `?` or fallback logic
- [ ] Error messages added for debugging
- [ ] Tests added to verify error paths
- [ ] CI passes without new unwraps

**References**:
- Audit Report: unwrap_audit_report.csv (lines {start}-{end})
- Remediation Guide: UNWRAP_AUDIT_ANALYSIS.md
```

### 3. CI/CD Integration
**Add to `.github/workflows/ci.yml`**:
```yaml
- name: Audit Unwraps
  run: |
    pwsh scripts/audit_unwrap.ps1
    $p0_count = (Import-Csv unwrap_audit_report.csv | Where-Object { $_.RiskLevel -eq "P0-Critical" }).Count
    if ($p0_count -gt 300) {
      Write-Error "P0 unwraps increased to $p0_count (threshold: 300)"
      exit 1
    }
```

---

## Progress Tracking

### Week 1 (Oct 9-13, 2025)
- [x] **Action 3.1**: Create audit script ✅
- [x] **Action 3.2**: Run initial scan (637 found) ✅
- [x] **Action 3.3**: Generate CSV report ✅
- [ ] **Action 3.4**: Create GitHub issues for top 10 crates
- [ ] **Action 3.5**: Fix 10 critical P0 cases (pilot)

### Week 2 (Oct 14-20, 2025)
- [ ] Fix remaining P0 cases (342 → 0)
- [ ] Begin P1 remediation (116 cases)

### Week 3 (Oct 21-27, 2025)
- [ ] Complete P1 fixes
- [ ] Add clippy lints to prevent regressions
- [ ] Update documentation

---

## Metrics & KPIs

### Current State (Baseline)
- **Total Unwraps**: 637
- **P0 Critical**: 342 (54%)
- **P1 High**: 116 (18%)
- **Coverage**: 354 files scanned

### Target State (Week 3)
- **Total Unwraps**: <200 (69% reduction)
- **P0 Critical**: 0 (100% elimination)
- **P1 High**: <20 (83% reduction)
- **CI Enforcement**: Active (prevent new P0/P1)

### Success Criteria
- ✅ Zero P0 unwraps in production code
- ✅ <20 P1 unwraps remaining (with justification comments)
- ✅ CI fails on new P0/P1 unwraps
- ✅ All core crates have <5 unwraps each

---

## Related Documentation

- **Audit Script**: `scripts/audit_unwrap.ps1`
- **CSV Report**: `unwrap_audit_report.csv`
- **Implementation Plan**: IMMEDIATE_ACTIONS_IMPLEMENTATION_PLAN.md
- **Previous Actions**:
  - Action 1: GPU Skinning (Oct 8) ✅
  - Action 2: Combat Physics (Oct 9) ✅
  - Action 3: Unwrap Audit (Oct 9) ✅ **(Current)**
- **Next**: Action 4 - Performance Baselines

---

## Conclusion

The `.unwrap()` audit successfully identified 637 instances across the codebase, with **342 critical cases requiring immediate attention**. The provided remediation strategy offers a phased approach to eliminate panic risks while maintaining code clarity and performance.

**Key Takeaways**:
1. **High Risk Concentration**: 54% of unwraps are in production code paths
2. **Core Systems Affected**: Rendering, AI, asset loading, and context management
3. **Actionable Plan**: Phased remediation over 3 weeks with clear priorities
4. **Automation Ready**: Script and CI integration prevent regressions

**Estimated Total Effort**: 24-34 hours over 3 weeks  
**Risk Reduction**: Eliminates 458 potential panic points (72% of total)

---

_Generated by AstraWeave Copilot - October 9, 2025_
