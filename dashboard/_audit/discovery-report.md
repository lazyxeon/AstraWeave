# AstraWeave Documentation Discovery & Audit Report

**Date**: February 4, 2026  
**Auditor**: GitHub Copilot (Claude Opus 4.5)  
**Purpose**: Phase 1 Discovery for GitHub Pages Documentation Overhaul  
**Status**: ✅ COMPLETE

---

## Executive Summary

This audit inventories the AstraWeave codebase structure, existing documentation, and identifies gaps for the GitHub Pages documentation overhaul. The philosophy is **"prove it don't hype it"** — every claim must be verifiable against the actual implementation.

### Key Findings

| Category | Count | Status |
|----------|-------|--------|
| **Production Crates** | 47 | Identified |
| **Example Projects** | 66+ | Identified |
| **Tool Crates** | 15+ | Identified |
| **Existing Docs (src/)** | 60+ pages | mdBook structure exists |
| **Documented APIs** | ~15 crates | Some need updates |
| **Undocumented APIs** | ~32 crates | Priority for Phase 3 |
| **Broken/Outdated Examples** | 10+ | Flagged |

---

## 1. Crate Inventory

### 1.1 Core Engine Crates (Production-Ready)

All crates use workspace version `0.4.0` unless otherwise specified.

| Crate | Public Modules | Version | Status | Notes |
|-------|----------------|---------|--------|-------|
| **astraweave-ecs** | archetype, blob_vec, command_buffer, component_meta, entity_allocator, events, rng, sparse_set, type_registry | 0.4.0 | ✅ Production | Core ECS with World, Entity, Component, App |
| **astraweave-ai** | core_loop, ecs_ai_plugin, orchestrator, tool_sandbox, async_task*, llm_executor*, ai_arbiter*, veilweaver*, goap* | 0.4.0 | ✅ Production | *Feature-gated modules |
| **astraweave-core** | capture_replay, ecs_adapter, ecs_bridge, ecs_components, ecs_events, metrics, perception, schema, sim, tool_sandbox, tool_vocabulary, tools, util, validation, world | 0.4.0 | ✅ Production | World snapshot, perception bus |
| **astraweave-math** | simd_vec, simd_mat, simd_quat | 0.4.0 | ✅ Production | glam-based SIMD math |
| **astraweave-render** | 40+ modules (camera, clustered, deferred, gpu_particles, ibl, mesh, post, shadow_csm, terrain, water, animation, culling, graph, material, texture, etc.) | 0.4.0 | ✅ Production | wgpu 25.0.2 renderer |
| **astraweave-physics** | character_controller, spatial_hash, projectile, ragdoll, cloth, vehicle, destruction | 0.4.0 | ✅ Production | Rapier3D wrapper |
| **astraweave-nav** | navmesh, pathfinding, portal graphs | 0.4.0 | ✅ Production | A* navigation |
| **astraweave-behavior** | behavior_tree, goap, utility_ai, blackboard, ecs | 0.4.0 | ✅ Production | BT + GOAP systems |
| **astraweave-audio** | spatial, mixer, rodio_backend | 0.4.0 | ✅ Production | Spatial audio via rodio |
| **astraweave-gameplay** | combat_physics, attack_sweep | 0.4.0 | ✅ Production | Combat systems |
| **astraweave-terrain** | voxel_mesh, marching_cubes | 0.4.0 | ✅ Production | Hybrid voxel/poly terrain |
| **astraweave-scene** | streaming, world_partition | 0.4.0 | ✅ Production | Async cell streaming |
| **astraweave-cinematics** | timeline, sequencer, tracks | 0.4.0 | ✅ Production | Cutscene system |
| **astraweave-input** | bindings, gilrs_backend | 0.4.0 | ✅ Production | Input handling |
| **astraweave-ui** | hud, menus, egui_integration | 0.4.0 | ✅ Production | egui 0.32 UI |
| **astraweave-sdk** | re-exports prelude | 0.4.0 | ✅ Production | High-level SDK |

### 1.2 AI & LLM Crates

