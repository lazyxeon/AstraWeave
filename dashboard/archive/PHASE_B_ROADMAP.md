# Phase B Roadmap: Performance & Scalability

**Timeline**: Months 4-6 (October 13 - December 31, 2025)  
**Status**: Planning  
**Prerequisites**: ✅ Week 8 Complete (2.70 ms, 370 FPS, 84% headroom)

---

## Executive Summary

Phase B focuses on **deep performance optimization and scalability** to push AstraWeave from 1,000 entities @ 370 FPS to **5,000+ entities @ 60 FPS**. Building on Week 8's lessons (Amdahl's Law, batching > scattering, overhead thresholds), we target **-30-41% additional frame time reduction** through architectural improvements rather than micro-optimizations.

**Target**: 2.70 ms → 1.6-1.9 ms (-30-41% total)  
**Stretch Goal**: < 1.5 ms (666+ FPS @ 1,000 entities)

---

## Week 8 Lessons Applied

### Critical Insights (Do NOT Violate)

1. **Amdahl's Law is Ruthless**
   - Current bottleneck: 59% sequential ECS overhead (collection + writeback)
   - Max parallel speedup: 1.24× without architectural changes
   - **Solution**: Redesign ECS for parallel-first architecture

2. **Batching > Scattering (3-5× faster)**
   - `collect() → process → writeback` beats scattered `get_mut()` by 3-5×
   - Archetype lookup is O(log n) per entity
   - **Solution**: Maintain batching patterns in all optimizations

3. **Overhead Threshold: >5 ms**
   - Rayon overhead: 50-100 µs (Week 8 tested)
   - SIMD work: 1 µs (too small to parallelize)
   - **Solution**: Only parallelize ECS queries, rendering, physics—not individual systems

4. **SIMD Auto-Vectorization is 80-85% Optimal**
   - glam achieves 2.08× speedup with zero manual SIMD
   - Hand-written AVX2 might get 2.5× (only 20% more)
   - **Solution**: Trust glam, focus on algorithms not instructions

5. **Cache Locality Cascades**
   - Spatial hash improved ALL systems 9-17%, not just collision
   - Memory layout changes propagate globally
   - **Solution**: Prioritize data-oriented design (DoD) transformations

---

## Phase B Goals

### Performance Targets

| Metric | Week 8 Baseline | Phase B Target | Stretch Goal |
|--------|----------------|----------------|--------------|
| **Frame Time** | 2.70 ms | 1.6-1.9 ms | < 1.5 ms |
| **FPS (1k entities)** | 370 | 500-625 | 666+ |
| **Entity Capacity** | ~200 @ 60 FPS | 500 @ 60 FPS | 1,000 @ 60 FPS |
| **Headroom** | 84% | 90%+ | 91%+ |
| **p95 Latency** | 3.2 ms | < 2.5 ms | < 2.0 ms |

### Technical Objectives

1. **Parallel ECS Architecture** (7-15% reduction)
   - Chunked parallel iteration for queries
   - Per-thread archetype access (unsafe pointers)
   - Lock-free component insertion/removal

2. **Collision Flat Grid** (15-22% reduction)
   - Replace HashMap with Vec2D for O(1) cell lookup
   - Eliminate 400-600 µs HashMap overhead
   - Maintain O(n log n) complexity

3. **Rendering Instancing** (3-7% reduction)
   - Batch draw calls by material
   - GPU instancing for repeated meshes
   - Reduce draw call overhead 10-100×

4. **RAG Foundation** (Infrastructure)
   - Vector database integration (Qdrant/Milvus)
   - Semantic search for AI context
   - Embedding generation pipeline

5. **Stress Testing Framework** (Quality)
   - 500/1000/2000/5000 entity scenarios
   - CI benchmarks with regression detection
   - Memory profiling (allocation patterns)

---

## Three-Month Breakdown

### Month 4 (October 13 - November 12, 2025)

**Theme**: Foundation & Hotspot Elimination

**Week 9 (Oct 13-19): Collision Flat Grid + Tracy Deep Dive**
- **Action 27**: Collision Flat Grid Optimization (2-3 days)
  - Replace `HashMap<(i32,i32), Vec<Entity>>` with `Vec2D<Vec<Entity>>`
  - Implement O(1) cell lookup: `grid[y * width + x]`
  - Benchmark: Target 1,100 µs → 500-700 µs (-400-600 µs)
  - **Expected Impact**: -15-22% frame time
  
