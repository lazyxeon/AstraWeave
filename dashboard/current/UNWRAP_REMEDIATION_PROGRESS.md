# AstraWeave Unwrap Remediation Progress

**Version**: 1.0.0  
**Date**: January 2025  
**Status**: 🚧 In Progress  
**Total `.unwrap()` calls**: 5,121

---

## Executive Summary

This document tracks the remediation of `.unwrap()` calls across the AstraWeave codebase. While `unwrap()` is convenient during development, production code should use proper error handling with `Result` and `Option` types to prevent runtime panics.

### Goals

1. **P0 Production Code**: Zero panicking `.unwrap()` in core runtime crates
2. **P1 Infrastructure**: Replace with `expect()` or proper error handling
3. **Test Code**: Acceptable in tests, but prefer `?` operator where possible

---

## Current State

### Top 30 Files by `.unwrap()` Count

| File | Count | Priority | Category |
|------|-------|----------|----------|
| astraweave-render/tests/coverage_booster_render.rs | 165 | Test | Test code - acceptable |
| tools/astraweave-assets/tests/lib_api_tests.rs | 104 | Test | Test code - acceptable |
| astraweave-audio/src/engine.rs | 85 | **P0** | Production - needs remediation |
| astraweave-memory/tests/storage_tests.rs | 85 | Test | Test code - acceptable |
| astraweave-embeddings/src/store.rs | 84 | **P0** | Production - needs remediation |
| tools/aw_editor/src/command.rs | 81 | **P1** | Editor - recoverable errors |
| astraweave-ecs/tests/concurrency_tests.rs | 77 | Test | Test code - acceptable |
| astraweave-llm/src/ab_testing.rs | 67 | **P1** | LLM - graceful degradation needed |
| astraweave-memory/src/memory_manager.rs | 61 | **P0** | Production - needs remediation |
| tools/astraweave-assets/tests/polyhaven_api_tests.rs | 60 | Test | Test code - acceptable |
| astraweave-llm/src/production_hardening.rs | 58 | **P0** | Production - ironic name! |
| astraweave-prompts/tests/loader_library_tests.rs | 52 | Test | Test code - acceptable |
| astraweave-context/src/token_counter.rs | 51 | **P0** | Production - needs remediation |
| astraweave-persona/tests/serialization.rs | 50 | Test | Test code - acceptable |
| astraweave-ai/src/goap/plan_visualizer.rs | 47 | **P1** | Visualization - non-critical |
| astraweave-memory/tests/adaptive_behavior_tests.rs | 46 | Test | Test code - acceptable |
| astraweave-persona/tests/sprint3_persona_tests.rs | 46 | Test | Test code - acceptable |
| astraweave-rag/tests/rag_pipeline_tests.rs | 45 | Test | Test code - acceptable |
| astraweave-memory/tests/pattern_tests.rs | 44 | Test | Test code - acceptable |
| tools/astraweave-assets/tests/lib_download_integration_tests.rs | 44 | Test | Test code - acceptable |
| persistence/aw-save/benches/save_benchmarks.rs | 44 | Bench | Benchmark - acceptable |
| astraweave-prompts/tests/advanced_prompts_test.rs | 41 | Test | Test code - acceptable |
| astraweave-prompts/tests/helper_function_tests.rs | 41 | Test | Test code - acceptable |
| astraweave-audio/tests/audio_engine_tests.rs | 40 | Test | Test code - acceptable |
| astraweave-context/src/window.rs | 39 | **P0** | Production - needs remediation |
| astraweave-net/tests/integration/packet_loss_tests.rs | 38 | Test | Test code - acceptable |
| tools/aw_editor/tests/integration_tests.rs | 37 | Test | Test code - acceptable |
| tools/aw_editor/tests/comprehensive_smoke_tests.rs | 35 | Test | Test code - acceptable |
| astraweave-context/src/history.rs | 34 | **P0** | Production - needs remediation |
| astraweave-prompts/src/sanitize.rs | 34 | **P0** | Production - needs remediation |

---

## Priority Matrix

### P0: Core Runtime Crates (Must Fix)

These crates are on the critical path during game execution. Panics here cause game crashes.

| Crate | File | Count | Status | Assigned |
|-------|------|-------|--------|----------|
| astraweave-audio | src/engine.rs | 85 | ⏳ Pending | - |
| astraweave-embeddings | src/store.rs | 84 | ⏳ Pending | - |
| astraweave-memory | src/memory_manager.rs | 61 | ⏳ Pending | - |
| astraweave-llm | src/production_hardening.rs | 58 | ⏳ Pending | - |
| astraweave-context | src/token_counter.rs | 51 | ⏳ Pending | - |
| astraweave-context | src/window.rs | 39 | ⏳ Pending | - |
| astraweave-context | src/history.rs | 34 | ⏳ Pending | - |
| astraweave-prompts | src/sanitize.rs | 34 | ⏳ Pending | - |

**Total P0**: ~446 `.unwrap()` calls to remediate

### P1: Infrastructure Crates (Should Fix)

These support critical systems but failures are recoverable.

| Crate | File | Count | Status | Assigned |
|-------|------|-------|--------|----------|
| tools/aw_editor | src/command.rs | 81 | ⏳ Pending | - |
| astraweave-llm | src/ab_testing.rs | 67 | ⏳ Pending | - |
| astraweave-ai | src/goap/plan_visualizer.rs | 47 | ⏳ Pending | - |

