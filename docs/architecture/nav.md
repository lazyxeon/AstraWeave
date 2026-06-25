---
schema_version: 1
trace_id: nav
title: "Navigation / Pathfinding System (navmesh)"
description: "Navigation / Pathfinding — navmesh (resolves the two-`nav`-crate conflict)"
primary_crate: astraweave-nav
domain: physics-world
lifecycle_status: active
integration_status: wired
summary: "Resolves the two-nav-crate conflict (astraweave-nav vs crates/astraweave-nav). nav.md §6"
owns: [astraweave-nav]
doc_version: "1.1"
last_verified_commit: 7c29b8182
---

# Architecture Trace: Navigation / Pathfinding System (navmesh)

## Metadata

| Field | Value |
|---|---|
| **System name** | Navigation / Pathfinding System (navmesh) |
| **Primary crates** | `astraweave-nav` (top-level, workspace member) |
| **Document version** | 1.1 |
| **Last verified against commit** | `7c29b8182` |
| **Last verified date** | 2026-06-25 |
| **Status** | Active (single-file library); consumers are mostly examples/tests — see §1 status note and §6 |
| **Owner notes** | [Reasoning not recovered from available sources] — crate predates the architecture-trace campaign. Phase 10 "AI-Orchestrated Dynamic Terrain" added the region-invalidation block (lib.rs §520-629). |

---

## 1. Executive Summary

**What this system does:**
`astraweave-nav` bakes a triangle-based navigation mesh from a list of geometric triangles (slope/upward-facing filtering + shared-edge adjacency), runs A* over the triangle-adjacency graph, and returns a smoothed waypoint path. It also tracks "dirty regions" (AABBs) for incremental rebake when terrain changes at runtime.

**Why it exists:**
It provides the navigability check + path query that the AI tool-validation layer (`MoveTo`), the scripting layer (Rhai `NavMeshProxy`), and the editor "Bake Navmesh" tool consult to answer "can an agent get from A to B, and along what route."

**Where it primarily lives:**
- [`astraweave-nav/src/lib.rs`](../../astraweave-nav/src/lib.rs) — the entire production implementation (~810 LoC of code; the rest of the 2,661-line file is `#[cfg(test)]` modules).
- Test/bench-only siblings: `astraweave-nav/src/{mutation_tests,stress_tests,edge_case_tests}.rs`, `astraweave-nav/tests/*.rs`, `astraweave-nav/benches/navmesh_benchmarks.rs`.

**Status note:**
The crate itself is Active and is a declared workspace member. However, of its non-test consumers, the most "real" runtime wiring is the **scripting** `NavMeshProxy` (reads `NavMesh` as an ECS resource) and the **editor** Bake button. The AI tool-sandbox nav check (`astraweave-ai/src/tool_sandbox.rs:400-410`) is present but its enabling path (`ValidationContext::with_nav`) has **zero non-test callers** — see §6. The `astraweave-gameplay::weaving` consumer bakes a navmesh and immediately discards it (`let _nav = …`). A second on-disk directory, [`crates/astraweave-nav/`](../../crates/astraweave-nav), is an **orphan** containing only one benchmark file and no crate manifest — see §6.

---

## 2. Authoritative Pipeline

```text
[Caller supplies &[Triangle], max_step, max_slope_deg]
    │
    │ NavMesh::bake(tris, max_step, max_slope_deg)
    ▼
[Stage 1: Triangle filtering → NavTri set]
    file: astraweave-nav/src/lib.rs  (bake, lines 430-468)
    role: drop degenerate (zero-normal) tris; drop downward-facing tris;
          drop tris whose angle-from-vertical exceeds max_slope_deg
    key data: Vec<NavTri> with idx, verts[3], normal, center, empty neighbors
    │
    │ O(n²) pairwise share_edge() test, eps = 1e-3
    ▼
[Stage 2: Adjacency build]
    file: astraweave-nav/src/lib.rs  (bake adjacency loop 470-479; share_edge 716-726)
    role: two NavTris are neighbors if they share >=2 vertices within eps
    key data: NavTri.neighbors: Vec<usize> populated symmetrically
    │
    │ (NavMesh now constructed; query phase below)
    ▼
[Stage 3: Endpoint snap]
    file: astraweave-nav/src/lib.rs  (find_path 490-497; closest_tri 728-737)
    role: map start/goal Vec3 to nearest NavTri by squared-distance to center
    key data: (start_tri_idx, goal_tri_idx) or empty path if mesh empty
    │
    │ astar_tri(&tris, s, g)
    ▼
[Stage 4: A* over triangle graph]
    file: astraweave-nav/src/lib.rs  (astar_tri 739-800)
    role: BinaryHeap-based A*; edge cost = center-to-center distance,
          heuristic = straight-line center distance to goal
    key data: Vec<usize> triangle-index path (empty if disconnected)
    │
    │ seed pts = [start] + interior tri centers + [goal]  (find_path 503-512)
    ▼
[Stage 5: Path point assembly + smoothing]
    file: astraweave-nav/src/lib.rs  (find_path 503-517; smooth 802-813)
    role: build waypoint list from triangle centers; 2 passes of a
          0.25/0.5/0.25 weighted neighbor-average smoothing (endpoints fixed)
    key data: Vec<Vec3> waypoints returned to caller
    ▼
[Output: Vec<Vec3> path, or empty Vec on no-path / empty-mesh]
```