- **Action 28**: Tracy Hotspot Analysis (1-2 days)
  - Capture 10,000-frame profiles @ 1k/2k/5k entities
  - Identify new bottlenecks post-flat-grid
  - Document allocation hotspots (malloc/free flamegraphs)
  - **Deliverable**: Top 10 hotspots ranked by time

- **Action 29**: Stress Test Framework Phase 1 (2-3 days)
  - 500/1000/2000/5000 entity scenarios
  - Automated CI benchmarks (criterion integration)
  - Memory profiling (heaptrack/valgrind integration)
  - **Deliverable**: Baseline stress test suite

**Week 10 (Oct 20-26): Rendering Instancing**
- **Action 30**: Material Batching (2-3 days)
  - Group draw calls by material/shader
  - Implement draw call deduplication
  - Benchmark: Target 380 µs → 200-300 µs (-80-180 µs)
  - **Expected Impact**: -3-7% frame time

- **Action 31**: GPU Instancing (2-3 days)
  - Instance buffer implementation (wgpu)
  - Per-instance data (transform, material ID)
  - Benchmark: 10-100× draw call reduction
  - **Deliverable**: Instanced rendering demo

**Week 11 (Oct 27 - Nov 2): Parallel ECS Design**
- **Action 32**: Parallel Query API Design (2-3 days)
  - `Query2Mut<A, B>` with `par_iter_mut()`
  - Chunked iteration (256 entities/chunk)
  - Unsafe per-thread archetype access
  - **Deliverable**: API design doc + prototype

- **Action 33**: Lock-Free Component Insertion (2-3 days)
  - Atomic operations for entity creation
  - Deferred archetype moves (staging buffer)
  - Benchmark: Insertion throughput
  - **Deliverable**: Lock-free ECS prototype

**Week 12 (Nov 3-9): Integration & Validation**
- **Action 34**: Phase B Month 1 Integration (3-4 days)
  - Merge flat grid, instancing, parallel query prototype
  - Regression testing (all 34+ benchmarks)
  - Tracy validation (frame time, allocations)
  - **Deliverable**: Month 1 completion report

**Month 1 Target**: 2.70 ms → 2.0-2.2 ms (-18-26%)

---

### Month 5 (November 10 - December 10, 2025)

**Theme**: Parallel ECS Implementation & Scalability

**Week 13 (Nov 10-16): Parallel ECS Core**
- **Action 35**: Parallel Query Implementation (3-4 days)
  - Rayon integration for chunked iteration
  - Per-thread archetype pointers (unsafe)
  - Benchmark: Collection 400 µs → 100-200 µs
  - **Expected Impact**: -7-15% frame time

- **Action 36**: Parallel System Execution (2-3 days)
  - System-level parallelism (independent systems)
  - Dependency graph analysis
  - Benchmark: System tick time
  - **Deliverable**: Parallel execution scheduler

**Week 14 (Nov 17-23): Memory Optimization**
- **Action 37**: Archetype Memory Pooling (2-3 days)
  - Slab allocator for archetypes
  - Minimize heap churn (tracy malloc analysis)
  - Benchmark: Allocation count reduction
  - **Deliverable**: Memory pool implementation

- **Action 38**: Component SOA Transformation (3-4 days)
  - Array-of-Structs → Struct-of-Arrays (hot components)
  - Cache-friendly layout (64-byte alignment)
  - Benchmark: Cache miss reduction
  - **Expected Impact**: 5-10% performance gain

**Week 15 (Nov 24-30): RAG Foundation**
- **Action 39**: Vector Database Integration (2-3 days)
  - Qdrant/Milvus integration
  - Embedding generation (sentence-transformers)
  - Benchmark: Query latency (<10 ms)
  - **Deliverable**: RAG prototype

- **Action 40**: Semantic Search for AI (2-3 days)
  - Context retrieval for LLM prompts
  - Dynamic few-shot example selection
  - Benchmark: Relevance metrics
  - **Deliverable**: AI context enhancement

**Week 16 (Dec 1-7): Stress Testing & Validation**
- **Action 41**: 5,000 Entity Stress Test (2-3 days)
  - Profile 5k entity scenarios
  - Identify scalability bottlenecks
  - Benchmark: Frame time @ 5k entities
  - **Deliverable**: Scalability report