**Total P1**: ~195 `.unwrap()` calls to remediate

### P2: Test/Benchmark Code (Acceptable)

`.unwrap()` is generally acceptable in test code where failures should cause test failures.

**Total P2**: ~4,480 `.unwrap()` calls (acceptable, no remediation needed)

---

## Remediation Patterns

### Pattern 1: Replace with `?` Operator (Preferred)

```rust
// Before
let file = File::open("config.toml").unwrap();

// After
let file = File::open("config.toml")?;
```

### Pattern 2: Replace with `expect()` for Invariants

```rust
// Before
let value = map.get(&key).unwrap();

// After
let value = map.get(&key).expect("key should exist after initialization");
```

### Pattern 3: Replace with `ok_or()` / `ok_or_else()`

```rust
// Before
let first = items.first().unwrap();

// After
let first = items.first().ok_or_else(|| anyhow!("items list is empty"))?;
```

### Pattern 4: Replace with `if let` / `match`

```rust
// Before
let value = opt.unwrap();
do_something(value);

// After
if let Some(value) = opt {
    do_something(value);
} else {
    warn!("Optional value was None, skipping");
}
```

### Pattern 5: Replace with `unwrap_or_default()` for Safe Defaults

```rust
// Before
let name = config.name.unwrap();

// After
let name = config.name.unwrap_or_default();
```

---

## Progress Tracking

### Sprint 1: P0 Audio & Embeddings (Week 1)

| Task | File | Before | After | Status |
|------|------|--------|-------|--------|
| Audio engine | astraweave-audio/src/engine.rs | 85 | - | ⏳ |
| Embeddings store | astraweave-embeddings/src/store.rs | 84 | - | ⏳ |

### Sprint 2: P0 Memory & LLM (Week 2)

| Task | File | Before | After | Status |
|------|------|--------|-------|--------|
| Memory manager | astraweave-memory/src/memory_manager.rs | 61 | - | ⏳ |
| Production hardening | astraweave-llm/src/production_hardening.rs | 58 | - | ⏳ |

### Sprint 3: P0 Context (Week 3)

| Task | File | Before | After | Status |
|------|------|--------|-------|--------|
| Token counter | astraweave-context/src/token_counter.rs | 51 | - | ⏳ |
| Window | astraweave-context/src/window.rs | 39 | - | ⏳ |
| History | astraweave-context/src/history.rs | 34 | - | ⏳ |

### Sprint 4: P0 Prompts & P1 (Week 4)

| Task | File | Before | After | Status |
|------|------|--------|-------|--------|
| Sanitize | astraweave-prompts/src/sanitize.rs | 34 | - | ⏳ |
| Editor command | tools/aw_editor/src/command.rs | 81 | - | ⏳ |
| A/B testing | astraweave-llm/src/ab_testing.rs | 67 | - | ⏳ |

---

## Automation

### Clippy Enforcement

Add to CI:
```yaml
- name: Check for unwrap in production code
  run: |
    cargo clippy -- -D clippy::unwrap_used -D clippy::expect_used \
      --allow clippy::unwrap_in_result
```

### Pre-commit Hook

```bash
#!/bin/bash
# .git/hooks/pre-commit

# Check for new unwrap() in non-test code
if git diff --cached --name-only | grep -E '\.rs$' | \
   xargs grep -l '\.unwrap()' | \
   grep -v 'tests/' | \
   grep -v '/benches/'; then
    echo "WARNING: New .unwrap() detected in production code"
    echo "Consider using ? operator or proper error handling"
fi
```

### Count Script

```powershell
# scripts/count_unwraps.ps1
param([string]$Path = ".")

Get-ChildItem -Path $Path -Recurse -Include *.rs -Exclude *target*, *fuzz* |
    Select-String -Pattern '\.unwrap\(\)' |
    Group-Object -Property Path |
    Sort-Object Count -Descending |
    Select-Object Count, @{N='File';E={$_.Name -replace [regex]::Escape($Path), ''}} |
    Format-Table -AutoSize
```

---

## Metrics

### Baseline (January 2025)

- **Total**: 5,121 `.unwrap()` calls
- **P0 (Production)**: ~446 calls
- **P1 (Infrastructure)**: ~195 calls
- **P2 (Tests/Benches)**: ~4,480 calls

### Target (End of Remediation)

- **P0**: 0 (all replaced with proper error handling)
- **P1**: <50 (only `expect()` with clear messages)
- **P2**: No target (tests acceptable)

---

## Success Criteria

1. ✅ All P0 production files have zero `.unwrap()` 
2. ✅ All P1 infrastructure files use `expect()` with descriptive messages
3. ✅ CI enforces `clippy::unwrap_used` on production code
4. ✅ New PRs with `.unwrap()` in production code trigger review warning

---

## Appendix: Full File Inventory

<details>
<summary>Click to expand full file list</summary>

Run this command to generate current inventory:
```powershell
Get-ChildItem -Path . -Recurse -Include *.rs -Exclude *target*, *fuzz* |
    Select-String -Pattern '\.unwrap\(\)' |
    Group-Object -Property Path |
    Sort-Object Count -Descending |
    ForEach-Object { "$($_.Count)`t$($_.Name)" }
```

</details>

---

## Revision History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0.0 | 2025-01 | Copilot | Initial creation |

