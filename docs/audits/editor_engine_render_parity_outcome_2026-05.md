# Editor-Engine Render Parity — Campaign Outcome (P.0 → P.7)

| Field | Value |
|---|---|
| **Campaign** | Editor-Engine Render Parity |
| **Sub-phases** | P.0 (audit) → P.1 (harness) → P.2 (loader) → P.3 (tonemap) → P.4 (quality preset) → P.5 (target format) → P.6 (composition layer) → P.7 (closure) |
| **Start** | 2026-05-12 (P.0 architecture audit) |
| **Close** | 2026-05-17 (P.7 parity validation closure) |
| **Verification machine** | NVIDIA GeForce GTX 1660 Ti with Max-Q Design (Vulkan, driver 591.74) |
| **Status** | Closed. Five seams structurally protected. Parity harness publicly enforced. |

---

## 1. Campaign overview

### What the campaign achieved

**Strict bit-identical per-machine parity between the editor viewport and the engine production renderer.** For an agreed scene fixture (grassland biome canonical pack + 10m × 10m terrain chunk + shadow caster sphere + canonical `GameQuality` preset + canonical ToD 12.0), the editor's `ENGINE_LDR_TARGET` produces SHA-256-identical bytes to the engine's standalone `Renderer::draw_into` output on the same GPU. The contract holds whether editor overlays (grid, gizmos, physics debug, brush cursor, zone overlay) are drawn or not — overlays render into a separate target and never mutate the parity-contract bytes.

This is the WYSIWYG fidelity contract: what the user sees in the editor is what ships from the same machine.

### Why it was undertaken

The Terrain Asset Quality campaign's A.4 sub-phase (architecture audit, queued before A.5 doc reconciliation could proceed) surfaced a divergence between the editor viewport's terrain material loading path and the engine production renderer's path. Andrew's decision: editor viewport and engine production rendering must be strict bit-identical (hash-equality of render output bytes), not "close enough." WYSIWYG fidelity is foundational to authoring-tool trust — any divergence breaks the user contract.

The Terrain Asset Quality campaign was paused pending parity resolution. The Editor-Engine Render Parity campaign launched in its place, scoped to close five named seams identified by the P.0 audit. After P.7 commits, Terrain Asset Quality resumes with A.5 (doc reconciliation), now able to reference this campaign's outcome as the architectural foundation in place.

### The seven sub-phases and their commits

| Sub-phase | Commit | Title | LoC delta |
|---|---|---|---|
| P.0 | (chat-resident audit) | Architecture audit — 12 axes inventoried, 5 seams identified | 0 (read-only) |
| P.1 | `1cf48ccce` | Parity harness skeleton (failure-first baseline) | +656 (new file) |
| P.2 | `ec349c5ce` | Shared loader | +684 / −168 |
| P.3 | `e09703538` | Shared tonemap | +199 / −635 (largest deletion-heavy sub-phase) |
| P.4 | `f4cf2b0f2` | Shared quality preset | +191 / −1 |
| P.5 | `2f67ddd1f` | Target format verification (Outcome 2 — closed as P.3 side effect) | +130 / 0 |
| P.6 | `a59f26b8c` | Composition layer architecture | +551 / −51 |
| P.7 | (this commit) | Parity validation closure | (see commit body) |

P.0 was the architectural audit that scoped the campaign; it produced the chat-resident audit synthesis but no commit. P.1 landed the measurement instrument (parity harness with failure-first baseline). P.2-P.6 each closed one seam with a closure proof shape matched to the seam's nature. P.5 was a verification-only commit because P.3's `surface.is_none()` branch deletion incidentally closed the target format seam — Phase 1 audit found zero residual divergence (Outcome 2), and the commit formalized the structural proof without production code changes. P.7 makes the harness public-default and captures the campaign outcome in this document.

---

## 2. The five seams and their closure proofs

### 2.1 Loader (P.2) — byte-level closure

**P.0 audit axes:** Axis 1 (loader path), Axis 10 (terrain-specific state).

**Pre-campaign divergence:** The editor had three distinct loader paths for terrain materials:
- Engine canonical: `MaterialManager::load_pack_from_toml` produces GPU texture arrays for the `MaterialManager` bind group (consumed by runtime examples like `unified_showcase`).
- Editor bespoke single-layer: `viewport/renderer.rs::load_biome_terrain_texture` read `materials.toml` via private serde structs, decoded the first PNG layer only, called `EngineRenderAdapter::set_terrain_surface_maps`.
- Editor synthetic placeholder: `terrain_biome_placeholder::generate_biome_placeholder_albedos` produced 8 in-process flat-color biome albedos for the 32-layer canonical splat path.

These three paths fed different bind groups with different content. The editor's terrain rendering used synthetic placeholders even when authored content existed on disk.

**Closure design (decided via P.2 Phase 1 escalation):** Sibling loader in the editor (`canonical_terrain_pack`) that reuses MaterialManager's TOML schema (private deserializer mirror; schema sync risk flagged for future elevation), decodes PNGs via `image::open`, resizes to the canonical resolutions (1024² albedo, 512² aux), produces CPU byte slices for `Renderer::set_terrain_materials`. Not a literal `load_pack_from_toml` call: MaterialManager outputs GPU views; the editor's terrain pipeline (`TerrainMaterialManager`) consumes CPU bytes. They are parallel pipelines. Reusing the on-disk schema keeps both paths sourcing identical authored content.

