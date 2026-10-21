# Hermes 2 Pro: Enum Case Fix and Initial Extended Validation

**Date**: January 13, 2025  
**Phase**: 7 (Extended Validation - In Progress)  
**Status**: 🔧 **ENUM CASE FIXED** - Testing in progress  

---

## Immediate Action: Enum Case Fix ✅

### Problem Identified
Phase 7 initial validation revealed that Hermes 2 Pro generated `"speed": "Run"` (capitalized) but the Rust schema expected `"speed": "run"` (lowercase). This caused 50% parse failure rate despite generating tactically valid JSON.

### Root Cause
Few-shot examples in `astraweave-llm/src/prompt_template.rs` showed capitalized enums:
- ❌ `"speed": "Walk|Run|Sprint"`
- ❌ `"direction": "Left|Right"`

Models learn from examples, so Hermes 2 Pro correctly followed the shown format but schema validation rejected it.

### Fix Applied

**Files Modified**:
1. `astraweave-llm/src/prompt_template.rs` (3 changes)

**Changes**:
```rust
// Line 147 - MoveTo speed enum
- {"act": "MoveTo", "x": INT, "y": INT, "speed": "Walk|Run|Sprint"?},
+ {"act": "MoveTo", "x": INT, "y": INT, "speed": "walk|run|sprint"?},

// Line 151 - Strafe direction enum
- {"act": "Strafe", "target_id": INT, "direction": "Left|Right"},
+ {"act": "Strafe", "target_id": INT, "direction": "left|right"},

// Line 165 - Dodge direction enum
- {"act": "Dodge", "direction": "Left|Right"?},
+ {"act": "Dodge", "direction": "left|right"?},

// Line 225 - Few-shot Example 3
- "plan": r#"{"plan_id": "ex3", "steps": [{"act": "Scan", "radius": 15.0}, {"act": "MoveTo", "x": 20, "y": 20, "speed": "Run"}]}"#,
+ "plan": r#"{"plan_id": "ex3", "steps": [{"act": "Scan", "radius": 15.0}, {"act": "MoveTo", "x": 20, "y": 20, "speed": "run"}]}"#,
```

**Compilation**: ✅ Verified with `cargo check -p astraweave-llm`

---

## Post-Fix Validation Test

### Single-Run Validation (After Fix)

**Command**: `cargo run -p hello_companion --release --features llm,ollama,metrics -- --llm --metrics`

**Result**: ✅ **SUCCESS**

| Metric | Value | Notes |
|--------|-------|-------|
| **Parse Success** | ✅ Stage 1/5 (Direct Parse) | No fallback needed! |
| **Response Time** | 53.9s | 17% faster than pre-fix (64.77s) |
| **Plan Quality** | Excellent | Scan → MoveTo → ThrowSmoke |
| **Steps Generated** | 3 | Tactically appropriate |
| **Tier** | FullLLM | No fallback to simplified prompt |

**Generated Plan**:
```json
{
  "plan_id": "p1",
  "steps": [
    {"act": "Scan", "radius": 8.0},
    {"act": "MoveTo", "x": 15, "y": 8, "speed": "run"},
    {"act": "ThrowSmoke", "x": 14, "y": 7}
  ]
}
```

**Analysis**: Model now uses lowercase `"run"` matching the schema! Parse succeeded on first attempt.

---

## Extended Validation Test Suite

### Test Script Created

**File**: `scripts/test_hermes2pro_validation.ps1`  
**Purpose**: Automated testing to collect statistical significance

**Features**:
- Runs hello_companion N times (default: 10, configurable)
- Captures: Parse success, steps, latency, tier, tools used
- Exports to CSV for analysis
- Calculates: Success rate, avg latency, tool frequency
- Color-coded console output (Green/Yellow/Red)

**Usage**:
```powershell
cd scripts
.\test_hermes2pro_validation.ps1 -Iterations 20
```

**Output**:
- Console: Real-time progress + summary statistics
- CSV: `hermes2pro_validation_results.csv` (detailed data)

### Test Matrix (Planned)

| Test Type | Iterations | Purpose | Status |
|-----------|------------|---------|--------|
| **Baseline** | 20 | Success rate after enum fix | 🔄 **IN PROGRESS** (5 runs) |
| **Temperature 0.3** | 10 | Deterministic output | ⏳ Pending |
| **Temperature 0.7** | 10 | Creative output | ⏳ Pending |
| **Simplified Prompt** | 10 | Latency optimization | ⏳ Pending |
| **Complex Scenarios** | 10 | Multi-enemy, low ammo, surrounded | ⏳ Pending |
| **All Tools** | 37 | Exercise each tool once | ⏳ Pending |

---

## Expected Outcomes

### Success Rate Prediction

**Pre-Fix**: 50% (1/2 runs)
- 100% JSON quality, but 50% parse failures (enum case)