### Stage-by-stage detail

#### Stage 1: Triangle filtering
**File(s):** `astraweave-nav/src/lib.rs` (`NavMesh::bake`, lines 430-468)
**Role:** Convert raw `Triangle`s into walkable `NavTri`s.
**Inputs:** `&[Triangle]`, `max_step: f32`, `max_slope_deg: f32`.
**Outputs:** `Vec<NavTri>` (filtered).
**Notes:** Normal is `(b-a)×(c-a)` normalized. Three filters apply: (a) `normalize_or_zero()` length² < 1e-6 → drop (degenerate), lines 438-440; (b) `n.dot(Vec3::Y) < 0.0` → drop (downward-facing), lines 444-448; (c) `angle_from_vertical > max_slope_deg` → drop, lines 450-458. `angle_from_vertical = acos(clamp(dot(n,Y),-1,1)).to_degrees()`. **`max_step` is stored on the struct but is not consulted by `bake` or `find_path`** — see §6 trap and §8 invariant 6.

#### Stage 2: Adjacency build
**File(s):** `astraweave-nav/src/lib.rs` (`bake` 470-479; `share_edge` 716-726)
**Role:** Link triangles that share an edge.
**Inputs:** filtered `Vec<NavTri>`.
**Outputs:** populated `neighbors` lists.
**Notes:** `share_edge` counts vertex pairs within `eps = 1e-3` and returns true when `shared >= 2`. The build is **O(n²)** in triangle count (nested `for i / for j>i`). This is the dominant cost at high triangle counts (10K-tri bake = 473 ms, §9).

#### Stage 3: Endpoint snap
**File(s):** `astraweave-nav/src/lib.rs` (`find_path` 490-497; `closest_tri` 728-737)
**Role:** Find the start and goal triangles.
**Inputs:** `start: Vec3`, `goal: Vec3`.
**Outputs:** `(usize, usize)` triangle indices, or early-return empty `Vec` if either snap fails (only when the mesh is empty).
**Notes:** `closest_tri` is a **linear scan** comparing squared distance from the query point to each triangle's *center* (not a point-in-triangle test). Ties broken by `total_cmp`. A query far outside the mesh still snaps to the nearest center.

#### Stage 4: A* over triangle graph
**File(s):** `astraweave-nav/src/lib.rs` (`astar_tri` 739-800)
**Role:** Shortest triangle-to-triangle route.
**Inputs:** `&[NavTri]`, `start`, `goal` indices.
**Outputs:** `Vec<usize>` triangle path; empty if no connection.
**Notes:** Standard A* with `BinaryHeap` (min-heap via reversed `Ord` on `f`), `HashMap` for `came`/`gscore`. Edge cost and heuristic both use Euclidean center distance, so the heuristic is admissible. Reconstruction (784-799) walks `came` from goal back to start; if the reconstructed head is not `start`, returns empty.

#### Stage 5: Path assembly + smoothing
**File(s):** `astraweave-nav/src/lib.rs` (`find_path` 503-517; `smooth` 802-813)
**Role:** Produce world-space waypoints.
**Inputs:** triangle-index path, `start`, `goal`.
**Outputs:** `Vec<Vec3>`.
**Notes:** Waypoints are `[start, <interior triangle centers>, goal]`. The first and last triangle centers are skipped (`.skip(1).take(len-2)`), so paths within a single triangle return exactly `[start, goal]`. `smooth` runs 2 fixed iterations of `pts[i] = 0.25*pts[i-1] + 0.5*pts[i] + 0.25*pts[i+1]` with endpoints held fixed; the `_tris` parameter is unused (no mesh-constrained smoothing / no string-pulling funnel).