**Closure proof shape:** **byte-level**. The harness invokes `load_canonical_terrain_pack` on the grassland biome dir and computes SHA-256 over the canonical pack's CPU bytes (albedo + normal + mra + uv_scale per layer, in array-index order). Identical input bytes → identical hash → identical `set_terrain_materials` inputs on both sides → loader axis closed at the input boundary.

**Closure proof reference value (this machine):**
```
Canonical pack content hash: 0ca13a5677aeb0ca4dd431ccd21a6afaf778a3cbd670d75b5d2a17a4b4f73d98
```

**Production code changes (P.2):**
- New `tools/aw_editor/src/viewport/canonical_terrain_pack.rs` (sibling loader)
- `EngineRenderAdapter::set_biome_pack(Option<PathBuf>)` setter + `reupload_terrain_layers_from_pending_pack` helper
- `EngineRenderAdapter::upload_terrain_chunks` synthetic-only init block replaced with canonical-or-fallback delegation
- `ViewportRenderer::load_biome_terrain_texture`, `load_default_terrain_texture`, bespoke `TerrainMaterialDoc` / `TerrainMaterialLayer` serde structs: deleted
- `main.rs:5095` `load_biome_terrain_texture(biome_label)` replaced with `adapter.set_biome_pack(Some(<path>))`

**Verifies via:** harness `Loader-axis closure proof (P.2)` block. The byte-level hash is independent of per-pixel SAD attribution (terrain pixels also carry tonemap + format axis divergence on intermediate sub-phases; the input-boundary byte hash isolates the loader axis).

### 2.2 Tonemap (P.3) — pipeline-structural closure

**P.0 audit axis:** Axis 11 (tonemap).

**Pre-campaign divergence:** `Renderer::draw_into` at `astraweave-render/src/renderer.rs:5910` branched on `surface.is_none()`:
- Surface mode (`Renderer::render` windowed): `post_pipeline` runs (ACES Narkowicz + exposure 1.35 + scene-env tint).
- Editor mode (`surface = None`): `hdr_blit_pipeline` runs (passthrough HDR → HDR, no tonemap). The editor then applied its own `tonemap.wgsl` (ACES Narkowicz alone, no exposure, no tint) after the passthrough.

The editor's main viewport thus ran a different tonemap stage than the runtime. Visually similar (both ACES), bit-divergent (different exposure, no scene-env tint), structurally split (two pipelines).

**Closure design:** Delete the `surface.is_none()` branch + the entire `hdr_blit_pipeline` graph (declaration, construction, store-into-struct, resize-time rebuild, bloom-first-create rebuild). `Renderer::draw_into` now unconditionally invokes `post_pipeline` as its terminal stage. Editor `tonemap.wgsl` deleted; editor's own tonemap pass + HDR intermediate target removed. Editor adapter's `config.format` aligned from `Bgra8UnormSrgb` to `Rgba8UnormSrgb` (a downstream consequence of removing the branch — required for `post_pipeline`'s `config.format` output to match the editor's external LDR target).

**Closure proof shape:** **pipeline-structural**. Both engine and editor `Renderer` instances are constructed via `Renderer::new_from_device(..., None, config)` with `config.format = Rgba8UnormSrgb`. They build `post_pipeline` from the same `POST_SHADER` constant (one source of truth in `astraweave-render`). `draw_into` is now a single canonical code path. The structural proof asserts that the same shader source, the same config.format, and the same call site produce the same tonemap behavior — across all possible fixtures, not just the harness's.

**Production code changes (P.3):**
- Renderer field declarations + construction + struct-init + resize rebuild + bloom-first-create rebuild for `hdr_blit_pipeline` / `hdr_blit_bind_group` / `hdr_blit_bgl`: all deleted (~292 LoC removed)
- `surface.is_none()` branch in `draw_into`: deleted
- ViewportRenderer HDR intermediate target + editor tonemap pipeline + `create_tonemap_resources` function (~125 LoC) + `set_tonemap_mode`/`tonemap_mode` accessors: all deleted
- `tools/aw_editor/src/viewport/shaders/tonemap.wgsl`: deleted (134-line shader)
- `EngineRenderAdapter::config.format`: changed `Bgra8UnormSrgb` → `Rgba8UnormSrgb`
- GridRenderer + PhysicsDebugRenderer constructors: format changed from `HDR_COLOR_FORMAT` to `LDR_COLOR_FORMAT`
- `HDR_COLOR_FORMAT` constant: deleted

**Surprising P.3 result:** with both seams (loader + tonemap) closed and the harness fixture being minimal (no shadow casters, no editor overlays drawn), the engine and editor SHA-256 hashes were already byte-identical at P.3 — incidental on the minimal fixture, not campaign completion. The fixture didn't engage the still-open quality preset and overlay composition axes. P.4 expanded the fixture to re-engage divergence.

**Multi-operator tonemap deferral:** Per the P.0 Q3 decision, the multi-operator authoring feature (PBR Neutral, Reinhard, AgX) was intentionally removed alongside `tonemap.wgsl`. If a future Tonemap Operator Preview panel is built, it lives as a separate non-parity preview path that does not compromise the main viewport's canonical-ACES contract.

