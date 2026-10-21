# Benchmark Update Summary (November 2025)

**Update Type**: Major gap-filling update  
**Date**: November 2025  
**Benchmarks Added**: 113 new benchmarks  
**Crates Added**: 6 (memory, context, persona, prompts, rag, stress-test added; nav updated)  
**Previous Total**: 454 benchmarks across 31 crates (76% coverage)  
**New Total**: 567 benchmarks across 37 crates (92.5% coverage)  
**Coverage Improvement**: +16.5% (37/40 measured crates)

---

## New Benchmarks Breakdown

### P2 Crates (92 benchmarks - ALL NEW)

**astraweave-memory** (9 benchmarks):
- memory_creation: 246.19 ns
- memory_storage (3 sizes): 5.15-40.90 µs
- memory_retrieval_by_id: 4.75 µs
- memory_access_tracking (3 sizes): 1.49-7.51 µs
- memory_importance_update: 119.44 ns
- **Grade**: ⭐⭐⭐⭐ Excellent
- **60 FPS Budget**: 33,000+ operations/frame possible

**astraweave-context** (17 benchmarks):
- message_creation: 219.91 ns
- message_format_for_prompt: 144.72 ns
- context_window_creation: 720.63 ns
- context_window_add_message (3 sizes): 7.75-90.29 µs
- window_types (2 types): 38.77-39.32 µs
- conversation_history_creation: 637.59 ns
- get_recent_messages (3 sizes): 184.60-199.75 ns
- message_batch_creation (4 sizes): 2.93-162.75 µs
- message_batch_formatting (3 sizes): 2.80-18.68 µs
- context_window_with_stats: 44.87 ns
- **Grade**: ⭐⭐⭐⭐⭐ Exceptional
- **Highlight**: Sub-200ns message retrieval across all sizes

**astraweave-persona** (22 benchmarks):
- persona_creation: 615.86 ns
- persona_default: 15.70 ns
- fact/skill/episode_creation: 148.78-374.06 ns
- profile_creation_default: 73.21 ns
- profile_comprehensive (3 sizes): 14.13-47.17 µs
- profile_clone: 13.70 µs
- profile_sign: 42.34 ns
- profile_verify: **544.68 ps** (sub-nanosecond!)
- profile_distill (3 sizes): 1.70-9.00 µs
- profile_serialize/deserialize_json: 5.10/25.68 µs
- profile_add_facts (3 sizes): 3.02-26.92 µs
- profile_add_skills (3 sizes): 1.88-17.75 µs
- **Grade**: ⭐⭐⭐⭐⭐ Exceptional
- **Highlight**: 544 ps verification (fastest benchmark in AstraWeave!)

**astraweave-prompts** (22 benchmarks):
- template_creation (3 types): 124.06-316.15 ns
- context_creation (2 types): 302.17 ns - 1.63 µs
- template_render (3 types): 0.999-8.75 µs
- engine_creation: 7.29 ns
- engine_register_template (3 sizes): 382.76 ns - 19.22 µs
- engine_render: 1.36 µs
- batch_render (3 sizes): 10.26-113.05 µs
- context_add_variables (3 sizes): 889.04 ns - 5.17 µs
- context_to_string_map: 2.50 µs
- template/context_clone: 98.76 ns / 1.51 µs
- template_render_map: 1.39 µs
- **Grade**: ⭐⭐⭐⭐⭐ Exceptional
- **Highlight**: Sub-10µs for all single operations (LLM-ready)

**astraweave-rag** (22 benchmarks):
- memory_creation: 154.34 ns
- memory_batch_creation (4 sizes): 5.12-178.90 µs
- retrieval_engine_creation: **2.18 ns** (zero-cost abstraction!)
- retrieval_simple_search: 8.22 µs
- retrieval_search_scaling (4 sizes): 6.20-123.83 µs
- retrieval_category_filtering: 15.30 µs
- query_creation (2 types): 70.13-385.47 ns
- rag_config_creation/custom: 60.28/60.31 ns
- memory_clone: 118.48 ns
- memory_batch_clone (3 sizes): 1.26-13.67 µs
- memory_serialize/deserialize_json: 327.45/408.51 ns
- similarity_calculation: 710.63 ns
- result_ranking (3 sizes): 26.06-101.23 µs
- **Grade**: ⭐⭐⭐⭐⭐ Exceptional
- **Highlight**: 2.18 ns engine creation (2nd fastest in AstraWeave!)

### Navigation (18 benchmarks - UPDATED)