---

## 3. Semantic Vocabulary

| Term | Definition | Used in |
|---|---|---|
| `Triangle` | Pure geometry: three `Vec3` vertices `a,b,c`. No adjacency. Has area/normal/perimeter/degeneracy helpers. | `lib.rs:37-125`; callers pass `&[Triangle]` into `bake` |
| `NavTri` | A *baked, walkable* triangle: `idx`, `verts[3]`, precomputed `normal`, `center`, and `neighbors: Vec<usize>`. | `lib.rs:146-233` |
| `NavMesh` | The baked mesh: `tris: Vec<NavTri>`, `max_step`, `max_slope_deg`, plus dirty-region state. | `lib.rs:419-708` |
| `Aabb` | Axis-aligned bounding box used both as a generic geometry helper and as the dirty-region unit. | `lib.rs:251-406` |
| bake | Construct a `NavMesh` from triangles (filter + adjacency). | `NavMesh::bake` |
| dirty region | An `Aabb` marking terrain that changed and may invalidate the mesh; overlapping regions are merged. | `lib.rs:520-629` |
| rebake | Re-run `bake` (full) when dirty regions exist. `partial_rebake` still calls the full `bake` internally. | `rebake_dirty_regions` 566-576; `partial_rebake` 582-606 |
| max_slope_deg | Maximum allowed angle (degrees) of a triangle's face from vertical (Y) for it to be walkable. | `bake` filter, lines 450-458 |
| max_step | Stored maximum step height. **Currently inert** — not read by any query. | field `lib.rs:421`; §6 trap |

### Terms to NOT confuse

- **`Triangle` vs `NavTri`:** `Triangle` is raw input geometry with no graph data; `NavTri` is the post-bake node with adjacency. Callers construct `Triangle`s; `bake` produces `NavTri`s. Do not add adjacency to `Triangle`.
- **`Aabb` as geometry helper vs `Aabb` as dirty-region:** the same struct serves both roles. In `invalidate_region`/`path_crosses_dirty_region` it means "a region of terrain that changed"; in `bounds()`/`from_triangle()` it is just a bounding box. Same type, two semantics.
- **"slope" angle-from-vertical convention:** `slope_degrees()`/the bake filter measure the angle between the face normal and `Vec3::Y`. 0° = flat horizontal ground, 90° = vertical wall. A `max_slope_deg` of 45 admits gentle ramps and rejects steeper ones.

---

## 4. Cross-System Touchpoints

### Upstream (what feeds this system)

| Source system | Interface | Data | Notes |
|---|---|---|---|
| Caller-supplied geometry | `NavMesh::bake(&[Triangle], f32, f32)` | Raw triangles + slope/step params | nav has **no dependency** on terrain/render; the only Cargo deps are `anyhow` + `glam` ([Cargo.toml](../../astraweave-nav/Cargo.toml)). Triangle sources are constructed by each consumer (editor from `level.obstacles`, examples by hand). |
| Editor obstacle list | `tools/aw_editor/src/main.rs:8521-8558` (`show_navmesh_controls`) | obstacle squares → 2 tris each | "Bake Navmesh" button; falls back to a 9×9 dummy grid when no obstacles exist. |

### Downstream (what consumes this system's output)

| Consumer system | Interface | Data | Notes |
|---|---|---|---|
| Scripting (Rhai) | `astraweave-scripting/src/lib.rs:155-159` reads `world.get_resource::<NavMesh>()`; exposed via `api::NavMeshProxy.find_path` (`api.rs:147-157`) | `Vec<Vec3>` path → Rhai `Vec<Dynamic>` | `NavMeshProxy` holds a `*const NavMesh` and is `unsafe impl Send/Sync` (`api.rs:143-144`). This is the most "wired" runtime consumer: the plugin reads `NavMesh` as an ECS resource each tick. One non-test app inserts a `NavMesh` ECS resource: `examples/scripting_advanced_demo/src/main.rs:32` (`app.world.insert_resource(nav)`, workspace member at root `Cargo.toml:120`) — verified; the only other `insert_resource::<NavMesh>` site is inside `astraweave-scripting`'s `#[cfg(test)]` block (`lib.rs:1038`, mod tests at `lib.rs:600`). No non-example shipping binary was found inserting the resource. |
| AI tool sandbox | `astraweave-ai/src/tool_sandbox.rs:400-410` (`validate_tool_action`, `MoveTo` arm) | calls `nav.find_path(start,goal).is_empty()` to block movement with no path | Reachable **only** when `ValidationContext::with_nav` was used to attach a mesh. `with_nav` has zero non-test callers (§6). |
| Editor | `tools/aw_editor/src/main.rs:498` stores `nav_mesh: NavMesh::bake(&[],…)`; rebakes on button | triangle count for display | Editor-local; not part of the engine runtime loop. |
| `astraweave-gameplay::weaving` | `weaving.rs:113` `let _nav = NavMesh::bake(nav_src, 0.5, 55.0);` | discarded | Result is bound to `_nav` and never used (comment: "demo pathing update"). |
| `astraweave-gameplay::biome` | `biome.rs:1` `use astraweave_nav::Triangle;` | Triangle type reuse | Uses the geometry type, not the mesh/query. |