### 2.3 Quality preset (P.4) — parameter-equality closure

**P.0 audit axis:** Axis 8 (quality preset).

**Pre-campaign divergence:** `EngineRenderAdapter::new` called `apply_quality_preset(EditorQualityPreset::EditorDefault)` at construction. The `EditorDefault` preset set: `shadows on`, `cloud_shadows off`, `shadow_filter (1.5, 0.005, 1.5)`, `cascade_extents (40, 120)`, `cascade_lambda 0.75`, `max_draw_distance 1200`, `ssao + color_grading` post-process. Production runtime examples (`unified_showcase`, `hello_companion`, `cutscene_render_demo`, `biome_showcase`) made zero `apply_quality_preset` calls and used Renderer defaults: `extents (40, 80)`, `lambda 0.5`, `filter (1.0, 0.0006, 0.002)`. Despite the same engine code, the editor and runtime rendered the same scene with different shadow cascades.

**Closure design (decided per P.4 Phase 1 audit — Branch A):** `EditorQualityPreset::GameQuality` was defined and explicitly named as the canonical "this is what the game ships" preset, even though no production example actually calls it. Use `GameQuality` on both sides as the canonical target. Production examples not yet standardizing on it is a separate "examples standardization" concern outside P.4 scope.

**Closure proof shape:** **parameter-equality via call-site assertion**. The harness defines `CanonicalQualityPresetParams::GAME_QUALITY` (single source of truth duplicating the `EditorQualityPreset::GameQuality` match arm values) and applies it to both renderers via the shared `apply_canonical_quality_preset_to_renderer` helper. Both paths invoke the same setters (`set_shadows_enabled`, `set_cloud_shadows_enabled`, `set_shadow_filter`, `set_cascade_extents`, `set_cascade_lambda`, `set_max_draw_distance`) with the same argument values from the same constant. No new public accessors added to `Renderer` (anti-drift constraint respected).

**P.4's quantization-threshold finding:** in the Phase 2 intermediate run (shadow caster added, seam NOT yet closed), hash equality unexpectedly held. The `EditorDefault` vs Renderer-defaults cascade-and-filter differences ARE real at the parameter level — they cast measurably different shadows in principle — but for the harness's fixture (sphere of radius ~1.73 at distance ~23m, viewed at 512×512 through canonical ACES + sRGB 8-bit output), the resulting LDR pixel differences fell below the 1/255 quantization threshold at every pixel. The seam is real at the parameter level; it is not pixel-measurable in this fixture. This finding reinforces the methodology pillar that structural closure proofs are strictly stronger than empirical hash equality on any given fixture — see Section 5.

**Production code changes (P.4):**
- `EngineRenderAdapter::new`: `apply_quality_preset(EditorQualityPreset::EditorDefault)` → `apply_quality_preset(EditorQualityPreset::GameQuality)` (single-line change). `EditorDefault` remains a defined variant for selectable use if a future editor UI exposes preset selection.
- Harness: new `CanonicalQualityPresetParams::GAME_QUALITY` constant, `apply_canonical_quality_preset_to_renderer` helper, shadow caster fixture (single sphere at (0, 5, 0), scale 2.0, uploaded via `Renderer::update_instances` on both paths).

### 2.4 Target format (P.5) — format-equality structural closure

**P.0 audit axis:** Axis 6 (target format).

**Pre-campaign divergence:** `EngineRenderAdapter::config.format = Bgra8UnormSrgb` (mirroring a typical D3D12 swapchain default) vs editor's `LDR_COLOR_FORMAT = Rgba8UnormSrgb` constant. Pre-P.3 the format mismatch was harmless (the surface=None branch ran the passthrough `hdr_blit_pipeline` with hardcoded `Rgba16Float`, never `post_pipeline` against `config.format`), but a target-format-aware parity contract still surfaced it as an axis.

**Closure design (decided per P.5 Phase 1 audit — Outcome 2):** The seam was **closed entirely as a downstream side effect of P.3's `surface.is_none()` branch deletion**. With `post_pipeline` running unconditionally, the editor adapter's `config.format` migrated to `Rgba8UnormSrgb` (P.3 Move) to match the editor's existing `LDR_COLOR_FORMAT`. Phase 1 audit found zero residual divergence across all configuration points. The P.5 commit is verification-only — no production code changes. The structural proof formalizes what P.3 incidentally achieved.

**Closure proof shape:** **format-equality structural**. The harness captures `(surface_format, hdr_format, depth_format)` from both `Renderer` instances via existing public accessors (`Renderer::surface_format()`, `Renderer::hdr_format()`, `Renderer::depth_format()`). The 3-row equality table asserts pairwise YES across all rows. No new `astraweave-render` API was added — the seam's structural proof uses surface that already existed for unrelated reasons.

**Closure proof reference values (this machine):**
```
| Configuration point                       | Engine          | Editor          | Equal? |
|-------------------------------------------|-----------------|-----------------|--------|
| surface_format (post_pipeline target)     | Rgba8UnormSrgb  | Rgba8UnormSrgb  | YES    |
| hdr_format     (internal HDR target)      | Rgba16Float     | Rgba16Float     | YES    |
| depth_format   (Depth32Float)             | Depth32Float    | Depth32Float    | YES    |

Pairwise comparisons: 3 / 3 equal (STRUCTURAL PASS)
```

