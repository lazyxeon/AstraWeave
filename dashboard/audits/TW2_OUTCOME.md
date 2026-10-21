# T.W.2 — Paintable Water: Carve Brush + Water-Volume Entities (outcome)

> **Beat:** T.W.2 (terrain series, closes the water arc) · **Date:** 2026-07-23/24
> **Commits:** code `1e3d8f692` (19 files, +1434/−131) · docs (this note + trace bumps) — hash in the session report.
> Evidence tiers: **built** / **run** / **verified**. Session screenshots in `d:/tmp/tw2_staging/` (local, not committed; frame `NN_*.png` cited below). Executes the ratified §3.5 order (`TW1_RATIFICATION_2026-07-21.md` §2 row 3): carve-brush first, volume entities for perched water, mask-paint deferred.

## 0. Summary

Water is now paintable two honest ways, each riding a system that already exists — no second water authority, no reclassification, no new persistence plumbing for terrain. **Carve Water** is a sculpt-family brush that "paints water" by lowering land toward a depth below the world level; the existing global plane floods the result frame-by-frame via depth occlusion, and the carved bed stays terrestrial-classified (the ratified emergent-dip *feature*). **WaterVolume** is an editor entity carrying a bounded water body (a mountain lake / oasis) that renders through the existing water pipeline as a bounded patch and persists via the entity save/load path. Both were verified live: a carved coastal pond and a drag-carved channel fill and undo/redo exactly; a Boreal mountain lake renders at altitude and is still there after a scene reload. The persistence work also **fixed a pre-existing bug** — Light/Camera entity components were silently dropped on every save/reload — and closed a missed-registration-surface trap the beat surfaced live (a Ctrl+S hotkey handler still using the overlay-dropping save).

## 1. Concern 1 — the Carve Water brush

### 1.1 What shipped (integration citations)

