# Hermes 2 Pro Migration - Phase 1 Audit Report
**Date**: October 15, 2025  
**Status**: ✅ COMPLETE  
**Total References Found**: 800+ matches across 50+ files

---

## Executive Summary

Comprehensive audit of all Phi-3 references in the AstraWeave codebase identified **50+ files** containing "phi3", "phi-3", or "PHI3" strings. The migration to Hermes 2 Pro Mistral 7B will touch:

- **7 Rust source files** (core implementation)
- **3 Modelfiles** (Ollama configurations)
- **30+ documentation files** (guides, reports, summaries)
- **5+ example files** (demos, benchmarks)
- **2 CSV reports** (unwrap audits)

---

## Critical Findings

### 1. **Primary Implementation Files** (MUST CHANGE)

These are the **core files** that must be updated for Hermes 2 Pro to work:

| File | Type | Priority | References |
|------|------|----------|------------|
| `astraweave-llm/src/phi3_ollama.rs` | Rust | P0-CRITICAL | 100+ (filename, module docs, model names) |
| `examples/hello_companion/src/main.rs` | Rust | P0-CRITICAL | 30+ (model name, comments, output) |
| `astraweave-llm/src/lib.rs` | Rust | P1-HIGH | 10+ (module exports, feature flags, debug output) |
| `astraweave-llm/src/phi3.rs` | Rust | P2-MEDIUM | 80+ (entire file - optional candle integration) |
| `astraweave-llm/Cargo.toml` | TOML | P1-HIGH | 2+ (feature flags, comments) |

### 2. **Ollama Model Configuration Files** (MUST UPDATE)

| File | Purpose | Action |
|------|---------|--------|
| `Modelfile.phi3-game` | Gaming-optimized phi3:3.8b | Rename + update to hermes2pro |
| `Modelfile.phi3-fast` | Low-latency phi3:medium | Rename + update to hermes2pro |

### 3. **Documentation Files** (SHOULD UPDATE)

30+ documentation files contain Phi-3 references. **Key ones**:

| File | Type | Priority | Phi-3 Mentions |
|------|------|----------|----------------|
| `README.md` | Core docs | P0 | 5+ |
| `.github/copilot-instructions.md` | AI assistant | P0 | 10+ |
| `PHASE_6_COMPLETION_SUMMARY.md` | Status report | P1 | 15+ |
| `PHASE_7_VALIDATION_REPORT.md` | Validation | P1 | 10+ |
| `PHI3_VALIDATION_FIX_SUMMARY.md` | Bug fix deep dive | P1 | 50+ |
| `docs/PHI3_SETUP.md` | Setup guide | P0 | 30+ |
| `examples/phi3_demo/README.md` | Demo docs | P2 | 20+ |

**Action**: Update all to reference "Hermes 2 Pro" instead of "Phi-3"

### 4. **Example & Demo Files** (SHOULD UPDATE)

| Directory/File | Purpose | Action |
|----------------|---------|--------|
| `examples/phi3_demo/` | Standalone demo (3 files) | Rename directory + update code |
| `examples/hello_companion/` | Primary demo | Update model name + docs |
| `Cargo.toml` (root) | Workspace member | Update `phi3_demo` reference |

### 5. **Historical Documentation** (MAY KEEP AS-IS)

These files document **completed phases** and can optionally retain Phi-3 references as historical context:

- `WEEK_4_ACTION_17_PHI3_COMPLETE.md` - Week 4 completion report
- `PHI3_OPTIMIZATION_COMPLETE.md` - Optimization journey
- `PHI3_IMPLEMENTATION_COMPLETE.md` - Implementation summary
- `PHI3_DAY1_COMPLETE.md`, `PHI3_DAY2_COMPLETE.md`, `PHI3_DAY3_COMPLETE.md` - Daily reports
- `PHI3_QUICK_START.md` - Quickstart (deprecated)
- `PHI3_INTEGRATION_FINAL_REPORT.md` - Final report
- `PHI3_PROMPT_TUNING_ANALYSIS.md` - Prompt analysis

**Recommendation**: Add header disclaimers like:
```markdown
> **HISTORICAL DOCUMENT**: This describes the original Phi-3 integration. 
> AstraWeave has since migrated to Hermes 2 Pro. See [HERMES2PRO_MIGRATION_COMPLETE.md](...)
```

---

## Model-Specific Code Analysis

### Current Phi-3 Implementation (`phi3_ollama.rs`)

