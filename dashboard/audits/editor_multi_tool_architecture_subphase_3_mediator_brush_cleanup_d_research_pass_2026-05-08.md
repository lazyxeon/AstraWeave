# Editor Multi-Tool Architecture — Sub-phase 3 Mediator Brush Cleanup-D Research-Pass

**Date**: 2026-05-08
**Status**: Research-pass complete; Andrew-gate (q) pending for Cleanup-D fix-pass methodology decision.
**Author**: Claude Opus 4.7 (1M context)
**Predecessor commits**: Cleanup-B `f7732d5d9` (no-op marker), Cleanup-A `1900bdc8e`, Real-Fix.E `609f85357`, Real-Fix.D `7067cc03d`, Real-Fix.C `ded9a0457`, Real-Fix.B `eaaa53433`, Real-Fix.A `0f569d212`.

---

## §0 — Executive Summary

Per Andrew-gate (n) n-3 + ChatGPT screen recording analysis, the chunk-boundary continuity layer was suspected to host one or more defects of unclear mechanism. Surface observations: erosion-specific chunk seam artifacts (Andrew-gate verbatim 2026-05-07) plus ChatGPT-surfaced concerns that "deformation directionality appears to change subtly across regions" (broader scope; possibly affects sculpt + smooth + other height-mutation brushes).

This research-pass code-reads the chunk-boundary continuity layer end-to-end (per §1.2 seven sub-items in the Cleanup-D research-pass prompt) and **converges on two primary mechanisms with concrete code evidence, plus one secondary skirt-related mechanism**. Both primary mechanisms localize entirely to `tools/aw_editor/src/terrain_integration.rs` (single file); both have small blast radius (~30-60 lines of code change in a single fix-pass session). **Methodology recommendation: Andrew-gate (q) q-1 (direct fix, single session)**.

### §0.1 Three confirmed mechanisms