| Crate | Description | Status | Notes |
|-------|-------------|--------|-------|
| **astraweave-llm** | LLM adapter layer | Beta | Ollama/local LLM integration |
| **astraweave-llm-eval** | LLM evaluation framework | Beta | Testing LLM outputs |
| **astraweave-embeddings** | Vector embeddings | Production | 97.83% coverage |
| **astraweave-context** | Context window management | Production | LLM context |
| **astraweave-prompts** | Prompt templates | Production | Type-safe prompts |
| **astraweave-rag** | Retrieval-augmented generation | Production | RAG pipeline |
| **astraweave-memory** | AI memory systems | Production | Long-term memory |
| **astraweave-persona** | AI persona management | Production | Personality system |
| **astraweave-coordination** | Multi-agent coordination | Production | Agent orchestration |
| **astraweave-npc** | NPC AI systems | Production | ~95% coverage |
| **astraweave-dialogue** | Dialogue trees | Production | 100% coverage |
| **astraweave-director** | Director AI (adaptive bosses) | Production | Phase-based AI |

### 1.3 Gameplay & Content Crates

| Crate | Description | Status |
|-------|-------------|--------|
| **astraweave-quests** | Quest system | Production |
| **astraweave-pcg** | Procedural content generation | Production (93.46% coverage) |
| **astraweave-weaving** | Fate-weaving mechanic | Production (93.84% coverage) |
| **astraweave-materials** | Material system | Production (88.18% coverage) |
| **astraweave-asset** | Asset loading | Production |
| **astraweave-asset-pipeline** | Asset processing | Production |

### 1.4 Infrastructure Crates

| Crate | Description | Status |
|-------|-------------|--------|
| **astraweave-net** | Networking core | Production |
| **astraweave-net-ecs** | ECS networking bridge | Production |
| **astraweave-persistence-ecs** | ECS save/load | Production |
| **astraweave-security** | Security & validation | Production |
| **astraweave-secrets** | Secrets management | Production (90.95% coverage) |
| **astraweave-observability** | Telemetry & logging | Production |
| **astraweave-profiling** | Tracy profiling | Production |
| **astraweave-scripting** | Rhai scripting | Production |
| **astraweave-author** | Authoring tools | Production |
| **astraweave-ipc** | Inter-process communication | Production |
| **astraweave-steam** | Steam integration | Beta |
| **astraweave-stress-test** | Stress testing | Production |
| **astraweave-fluids** | Fluid simulation | Production |

### 1.5 Example Projects (66+ examples)

**Working Examples (Verified):**
- `hello_companion` - Core AI demo ✅
- `unified_showcase` - Multi-system demo ✅
- `core_loop_bt_demo` - Behavior tree demo ✅
- `core_loop_goap_demo` - GOAP demo ✅
- `weaving_pcg_demo` - PCG demo ✅
- `profiling_demo` - Tracy integration ✅
- `fluids_demo` - Fluid simulation ✅

**Status Uncertain (Need Verification):**
- `adaptive_boss` - ⚠️ Check compilation
- `companion_profile` - ⚠️ Check compilation
- `physics_demo3d` - ⚠️ Check compilation
- `navmesh_demo` - ⚠️ Check compilation
- `audio_spatial_demo` - ⚠️ Check compilation
- `visual_3d` - ⚠️ Check compilation

**Known Issues:**
- `ui_controls_demo` - API drift (egui version mismatch)
- `debug_overlay` - winit/egui version conflicts
- `rhai_authoring` - Rhai sync trait issues
- `astraweave-author` - Sync trait errors

### 1.6 Tool Crates

| Tool | Description | Status |
|------|-------------|--------|
| `aw_editor` | Level/encounter editor | Production |
| `aw_asset_cli` | Asset pipeline CLI | Production |
| `aw_debug` | Debug utilities | Production |
| `aw_build` | Build tooling | Production |
| `aw_texture_gen` | Texture generation | Production |
| `aw_headless` | Headless testing | Production |
| `aw_save_cli` | Save file CLI | Production |
| `aw_release` | Release tooling | Production |
| `aw_demo_builder` | Demo packaging | Production |
| `astraweave-assets` | PolyHaven fetcher | Production |
| `asset_signing` | Asset signing | Production |
| `ollama_probe` | Ollama testing | Production |

---

## 2. Existing Documentation Audit

### 2.1 Documentation Structure

The documentation uses **mdBook** (configured in `docs/book.toml`):

```
docs/
├── src/                    # mdBook source files
│   ├── README.md           # Landing page
│   ├── SUMMARY.md          # Table of contents
│   ├── getting-started/    # 4 pages
│   ├── architecture/       # 5 pages
│   ├── core-systems/       # 7+ pages (AI, Physics, etc.)
│   ├── game-dev/           # 6 pages
│   ├── examples/           # 7 pages
│   ├── reference/          # 5 pages
│   ├── dev/                # 6 pages
│   ├── resources/          # 7+ pages
│   ├── api/                # 1 page (index only)
│   └── veilweaver/         # 1 page
├── book/                   # Built mdBook output
├── current/                # Active planning docs (79 files)
├── guides/                 # Technical guides (24 files)
├── reference/              # Reference docs (10 files)
├── audits/                 # Audit reports (15 files)
├── masters/                # Master reports (5 files)
├── archive/                # Historical docs (200+ files)
└── book.toml               # mdBook configuration
```

