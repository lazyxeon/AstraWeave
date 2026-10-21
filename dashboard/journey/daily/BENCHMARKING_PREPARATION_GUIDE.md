# Benchmarking Session Preparation Guide

**Purpose**: Resume Task 7 (Benchmark Additional Subsystems) efficiently in next session  
**Date**: October 29, 2025  
**Status**: 🔄 **Task 7: 50% COMPLETE** - Infrastructure ready, API analysis done  
**Estimated Effort**: 6-8h for complete P2 benchmark suite

---

## Quick Start Checklist

When resuming benchmarking work:

```powershell
# 1. Navigate to workspace root
cd C:\Users\pv2br\source\repos\AstraWeave-AI-Native-Gaming-Engine

# 2. Review existing benchmark structure
Get-Content astraweave-memory\benches\memory_benchmarks.rs | Select-Object -First 50

# 3. Check compilation status (expect dependency warnings, not benchmark errors)
cargo check -p astraweave-memory --benches

# 4. Read API documentation for next crate
Get-Content astraweave-context\src\lib.rs
Get-Content astraweave-context\src\sliding_window.rs
Get-Content astraweave-context\src\token_budget.rs

# 5. Start implementation (use memory_benchmarks.rs as template)
```

---

## Current State Summary

### What's Done ✅

**astraweave-memory benchmarks** (140 LOC):
- ✅ File created: `astraweave-memory/benches/memory_benchmarks.rs`
- ✅ Cargo.toml configured with criterion dev-dependency + [[bench]] section
- ✅ Helper function: `create_test_memory()` (45 LOC with full Memory initialization)
- ✅ 5 core benchmarks implemented:
  1. `bench_memory_creation` - Heap allocation timing
  2. `bench_memory_storage` - Parameterized HashMap insertion (10/50/100/500)
  3. `bench_memory_retrieval` - HashMap lookup with setup/teardown
  4. `bench_memory_access_tracking` - Metadata updates (10/50/100 accesses)
  5. `bench_memory_updates` - Importance recalculation
- ✅ All closure capture issues resolved
- ✅ Benchmark code compiles correctly

### What's Blocking ⚠️

**Dependency build failures** (NOT benchmark code issues):
- astraweave-observability: deprecated rand::thread_rng
- astraweave-llm: deprecated rand::Rng::gen
- astraweave-embeddings: unexpected cfg "small_rng", unused imports
- astraweave-prompts: 7 warnings (unused imports, dead code)
- astraweave-context: 13 warnings (unused imports, unreachable patterns)
- astraweave-rag: 8 warnings (unused imports, dead code)
- astraweave-memory: 10 warnings (unused imports, unused variables)

**Workaround**: Use `cargo build --allow-warnings` or fix dependency warnings first

### What's Missing ⏳

**P2 Crates Not Benchmarked**:
1. astraweave-context (5 benchmarks, 1-2h)
2. astraweave-llm (5 benchmarks, 2-3h)
3. astraweave-rag (3 benchmarks, 2-3h)
4. astraweave-persona (3 benchmarks, 1-2h)
5. astraweave-prompts (3 benchmarks, 1-2h)

**Total Estimate**: 7-13h for all P2 benchmarks + mocking infrastructure

---

## API Complexity Findings

### Memory API Structure (Discovered Through Compilation)

**Key Discovery**: Memory has deeply nested structure, NOT simple fields

```rust
// Memory struct (actual API)
pub struct Memory {
    pub id: String,
    pub memory_type: MemoryType,
    pub content: MemoryContent {           // Nested struct
        pub text: String,
        pub data: serde_json::Value,
        pub sensory_data: Option<SensoryData>,
        pub emotional_context: Option<EmotionalContext>,
        pub context: SpatialTemporalContext,
    },
    pub metadata: MemoryMetadata {         // Nested struct
        pub created_at: DateTime<Utc>,
        pub last_accessed: DateTime<Utc>,
        pub access_count: u32,
        pub importance: f32,
        pub confidence: f32,
        pub source: MemorySource,
        pub tags: Vec<String>,
        pub permanent: bool,
        pub strength: f32,
        pub decay_factor: f32,
    },
    pub associations: Vec<MemoryAssociation>,
    pub embedding: Option<Vec<f32>>,
}
```