- **Action 42**: Phase B Month 2 Integration (3-4 days)
  - Merge parallel ECS, memory optimization, RAG
  - Regression testing
  - **Deliverable**: Month 2 completion report

**Month 2 Target**: 2.0-2.2 ms → 1.6-1.9 ms (-18-26% additional)

---

### Month 6 (December 8-31, 2025)

**Theme**: Polish, Documentation & Production Readiness

**Week 17 (Dec 8-14): GPU Compute Foundation**
- **Action 43**: Compute Shader Integration (2-3 days)
  - wgpu compute pipeline setup
  - GPU particle system (proof of concept)
  - Benchmark: CPU vs GPU performance
  - **Deliverable**: GPU compute demo

- **Action 44**: SIMD Particle Updates (2-3 days)
  - Batch particle position/velocity updates
  - GPU-side collision detection
  - Benchmark: 10k+ particles @ 60 FPS
  - **Deliverable**: GPU particle system

**Week 18 (Dec 15-21): Documentation & Knowledge Transfer**
- **Action 45**: Phase B Architecture Guide (3-4 days)
  - Parallel ECS patterns
  - Memory optimization techniques
  - RAG integration guide
  - **Deliverable**: 20,000+ word architecture doc

- **Action 46**: Performance Tuning Guide (2-3 days)
  - Tracy profiling workflow
  - Optimization checklist
  - Common pitfalls (based on Week 8 lessons)
  - **Deliverable**: Performance engineering handbook

**Week 19 (Dec 22-28): Final Integration**
- **Action 47**: Phase B Full Integration (3-4 days)
  - Merge all Month 1-3 work
  - Full regression suite (34+ benchmarks)
  - Tracy comprehensive profiling
  - **Deliverable**: Phase B completion report

- **Action 48**: Production Readiness Audit (2-3 days)
  - Security review (unwrap audit update)
  - CI/CD pipeline validation
  - Deployment checklist
  - **Deliverable**: Production readiness report

**Week 20 (Dec 29-31): Phase C Planning**
- **Action 49**: Phase C Roadmap (1-2 days)
  - Networking & multiplayer
  - Advanced rendering (Nanite, DDGI)
  - AI/ML integration (transformers, RL)
  - **Deliverable**: Phase C kickoff plan

**Month 3 Target**: Maintain 1.6-1.9 ms, achieve stretch goal < 1.5 ms

---

## Success Metrics

### Primary KPIs (Must Achieve)

| Metric | Baseline (Week 8) | Target | Status |
|--------|-------------------|--------|--------|
| Frame Time @ 1k | 2.70 ms | 1.6-1.9 ms | ⏳ Pending |
| FPS @ 1k | 370 | 500-625 | ⏳ Pending |
| Entity Capacity | ~200 @ 60 FPS | 500 @ 60 FPS | ⏳ Pending |
| p95 Latency | 3.2 ms | < 2.5 ms | ⏳ Pending |

### Secondary KPIs (Nice to Have)

| Metric | Baseline | Target | Status |
|--------|----------|--------|--------|
| Draw Calls | ~1,000 | < 100 | ⏳ Pending |
| Allocation Count | ~50k/frame | < 10k/frame | ⏳ Pending |
| Cache Miss Rate | Unknown | < 5% | ⏳ Pending |
| Parallel Speedup | 1.0× | 2-4× | ⏳ Pending |

### Quality Metrics

- ✅ **Compilation**: Zero errors (maintained from Phase A)
- ✅ **Tests**: 100% passing (34+ benchmarks, all unit tests)
- ✅ **Documentation**: 50,000+ words (Phase B adds 30,000+)
- ✅ **Production Safety**: 0 unwraps in hot paths (ongoing audit)

---

## Risk Assessment & Mitigation

### High-Risk Items

1. **Parallel ECS Complexity** (Likelihood: High, Impact: High)
   - **Risk**: Unsafe code introduces data races, UB
   - **Mitigation**: 
     - Miri validation for unsafe code
     - ThreadSanitizer testing
     - Extensive unit tests (race conditions)
     - Fallback: Keep sequential ECS as default, parallel as opt-in