### Bidirectional / Coupled

- None. `astraweave-nav` neither imports nor is imported in a feedback loop; consumers build triangles, call `bake`/`find_path`, and read results.

---

## 5. Active File Map

| File | Role | Status | Notes |
|---|---|---|---|
| [`astraweave-nav/src/lib.rs`](../../astraweave-nav/src/lib.rs) | Entire production implementation: `Triangle`, `NavTri`, `Aabb`, `NavMesh`, `bake`, `find_path`, region invalidation, free fns `share_edge`/`closest_tri`/`astar_tri`/`smooth` | Active | 2,661 lines total; production code ~lines 1-813, remainder is `#[cfg(test)] mod tests` + inline `mod mutation_tests`/`stress_tests`/`edge_case_tests` declarations |
| [`astraweave-nav/Cargo.toml`](../../astraweave-nav/Cargo.toml) | Crate manifest; deps = `anyhow`, `glam`; declares `navmesh_benchmarks` bench | Active | Workspace member at root `Cargo.toml:45`; path dep `Cargo.toml:209` |
| `astraweave-nav/src/mutation_tests.rs` | Mutation-resistant unit tests (`mod mutation_tests` at lib.rs:34) | Active (test-only) | `#[cfg(test)]` |
| `astraweave-nav/src/stress_tests.rs` | Stress/scale tests (declared at lib.rs:2656) | Active (test-only) | |
| `astraweave-nav/src/edge_case_tests.rs` | Edge-case tests (declared at lib.rs:2661) | Active (test-only) | |
| `astraweave-nav/tests/*.rs` | Integration/behavioral/mutation-hardening tests (`behavioral_correctness_tests`, `mutation_hardening_tests`, `mutation_resistant_tests`, `mutation_resistant_comprehensive_tests`, `slope_debug`, `winding_detector`) | Active (test-only) | |
| [`astraweave-nav/benches/navmesh_benchmarks.rs`](../../astraweave-nav/benches/navmesh_benchmarks.rs) | Criterion benchmarks with inline correctness assertions | Active (bench-only) | Canonical bench; baselines in §9 |
| [`crates/astraweave-nav/benches/navmesh_benchmarks.rs`](../../crates/astraweave-nav/benches/navmesh_benchmarks.rs) | Older/stripped copy of the bench, **no `src/`, no `Cargo.toml`** | Legacy / Orphan | NOT a workspace member; see §6 |

**Status definitions:** Active = canonical, load-bearing. Test-only/bench-only = compiled under test/bench, not in the runtime binary. Orphan = on disk, not declared in any workspace manifest.

---

## 6. Conflict Map / Residue

### Coexisting abstractions

| Abstraction | Files | Status | Disposition |
|---|---|---|---|
| Canonical nav crate | [`astraweave-nav/`](../../astraweave-nav) (root) | Active | Workspace member (`Cargo.toml:45`, path dep line 209). This is the crate every importer resolves to (`astraweave-nav = { path = "astraweave-nav" }`). |
| Orphan nav directory | [`crates/astraweave-nav/`](../../crates/astraweave-nav) | Orphan | Contains **only** `benches/navmesh_benchmarks.rs`. No `Cargo.toml`, no `src/`. It is **not** listed in the workspace `members` (the list has `astraweave-nav`, not `crates/astraweave-nav`; the `crates/*` entries that *are* members are enumerated explicitly, e.g. `crates/astraweave-persistence-player`, `crates/astraweave-blend`, `crates/astraweave-alloc` — nav is absent). Cargo cannot build it. The file is an older variant of the root bench: the root version (post `745c100a8`/`de531fd09` era) adds `assert_triangle_valid`/`assert_path_valid` correctness assertions and uses `criterion::black_box`; the orphan uses `std::hint::black_box` and omits the assertions. |