1. **M-D5 (Erosion-specific seam — primary for Defect Class 5)**: erosion's per-chunk-local `avg_height` computation diverges between adjacent chunks (chunk A's interior average differs from chunk B's interior average), and erosion's blend formula `erode * (1.0 - falloff * 0.1) + avg_height * falloff * 0.1` settles each chunk's edge toward that chunk's local average — creating a measurable step at the chunk boundary. **Smooth brush has the same mechanism but with much smaller settling factor (`strength * falloff * 0.3` vs erosion's effective settling)**; explains why Andrew observed seams specifically with erosion. Site: `terrain_integration.rs:1874-1896` (avg_height computation) + `terrain_integration.rs:1923-1926` (erosion formula).

2. **M-D9 (Post-brush normal stitching gap — primary for Defect Class 9, also contributes to Defect Class 5)**: chunk-boundary normals are correctly cross-stitched at initial generation (`stitch_edge_normals` called once at `terrain_integration.rs:423`), but the brush dispatch's "fast path patch" at `terrain_integration.rs:1946-1965` recomputes vertex normals using single-chunk `calculate_normal` (lines 926-955) which falls back to `h_center` at chunk edges (clamped half-gradient). After brush stroke, chunk-edge vertex normals diverge between adjacent chunks — visible as a lighting seam.

3. **M-SK (Secondary — skirt vertex height freeze)**: skirts are generated at chunk-initial-generation time with `skirt_drop = chunk_size * 0.015` below surface (e.g., ~96cm for 64m chunks). Brush mutations to surface vertices do NOT update skirt vertex heights; surface vertex height changes but skirt stays frozen at original height. If a brush stroke raises surface significantly, skirts may become visible (or may sink below now-lower surface). Site: `terrain_integration.rs:861-919` (skirt generation; surface-vertex-driven) + brush dispatch fast-path patch limits iteration to `idx < gen_chunk.vertices.len()` where `idx = gz * resolution + gx` (surface vertices only).

### §0.2 Defect Class 5 + 9 merge classification

The two primary defect classes (Defect Class 5 — erosion-specific seams; Defect Class 9 — general chunk-boundary continuity) **share the M-D9 normal-stitching gap as a root contributor**; Defect Class 5 has the **additional M-D5 erosion avg_height divergence** that makes its seams substantially more visible than other height-mutation brushes' seams. Treating them as joint fix-pass scope is appropriate: a single session can address M-D5 + M-D9 with optional skirt-touchup for M-SK.

### §0.3 Methodology recommendation

**Andrew-gate (q) q-1 (direct fix, single session)**. The mechanisms are clear from code-reading; instrumentation chain (parallel to Rounds 1-8) would add a round of overhead without epistemic gain. Estimated fix-pass blast radius: 1 file modified (`terrain_integration.rs`); +30 to +80 lines net; 4-6 new tests.

---

## §1 — Surface Observations

### §1.1 Andrew-gate verbatim (Real-Fix.C 2026-05-07)

> "when using the erosion tool it exposes stitching seams in the chunk boundaries, none of the other tools cause this issue that i could produce but the erode tool causes it almost everytime."

**Formalization**:
- **Defect class label**: Defect Class 5 — Erosion-specific chunk seam artifacts.
- **Trigger**: Erode brush mode (`BrushMode::Erode`) applied across or near chunk boundaries.
- **Symptom**: Visible "stitching" line at chunk boundary; surface appears discontinuous.
- **Frequency**: "Almost every time" — high-probability triggered by erosion.
- **Other brush modes**: NOT observed to produce visible seams (Sculpt, Lower, Smooth, Flatten, Noise, Paint, ZoneBlend).
- **Severity**: Medium (cosmetic but consistently reproducible).

### §1.2 ChatGPT screen recording analysis (2026-05-08)

> "deformation directionality appears to change subtly across regions. That suggests chunk-local evaluation rather than globally continuous sampling."

**Formalization**:
- **Defect class label**: Defect Class 9 — General chunk-boundary continuity.
- **Trigger**: Possibly broader than erosion-specific; observed across multiple sculpt operations.
- **Symptom**: Subtle directional change in deformation at chunk boundaries.
- **Frequency**: Subtle; possibly perceptual artifact of video bitrate; possibly genuine mechanism.
- **ChatGPT hypothesis candidates** (not runtime-verified):
  - LOD discontinuity at chunk boundaries.
  - Per-chunk normal calculation with no cross-chunk normalization.
  - Vertex position discontinuity (seam vertices not snapped).
  - Skirt geometry interaction.
  - Brush radius clipped per chunk.
  - Neighboring chunk data fetched asynchronously.
  - Compute dispatch overlap gaps.
  - Brush influence evaluated in chunk-local space incorrectly.

### §1.3 What this research-pass adds

- Verified vs refuted ChatGPT hypotheses (per §3 enumeration below).
- Surfaced two NEW mechanism candidates (M-D5 erosion avg_height divergence; M-SK skirt vertex height freeze) not in ChatGPT enumeration.
- Confirmed shared root mechanism (M-D9 normal-stitching gap) underlying both Defect Class 5 and Defect Class 9.

---

## §2 — Chunk-Boundary Continuity Layer Architectural Map

Per §1.2 pre-execution code-reading findings.

### §2.1 Chunk definition + management (§1.2.1)

**Chunk identity**: `ChunkId { x: i32, z: i32 }` (integer grid coordinates).

**Chunk storage**: `self.generated_chunks: HashMap<ChunkId, GeneratedChunk>` at `terrain_integration.rs`.

**Per-chunk state** (`GeneratedChunk` struct):
- `chunk: Chunk` — the `astraweave_terrain::Chunk` containing heightmap + biome_map.
- `vertices: Vec<TerrainVertex>` — surface vertices (resolution × resolution) + skirt vertices.
- `indices: Vec<u32>` — triangulated surface + skirt geometry.

**Heightmap resolution**: configurable per chunk; typical 64×64 or 128×128. Cell size: `chunk_size / (resolution - 1)`.

**World coordinate mapping**: `chunk_origin_x = chunk_id.x as f32 * chunk_size`; vertex at (gx, gz) lives at world `(chunk_origin_x + gx * cell_size, height, chunk_origin_z + gz * cell_size)`.

**Adjacent chunk relationship**: chunks at `(x, z)` and `(x+1, z)` share the vertex column at `gx = resolution-1` of chunk A = `gx = 0` of chunk B (in world coordinates). However, these are STORED as INDEPENDENT vertex arrays in `gen_chunk.vertices` of each chunk; there is no shared-memory vertex sync mechanism.

### §2.2 Per-chunk normal calculation (§1.2.2) — CRITICAL FINDING

**Function**: `TerrainState::calculate_normal(heightmap, x, z, cell_size)` at `terrain_integration.rs:926-955`.

**Algorithm**: central-difference gradient from heightmap neighbors:
```rust
let h_left  = if x > 0              { heightmap.get_height(x-1, z) } else { h_center };
let h_right = if x < resolution - 1 { heightmap.get_height(x+1, z) } else { h_center };
let h_up    = if z > 0              { heightmap.get_height(x,   z-1) } else { h_center };
let h_down  = if z < resolution - 1 { heightmap.get_height(x,   z+1) } else { h_center };
let dx = (h_right - h_left) / (2.0 * cell_size);
let dz = (h_down  - h_up  ) / (2.0 * cell_size);
Vec3::new(-dx, 1.0, -dz).normalize()
```

**Edge-vertex behavior**: at chunk edges, the missing neighbor is REPLACED with `h_center` (clamped half-gradient). This means edge-vertex normals are CHUNK-LOCAL and IGNORE the neighbor chunk's height data.

**Cross-chunk normal stitching**: `TerrainState::stitch_edge_normals(&mut self, chunk_size)` at `terrain_integration.rs:961-1081`.

**Algorithm**: for each chunk, fetch adjacent chunks' edge height columns/rows, then recompute edge-vertex normals using full central-difference across the chunk boundary:
```rust
let h_left = if x > 0 {
    hm.get_height(x-1, z)
} else if let Some(ref lh) = left_heights {
    lh.get(z).copied().unwrap_or(h_center)
} else {
    h_center
};
// ... similar for h_right, h_up, h_down
```

**Call-sites of `stitch_edge_normals`**: EXACTLY ONE — line 423 in `generate_chunks_around` (or whatever function calls it after initial generation loop). Brush dispatch does NOT call `stitch_edge_normals`.

**This is the M-D9 mechanism**:
- Initial chunk generation: `stitch_edge_normals` fires → edge normals are cross-chunk-correct.
- Brush stroke modifies vertex heights → fast-path patch at line 1946-1965 recomputes vertex normals using single-chunk `calculate_normal` → edge-vertex normals revert to clamped half-gradients.
- Adjacent chunks' edge-vertex normals diverge → visible lighting seam.
- Symptom present for ALL height-mutation brushes (Sculpt, Lower, Smooth, Flatten, Erode, Noise) that hit chunk-edge vertices. Whether the seam is *perceptually visible* depends on brush strength + viewing angle + how much height changed at the seam.

### §2.3 Vertex position at chunk boundaries (§1.2.3)

**Position storage**: each chunk's `vertices: Vec<TerrainVertex>` stores per-vertex position in world coordinates (not chunk-local).

**Edge-vertex positions**: surface vertices at chunk edges have world-X positions of `chunk_origin_x + 0` (left edge) and `chunk_origin_x + chunk_size` (right edge). Two adjacent chunks share the same world-X position for their respective edge columns; height values come from each chunk's heightmap.

**Cross-chunk vertex sync**: NONE. There is no shared-memory vertex sync — chunks are independent vertex arrays.

**Brush mutation behavior**: a brush stroke that crosses chunk boundary iterates `for chunk_id in chunk_ids` (line 1843) → for each affected chunk, iterates `for gz in 0..resolution { for gx in 0..resolution { ... } }` (line 1898) → modifies that chunk's own heightmap independently.

**Critical**: each chunk's brush dispatch reads its own `chunk_origin_x + gx * cell_size` to determine vertex's world position, then checks distance to brush center. So if brush center is at world `(20.0, _, 0.0)` and chunks tile at `chunk_size = 64`, chunks at `(0, 0)` and `(0, _, _)` both see the same brush center; their edge-vertex distance computations are consistent.

**Height continuity at chunk boundary**: as long as both chunks' heightmaps agree on the height at their shared boundary, vertex positions are continuous. The heightmap data is initially generated coherently across chunks (procedural noise is world-coordinate-driven, not chunk-local), so initial positions agree. Brush mutations CAN cross chunk boundaries — at world position X = chunk_size (the shared edge), chunk A's `gx = resolution-1` and chunk B's `gx = 0` are both within radius, both get mutated with the same `dist`, `falloff`, and effectively the same `new_h` formula (for brushes without per-chunk state) — height stays coherent.

**Refutes ChatGPT hypothesis "Vertex position discontinuity"**: heights coherent at chunk boundaries for non-erosion non-smooth brushes; per-chunk avg_height divergence (M-D5) is the erosion-specific divergence, not a general position discontinuity.

### §2.4 Skirt geometry handling (§1.2.4)

**Site**: `terrain_integration.rs:861-919` in `generate_heightmap_mesh`.

**Configuration**: `skirt_drop = chunk_size * 0.015` (e.g., 64m chunk → ~96cm drop).

**Generation algorithm**: for each of the 4 chunk edges (bottom z=0, top z=res-1, left x=0, right x=res-1), duplicate edge surface vertices with Y lowered by `skirt_drop` and outward-pointing normal `[0, -1, 0]`. Connect via quad strips (`surface[i] → skirt[i] → surface[i+1] → skirt[i+1]`).

**Purpose**: "tiny skirt — just enough to hide inter-chunk gaps" (per code comment at line 865). When two adjacent chunks' edge heights momentarily diverge (rendering coordinate snapping, sub-pixel rounding, frame-stale heightmap data), the downward skirt fills the visual gap so the sky dome doesn't show through.

**Skirt vertex storage**: skirt vertices appended AFTER surface vertices in `gen_chunk.vertices`. So `idx < resolution * resolution` are surface vertices; `idx >= resolution * resolution` are skirt vertices.

**Brush mutation behavior** (CRITICAL):
The brush dispatch's "fast path patch" at line 1949-1965 iterates ONLY surface vertices (loops `for gz in 0..resolution as usize { for gx in 0..resolution as usize { let idx = gz * resolution + gx; ... } }`). Skirt vertices (`idx >= resolution * resolution`) are NOT touched.

**Consequence (M-SK)**: After brush stroke modifies a chunk-edge surface vertex, that vertex's height changes (e.g., +5m via Sculpt) but the corresponding skirt vertex stays at its original height (now ~6m below the new surface height instead of 96cm). The skirt position becomes effectively detached from the surface.

**Visual impact**:
- If brush lowers terrain significantly (Lower / Erode), surface vertex may sink BELOW skirt vertex → skirt now exposes its top edge to view (visible artifact).
- If brush raises terrain significantly (Sculpt / Noise), surface vertex rises above skirt → skirt no longer hides inter-chunk gap (gap may become visible).
- Erosion at chunk boundary: surface settles to (chunk-local avg_height ≈ 4.7m for chunk A vs 5.0m for chunk B), creating a step. Skirt is below both at original height; doesn't help mask the step itself.
- Erosion's seam visibility is therefore COMPOUNDED by skirt-vs-surface decoupling.

### §2.5 Brush radius / footprint at chunk boundaries (§1.2.5)

**Site**: `TerrainState::apply_brush` at `terrain_integration.rs:1824-1978`.

**Algorithm**:
1. Collect all chunk IDs in workspace (line 1841): `chunk_ids: Vec<ChunkId> = self.generated_chunks.keys().cloned().collect()`.
2. For each chunk, AABB-reject if brush circle doesn't overlap chunk's world bounds (lines 1843-1854).
3. For overlapping chunks, iterate all chunk vertices `for gz in 0..resolution { for gx in 0..resolution { ... } }`.
4. For each vertex, compute `dist = ((px - world_x).powi(2) + (pz - world_z).powi(2)).sqrt()`; reject if `dist > radius`.
5. Apply mutation per brush mode.

**Cross-boundary behavior**: the brush DOES correctly extend across chunk boundaries — it iterates ALL chunks and modifies all in-radius vertices of all of them. No per-chunk clipping at the boundary; brush footprint is world-space, not chunk-space.

**Refutes ChatGPT hypothesis "Brush radius clipped per chunk"**: brush footprint is correctly world-coordinate-driven; refuted by code-reading.

**Per-brush variance at boundaries**:
- **Sculpt, Lower, Noise**: stateless per-vertex; no cross-vertex dependency; behave continuously across chunk boundaries (modulo M-D9 normal stitching gap).
- **Flatten**: uses `flatten_target` which is a single world-space height (captured at first-click via `sample_height_at`). Continuous across boundaries.
- **Smooth**: uses `avg_height` PER-CHUNK (line 1874-1896 computes within current chunk only; reset between chunks in the for-loop). Adjacent chunks' avg_height differ → adjacent edges blend toward different averages. Same mechanism as erosion (M-D5) but with `blend = strength * falloff * 0.3` (small settling factor).
- **Erode**: same per-chunk avg_height + larger effective settling toward avg_height → most visible seam.

### §2.6 Erosion algorithm specifics (§1.2.6)

**Site**: `terrain_integration.rs:1923-1926`.

**Formula**:
```rust
BrushMode::Erode => {
    let erode = current_h - strength * falloff * 3.0;
    erode * (1.0 - falloff * 0.1) + avg_height * falloff * 0.1
}
```

**Breakdown**:
1. `erode = current_h - strength * falloff * 3.0` — primary erosion: lower height by `strength * falloff * 3.0` (e.g., at brush center with strength=0.5: -1.5m).
2. `erode * (1.0 - falloff * 0.1) + avg_height * falloff * 0.1` — secondary settling: blend the eroded value toward chunk-local `avg_height` by 10% × falloff.

**At brush center** (falloff = 1.0): blend factor = 0.1; vertex ends at `erode * 0.9 + avg_height * 0.1`.
**At brush edge** (falloff = 0.0): blend factor = 0.0; vertex ends at `erode = current_h - 0 = current_h` (unchanged).

**Per-chunk avg_height (M-D5 mechanism)**: `avg_height` is computed within the per-chunk dispatch loop at line 1874-1896. It samples ALL in-radius heights WITHIN THE CURRENT CHUNK ONLY. If brush spans two chunks, each chunk's avg_height is computed from its own interior — different values.

**Numerical example**:
- Chunk A heightmap interior in brush radius: heights = [4.5, 4.8, 5.0, 5.2, 5.5]; avg_height_A = 5.0m.
- Chunk B heightmap interior in brush radius: heights = [4.2, 4.5, 4.7, 4.9, 5.0]; avg_height_B = 4.66m.
- Edge vertex at world boundary: distance to brush center = d → falloff = falloff_curve.eval(d/radius).
- Chunk A's edge vertex: settles to `erode * 0.9 + 5.0 * 0.1 = erode * 0.9 + 0.5`.
- Chunk B's edge vertex: settles to `erode * 0.9 + 4.66 * 0.1 = erode * 0.9 + 0.466`.
- Height step at boundary: `(5.0 - 4.66) * 0.1 = 0.034m × falloff_at_edge`.
- For falloff_at_edge = 0.7 and strength = 1.0: step = 0.024m visible.

**Per-vertex impact across the chunk boundary**: ~2-5cm height step per pass. Visible because:
1. Light-source angle amplifies small height steps into visible shadows.
2. Erosion is typically used in repeated strokes; step accumulates per stroke.
3. The post-brush normal recomputation at the same chunk boundary uses clamped half-gradient (M-D9) → normals diverge → lighting seam compounds the height step.

**Refutes ChatGPT hypothesis "Compute dispatch overlap gaps"**: brush dispatch is single-threaded; no parallel-dispatch race.
**Confirms ChatGPT hypothesis "Per-chunk normal calculation"**: confirmed at M-D9 mechanism but more specifically the brush dispatch's post-mutation normal recomputation, not initial generation.

### §2.7 Cross-chunk update / dirty-marking semantics (§1.2.7)

**Site**: `terrain_integration.rs:1967-1970` (chunk dirty-marking in `apply_brush`).

**Algorithm**: when a chunk is modified by brush stroke, mark its index in `self.dirty_chunk_indices` for incremental GPU re-upload on next frame.

**Cross-chunk behavior**: ALL affected chunks get their indices pushed to `dirty_chunk_indices`. No single-chunk gating; multi-chunk strokes correctly invalidate all touched chunks.

**Race condition risk**: NONE visible — brush dispatch is single-threaded; chunks process sequentially within `apply_brush`.

**GPU re-upload consistency**: dirty chunks re-uploaded via `update_terrain_chunk` (Real-Fix.B's `upload_or_update_terrain_chunk_forward` helper); splat re-builds happen via `engine_adapter` (Real-Fix.D's canonical pipeline). No inter-chunk inconsistency in the upload chain.

**Refutes ChatGPT hypotheses "Neighboring chunk data fetched asynchronously" + "Compute dispatch overlap gaps"**: dispatch is single-threaded; cross-chunk consistency preserved.

---

## §3 — Defect Class Hypothesis Enumeration

### §3.1 Defect Class 5 — Erosion-specific chunk seam artifacts

**Primary mechanism (M-D5)**: per-chunk `avg_height` divergence between adjacent chunks; erosion's 10% blend toward avg_height creates a height step at chunk boundary.

**Evidence-grounding**:
- Code site: `terrain_integration.rs:1874-1896` (avg_height computation) + `terrain_integration.rs:1923-1926` (erosion formula).
- Per-chunk dispatch loop confines avg_height to current chunk's interior only.
- Step size ≈ `(chunk_A_avg - chunk_B_avg) * falloff_at_edge * 0.1 * strength` per stroke; cumulative across strokes.

**Compounding contributors**:
- M-D9 normal-stitching gap (post-brush normals at chunk edge use clamped half-gradient).
- M-SK skirt vertex height freeze (skirts may become exposed/lowered).

**Probability**: high. Code-reading directly identifies this mechanism. Matches Andrew's observation specifically about erosion ("almost everytime").

**Other height-mutation brushes**: Smooth has same M-D5 mechanism with smaller settling factor (`0.3` × current_h vs avg_height); Smooth seams are subtler. Sculpt/Lower/Noise/Flatten are M-D5-free (per-vertex stateless or world-space target).

### §3.2 Defect Class 9 — General chunk-boundary continuity

**Primary mechanism (M-D9)**: post-brush vertex-normal recomputation uses single-chunk `calculate_normal` which falls back to `h_center` (clamped half-gradient) at chunk edges; adjacent chunks' edge-vertex normals diverge.

**Evidence-grounding**:
- Code site: `terrain_integration.rs:1946-1965` (fast-path patch) + `terrain_integration.rs:926-955` (`calculate_normal`).
- Generation-time cross-chunk stitching (`stitch_edge_normals` at line 961-1081) is NOT called from brush dispatch — only from initial generation at line 423.
- Adjacent chunks' edge normals revert to chunk-local clamped form after any brush mutation that touches edge vertices.

**Secondary mechanism (M-SK)**: skirt vertex heights not updated after brush mutations; skirts decouple from surface vertex heights.

**Evidence-grounding**:
- Code site: `terrain_integration.rs:861-919` (skirt generation) + `terrain_integration.rs:1949-1964` (fast-path patch limited to surface vertices).
- Mathematical scope: skirts use surface-vertex-height-at-generation; brush updates surface heights only.

**Probability**: high (M-D9); medium (M-SK).

**Visibility per brush mode**:
- Sculpt / Noise / Erode (raise/lower large): high visibility (significant height delta + edge normal change).
- Smooth / Flatten (typical small height delta at edge): low visibility (subtle).
- Lower (large lowering): may expose skirts; medium visibility.
- Paint / ZoneBlend: zero (no height mutation).

### §3.3 Merge or distinct classification

**Conclusion: shared root mechanism + erosion-specific addition**.

- Defect Class 5 and Defect Class 9 both have M-D9 (normal-stitching gap) as a contributor at every edge of every height-mutation stroke.
- Defect Class 5 has the ADDITIONAL M-D5 (per-chunk avg_height divergence) that makes erosion seams substantially more visible than other brushes' seams.
- M-SK (skirt height freeze) contributes to both classes; relatively secondary.

**Joint fix is appropriate**:
- Apply normal stitching after brush dispatch (M-D9 fix).
- Compute avg_height globally across all in-radius chunks (M-D5 fix).
- Update skirt vertex heights when surface vertices update (M-SK fix; optional in same session).

### §3.4 Refuted ChatGPT hypotheses

| ChatGPT hypothesis | Status | Code-reading evidence |
|---|---|---|
| LOD discontinuity at chunk boundaries | REFUTED | No LOD system in editor terrain rendering; single LOD per chunk. |
| Vertex position discontinuity | REFUTED | Heightmaps initially coherent at boundaries; brush dispatch coherently updates same world position from both chunks. Per-vertex positions stay continuous (height continuity comes from coherent heightmap; brush mutation gradient is symmetric across edge). |
| Brush radius clipped per chunk | REFUTED | Brush dispatch iterates all chunks; world-space distance check; correctly extends across boundaries. |
| Neighboring chunk data fetched asynchronously | REFUTED | Single-threaded brush dispatch; no async data fetch. |
| Compute dispatch overlap gaps | REFUTED | No parallel compute dispatch; single-threaded. |
| Brush influence evaluated in chunk-local space incorrectly | REFUTED | Brush influence uses world coordinates throughout; chunk-origin offset correctly applied. |
| Per-chunk normal calculation with no cross-chunk normalization at seams | CONFIRMED + REFINED | Initial generation HAS cross-chunk stitching (`stitch_edge_normals`); brush dispatch does NOT. The "gap" is between two known paths, not a missing capability. |
| Skirt geometry interaction with specific brush operations | CONFIRMED + REFINED | Skirts have surface-vertex-driven generation but brush dispatch's fast-path patch is surface-only; skirts decouple after brush. Secondary mechanism. |

### §3.5 New mechanisms surfaced (not in ChatGPT enumeration)

- **M-D5 erosion per-chunk avg_height divergence** — primary mechanism for Defect Class 5; not in ChatGPT enumeration.
- **M-SK skirt vertex height freeze (compounding)** — refined version of ChatGPT skirt hypothesis with specific mechanism.

---

## §4 — Methodology Recommendation

### §4.1 Defect Class 5 — recommended approach

**Direct fix (Andrew-gate (q) q-1)**.

Rationale:
- M-D5 mechanism surfaced by code-reading without ambiguity.
- Fix is mechanically small (compute avg_height globally across all chunks in radius, not per-chunk).
- Estimated blast radius: ~20-30 lines in `apply_brush` (two-pass avg_height computation: first pass collects samples from all chunks in radius; second pass dispatches per-chunk with global avg_height).
- Tests: 2-4 new tests covering erosion-at-boundary behavior.

### §4.2 Defect Class 9 — recommended approach

**Direct fix (Andrew-gate (q) q-1)**.

Rationale:
- M-D9 mechanism surfaced by code-reading without ambiguity.
- Fix mechanically small (call `stitch_edge_normals` at end of `apply_brush` if any chunk was modified).
- `stitch_edge_normals` already exists at line 961; just needs to be invoked from brush dispatch.
- Estimated blast radius: ~5 lines in `apply_brush` (one call site addition).
- Tests: 2-3 new tests covering normal stitching after brush strokes.

### §4.3 Joint approach if classes share mechanism

**Single fix-pass session addresses both Defect Class 5 + Defect Class 9**:

```rust
// At end of apply_brush, before returning `modified`:
if modified {
    self.stitch_edge_normals(chunk_size);
}
// Also: refactor avg_height computation to be cross-chunk for Smooth/Erode.
```

Estimated blast radius: ~30-60 lines total + 4-6 new tests. Single session; ~1-2 hours of work.

### §4.4 M-SK (skirt height freeze) — optional addendum

**Direct fix can be bundled in same session if scope allows**:

Option A — defer M-SK: skirts continue to decouple after brush; visible artifact remains in edge cases (large height deltas). Document as known limitation; address in follow-up.

Option B — fix M-SK in joint session: extend brush dispatch's fast-path patch to also update skirt vertex heights. Estimated +20-40 lines (need to identify per-chunk skirt vertex index offsets per edge). Recommended IF the joint fix session has bandwidth.

### §4.5 Methodology recommendation summary

| Aspect | Recommendation |
|---|---|
| Cleanup-D fix-pass approach | Direct fix (q-1) |
| Sessions estimated | 1 |
| Blast radius | 1 file (`terrain_integration.rs`); +30-80 lines net |
| Tests | 4-6 new tests |
| Scope | M-D5 + M-D9 (joint); M-SK optional |
| Instrumentation chain | NOT recommended (mechanisms clear from code-reading) |

---

## §5 — Forward Chain Proposal

### §5.1 Cleanup-D fix-pass session shape

**Session 1 — Cleanup-D fix-pass (direct fix; M-D5 + M-D9 joint; M-SK optional)**:
- Apply normal stitching after brush dispatch (M-D9 fix).
- Refactor avg_height to be cross-chunk for Smooth/Erode (M-D5 fix).
- Optionally update skirt vertex heights post-brush (M-SK fix).
- Add 4-8 new tests covering chunk-boundary behavior under each height-mutation brush.
- Andrew-gate verification: erosion seams ELIMINATED; general chunk-boundary continuity PRESERVED.

### §5.2 Optional Session 2 if M-SK deferred

**Session 2 (optional) — Cleanup-D skirt addendum**:
- Update skirt vertex heights post-brush.
- Test coverage for skirt-decoupling edge cases.

### §5.3 Sub-phase 3.C closeout

**Following session — Sub-phase 3.C closeout**: campaign doc Sub-phase 3 COMPLETE. Documents:
- Complete campaign chain commit history (Real-Fix.A through Cleanup-D fix-pass).
- §7.1-§7.7 methodology lessons + candidate §7.8 (audit-era misclassification per Cleanup-B finding).
- §7.7 structural axiom validated at four layers + three granularity scales (Round 5 / 6 / 7 / 8 closure narrative).
- Edit 2 (no second implementation) multi-granularity discipline.
- Canonical pipeline composability.
- Methodology body of practice carry-forward for Sub-phase 4+ AAA-fidelity foundational work.

---

## §6 — Adjacent Concerns Out-of-Scope

### §6.1 Defect Class 6/7/8 brush mathematics

ChatGPT-surfaced concerns: vertex-column striation, falloff inconsistency, frame-rate-dependent accumulation. Adjacent to chunk-boundary work but distinct mechanisms (operate within-chunk, not across-chunk-boundary). **Deferred to potential Sub-phase 7 ("Brush Mathematics Polish")**. Research-pass notes adjacency without scope expansion.

### §6.2 Defect Class 10 — Texture asset content gap

Materials 8-21 share single placeholder texture; visual differentiation pending content work. **Decoupled from chunk-boundary work**; parallel content effort.

### §6.3 LOD system (does not exist in editor)

Editor renders chunks at single LOD; no LOD discontinuity hypothesis applicable. If LOD is added in Sub-phase 4+ AAA-fidelity work, LOD-boundary continuity becomes a new defect class to consider (not Cleanup-D scope).

---

## §7 — Methodology Body of Practice Carry-Forward

### §7.1 Research-pass-first when mechanism is unclear

Cleanup-D research-pass follows the Round 3 audit precedent: when surface observations don't converge on a clear mechanism (especially when ChatGPT analysis enumerates multiple plausible candidates), formal characterization before fix-pass methodology decision yields:
1. Per-defect mechanism with code-reading evidence (refuted/confirmed/refined).
2. Joint vs distinct classification.
3. Direct-fix vs instrumentation-chain recommendation grounded in mechanism clarity.

This research-pass surfaces TWO primary mechanisms by code-reading alone (no instrumentation needed) — direct fix is appropriate. Counterfactual: had mechanisms been ambiguous, instrumentation chain (parallel to Rounds 1-8) would have been recommended.

### §7.2 Methodology lesson §7.8 candidate (per Cleanup-B finding)

Cleanup-B `f7732d5d9` surfaced that `8f4668599`'s "bypass-driven mode collapse approach" framing in the audit was retrospectively imprecise — `8f4668599` was a minimal pre-existing-bug fix, not a bypass mechanism. **Methodology lesson candidate §7.8**: audit-era hypothesis framings can be retrospectively imprecise as campaign chain progresses; verify cleanup scope against post-campaign-chain canonical state, not just audit specifications. This research-pass document records the candidate for Sub-phase 3.C closeout consolidation; the closeout decides whether §7.8 elevates from candidate to canonical lesson.

### §7.3 §7.7 structural axiom (not extended in this research-pass)

§7.7 structural axiom — wrapped-component resource-identity at every boundary — was confirmed at four layers (depth-target, mesh-data, texture-data, UI/renderer-capacity) in Round 5 / 6 / 7 / 8. Cleanup-D's mechanisms (M-D5, M-D9, M-SK) are NOT §7.7 instances — they are state-propagation gaps within a single component (chunk-edge state at brush dispatch vs initial-generation pathways). Different anti-pattern class entirely. Cleanup-D fix-pass does not extend §7.7 evidence; it adds a separate lesson about "state-propagation pathway equivalence" — a brush dispatch should produce equivalent end-state to initial-generation, but the pathways diverged.

### §7.4 Direct-fix vs instrumentation-chain decision discipline

| Scenario | Recommendation |
|---|---|
| Single mechanism surfaced by code-reading without ambiguity | Direct fix |
| Multiple mechanisms; no clear primary; ChatGPT analysis enumerates candidates | Instrumentation chain (Rounds 1-8 pattern) |
| One primary candidate + uncertain secondary; need verification | Hybrid (small instrument + direct fix) |
| Mechanism is broader than cleanup scope (e.g., depends on Sub-phase 4+ infrastructure) | Defer to Sub-phase N |

Cleanup-D research-pass selects "direct fix" per §4.

---

## §8 — Andrew-Gate (q) Decision Points

### §8.1 Primary decision: Cleanup-D fix-pass methodology

| Option | Description | Recommended? |
|---|---|---|
| **q-1** | Direct fix (1 session). M-D5 + M-D9 joint; M-SK optional. | ✅ **RECOMMENDED** |
| q-2 | Multi-round instrumentation chain (3-5 sessions; parallel to Rounds 1-8). | Not recommended; mechanisms clear from code-reading. |
| q-3 | Hybrid (small instrument + direct fix; 2 sessions). | Not recommended; instrumentation would not surface additional mechanism. |
| q-4 | Defer Defect Class 5/9 to Sub-phase 7 or follow-up campaign. | Possible if Andrew prefers to bundle with brush mathematics polish; not recommended if campaign closure preferred soon. |

### §8.2 Secondary decision: M-SK inclusion

| Option | Description | Recommended? |
|---|---|---|
| sk-A | Include M-SK in same Cleanup-D fix-pass session | Recommended IF session bandwidth allows (~+20-40 lines) |
| sk-B | Defer M-SK to follow-up session | Recommended IF Cleanup-D fix-pass session is already tight on scope |

### §8.3 Tertiary decision: methodology lesson §7.8 elevation

| Option | Description |
|---|---|
| §7.8-A | Promote §7.8 from candidate to canonical methodology lesson at Sub-phase 3.C closeout |
| §7.8-B | Keep §7.8 as candidate; reconsider after Sub-phase 6 closeout |

This research-pass surfaces §7.8 candidate; Sub-phase 3.C closeout decides elevation.

---

## §9 — Revision History

- **2026-05-08** (this document, initial publication): Cleanup-D research-pass complete; pre-execution §1.2 seven sub-items code-read; two primary mechanisms (M-D5, M-D9) + one secondary (M-SK) characterized; direct-fix methodology recommended; Andrew-gate (q) pending.