2. **Performance Regression** (Likelihood: Medium, Impact: High)
   - **Risk**: New code slows down existing systems
   - **Mitigation**:
     - Continuous benchmarking (CI enforcement)
     - Tracy profiling at each milestone
     - Rollback plan (git bisect for regressions)

3. **GPU Compute Portability** (Likelihood: Medium, Impact: Medium)
   - **Risk**: Compute shaders fail on some platforms (Metal, DX12)
   - **Mitigation**:
     - CPU fallback for all GPU features
     - Cross-platform CI testing
     - Feature flags for GPU compute

### Medium-Risk Items

4. **RAG Integration Complexity** (Likelihood: Medium, Impact: Low)
   - **Risk**: Vector DB adds latency, memory overhead
   - **Mitigation**:
     - Async loading (don't block game loop)
     - LRU cache for embeddings
     - Benchmark latency targets (<10 ms)

5. **Scope Creep** (Likelihood: High, Impact: Medium)
   - **Risk**: Phase B expands beyond 3 months
   - **Mitigation**:
     - Strict action limit (27-49, max 23 actions)
     - Weekly progress reviews
     - Defer non-critical work to Phase C

---

## Dependencies & Prerequisites

### External Dependencies

- **Tracy**: 0.11.1+ (already integrated)
- **Rayon**: 1.10+ (tested in Week 8, reverted)
- **wgpu**: 25.0.2 (already integrated)
- **Qdrant/Milvus**: Latest stable (for RAG)

### Internal Prerequisites

- ✅ **Week 8 Complete**: 2.70 ms baseline validated
- ✅ **Benchmark Suite**: 34+ benchmarks with CI enforcement
- ✅ **Tracy Integration**: Zero-overhead profiling ready
- ✅ **Spatial Hash**: Cache locality foundation established

---

## Phase B vs Week 8 Comparison

| Optimization | Week 8 Approach | Phase B Approach | Lesson Applied |
|--------------|-----------------|------------------|----------------|
| **Collision** | Spatial hash (O(n log n)) | Flat grid (O(1) lookup) | Eliminate HashMap overhead |
| **Movement** | SIMD batching | Same (trust glam) | SIMD auto-vec is 80-85% optimal |
| **ECS** | Sequential collect/writeback | Parallel chunked iteration | Amdahl's Law requires architecture change |
| **Rendering** | Per-entity draw calls | Batched + instanced | Batching > scattering (3-5×) |
| **Parallelization** | Rayon on 1 µs work (failed) | Rayon on 400 µs+ work | Overhead threshold: >5 ms |

---

## Communication & Reporting

### Weekly Standups (Virtual)
- **Monday**: Week planning, action prioritization
- **Wednesday**: Mid-week progress check, blockers
- **Friday**: Week completion, metrics review

### Monthly Reports
- **Month 1**: Flat grid, instancing, parallel ECS design
- **Month 2**: Parallel ECS impl, memory optimization, RAG
- **Month 3**: GPU compute, documentation, production readiness

### Milestones
- **Week 12**: Month 1 complete (2.0-2.2 ms target)
- **Week 16**: Month 2 complete (1.6-1.9 ms target)
- **Week 20**: Phase B complete (< 1.5 ms stretch goal)

---

## Phase C Preview (Months 7-12)

**Focus Areas**:
1. **Networking & Multiplayer**: Server-authoritative, intent replication
2. **Advanced Rendering**: Nanite virtualized geometry, DDGI/VXGI lighting
3. **AI/ML Integration**: Transformer models, reinforcement learning
4. **Production Polish**: Asset pipeline, editor tooling, deployment

**Target**: Veilweaver alpha release (playable demo, 10+ hours gameplay)

---

## Conclusion

Phase B transforms AstraWeave from a **proof-of-concept** (Week 8: 370 FPS @ 1k entities) to a **production-ready engine** (Phase B: 500+ FPS @ 1k, 60 FPS @ 5k). By applying Week 8's hard-won lessons—Amdahl's Law, batching patterns, overhead thresholds, SIMD trust, cache locality—we achieve **-30-41% frame time reduction** through **architectural redesign** rather than micro-optimizations.

**Next Step**: Week 9 kickoff (Action 27: Collision Flat Grid Optimization)

---

**Version**: 1.0  
**Status**: Planning  
**Owner**: AstraWeave Copilot (AI-generated)  
**Last Updated**: October 12, 2025