### Naming collisions

- **`NavMesh`**: only one definition in the workspace (`astraweave-nav/src/lib.rs:419`). A workspace-wide grep for `struct NavMesh` returns this single site — there is **no** duplicate/peer navmesh implementation (unlike the terrain/vertex-format duplications warned about in CLAUDE.md).
- **`Triangle` / `NavTri`**: `astraweave-gameplay::biome` imports `astraweave_nav::Triangle` (`biome.rs:1`) and `weave_portals` imports `astraweave_nav::NavMesh` (`weave_portals.rs:1`; `Triangle` only inside its `#[cfg(test)]` module at `weave_portals.rs:140`) rather than defining their own, so no collision.
- **`Aabb`**: there are other AABB-like types elsewhere in the engine (verified: `astraweave-physics/src/spatial_hash.rs:67` `AABB`, `astraweave-render/src/impostor_bake.rs:63` `Aabb`, `astraweave-scene/src/world_partition.rs:116` `AABB`), but `astraweave_nav::Aabb` is a self-contained type used only within nav for geometry helpers and dirty regions; a workspace grep finds no consumer importing `astraweave_nav::Aabb`. No confirmed cross-crate collision via this crate's surface.

### Known cognitive traps

- **Trap: `max_step` is inert.**
  - Why it's confusing: `NavMesh` stores `max_step` (lib.rs:421), the constructor takes it, the README/doc-comment list it as a baking parameter, and every test/example threads a value through. It implies step-height filtering occurs.
  - What's actually true: `bake` only filters on slope and upward-facing-ness; **no code reads `max_step`** to filter or constrain anything. It is preserved across rebakes (`rebake_dirty_regions` reuses `self.max_step`, line 572) but never consulted in a query. See §8 invariant 6.

- **Trap: pathing uses triangle *centers*, not edges.**
  - Why it's confusing: many navmesh systems use a "funnel"/string-pulling algorithm over shared edges for taut paths.
  - What's actually true: A* edge cost, the heuristic, and the emitted waypoints are all based on triangle **centers** (lib.rs:773, 778, 510). There is no funnel algorithm; `smooth` is a generic 3-point average, not edge-constrained. Paths therefore route center-to-center and are softened, not optimal taut paths.

- **Trap: the AI nav check looks wired but its on-ramp is test-only.**
  - Why it's confusing: `tool_sandbox.rs:400-410` clearly calls `nav.find_path(...).is_empty()` to block `MoveTo`, suggesting the runtime AI loop enforces navigability.
  - What's actually true: that branch only runs when `context.nav_mesh.is_some()`. The only way to set it is `ValidationContext::with_nav`, whose **only call sites are inside `#[cfg(test)]`** (tool_sandbox.rs:556/861/877/2113/2155) and benches. No production `src/` file builds a `ValidationContext` with a nav mesh. The navigability gate is effectively dormant in shipped runtime paths. (Per CLAUDE.md Integration Completeness item 1 and Key Lesson 8: "wired beats tested.")

- **Trap: `partial_rebake` is not partial.**
  - Why it's confusing: the name and docstring promise incremental work.
  - What's actually true: it counts affected triangles, then calls `rebake_dirty_regions`, which does a **full** `NavMesh::bake` of all triangles (lib.rs:599-603, 566-576). The doc-comment itself says "In a production system, you would only rebuild the affected portions."

### Orphan/dormant inventory (CLAUDE.md taxonomy)

- **Orphan source:** `crates/astraweave-nav/benches/navmesh_benchmarks.rs` (file on disk, not in any compiled crate).
- **In-design-but-tested surface:** `ValidationContext::with_nav` + the `MoveTo` nav gate in `astraweave-ai` (tested, zero production callers).
- **Dormant field:** `NavMesh::max_step` (settable, never read by a query).

---

## 7. Decision Log

### Decision: Triangle-graph A* with center-based cost/heuristic
- **Date:** [Unknown — predates trace campaign]
- **Status:** Accepted (in code as of `7c29b8182`).
- **Context:** Need a path query over a baked mesh with minimal dependencies (crate deps are only `anyhow` + `glam`).
- **Decision:** Run A* over the triangle-adjacency graph using triangle-center Euclidean distance as both edge cost and heuristic (`astar_tri`, lib.rs:739-800).
- **Alternatives considered:** [Reasoning not recovered from available sources] — no funnel/string-pulling or portal-edge algorithm is present; whether it was considered is not documented.
- **Consequences:** Admissible heuristic (centers), so A* returns a shortest *center-path*, but emitted waypoints are not edge-tightened. `smooth` softens corners post hoc.