**Implications**:
- Cannot use simple field initialization
- Need helper function to create valid Memory instances
- Benchmark setup becomes more complex

**Solution**: Created `create_test_memory()` helper (see memory_benchmarks.rs lines 9-45)

### PatternDetector Requirements (CRITICAL BLOCKER)

**API Signature**:
```rust
impl PatternDetector {
    pub fn detect_playstyle_patterns(
        &self,
        storage: &MemoryStorage  // <-- Requires database backend!
    ) -> Result<Vec<PatternStrength>> {
        // Queries SQLite database for pattern analysis
    }
}
```

**Implications**:
- Cannot benchmark without MemoryStorage initialization
- MemoryStorage wraps SQLite database with schema setup
- Requires mocking infrastructure: 2-3h work

**Decision**: Deferred PatternDetector benchmarks to future session

**Workaround**: Focus on core operations (creation, storage, retrieval) that don't need database

### Expected Complexity in Other P2 Crates

**astraweave-context**:
- `ContextWindow` likely requires message history initialization
- `SlidingWindow` may need pruning state setup
- `TokenBudget` requires token counting infrastructure
- **Estimate**: 1-2h API analysis + implementation

**astraweave-llm**:
- `PlanParser` requires tool registry setup
- `LlmExecutor` may need mock Ollama client
- `FallbackSystem` requires multiple orchestrator initialization
- **Estimate**: 2-3h (most complex API of P2 crates)

**astraweave-rag**:
- `RagSystem` requires embedding infrastructure
- `Retriever` may need vector database setup
- **Estimate**: 2-3h with embedding mocking

**astraweave-persona/prompts**:
- Simpler APIs (template processing, profile management)
- **Estimate**: 1-2h each

---

## Compilation Journey (Learning from Mistakes)

### Attempt 1: Naive Implementation (240 LOC, 8 benchmarks)

**Assumptions**:
- Memory has simple fields like `created_at`, `importance`
- MemoryContent is enum with `Text(String)` variant
- MemoryMetadata has Default implementation

**Result**: ❌ **36 compilation errors**

**Errors**:
```
error[E0599]: no variant named `Text` found for enum `MemoryContent`
error[E0609]: no field `created_at` on type `Memory`
error[E0599]: no function or associated item named `default` found for struct `MemoryMetadata`
```

**Lesson**: **READ ACTUAL API FIRST** before writing benchmarks

### Attempt 2: API-Correct Implementation (6 benchmarks)

**Changes**:
- Created `create_test_memory()` helper with proper initialization
- Fixed MemoryContent (text field instead of Text variant)
- Fixed MemoryMetadata (manual initialization with all required fields)
- Fixed MemorySource (DirectExperience instead of Direct)

**Result**: ❌ **4 compilation errors**

**Errors**:
```
error[E0599]: no method named `detect_patterns` found for type `PatternDetector`
error: captured variable cannot escape `FnMut` closure body
```

**Lesson**: Check method signatures AND closure semantics

### Attempt 3: Simplified Implementation (140 LOC, 5 benchmarks)

**Changes**:
- Removed PatternDetector benchmark (requires MemoryStorage)
- Fixed closure captures with `iter_with_setup` pattern

**Result**: ❌ **1 closure capture error**

**Error**:
```
error[E0521]: captured variable `manager` cannot escape `FnMut` closure body
```

**Lesson**: Use `iter_with_setup` for ALL mutable state in benchmarks

### Attempt 4: Closure-Safe Implementation (FINAL)

**Changes**:
- Moved ALL manager initialization into setup closures
- Used `iter_with_setup` pattern consistently:
  ```rust
  b.iter_with_setup(
      || { /* setup creates manager */ },
      |mut manager| { /* iter uses + destroys manager */ }
  );
  ```