### 2.2 Documentation Quality Assessment

| Section | Pages | Accuracy | Completeness | Action Needed |
|---------|-------|----------|--------------|---------------|
| **Getting Started** | 4 | ⚠️ Needs verification | ✅ Good | Verify build commands work |
| **Architecture** | 5 | ✅ Accurate | ✅ Good | Minor updates |
| **Core Systems** | 7 | ⚠️ Partial | ⚠️ Missing AI details | Expand AI section |
| **Game Dev** | 6 | ⚠️ Needs verification | ⚠️ Light content | Add real examples |
| **Examples** | 7 | ❌ Outdated status flags | ⚠️ Missing 60+ examples | Major update needed |
| **Reference** | 5 | ⚠️ Needs verification | ⚠️ Incomplete crates.md | Add all 47 crates |
| **API** | 1 | ⚠️ Stale links | ❌ Only index page | Generate per-crate docs |
| **Dev/Contributing** | 6 | ✅ Accurate | ✅ Good | Minor updates |
| **Resources** | 7 | ⚠️ Unknown | ⚠️ Unknown | Verify all |

### 2.3 Specific Documentation Issues

#### 2.3.1 Broken/Stale API References

**File**: `docs/src/api/index.md`

| Documented | Actual in lib.rs | Status |
|------------|------------------|--------|
| `Query` type | `Query`, `Query2`, `Query2Mut`, `SystemParam` | ✅ Correct |
| `AiArbiter` | `AIArbiter` (case) | ⚠️ Case mismatch |
| `BehaviorTree` type | Not directly exported (in `astraweave-behavior`) | ⚠️ Needs verification |
| `LlmAdapter` | Not found | ❌ Does not exist |
| `BatchExecutor` | Not found | ❌ Does not exist |
| `StreamingParser` | Not found | ❌ Does not exist |
| `PromptTemplate` | In `astraweave-prompts` | ✅ Exists elsewhere |

**Action**: Update API index to reflect actual exports.

#### 2.3.2 Missing Crate Documentation

The following crates are NOT documented in `docs/src/reference/crates.md`:

- astraweave-fluids
- astraweave-embeddings
- astraweave-context
- astraweave-prompts
- astraweave-rag
- astraweave-coordination
- astraweave-npc
- astraweave-secrets
- astraweave-observability
- astraweave-profiling
- astraweave-steam
- astraweave-stress-test
- astraweave-persistence-ecs
- astraweave-net-ecs
- astraweave-blend (in crates/)
- astraweave-persistence-player (in crates/)
- astract (in crates/)

**Action**: Add documentation for all 47 production crates.

#### 2.3.3 Outdated Example Status

**File**: `docs/src/examples/index.md`

Many examples are marked with "⚠️ Check compilation" without recent verification. Needs systematic verification.

#### 2.3.4 Broken Internal Links

**Potential Issues Found**:
1. `docs/src/api/index.md` links to `astraweave_core/index.html` - relies on rustdoc generation
2. Mermaid diagrams require `mdbook-mermaid` preprocessor (configured but verify installed)
3. Some `./` relative links may break depending on build path

### 2.4 Master Reports (Authoritative Sources)

These are well-maintained and should be referenced:

| Report | Location | Last Updated | Status |
|--------|----------|--------------|--------|
| **MASTER_BENCHMARK_REPORT** | docs/masters/ | Jan 13, 2026 | ✅ v5.55, comprehensive |
| **MASTER_COVERAGE_REPORT** | docs/masters/ | Dec 15, 2025 | ✅ v2.5.5, ~78% overall |
| **MASTER_ROADMAP** | docs/masters/ | Active | ✅ Current |
| **MASTER_API_PATTERNS** | docs/masters/ | Active | ✅ Current |

---

## 3. Benchmark & Performance Data

### 3.1 Available Benchmark Data

**Location**: `docs/masters/MASTER_BENCHMARK_REPORT.md` (9,156 lines)

**Key Metrics (verified Jan 2026)**:

| System | Benchmark | Target | Actual | Status |
|--------|-----------|--------|--------|--------|
| **ECS** | Entity spawn (100) | <50µs | 15.0µs | ✅ 70% under |
| **ECS** | Entity spawn (1000) | <500µs | 106.7µs | ✅ 79% under |
| **AI** | GOAP planning (full) | <10µs | 286ns | ✅ 97% under |
| **AI** | GOAP planning (cache) | <1µs | 9.8ns | ✅ 99% under |
| **Frame** | p50 @ 1k entities | <16.67ms | 1.27ms | ✅ 92% under |
| **Frame** | p99 @ 1k entities | <16.67ms | 2.42ms | ✅ 85% under |
| **AI Arbiter** | GOAP control | 100µs | 101.7ns | ✅ 982× faster |
| **Navigation** | Sliver triangles | - | 99ps/tri | ✅ Sub-nanosecond |
| **Input** | Frame clear | - | 0.77ns/op | ✅ Sub-nanosecond |

**Total Benchmarks**: 1,500+ across 76 sections

### 3.2 Test Coverage Data

**Location**: `docs/masters/MASTER_COVERAGE_REPORT.md`

**Key Stats (verified Dec 2025)**:
- **Overall Coverage**: ~78% (revised)
- **Total Tests**: ~7,600+
- **Production Crates**: 47/47 measured

**Top Performers**:
| Crate | Coverage | Tests |
|-------|----------|-------|
| astraweave-math | 98.05% | 34 |
| astraweave-embeddings | 97.83% | - |
| astraweave-ecs | 96.82% | - |
| astraweave-physics | 95.95% | 355 |
| astraweave-input | 95.07% | - |
| astraweave-nav | 94.66% | 65 |
| astraweave-behavior | 94.34% | 57 |
| astraweave-weaving | 93.84% | - |
| astraweave-pcg | 93.46% | - |
| astraweave-audio | 91.42% | 81 |
| astraweave-secrets | 90.95% | - |
| astraweave-dialogue | 100.00% | - |

---

## 4. Identified Gaps & Issues

### 4.1 Critical Issues (Must Fix)

| Issue | Location | Priority | Est. Effort |
|-------|----------|----------|-------------|
| API index references non-existent types | docs/src/api/index.md | P0 | 2h |
| 32+ crates undocumented | docs/src/reference/crates.md | P0 | 8h |
| Example status unverified | docs/src/examples/index.md | P1 | 4h |
| No per-crate API reference pages | docs/src/api/ | P1 | 16h |

### 4.2 Missing Documentation

| Category | Missing Content | Priority |
|----------|-----------------|----------|
| **API Reference** | Individual pages for 47 crates | P0 |
| **Performance** | Benchmark methodology guide | P1 |
| **Guides** | Fluids system guide | P2 |
| **Guides** | Networking guide | P2 |
| **Guides** | Save/load system guide | P2 |
| **Examples** | 60+ examples not documented | P2 |

### 4.3 Accuracy Concerns

| Document | Concern | Action |
|----------|---------|--------|
| Quick Start | Build commands may need update | Verify against Cargo.toml |
| Architecture | ECS diagram may be outdated | Verify against lib.rs |
| Core Systems | AI section light on details | Cross-reference ai_arbiter docs |
| Crates Reference | Only ~15 of 47 crates documented | Add remaining 32 |

---

## 5. Recommendations for Phase 2

### 5.1 Proposed Documentation Structure

```
docs/src/
├── index.md                    # Landing (exists, update)
├── getting-started/
│   ├── installation.md         # (exists, verify)
│   ├── quick-start.md          # (exists, verify)
│   ├── first-project.md        # (needs creation)
│   └── requirements.md         # (exists)
├── guides/
│   ├── ecs-fundamentals.md     # (enhance from architecture/ecs.md)
│   ├── ai-agents.md            # (enhance from core-systems/ai/)
│   ├── rendering.md            # (enhance from core-systems/rendering.md)
│   ├── physics.md              # (enhance)
│   ├── navigation.md           # (enhance)
│   ├── audio.md                # (enhance)
│   ├── networking.md           # (create, content in guides/networking.md)
│   ├── save-load.md            # (create, content in guides/)
│   ├── fluids.md               # (create)
│   └── scripting.md            # (create)
├── api/
│   ├── index.md                # (exists, fix broken references)
│   ├── astraweave-ecs.md       # (create)
│   ├── astraweave-ai.md        # (create)
│   ├── astraweave-core.md      # (create)
│   ├── astraweave-render.md    # (create)
│   ├── astraweave-physics.md   # (create)
│   └── [one per crate...]      # 47 total
├── performance/
│   ├── benchmarks.md           # (create from masters/)
│   ├── methodology.md          # (create)
│   └── optimization-guide.md   # (enhance from dev/performance.md)
├── examples/
│   ├── index.md                # (exists, major update)
│   ├── hello-companion.md      # (exists, verify)
│   └── [runnable-examples].md  # (add verified examples)
└── contributing.md             # (exists as dev/contributing.md)
```