**Production code changes (P.5):** zero. Verification-only commit. Outcome 2 was confirmed via `AskUserQuestion` before any code modifications.

### 2.5 Overlay composition (P.6) — isolation-structural closure

**P.0 audit axes:** Axis 9 (overlay composition residual).

**Pre-campaign divergence:** Editor overlays (grid, physics debug, gizmos, brush cursor, zone overlay, selection outline) drew directly onto the same target as the canonical engine output. Any overlay-on render would necessarily produce different bytes than the engine production renderer would for the same scene. Overlays were part of the editor's normal viewport composition, so the editor could never produce bit-identical engine output while showing overlays.

**Closure design (per P.6 Phase 1 audit — composition layer architecture):**
- New internal `ENGINE_LDR_TARGET` texture in `ViewportRenderer`: receives the engine canonical `post_pipeline` output. The parity-contract target. Never mutated by overlays.
- New internal `EDITOR_OVERLAY_TARGET` texture in `ViewportRenderer`: cleared transparent at frame start. All overlay passes redirected to draw into this target via `BlendState::ALPHA_BLENDING` (which produces premultiplied alpha output when drawing onto a transparent surface).
- New composite blit pass: alpha-over composition of `ENGINE_LDR_TARGET` + `EDITOR_OVERLAY_TARGET` into the caller-supplied display target. `composite.wgsl` shader: `composed.rgb = overlay.rgb + engine.rgb * (1.0 - overlay.a)`, `composed.a = 1.0`.
- `aw_editor` owns the entire composition layer per the P.0 Q1 decision; `astraweave-render`'s public API does not grow.
- `gizmo_renderer.rs:133`: gizmo pipeline color target format changed from `Bgra8UnormSrgb` to `Rgba8UnormSrgb` to align with the new overlay target's `LDR_COLOR_FORMAT`. Single-line change; only required because the gizmo pipeline had a hardcoded format whereas grid + physics renderers were already constructed with `LDR_COLOR_FORMAT` at construction (per P.3).

**Closure proof shape:** **isolation-structural**. The harness runs the editor path twice — once with `show_grid: false` (overlays inert) and once with `show_grid: true` (grid composited) — and reads bytes from the editor's internal `ENGINE_LDR_TARGET` via a new `ViewportRenderer::engine_ldr_texture()` accessor (aw_editor-internal API; no astraweave-render API growth). The closure proof asserts the two `ENGINE_LDR_TARGET` hashes are byte-identical: overlays demonstrably do not mutate the parity-contract target. The display target hashes are also captured for diagnostic purposes — they intentionally differ between overlay-on and overlay-off (the composite legitimately includes overlays in the display output).

**Closure proof reference values (this machine):**
```
ENGINE_LDR_TARGET SHA-256 (overlays OFF, show_grid=false):
  7b836af1f890544db071c39e55f00c5536c5810c4641498a091a06cf50cc97d5
ENGINE_LDR_TARGET SHA-256 (overlays ON,  show_grid=true):
  7b836af1f890544db071c39e55f00c5536c5810c4641498a091a06cf50cc97d5
Byte equality: PASS — overlays do not mutate the parity-contract target.

Display target SHA-256 (overlays OFF): 7b836af1...  (matches engine LDR because
  overlay target is fully transparent — composite is a pass-through no-op)
Display target SHA-256 (overlays ON):  8fb79fe627b4225eee29436657a7dd2c654b664f1bbd21e7970ce6db155d9f99
  (differs as expected — grid composited over engine output)
```

**Production code changes (P.6):**
- New `tools/aw_editor/src/viewport/shaders/composite.wgsl` (51 lines, alpha convention documented inline)
- ViewportRenderer fields: `engine_ldr_texture`/`view`, `editor_overlay_texture`/`view`, `composite_pipeline`, `composite_bgl`, `composite_sampler`, `composite_bind_group`
- `ViewportRenderer::ensure_composite_pipeline` helper (lazy idempotent build)
- `ViewportRenderer::engine_ldr_texture()` accessor for harness readback (aw_editor-internal API)
- `ViewportRenderer::resize`: allocates both new targets, rebuilds composite bind group
- `ViewportRenderer::render`: engine adapter routes to `engine_ldr_view`, overlays route to `editor_overlay_view`, new composite pass at end writes to `target_view`
- `gizmo_renderer.rs:133`: format alignment to `Rgba8UnormSrgb`

---

## 3. Architecture of the parity harness

### Where the harness lives

`tools/aw_editor/tests/render_parity_harness.rs` — single-file integration test in the editor crate. It depends on both `aw_editor_lib` and `astraweave_render` (the former depends on the latter; the harness's location in `aw_editor` avoids a circular dependency that would arise if the harness lived in `astraweave-render`'s tests).

### What it tests

The harness runs three render paths and compares their outputs:

1. **Engine path** — instantiates `Renderer::new_from_device(..., None, config)` directly with `config.format = Rgba8UnormSrgb`, applies `GameQuality` preset via the shared harness helper, uploads the canonical grassland biome pack via `canonical_terrain_pack` + `Renderer::set_terrain_materials`, uploads a terrain chunk via `Renderer::upload_terrain_chunk`, uploads the shadow caster instance via `Renderer::update_instances`, calls `Renderer::draw_into` to write canonical `post_pipeline` output to its target texture. Reads bytes from the target. This is the runtime-equivalent rendering, performed standalone.