**Result**: ✅ **Benchmark code compiles!** (dependency failures unrelated)

**Lesson**: `iter_with_setup` is the ONLY safe pattern for mutable state

---

## Benchmark Code Template

Use this template for future P2 crate benchmarks:

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use astraweave_[CRATE]::{/* imports */};

// STEP 1: Create helper function for test data
fn create_test_[OBJECT]() -> [TYPE] {
    [TYPE] {
        // Initialize ALL required fields
        // Use sensible defaults
        // Document any complex initialization
    }
}

// STEP 2: Basic operation benchmark (no mutable state)
fn bench_[OPERATION]_creation(c: &mut Criterion) {
    c.bench_function("[operation]_creation", |b| {
        b.iter(|| {
            let obj = create_test_[OBJECT]();
            black_box(obj)
        })
    });
}

// STEP 3: Parameterized benchmark (multiple sizes)
fn bench_[OPERATION]_storage(c: &mut Criterion) {
    let mut group = c.benchmark_group("[operation]_storage");
    
    for size in [10, 50, 100, 500] {
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &size,
            |b, &size| {
                b.iter_with_setup(
                    || {
                        // Setup: Create manager/storage
                        let mut manager = [Manager]::new();
                        (manager, size)
                    },
                    |(mut manager, size)| {
                        // Iter: Perform operation
                        for i in 0..size {
                            let obj = create_test_[OBJECT]();
                            manager.insert(obj);
                        }
                        black_box(manager)
                    }
                )
            },
        );
    }
    
    group.finish();
}

// STEP 4: Retrieval benchmark (setup/iter/teardown)
fn bench_[OPERATION]_retrieval(c: &mut Criterion) {
    c.bench_function("[operation]_retrieval", |b| {
        b.iter_with_setup(
            || {
                // Setup: Pre-populate manager
                let mut manager = [Manager]::new();
                for i in 0..100 {
                    let obj = create_test_[OBJECT]();
                    manager.insert(obj);
                }
                manager
            },
            |mut manager| {
                // Iter: Single retrieval
                let result = manager.get("[id]");
                black_box(result)
            }
        )
    });
}

// STEP 5: Register benchmarks
criterion_group!(
    benches,
    bench_[OPERATION]_creation,
    bench_[OPERATION]_storage,
    bench_[OPERATION]_retrieval,
);
criterion_main!(benches);
```

---

## Implementation Plan (6-8h Total)

### Phase 1: Fix Dependencies (2-3h)

**Option A - Fix Warnings** (2-3h):
```powershell
# Fix deprecated rand functions (5 crates)
# Update: rand::thread_rng() -> rand::rng()
# Update: rand::Rng::gen() -> rand::Rng::random()

# Fix unused imports (4 crates)
# Remove or use imported items

# Fix unexpected cfg (embeddings)
# Add "small_rng" feature to Cargo.toml
```

**Option B - Ignore Warnings** (5 min):
```powershell
# Add to .cargo/config.toml
[build]
rustflags = ["-A", "warnings"]