**Model variants detected:**
```rust
// Current Phi-3 variants
"phi3:medium"  // 14B, 7.9GB Q4 - Primary model
"phi3:game"    // 3.8B, 2.3GB - Gaming optimized
"phi3:3.8b"    // Mini variant
```

**Chat format** (lines 236-273):
```rust
// Phi-3 uses custom format (not explicitly defined in code)
// Ollama handles this internally
```

**Context window**:
- Not explicitly set (Ollama default for Phi-3 is 4096 tokens)
- Hermes 2 Pro: 8192 tokens (2× larger)

**System prompt** (line 39-56):
```rust
const DEFAULT_SYSTEM_PROMPT: &str = r#"You are a tactical AI agent in a combat scenario.
Your role is to analyze the battlefield and generate optimal action sequences.
..."#;
```

**Key finding**: No Phi-3-specific chat format in code! Ollama handles it.

---

### Required Code Changes

**1. Model Name Changes**

| Current | Hermes 2 Pro Equivalent |
|---------|------------------------|
| `phi3:medium` | `adrienbrault/nous-hermes2pro:Q4_K_M` |
| `phi3:game` | `adrienbrault/nous-hermes2pro:Q4_K_M` (same model) |
| `phi3:3.8b` | `adrienbrault/nous-hermes2pro:Q4_K_M` (7B > 3.8B) |

**2. Chat Format Changes**

**Phi-3** (internal to Ollama):
```xml
<|system|>...<|user|>...<|assistant|>...
```

**Hermes 2 Pro** (ChatML):
```xml
<|im_start|>system...<|im_end|>
<|im_start|>user...<|im_end|>
<|im_start|>assistant...<|im_end|>
```

**Impact**: Ollama handles chat formatting automatically. **No code changes needed** for chat format!

**3. Context Window**

```rust
// No change needed - Ollama will use Hermes 2 Pro's 8192 token window
// automatically. This is a FREE upgrade (4096 → 8192)
```

**4. Function Calling**

Hermes 2 Pro has **native function calling** support. Current implementation uses JSON plans.

**Option A**: Keep existing JSON plan format (works with any LLM)  
**Option B**: Migrate to OpenAI-compatible function calling (Hermes 2 Pro native)

**Recommendation**: Start with Option A (minimal changes), add Option B in Phase 8

---

## File-by-File Breakdown

### Core Implementation (7 files)

**1. `astraweave-llm/src/phi3_ollama.rs` (394 lines)**

**Changes**:
- Filename: `phi3_ollama.rs` → `hermes2pro_ollama.rs`
- Struct: `Phi3Ollama` → `Hermes2ProOllama`
- Model defaults: `"phi3:medium"` → `"adrienbrault/nous-hermes2pro:Q4_K_M"`
- All docs/comments mentioning Phi-3

**Lines to change**: 100+

**2. `astraweave-llm/src/lib.rs` (1,233 lines)**

**Changes**:
- Line 93: Debug output `"PROMPT SENT TO PHI-3"` → `"HERMES 2 PRO"`
- Line 132: Debug output `"PHI-3 RAW RESPONSE"` → `"HERMES 2 PRO"`
- Line 1228-1229: Feature flag `#[cfg(feature = "phi3")]`
- Line 1229: Module `pub mod phi3;`
- Line 1231: Comment `"// Phi-3 via Ollama"`
- Line 1233: Module `pub mod phi3_ollama;`

**Lines to change**: 7

**3. `astraweave-llm/src/phi3.rs` (483 lines)**

**OPTIONAL FILE** - Candle-based Phi-3 integration (feature-gated, not used in production)

**Options**:
A. Keep as-is (deprecated, feature-gated)
B. Delete entirely (unused)
C. Rename to `deprecated_phi3.rs` with header

**Recommendation**: Option A (minimal effort)

**4. `examples/hello_companion/src/main.rs` (800 lines)**

**Changes**:
- Line 1: File header comment
- Line 7: Features comment
- Line 20: Example comment
- Line 56: Enum variant comment
- Line 72: Display impl
- Line 458: Warning message
- Line 712: Section header comment
- Line 717: Output message
- Line 722-723: Model name and comment
- Line 726: Actual model name `"phi3:game"`
- Line 743-747: Output messages
- Line 784-795: Model validation logic

**Lines to change**: 30+

**5. `examples/hello_companion/README.md` (214 lines)**

**Changes**: 8 mentions of "Phi-3" in docs

**6. `astraweave-llm/Cargo.toml`**

**Changes**:
- Line 24: Comment `"# Phi-3 Medium Q4 inference"`
- Line 50: Feature flag `phi3 = [...]`