- **`BrushMode::CarveWater`** ([terrain_panel.rs:331](../../tools/aw_editor/src/panels/terrain_panel.rs)) added to the existing sculpt brush family — `name`/`icon`/`all()` all touched (the enum's registration surfaces). Selectable in the Mode row alongside Sculpt/Lower/…; a **Carve Depth** slider (0.5–10 m, default 3.0) appears only in this mode, plus a "💧 Water is disabled — carves won't flood…" affordance when the water checkbox is off.
- **Brush math** ([terrain_integration.rs `apply_brush`](../../tools/aw_editor/src/terrain_integration.rs)): the shared `apply_brush`'s `flatten_target` argument was generalized to `target_height`; the CarveWater arm blends toward `water_level − carve_depth` **only downward** — `if current_h <= target { current_h } else { blend }`. So repeated strokes converge on the target and never over-deepen (idempotent at depth), and the falloff makes the rim descend partially → a shore, not a cliff. The panel computes the target per-application via `brush_target_height()` so it tracks the live level slider.
- **Undo**: carve reuses the *identical* machinery as sculpt — the same per-chunk pre-stroke snapshots (`stroke_pre_snapshots`), the same `end_brush_stroke` → `TerrainBrushCommand` → `undo_stack.push_executed` path ([main.rs:4081-4090](../../tools/aw_editor/src/main.rs)). No carve-specific undo code exists; it was only a new match arm.
- **No water config touched**: the carve is a pure terrain operation. It does not read or write the water enable, census, or level — one law (elevation vs the single world level), no second authority.

### 1.2 Tests (run + verified)

`cargo test -p aw_editor --lib tw2` → **6 passed** (4,024 filtered) — the 4 carve tests + 2 persistence tests:
- `tw2_carve_water_clamps_at_target_and_is_idempotent` — 200 full-strength applications converge to the target; 50 more never undershoot.
- `tw2_carve_water_falloff_makes_a_shore_not_a_cliff` — after a finite stroke, rim samples sit strictly above the bed (gradual shore); terrain beyond the radius is untouched.
- `tw2_carve_water_undo_round_trip` — the stroke records per-chunk (pre, post) deltas; applying pre restores the original height exactly; applying post redoes it.
- `tw2_carve_water_never_raises_ground` — with a target *above* some terrain (where Flatten would raise it), carve never lifts any footprint sample.

### 1.3 Live observation (run + verified; `d:/tmp/tw2_staging/`)

Mediterranean seed 12345 r6:
- **Pond floods** (`18`, `20`): a viewport-click carve on dry grassland at world (74, 611) produced a teal water pond immediately — the plane flooding the carved bowl. A cluster of carves widened it into an irregular pond.
- **Channel floods** (`27`): a *drag*-carve traced a winding water channel across the terrain — a river of water carved into the land. Status bar: **"Undo (1): Terrain Carve Water"**.
- **Undo/redo reverts exactly** (`28`): Ctrl+Z removed the channel, terrain restored to smooth grass, **"Redo (1): Terrain Carve Water"**. (Note: the undo entry registers on the drag-*release* gesture — `response.drag_stopped_by(Primary)` at [widget.rs:1490](../../tools/aw_editor/src/viewport/widget.rs); a stationary synthetic click applies the brush but does not register as a drag, so it never pushes undo. This affects **all** brushes equally — not carve-specific — and a human painting always drags.)
- **Tooltip confirms the semantics** (`19`): the Carve Depth hover reads *"How far below the water level the carved bed settles. The brush lowers ground toward (water level − depth) and never deepens ground already below it."*

## 2. Concern 2 — WaterVolume entities

### 2.1 What shipped (integration citations)

- **Render** ([water.rs](../../astraweave-render/src/water.rs)): `WaterVolumeDesc` (center, half-extents, surface_level, colors, wave params) + a `VolumeGpu` per volume (own 512-B uniform block + bind group + patch mesh + instance). `WaterRenderer::set_water_volumes` (change-detected; empty clears), `generate_volume_patch` (flat rect grid, upward-wound, same family as the plane tiles). Volumes render through the **same** pipeline/shader as the global plane — `render()` draws them after the plane with each volume's uniform block (level/colors/wave override, `weave_count = 0` so an authored lake never inherits a sea-aimed Part/Raise/Freeze), depth-occluded by terrain within the bounds. `plane_visible` gate + `has_renderable_content()` (dormant = no plane chunks AND no volumes) so a Desert-oasis volume renders with the global checkbox off. `prepare_scene` re-points volume bind groups + refreshes their uniforms each frame.
- **Renderer/adapter plumbing**: `Renderer::set_water_volumes` / `set_water_plane_visible` / `has_water_renderer` ([renderer.rs](../../astraweave-render/src/renderer.rs)); the editor `EngineRenderAdapter::set_water_volumes` resolves `WaterVolumeSpec` → `WaterVolumeDesc` via `water_style_params` (the shared plane+volume style resolver, factored out of T.W.1.A), installing a plane-hidden renderer when volumes exist without the plane. Viewport `renderer.rs` + `widget.rs` collect volumes from the `EntityManager` (same pattern as lights).
- **Entity** ([main.rs](../../tools/aw_editor/src/main.rs), [tab_viewer/mod.rs](../../tools/aw_editor/src/tab_viewer/mod.rs), [inspectors.rs](../../tools/aw_editor/src/tab_viewer/inspectors.rs)): `WaterVolume` archetype (SpawnArchetype + AddComponent default), "**+ Add → Environment → Water Volume**" menu entry, a `show_water_volume_inspector` (XZ half-extents + `WaterStyle` combo; the entity's Y **is** the surface level — edit via Transform/gizmo), and the `💧 Water Volume` inspector header. All registration surfaces touched (spawn arm, add-menu, add-component list, inspector dispatch + header).
- **Persistence** ([scene_serialization.rs](../../tools/aw_editor/src/scene_serialization.rs)): `EntityData` gained a `components: HashMap<String, Value>` (serde `default` + `skip_serializing_if` empty — **pre-TW2 files parse unchanged**). `save_scene_with_overlay` / `load_scene_with_overlay` carry the `EntityManager` overlay's typed payloads (Light/Camera/WaterVolume/…) through the file; the loader re-applies them onto the rebuilt `EntityManager`.

### 2.2 The pre-existing bug this beat found and fixed

Building volume persistence surfaced that **typed entity components were silently dropped on every save/reload** — a saved Light lost its light data, and WaterVolume would have too. The plain `save_scene(world)` only serialized ECS/world data, never the `EntityManager` overlay where Light/Camera/WaterVolume payloads live. The fix (`save_scene_with_overlay` / `load_scene_with_overlay`) was applied to **all** save/load surfaces — but the live editor test caught a **missed registration surface**: the `Ctrl+S` and `Ctrl+Shift+S` hotkey handlers ([hotkeys.rs](../../tools/aw_editor/src/subsystems/hotkeys.rs)) still called the overlay-dropping `save_scene` (the menu `on_save_scene` had been updated; the hotkey had not). The first live save wrote a file with no components; the fix wired the hotkeys and was **re-verified live** (§2.4). The two secondary load paths (drag-drop `.ron`, asset-browser) — which never even synced the `EntityManager` pre-TW2 — were consolidated onto the canonical overlay-aware `load_scene_from_path`.

### 2.3 Tests (run + verified)

`cargo test -p astraweave-render --lib water` → **23 passed** (1,263 filtered) — T.W.1.A's 22 + `test_volume_patch_geometry`, plus volume assertions folded into the on-GPU test (`volume_count`, `plane_visible`, `has_renderable_content` gating, change-detect no-op vs rebuild-invalidation, volumes-only renderable). `cargo test -p aw_editor --lib scene_serialization` → **30 passed** — incl. `tw2_component_overlay_round_trip` (ron round-trip + pre-TW2 back-compat) and `tw2_water_volume_persists_through_file` (the full `save_scene_with_overlay` → file → `load_scene_with_overlay` production path, asserting half-extents + style + surface-Y survive).

### 2.4 Live observation (run + verified; `d:/tmp/tw2_staging/`)

- **Bounded lake renders** (`31`, Mediterranean): the WaterVolume renders as a calm, bounded teal patch with a foam shoreline edge, at the entity's Y (≈37, above sea level), depth-occluded by terrain — a discrete rectangle, not the infinite plane. Inspector confirms the `WaterVolume` component.
- **Boreal mountain lake** (`39`): on snowy Boreal mountain terrain, the volume reads as a calm dark lake at altitude (Y≈91), the snow contour occluding its upper edge — the target picture. Calm Lake style (no whitecaps), bounded.
- **Save→reload persistence** (`40`–`42`): Ctrl+S wrote the scene; the on-disk file (verified by `cat`) contains `components: { "WaterVolume": { half_extent_x: 24.0, half_extent_z: 24.0, style: "Lake" } }`; File → Recent Files → reload restored the entity and **the mountain lake rendered identically** ("Loaded scene: untitled.scene.ron"). The volume was reconstructed from the file, not memory — persistence proven end-to-end.

### 2.5 Budget (work item 3; run + verified)

`water_budget_probe` extended with a "near + 4 volume patches" pass (min-spec **GTX 1660 Ti Max-Q**, Vulkan, timestamps, medians/300 frames): near **0.2519 ms**, horizon **0.1721 ms**, **near + 4 volumes 0.2606 ms** — the ~0.26 ms class, within the 2.0 ms envelope. (An earlier run mis-measured volumes because their uniform blocks weren't refreshed per-frame — fixed in `update`; the cited numbers are post-fix.)

## 3. Verification ladder

1. Suites: render water **23 passed**; aw_editor tw2 **6 passed**; scene_serialization **30 passed**; terrain_panel **58 passed**; mutation_resistant_terrain **70 passed** (brush count updated 8→9 for CarveWater — enumeration registration surface); mutation_resistant_comprehensive_v2 **268 passed**; wave2_plugin_scene_tests **105 passed** (the two `EntityData` literal test files updated for the new `components` field). `cargo check --workspace` **clean**; clippy **clean** on every touched file.
2. Budget probe before/after (§2.5).
3. Live observation matrix (§1.3, §2.4), including the save→reload persistence round-trip and the hotkey-fix re-verification.

## 4. Director gate repro (the payoff of the water arc)

`cargo editor` → Terrain panel → generate a world (Mediterranean or Boreal), seed 12345, radius 6:
1. **Carve a pond, watch it fill**: Sculpting Brushes → activate → **Carve Water** → drag on dry land → a flooded pond appears. Drag a **channel** toward existing water → it fills continuously. **Ctrl+Z** reverts the stroke exactly; **Ctrl+Y/Shift+Z** redoes.
2. **Place a mountain lake and reload**: + Add → Environment → **Water Volume** → it renders as a calm bounded lake at the entity's Y (raise the entity in Transform to sit it at altitude); adjust extents/style in the Inspector. **Ctrl+S**, then File → Recent Files → the scene → the lake is **still there**.
Render verdict + feel verdict (carve ergonomics: size/strength/falloff/depth defaults; lake calmness) are the director's. That closes the water arc and T.W.2.

## 5. Residue / open items

- **Undo needs a drag gesture** (§1.3): a single stationary click applies the brush but doesn't push undo (egui `drag_stopped_by` only fires on drag-release). This is pre-existing brush behavior, not carve-specific; a human always drags. Noted for a possible click-also-registers-undo polish.
- **Terrain edits stay session-transient** (ratified — M3's concern): carves are lost on regenerate/reload. Only *entities* (WaterVolume) persist. Unchanged by this beat.
- **Style defaults** (Ocean 0.55/0.75, Lake 0.12/off, …) carried from T.W.1.A; volumes default to Lake. First-pass tuning; adjustable at the re-gate.
- The T.W.1.A submerged-terrain splat-refraction visual (checkered pattern through clear shallow water) is unchanged — a terrain-material concern, not water-owned.
- No mask-paint, no per-body plane levels, no hydrology/flow — all ruled out or deferred per the ratification.
