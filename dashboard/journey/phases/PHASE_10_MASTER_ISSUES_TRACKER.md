# Phase 10: Mutation Testing - Master Issues Tracker

**Version**: 5.0  
**Date**: January 21, 2026  
**Status**: 🎯 ACTIVE TRACKING  
**Purpose**: Comprehensive list of all test quality issues found during mutation testing for systematic remediation

---

## Executive Summary

**Total Issues Found**: 805 (from 7 completed crates)  
**Critical Issues (P0)**: 121 (loops, AABB, quaternion, audio, transform, meshlet, pathfinding, ECS iterators)  
**High Priority (P1)**: 272 (geometry, DSP, GPU buffers, animation, skinning, LOD, AI tools, archetype)  
**Medium Issues (P2)**: 329 (comparisons, boolean accessors, return values, blob_vec)  
**Low Issues (P3)**: 82 (formatting, logging, edge cases)

**Overall Assessment**: Improving quality (70.42% average across 7 crates)
- Math: ⭐ 94.37% (4 issues) - EXCEPTIONAL
- Nav: ⭐ 85.00% (42 issues) - EXCELLENT  
- Core: ⭐ 85.57% (72 issues) - EXCELLENT
- ECS: ⭐⭐⭐ 79.17% (70 issues) - GOOD (just below target)
- Audio: ⚠️ 58.67% (31 issues) - BELOW TARGET
- Scene: ⚠️ 57.59% (218 issues) - BELOW TARGET
- Asset: 🔴 32.60% (368 issues) - CRITICAL

---

## Summary by Crate

| Crate | Score | Grade | Missed | Timeouts | Status |
|-------|-------|-------|--------|----------|--------|
| astraweave-math | 94.37% | ⭐⭐⭐⭐⭐ | 4 | 0 | ✅ Complete |
| astraweave-nav | 85.00% | ⭐⭐⭐⭐ | 42 | 0 | ⚠️ Partial |
| astraweave-core | 85.57% | ⭐⭐⭐⭐ | 72 | 29 | ✅ Complete |
| astraweave-ecs | 79.17% | ⭐⭐⭐ | 70 | 6 | ✅ Complete |
| astraweave-audio | 58.67% | C- | 31 | 40 | ✅ Complete |
| astraweave-scene | 57.59% | C- | 218 | 7 | ✅ Complete |
| astraweave-asset | 32.60% | 🔴 F | 368 | 5 | ✅ Complete |
| **Average** | **70.42%** | **C+** | **805** | **87** | **7/12 P0** |

---

## Issues by Severity

### P0 - CRITICAL (121 issues)

Issues that could cause system failures, data corruption, or infinite loops.

#### astraweave-math (1 issue)
- **Issue #4**: `simd_quat.rs:119` - `normalize_quat_simd` returns Default::default()

#### astraweave-nav (8 issues)
- **Issues #5-12**: AABB `contains` boolean logic

#### astraweave-audio (8 issues)
- **Issues #47-54**: `SimpleSineTts::synth_to_path` arithmetic + timeouts

#### astraweave-scene (27 issues)
- **Issues #78-104**: Transform inverse, is_identity, frustum/cell radius loops

#### astraweave-asset (42 issues)
- **Issues #296-337**: `generate_meshlets` loop timeouts, GLTF animation loading

#### astraweave-core (29 issues)
- **Issues #664-692**: los_clear, path_exists, astar_path, draw_line_obs loops

#### astraweave-ecs (6 issues) ✅ NEW
- **Issues #741-746**: `system_param.rs` Query iterator `*=` mutations (TIMEOUT)
  - Line 133, 139: `Query::next` iterator infinite loop
  - Line 195, 201: `Query2::next` iterator infinite loop
  - Line 271, 277: `Query2Mut::next` iterator infinite loop

---

### P1 - HIGH (272 issues)

Issues that could cause incorrect behavior, visual glitches, or performance problems.