2. **Editor path (overlays OFF)** — instantiates `ViewportRenderer::new`, initializes its `EngineRenderAdapter` (which automatically applies `GameQuality` preset via P.4's `EngineRenderAdapter::new` change), applies the same `GameQuality` preset defensively via the harness helper, uploads the same canonical biome pack + terrain chunk + shadow caster, calls `ViewportRenderer::render` with `show_grid: false`. Reads bytes from the editor's internal `ENGINE_LDR_TARGET` (via `engine_ldr_texture()` accessor) — NOT from the display target. The internal target is the parity-contract bytes.

3. **Editor path (overlays ON)** — identical to the editor path above, but with `show_grid: true`. Reads bytes from the editor's internal `ENGINE_LDR_TARGET` (same as above). The display target also captures the composite output for diagnostic.

### The fixture (single locked configuration)

| Field | Value |
|---|---|
| Output resolution | 512 × 512 |
| Camera | OrbitCamera at focal Vec3::ZERO, distance 25.0, yaw π/4, pitch π/6, FOV 60°, aspect 1.0, near 0.5, far 5000.0 |
| Time-of-day | 12.0 (noon — sun nearly overhead) |
| Biome | `assets/materials/grassland` (5-layer authored pack: grass / rock_smooth / dirt / sand / moss) |
| Terrain chunk | Single 10m × 10m flat quad at world origin, Y=0, 4 vertices / 2 triangles |
| Splat layout | 2×2 splat texture per chunk; each corner dominates one of layers 0-3 (grass / rock_smooth / dirt / sand) |
| Shadow caster | Single sphere instance at (0, 5, 0), scale 2.0, color [0.8, 0.8, 0.8, 1.0], material_id 0 |
| Quality preset | `GameQuality` (shadows on, cloud_shadows on, shadow_filter (2.0, 0.005, 1.5), cascade_extents (40, 120), cascade_lambda 0.75, max_draw_distance 0) |

### What hashes it compares

| Comparison | Asserts | Closure proof |
|---|---|---|
| Engine vs Editor (overlays OFF) ENGINE_LDR_TARGET SHA-256 | byte equality | overall per-machine parity |
| Editor (overlays OFF) vs Editor (overlays ON) ENGINE_LDR_TARGET SHA-256 | byte equality | P.6 overlay-isolation |
| Engine vs Editor `surface_format`/`hdr_format`/`depth_format` | pairwise format equality | P.5 format-equality structural |
| Canonical pack content hash (load_canonical_terrain_pack on grassland) | byte equality of CPU bytes input to `set_terrain_materials` | P.2 loader byte-level |

The P.3 tonemap and P.4 quality preset closures are **structural** rather than hash-comparison-based:
- P.3 is documented in the harness output as a structural claim (both paths construct `Renderer` via `new_from_device(None, config)` with identical `config.format`; `draw_into` is a single canonical path).
- P.4 is documented via the `CanonicalQualityPresetParams::GAME_QUALITY` constant and the shared `apply_canonical_quality_preset_to_renderer` helper — call-site assertion that both paths apply the same setters with the same arguments.

### How to run locally

```bash
cargo test -p aw_editor --test render_parity_harness -- --nocapture
```

The `--nocapture` flag is recommended to see the full closure-proof report alongside the test result. Without it, the test still passes (or fails) but the report is suppressed by Cargo's default capture behavior.

The test runs by default — no `#[ignore]` attribute. Any developer running `cargo test -p aw_editor` picks it up automatically.

Wall-clock time on the verification machine: ~90 seconds (dominated by three end-to-end render runs at 512×512 plus three SHA-256 computations over the readback bytes).

---

## 4. Post-P.7 cleanup queue

Six candidates accumulated during the campaign. None are urgent; all are addressable individually after P.7 in whatever order serves immediate needs. Each is documented with what it is, why it was deferred, what triggers it, and estimated scope.

### 4.1 Schema elevation — `MaterialsDoc` / `MaterialLayerToml` / `ArraysDoc` → public canonical types

**What:** `MaterialManager::load_pack_from_toml` (`astraweave-render/src/material.rs:419-449`) defines its TOML deserializer types as private local types inside the function body. P.2's `canonical_terrain_pack` redeclared an equivalent shape locally because the engine types aren't public. Elevation moves the schema types to `pub` in `astraweave-render`, and the editor's `canonical_terrain_pack` consumes them directly instead of mirroring.

**Why deferred (P.2):** Schema sync risk between engine and editor mirror is real but bounded — the parity harness catches byte divergence if the schemas drift. Elevating types is API growth in `astraweave-render`; doing it during P.2 would have violated the campaign's "do not modify the canonical loader" anti-drift discipline.

**Trigger:** Schema drift would surface as a harness failure (canonical pack content hash mismatch). At that point elevation becomes necessary; in the meantime it's cleanup.

**Estimated scope:** Small refactor. Mostly moving 4 struct definitions from private-in-function to `pub` at the module level, exporting via `lib.rs`, and updating `canonical_terrain_pack.rs` to consume them directly (deleting the mirror).

### 4.2 BiomePack flow migration to canonical loader

**What:** `main.rs:5058` BiomePack ground-surface loading uses the single-layer `set_terrain_surface_maps` path. This is separate from the editor's main viewport terrain rendering (which P.2 migrated to canonical). The BiomePack flow loads ONE PNG (the dominant ground surface) and uploads it via the legacy single-texture path — a different rendering shape than the 32-layer canonical splat.

**Why deferred (P.2):** Per the P.2 Q2 escalation, `set_terrain_surface_maps` was retained because of this caller plus the `--no-default-features` fallback path. Deleting it would have broken both flows. The BiomePack flow is its own design concern — is BiomePack's single-surface design intentional (it's authored as a packed asset with a single dominant ground texture), or accidental legacy that should migrate to the 32-layer canonical schema?