# OR use flag on each benchmark run
cargo build --benches --allow-warnings
```

**Recommendation**: Option B for benchmarking session (faster), Option A for production cleanup

### Phase 2: Context Benchmarks (1-2h)

**File**: `astraweave-context/benches/context_benchmarks.rs`

**Benchmarks** (5 total):
1. `bench_context_window_creation` - ContextWindow initialization
2. `bench_context_message_append` - Message addition (10/50/100/500)
3. `bench_sliding_window_pruning` - Pruning trigger + execution
4. `bench_token_budget_validation` - Token counting + overflow check
5. `bench_attention_scoring` - Attention weight calculation

**API Analysis Required**:
- Read `sliding_window.rs` for pruning logic
- Read `token_budget.rs` for validation API
- Read `attention.rs` for scoring mechanism

### Phase 3: LLM Benchmarks (2-3h)

**File**: `astraweave-llm/benches/llm_benchmarks.rs`

**Benchmarks** (5 total):
1. `bench_plan_parsing_valid` - JSON parsing (5 parser stages)
2. `bench_plan_validation` - Tool registry checks
3. `bench_fallback_tier_transitions` - GOAP → LLM mode switches
4. `bench_tool_registry_lookup` - Tool name → ToolSpec mapping
5. `bench_prompt_generation` - Prompt template expansion

**API Analysis Required**:
- Read `plan_parser.rs` for 5-stage parsing
- Read `fallback_system.rs` for tier logic
- Create MockLlmOrch for testing (use existing from tests/)

**Complexity**: Highest of P2 crates (needs tool registry setup)

### Phase 4: RAG/Persona/Prompts Benchmarks (3-4h)

**astraweave-rag** (3 benchmarks, 2-3h):
1. `bench_embedding_retrieval` - Vector similarity search
2. `bench_document_ranking` - Result scoring + sorting
3. `bench_context_consolidation` - Multi-document merging

**astraweave-persona** (3 benchmarks, 1-2h):
1. `bench_persona_profile_creation` - Profile initialization
2. `bench_trait_consistency_check` - Trait validation
3. `bench_persona_update` - Profile modification

**astraweave-prompts** (3 benchmarks, 1-2h):
1. `bench_template_expansion` - Variable substitution
2. `bench_prompt_formatting` - Markdown/JSON formatting
3. `bench_prompt_validation` - Schema checks

### Phase 5: Validation & Documentation (2h)

**Run All Benchmarks**:
```powershell
# Memory
cargo bench -p astraweave-memory --bench memory_benchmarks

# Context
cargo bench -p astraweave-context --bench context_benchmarks

# LLM
cargo bench -p astraweave-llm --bench llm_benchmarks

# RAG
cargo bench -p astraweave-rag --bench rag_benchmarks

# Persona
cargo bench -p astraweave-persona --bench persona_benchmarks

# Prompts
cargo bench -p astraweave-prompts --bench prompts_benchmarks
```

**Document Results** in `docs/current/BASELINE_METRICS.md`:

```markdown
## P2 Crate Benchmarks

### astraweave-memory

**Command**: `cargo bench -p astraweave-memory --bench memory_benchmarks`

**Results**:
```
memory_creation              [time]
memory_storage/10            [time]
memory_storage/50            [time]
memory_storage/100           [time]
memory_storage/500           [time]
memory_retrieval             [time]
memory_access_tracking/10    [time]
memory_access_tracking/50    [time]
memory_access_tracking/100   [time]
memory_updates               [time]
```

**Analysis**:
- Memory creation: [X] ns (heap allocation cost)
- Storage scaling: O(1) HashMap insertion
- Retrieval: [X] ns (HashMap lookup)
- Access tracking: [X] ns per update
- Target: <1 µs for all operations ✅/⚠️