#### astraweave-ecs (27 issues) ✅ NEW
- **Issues #747-759**: `blob_vec.rs` memory management (13 issues)
- **Issues #760-772**: `archetype.rs` entity storage (13 issues)
- **Issues #773**: `entity_allocator.rs` allocation (1 issue)

---

### P2 - MEDIUM (329 issues)

Issues that indicate weak test coverage but lower immediate risk.

#### astraweave-ecs (30 issues) ✅ NEW
- **Issues #774-783**: `blob_vec.rs` remaining mutations (14 issues)
- **Issues #784-793**: `counting_alloc.rs` allocator tracking (10 issues)
- **Issues #794-799**: `events.rs` event handling (6 issues)

---

### P3 - LOW (82 issues)

Issues with minimal production impact.

#### astraweave-ecs (7 issues) ✅ NEW
- **Issues #800-805**: Minor mutations in sparse_set, rng, command_buffer, lib

---

## Detailed Issue Listings

### astraweave-ecs - Hotspot Files

| File | Missed | % of Crate | Key Functions |
|------|--------|------------|---------------|
| blob_vec.rs | 27 | 38.6% | BlobVec memory operations |
| archetype.rs | 13 | 18.6% | Archetype entity storage |
| counting_alloc.rs | 10 | 14.3% | Memory allocation tracking |
| events.rs | 8 | 11.4% | Event queue handling |
| entity_allocator.rs | 3 | 4.3% | Entity ID allocation |
| lib.rs | 3 | 4.3% | Core ECS functions |
| sparse_set.rs | 3 | 4.3% | Component storage |
| rng.rs | 2 | 2.9% | Random number generation |
| command_buffer.rs | 1 | 1.4% | Deferred commands |

### astraweave-ecs - Timeout Locations (P0)

| File | Line | Function | Mutation |
|------|------|----------|----------|
| system_param.rs | 133 | Query::next | `*=` |
| system_param.rs | 139 | Query::next | `*=` |
| system_param.rs | 195 | Query2::next | `*=` |
| system_param.rs | 201 | Query2::next | `*=` |
| system_param.rs | 271 | Query2Mut::next | `*=` |
| system_param.rs | 277 | Query2Mut::next | `*=` |

---

## Remediation Priority Queue

### Immediate (After All Testing)

1. **P0 Timeout Issues** (87 issues across 5 crates)
   - Add loop bounds/iteration limits
   - Estimated: 10 hours

2. **P0 ECS Iterator Safety** (6 issues)
   - Query iterator infinite loop protection
   - Estimated: 2 hours

3. **P0 AABB Logic** (8 issues)
   - Comprehensive AABB boundary tests
   - Estimated: 2 hours

---

## Testing Progress

| Tier | Crates | Complete | Remaining | Avg Score |
|------|--------|----------|-----------|-----------|
| P0 | 12 | 7 (58%) | 5 | 70.42% |
| P1 | 8 | 0 | 8 | - |
| P2 | 5 | 0 | 5 | - |
| **Total** | **25** | **7 (28%)** | **18** | **70.42%** |

---

## Issue Count by Crate (Quick Reference)

| Crate | P0 | P1 | P2 | P3 | Total |
|-------|----|----|----|----|-------|
| math | 1 | 1 | 2 | 0 | **4** |
| nav | 8 | 17 | 15 | 2 | **42** |
| audio | 8 | 12 | 8 | 3 | **31** |
| scene | 27 | 67 | 98 | 25 | **218** |
| asset | 42 | 128 | 158 | 40 | **368** |
| core | 29 | 20 | 18 | 5 | **72** |
| ecs | 6 | 27 | 30 | 7 | **70** |
| **TOTAL** | **121** | **272** | **329** | **82** | **805** |

---

**Status**: 🎯 ACTIVE TRACKING - 7/25 crates complete, 805 issues documented  
**Next Update**: After astraweave-gameplay mutation test completes