**Lines to change**: 2

**7. `astraweave-ai/src/orchestrator.rs` (302 lines)**

**Changes**: 3 comments mentioning "Phi-3"

---

### Documentation Files (30+ files)

**Priority 0 (User-Facing)**:
1. `README.md` - 5 mentions
2. `.github/copilot-instructions.md` - 10 mentions
3. `docs/PHI3_SETUP.md` - 30+ mentions (rename to `HERMES2PRO_SETUP.md`)

**Priority 1 (Phase Reports)**:
1. `PHASE_6_COMPLETION_SUMMARY.md` - 15 mentions
2. `PHASE_7_VALIDATION_REPORT.md` - 10 mentions
3. `PHI3_VALIDATION_FIX_SUMMARY.md` - 50+ mentions (rename to `PHASE_7_VALIDATION_FIX_SUMMARY.md`)
4. `PHASE_6_AND_7_ROADMAP.md` - 4 mentions

**Priority 2 (Historical)**:
- 20+ completion reports, optimization docs, integration guides

**Action**: Bulk find/replace with human review for context

---

### Example Directory Cleanup

**`examples/phi3_demo/`** (3 files, 500+ lines total):

**Files**:
- `Cargo.toml`
- `src/main.rs`
- `src/bin/bench.rs`
- `README.md`

**Actions**:
1. Rename directory: `phi3_demo` → `hermes2pro_demo`
2. Update `Cargo.toml` root workspace member
3. Update all source files + docs

---

### Modelfiles (2 files)

**`Modelfile.phi3-game`**:
```dockerfile
FROM phi3:3.8b
PARAMETER temperature 0.5
PARAMETER top_p 0.9
PARAMETER top_k 40
PARAMETER num_ctx 4096
```

**New `Modelfile.hermes2pro-game`**:
```dockerfile
FROM adrienbrault/nous-hermes2pro:Q4_K_M
PARAMETER temperature 0.5
PARAMETER top_p 0.9
PARAMETER top_k 40
PARAMETER num_ctx 8192  # 2× larger!
```

**`Modelfile.phi3-fast`**: Same updates

---

## Chat Format Migration (GOOD NEWS!)

**Critical finding**: Ollama handles chat format **automatically** based on model.

**Phi-3** (Ollama internal):
```rust
// No explicit format in code - Ollama handles it
```

**Hermes 2 Pro** (Ollama internal):
```rust
// No changes needed - Ollama will use ChatML automatically
```

**Conclusion**: **NO CHAT FORMAT CODE CHANGES REQUIRED!** ✅

Ollama abstracts this completely. When we switch from `phi3:medium` to `adrienbrault/nous-hermes2pro:Q4_K_M`, Ollama will automatically use the correct chat template.

---

## Context Window Migration

**Phi-3**: 4096 tokens (implicit default)  
**Hermes 2 Pro**: 8192 tokens (2× larger)

**Code impact**: None! Ollama will use model's default.

**Benefits**:
- More tool descriptions (37 tools fit easily)
- Longer conversation history
- More few-shot examples

---

## Function Calling (Future Enhancement)

**Current**: JSON plan format (model-agnostic)
```json
{
  "reasoning": "...",
  "actions": [
    {"tool": "MoveTo", "args": {"x": 10, "y": 5}}
  ]
}
```

**Hermes 2 Pro Native** (OpenAI-compatible):
```json
{
  "tool_calls": [
    {
      "id": "call_1",
      "type": "function",
      "function": {
        "name": "MoveTo",
        "arguments": "{\"x\": 10, \"y\": 5}"
      }
    }
  ]
}
```

**Migration Strategy**:
1. **Phase 3**: Keep existing JSON format (zero risk)
2. **Phase 8** (Optional): Add native function calling support

---

## Feature Flags Review

**Current**:
```toml
[features]
phi3 = ["candle-core", "candle-nn", ...]  # Optional candle integration
ollama = ["reqwest", "serde_json"]         # Ollama backend (USED)
llm = ["ollama"]                           # Alias
```

**Proposed**:
```toml
[features]
# Deprecated - kept for backwards compatibility
phi3 = ["candle-core", "candle-nn", ...]  

# New Hermes 2 Pro (no changes needed - same Ollama backend)
ollama = ["reqwest", "serde_json"]
llm = ["ollama"]
```

**Recommendation**: No feature flag changes needed. The `ollama` feature already supports any Ollama model.

---

## Expected Performance Improvements