### Decision: Slope + upward-facing filter at bake time; `max_step` stored but unused
- **Date:** [Unknown]
- **Status:** Accepted.
- **Context:** Distinguish walkable ground from walls/ceilings.
- **Decision:** Reject zero-normal, downward-facing, and over-`max_slope_deg` triangles (lib.rs:438-458). `max_step` is accepted as a parameter and stored.
- **Alternatives considered:** [Reasoning not recovered from available sources].
- **Consequences:** Step-height handling is not implemented; `max_step` is inert (§6 trap, §8 inv 6).

### Decision: Phase 10 region-invalidation via merged AABB dirty regions, full rebake
- **Date:** Phase 10 ("AI-Orchestrated Dynamic Terrain"; see `docs/journey/phases/PHASE_10A_DAY_1_ASTRAWEAVE_NAV_PARTIAL.md` and the Phase 10 trackers under `docs/journey/phases/`).
- **Status:** Accepted; in code (lib.rs:520-629).
- **Context:** Runtime terrain changes (the doc-comment at lib.rs:521 attributes this to "AI-Orchestrated Dynamic Terrain") need to mark the mesh stale.
- **Decision:** `invalidate_region` merges overlapping `Aabb`s; `needs_rebake`/`rebake_dirty_regions` drive a full re-bake; `path_crosses_dirty_region` lets callers detect invalidated paths.
- **Alternatives considered:** Incremental partial rebake — scaffolded (`partial_rebake`) but it delegates to a full rebake (§6 trap).
- **Consequences:** Correct but O(full-bake) on any dirty region; the 10K-tri full bake is 473 ms (§9), so frequent invalidation is expensive.

### Decision: `#![forbid(unsafe_code)]` in the nav crate
- **Date:** Commit `e060d7973` ("quality: doc-tests, unsafe audit, fmt, dep hygiene, forbid(unsafe_code)").
- **Status:** Accepted (lib.rs:1).
- **Context:** Workspace-wide unsafe-audit hygiene pass.
- **Decision:** The crate forbids `unsafe`. (Note: the *consumer* `astraweave-scripting` uses `unsafe impl Send/Sync` on its own `NavMeshProxy` raw pointer — that unsafe lives outside nav.)
- **Consequences:** All nav code is safe Rust; no Miri obligation inside this crate.

---

## 8. Known Invariants

| # | Invariant | Checkable? | Enforced by |
|---|---|---|---|
| 1 | Adjacency is symmetric: if `j ∈ tris[i].neighbors` then `i ∈ tris[j].neighbors`. | Yes | `bake` pushes both directions (lib.rs:475-476); `test_navmesh_bake_adjacency_two_triangles` |
| 2 | `find_path` returns `[]` for an empty mesh or when start/goal triangles are disconnected. | Yes | `find_path` 491-501; `test_find_path_empty_navmesh`, `test_find_path_no_connection` |
| 3 | A non-empty path begins exactly at `start` and ends exactly at `goal` (endpoints are appended, not snapped to centers). | Yes | `find_path` 504/512; `smooth` holds endpoints fixed; `path_exists_simple_strip` asserts endpoint coords |
| 4 | Degenerate (zero-area) and downward-facing triangles are excluded from the baked mesh. | Yes | `bake` 438-448; `test_navmesh_bake_filters_steep_slopes`, mutation tests |
| 5 | A triangle is walkable iff angle-from-`Y` ≤ `max_slope_deg`. | Yes | `bake` 450-458; `NavTri::slope_degrees`/`is_walkable` |
| 6 | `max_step` does **not** influence baking or pathfinding (it is stored, never read by a query). | Yes (by absence) | Verified: `max_step` is read only at construction (`lib.rs:483`), rebake-reuse (`lib.rs:572`), and a Debug/status format string (`lib.rs:701/704`); no `bake`/`find_path`/`astar_tri`/filter path reads it. Remaining sites are `#[cfg(test)]` assertions. |
| 7 | `bake` is O(n²) in triangle count (pairwise adjacency). | No (perf, not assert) | `bake` nested loop 472-479; benchmark trend (§9) |
| 8 | Overlapping dirty regions merge into one `Aabb`; non-overlapping ones accumulate. | Yes | `invalidate_region` 527-540; `test_navmesh_invalidate_region_merge` |
| 9 | The crate contains no `unsafe` code. | Yes | `#![forbid(unsafe_code)]` lib.rs:1 (compile-time) |