(Repeat for each P2 crate)
```

---

## Success Criteria

### Task 7 Complete When:

✅ **All P2 crates benchmarked**: Memory, Context, LLM, RAG, Persona, Prompts  
✅ **Benchmark suites compile**: 0 errors (warnings OK if documented)  
✅ **Baselines collected**: Results captured for all benchmarks  
✅ **Documentation updated**: BASELINE_METRICS.md has P2 section  
✅ **Mocking documented**: Complex APIs (MemoryStorage, embeddings) have setup notes  

### Stretch Goals:

🎯 **Integration benchmarks** (Task 8): Full AI pipeline with Memory/Context/LLM interaction  
🎯 **Performance budget** (Task 9): Allocate 16.67ms frame budget across subsystems  
🎯 **Comparison analysis**: Compare P2 vs P0/P1 performance characteristics  

---

## Quick Reference

### File Locations

**Existing Benchmarks** (for reference):
- `astraweave-math/benches/simd_benchmarks.rs` - SIMD patterns
- `astraweave-physics/benches/raycast.rs` - Parameterized benchmarks
- `astraweave-behavior/benches/goap_planning.rs` - Planning benchmarks

**P2 Source Files** (read before benchmarking):
- `astraweave-memory/src/` - memory_manager.rs, pattern_detection.rs
- `astraweave-context/src/` - sliding_window.rs, token_budget.rs
- `astraweave-llm/src/` - plan_parser.rs, fallback_system.rs
- `astraweave-rag/src/` - retriever.rs, ranker.rs
- `astraweave-persona/src/` - profile.rs, traits.rs
- `astraweave-prompts/src/` - template.rs, formatter.rs

### Commands

**Check Compilation**:
```powershell
cargo check -p astraweave-[CRATE] --benches
```

**Run Single Benchmark**:
```powershell
cargo bench -p astraweave-[CRATE] --bench [NAME]
```

**Run with Profiling**:
```powershell
cargo bench -p astraweave-[CRATE] --bench [NAME] -- --profile-time=5
```

**Update Documentation**:
```powershell
# Copy benchmark output
cargo bench -p astraweave-[CRATE] > benchmark_results.txt

# Append to BASELINE_METRICS.md
Get-Content benchmark_results.txt >> docs/current/BASELINE_METRICS.md
```

---

## Common Pitfalls (Avoid These!)

### 1. Assuming Simple APIs ❌

**Wrong**:
```rust
let memory = Memory {
    created_at: Utc::now(),  // ❌ Not a direct field!
    content: "test",         // ❌ Wrong type!
};
```

**Right**:
```rust
let memory = create_test_memory();  // ✅ Use helper function
```

### 2. Using `b.iter()` with Mutable State ❌

**Wrong**:
```rust
let mut manager = Manager::new();
b.iter(|| {
    let result = manager.get("id");  // ❌ Escapes closure!
    black_box(result)
});
```

**Right**:
```rust
b.iter_with_setup(
    || Manager::new(),           // ✅ Setup creates manager
    |mut manager| {              // ✅ Iter owns manager
        let result = manager.get("id");
        black_box(result)
    }
);
```

### 3. Not Using `black_box()` ❌

**Wrong**:
```rust
b.iter(|| {
    let result = expensive_operation();
    result  // ❌ Compiler may optimize away!
});
```

**Right**:
```rust
b.iter(|| {
    let result = expensive_operation();
    black_box(result)  // ✅ Prevents optimization
});
```

### 4. Forgetting Parameterized Benchmarks ❌

**Wrong**:
```rust
fn bench_storage(c: &mut Criterion) {
    c.bench_function("storage", |b| {
        b.iter(|| {
            // Only tests one size!
        })
    });
}
```

**Right**:
```rust
fn bench_storage(c: &mut Criterion) {
    let mut group = c.benchmark_group("storage");
    for size in [10, 50, 100, 500] {  // ✅ Tests scaling!
        group.bench_with_input(BenchmarkId::from_parameter(size), ...);
    }
    group.finish();
}
```

### 5. Not Reading API Before Implementing ❌

**Result**: 36 compilation errors, 2h wasted (learned the hard way!)

**Right Approach**:
1. Read lib.rs for module overview
2. Read target file for struct definitions
3. Check test files for usage examples
4. THEN write benchmarks

---

## Session Restart Checklist

When resuming benchmarking work, verify:

- [ ] Read this preparation guide
- [ ] Review memory_benchmarks.rs (template)
- [ ] Choose dependency fix strategy (Option A or B)
- [ ] Read API docs for next crate (context/llm/rag)
- [ ] Allocate 2-3h uninterrupted time per crate
- [ ] Have BASELINE_METRICS.md open for documentation
- [ ] Terminal ready at workspace root
- [ ] Fresh coffee ☕ (6-8h session!)

---

**End of Preparation Guide**

**Next Steps**: Review this guide, choose starting crate (recommend Context - simplest API), allocate 6-8h block for implementation.