Based on Hermes 2 Pro benchmarks and AstraWeave's current metrics:

| Metric | Phi-3 (Current) | Hermes 2 Pro (Expected) | Improvement |
|--------|-----------------|------------------------|-------------|
| **Success Rate** | 40-50% | 75-85% | +35-40% ✅ |
| **Latency** | 3-5s | 2-4s | -20-25% ✅ |
| **JSON Parse Errors** | 50% | <10% | -80% ✅ |
| **Tool Hallucinations** | 40% | <5% | -88% ✅ |
| **Context Window** | 4096 tokens | 8192 tokens | 2× ✅ |
| **Model Size** | 2.2GB (game) / 7.9GB (medium) | 4.4GB (Q4_K_M) | Consistent |

---

## Risk Assessment

**Low Risk**:
- ✅ Ollama backend unchanged (same client code)
- ✅ No chat format code changes needed
- ✅ Same JSON plan format works
- ✅ Context window increase (no downsides)
- ✅ Model size similar (4.4GB vs 2.2-7.9GB range)

**Medium Risk**:
- ⚠️ Model name must be exact: `adrienbrault/nous-hermes2pro:Q4_K_M`
- ⚠️ Prompt tuning may be needed (different training data)
- ⚠️ Success rate improvement unproven (need validation)

**High Risk**:
- ❌ None identified

---

## Migration Checklist

### Phase 1: Audit (COMPLETE)
- [x] Find all Phi-3 references (800+ matches)
- [x] Categorize files (core, docs, examples)
- [x] Identify model-specific code
- [x] Analyze chat format requirements
- [x] Review feature flags

### Phase 2: Specs (Next)
- [ ] Document Hermes 2 Pro model details
- [ ] Confirm Ollama model name
- [ ] Test Hermes 2 Pro installation
- [ ] Validate chat format compatibility

### Phase 3: Code Migration
- [ ] Rename `phi3_ollama.rs` → `hermes2pro_ollama.rs`
- [ ] Update model names in code
- [ ] Update debug output messages
- [ ] Update comments and docs
- [ ] Run `cargo check -p astraweave-llm`
- [ ] Run `cargo check -p hello_companion`

### Phase 4: Documentation
- [ ] Update README.md
- [ ] Update copilot-instructions.md
- [ ] Update setup guides
- [ ] Add migration disclaimer to historical docs

### Phase 5: Examples
- [ ] Rename `phi3_demo` → `hermes2pro_demo`
- [ ] Update hello_companion
- [ ] Update benchmarks

### Phase 6: Modelfiles
- [ ] Create `Modelfile.hermes2pro-game`
- [ ] Create `Modelfile.hermes2pro-fast`
- [ ] Test Ollama build

### Phase 7: Validation
- [ ] Install `adrienbrault/nous-hermes2pro:Q4_K_M`
- [ ] Quick inference test
- [ ] Run `hello_companion --demo-all`
- [ ] Benchmark comparison

### Phase 8: Cleanup
- [ ] Remove old Modelfiles (optional)
- [ ] Deprecate `phi3.rs` (optional)
- [ ] Update metrics baselines

---

## Files Summary Table

| Category | File Count | Priority | Action |
|----------|------------|----------|--------|
| **Core Rust** | 7 | P0 | Rename + update model names |
| **Modelfiles** | 2 | P1 | Recreate with Hermes 2 Pro |
| **User Docs** | 3 | P0 | Update references |
| **Phase Reports** | 4 | P1 | Update references |
| **Historical Docs** | 20+ | P2 | Add disclaimers |
| **Examples** | 5 | P1 | Rename directory + update |
| **Benchmarks** | 3 | P2 | Update references |
| **Test Files** | 2 | P1 | Update assertions |

**Total files to modify**: ~50

---

## Next Steps

1. **Install Hermes 2 Pro**:
   ```bash
   ollama pull adrienbrault/nous-hermes2pro:Q4_K_M
   ```

2. **Quick Test**:
   ```bash
   ollama run adrienbrault/nous-hermes2pro:Q4_K_M "Generate a JSON plan..."
   ```

3. **Proceed to Phase 3**: Code migration (file renames + model names)

---

**Timeline Estimate**: 3-4 hours (revised from 4-5 hours due to chat format simplification)

**Status**: ✅ **Phase 1 COMPLETE** - Ready for Phase 3 code migration

**Next Phase**: [HERMES2PRO_MIGRATION_PHASE3_CODE.md](HERMES2PRO_MIGRATION_PHASE3_CODE.md)