**Post-Fix (Expected)**: **75-90%**
- Rationale: Enum case was the ONLY failure mode observed
- Model generated tactically valid JSON both times
- With correct examples, model should follow format consistently

**Target** (Original Specification): 75-85%
- Within reach if enum fix resolves primary failure mode

### Latency Prediction

**First Attempt (13k char prompt)**: ~55-65s
- Observed: 53.9s, 64.77s (avg ~59s)

**Simplified Prompt (2k char)**: ~8-10s
- Observed: 8.46s in Phase 7 initial validation

**Optimization Potential**: 7.7× speedup via shorter prompts (if success rate maintained)

---

## Next Actions (Priority Order)

### 1. Complete Baseline Validation ⏳ **IN PROGRESS**
- Wait for 5-run test to complete (~5 min)
- Analyze results (success rate, latency distribution)
- If >70% success, expand to 20 runs
- If <70%, investigate new failure modes

### 2. Document Baseline Results (30 min)
- Update `HERMES2PRO_MIGRATION_PHASE7_VALIDATION.md`
- Add post-fix success rate comparison
- Create charts (success rate trend, latency distribution)

### 3. Temperature Experimentation (1 hour)
- Test temp=0.3 (deterministic, 10 runs)
- Test temp=0.7 (creative, 10 runs)
- Compare: JSON reliability vs tactical creativity
- Recommendation: Best temperature for production

### 4. Prompt Length Optimization (1 hour)
- Test simplified prompt (2k chars, 10 runs)
- Test full prompt (13k chars, 10 runs)
- Compare: Success rate vs latency tradeoff
- Recommendation: Which prompt for production

### 5. Comprehensive Tool Coverage (2 hours)
- Create 37 scenarios (one per tool)
- Run each scenario once
- Verify: All tools can be generated correctly
- Identify: Tools LLM struggles with

### 6. Edge Case Testing (1 hour)
- Surrounded (3+ enemies)
- Low ammo (<5)
- Low health (<30 HP)
- No cover available
- Multiple objectives
- Verify: LLM adapts tactics appropriately

---

## Success Criteria for Phase 7 Completion

✅ **Enum Case Fixed**: COMPLETE  
⏳ **Baseline Validation**: IN PROGRESS (1/20 runs)  
⏳ **Success Rate >75%**: Pending baseline results  
⏳ **Latency Acceptable**: Target <10s (simplified) or <60s (full)  
⏳ **Temperature Optimized**: Pending experimentation  
⏳ **Documentation Complete**: Pending final results  

**Estimated Time to Completion**: 4-6 hours (including test runtime)

---

## Risks & Mitigation

### Risk 1: Success Rate Still Low (<70%)
**Likelihood**: Low (enum case was only failure mode)  
**Mitigation**: 
- Analyze new failure patterns
- Further prompt refinement
- Consider case-insensitive enum parsing

### Risk 2: High Latency (>60s)
**Likelihood**: Medium (observed 53-64s on full prompt)  
**Mitigation**:
- Use simplified prompt (proven 8.46s)
- Implement prompt caching
- Consider faster model (Hermes 2 Pro mini if available)

### Risk 3: Tactical Quality Degrades
**Likelihood**: Very Low (100% quality observed so far)  
**Mitigation**:
- Monitor plan appropriateness across scenarios
- Compare vs Behavior Tree baseline
- Add fallback to heuristic if LLM plan seems irrational

---

## Timeline

| Time | Activity | Status |
|------|----------|--------|
| **T+0** | Enum case fix applied | ✅ COMPLETE |
| **T+5min** | Single validation test | ✅ SUCCESS |
| **T+10min** | Test script created | ✅ COMPLETE |
| **T+15min** | 5-run baseline started | 🔄 IN PROGRESS |
| **T+20min** | Baseline results available | ⏳ Pending |
| **T+1hr** | 20-run extended baseline | ⏳ Pending |
| **T+2hrs** | Temperature tests (0.3, 0.7) | ⏳ Pending |
| **T+3hrs** | Prompt length comparison | ⏳ Pending |
| **T+5hrs** | Tool coverage + edge cases | ⏳ Pending |
| **T+6hrs** | Final documentation | ⏳ Pending |

**Current Progress**: ~10% (enum fix + single test + script)  
**Remaining**: ~90% (statistical validation + experimentation)

---

## Files Modified (This Session)

1. ✅ `astraweave-llm/src/prompt_template.rs` (enum case fixes)
2. ✅ `scripts/test_hermes2pro_validation.ps1` (validation automation)
3. ✅ `HERMES2PRO_MIGRATION_PHASE7_VALIDATION.md` (initial report)
4. 🔄 `hermes2pro_validation_results.csv` (being generated)

---

**Report Status**: Preliminary (waiting for extended validation results)  
**Next Update**: After 5-run baseline completes (~5 minutes)  
**Author**: GitHub Copilot (AI-generated documentation)  
**Project**: AstraWeave AI-Native Game Engine