---

## 9. Performance & Resource Profile

Source: [`docs/masters/MASTER_BENCHMARK_REPORT.md`](../masters/MASTER_BENCHMARK_REPORT.md) §5.5 (benchmarks from `astraweave-nav/benches/navmesh_benchmarks.rs`).

### Hot paths
- **`find_path` (A* + snap + smooth):** A* short path (2-5 hops) ≈ 2.44 µs; medium (10-20 hops) ≈ 54.5 µs; long (50-100 hops) ≈ 17.0 µs. Throughput ≈ 142K queries/s at 100 tris. `closest_tri` snap is a linear scan over all triangles, so per-query cost grows with mesh size.
- **`NavMesh::bake`:** 100 tris ≈ 55.9 µs; **10K tris ≈ 473 ms** — flagged in the report as "Must be async". The O(n²) adjacency loop dominates at scale.

### Cold paths
- **Region invalidation / rebake:** `invalidate_region` is cheap (small AABB merge); `rebake_dirty_regions`/`partial_rebake` pay a **full** `bake` (so the 473 ms figure applies to any rebake of a large mesh). Intended to run on terrain-change events, not per frame.

### Resource ownership
- A `NavMesh` owns its `Vec<NavTri>` and `Vec<Aabb>` dirty regions. In the scripting path it is held as an **ECS resource** and read immutably each tick via `world.get_resource::<NavMesh>()` (`astraweave-scripting/src/lib.rs:155`), then handed to scripts as a `*const NavMesh` inside `NavMeshProxy` (lifetime scoped to the script call, per the SAFETY comment at `api.rs:151-154`).

---

## 10. Testing & Validation

- **Unit tests:** `astraweave-nav/src/lib.rs` `#[cfg(test)] mod tests` (lib.rs:815-2655) — baking, pathfinding, helper, smoothing, AABB, and region-invalidation tests.
- **Mutation tests:** `astraweave-nav/src/mutation_tests.rs` (inline `mod`) + `astraweave-nav/tests/{mutation_hardening_tests,mutation_resistant_tests,mutation_resistant_comprehensive_tests}.rs`. The crate was part of the workspace mutation-testing campaign (commit `de531fd09`; archived shard outputs at `archive/mutation_outputs/mutation_results/nav_shard*of3_*.txt`).
- **Stress tests:** `astraweave-nav/src/stress_tests.rs` — disconnected islands, mazes, repeated queries.
- **Edge-case tests:** `astraweave-nav/src/edge_case_tests.rs`, plus `tests/slope_debug.rs` and `tests/winding_detector.rs` (winding/normal-direction diagnostics).
- **Behavioral correctness:** `astraweave-nav/tests/behavioral_correctness_tests.rs`.
- **Benchmarks:** `astraweave-nav/benches/navmesh_benchmarks.rs` (Criterion) — embeds correctness assertions (`assert_triangle_valid`, `assert_path_valid`) so benches double as validation. Baselines in §9.
- **Miri:** Not required — crate is `#![forbid(unsafe_code)]`.
- **Manual validation:** `examples/navmesh_demo`, `examples/nav_physics_bridge` exercise bake + `find_path` end-to-end as runnable demos.

---

## 11. Open Questions / Parked Decisions