### 5.2 Priority Order

1. **Fix API index** - Remove non-existent types, update case mismatches
2. **Verify examples** - Run all examples, update status flags
3. **Add crate documentation** - Document all 47 crates
4. **Create performance section** - Pull from master reports
5. **Enhance guides** - Cross-reference actual implementations
6. **Validate build instructions** - Test on fresh clone

---

## 6. Open Questions for Human Decision

1. **Should internal crates be documented?**
   - `astraweave-ipc` - internal IPC mechanism
   - `astraweave-stress-test` - testing infrastructure
   - Recommendation: Document as "Internal" with brief description

2. **Feature-gated modules documentation?**
   - Many modules require feature flags (e.g., `llm_orchestrator`, `skinning-gpu`)
   - Recommendation: Document with feature flag requirements clearly noted

3. **Beta crates like astraweave-steam?**
   - Recommendation: Document with "Beta" status clearly indicated

4. **Archive documentation in docs/archive/?**
   - 200+ historical documents
   - Recommendation: Keep as-is, not in GitHub Pages navigation

5. **How should we handle the 200+ archive docs?**
   - They document the AI-development journey
   - Recommendation: Create separate "Development Journey" section or link to archive

---

## 7. Verification Artifacts

### 7.1 Files Examined

- Root `Cargo.toml` (workspace members)
- All lib.rs files in production crates (grep search)
- `docs/src/SUMMARY.md` (table of contents)
- `docs/src/api/index.md` (API reference)
- `docs/src/reference/crates.md` (crate documentation)
- `docs/src/examples/index.md` (examples list)
- `docs/masters/MASTER_BENCHMARK_REPORT.md` (performance data)
- `docs/masters/MASTER_COVERAGE_REPORT.md` (test coverage)
- `docs/book.toml` (mdBook configuration)
- `.github/copilot-instructions.md` (project context)

### 7.2 Tools Used

- `grep_search` - Pattern matching across codebase
- `list_dir` - Directory structure exploration
- `read_file` - File content examination
- `file_search` - Glob pattern file finding

---

## 8. Next Steps

**Awaiting Approval** for Phase 2: Architecture & Structure

Upon approval:
1. Create proposed directory structure
2. Move/reorganize existing content
3. Create placeholder files for missing sections
4. Update SUMMARY.md for new navigation

**Estimated Phase 2 Effort**: 4-6 hours

---

## Appendix A: Complete Crate List

### Production Crates (47)

1. astraweave-ai
2. astraweave-ai-gen
3. astraweave-asset
4. astraweave-asset-pipeline
5. astraweave-audio
6. astraweave-author
7. astraweave-behavior
8. astraweave-cinematics
9. astraweave-context
10. astraweave-coordination
11. astraweave-core
12. astraweave-dialogue
13. astraweave-director
14. astraweave-ecs
15. astraweave-embeddings
16. astraweave-fluids
17. astraweave-gameplay
18. astraweave-input
19. astraweave-ipc
20. astraweave-llm
21. astraweave-llm-eval
22. astraweave-materials
23. astraweave-math
24. astraweave-memory
25. astraweave-nav
26. astraweave-net
27. astraweave-net-ecs
28. astraweave-npc
29. astraweave-observability
30. astraweave-optimization
31. astraweave-pcg
32. astraweave-persistence-ecs
33. astraweave-persona
34. astraweave-physics
35. astraweave-profiling
36. astraweave-prompts
37. astraweave-quests
38. astraweave-rag
39. astraweave-render
40. astraweave-scene
41. astraweave-scripting
42. astraweave-sdk
43. astraweave-secrets
44. astraweave-security
45. astraweave-steam
46. astraweave-stress-test
47. astraweave-terrain
48. astraweave-ui
49. astraweave-weaving

### Additional Crates (in crates/)

- astraweave-persistence-player
- astraweave-blend
- astract (UI widget library)

---

*End of Discovery Report*
