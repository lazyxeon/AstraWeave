# Benchmarking Quick Start Card

**Session Date**: Ready for next session  
**Current State**: Task 7 - 50% Complete  
**Time Required**: 6-8h for full P2 benchmark suite  
**Status**: ✅ Infrastructure ready, 🔄 Implementation pending

---

## Immediate Resume Steps

```powershell
# 1. Navigate to workspace
cd C:\Users\pv2br\source\repos\AstraWeave-AI-Native-Gaming-Engine

# 2. Read preparation guide (5 min)
code docs\journey\daily\BENCHMARKING_PREPARATION_GUIDE.md

# 3. Choose dependency fix strategy
# OPTION A (quick): Add --allow-warnings flag
# OPTION B (thorough): Fix all warnings (2-3h)

# 4. Verify memory benchmarks compile
cargo check -p astraweave-memory --benches --allow-warnings

# 5. Start next crate (recommend: Context - simplest API)
New-Item astraweave-context\benches\context_benchmarks.rs
```

---

## Implementation Order (Recommended)

### 1️⃣ Context (1-2h) - **START HERE**
- Simplest API of remaining P2 crates
- 5 benchmarks: window creation, message append, pruning, token budget, attention
- Template: Copy structure from memory_benchmarks.rs

### 2️⃣ Persona (1-2h) - **NEXT**
- Simple profile management API
- 3 benchmarks: profile creation, trait consistency, updates
- Low complexity, good momentum builder

### 3️⃣ Prompts (1-2h) - **AFTER PERSONA**
- Template expansion and formatting
- 3 benchmarks: template expansion, formatting, validation
- Similar complexity to Persona

### 4️⃣ LLM (2-3h) - **MOST COMPLEX**
- Requires tool registry setup
- 5 benchmarks: parsing, validation, fallback, registry, prompts
- Use MockLlmOrch from tests/ directory

### 5️⃣ RAG (2-3h) - **FINAL P2 CRATE**
- Requires embedding mocking
- 3 benchmarks: retrieval, ranking, consolidation
- Save for when experienced with benchmark patterns

---

## Files to Reference

**Template** (copy this structure):
```
astraweave-memory\benches\memory_benchmarks.rs
```

**API Documentation** (read before implementing):
```
astraweave-context\src\lib.rs           # Module overview
astraweave-context\src\sliding_window.rs # Pruning API
astraweave-context\src\token_budget.rs   # Budget API
astraweave-context\src\attention.rs      # Scoring API
```

**Test Examples** (for API usage patterns):
```
astraweave-context\src\sliding_window.rs  # See tests at bottom
astraweave-memory\src\memory_manager.rs   # See tests at bottom
```

---

## Benchmark Template (Copy-Paste Ready)

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use astraweave_context::{ContextWindow, SlidingWindow, Message};

// Helper function
fn create_test_message() -> Message {
    Message {
        role: "user".to_string(),
        content: "test".to_string(),
        timestamp: chrono::Utc::now(),
    }
}

// Basic benchmark
fn bench_context_window_creation(c: &mut Criterion) {
    c.bench_function("context_window_creation", |b| {
        b.iter(|| {
            let window = ContextWindow::new(100);
            black_box(window)
        })
    });
}

// Parameterized benchmark
fn bench_message_append(c: &mut Criterion) {
    let mut group = c.benchmark_group("message_append");
    
    for size in [10, 50, 100, 500] {
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &size,
            |b, &size| {
                b.iter_with_setup(
                    || ContextWindow::new(1000),
                    |mut window| {
                        for _ in 0..size {
                            window.add_message(create_test_message());
                        }
                        black_box(window)
                    }
                )
            },
        );
    }
    
    group.finish();
}

// Register benchmarks
criterion_group!(
    benches,
    bench_context_window_creation,
    bench_message_append,
);
criterion_main!(benches);
```

---

## Common Commands

**Check compilation**:
```powershell
cargo check -p astraweave-context --benches --allow-warnings
```

**Run benchmarks**:
```powershell
cargo bench -p astraweave-context --bench context_benchmarks
```

**Capture output for documentation**:
```powershell
cargo bench -p astraweave-context --bench context_benchmarks > context_results.txt
```

**Quick test (single iteration)**:
```powershell
cargo bench -p astraweave-context --bench context_benchmarks -- --quick
```

---

## Success Checklist (Per Crate)

- [ ] Created `benches/[CRATE]_benchmarks.rs`
- [ ] Added `[[bench]]` section to Cargo.toml
- [ ] Added `criterion` to dev-dependencies
- [ ] Created helper function(s) for test data
- [ ] Implemented 3-5 benchmarks covering core operations
- [ ] Used `iter_with_setup` for mutable state
- [ ] Used `black_box()` to prevent optimization
- [ ] Compilation succeeds (with or without --allow-warnings)
- [ ] Ran benchmarks and captured output
- [ ] Documented results in BASELINE_METRICS.md

---

## Key Lessons (From Memory Benchmarks)

✅ **DO**: Read actual API before writing code  
✅ **DO**: Use `iter_with_setup` for mutable state  
✅ **DO**: Use `black_box()` on all benchmark results  
✅ **DO**: Create helper functions for complex initialization  
✅ **DO**: Parameterize benchmarks (test 10/50/100/500 sizes)  

❌ **DON'T**: Assume simple API structure  
❌ **DON'T**: Use `b.iter()` with mutable state  
❌ **DON'T**: Forget to add Cargo.toml configuration  
❌ **DON'T**: Block on dependency warnings (use --allow-warnings)  
❌ **DON'T**: Try to benchmark complex APIs without mocking  

---

## Time Budget

**Per Crate Breakdown**:
- API analysis: 15-30 min
- Helper functions: 15-30 min
- Benchmark implementation: 30-60 min
- Cargo.toml setup: 5 min
- Testing & debugging: 15-30 min
- **Total per crate**: 1.5-3h

**Full Session**:
- Context: 1-2h
- Persona: 1-2h
- Prompts: 1-2h
- LLM: 2-3h
- RAG: 2-3h
- Documentation: 1h
- **Total**: 8-13h

**Recommended Split**:
- Session 1 (3h): Context + Persona
- Session 2 (3h): Prompts + half of LLM
- Session 3 (3h): Finish LLM + RAG + documentation

---

## Current Achievements

✅ **Tasks 1-6 COMPLETE** (4h vs 8-12h estimate = 67% time savings)  
✅ **25 test failures fixed** (99.7% success rate achieved)  
✅ **P2 coverage measured** (42.63% average, +12.35pp improvement)  
✅ **Critical bug discovered** (PascalCase tool validation - production-blocking)  
✅ **Documentation complete** (COVERAGE_AND_TESTING_SESSION_COMPLETE.md)  
✅ **Benchmarking prepared** (memory_benchmarks.rs, preparation guide, this card)  

🔄 **Task 7 IN PROGRESS** (50% complete - infrastructure ready)  
⏳ **Tasks 8-10 PENDING** (integration benchmarks, budget analysis, final reports)  

---

## Contact Points

**Preparation Guide**: `docs/journey/daily/BENCHMARKING_PREPARATION_GUIDE.md`  
**Session Summary**: `docs/journey/daily/COVERAGE_AND_TESTING_SESSION_COMPLETE.md`  
**Memory Template**: `astraweave-memory/benches/memory_benchmarks.rs`  
**Master Report**: `docs/current/MASTER_COVERAGE_REPORT.md`  

---

**Ready to Resume**: Read preparation guide → Choose starting crate → Allocate 2-3h → Start implementing!

**Estimated Completion**: 6-8h total for all P2 benchmarks