- **Is the AI navigability gate meant to be wired?** `tool_sandbox.rs`'s `MoveTo` nav check is fully implemented but only reachable via `ValidationContext::with_nav`, which has no production caller. Is this intended forward-design, or a missing wire-up in the AI runtime loop? (Context: CLAUDE.md Integration Completeness item 1.)
- **Does any shipping app insert a `NavMesh` ECS resource?** The scripting plugin reads `world.get_resource::<NavMesh>()`; if no app inserts that resource, `nav_ptr` is always null and `NavMeshProxy::find_path` returns `[]`. (Verification note 2026-06-25: one **example** app does insert it — `examples/scripting_advanced_demo/src/main.rs:32` calls `app.world.insert_resource(nav)` after baking a 2-triangle ground mesh, exercising the full `NavMeshProxy` path. The only other insert site is in scripting's own `#[cfg(test)]` block. No non-example shipping binary was found; whether a real runtime app is intended to insert the resource remains the owner's call.)
- **Should `crates/astraweave-nav/` be reconciled?** It is an orphan (no manifest, stale bench copy). Its existence can mislead grep-based discovery into thinking there are two nav crates. (Surfaced factually per §6; disposition is the owner's call.)
- **Is `max_step` intended to gain meaning?** It is plumbed through every API and test but read by nothing. Either step-height filtering is unimplemented forward-design, or the parameter is vestigial.
- **`partial_rebake` incrementality:** the function's own doc-comment notes the partial path is unimplemented (delegates to full bake). Is incremental rebake a planned follow-up?
- **Scaling:** 10K-tri bake at 473 ms (O(n²) adjacency) is flagged "must be async" in the benchmark report. Is large-mesh baking expected to move off the main thread or precompute offline?

---

## 12. Maintenance Notes

**Update this doc when:**
- `astraweave-nav/src/lib.rs` changes the bake filters, A*/heuristic, smoothing, or region-invalidation logic (§2, §8).
- A production caller for `ValidationContext::with_nav` or a `NavMesh` ECS-resource insertion is added (would flip §6/§11 wired-status findings).
- `max_step` becomes load-bearing (would close §6 trap and §8 invariant 6).
- The `crates/astraweave-nav/` orphan is removed or promoted to a member (§6).
- Benchmark baselines shift >10% (§9 — also update `MASTER_BENCHMARK_REPORT.md`).

**Verification process:**
- Confirm the pipeline against `lib.rs` line ranges in §2; re-grep `with_nav`, `get_resource::<NavMesh>`, and `NavMesh::bake` across `--type rust -g '!*test*' -g '!*example*'` to re-confirm the wired/dormant split.
- Re-check the workspace `members` list in root `Cargo.toml` for whether `crates/astraweave-nav` was added.
- Stamp the new commit hash and date in the Metadata table after verification.

---

## Appendix A: Quick reference for agents

**If you're working on this system, remember:**
1. There is exactly **one** `NavMesh` implementation: `astraweave-nav/src/lib.rs`. `crates/astraweave-nav/` is an orphan bench file, not a second crate — do not "fix" it into the build without the owner's call.
2. **`max_step` does nothing.** Don't assume step-height filtering happens; only slope + upward-facing filtering does.
3. Pathing is **center-to-center A* + generic smoothing**, not a funnel/string-pull. Don't claim taut/optimal paths.
4. The AI `MoveTo` nav gate is real code but **dormant** (no production `with_nav` caller). The genuinely-wired runtime consumer is the **scripting `NavMeshProxy`** (ECS `NavMesh` resource) and the **editor** Bake button.

**Files you'll most likely touch:**
- `astraweave-nav/src/lib.rs` (everything)
- `astraweave-nav/benches/navmesh_benchmarks.rs` (if you change perf-sensitive paths)

**Files you should NOT touch without strong reason:**
- `crates/astraweave-nav/benches/navmesh_benchmarks.rs` — orphan, not compiled; editing it has no effect on the build.

**Common mistakes when changing this system:**
- Adding a "second navmesh path" — there is no peer; extend `lib.rs` (CLAUDE.md scope-discipline rule).
- Treating `partial_rebake` as incremental — it does a full `bake`.
- Assuming `find_path` does point-in-triangle containment — `closest_tri` is a nearest-*center* scan, so off-mesh queries still snap somewhere.
- Adding behavior keyed on `max_step` without wiring it into `bake`/`find_path` first.

---

## Appendix B: Historical context

The crate is one of the engine's earliest "P0" foundation crates: it appears throughout the Phase 0 / Week 1-6 coverage and benchmark journey docs (e.g. `docs/journey/phases/PHASE_0_WEEK_1_*`, `docs/archive/NAVIGATION_BENCHMARK_RESULTS.md`) and was a target of the mutation-testing campaign (`docs/journey/phases/PHASE_10_MUTATION_TESTING_PLAN.md`, shard outputs under `archive/mutation_outputs/`). The region-invalidation block (lib.rs §520-629) was added under Phase 10 "AI-Orchestrated Dynamic Terrain" to let runtime terrain edits mark the mesh stale. The `#![forbid(unsafe_code)]` attribute and doc-test/dep-hygiene pass landed in commit `e060d7973`. The minimal dependency footprint (only `anyhow` + `glam`) is deliberate per the README ("Minimal dependency footprint").