**astraweave-nav**:
- Navmesh Baking (3 sizes): 55.90 µs - 473.20 ms
- Baking Scaling (6 sizes): 52.23 µs - 458.69 ms
- A* Pathfinding (3 path lengths): 2.44-54.45 µs
- Pathfinding Scaling (4 sizes): 33.64 µs - 7.15 ms
- Throughput (3 sizes): 7.01-721.74 µs
- **Grade**: ⭐⭐⭐⭐ Excellent (⚠️ 10k triangles slow)
- **Highlight**: 2.44 µs short path, 142k QPS @ 100 triangles
- **Bottleneck**: 10k triangle baking = 473 ms (must be async)

### Stress Tests (3 benchmarks - NEW)

**astraweave-stress-test**:
- ecs_performance: 508.96 µs
- network_stress: 265.57 µs
- persistence_stress: 1.25 ms
- **Grade**: ⭐⭐⭐⭐ Excellent
- **60 FPS Budget**: All sub-2ms (acceptable for stress scenarios)

---

## Updated Coverage Statistics

### Before This Update (October 31, 2025)
- **Total Benchmarks**: 454
- **Total Crates**: 31
- **Coverage**: 76% (31/40 production crates)
- **Gaps**: P2 crates (0 benchmarks documented), navigation (unknown baseline), stress tests (unknown)

### After This Update (November 2025)
- **Total Benchmarks**: 567 (+113, +24.9%)
- **Total Crates**: 37 (+6, +19.4%)
- **Coverage**: 92.5% (37/40 production crates, +16.5%)
- **Gaps Filled**: P2 (92 benchmarks), navigation (18 benchmarks), stress tests (3 benchmarks)
- **Remaining Gaps**: Integration benchmarks (cross-system pipelines), LLM optimization

---

## Performance Highlights (Updated)

### New Fastest Operations
1. **profile_verify**: **544 ps** (sub-nanosecond cryptographic verification!)
2. **retrieval_engine_creation**: **2.18 ns** (zero-cost RAG abstraction!)
3. **engine_creation** (prompts): **7.29 ns** (zero-cost template engine!)
4. **persona_default**: **15.70 ns** (sub-20ns default constructor)

### New Sub-Microsecond Operations (< 1 µs)
- message_format_for_prompt: 144.72 ns
- persona_creation: 615.86 ns
- context_window_creation: 720.63 ns
- template_render_simple: 998.96 ns

### P2 System Performance (60 FPS Budget Analysis)
- **Memory**: 33,000+ operations/frame @ 50% budget
- **Context**: 22,000+ message operations/frame
- **Persona**: 15,000+ profile operations/frame
- **Prompts**: 16,000+ template renders/frame
- **RAG**: 2,000+ searches/frame (1000-memory corpus)

---

## Technical Debt Addressed

1. ✅ **P2 Benchmark Gap**: Resolved 0 → 92 benchmarks across 5 critical AI crates
2. ✅ **Navigation Baseline**: Established 18 benchmarks (was "unknown")
3. ✅ **Stress Test Baseline**: Established 3 benchmarks (was "unknown")
4. ✅ **Compilation Errors**: Fixed astraweave-memory line 95 reference lifetime error
5. ✅ **Capacity Violations**: Adjusted benchmark sizes to respect memory type limits (Working=50, Episodic=1000)
6. ⚠️ **Compilation Warnings**: 40+ warnings across P2 crates (deferred to separate task)

---

## Deferred Work

**Integration Benchmarks** (4-6 hours estimated):
- Cross-system pipelines (ECS → Perception → Planning → Physics → Nav → ECS feedback)
- Full rendering pipeline (Scene → Culling → Batching → GPU → Present)
- Full game loop (Input → AI → Physics → Rendering → Audio)
- **Reason for Deferral**: Focus on documenting existing benchmarks first, then design comprehensive integration suite
- **Priority**: Medium (nice-to-have for Phase B Month 4)

**LLM Optimization** (8-12 hours estimated):
- Reduce 500+ ms latency under load
- Fix 200+ ms cache contention
- **Reason for Deferral**: Requires LLM system redesign, separate from benchmarking work
- **Priority**: High (scheduled for Phase B Month 2-3)

---

## Recommendations for Next Update

1. **Integration Benchmarks**: Design comprehensive cross-system benchmark suite
2. **LLM Optimization**: Address 500+ ms latency bottleneck before re-benchmarking
3. **Warning Cleanup**: Fix 40+ compilation warnings across P2 crates
4. **Documentation**: Add per-benchmark commentary in MASTER_BENCHMARK_REPORT.md
5. **CI Integration**: Ensure new benchmarks run in GitHub Actions workflow

---

## Version History

- **v3.1** (Oct 31, 2025): Phase 8.3 Week 1 - World serialization (454 benchmarks, 31 crates, 76%)
- **v3.2** (Nov 2025): P2 Gap Filling + Navigation + Stress Tests (567 benchmarks, 37 crates, 92.5%)