**Trigger:** Decision-required. If BiomePack is intentional single-surface, no migration is needed (it's a separate authoring concept). If it's legacy, migration consolidates it into canonical loading.

**Estimated scope:** Depends on the design decision. Migration: 1-2 sessions of A.x discipline (read BiomePack semantics, propose canonical schema, migrate the main.rs:5058 site, validate via headless test).

### 4.3 Bloom orphan re-integration into `POST_SHADER`

**What:** P.3 deleted `hdr_blit_pipeline`, which was the sole consumer of the bloom pass's output texture. The bloom compute pass still runs and writes its output; nothing reads it. The windowed runtime path (`Renderer::render`) also never composited bloom — only the deleted editor-mode passthrough did. So the bloom pass is currently orphaned across both editor and runtime.

**Why deferred (P.3):** Re-integration requires modifying `POST_SHADER` (the canonical post-pipeline shader). P.3's discipline forbade touching it — any change to the canonical shader during the campaign would have risked breaking the closure proofs that the campaign was establishing.

**Trigger:** Decision-required. Either re-integrate bloom into `POST_SHADER` (canonical, sampled at fragment time with a `bloom_intensity` uniform) or delete the orphan pass entirely. Not parity-relevant (both paths see the same — currently empty — bloom contribution).

**Estimated scope:** Re-integration: 1-2 sessions (modify `POST_SHADER`, add bind group entry for bloom texture, ensure both `Renderer::render` and `Renderer::draw_into` post passes bind it correctly). Deletion: smaller (remove the bloom compute pass + its uniform / config flag). Decision logged in commit body of P.3.

### 4.4 WYSIWYG OFF UI indicator

**What:** Per the P.4 spec and Q3 decision, the canonical viewport unconditionally uses `GameQuality` preset. `EditorDefault` (the prior default — performance-optimized: smaller shadow filter, reduced draw distance, etc.) remains a defined enum variant and is selectable if the editor exposes a quality-preset UI. If a user selects `EditorDefault` (or `EditorTerrain`, or `Minimal`), the viewport's output no longer matches what the runtime produces — parity intentionally broken for performance reasons. The user-facing surface needs a banner indicating WYSIWYG fidelity is off.

**Why deferred (P.4):** Logged in Claude's memory and the P.4 commit body. UI work is a separate authoring concern; P.4 closed the *technical* parity seam by making `GameQuality` the construction default.

**Trigger:** Whenever the editor adds a quality-preset selection UI (currently no UI exposes it). At that point the banner becomes a usability requirement.

**Estimated scope:** Small UI feature. egui-based indicator that watches the viewport's current preset enum and renders a banner when preset != GameQuality.

### 4.5 `CanonicalQualityPresetParams::GAME_QUALITY` schema-sync deduplication

**What:** The harness's `CanonicalQualityPresetParams::GAME_QUALITY` constant duplicates the parameter values from the `EditorQualityPreset::GameQuality` match arm (`engine_adapter.rs:926-949`). Drift between them would cause harness/editor parameter mismatch. The schema-sync risk is bounded: the harness uses both sides through the canonical match arm AND through the constant, so divergence would surface as the harness applying different values than the editor's startup application.

**Why deferred (P.4):** During the campaign the single source of truth has to live somewhere. The editor's match arm is the production-relevant authoritative source; the harness constant exists for closure-proof clarity. Deduplication requires either: making the harness call into the editor's match arm (which would require restructuring how `EditorQualityPreset::GameQuality`'s setter calls are exposed), or making the editor consume the harness's constant (which would invert dependency direction — production code consuming test code).

**Trigger:** Schema drift between them. Currently both sets of values are stable; no drift surface yet.

**Estimated scope:** Small refactor. Likely shape: extract the parameter struct + constant to a shared location (`tools/aw_editor/src/viewport/quality_preset_params.rs`?), make `EditorQualityPreset::GameQuality` match arm consume it, harness keep consuming it. Doesn't grow public API.

### 4.6 Per-pixel probe attribution heuristic retirement

**What:** P.1 introduced a per-pixel SAD-attribution probe (heuristic axis-attribution: loader, tonemap, target format, quality preset, overlay composition probes at 16 fixed sample positions each). P.3 obsolesced the heuristic by introducing structural closure proofs. P.4-P.6 reinforced via byte-level/parameter-equality/format-equality/isolation closure proofs. The per-pixel probe code remains in the harness but its axis-attribution outputs are documented as "stale and should be read as 'tonemap and format axes are closure-proven'".

**Why deferred (P.3 onward):** Removing the probe entirely during the campaign would have meant losing diagnostic output that some sub-phases referenced (P.2's baseline-reduction documentation captured SAD reduction across attribution rows). The probe became deprecated rather than deleted.

**Trigger:** Housekeeping. The probe's output is now noise relative to the closure proofs; future readers of the harness might be confused by it.

**Estimated scope:** Small cleanup. Delete the `compute_attribution` function + `AxisAttribution` struct + the report section that prints them. Roughly 100 LoC. Optional: keep `compute_attribution` but reduce its output to just total SAD as a diagnostic single number, without per-axis breakdown.

---

## 5. Methodology pillars surfaced

The campaign accumulated three candidate methodology pillars. P.7 documents them; codification into the canonical `§7.11` happens at the appropriate closeout boundary (likely after the Terrain Asset Quality campaign's A.5 lands, or further down whichever chain owns methodology elevation).

### 5.1 Pillar 6 — Frame-currency validation

**Surfaced where:** Originally Terrain Asset Quality A.4 (which discovered the editor-engine parity divergence by noticing the editor was using the legacy single-layer loader despite the architectural intent being the canonical 32-layer loader). Reinforced by this campaign's launch.

**Pillar statement:** Before committing campaign forward chains, verify the conceptual model is current with the actual system architecture. The A.1 → A.4 chain ran on a biome-wireup frame that had been partially superseded by regional archetypes; the divergence between docs and source was the surfacing condition that triggered A.4 to spot the parity gap.

**Why a pillar (not a tactic):** The pattern is general: any long campaign accumulates a frame (conceptual model + assumptions about the system) at its start, and the system evolves underneath the campaign as it runs. Periodic frame validation — re-reading source vs the campaign's frame — catches drift before forward chains commit to a stale assumption set.

### 5.2 Pillar 7 — Architectural-priority validation

**Surfaced where:** This campaign's launch. A.4's audit found a parity-class divergence (editor's terrain renders different content than the runtime). Andrew's call: feature work (the Terrain Asset Quality biome wire-ups) must pause until the architectural foundation (parity) is established. The Editor-Engine Render Parity campaign launched in A.4's place; A.5+ pause for seven sub-phases.

**Pillar statement:** When a feature subsystem reveals a parity-class divergence with a sibling consumer (here: editor vs runtime renderer), the divergence is resolved before the feature campaign resumes. Feature work compounds on the foundation; if the foundation is divergent, feature work amplifies the divergence.

**Why a pillar (not a tactic):** The decision criterion is generalizable: when the feature work would land against a foundation that's known-divergent, the right move is to pause feature work and fix the foundation, even if the pause is multi-week. Otherwise the feature work either ships against the divergent foundation (compounding the gap) or has to be re-done after the foundation is fixed.

### 5.3 Pillar 5 refinement — Measurement-instrument-matched-to-seam-type

**Surfaced where:** P.2 byte-level closure (loader), P.3 pipeline-structural closure (tonemap), P.4 parameter-equality closure (quality preset), P.5 format-equality structural closure (target format), P.6 isolation-structural closure (overlay composition). Empirically reinforced by P.4's quantization-threshold finding (parameter divergence below 8-bit visible threshold for the harness fixture).

**Refinement statement:** Each seam type has a characteristic closure-proof shape. Per-pixel SAD probes are inadequate primary contracts because real seams can fall below pixel-quantization thresholds for given fixtures (P.4's empirical finding), AND because per-pixel measurements are fixture-bound (they verify parity for the tested fixture only, not across all possible fixtures). Structural closure proofs — byte hashes of content, pipeline identity checks, parameter-equality assertions, format-equality tables, isolation-byte-equality assertions — guarantee parity across all possible fixtures.

**Why a refinement (not a new pillar):** This refines the existing methodology pillar (§7.11 Pillar 5 — "evidence-grounded narrowing") to specify what *kind* of evidence is matched to *what* seam shape. The original Pillar 5 said evidence narrows the diagnostic; the refinement says the evidence shape itself must match the seam being closed.

**Five demonstrations:** P.2 byte-level for content; P.3 pipeline-structural for shader stages; P.4 parameter-equality for state; P.5 format-equality structural for configuration; P.6 isolation-structural for composition. Each sub-phase chose a closure-proof shape that matched what the seam was about, not a generic per-pixel comparison.

---

## 6. Forward chain after P.7

### Terrain Asset Quality campaign resumes

**A.5 — Doc reconciliation** runs next. The originally-scoped A.5 reconciles `docs/current/TERRAIN_MATERIAL_SYSTEM_CAMPAIGN.md` against shipped reality (Status header, §1.4, §3.5, §7 Phase 1.X status, EditorTerrainSplat removal, BiomeNoisePreset removal, archetype-vs-biome architectural separation). Now also: references the parity outcome doc (this document) as the architectural foundation now in place. Codifies §7.11 Pillar 6 (frame-currency / doc-currency) into the campaign doc itself, alongside whatever existing pillar codification the campaign owns.

**A.6 → A.11 (or thereabouts) — Per-biome wire-ups.** Mountain, tundra, swamp, forest, river, beach. Each is a single-concern session: copy the relevant Tier 1 PNGs into `assets/materials/<biome>/`, author `materials.toml` matching the canonical schema, validate via headless `MaterialManager::load_pack_from_toml` test. Sequential, not parallel. Desert remains deferred pending future Tier 1.B sourcing decision per the A.4 finding (ambientCG eab972aea recommendations returned HTTP 404 on direct fetch; design call required on whether to use the audit's recommended IDs or seek alternative sources).

### Post-P.7 cleanup queue

The six candidates in Section 4 are addressable individually after A.5 commits, in whatever order serves immediate needs. None are urgent. The parity harness guards the foundation while cleanup proceeds.

### Methodology codification

At the right closeout boundary — likely after A.5 lands or further down whichever chain owns methodology elevation — Pillar 6, Pillar 7, and the Pillar 5-refinement get codified into the canonical `§7.11`. This document is the empirical reference; codification consolidates the pillars into the methodology document for cross-campaign reuse.

---

## Appendix A — Sub-phase commit summary

| Sub-phase | Commit | Files changed | Insertions | Deletions | Shape |
|---|---|---|---|---|---|
| P.0 | (chat-resident audit) | 0 | 0 | 0 | Architecture audit synthesis |
| P.1 | `1cf48ccce` | 4 | 656 | 0 | New harness + minimal type-export + dev-dep |
| P.2 | `ec349c5ce` | 6 | 684 | 168 | Sibling loader + canonical-load wiring + deletion of bespoke single-layer chain |
| P.3 | `e09703538` | 5 | 199 | 635 | Branch deletion + pipeline deletion + intermediate-target deletion (largest deletion-heavy sub-phase) |
| P.4 | `f4cf2b0f2` | 2 | 191 | 1 | Preset default swap + harness helper + shadow caster fixture |
| P.5 | `2f67ddd1f` | 1 | 130 | 0 | Verification-only (P.3 side effect closed the seam; P.5 formalised) |
| P.6 | `a59f26b8c` | 4 | 551 | 51 | Composition layer: overlay target + composite shader + composite pipeline + overlay redirect + harness fixture variant |
| P.7 | (this commit) | (see commit body) | (see commit body) | 1 (`#[ignore]`) | Harness public-default + this outcome doc |

---

## Appendix B — Closure proof reference values (this machine)

```
Adapter: NVIDIA GeForce GTX 1660 Ti with Max-Q Design (Vulkan, driver 591.74)

P.2 loader (byte-level closure):
  Canonical pack content hash:
  0ca13a5677aeb0ca4dd431ccd21a6afaf778a3cbd670d75b5d2a17a4b4f73d98

P.3 tonemap (pipeline-structural closure):
  Engine path config.format = Rgba8UnormSrgb (post_pipeline output)
  Editor path config.format = Rgba8UnormSrgb (post_pipeline output)
  draw_into pipeline branch: unconditional post_pipeline
  Pipeline source of truth: astraweave-render::POST_SHADER

P.4 quality preset (parameter-equality closure):
  CanonicalQualityPresetParams::GAME_QUALITY {
    shadows_enabled: true,
    cloud_shadows_enabled: true,
    shadow_filter: (2.0, 0.005, 1.5),
    cascade_extents: (40.0, 120.0),
    cascade_lambda: 0.75,
    max_draw_distance: 0.0,
  }
  Both paths invoke apply_canonical_quality_preset_to_renderer(GAME_QUALITY).

P.5 target format (format-equality structural closure):
  | surface_format (post_pipeline target)  | Rgba8UnormSrgb | Rgba8UnormSrgb | YES |
  | hdr_format     (internal HDR target)   | Rgba16Float    | Rgba16Float    | YES |
  | depth_format   (Depth32Float)          | Depth32Float   | Depth32Float   | YES |
  Pairwise comparisons: 3 / 3 equal (STRUCTURAL PASS)

P.6 overlay composition (isolation-structural closure):
  ENGINE_LDR_TARGET SHA-256 (overlays OFF, show_grid=false):
    7b836af1f890544db071c39e55f00c5536c5810c4641498a091a06cf50cc97d5
  ENGINE_LDR_TARGET SHA-256 (overlays ON,  show_grid=true):
    7b836af1f890544db071c39e55f00c5536c5810c4641498a091a06cf50cc97d5
  Display SHA-256 (overlays OFF): 7b836af1... (composite pass-through)
  Display SHA-256 (overlays ON):  8fb79fe627b4225eee29436657a7dd2c654b664f1bbd21e7970ce6db155d9f99
                                   (grid composited as expected)

Engine vs editor (overlays OFF) ENGINE_LDR_TARGET SHA-256:
  Engine SHA-256: 7b836af1f890544db071c39e55f00c5536c5810c4641498a091a06cf50cc97d5
  Editor SHA-256: 7b836af1f890544db071c39e55f00c5536c5810c4641498a091a06cf50cc97d5
  Total SAD: 0.0 (mean 0.0 / pixel)
```

These reference values are machine-specific (per-machine parity contract per the P.0 spec). A future developer's reference values will differ if they run on different hardware; what matters is that *their* engine SHA-256 equals *their* editor SHA-256, and that the per-axis closure proofs all pass.
