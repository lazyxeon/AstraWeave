---
schema_version: 1
trace_id: aw_editor
title: "aw_editor (Visual Editor)"
description: "aw_editor (Visual Editor)"
primary_crate: aw_editor
domain: tools
lifecycle_status: active
integration_status: wired
owns: [aw_editor]
doc_version: "1.20"
last_verified_commit: 154d723aa
---

# Architecture Trace: aw_editor (Visual Editor)

## Metadata

| Field | Value |
|---|---|
| **System name** | aw_editor (AstraWeave visual editor / level + asset + tool authoring app) |
| **Primary crates** | `tools/aw_editor/` (sole primary crate; two binaries: `aw_editor` GUI + `aw_game_runtime` standalone game runner — both windowed via winit+wgpu, see §6 trap notes) |
| **Document version** | 1.20 |
| **Last verified against commit** | `154d723aa` (v1.16, 2026-08-02: L.3.C-resolution — L-series CSM harnesses + `tests/common` relief module; trace updated in the same commit as the code, so this names its parent); prior L.2-calibration working tree over `5b78fdd78` (v1.13 pass); prior `b1295b93f` (v1.12); prior L.1 `1d8e1c792` (v1.11); prior ED-3 `09ea417af` (v1.10), prior `03f9d9a2d` (v1.9), prior `1e3d8f692` (v1.7), `8232b150b` (v1.5). Note: v1.8 (ED-2) bumped the front-matter `doc_version` only and left this table at 1.7 — corrected here. |
| **Last verified date** | 2026-07-29 (v1.15, L.3.A — the camera-anchored shadow boundary, editor side: engine-only fix (render trace v1.18 — third survey cascade to 3000 m); the editor surface is the new **camera-MOTION verification harness** `tests/l3a_proof.rs` (radial shadow-effect profile at two survey altitudes with flat-plane row→distance mapping + stated dune-height error bars; a 600 m lateral sweep at the rejection recording's Y≈219 with two world-anchored shadow features tracked by reprojection — the assertions that camera position must not change a world feature's shadow state; `L3A_EXPECT=fixed` arms them). Close/mid station frames byte-match L.3 (c0/c1 pinned); y414 shows relief shadowed; off-toggle bit-exact. The verification bar minted: camera-fitted systems get motion legs, never static stations alone. Evidence: `docs/audits/L3A_OUTCOME.md`. Prior v1.14, 2026-07-28 (L.3 — terrain CSM cast+receive, editor side: (1) **the editor terrain scene now renders terrain shadows** — engine-side caster+receiver (render trace v1.17, invariant 20); the adapter's involvement is configuration truth: at defaults the scene's shadow state comes from **`GameQuality` applied at adapter init** (`apply_quality_preset(GameQuality)` in `new`) because the terrain-upload block's auto-switch to `EditorTerrain` is gated on `quality_preset != GameQuality` — i.e. the `EditorTerrain` preset does NOT apply on the default path (proven: two harness legs differing only in the preset's `set_shadow_filter` bias were bit-identical across 12 frames). The upload block's own `set_shadow_filter(1.5, 0.0005, 1.0)` is the delivered filter config (receiver bias 0.0005; the 3rd arg slope-scale is write-only — never uploaded, logged residue). (2) The `EditorTerrain` preset itself now enables shadows with explicit filter values (the ratified T2F §5 A(iii): it must not silently kill terrain shadows when a user selects it) and its variant/docs state the truth. (3) The upload block's `set_cascade_extents(80, 250)` is a **no-op** (fields never read — sphere fitting superseded them); `set_cascade_lambda(0.7)` IS live → split0 ≈ 86.1 m in the terrain scene. (4) New `tests/l3_proof.rs` (cloned from l2_proof): 4 pinned stations × default-sun (43.2°, the panel-pinned override — `set_time_of_day` is INERT in terrain scenes, proven byte-identical, so the raking-sun leg drives `set_lighting_params` varying only `sun_dir` to elevation 20°/same azimuth 51.3°) + rake + ED-3 normals mask + `L3_SHADOWS=off` sentinel toggle leg + per-cascade caster culling stats + 2-station perf. Toggle proof: off-state vs pre-L.3 = 0 differing pixels (desert_close + all normals; ≤3 px @ 1 LSB elsewhere). Evidence: `docs/audits/L3_OUTCOME.md`. Prior v1.13, L.2 calibration — director-ratified A+B+E shipped: (1) **the init IBL bake runs on a WORKER THREAD** (`spawn_default_environment_bake` at adapter construction: a second `IblManager` on the `l2-env-bake` thread bakes the tracked kloppenheim HDRI and channels `IblResources` back; `poll_env_bake` — called per frame from `render_to_texture` — installs via `Renderer::install_baked_environment`; the UI thread never blocks on the ~2.5 s bake; frames before arrival use the fallback sky-fill env, a one-time startup transition; `wait_env_bake(timeout)` gives harnesses a deterministic post-bake state — l1_proof/l2_proof call it). (2) Engine-side calibration: bake-cube sun-clamp + fixed intensity 1.5 (render trace v1.16). (3) Final A/B at shipped calibration: desert +33 / grass +22 mean luma vs L.1 baseline, sd 9.9–15.6, glitter-free; L.1 controls re-certified (defaults-push 0 differing px; exposure 79/146/197; late-push landed 50.19 vs ~146 dropped, thr 140). Evidence: `docs/audits/L2_OUTCOME.md` §0.1. Prior v1.12, 2026-07-27 (L.2 — IBL for editor terrain: (1) **the editor bakes an environment at adapter init** — `EngineRenderAdapter::new` → `bake_default_environment()`: tracked `assets/hdri/polyhaven/kloppenheim_02_puresky_2k.hdr` (catalog `kloppenheim_daytime`, CC0, present in a bare clone), tier Medium (the load_hdri/clear_hdri convention), probed from `assets`/`../../assets`, absence/failure → `tracing::warn!` + continue on the fallback bind groups (2.48 s cold, once per adapter). (2) Terrain pixels are environment-lit via the engine's shared `compute_ibl` at group(3) — the World-panel ambient controls are relabelled **"(statics)"** with hover text (honest repurpose per the L.2 binding condition: they still drive the static-mesh ambient floor; terrain no longer reads them). (3) `l1_proof.rs` late-push dark params recalibrated for the IBL fill (dark push now also carries exposure 0.5; threshold 140; landed 92.31 vs ~178 dropped). (4) New `tests/l2_proof.rs` A/B harness (4 pinned stations × lit+ED-3-normals; the normals pair doubles as the sky/ground attribution mask — byte-identical at all stations, zero sky pixels in the pinned framings). (5) **Default calibration STOPPED pending ratification** — the delivered default over-drives IBL ~2.4–2.5× with irradiance fireflies; measured diagnosis + previewed remedies in `docs/audits/L2_OUTCOME.md` §4–5. Prior v1.11, 2026-07-26 (L.1 — lighting calibration & honesty, all visually neutral at defaults, proven 0 differing pixels at 4 pinned stations: (1) **the frame-1 lighting-push race is FIXED** — `ViewportRenderer::set_lighting_params` PARKS a push arriving before the async engine adapter exists (`pending_lighting_params`) and `init_engine_adapter` delivers it, so a push is never silently dropped and the caller's change-cache in `main.rs` stays truthful (pre-L.1 the World panel's frame-1 push was dropped + cached as delivered, leaving the panel displaying sun 2.2x while the renderer delivered 1.0x until a slider touch jumped the scene +42%; T2F_LIGHTING_RECON.md par.3). (2) **The World panel lighting defaults are reconciled to the DELIVERED state** via pinned constants in `viewport/types.rs` (`DEFAULT_SUN_*`/`DEFAULT_AMBIENT_*`/`DEFAULT_EXPOSURE`, single source for the panel struct defaults, `TerrainLightingParams::default()`, and the engine adapter's terrain-upload block; bit-identity of the pinned sun direction with the upload hardcode is asserted by test). (3) **The Exposure slider is LIVE end-to-end** — `TerrainLightingParams.exposure` -> `EngineRenderAdapter::set_lighting_params` -> `SceneEnvironment.exposure` -> the scene-env UBO pad float at offset 72 -> `uPostScene.exposure` in the POST shaders (default 1.35 = the former hardcode). (4) **Preset fictions deleted**: EditorTerrain/EditorDefault no longer claim SSAO, GameQuality no longer claims TAA — no such passes exist in this render path (only `bloom_enabled` is consumed); flags zeroed, enum/setter docs rewritten, dormant machinery untouched. Proof: `tests/l1_proof.rs` (neutrality: 0 differing pixels before/after at 4 stations; exposure 0.5/1.35/3.0 pairwise-different, byte-identical at the default; pre-init push lands after adapter init — both control tests FAIL at the pre-L.1 commit, demonstrated). Evidence: `docs/audits/L1_OUTCOME.md`. Prior v1.10, 2026-07-26 (ED-3: **all three settable-but-never-read surfaces CLOSED - every control now works or is hidden.** (1) The viewport shading dropdown is LIVE: `ShadingMode` grew `Normals`/`Uvs` (5 modes), `ViewportRenderer::render`'s formerly-dead `_shading_mode` param is consumed and forwarded via `EngineRenderAdapter::set_debug_shading` into the scene-env UBO's new `debug_mode` (a commandeered pad float, 96-B layout unchanged) plus a wireframe pipeline swap; all three live fragment shaders branch on it uniformly (no second render path). (2) The docked five-entry dropdown maps 1:1 onto the same enum - the silent Normals/UVs-to-Lit collapse is deleted (`main.rs` handler). Wireframe ships as polygon-mode-Line variants of the static-mesh + terrain-forward pipelines, feature-gated: `toolbar.wireframe_supported` (synced per frame from the renderer) hides the entry on devices without `POLYGON_MODE_LINE`. (3) The Terrain panel's dead absolute Amplitude slider is replaced by **"Amplitude x (spline scale)"** - `TerrainPanel::amplitude_scale` -> `TerrainState::set_amplitude_scale` -> `NoiseConfig::base_amplitude_scale` (default 1.0 = bit-identical; archetype splines stay the authority) - which makes T.3's amplitude-finality gate actionable. Proof: `tests/ed3_proof.rs` (2 GPU tests - five modes render 99.8-100% different from Lit; three amplitude scales differ pairwise ~99.5%; on pre-ED-3 code both collapse to byte-identical frames, the defect itself). Evidence: `docs/audits/ED3_OUTCOME.md`. Prior v1.9, 2026-07-25 (T.2d third pass `03f9d9a2d`: **three settable-but-never-read surfaces recorded in §11**, no code change. (1) The viewport shading dropdown is inert — `ShadingMode` is converted at `viewport/widget.rs:880` and handed to `ViewportRenderer::render`, whose parameter is `_shading_mode: u32` (`viewport/renderer.rs:570`) and is unused; `has_lighting()`/`is_wireframe()` have zero non-test callers. (2) The docked Viewport panel advertises five modes (`tab_viewer/mod.rs:2082`) of which **Normals** and **UVs** are silently mapped to `Lit` (`main.rs:5208-5214`) — debug visualisations promised but never built, which is exactly what a renderer diagnosis reaches for first. (3) The Terrain panel's *Base Amplitude* slider is inert (trace: `docs/architecture/terrain.md` §11), proven by `t2d_camera_light.rs` Experiment F rendering three amplitudes byte-identical. Evidence: `docs/audits/T2D_CAMERA_LIGHT.md` §10. Prior v1.8, ED-2 `03f9d9a2d`: camera pin/restore + viewport capture, Invariants 30/31. Prior v1.7, 2026-07-24 (v1.7, T.W.2 `1e3d8f692`: **paintable water**. (1) **Carve Water brush** — `BrushMode::CarveWater` in the sculpt family (`terrain_panel.rs`; `name`/`icon`/`all()` touched); the shared `apply_brush`'s `flatten_target` arg generalized to `target_height`, its CarveWater arm blends terrain toward `water_level − carve_depth` DOWNWARD-ONLY (idempotent at depth, falloff→shore); the panel's `brush_target_height()` recomputes it per-application; carve-depth slider + water-off affordance; reuses the sculpt undo path verbatim (`end_brush_stroke`→`TerrainBrushCommand`, no carve-specific undo). A pure terrain op — touches no water config. (2) **WaterVolume entity** — `WaterVolume` archetype + `+ Add → Environment → Water Volume` + `show_water_volume_inspector` (extents + `WaterStyle`; entity Y = surface level) + AddComponent default (all registration surfaces touched); `viewport/widget.rs` collects `WaterVolumeSpec` from the `EntityManager` like lights, `viewport/renderer.rs`→`EngineRenderAdapter::set_water_volumes` resolves style via `water_style_params` (factored, shared plane+volume) and forwards to `Renderer::set_water_volumes`. (3) **Component persistence** — `EntityData` gained `components: HashMap` (serde default + skip-if-empty; pre-TW2 files parse); `save_scene_with_overlay`/`load_scene_with_overlay` carry the `EntityManager` overlay through the file, **fixing a pre-existing bug where Light/Camera components were dropped on every save/reload**. All save surfaces updated incl. the `Ctrl+S`/`Ctrl+Shift+S` hotkeys (`subsystems/hotkeys.rs` — a missed registration surface caught live); the drag-drop `.ron` + asset-browser load paths consolidated onto `load_scene_from_path`. New Invariant 29. Prior v1.6, T.W.1 `88c6ba588`: **editor water surfaced** — `set_water_enabled` format fix (`surface_format()`→`hdr_format()`; the old argument panicked wgpu against the Rgba16Float water pass, `TWR_WATER_RECON.md` §1.6) + W.2c.2 scaffolding-weave deletion (`engine_adapter.rs`); legacy `primary_biome` water gate in `main.rs` replaced by the aquatic-biome census default (`TerrainState::has_aquatic_biomes`) + Terrain-panel Water checkbox/level slider (level defaults `astraweave_terrain::SEA_LEVEL`; was a frozen 0.0 field with no UI); per-archetype `WaterStyle` map (`water_style_for_archetype`, coast→Ocean, Boreal/Desert→Lake) threaded through `widget`/`viewport::renderer` `set_water_enabled(enabled, style)`; ratified hygiene deletions — dead SPH-era panel fluid block (`FluidSimParams`/`WaterBodyPreset`/`FluidQualityPreset`/`DetectedWaterBodyInfo`/`FluidStats` + ~200 tested-but-uncalled tests) and zero-caller `apply_brush_paint`+`id_to_biome` (`chunk.biome_map_mut` retains its generation caller). New Invariant 28. Prior v1.5, T.0 trace-sync: **multi-biome terrain render integration — the E3 build's editor side** (`d506658d8`, 2026-07-03; landed BEFORE the v1.3/v1.4 passes but never traced — reconstruction at `docs/audits/E3_PREFLIGHT_2026-07.md`). `generate_terrain` threads per-vertex `chunk.biome_ids()` into `generate_heightmap_mesh` (`terrain_integration.rs:375-388`, param `:766`); `build_biome_slot_field` (`:1293-1370`) maps each vertex's Whittaker `BiomeId` through the exhaustive 19→8 `biome_id_to_slot` (`:1250-1274`), adds a slope-rock overlay toward slot 3 (consts `:1304-1306`) and a 1-ring box blur, and the result drives per-vertex `material_ids`/`material_weights` into `TerrainVertex` (`:837-840`, `:861-867`); the legacy single-`primary_biome` splat generator survives only as the fallback when biome data is absent (`:797-802`, `:841-852`). Biome-driven assignment emits pack-layer indices DIRECTLY (arrays.toml order matches `biome_id_to_slot` 1:1) and therefore bypasses the palette remap — the remap governs manual painting only (the two paths coexist; see Invariant 27). Also records the four `E3-terrain (temporary)` judging aids as REVERT-BEFORE-v1 debt owned by terrain beat T.3 (see §6). Prior v1.4 2026-07-15: AD.5.A Fix 3 — paint-palette→pack-layer name remap (AD.4.A §5a Option C, director-ratified). New `viewport/palette_remap.rs` (`PaletteRemap::resolve` joins `MATERIAL_NAMES` stems against the loaded pack's per-layer albedo stems; placeholder set gets identity 0..8); `CanonicalLayerBytes.albedo_stem` retained at load as the join key; `EngineRenderAdapter` re-resolves on every terrain-layer upload and exposes `palette_remap()`; `EditorApp::update` syncs it per frame (try_lock) into the live `dock_tab_viewer.terrain_panel`; both paint call sites write the REMAPPED layer index (raw-palette-id writes removed) and unresolved entries render disabled with tooltip. Prior pass 2026-07-02 v1.3: AstraWeave branded theme as default — palette single-source `ui/palette.rs`, dock chrome unified via `DockLayout::themed_style`, window icon + top-bar brand mark `ui/branding.rs`, splash video re-enabled with first-frame gate reverting the functional half of `1fc266c93`. Feedback pass 2026-07-02: palette re-tinted to neon sky-blue accent over black metallic-flake surfaces; `world_wizard.rs` theme-routed and made a true modal (Foreground order, input-swallowing dim, one-shot raise above HUD overlays — re-raising per frame would cover its own ComboBox popups); dark branded Win11 title bar via `DWMWA_CAPTION_COLOR`/`DWMWA_TEXT_COLOR` in the DWM composition helper. Prior full pass 2026-05-12 v1.2) |
| **Status** | Active (mid-campaign — see Owner notes) |
| **Owner notes** | The editor is by far the largest workspace crate: 216 `.rs` files, ~224,584 LoC (including tests), 49 files in `panels/` (48 panel files + `mod.rs`), 41 `PanelType` enum variants (verified 2026-05-12 by reading `panel_type.rs:107-233`), 123 fields on the `EditorApp` god struct (verified 2026-05-12), 18 direct `astraweave-*` path dependencies (verified 2026-05-12 — earlier "24+" claim was wrong), 11 feature flags (`editor-core`, `editor-graphs`, `editor-materials`, `editor-terrain`, `editor-nav`, `editor-sim`, `editor-full`, `profiling`, `fast-alloc`, `terrain-splat-arrays`, `impostor-bake` — earlier "8 features" claim was wrong, list was always correct). Plus 20 `.md` files inside `tools/aw_editor/` itself and 19 `EDITOR_*.md` / `AW_EDITOR_*.md` files under `docs/current/` (verified counts 2026-05-12; earlier figures were estimates). It is mid-campaign on multiple architectural fronts as of `a2474c5b7`. Three load-bearing reference documents already exist that this trace builds on rather than duplicates: `docs/current/EDITOR_BEHAVIORAL_CORRECTNESS_AUDIT.md` (37-fix audit, 14 critical + 18 high findings, 2026-04-04), `docs/current/FIX27_UNIFIED_PIPELINE_CAMPAIGN.md` (7-phase eliminate-dual-pipeline campaign, 2026-04-05), `docs/current/EDITOR_MULTI_TOOL_ARCHITECTURE_CAMPAIGN.md` (Sub-phase 3 in flight as of 2026-05-08, "Round 8 Closure" — the §7.7 "wrapped-component resource identity trap" anti-pattern was elevated from candidate to structural axiom in this campaign). Read those for depth on specific concerns; this trace covers the structural shape. |

---

Viewport renderer sharing (2026-10-15, doc 1.18): `ViewportWidget` keeps `ViewportRenderer` in `Rc<RefCell<_>>`. Callers borrow it on the UI thread only (`App::update`, dock subsystems, the widget paint path). The type is not `Send`. The IBL bake thread does not receive this renderer. `eframe::App` does not require `Send`.

## 1. Executive Summary

**What this system does:**
A full-featured GUI editor for AstraWeave game content: levels (`LevelDoc` TOML), scenes (`SceneData` RON), materials, terrain, biome painting, foliage scatter, behavior graphs, dialogue trees, quests, animations, navigation meshes, and ~30 other authoring domains — each surfaced as a panel inside an `egui_dock`-based docked layout (`tools/aw_editor/src/dock_layout.rs:1-966`, `tools/aw_editor/src/dock_panels.rs:1-626`). The editor also runs games it has authored: there are two binaries (`aw_editor` GUI + `aw_game_runtime` standalone runner — both windowed via winit+wgpu) and a Play-in-Editor (`PiE`) mode that swaps the live `EditorRuntime` between `Editing`, `Playing`, `Paused`, and `SteppingOneFrame` states (`tools/aw_editor/src/runtime.rs:18-25`).

**Why it exists:**
Per `tools/aw_editor/README.md:1-3`: "A non-programmer friendly GUI tool for creating and editing levels, encounters, and boss fights in AstraWeave." The scope has expanded substantially since that line was written — the README still describes only the level-editing surface, but the actual crate spans visual editing, asset authoring, blueprint design, build management, plugin hosting, performance profiling, and game runtime hosting. The aspirational scope is documented at `tools/aw_editor/EDITOR_ROADMAP_TO_WORLD_CLASS.md` and `tools/aw_editor/INTEGRATION_ROADMAP.md`.

**Where it primarily lives:**
- `tools/aw_editor/src/main.rs` — 9,681 lines, contains the `EditorApp` god struct (123 fields verified 2026-05-12 by `awk 'NR==216,/^\}$/' | awk '/^[[:space:]]+(pub )?[a-z_]+: /'`, lines 216-?) and `impl eframe::App for EditorApp` (line 9119). `fn main()` at line 9582.
- `tools/aw_editor/src/lib.rs` — 109 lines, re-export surface for tests/benches.
- `tools/aw_editor/src/tab_viewer/mod.rs` — 8,185 lines, 157 functions. The `egui_dock::TabViewer` impl that translates `PanelType` enum variants to per-panel `.ui()` calls + per-panel state.
- `tools/aw_editor/src/panel_type.rs` — 794 lines, the `PanelType` enum with **41 variants** (one per dockable panel) and `PanelCategory` enum with 6 categories (`Scene`, `Assets`, `Debug`, `Tools`, `System`, `Content`).
- `tools/aw_editor/src/panels/` — **49 panel files** (one per authoring domain).
- `tools/aw_editor/src/viewport/` — 3D viewport stack (camera, renderer adapter, gizmos, grid, physics debug, blueprint overlay) + 3 WGSL shaders (`gizmo.wgsl`, `grid.wgsl`, `tonemap.wgsl`).
- `tools/aw_editor/src/gizmo/` — 12 files for translate/rotate/scale handles + snapping + picking + constraints.
- `tools/aw_editor/src/active_tool/` — Newer dispatcher architecture for paint tools (Multi-Tool Architecture campaign Sub-phase 2, 2026-05-04).
- `tools/aw_editor/src/command.rs` — 2,858 lines, undo/redo `UndoStack` + `EditorCommand` trait.
- `tools/aw_editor/src/runtime.rs` — 1,690 lines, `EditorRuntime` with deterministic play/pause/stop.
- `tools/aw_editor/src/scene_state.rs` — 714 lines, `EditorSceneState` canonical edit-mode world owner.
- `tools/aw_editor/src/entity_manager.rs` — `EntityManager` + `SelectionSet`.
- `tools/aw_editor/src/plugin.rs` — `PluginManager` + `PluginEvent` for runtime extension.
- `tools/aw_editor/src/subsystems/` — 5 files (`audio_animation.rs`, `docking_sync.rs`, `hotkeys.rs`, `scene_stats.rs`, `mod.rs`) extracted from `main.rs` in commit `0de315693` (2026-04-08, "Refactor editor subsystems: extract docking synchronization, hotkey handling, and scene statistics into separate modules") — note: the 2026-04-03 dead-code cleanup referenced in `main.rs:1-7` is a separate event (738→0 dead-code warnings, no file extraction).
- `tools/aw_editor/src/ui/` — `MenuBar`, `StatusBar`, `ProgressManager`, `ToastManager`, `ResourceUsage`.
- `tools/aw_editor/Cargo.toml` — 11 features (`editor-core`, `editor-graphs`, `editor-materials`, `editor-terrain`, `editor-nav`, `editor-sim`, `editor-full`, `profiling`, `fast-alloc`, `terrain-splat-arrays`, `impostor-bake`). `kittest` was removed with the zbus 5 audit: `egui_kittest` 0.32 forces AccessKit. 18 direct `astraweave-*` path deps (verified by `grep '^astraweave-' Cargo.toml`). Linux file dialogs use `rfd` 0.17, which needs `libdbus` or `zenity` at runtime (the XDG portal). GTK3 headers are not required to build.

**Status note:**
The crate is **mid-campaign on multiple architectural fronts**:
1. **Multi-Tool Architecture Campaign** (Sub-phase 3, "Mediator Brush" diagnostic, currently at Round 8 closure as of 2026-05-08): migrating brush-tool dispatch from per-frame mediator code in `main.rs` to a canonical `ActiveTool` trait + `Dispatcher`. Eight rounds of evidence-grounded narrowing surfaced the **§7.7 "wrapped-component resource identity trap" anti-pattern** at four different architectural layers (depth target, mesh data inter-component, texture data intra-component, UI/renderer capacity boundary). The campaign doc proposes elevating this from "candidate corollary" to a first-class top-level CLAUDE.md Edit. See `docs/current/EDITOR_MULTI_TOOL_ARCHITECTURE_CAMPAIGN.md` Status header.
2. **Behavioral Correctness Audit** post-remediation (2026-04-04): 14 CRITICAL + 18 HIGH findings catalogued at `docs/current/EDITOR_BEHAVIORAL_CORRECTNESS_AUDIT.md`. Most known-issue items from Phase 1 are FIXED, but Phase 4 enumerated 9 operations that bypass the undo stack (creation paths) and Phase 6 identified 12 critical + 18 high silent failures.
3. **Fix27 Unified Pipeline Campaign** post-completion: the dual `FastPreview` + `EnginePBR` rendering paths have been collapsed into a single engine-driven path (per `docs/current/FIX27_UNIFIED_PIPELINE_CAMPAIGN.md`); the viewport's docstring at `src/viewport/mod.rs:11-27` now describes the unified architecture as canonical.

---

## 2. Authoritative Pipeline

The editor is an `eframe`/`egui` application driven by a single per-frame update loop. The pipeline below is the per-frame data flow; campaign work mutates the shape of individual stages but the overall topology has been stable since the docking system landed.

```text
[OS: window events from winit (via eframe)]
    │
    │ eframe wraps the egui Context; calls update() once per redraw request
    ▼
[EditorApp::update(ctx, frame)]                        (main.rs:9119-9120)
    │
    │ 1. Splash screen check (until first frame after splash ends)
    │ 2. Tutorial overlay check
    │ 3. tick_audio_subsystem(frame_time)                            (subsystems/audio_animation.rs:10-34)
    │ 4. tick_animation_subsystem(frame_time)                        (subsystems/audio_animation.rs:37-89)
    │ 5. tick_movement_scripts(frame_time)  [play mode only]         (subsystems/audio_animation.rs:92-118)
    │ 6. process_pending_hotkeys()
    │ 7. process_pending_decomp_messages()  [background blend import]
    │ 8. process_file_watcher_events()      [hot-reload]
    │ 9. Auto-save check
    │
    ▼
[Menu bar + status bar + dialogs]                      (ui/menu_bar.rs, ui/status_bar.rs)
    │
    │ MenuBar::show — File / Edit / View / Asset / Build / Help
    │   - Translates clicks into MenuActionHandler trait calls (main.rs:8520)
    │   - Confirm dialogs for destructive ops (new scene, open scene, quit)
    │
    ▼
[Dock layout rendering]                                (dock_layout.rs:1-966, dock_panels.rs:1-626)
    │
    │ if use_docking { DockArea::new(&mut dock_state).show(ctx, &mut tab_viewer) }
    │ else            { /* legacy non-docked floating panels — preserved for fallback */ }
    │
    ▼
[Per-panel ui() dispatch via egui_dock::TabViewer]     (tab_viewer/mod.rs)
    │
    │ For each visible Tab (one per PanelType variant — 41 total):
    │   match panel_type {
    │       PanelType::Hierarchy    => hierarchy_panel.show(ui, ...)
    │       PanelType::AssetBrowser => asset_browser.show(ui, ...)
    │       PanelType::Viewport     => viewport.ui(ui, ...)
    │       ... 38 more arms ...
    │   }
    │
    │ Each panel:
    │   - Reads its slice of EditorApp state (via EditorTabViewer's borrow-broker)
    │   - Mutates its own panel struct (most panels are owned by EditorApp)
    │   - Emits PanelEvent values onto a queue for cross-panel coordination
    │   - May trigger EditorCommand pushes onto the UndoStack
    │
    ▼
[Viewport rendering — special-cased panel]             (viewport/widget.rs, viewport/renderer.rs)
    │
    │ ViewportWidget::ui() is called inside the PanelType::Viewport arm.
    │ Internal pipeline (per viewport/mod.rs:11-27 doc comment):
    │
    │ ViewportRenderer (coordinator)
    │   ├─ EngineRenderAdapter      → astraweave_render::Renderer
    │   │                              (terrain, sky, water, weather, scatter, entities, post-FX)
    │   ├─ GridRenderer             → grid.wgsl, floor grid + axes
    │   ├─ GizmoRenderer            → gizmo.wgsl, transform handles
    │   └─ PhysicsDebugRenderer     → collider wireframes
    │
    │ Phase 1.X (active_tool campaign): brush-style tools route input events
    │ through a `Dispatcher` (active_tool/dispatcher.rs) rather than per-frame
    │ mediator code in main.rs. As of 2026-05-08 only TerrainPanel is migrated;
    │ Sub-phase 5 will migrate RegionalArchetypePanel.
    │
    ▼
[PanelEvent processing]                                (tab_viewer/panel_event.rs)
    │
    │ EditorTabViewer.take_*_actions() drained per frame in EditorApp::update:
    │   - take_audio_actions() → audio_bridge.process_actions(...)  (audio_bridge.rs:111-208)
    │   - take_animation_actions() → animation_bridge.process_actions(...)
    │   - take_input_binding_actions() → (NEVER DRAINED — see §6 net_ecs trace, parallel)
    │   - take_*_actions() for ~16 other panel types
    │ (Note: per editor behavioral audit, 18 PanelEvent handlers are stubs.)
    │
    ▼
[Runtime tick (if RuntimeState == Playing)]             (runtime.rs)
    │
    │ EditorRuntime::tick() advances:
    │   - The simulation App (ECS App built via astraweave_core::ecs_adapter::build_app)
    │   - The world snapshot
    │   - Stats (frame time, tick count)
    │
    ▼
[Per-frame post-processing]
    │
    │ - Cache invalidation: entity_list_generation vs entity_list_synced_generation
    │ - Resource sampling (last_resource_sample timer)
    │ - Title bar update (cached_title_dirty / title_cmd_sent)
    │ - Window focus tracking + DWM repaint hacks (Windows-specific cursor recovery)
    │ - Initial-maximize one-shot send
```

### Stage-by-stage detail

#### Stage 1: Construction (`EditorApp::default` and `EditorApp::new`)
**Files:** `main.rs:425-2635` (Default::default), `main.rs:2636-2742` (new).
**Role:** Build the god-object with default values, attempt asset manifest load, register plugins, wire panel structs.
**Notes:**
- `EditorApp::default` runs first: tries `asset_db.load_manifest("assets/assets.json")` (line 431) — if no manifest exists, starts empty (scanning 10.9 GB / 112K files at startup was prohibitive per inline comment). User must trigger scan via Asset Inspector.
- `EditorApp::new` (called from `fn main` via `eframe::run_native`) does GPU-dependent init: wgpu device, viewport widget creation, splash screen.
- The struct declaration spans `main.rs:216-?` and has 123 fields (verified 2026-05-12 via `awk` field-pattern count over the struct body). Some fields are gated by `#[cfg(target_os = "windows")]` (e.g. `hwnd_cache` at line 417). The `ARCHITECTURAL_AUDIT_REPORT.md:22` (2025-12-22) reported 55+ fields; the count has more than doubled since.

#### Stage 2: Per-frame update entrypoint (`impl eframe::App for EditorApp::update`)
**File:** `main.rs:9119-9120` (impl block at line 9119; `fn update` immediately follows).
**Role:** The eframe-mandated entry point. Single function but very long — receives `&egui::Context` and `&mut eframe::Frame`, performs the entire frame's logic.
**Notes:**
- Calls into the `subsystems/` modules (extracted in commit `0de315693`, 2026-04-08) for audio, animation, movement, hotkeys, scene stats.
- The function contains the legacy `if let (Some(world), Some(viewport))` gate that the Multi-Tool Architecture campaign Sub-phase 3 H5-diagnostic identified at `main.rs:3867` as the root cause of one defect class (since fixed in commit `8f4668599`).

#### Stage 3: Docking system (`egui_dock` integration)
**Files:** `dock_layout.rs:1-966` (DockLayout + LayoutPreset), `dock_panels.rs:1-626` (panel registration), `tab_viewer/mod.rs:1-8185` (TabViewer impl).
**Role:** Persistent dock state across frames; per-panel `.ui()` dispatch; layout presets (default, full, minimal, etc.).
**Notes:**
- The `EditorTabViewer` struct (`tab_viewer/mod.rs`) owns most panel state. It is borrowed mutably during the dock render call, then drained for `PanelEvent`s afterward.
- `EditorDrawContext` (`tab_viewer/mod.rs:55-`) is a temporary borrow wrapper that gives the Viewport panel access to `ViewportWidget` while other panels work through `EditorTabViewer` alone.
- `LayoutPreset` enum encapsulates pre-canned layouts (developer profile, level designer profile, etc.).

#### Stage 4: Panel implementation (49 panel files)
**Directory:** `tools/aw_editor/src/panels/`.
**Role:** Each panel is a struct + `impl Panel` (the trait declared in `panels/mod.rs`) + companion `*Action` enum for event emission.
**Notes:**
- Panels include: `AudioPanel`, `AnimationPanel`, `AssetBrowser`, `BehaviorGraph`, `BlendImportPanel`, `BlueprintPanel`, `BuildManagerPanel`, `ChartsPanel`, `CinematicsPanel`, `ConsolePanel`, `DialogueEditorPanel`, `DistributionPanel`, `EntityCatalog`, `EntityPanel`, `EnvironmentPresetPanel`, `FoliagePanel`, `FrameDebugger`, `GameplayPresets`, `GraphPanel`, `HierarchyPanel`, `ImportDoctorPanel`, `InputBindingsPanel`, `LightingPanel`, `LocalizationPanel`, `LodConfigPanel`, `MaterialEditorPanel`, `NavigationPanel`, `NetworkingPanel`, `ParticleSystemPanel`, `PcgPanel`, `PerformancePanel`, `PhysicsPanel`, `PostProcessPanel`, `ProfilerPanel`, `ProjectSettingsPanel`, `RegionalArchetypePanel`, `SceneStatsPanel`, `SplineEditorPanel`, `TerrainPanel`, `ThemeManagerPanel`, `TransformPanel`, `UiEditorPanel`, `WorldPanel`, `WorldWizard`, + a few that don't map 1:1 to `PanelType` (e.g., `PluginManagerPanel` is owned by `EditorApp.plugin_panel`).
- Each panel struct typically holds: its own state, a `pending_actions: Vec<*Action>`, and helper methods. Per the conventions established by `input_bindings_panel.rs:866` and `audio_panel.rs`, panels emit actions via `queue_action()` and external code drains via `take_actions()` — but per `docs/architecture/net_ecs.md` §6, several panels' action queues are never drained.
- Several panels are independently-shipped sub-systems that duplicate engine functionality (see §6, particularly `input_bindings_panel.rs` which is a 2511-line standalone re-implementation of `astraweave-input` types).

#### Stage 5: Viewport rendering (post-Fix27 unified pipeline)
**Files:** `viewport/widget.rs`, `viewport/renderer.rs`, `viewport/engine_adapter.rs`, `viewport/grid_renderer.rs`, `viewport/gizmo_renderer.rs`, `viewport/physics_renderer.rs`, `viewport/blueprint_overlay.rs`, `viewport/camera.rs`, `viewport/terrain_splat.rs`, `viewport/terrain_splat_builder.rs`, `viewport/terrain_biome_placeholder.rs`, `viewport/types.rs`, `viewport/toolbar.rs`, `viewport/shaders/*.wgsl` (3 shaders).
**Role:** 3D rendering of the scene + editor overlays.
**Notes:**
- The unified-pipeline architecture (per `viewport/mod.rs:11-27`) is the post-Fix27 design: engine renderer drives terrain/sky/water/weather/scatter/entities/post-FX; editor-local renderers handle grid, gizmos, physics debug, blueprint overlay.
- The `engine_adapter.rs` is the integration seam with `astraweave-render::Renderer`. Per `docs/current/EDITOR_MULTI_TOOL_ARCHITECTURE_CAMPAIGN.md` Round 5 closure entry (2026-05-07), this seam was the site of the original §7.7 trap: the engine adapter's internal depth target was distinct from `aw_editor`'s own `self.depth_texture`, causing brush cursor mis-positioning until commit `0f569d212` plumbed a `depth_view` parameter through `Renderer::draw_into`.
- Three WGSL shaders are owned by the editor (`gizmo.wgsl`, `grid.wgsl`, `tonemap.wgsl`) — down from a larger set that included `entity.wgsl`, `shadow.wgsl`, `brdf_lut.wgsl`, `mipmap_blit.wgsl` per the Fix27 campaign target (those were deleted when the engine renderer took over).
- The `impostor-bake` feature gates `impostor_registry.rs` + `impostor_wiring.rs` (LOD3 billboard pipeline, enabled in default features per Cargo.toml:31-35).

#### Stage 6: Gizmo system
**Files:** `gizmo/mod.rs`, `gizmo/translate.rs`, `gizmo/rotate.rs`, `gizmo/scale.rs`, `gizmo/constraints.rs`, `gizmo/snapping.rs`, `gizmo/picking.rs`, `gizmo/rendering.rs`, `gizmo/input.rs`, `gizmo/state.rs`, `gizmo/scene_viewport.rs`, `gizmo/tests_gizmo_math.rs`.
**Role:** Transform manipulation handles (translate/rotate/scale) with axis constraints, grid snapping, ray-casting picking.
**Notes:**
- 12 files; gizmo math (translate/rotate) was VERIFIED CORRECT in the Behavioral Correctness Audit. Scale gizmo is "UP-only" per audit Phase 2 MEDIUM finding.
- `gizmo_benchmarks.rs` exists at `tools/aw_editor/benches/gizmo_benchmarks_simple.rs`.

#### Stage 7: Active-tool dispatcher (Multi-Tool Architecture campaign)
**Files:** `active_tool/mod.rs`, `active_tool/dispatcher.rs`, `active_tool/tests.rs`.
**Role:** Canonical event-routing infrastructure for brush-style tools (terrain paint, regional archetype paint, future paint tools).
**Notes:**
- Introduced in 2026-05-04 in commits `813ac29a1` + `2c791fa39` + `ece7bb3b4` + `6016b3c8f` (per `EDITOR_MULTI_TOOL_ARCHITECTURE_CAMPAIGN.md` Sub-phase 2).
- `ActiveTool` trait + `Dispatcher` struct + UUID-based registry. 15 unit tests pass.
- As of `a2474c5b7`, only `TerrainPanel` is registered (Sub-phase 3); `RegionalArchetypePanel` migration deferred to Sub-phase 5.

#### Stage 8: Command + undo stack
**File:** `command.rs:1-2858`.
**Role:** `EditorCommand` trait + `UndoStack` struct; concrete commands `MoveEntityCommand`, `RotateEntityCommand`, `ScaleEntityCommand`, terrain brush undo via `TerrainUndoQueue` side-channel.
**Notes:**
- Per Behavioral Correctness Audit Phase 4: **9 operations bypass the undo stack** (creation paths). Entity deletion bug at `command.rs:1250` was FIXED (undo restores both World AND EntityManager). Spawn undo bug fixed to use `destroy_entity()` at `command.rs:1140`.

#### Stage 9: Runtime + Play-in-Editor
**File:** `runtime.rs:1-1690`.
**Role:** Deterministic play/pause/step/stop of an `astraweave-ecs::App` built via `astraweave_core::ecs_adapter::build_app`.
**Notes:**
- `RuntimeState` enum: `Editing`, `Playing`, `Paused`, `SteppingOneFrame` (`runtime.rs:18-25`).
- `EditorRuntime` owns the running `App`, the world snapshot at play-start (for stop-restore), and runtime statistics.
- PiE transitions go through `EditorMode` (`editor_mode.rs`, 3-state: `Edit`, `Play`, `Paused`) which gates input/output between editor and game.

#### Stage 10: Plugin system
**File:** `plugin.rs`.
**Role:** Runtime extension points via `EditorPlugin` trait + `PluginManager` + `PluginEvent` enum.
**Notes:**
- `PluginManagerPanel` provides a UI for managing plugins.

#### Stage 11: Hot-reload + file watcher
**File:** `file_watcher.rs`.
**Role:** Watches the project directory for changes, re-triggers asset reloads.
**Notes:**
- Per Behavioral Correctness Audit Phase 6 (silent failures): had 5 `.lock().unwrap()` calls flagged as mutex-poison vulnerabilities. **FIXED:** verified at `file_watcher.rs:363, 375, 388, 398, 424` — all `.lock()` calls now use `.unwrap_or_else(|poisoned| poisoned.into_inner())` (graceful poison recovery). Zero `.lock().unwrap()` remaining in this file.

---

## 3. Semantic Vocabulary

| Term | Definition | Used in |
|---|---|---|
| **EditorApp** | The god struct. Owns scene state, all panel state, viewport, runtime, undo stack, dock state, plugin manager, asset DB, settings, dialog flags, cached UI values. 123 fields (verified 2026-05-12). | `main.rs:216-?` |
| **PanelType** | Enum with 41 variants (one per dockable panel) + 6 category variants. Used as the key for `egui_dock` tab identification. | `panel_type.rs:107-` |
| **PanelEvent** | Enum of cross-panel events. Drained per-frame from `EditorTabViewer.take_*_actions()` and dispatched to bridges, services, or other panels. | `tab_viewer/panel_event.rs` |
| **EditorRuntime** | Owns the active simulation `App` during Play-in-Editor mode. Holds `RuntimeState` + snapshot for stop-restore. | `runtime.rs:1-1690` |
| **EditorSceneState** | The canonical edit-mode world owner (Week 1 work, 2025-09 era). Holds the `World` instance that panels read from / write to during Editing mode. | `scene_state.rs:1-714` |
| **EntityManager** | Independent registry mapping `EntityId: u64` to `EditorEntity` (per-entity transform + selection state). Bridged to `World::Entity: u32` via `entity_id_to_world` at `main.rs:139-150` with overflow check. | `entity_manager.rs` |
| **SelectionSet** | The set of currently-selected entities. Drives gizmo + inspector behavior. | `entity_manager.rs` |
| **LevelDoc** | The canonical level-document TOML schema: `{ title, biome, seed, sky, biome_paints, obstacles, npcs, fate_threads, boss }`. | `level_doc.rs` |
| **SceneData** | Phase 2.2 scene-save format (RON), distinct from `LevelDoc`. The two coexist (see §6). | `scene_serialization.rs` |
| **PrefabData / PrefabInstance** | Hierarchical prefab system with instance snapshots. | `prefab.rs` |
| **EditorCommand** | Trait + concrete impls for undoable operations. `UndoStack` is the stack. | `command.rs` |
| **DockLayout / DockState** | The persistent docking layout (positions, sizes, which panels are docked where, which floating). | `dock_layout.rs` |
| **EditorTabViewer** | Implements `egui_dock::TabViewer`. Owns most panel state; per-frame mutable borrow. | `tab_viewer/mod.rs` |
| **ActiveTool / Dispatcher** | Newer (2026-05-04) trait + registry for brush-style tools. Trait-object collection keyed by UUID. | `active_tool/` |
| **MenuActionHandler** | Trait implemented by `EditorApp` (line 8520) that gives `MenuBar` a callback surface for menu clicks. | `ui/menu_bar.rs` + `main.rs:8520` |
| **EditorMode** | 3-state coarse UI mode: `Edit`, `Play`, `Paused` (verified `editor_mode.rs:3-10` 2026-05-12; earlier doc claim of "2-state `Editing`/`Playing`" was wrong on both variant count AND variant names). Distinct from `RuntimeState` (4-state finer machine). | `editor_mode.rs` |
| **PluginManager / PluginEvent** | Runtime extension points. | `plugin.rs` |
| **WorldWizard** | Modal dialog for guided new-world creation. | `panels/mod.rs` (re-export) |
| **MaterialLiveDoc** | Live-editing document for the Material Inspector. | `main.rs` (referenced as field) |
| **TerrainUndoQueue** | Side-channel undo recording for terrain brush operations (parallel to main `UndoStack`). | `command.rs` |
| **MovementSystem** | Per-entity movement script ticker (`subsystems/audio_animation.rs::tick_movement_scripts`). Only ticks in play mode. | `movement_scripts.rs` |
| **AssetDatabase** | Index of project assets, loaded from `assets/assets.json` manifest at startup. Re-scanned via Asset Inspector. | `astraweave-asset::AssetDatabase` (external) |
| **EditorAudioBridge / EditorAnimationBridge** | Per-panel command consumers — translate `PanelEvent`s into `astraweave-audio` / animation library calls. | `audio_bridge.rs`, `animation_bridge.rs` |
| **EditorDrawContext** | Temporary borrow wrapper passed during dock render so Viewport panel can access `ViewportWidget` while other panels access `EditorTabViewer`. | `tab_viewer/mod.rs:55-` |
| **FateThread** | A trigger+ops pair in the level document. Used for AI director scripting. | `level_doc.rs` |
| **§7.7 trap** ("wrapped-component resource identity trap") | Architectural anti-pattern surfaced 4 times in the Multi-Tool Architecture campaign: when component A wraps/coexists with component B and both manage similar resources of the same logical type, the higher-level component's reads from its own resource don't reflect the lower-level component's writes to its own resource — bug is at the boundary. | `docs/current/EDITOR_MULTI_TOOL_ARCHITECTURE_CAMPAIGN.md` Status header |

### Terms to NOT confuse

- **`EditorMode` (3-state) vs `RuntimeState` (4-state):** `EditorMode { Edit \| Play \| Paused }` is the coarse UI mode (verified `editor_mode.rs:3-10`); `RuntimeState { Editing \| Playing \| Paused \| SteppingOneFrame }` is the fine state machine of `EditorRuntime`. They are linked but distinct: roughly `EditorMode::Edit` ↔ `RuntimeState::Editing`; `EditorMode::Play` ↔ `RuntimeState::Playing` or `SteppingOneFrame`; `EditorMode::Paused` ↔ `RuntimeState::Paused`. The naming is genuinely inconsistent (`Edit` vs `Editing`, `Play` vs `Playing`) — both enums coexist independently.

- **`World` (live game world) vs `EditorSceneState` (edit-mode owner) vs `EntityManager` (separate registry) vs `SceneData` (serialization format) vs `LevelDoc` (level TOML):** Five distinct representations of "the scene." Per Behavioral Correctness Audit §1.2 line 80: "Cache inconsistency risk" — this is the trap. `World` is what the simulation sees; `EntityManager` is the editor's overlay (per-entity selection, material slots, etc.); `EditorSceneState` is the edit-mode owner; `SceneData` is what `scene_serialization.rs` reads/writes (RON); `LevelDoc` is what the legacy level TOML reads/writes.

- **`EntityId: u64` (EntityManager) vs `Entity: u32` (World):** Bridged by `entity_id_to_world` at `main.rs:139-150` with explicit overflow check that warns and returns `None` if `EntityId > u32::MAX`. Most editor APIs traffic in the `u64` form.

- **`PanelType` (enum, 41 variants) vs Panel structs in `panels/` (49 files):** Most panels have a 1:1 mapping (one `PanelType` variant per panel), but several `panels/*.rs` files do not appear in `PanelType`'s match (e.g., `WorldWizard`, `PluginManagerPanel`, `ThemeManagerPanel`'s in-line variant, `EntityCatalog`). The set of variants is enumerated at `panel_type.rs:109-232`.

- **`tab_viewer/mod.rs::EditorTabViewer` vs `tab_viewer/mod.rs::SimpleTabViewer`:** `EditorTabViewer` is the production tab viewer that holds all panel state. `SimpleTabViewer` (also re-exported at `lib.rs:74`) is a lighter-weight placeholder used for tests / library consumers.

- **`docking_sync` subsystem vs `dock_layout` module vs `dock_panels` module:** Three different things. `subsystems/docking_sync.rs` is the per-frame sync between `DockLayout` and `EditorTabViewer`. `dock_layout.rs` is the persistent layout state + presets. `dock_panels.rs` is the registration table.

- **`subsystems/` (5 files) vs `panels/` (49 files, 48 panel files + `mod.rs`):** `subsystems/` are *extracted* helpers from `main.rs` (commit `0de315693`, 2026-04-08); they don't render UI directly. `panels/` are the dockable UI components.

- **`active_tool::Dispatcher` (newer) vs per-frame mediator in `main.rs:3833-3877` (older, being deprecated):** Per the Multi-Tool Architecture campaign, the mediator code in `main.rs` is being replaced by the dispatcher. Sub-phase 3 migrated `TerrainPanel`; Mediator Removal session is a separate Andrew-gated session.

---

## 4. Cross-System Touchpoints

### Upstream (what feeds this system)

| Source system | Interface | Data | Notes |
|---|---|---|---|
| `astraweave-core` (`Cargo.toml:68`) | `World`, `Entity`, `IVec2`, `Team`, `ecs_adapter::build_app` | The grid-coordinate world used by the simulation; `EditorSceneState` owns one instance. | The editor still uses `IVec2`-based grid coordinates internally even though the engine renderer now supports continuous 3D. The bridging happens at `viewport/engine_adapter.rs`. |
| `astraweave-ecs` (`Cargo.toml:69`) | `App`, `World` (the ECS variant, distinct from `astraweave-core::World`) | The running ECS App used during PiE play mode. | `runtime.rs:5-6` imports both. |
| `astraweave-render` (`Cargo.toml:73`, features `gltf-assets, textures`) | `Renderer`, plus features `terrain-splat-arrays` and `impostor-bake` re-exported as editor features | Core rendering pipeline (PBR, shadows, IBL, post-FX, terrain). | Post-Fix27 this is the SOLE renderer for scene content; the editor only contributes overlays (grid, gizmo, physics debug, blueprint overlay). See `docs/current/FIX27_UNIFIED_PIPELINE_CAMPAIGN.md`. |
| `astraweave-asset` + `astraweave-asset-pipeline` (`Cargo.toml:75, 92`, features `blend`) | `AssetDatabase`, blend asset import | Project asset index, blend-file decomposition for blueprints. | Default empty-DB-at-startup pattern noted at `main.rs:429-435`. |
| `astraweave-blend` (`Cargo.toml:76`) | Blend importer | `.blend` file decomposition into reusable game assets. | Driven by `blend_scanner.rs` + `panels/blend_import_panel.rs`. |
| `astraweave-audio` (`Cargo.toml:77`) | `AudioEngine` | Audio playback for editor and game preview. | Bridged via `audio_bridge.rs` (per `docs/architecture/audio.md`). |
| `astraweave-behavior` (`Cargo.toml:78`) | `BehaviorTree`, `BehaviorGraph` | AI behavior authoring; surfaced as `behavior_graph/` editor + `BehaviorGraph` panel. | Doc references `behavior_graph::{BehaviorGraphDocument, BehaviorGraphEditorUi}` at `main.rs:97`. |
| `astraweave-dialogue` (`Cargo.toml:79`) | `DialogueGraph` | Dialogue tree authoring; `DialogueEditorPanel`. | Imported at `main.rs:94`. |
| `astraweave-quests` (`Cargo.toml:80`) | `Quest` | Quest graph authoring. | Imported at `main.rs:95`. |
| `astraweave-nav` (`Cargo.toml:81`) | `NavMesh` | Navigation mesh generation + editing; `NavigationPanel`. | Imported at `main.rs:95`. |
| `astraweave-physics` (`Cargo.toml:83`) | `PhysicsWorld`, debug rendering | Physics simulation + collider visualization in viewport. | `PhysicsPanel` + `viewport/physics_renderer.rs`. |
| `astraweave-terrain` (`Cargo.toml:85`) | Terrain mesh generation, splat material system | Terrain editor surface; `TerrainPanel`. | Heavily exercised by the Multi-Tool Architecture campaign Sub-phase 3 ("Mediator Brush" diagnostic rounds 1-8). |
| `astraweave-security` (`Cargo.toml:84`) | Path validation | Validates output paths in `package_demos` / `save_to_file` (per `4889a9a33` commit). | Added 2025-11-13. |
| `astraweave-author` (`Cargo.toml:74`) | Authoring helpers | Re-exported authoring types. | Declared in `Cargo.toml:74` but **NOT imported by any `.rs` file** in `tools/aw_editor/` (verified by `grep -r "astraweave_author"` across the crate). Dormant dependency — likely from an earlier integration plan that was not completed. |
| `astraweave-profiling` (`Cargo.toml:70`, feature-gated by `profiling`) | `frame_mark!`, `plot!`, `span!` macros | Tracy-style profiling instrumentation. | Used in `runtime.rs`. |
| `astraweave-observability` (`Cargo.toml:82`) | Tracing setup | Editor's tracing/log infrastructure. | Declared in `Cargo.toml:82` but **NOT imported by any `.rs` file** in `tools/aw_editor/` (verified by `grep -r "astraweave_observability"` across the crate). Dormant dependency. Editor uses the `tracing` crate directly (e.g., `telemetry.rs:4`, `console_bridge.rs`) instead. |
| `astraweave-alloc` (`Cargo.toml:72`, optional via `fast-alloc`) | `setup_global_allocator!` macro | Opt-in mimalloc global allocator. Per Cargo.toml comment, enabled by default per 2026-04-17 mimalloc experiment. | `main.rs:11-12`. |
| `astract` (`Cargo.toml:67`, path `crates/astract`) | UI component library | Astract panel framework (custom widget library — `WorldPanel`, `EntityPanel`, `PerformancePanel`, `ChartsPanel`, `AdvancedWidgetsPanel`, etc.). | Per `main.rs:106-110`. |
| `egui_dock` + `eframe` + `egui` + `egui-wgpu` + `wgpu` (workspace deps) | UI rendering + dock infrastructure + GPU surface | The whole UI is `egui`-based. | The `egui` dep mirrors the `eframe`/`egui_dock` versions to avoid drift. |
| Filesystem (project files) | `LevelDoc` TOML, `SceneData` RON, `.awsv` saves, `.blend` files, `.gltf` meshes, image textures, etc. | All project content. | `RecentFilesManager` (`recent_files.rs`) tracks recently-opened files; `file_watcher.rs` watches for hot-reload. |
| Filesystem (editor state) | `editor_preferences` (per-user prefs), auto-save snapshots, crash recovery files | Persistent editor state. | `editor_preferences.rs`, `main.rs:308-323` (auto-save fields + recovery). |
| OS (winit via eframe) | Window events, keyboard, mouse, file drops | Standard windowing input. | `tools/aw_editor/Cargo.toml:57` pulls winit. |

### Downstream (what consumes this system's output)

| Consumer system | Interface | Data | Notes |
|---|---|---|---|
| Filesystem (`.toml` levels, `.ron` scenes, `.blend` outputs, `.awsv` saves, etc.) | `LevelDoc` TOML write, `SceneData` RON write, package_demos export | Authored content. | Saved via `MenuBar::on_save` → `EditorApp::save_*` family. |
| `aw_game_runtime` binary (`Cargo.toml:122-126`) | Reads the saved scene/level files via `aw_editor_lib::game_project::GameProject` | Standalone game-runner binary at `src/bin/aw_game_runtime.rs` (336 lines, clap-based CLI). | NOT headless — creates a winit window + wgpu device + surface (`aw_game_runtime.rs:91-165`). Earlier description in this doc was incorrect. |
| Plugin system (any `EditorPlugin` impl) | `PluginEvent` + plugin context API | Runtime-loaded extensions. | `plugin.rs::PluginManager`. |
| Distribution + package_demos | `distribution.rs` + `panels/distribution_panel.rs` | Game-build outputs ready for distribution. | Heavy on cross-crate orchestration. |

### Bidirectional / Coupled

- **`EditorApp` ↔ everything:** The god struct holds direct references to nearly every subsystem (audio_bridge, animation_bridge, runtime, undo_stack, plugin_manager, viewport, dock state, all panels). The `ARCHITECTURAL_AUDIT_REPORT.md` (2025-12-22) flagged this as the highest-priority architectural issue.
- **`EditorSceneState` ↔ `EntityManager` ↔ `World`:** Three representations of "what's in the scene." The `cache_inconsistency_risk` documented in audit §1.2 is real — the doc explicitly cites "circular dependencies risk: `scene_state` ↔ `entity_manager` ↔ `main.rs`."
- **`EditorTabViewer` ↔ all panels:** The tab viewer holds most panel state. Per-frame mutable borrow. Panels emit events through the tab viewer's queues.
- **`UndoStack` ↔ panels + viewport + commands:** Panels push commands; viewport edits push commands; commands mutate `World` + `EntityManager`. Per audit Phase 4: 9 operations still bypass.

### Documentation references with no code backing

The editor's `docs/src/` tree is small (the aspirational wiki sweep documented in audio/input/net traces did NOT cover the editor). However, the editor accumulates many roadmap and aspirational design docs **inside the crate itself**:

- `tools/aw_editor/EDITOR_ROADMAP_TO_WORLD_CLASS.md` — World-class editor target features (much beyond current state).
- `tools/aw_editor/GAP_ANALYSIS_AND_REMEDIATION_PLAN.md` — Gap analysis.
- `tools/aw_editor/INTEGRATION_ROADMAP.md` — Integration plan.
- `tools/aw_editor/PHASE_*_PROGRESS_REPORT.md` / `PHASE_*_SESSION_SUMMARY.md` / `PHASE_*_IMPLEMENTATION_PLAN.md` — Per-phase status. Phases 1 through 4 are documented (multiple sub-phases per phase).
- `tools/aw_editor/PRODUCTION_READINESS_AUDIT.md` — Production-readiness gap analysis.
- `tools/aw_editor/VIEWPORT_ENHANCEMENT_COMPLETE.md` / `VIEWPORT_STATUS.md` — Viewport campaign closure.
- `tools/aw_editor/TEST_COVERAGE.md` — Test coverage report.
- `tools/aw_editor/CODE_QUALITY_STATUS.md` — Code quality status.
- `docs/current/AW_EDITOR_*.md` (10 files verified 2026-05-12: `AUTHORING_PLAN`, `CORRECTNESS_AUDIT_REPORT`, `INTERACTION_PLAN`, `KNOWN_ISSUES`, `MUTATION_VULNERABILITY_REPORT`, `QUICK_REFERENCE`, `RECOVERY_ROADMAP`, `SIMULATION_PLAN`, `UAT`, `WEEK_4_START`) — Various authoring/correctness/interaction/known-issues plans.
- `docs/current/EDITOR_*.md` (9 files verified 2026-05-12: `AUDIT_REPORT`, `BASELINE`, `BEHAVIORAL_CORRECTNESS_AUDIT`, `MULTI_TOOL_ARCHITECTURE_CAMPAIGN`, `MUTATION_TESTING_VULNERABILITY_REPORT`, `PREFAB_HIERARCHY_VALIDATION`, `RUNTIME_VALIDATION`, `STATUS_REPORT`, `TEST_STRATEGY_REPORT`) — Various editor-specific audit / status / strategy reports. Earlier "12+" figure was an estimate; verified count is 9.

These are historical and forward-looking, not API specifications, so they are correct documentation references — they aren't "aspirational types referencing nonexistent code" in the way the audio/input/net aspirational docs are. But agents should be aware that some of these (especially the older PHASE_1/PHASE_2 docs) describe the editor at an earlier state.

---

## 5. Active File Map

The editor has 216 `.rs` files. This map captures the **load-bearing structural pieces** by directory, with individual files called out where they are central to one architectural responsibility. Per-panel files in `panels/` are summarized as a single row; agents needing per-panel detail should `cat` the relevant panel file directly.

| File / Directory | Role | Status | Notes |
|---|---|---|---|
| `src/main.rs` | Binary entrypoint + `EditorApp` god struct + 182 impl methods + `impl eframe::App` + `fn main` | Active | 9,681 lines. `EditorApp::default` at line 425; `EditorApp::new` at 2636; `impl eframe::App for EditorApp` at 9119; `fn main` at 9582. 123-field god struct (verified 2026-05-12; flagged at 55+ fields in `ARCHITECTURAL_AUDIT_REPORT.md:22` 2025-12-22). |
| `src/lib.rs` | Library entry; module declarations + re-exports for tests/benches | Active | 109 lines. Allows `aw_editor_lib` to be consumed for testing without compiling the binary. |
| `src/panels/` (49 files) | One file per dockable panel; each implements `Panel` trait + companion `*Action` enum | Active (per-panel: some are stubs, some are production-grade, see audits) | The audit at `EDITOR_BEHAVIORAL_CORRECTNESS_AUDIT.md` Phase 7 enumerates "18 stub PanelEvent handlers" — agents should read that doc for per-panel maturity. |
| `src/panels/mod.rs` | Panel trait + re-exports of all 49 panel structs | Active | Defines the cross-panel contract. |
| `src/tab_viewer/mod.rs` | `EditorTabViewer` + `SimpleTabViewer` + `egui_dock::TabViewer` impl; 157 functions | Active | 8,185 lines. The central panel-state-holder + dispatch hub. |
| `src/tab_viewer/{inspectors,models,panel_event,sky_colors}.rs` | Per-panel inspector UI, data-only structs, PanelEvent enum, sky-color computation | Active | Split from tab_viewer/mod.rs during a refactor. |
| `src/panel_type.rs` | `PanelType` enum (41 variants) + `PanelCategory` (6 variants) + helper methods | Active | 794 lines. |
| `src/dock_layout.rs` | `DockLayout` + `LayoutPreset` enum (developer, level designer, etc.) | Active | 966 lines. Dock chrome derives from the active egui style via the single `themed_style()` fn — called at construction AND re-derived per frame in `show`/`show_inside` (theme-reactive; do not cache a second styled copy). |
| `src/dock_panels.rs` | Panel registration table for `egui_dock` | Active | 626 lines. |
| `src/viewport/mod.rs` + `widget.rs` + `renderer.rs` + `engine_adapter.rs` | 3D viewport widget; post-Fix27 unified pipeline coordinator | Active | The architecture docstring at `mod.rs:11-27` is canonical post-Fix27. `engine_adapter.rs` was the site of the Round 5 §7.7 trap (Mediator Brush diagnostic). |
| `src/viewport/{grid,gizmo,physics,blueprint_overlay}_renderer.rs` | Editor-local overlay renderers | Active | Local renderers preserved post-Fix27 unification. |
| `src/viewport/camera.rs` | Orbit camera + ray casting + depth unprojection | Active | `read_depth_at_pixel` + `unproject_depth_to_world` are exercised by the Multi-Tool Architecture diagnostic Rounds 4-5. |
| `src/viewport/{terrain_splat,terrain_splat_builder,terrain_biome_placeholder}.rs` | Terrain texture management | Active | `terrain_splat_builder.rs` was the site of the Round 7 §7.7 trap (texture-data intra-component sibling-attribute drift); fixed in Real-Fix.C commit `ded9a0457`. |
| `src/viewport/palette_remap.rs` + `viewport/canonical_terrain_pack.rs` | Paint-palette→pack-layer name remap (AD.5.A Fix 3 / AD.4.A §5a Option C) + canonical biome pack loader | Active (new) | `PaletteRemap::resolve` joins library `MATERIAL_NAMES` against per-layer albedo stems (`CanonicalLayerBytes.albedo_stem`, retained at load). Resolved by `EngineRenderAdapter` on every terrain-layer upload; synced per frame into `dock_tab_viewer.terrain_panel`; paint writes the remapped layer index, unresolved palette entries render disabled. |
| `src/viewport/types.rs` | Shared viewport types | Active | `TerrainVertex` defined here; was the site of the 96-vs-36 byte mismatch flagged CRITICAL in Behavioral Correctness Audit; subsequently unified by Fix27. |
| `src/viewport/shaders/` | 3 WGSL shaders | Active | `gizmo.wgsl`, `grid.wgsl`, `tonemap.wgsl`. The other shaders (entity, shadow, brdf_lut, mipmap_blit) were deleted post-Fix27. |
| `src/viewport/{impostor_registry,impostor_wiring}.rs` | LOD3 impostor billboard pipeline | Active (feature-gated by `impostor-bake`) | Per Cargo.toml:31-35 the `impostor-bake` feature is on by default since stage 3d (April 2026). |
| `src/gizmo/` (12 files) | Translate/rotate/scale handles + snapping + picking + tests | Active | `tests_gizmo_math.rs` covers translate/rotate (audit Phase 2 VERIFIED CORRECT). Scale gizmo "UP-only" per audit MEDIUM finding. |
| `src/active_tool/` (3 files) | `ActiveTool` trait + `Dispatcher` + 15 unit tests | Active (new) | Introduced 2026-05-04 (Multi-Tool Architecture Sub-phase 2). |
| `src/behavior_graph/` (5 files) | Behavior graph document + node graph widget + UI | Active | `document.rs`, `node_graph_widget.rs`, `ui.rs`, `tests_document.rs`. |
| `src/subsystems/` (5 files) | Per-frame tick helpers extracted from main.rs | Active | `audio_animation.rs`, `docking_sync.rs`, `hotkeys.rs`, `scene_stats.rs`, `mod.rs`. |
| `src/ui/` (6 files) | `MenuBar`, `StatusBar`, `ProgressManager`, `ToastManager`, `ResourceUsage`, `palette`, `branding` | Active | UI utilities. `ui/palette.rs` (Active (new), 2026-07) is the ONLY place branded theme colors are defined — constants + `astraweave_visuals()` + theme-aware helpers (`accent`, `overlay_fill`, `accent_button`, `empty_selection_state`); consumed by theme_manager, dock_layout, toolbar, status_bar, tab_viewer, entity_catalog, main.rs. `ui/branding.rs` (Active (new), 2026-07) loads `assets/Astraweave_logo.jpg` for the window icon + top-bar brand mark — fallible, warn-and-continue. |
| `src/audio_bridge.rs` | `EditorAudioBridge` — `AudioEngine` owner + `AudioAction` consumer | Active | Documented in `docs/architecture/audio.md` §4. 10 of 25 `AudioAction` variants are explicit no-ops. |
| `src/animation_bridge.rs` | `EditorAnimationBridge` — clip library + per-entity animation state | Active | Used by `tick_animation_subsystem` (`subsystems/audio_animation.rs:37-89`). |
| `src/console_bridge.rs` | Tracing → editor console bridge | Active | Routes `tracing` output into the in-editor `ConsolePanel`. |
| `src/runtime.rs` | `EditorRuntime` + `RuntimeState` + Play-in-Editor logic | Active | `enter_play` calls `register_mod_runtime` before `build_app`. Committed swaps are copied onto the sim app's `ScriptReplayLog`. |
| `src/scene_state.rs` | `EditorSceneState` + `TransformableScene` trait | Active | 714 lines. The canonical edit-mode `World` owner. |
| `src/entity_manager.rs` | `EditorEntity` + `EntityManager` + `SelectionSet` + `MaterialSlot` | Active | The editor's per-entity overlay (distinct from `World::Entity`). |
| `src/scene_serialization.rs` | RON-based scene save/load via `SceneData` + `EntityData` | Active | Phase 2.2 deliverable. |
| `src/level_doc.rs` | `LevelDoc` TOML schema | Active | The legacy level format (per `README.md`). Coexists with `SceneData` (see §6). |
| `src/command.rs` | `EditorCommand` trait + `UndoStack` + concrete transform/spawn/destroy commands + `TerrainUndoQueue` | Active | 2,858 lines. Audit Phase 4: 9 ops still bypass undo. |
| `src/prefab.rs` | `PrefabManager` + `PrefabData` + `PrefabInstance` | Active | Phase 4.1 deliverable. |
| `src/recent_files.rs` | `RecentFilesManager` | Active | Phase 3 deliverable. |
| `src/editor_mode.rs` | `EditorMode { Edit \| Play \| Paused }` (3-state coarse mode; transition rules at `editor_mode.rs:55-`). | Active | Phase 4.2 deliverable. |
| `src/editor_preferences.rs` | Per-user preferences persistence | Active | Phase 9 deliverable. |
| `src/clipboard.rs` | Copy/paste/duplicate clipboard | Active | Phase 3.4 deliverable. |
| `src/dialogs.rs` | Modal dialog helpers | Active | |
| `src/file_helpers.rs` | Path / IO utilities | Active | |
| `src/file_watcher.rs` | `notify`-based file system watcher for hot reload | Active | Classifies `.rhai` as `ReloadEvent::Script` via `classify_reload_path`. Mutex poison recovery uses `into_inner` (audit Phase 6). |
| `src/rhai_reload.rs` | Stages `.rhai` bytes and publishes them at the tick boundary | Active | `stage_file` uses `read_rhai_under_root`. `apply_at_tick_boundary` runs immediately before `runtime.tick` and queues each body. `EditorRuntime::run_fixed_steps` records `(tick, path, sha256, source)` only when a fixed step actually runs. |
| `src/game_project.rs` | `game.toml` project configuration | Active | |
| `src/polish.rs` | Editor polish helpers (animations, transitions, etc.) | Active | |
| `src/movement_scripts.rs` | Per-entity movement script ticker | Active | Ticked in play mode only. |
| `src/voxel_tools.rs` | Phase 10 voxel editing | Active | 845 lines. |
| `src/terrain_integration.rs` | Terrain integration glue + **the multi-biome render boundary (E3-terrain.1)** | Active | `biome_id_to_slot` (19→8 exhaustive match, `:1250-1274`) + `build_biome_slot_field` (rock overlay + 1-ring blur, `:1293-1370`) turn per-vertex `chunk.biome_ids()` into `TerrainVertex.material_ids/material_weights` (`:837-840`, `:861-867`); legacy single-biome splat path is fallback-only (`:797-802`). Site of one §7.7 trap instance (`apply_brush_at`, ZoneBlend stub). |
| `src/blend_scanner.rs` | Blend asset discovery for blueprint zones | Active | |
| `src/brdf_preview.rs` | BRDF preview thumbnail rendering | Active | |
| `src/asset_pack.rs` | Asset packaging | Active | |
| `src/distribution.rs` | Game distribution helpers | Active | |
| `src/component_ui.rs` | Component-based inspector (Phase 2.3) | Active | |
| `src/material_inspector.rs` | Material inspector (Phase PBR-G Task 2) | Active | Owns `MaterialLiveDoc`. |
| `src/plugin.rs` | `EditorPlugin` trait + `PluginManager` | Active | Phase 5.3 deliverable. |
| `src/tutorial.rs` | First-run tutorial walkthrough | Active | 344 lines. |
| `src/splash.rs` | Startup splash screen with logo + cinematic video | Active | Uses `mp4` + `openh264` deps from Cargo.toml:94-95. Video playback re-enabled 2026-07 (reverts the functional half of `1fc266c93`): decoder thread `splash-video-decode` spawns at construction; phase 0 advances to video only once the first frame is buffered (capped at `VIDEO_WAIT_CAP_SECS`); video phase ends on stream-end or the 8.5 s cap; skip-on-click preserved. |
| `src/headless.rs` | Headless testing infrastructure | Active (tests) | Exposed via `lib.rs:107`. |
| `src/interaction.rs` | Gizmo interaction helpers (auto-tracking) | Active | Phase 8.1 Week 5 Day 3 deliverable. |
| `src/telemetry.rs` | Telemetry collection (`EditorTelemetryEvent` enum: `SelectionChanged`, `GridSettingsChanged`, `GizmoStarted`, `GizmoCommitted`, `GizmoCancelled`) | Active | 615 lines. Routes events through the `tracing` crate (`use tracing::info;` at line 4). Verified — exports `EditorTelemetryEvent` enum + `EDITOR_TELEMETRY` `OnceLock` singleton. Consumed by selection/grid/gizmo paths. |
| `src/mutation_tests.rs` | Mutation-resistance test harness | Active (tests) | Test-only module. |
| `src/tests_*.rs` (asset_pack, dock_layout, game_project, polish) | Per-module test files | Active (tests) | Test-only modules; total ~440 lines. |
| `src/bin/aw_game_runtime.rs` | Standalone windowed game runner binary | Active (separate bin) | Per `Cargo.toml:122-126`. 336 lines; clap CLI; creates winit window + wgpu surface. First-commit `32eb5a26c` (2026-01-14, "Chore/root level cleanup #169"). Reuses `aw_editor_lib::game_project::GameProject`. |
| `tests/` (~80 files including `wave3_mutation_remediation.rs` at 1213 lines, `workflow_project_lifecycle.rs`) | Integration tests | Active (tests) | |
| `tests/common/mod.rs` | Shared L-series relief arithmetic (`min_castable_relief`, `prominence`, `main_grid`) | Active (test support) | NOT a test target — Cargo auto-discovers `tests/*.rs`, not `tests/*/mod.rs`. Included by `mod common;` from `l3c_relief_census.rs` and `l3a_proof.rs` so the CPU census and the GPU perf leg cannot disagree about `h_min` (L.3.C published one derived from a modelled ortho pad the renderer did not use). |
| `tests/l3_proof.rs` · `l3a_proof.rs` · `l3c_relief_census.rs` | Terrain-CSM verification harnesses (L.3 → L.3.C-resolution) | Active (tests, `#[ignore]`d GPU) | `l3_proof`: pinned station A/B (byte-identity guarantee) + min-spec perf. `l3a_proof`: camera-MOTION leg (`l3a_boundary_evidence`), CONTINUOUS per-frame leg with per-far-cascade refresh telemetry (`l3b_continuous_flight`), and min-spec survey-framing timing + emitted-fit relief census (`l3c_perf_survey`). `l3c_relief_census`: CPU-only relief distribution + layout pricing. Twin-flight differencing lives in `scripts/l3b/`; exact station diffs in `scripts/l3b/l3_station_diff.py`. |
| `benches/gizmo_benchmarks_simple.rs` + `benches/editor_performance.rs` | Criterion benchmarks | Active | Per `Cargo.toml:102-110`. |
| `ARCHITECTURAL_AUDIT_REPORT.md`, `EDITOR_ROADMAP_TO_WORLD_CLASS.md`, `INTEGRATION_ROADMAP.md`, `PHASE_*_*.md` × 7, etc. | Roadmap + audit + per-phase docs (19 `.md` files inside `tools/aw_editor/`) | Active (reference docs) | These are not source code but are load-bearing for understanding the system's evolution. |

**Status definitions:**
- **Active**: Canonical, load-bearing, edit freely with care.
- **Active (tests)**: Test-only files.
- **Active (separate bin)**: Compiles as a separate binary from `aw_editor`.
- **Active (new)**: Recently introduced (within last ~1 month).
- **Active (feature-gated)**: Compiled only when a Cargo feature is enabled.
- **Active (reference docs)**: `.md` files inside the crate that document state.

---

## 6. Conflict Map / Residue

The editor accumulates substantial historical residue. The Behavioral Correctness Audit (2026-04-04), Fix27 campaign (2026-04-05), and Multi-Tool Architecture campaign (in flight as of 2026-05-08) have documented and partially addressed many of these. The rows below are forensic — they document situations Andrew is aware of, not recommendations.

### Coexisting abstractions

| Abstraction | Files | Status | Disposition |
|---|---|---|---|
| Five representations of "the scene": `LevelDoc` (TOML) vs `SceneData` (RON) vs `World` (live) vs `EditorSceneState` (edit-mode owner) vs `EntityManager` (overlay) | `level_doc.rs` vs `scene_serialization.rs` vs `astraweave-core::World` vs `scene_state.rs:1-714` vs `entity_manager.rs` | Five coexisting representations | The legacy `LevelDoc` TOML (per `README.md`) was the original scene format. `SceneData` (Phase 2.2, RON-based) is the newer scene-save format. `EditorSceneState` is the edit-mode owner of a `World` instance. `EntityManager` overlays per-entity editor metadata (selection, material slots) on top of `World`. Audit §1.2 flagged this as "cache inconsistency risk" with circular-dep concern. |
| God-struct `EditorApp` (123 fields, 9,681-line main.rs) | `main.rs:216-?` | Acknowledged anti-pattern | Documented as the highest-priority architectural issue in `ARCHITECTURAL_AUDIT_REPORT.md:22` (55+ fields when audited 2025-12-22; verified at 123 fields on 2026-05-12 — more than doubled). Several extractions have occurred: `subsystems/` (5 helpers, commit `0de315693` 2026-04-08), `tab_viewer/` (panel state holder, commit `faf8ef439` 2026-01-13), `ui/` (menu bar etc.), but the core struct retains the bulk of state. |
| **§7.7 "wrapped-component resource identity trap" anti-pattern — four distinct instances** | (1) `viewport/engine_adapter.rs` depth target vs `aw_editor`'s `self.depth_texture` (FIXED Real-Fix.A `0f569d212`, 2026-05-07); (2) `astraweave-render::Renderer::terrain_forward.chunks` HashMap vs editor's `terrain_clusters` Vec / `self.models["terrain_cluster_*"]` (FIXED Real-Fix.B `eaaa53433`, 2026-05-07); (3) `TerrainVertex.biome_weights_0/1` vs `material_ids/material_weights` (FIXED Real-Fix.C `ded9a0457`, 2026-05-08); (4) UI material library (22 items) vs splat-build cap (8 items) vs renderer texture-array (8 slots) — Andrew-gate pending decision (d) per `EDITOR_MULTI_TOOL_ARCHITECTURE_CAMPAIGN.md` Round-8-Closure | Multi-layer architectural pattern | Elevated from "candidate" to "structural axiom" by 4-layer evidence in same campaign. The campaign doc proposes elevating this to first-class CLAUDE.md Edit. Each instance has same shape: same logical role represented in two places; bug at the boundary. |
| Per-frame mediator code in `main.rs:3833-3877` (legacy) vs `active_tool::Dispatcher` (newer canonical) | `main.rs:3833-3877` vs `active_tool/dispatcher.rs` | Mid-migration | Mediator code currently routes brush input for TerrainPanel (legacy path). Dispatcher routes the same input for the registered ActiveTool (new path). Both run currently. Mediator Removal session deferred (Andrew-gated) per Multi-Tool Architecture campaign §0. |
| **Parallel input system in `panels/input_bindings_panel.rs`** (2,511 lines, reimplements 13 input-domain types) | `panels/input_bindings_panel.rs` | Standalone, never drained | Documented at length in `docs/architecture/input.md` §6: the editor defines its own `InputBindingAction`, `InputDevice`, `BindingPreset`, `GamepadButton`, `KeyboardKey`, `MouseButton`, `ActionBinding`, `AxisBinding`, `BindingConflict`, `InputBindingsPanel`, `InputTarget`, `InputTab` types. The editor's `Cargo.toml` does not declare `astraweave-input` as a dep. The panel renders in the dock but its `pending_actions` queue is never drained externally (verified by workspace grep). |
| `EditorTabViewer` (production) vs `SimpleTabViewer` (lighter test/library variant) | `tab_viewer/mod.rs::EditorTabViewer` + `SimpleTabViewer`, both re-exported at `lib.rs:74` | Two viewers in same module | Documented at `tab_viewer/mod.rs:5-9`: "Acts as a bridge between the docking system and the actual panel implementations. It renders panel placeholders and can be extended to render actual panel content." |
| `MenuActionHandler` trait + `EditorApp` impl at `main.rs:8520` vs in-place menu logic in `ui/menu_bar.rs` | `main.rs:8520` vs `ui/menu_bar.rs` | Indirection layer (intentional) | The trait lets `MenuBar` be unit-tested without `EditorApp`. Not residue — documented architectural choice. Listed here for orientation only. |
| `use_docking: bool` field on `EditorApp` (legacy non-docked path preserved) | `main.rs` (EditorApp field) | Fallback path | Pre-docking-system layouts (floating panels) coexist with the docking system. The non-docked path is preserved for fallback but the dock path is the canonical UX. |
| Aspirational test artifacts checked into repo | `persistence/aw-save/slot*.awsv` (5 files); `astraweave-persistence-ecs/savegame.bin` | Inadvertent commits | Documented in `docs/architecture/persistence_ecs.md`. The editor doesn't own these but is the most likely producer (via `save_integration` example or `aw_save_cli`). |
| **Multiple panels with `pending_actions` queues that are never drained externally** | `input_bindings_panel.rs`, plus per Behavioral Correctness Audit: "18 stub PanelEvent handlers" | Active stub panels | The audit lists 18 specific stub handlers. This is the cross-panel manifestation of the parallel-input-system pattern. |

### Naming collisions

- **`World` (4+ meanings):** `astraweave-core::World` (grid simulation); `astraweave-ecs::World` (the ECS world inside the Play-in-Editor `App`); `WorldPanel` (an editor panel); `WorldWizard` (modal dialog). All four appear in the same `EditorApp` struct.
- **`Entity`:** `astraweave-core::Entity = u32` (live world); `EditorEntity` struct (entity_manager.rs); `EntityId = u64` (entity_manager.rs); `EntityPanel`, `EntityCatalog` (panels). `entity_id_to_world` (`main.rs:139-150`) bridges with overflow check.
- **`Snapshot`:** `EditorRuntime`'s play-stop snapshot; `astraweave-net::Snapshot` (transitively present via render deps?); various per-panel "snapshot" methods. Not in the same scope as cross-imports but confusing in cross-crate discussions.
- **`Plugin`:** `EditorPlugin` (editor's extension trait, `plugin.rs`); `astraweave_ecs::Plugin` (ECS Plugin trait used in the runtime); various external plugin systems. The names don't conflict in code but appear in different contexts.
- **`State`:** `RuntimeState` (4-state); `EditorSceneState` (the edit-mode `World` owner); `EditorMode` (a 3-state thing called `Mode` not `State`, but reads like one); per-panel `state` fields throughout. The audit's "circular dependencies risk" finding traces here.
- **`Mode`:** `EditorMode { Edit \| Play \| Paused }`; `GizmoMode { Translate \| Rotate \| Scale }`; `RenderMode` (`viewport/renderer.rs::RenderMode`); `BrushMode` (`terrain_integration.rs::BrushMode` — `Sculpt \| Flatten \| Lower \| Smooth \| Erode \| Noise \| Paint \| ZoneBlend`). All distinct.

### Known cognitive traps

- **Trap:** `EditorApp` has 123 fields including ~15 Windows-specific DWM/HWND fields, ~10 cache-invalidation generation fields, ~5 splash/tutorial/timing fields. Many fields look unused but are referenced from far-off methods in the 9,681-line `main.rs`.
  - **What's actually true:** The `#[allow(dead_code)]` on the struct itself (`main.rs:215`) means rustc cannot prove field liveness across the binary surface. The library target (via `lib.rs`) compiles without `dead_code` warnings — per the cleanup comment at `main.rs:1-8`. Searching for usage requires reading `main.rs` end-to-end.

- **Trap:** `entity_id_to_world` silently returns `None` on overflow (`main.rs:139-150`).
  - **Why it's confusing:** It logs a `warn!` but doesn't error — callers must handle the `Option`. With `EntityId: u64` and growing entity counts in stress tests, overflow into the >`u32::MAX` range is theoretically possible.
  - **What's actually true:** The bridge function is well-tested and the warn-fallback is intentional, but agents who pattern-match `entity_id_to_world(id).unwrap()` will hit a panic at the overflow boundary.

- **Trap:** `subsystems/audio_animation.rs::tick_movement_scripts` is gated on `self.runtime.is_playing()` (`subsystems/audio_animation.rs:92-96`) — movement scripts do not run in edit mode.
  - **Why it's confusing:** Scripts attached to entities in the editor will appear to do nothing until Play is hit.
  - **What's actually true:** Documented design. Edit-mode movement is via gizmos.

- **Trap:** `auto_save_to_separate_dir` and `auto_save_keep_count` fields exist on `EditorApp` (Week 7 enhanced auto-save settings), but auto-save behavior was documented as a [NEEDS VERIFICATION] in the audit Phase 6 (silent failures).
  - **What's actually true:** Auto-save fires when `auto_save_enabled` is true AND `now - last_auto_save >= auto_save_interval_secs` (`main.rs:9441-9445`). Both Week 7 enhancements are wired through: `auto_save_to_separate_dir = true` routes to `.autosave/` with timestamped filenames (`{base_name}_{YYYYMMDD_HHMMSS}.autosave.scene.ron`, `main.rs:1065-1088`); `auto_save_keep_count` rotation is enforced by `cleanup_old_autosaves` (`main.rs:1114-1170`) which sorts by mtime newest-first and removes everything past index `keep_count`.

- **Trap:** The viewport widget creation is conditional on a successful GPU device init. If wgpu init fails, `viewport: Option<ViewportWidget>` stays `None` (`main.rs:276`), and the editor runs without a 3D viewport — but most panels assume `viewport` is populated.
  - **What's actually true:** The fallback path renders panel placeholders. Audit Phase 6 flagged this as a "silent failure" pattern.

- **Trap:** The "use_docking: bool" field allows running without the docking system. Pre-docking floating-panel UX is preserved but rarely tested.
  - **What's actually true:** The non-docked path is essentially abandoned but compiles. Audit didn't specifically flag it.

- **Trap:** The Phase phase-number labeling in code comments (`Phase 1.1`, `Phase 4.2`, `Phase 5.3 T7 stage 3a`, etc.) is not monotonically increasing — it reflects multiple parallel campaigns. The "phases" cross between editor and engine work.
  - **What's actually true:** Don't try to read these as a linear timeline. Cross-reference against the per-phase docs in `tools/aw_editor/PHASE_*.md` and `docs/current/PHASE*.md`.

- **Trap:** The editor has its own WGSL shaders (`gizmo.wgsl`, `grid.wgsl`, `tonemap.wgsl`) post-Fix27. Earlier shaders (`entity.wgsl`, `shadow.wgsl`, `brdf_lut.wgsl`, `mipmap_blit.wgsl`) were DELETED by the Fix27 campaign.
  - **What's actually true:** Per `viewport/mod.rs:11-27`, the engine renderer now handles everything except editor overlays. The Behavioral Correctness Audit's CRITICAL findings VC-1, VC-2 (entity.wgsl epsilon, missing Fresnel) referenced the *deleted* editor shaders; current pipeline routes through `astraweave-render` which has its own fix-status.

- **Trap:** `Cargo.toml` declares `default = ["editor-core", "impostor-bake", "fast-alloc", "terrain-splat-arrays"]` (line 17). The `editor-full` feature exists (line 24) but is NOT in default.
  - **What's actually true:** Default builds include impostor + mimalloc + splat-arrays, but `editor-graphs`, `editor-materials`, `editor-terrain`, `editor-nav`, `editor-sim` (sub-feature flags of `editor-full`) are not enabled. **Verified 2026-05-12:** these sub-feature flags appear ONLY in `Cargo.toml` — `grep -rn "editor-graphs\|editor-materials\|editor-terrain\|editor-nav\|editor-sim\|editor-full"` across `tools/aw_editor/` returns no `.rs` file hits. No `#[cfg(feature = "editor-*")]` gates exist. They are dormant flags.

- **Trap:** The `aw_game_runtime` binary (`src/bin/aw_game_runtime.rs`) is built alongside `aw_editor` but is a separate executable. Per `Cargo.toml:122-126`: `test = false, bench = false`.
  - **What's actually true:** When users `cargo build -p aw_editor`, both binaries compile. Scope verified from `src/bin/aw_game_runtime.rs:1-15` doc comment: "standalone game runtime binary that runs games created with the AstraWeave Editor. It loads the game project configuration, initializes the engine systems, and runs the game loop." 336 lines; `clap`-based CLI with `--project`, `--dev`, `--width`, `--height`, `--fullscreen`, `--skip-splash` flags. Imports `aw_editor_lib::game_project::GameProject` (so it reuses the editor's project schema). NOT headless — it creates a winit window with wgpu surface.

### Temporary state — REVERT-BEFORE-v1 (owned by terrain beat T.3)

Four `E3-terrain (temporary)` sites were added by the E3 build (`d506658d8`) so the director could judge terrain shape unobstructed. They are deliberate, tagged in-code with that exact marker string, and **must be reverted** when terrain beat T.3 lands (director-ratified sequence, 2026-07-20 — `docs/audits/T_SERIES_RATIFICATION_2026-07-20.md`):

| Site | What it does | Revert target |
|---|---|---|
| `main.rs:5477-5478` + `main.rs:7812` | Scatter placements cleared / not uploaded ("scatter cleared so placed assets don't obstruct terrain-shape judging") | Re-enable scatter generation + upload |
| `viewport/engine_adapter.rs:1751-1754` | Fog pushed 800/1800 → 60000/120000 | Restore production fog envelope |
| `viewport/camera.rs:262-264` | Camera far plane 5000 → 40000 (whole radius-10 world visible) | Restore far 5000 (or the T.3-ratified value) |

`rg "E3-terrain \(temporary\)"` finds all four; zero hits after T.3 is the revert-complete check.

---

## 7. Decision Log

The editor has accumulated dozens of small architectural decisions over 8 months of evolution. The decisions below are the major ones; per-phase decisions are documented in the corresponding `PHASE_*.md` files inside `tools/aw_editor/`.

### Decision: Build the editor on `eframe` + `egui` + `egui_dock`
- **Date:** Initial 2025-09-09, commit `874609211` ("Add tooling enhancements: Level Editor, Asset Pipeline, and Debug Toolkit (#45)").
- **Status:** Accepted (locked in by 200+ panel `.ui()` calls).
- **Context:** AstraWeave needed a GUI editor. egui is the de-facto Rust immediate-mode UI. egui_dock layers docking on top.
- **Decision:** Use `eframe::App` as the top-level driver; `egui_dock::TabViewer` for docking; `egui` for all per-panel UI.
- **Alternatives considered:** [Reasoning not recovered from available sources]
- **Consequences:**
  - Editor inherits egui's immediate-mode programming model: panels re-render every frame.
  - Tied to egui version cadence — egui-wgpu pinned to `0.32.0`. `egui_kittest` is not a dependency.

### Decision: `EditorApp` god-struct pattern (deferred extraction)
- **Date:** Initial 2025-09-09; partial extractions to `subsystems/` in commit `0de315693` (2026-04-08, "Refactor editor subsystems: extract docking synchronization, hotkey handling, and scene statistics into separate modules"); `tab_viewer/mod.rs` first appears in commit `faf8ef439` (2026-01-13, "Merge/editor stability phase1 to main (#166)") — same Phase-1-stability merge that introduced `panel_type.rs`.
- **Status:** Accepted but flagged as anti-pattern in `ARCHITECTURAL_AUDIT_REPORT.md:22`.
- **Context:** A single struct holds all editor state. Per-frame updates touch many fields.
- **Decision:** Accept the god-struct pattern with ad-hoc per-field allocs and a giant `eframe::App::update` method.
- **Alternatives considered:** Audit §1.1 lines 61-67 lists alternatives: extract `SceneManager`, `SelectionService`, `AssetService`, `ToolService`, limit `main.rs` to UI composition. Not executed.
- **Consequences:** All editor state in one struct; many `let foo = self.foo` reborrows; per-method mutable-borrow conflicts force complex bookkeeping. Recovery is happening incrementally via `subsystems/` extractions.

### Decision: Unified rendering pipeline (Fix27 campaign)
- **Date:** Campaign plan 2026-04-05 in `docs/current/FIX27_UNIFIED_PIPELINE_CAMPAIGN.md`; executed across 7 phases over 7-9 weeks.
- **Status:** Completed (the dual-pipeline coexisting abstraction noted in §6 is RESOLVED for entity rendering).
- **Context:** The editor had two parallel rendering pipelines: `FastPreview` (editor-local shaders) + `EnginePBR` (wrapping `astraweave-render::Renderer`). They had diverged across 12 dimensions.
- **Decision:** Delete the FastPreview path. Route ALL scene rendering through `astraweave-render::Renderer`. Preserve editor overlays (grid, gizmo, physics debug, blueprint) via formal injection.
- **Alternatives considered:** Per `FIX27_UNIFIED_PIPELINE_CAMPAIGN.md`: keep both paths and reconcile shaders. Rejected as ~4,000 lines of duplicated code.
- **Consequences:** ~4,000 lines deleted. Single source of truth for PBR/shadows/IBL/post-FX. Visual parity between editor and runtime guaranteed by construction. Editor shaders `entity.wgsl`, `shadow.wgsl`, `brdf_lut.wgsl`, `mipmap_blit.wgsl` deleted. Behavioral Correctness Audit's CRITICAL findings VC-1, VC-2 (entity.wgsl issues) became obsolete in the editor (still applicable to `astraweave-render` if those issues reside there).

### Decision: Active-tool dispatcher (Multi-Tool Architecture campaign)
- **Date:** Campaign design 2026-05-04 commits `75b68e7c7` + `8fad61bd3` + `8c92890b9`. Sub-phase 2 (trait + Dispatcher core) commit `813ac29a1` 2026-05-04.
- **Status:** In progress. Sub-phase 2 COMPLETE; Sub-phase 3 mid-diagnostic (Round 8 closure 2026-05-08).
- **Context:** Brush-tool input routing was hardcoded in a per-frame mediator at `main.rs:3833-3877` for TerrainPanel. Adding new brush tools (RegionalArchetypePanel and future paint tools) required architectural debate per tool.
- **Decision:** Introduce a canonical `ActiveTool` trait + `Dispatcher` struct + UUID-based registry. Each brush tool implements the trait; dispatcher routes input events. New tool addition becomes a `dispatcher.register_tool(...)` call rather than an architectural debate. Per `EDITOR_MULTI_TOOL_ARCHITECTURE_CAMPAIGN.md` §2.5 + Andrew Q5 mod-friendliness.
- **Alternatives considered:** Per `docs/audits/editor_multi_tool_architecture_research_2026-05-03.md`: 5 approaches (research audit §7). Approach I+II hybrid selected per §7.7 synthesis (registry/manager owns trait-object collection; UUID identity for open-set extensibility; per-event dispatch on active trait-implementation).
- **Consequences:** TerrainPanel migrated (Sub-phase 3). RegionalArchetypePanel migration deferred to Sub-phase 5. Sub-phase 3 surfaced the §7.7 architectural anti-pattern at 4 distinct layers (campaign doc Round 8 closure entry).

### Decision: Two binaries (`aw_editor` GUI + `aw_game_runtime` standalone game runner)
- **Date:** 2026-01-14, commit `32eb5a26c` ("Chore/root level cleanup (#169)") — first commit introducing `tools/aw_editor/src/bin/aw_game_runtime.rs` (verified via `git log --follow --diff-filter=A`).
- **Status:** Accepted (live in Cargo.toml).
- **Context:** The editor needs to test-run authored content; users may want to ship games separate from the editor.
- **Decision:** Compile two binaries from the same crate. The game runtime gets `src/bin/aw_game_runtime.rs`; the editor is at `src/main.rs`. The runtime reuses `aw_editor_lib::game_project::GameProject` for project loading so editor and runtime stay schema-aligned.
- **Alternatives considered:** [Reasoning not recovered from available sources]
- **Consequences:** Both binaries compile on every `cargo build -p aw_editor`. Game runtime has `test = false, bench = false` so doesn't slow tests. The runtime is windowed (winit + wgpu), not headless — for headless testing the crate exposes `src/headless.rs` instead.

### Decision: `fast-alloc` (mimalloc) in default features
- **Date:** 2026-04-17 per Cargo.toml:13-16 comment ("the 2026-04-17 mimalloc experiment").
- **Status:** Accepted (in `default` array since the experiment).
- **Context:** The mimalloc experiment showed measurable allocator wins for editor workloads.
- **Decision:** Enable mimalloc globally via `astraweave_alloc::setup_global_allocator!()` at `main.rs:11-12`. Opt-out path: `--no-default-features --features editor-core,impostor-bake`.
- **Alternatives considered:** Platform default allocator (preserved as opt-out).
- **Consequences:** Default editor builds use mimalloc. Profiling for platform-specific heap-leak debug requires the opt-out path.

### Decision: `impostor-bake` (LOD3 impostors) in default features
- **Date:** Stage 3d, April 2026, per Cargo.toml:31-35 comment.
- **Status:** Accepted (in `default` array since stage 3d).
- **Context:** LOD3 scatter rendering was migrating to a new `ImpostorPass` path.
- **Decision:** Make `impostor-bake` the canonical LOD3 path; flipping it off via `--no-default-features` gives graceful LOD0/1/2 degradation.
- **Alternatives considered:** Keep the new path opt-in. Rejected — old code removed.
- **Consequences:** Editor builds always carry the impostor registry + wiring. The non-default path returns no LOD3 billboards (graceful degradation).

### Decision: Egui-version pinning (no workspace re-export)
- **Date:** Introduced in commit `7a5fcab74` (2025-11-04, "Astract/Gizmos editor implementation") — same commit that added the `astract` UI library to the editor (verified via `git log -G 'egui-wgpu' -- tools/aw_editor/Cargo.toml`).
- **Status:** Accepted (`egui-wgpu = "0.32.0"` at Cargo.toml:40 — explicit, not workspace).
- **Context:** `egui`, `egui_dock`, `eframe` come from workspace; `egui-wgpu` is pinned independently.
- **Decision:** Pin egui-wgpu directly to avoid version drift with the docking system.
- **Alternatives considered:** Workspace pin. [Reasoning not recovered from available sources why this one was pinned out-of-workspace.]
- **Consequences:** Editor's egui-wgpu can be bumped without touching workspace.

### Decision: `#[warn(dead_code)]` rather than `#[allow(dead_code)]` at crate level (with targeted allows)
- **Date:** 2026-04-03 per `main.rs:1-8` cleanup comment.
- **Status:** Accepted.
- **Context:** The library target (`lib.rs`) shares modules with the binary (`main.rs`). Items used only via the lib target appear dead in the binary build.
- **Decision:** `#![warn(dead_code)]` at `main.rs:8`; targeted `#[allow(dead_code)]` on `mod` declarations for shared modules, and per-item allows with justification comments inside `main.rs`.
- **Alternatives considered:** `#![allow(dead_code)]` at crate level. Rejected — hides real dead code.
- **Consequences:** The lib target compiles with zero `dead_code` warnings (confirming all suppressed items are exercised through tests). Bin target has explicit allows.

### Decision: Editor-specific aspirational docs (19 `.md` files inside the crate)
- **Date:** Various, 2025-09 through 2026-04.
- **Status:** Accepted as a documentation pattern.
- **Context:** The editor evolves rapidly. Roadmaps, audits, per-phase status reports accumulate.
- **Decision:** Keep these `.md` files inside the crate (alongside source). 19 in `tools/aw_editor/`; another 22+ in `docs/current/`.
- **Alternatives considered:** [Reasoning not recovered from available sources]
- **Consequences:** Documentation is plentiful but not always discoverable. Agents must search both `tools/aw_editor/*.md` and `docs/current/*EDITOR*.md` and `docs/current/AW_EDITOR_*.md`.

### Decision: AstraWeave branded preset as default theme (code-defined constants module, not `CustomColors` growth) — 2026-07-01

- **Context:** Concept-image restyle (deep navy + teal, rounded corners, branded chrome). The existing theme system (`panels/theme_manager.rs`: `EditorTheme` presets + `CustomColors` + TOML `EditorPreferences`) was the mandated extension point (never-build-a-second-implementation).
- **Decision:** Add `EditorTheme::AstraWeave` as a new first/default variant whose `Visuals` are built by `ui/palette.rs::astraweave_visuals()` from `pub const` palette constants. Do NOT grow `CustomColors` — it is the user-editable persistence model for the Custom theme (8 TOML-serialized RGB triples, each with a picker row); growing it forces serde/picker churn for preset-only needs. Precedent: `EditorTheme::high_contrast_visuals()` (code-defined preset without `CustomColors`).
- **Consequences:** Non-`Visuals` colors (overlay cards, severity chips, catalog category hues, dialog fills) reference the same palette constants, so the branded look has one source of truth. Users who explicitly saved `theme = "Dark"` keep Dark (unit-variant addition is serde-additive; covered by `test_old_schema_prefs_toml_still_loads`); only fresh defaults get AstraWeave. The 3D viewport render output is explicitly excluded (parity-critical, post-Fix27).

### Decision: 41-variant `PanelType` enum (closed registry, not extensible at runtime)
- **Date:** First-commit `faf8ef439` (2026-01-13, "Merge/editor stability phase1 to main (#166)") — `tools/aw_editor/src/panel_type.rs` first appears here, in the same merge that introduced `tab_viewer/mod.rs` and the docking architecture.
- **Status:** Accepted.
- **Context:** Each dockable panel needs a `PanelType` variant for `egui_dock` to identify it.
- **Decision:** Closed `#[non_exhaustive]` enum (`panel_type.rs:107`); 41 variants for the existing dockable panels.
- **Alternatives considered:** String-typed panel IDs (would lose compile-time exhaustiveness checking in tab dispatch); plugin-registered panel types (would require runtime dispatch table — partially implemented via `PluginManager`).
- **Consequences:** Adding a new dockable panel requires: edit `PanelType` enum, edit `panel_type.rs` helpers (`in_category`, `all`, `default_*_panels`), edit `tab_viewer/mod.rs` match arms, possibly edit `dock_panels.rs` registration. The "5 sites" coupling here is the editor analog of the persistence-ecs trace's "5 sites to add a component."

---

## 8. Known Invariants

| # | Invariant | Checkable? | Enforced by |
|---|---|---|---|
| 1 | `EntityManager.entity_id < u32::MAX` for entity IDs that need to be passed to `astraweave-core::World` | Yes (runtime) | `main.rs:139-150` `entity_id_to_world` — returns `None` and warns on overflow. |
| 2 | `RuntimeState` transitions follow the state machine at `runtime.rs:42-60` | Yes (compile-time guards + tests) | `RuntimeState::can_transition_to` method enforces; tests in `runtime.rs`. |
| 3 | Movement scripts only tick when `runtime.is_playing()` | Yes | `subsystems/audio_animation.rs:92-96` early-return. |
| 4 | `editor_mode` is updated in lock-step with `runtime_state` transitions: roughly `Edit` ↔ `RuntimeState::Editing`; `Play` ↔ `RuntimeState::{Playing, SteppingOneFrame}`; `Paused` ↔ `RuntimeState::Paused`. Naming differs between the two enums but the two transition together. | Yes (by-call-site convention; no compile-time assertion) | `main.rs:2370, 2390` set `EditorMode::Play` alongside runtime play-start; `main.rs:2402, 2451` set `EditorMode::Paused`; `main.rs:2423` sets `EditorMode::Edit` on stop. No central `debug_assert!` enforces consistency at transition points, but every transition call site updates both. |
| 5 | `UndoStack` push must happen in the same `EditorApp::update` frame as the mutation | Yes (by construction) | Commands are pushed inside event-handler arms in `tab_viewer/mod.rs` and `main.rs`. |
| 5b | Play mode records the integer 60 Hz step count of every `tick` / `step_frame` call, including zero. Replay reads `recorded_fixed_steps()` instead of re-deriving steps from wall-clock `dt`. `Instant` in this file measures frame cost only | Yes | `runtime.rs` `run_fixed_steps`; test `play_session_records_fixed_step_counts` |
| 6 | Auto-save triggered only when `auto_save_enabled` AND `now - last_auto_save >= auto_save_interval_secs` | Yes | `main.rs:9441-9445` (check inside `EditorApp::update`); fires `perform_auto_save` at `main.rs:1063-1111`. |
| 7 | `viewport: Option<ViewportWidget>` is `None` only if wgpu init failed at startup | Yes | `main.rs:276` field type. `EditorApp::new` populates it on success. |
| 8 | `egui` version across the editor matches `egui-wgpu = 0.32.0` | Yes (compile-time) | Cargo version resolution. `egui_kittest` is not linked. |
| 9 | `PanelType::all()` returns all 41 variants and `PanelType::in_category(c)` partitions cleanly | Yes (tests) | `panel_type.rs:505` `all`; `panel_type.rs:496` `in_category`. |
| 10 | `EditorAudioBridge` panel-actions are drained per frame via `tick_audio_subsystem` | Yes | `subsystems/audio_animation.rs:10-34`. |
| 11 | The 49 panels in `panels/` each implement the `Panel` trait declared in `panels/mod.rs` | Yes (compile-time) | Trait bound checked by rust compiler when each panel is constructed. |
| 12 | `EditorApp` is `eframe::App` | Yes (compile-time) | `main.rs:9119` `impl eframe::App for EditorApp`. |
| 13 | The editor binary `aw_editor` lives at `src/main.rs`; the runtime binary at `src/bin/aw_game_runtime.rs` | Yes | `Cargo.toml:116-126`. |
| 14 | `#![forbid(unsafe_code)]` is NOT declared at the crate root | Yes (compile-time) | Editor uses raw-window-handle, wgpu raw resources, and Windows-specific HWND access (`main.rs:417`); cannot forbid unsafe. |
| 15 | The `library` target (`lib.rs`) compiles with zero `dead_code` warnings | Yes (compile-time) | Per `main.rs:1-7` cleanup-comment claim; the binary target uses targeted allows. |
| 16 | Plugin events go through `PluginManager`, not direct trait calls | Yes | `plugin.rs::PluginManager`. |
| 17 | Auto-save snapshot count is bounded by `auto_save_keep_count` field | Yes (runtime, when `auto_save_to_separate_dir = true`) | `cleanup_old_autosaves` (`main.rs:1114-1170`) sorts `.autosave/` entries matching `{base}_*.autosave.scene.ron` by mtime newest-first and `fs::remove_file`'s every entry past index `auto_save_keep_count`. Only invoked when separate-dir mode is on (`main.rs:1101-1103`); same-path-overwrite mode trivially keeps 1 file. |
| 18 | The lock file (`lock_file_path` field) prevents concurrent editor instances on the same project | Yes (runtime) | `main.rs` lock-file creation + check (Week 7 Day 5 crash recovery). |
| 19 | Production-path `.unwrap()` count remains low. Audit baseline was 110+; closed by Wave-3 mutation remediation campaign (`tests/wave3_mutation_remediation.rs`). Verified 2026-05-12: **only ~12 `.unwrap()` calls remain before the first `#[cfg(test)]` boundary across `tools/aw_editor/src/`** — distributed as `mutation_tests.rs` (8, itself a test-harness module), `terrain_integration.rs` (3), `runtime.rs` (1). Zero unwraps in the 9,681-line `main.rs`. | Yes (per-commit measurable) | `grep -c '\.unwrap()' file` + `#[cfg(test)]` boundary heuristic; CI lint `unwrap_used = "allow"` per Cargo.toml:9-10 (test code uses unwrap extensively for clarity). |
| 20 | The editor accent color rides `Visuals::hyperlink_color` for ALL themes — call sites read it via `ui::palette::accent(visuals)` and never hard-code teal (or any accent) inline. | Yes (greppable) | `ui/palette.rs` routing conventions header; consumers in dock_layout, toolbar, tab_viewer, main.rs. |
| 21 | `ui::palette::astraweave_visuals()` never sets `Visuals::override_text_color` — doing so flattens the weak/faint text tiers and severity colors. (`CustomColors::to_visuals()` DOES set it; that is Custom-theme behavior, not a bug.) | Yes (unit test) | `ui/palette.rs` test `astraweave_visuals_never_overrides_text_color`. |
| 22 | Perf/FPS/memory indicator tier thresholds (16.67/33.33 ms frame budget; ≥55/≥30 FPS; <50/<80 % memory) are functional semantics, not styling — restyles may remap the colors (SUCCESS/WARNING/ERROR palette constants) but must not alter the thresholds. | Yes (guard comments at each site) | `viewport/toolbar.rs` histogram, `ui/status_bar.rs` `show_fps`/`show_resource_usage`, `main.rs` top-bar FPS. |
| 23 | The splash never blocks startup: video decode runs entirely on the `splash-video-decode` background thread; phase 0 waits for the first frame at most `VIDEO_WAIT_CAP_SECS`; phase 1 bails after 0.5 s without frames; channel disconnect degrades to logo-only; click/keypress skips at any point. | Yes (by construction) | `splash.rs` constructor + `show()` phase machine. |
| 24 | The 3D viewport render output (including its clear/background color) is parity-critical (post-Fix27, bit-identical to runtime) and is excluded from editor theming. Theme work touches egui chrome only. | Yes (by convention) | §7 Fix27 decision; 2026-07 theme decision above. |
| 25 | Dock `egui_dock::Style` is derived exclusively by `DockLayout::themed_style(&egui::Style)` and re-derived from `ctx.style()` every frame in `show`/`show_inside` — never cache or hand-build a second styled copy. | Yes (single fn, greppable) | `dock_layout.rs::themed_style`. |
| 26 | Terrain paint never writes a raw palette ID into `TerrainVertex.material_ids`: both paint call sites (`terrain_panel.rs` `apply_brush_at` + the Apply Brush button) route `selected_material` through `PaletteRemap::layer_for` and no-op when unresolved; the palette UI offers only entries the loaded pack resolves (greyed otherwise). The remap is re-resolved by `EngineRenderAdapter::reupload_terrain_layers_from_pending_pack` on every layer upload (pack swap included) — never cached across pack changes. | Yes (unit tests + greppable) | `viewport/palette_remap.rs` tests; `terrain_panel.rs::paint_layer_for_selection`; `rg 'apply_brush_paint_material'` call sites. |
| 27 | Biome-driven terrain material assignment emits **pack-layer indices directly** (`build_biome_slot_field` writes `w[biome_id_to_slot(b)] = 1.0`, `terrain_integration.rs:1313-1318`) and never consults the palette remap; the remap governs the manual-paint path only (Invariant 26). This coexistence rests on the `arrays.toml` slot order matching `biome_id_to_slot` 1:1 (`assets/materials/biomes/arrays.toml:2-3` names the function as its contract) — changing either side alone breaks biome coloring silently. | Yes (greppable) | Exhaustive match in `biome_id_to_slot` (new `BiomeId` variant = compile error); `arrays.toml` comment contract; loader test suite in `viewport/canonical_terrain_pack.rs` |
| 28 | (T.W.1) Editor water is census-gated and HDR-correct: the enable default comes from the generated world's per-vertex aquatic-biome census (`TerrainState::has_aquatic_biomes` / `ids_contain_aquatic` — Ocean/Coast/River; set in `terrain_panel` `poll_generation`, overridable via the Water checkbox); `set_water_enabled` constructs the `WaterRenderer` against `renderer.hdr_format()` (never the surface format — the pre-T.W.1 bug panicked wgpu, `TWR_WATER_RECON.md` §1.6, and `Renderer::set_water_renderer` now asserts the match at install); the level defaults to `astraweave_terrain::SEA_LEVEL` and is user-set via the panel slider. The legacy `primary_biome` string gate is gone. | Yes (unit tests + install assert) | `terrain_integration.rs` census tests (`tw1_*`); render-crate install-assert + draw-through tests; `main.rs` water sync block |
| 29 | (T.W.2) Typed entity components (Light/Camera/WaterVolume/…) persist through save/reload: EVERY production save goes through `save_scene_with_overlay` (captures the `EntityManager` overlay's `components` map) and EVERY production load through `load_scene_with_overlay` + `load_scene_from_path` (re-applies it). The bare `save_scene`/`load_scene` (overlay-dropping) have zero production callers — a caller using them silently drops component data (the pre-T.W.2 Light-loss bug; the `Ctrl+S` hotkey was the last such surface, fixed). The Carve Water brush is a terrain op that touches no water config; WaterVolume entity Y IS its water surface level. | Yes (grep + file round-trip test) | `scene_serialization` `tw2_component_overlay_round_trip` + `tw2_water_volume_persists_through_file`; Integration-Completeness grep `rg 'save_scene\('` shows only `on_save_scene`/menu wiring |
| 30 | (ED-2) **Restore is exact and stable.** `OrbitCamera` smooths four value/target pairs (`distance`/`zoom_target`, `focal_point`/`focal_point_target`, `pitch`/`pitch_target`, `yaw`/`yaw_target`) every frame at k=8-25, so ANY setter that writes a value must also write its target — `set_yaw`/`set_pitch` did not, and every F1-F12 bookmark and `ViewportCameraPreset` restore silently drifted back within ~50 ms. All four setters now sync; `fovy`/`near`/`far`/`aspect` are unsmoothed and need no target. `CameraState::capture_state`/`apply_state` is the single capture/apply surface (the F1-F12 `CameraBookmark` is an alias for it, not a second partial copy), and a round trip reproduces `view_matrix()` **bit-identically**. `aspect` is captured for provenance but deliberately NOT applied — it belongs to the current viewport size. | Yes (unit tests, proven to fail pre-fix) | `viewport/camera.rs` tests `set_yaw_survives_smoothing`, `set_pitch_survives_smoothing`, `capture_apply_round_trip_reproduces_the_view_matrix_exactly`, `restored_view_does_not_drift_when_the_smoother_runs`; `editor_preferences.rs::camera_stations_round_trip_through_a_file`; GPU test `tests/ed2_capture.rs` (0/307200 differing pixels) |
| 31 | (ED-2) **A viewport capture is the live frame, and there is one capture path.** `ViewportWidget` services a pending capture immediately after `renderer.render`, reading back the very `render_texture` egui then displays — never a re-render through another route (the failure mode that stalled T.2d's diagnosis). `request_capture` is the sole entry point: the Ctrl+Shift+C hotkey, the Camera menu item, per-station **Shot** and **Capture ALL stations** all call it, so hotkey and menu cannot diverge (the 5.C save-hotkey lesson). Every capture writes a `<name>.camera.json` sidecar carrying the full `CameraState`. | Yes (GPU test + grep) | `tests/ed2_capture.rs`; `rg 'capture_frame_png'` shows one caller |
| 32 | **`.rhai` hot reload does not publish bytes until the tick boundary, and play mode runs those bytes on the mod sandbox.** `RhaiHotReload::stage_file` only queues. `apply_at_tick_boundary` (immediately before `runtime.tick`) updates `loaded` and queues the body. `enter_play` calls `register_mod_runtime` so `build_app` installs `ModScriptPlugin`. `run_fixed_steps` writes the swap into `ScriptReplayLog` (runtime copy and the sim app resource) at `tick_count` only when `steps > 0`. `mod_script_system` applies a record whose tick equals `ModClock.tick`. A path that canonicalizes outside the watch root is rejected, including a symlink that escapes. | Yes | `rhai_reload.rs` tests `changed_file_is_visible_only_after_the_tick_boundary`, `editor_path_rejects_files_outside_the_mod_root`; `runtime.rs` test `script_swap_commits_on_the_tick_that_runs`. |
| 33 | The parity harness compares two renderers that share **one** default-HDRI bake. `editor_engine_render_parity` calls `ViewportRenderer::bake_default_environment` once and `install_baked_environment` on both the direct `Renderer` and the editor adapter. The editor path uses `init_engine_adapter_without_env_bake`, so `render_to_texture`'s `poll_env_bake` cannot install a second bake between the warm-up frame and the measurement frame. The live editor still starts the worker bake from `EngineRenderAdapter::new`. | Yes | `tools/aw_editor/tests/render_parity_harness.rs` hash assert |

---

## 9. Performance & Resource Profile

The editor is performance-sensitive: many panels render every frame, the viewport drives wgpu, the runtime can tick a full simulation. Specific profiling work is documented in `tools/aw_editor/PHASE_*` reports and `docs/current/MASTER_BENCHMARK_REPORT.md`.

### Hot paths

- **`EditorApp::update` body** (`main.rs:9120-?`) — runs once per redraw request from eframe. Long function; cost varies per-panel.
- **`EditorTabViewer::tab` dispatch** — switches on `PanelType`; each panel's `.ui()` cost dominates.
- **Viewport rendering** — `ViewportWidget::ui` → wgpu pipeline submission. Per Behavioral Correctness Audit §1.5: CPU-GPU texture readback every frame was flagged as 10-20ms overhead. **FIXED** (verified 2026-05-12): `viewport/renderer.rs:1140-1144` `read_depth_at_pixel` doc comment confirms 1-frame deferred async readback (`map_async` + `AtomicBool` flag) replaces the prior `device.poll(Wait)` blocking pattern. Fix-doc cites 0.5-2ms saved per frame (smaller than the audit's 10-20ms figure).
- **`subsystems/audio_animation.rs::tick_animation_subsystem`** — runs every frame, applies CPU skinning for entities with active skeleton animations.
- **Egui style application** — `style_applied: bool` cache guard in `EditorApp` (line 388) avoids cloning `Style` every frame.
- **Cache invalidation: entity list sync** — `entity_list_generation` vs `entity_list_synced_generation` (`main.rs:378-381`) avoids re-syncing every frame.

### Cold paths

- **Asset manifest scan** — Skipped at startup per `main.rs:429-435` (10.9 GB / 112K files would block startup). User triggers via Asset Inspector.
- **Plugin loading** — Once per plugin via `PluginManager::load`.
- **File watcher events** — Triggered by filesystem changes; debounced.
- **Save operations** — TOML/RON serialize + atomic file write.
- **Background blend decomposition** — Runs in a separate thread; messages via `DecompThreadMsg` mpsc channel.
- **Splash screen** — Plays once at startup using `mp4` + `openh264`.

### Resource ownership

- **`World`** — owned by `EditorSceneState` in edit mode; owned by `EditorRuntime` during play.
- **`EntityManager`** — owned by `EditorApp`. The editor's per-entity overlay; never directly serialized.
- **`UndoStack`** — owned by `EditorApp`. **Bounded**: `command.rs:236-251` defines `max_size: usize` field; constructor at `command.rs:259-266` takes `max_size` (recommended 50-200, default 100 per docstring); `execute()` "prune[s] old commands if over max_size" (`command.rs:275`). `UndoStackIssue` enum reports `NearCapacity` / `AtCapacity` to surface limits to the UI.
- **`PluginManager`** — owned by `EditorApp`. Holds all loaded plugin trait objects.
- **`AssetDatabase`** — owned by `EditorApp`. Loaded from `assets/assets.json` manifest.
- **`ViewportWidget`** — owned by `EditorApp.viewport: Option<ViewportWidget>`. Holds wgpu device handles + render pipelines + GPU resources.
- **Panel state** — most panels are owned by `EditorTabViewer`; some (e.g., `WorldWizard`, `PluginManagerPanel`) by `EditorApp` directly.
- **`audio_bridge` / `animation_bridge`** — owned by `EditorApp` directly. Each owns its respective engine instance.
- **Lock file** — `lock_file_path` is created at startup, deleted on graceful shutdown.

---

## 10. Testing & Validation

- **Test count**: ~9,427 `#[test]` annotations total <!-- Source: CLAIMS_REGISTRY.md#editor-test-markers --> (verified by `grep -rh "^[[:space:]]*#\[test\]"`): ~4,089 inline (`src/`) + ~5,308 in `tests/`. The CLAUDE.md figure of "3,892+ tests" (referenced in the project overview) is older and predates the Wave 2 mutation-resistant integration test campaign. Per `tools/aw_editor/TEST_COVERAGE.md`, the editor has comprehensive test coverage with explicit per-module reporting.
- **Inline `#[cfg(test)] mod tests`**: present in nearly all source files.
- **Integration tests**: 66 files in `tools/aw_editor/tests/`. Original ~6 files include `wave3_mutation_remediation.rs` (1,213 lines), `workflow_project_lifecycle.rs` (113 lines), `behavior_editor.rs`, `dialogue.rs`, `editor_scene_state.rs`, `delete_command_tests.rs`, `prefab_workflow.rs`, `play_mode.rs`, `comprehensive_smoke_tests.rs`, `grid_render.rs`, `integration_tests.rs`, `subsystem_contracts.rs`, `ui_*.rs` (3 files), `undo_transactions.rs`, `build_pipeline_integration.rs`. The bulk (~46 files) is the `wave2_*` mutation-resistant test suite (e.g. `wave2_environment_mood_fog_tests.rs` has 203 tests in one file) plus 16 `mutation_resistant_*.rs` per-subsystem files (animation, asset, audio, cinematics, command, comprehensive_tests, comprehensive_v2, entity, gizmo, interaction, lighting, material, navigation, particles, performance, physics, postprocess, prefab, runtime, terrain, tests, ui).
- **Mutation testing**: `tools/aw_editor/src/mutation_tests.rs` (test-only module) + 16+ `tests/mutation_resistant_*.rs` integration suites + `tests/wave3_mutation_remediation.rs`. Per `tools/aw_editor/AW_EDITOR_MUTATION_VULNERABILITY_REPORT.md` and `docs/current/EDITOR_MUTATION_TESTING_VULNERABILITY_REPORT.md`, the editor has dedicated mutation-resistance analysis.
- **Benchmarks**: `benches/gizmo_benchmarks_simple.rs` + `benches/editor_performance.rs` (Cargo.toml:102-110). Run via `cargo bench -p aw_editor`.
- **CI**:
  - `.github/workflows/aw-editor-tests.yml` — verified present.
  - `.github/workflows/editor-ci.yml` — verified present.
- **Headless testing infrastructure**: `src/headless.rs` (re-exported at `lib.rs:107`).
- **Headless egui frames** (`tests/ui_automation_smoke.rs`, `tests/ui_menu_interactions.rs`): paint `ViewportToolbar` and `MenuBar` on `egui::Context` and assert state. The old `egui_kittest` click-by-label cases are not built. They need AccessKit, which pulls zbus 5.
- **Audit-driven validation**: The Behavioral Correctness Audit (37 fixes from `EDITOR_BEHAVIORAL_CORRECTNESS_AUDIT.md`) and the Multi-Tool Architecture campaign (Sub-phase 3 instrumentation rounds with megabyte-scale `brush-dbg-r*.txt` log files) constitute a runtime-evidence validation regime alongside unit tests.

---

## 11. Open Questions / Parked Decisions

- **CLOSED 2026-10-16.** `editor_engine_render_parity` diverged on llvmpipe because the editor adapter's worker HDRI bake finished between the warm-up frame and the measurement frame (`poll_env_bake` inside `render_to_texture`), while the harness's direct `Renderer` never baked. The equirect sky fades to black near the horizon, so the editor LDR target gained black bands the engine frame did not. The harness now bakes once and installs that `IblResources` on both renderers, and the editor path does not start the worker. Invariant 33.

The editor accumulates open questions at a much higher rate than smaller systems. These are split into categories by source.

### From `ARCHITECTURAL_AUDIT_REPORT.md` (2025-12-22)

- **God-struct refactor.** `EditorApp` 55+ → 123 fields (verified 2026-05-12). Should this be extracted into `SceneManager`, `SelectionService`, `AssetService`, `ToolService` per audit §1.1 lines 61-67? The audit recommends; no campaign has been launched for this. Decisional — Andrew's call.
- ~~**Mutex poisoning in `file_watcher.rs`.** 5 `.lock().unwrap()` calls flagged.~~ **CLOSED 2026-05-12 by deep-investigation pass.** All 5 sites now use `.lock().unwrap_or_else(|p| p.into_inner())` for graceful poison recovery (`file_watcher.rs:363, 375, 388, 398, 424`). Resolution lives in §2 Stage 11 and §5 file_watcher row.
- ~~**110+ `unwrap()` calls in production code.**~~ **CLOSED 2026-05-12 by deep-investigation pass.** Audit baseline was 110+. Verified production-path count is now ~12 (`mutation_tests.rs:8` — itself a test-harness module; `terrain_integration.rs:3`; `runtime.rs:1`). Zero in `main.rs`. Resolution lives in §8 Invariant 19. Methodology: counted lines before the first `#[cfg(test)]` boundary per file across all 49 unwrap-containing files in `tools/aw_editor/src/`. Decisional residue: whether the 3 `terrain_integration.rs` and 1 `runtime.rs` unwraps deserve `.context()`-wrap is a separate code-quality call.

### From `EDITOR_BEHAVIORAL_CORRECTNESS_AUDIT.md` (2026-04-04)

- **9 operations bypass the undo stack** (entity creation paths). Should each be wrapped in commands? Decisional — Andrew's call. Factual context: audit's executive summary at `EDITOR_BEHAVIORAL_CORRECTNESS_AUDIT.md:29` said "6 of 15 operations" while Phase 4 detail at line 48 says "9 operations" — the "9" is the post-remediation count and is what `command.rs` Audit Phase 4 enumerates. The Wave-3 mutation remediation campaign closed several earlier ones (`command.rs:1140` spawn undo, `command.rs:1250` destroy/entity-manager undo).
- **18 stub PanelEvent handlers**. Which should be wired vs deleted? Decisional — Andrew's call. **Factual context (2026-05-12):** verified by enumerating `pending_actions` queues vs `take_*_actions` drain functions. 25 panel files in `panels/` have `pending_actions: Vec<*Action>` fields (verified by `grep -c "pending_actions:" panels/*.rs`); only 7 distinct `take_*_actions` functions are called externally (verified by `grep -rhEo "take_[a-z_]+_actions" tools/aw_editor/src/`): `take_asset_browser_actions`, `take_audio_actions`, `take_blend_import_actions`, `take_blueprint_actions`, `take_lighting_actions`, `take_pending_actions`, `take_pending_set_active_tool_actions`. ~18 panels enqueue actions that are never drained externally — consistent with the audit's count.
- ~~**Permanent mesh blacklist** in `entity_renderer.rs:174`~~ **CLOSED 2026-05-12 by deep-investigation pass.** `entity_renderer.rs` is deleted (per `viewport/types.rs:222-224` doc comment); the `failed_mesh_paths` HashSet is gone with the file. Resolution lives in §5 `viewport/types.rs` row and §6 (no longer a §6 issue).
- **Scale gizmo UP-only** per audit MEDIUM finding. Not a question (a statement); kept as a known constraint until the audit reopens it.

### From `EDITOR_MULTI_TOOL_ARCHITECTURE_CAMPAIGN.md` (Round 8 closure 2026-05-08)

- **§7.7 anti-pattern elevation.** Should the "wrapped-component resource identity trap" be elevated from candidate corollary to first-class CLAUDE.md Edit (possibly Edit 4 or 5)? Suggested ~150-word wording in §12 of the campaign doc. Andrew-gate decision pending.
- **Andrew-gate fix design decision (h) Round-8 candidate options:** Option D-1 (raise `MAX_TERRAIN_LAYERS` 8→22+), Option D-2 (canonical material library shared between UI + renderer), Option D-3 (reduce UI to 8). Decision pending.
- **Sub-phase 4-6 + Mediator Removal + Real-Fix.D/E + multiple cleanup sessions deferred.** Path forward is dependent on Andrew-gate decisions for Real-Fix.D.

### WS-MOD (2026-10-14)

- ~~**No `.rhai` hot reload.**~~ **CLOSED.** `ReloadEvent::Script` stages bytes; `RhaiHotReload::apply_at_tick_boundary` publishes them immediately before `runtime.tick`. Play mode installs `ModScriptPlugin` through `build_app`. See invariant 32.

### Editor-specific structural questions

- **Multiple `pending_actions` queues never drained.** Should orphaned action queues be drained, panels removed, or queues deleted? Decisional — Andrew's call. **Factual context (2026-05-12):** 25 panels have `pending_actions: Vec<*Action>` fields; only 7 `take_*_actions` drain functions exist in the entire crate (see "18 stub PanelEvent handlers" question above for the enumerated list). Specific cross-references: `input_bindings_panel.rs` parallel-implementation finding (`docs/architecture/input.md` §6); audit's "18 stub PanelEvent handlers" (`EDITOR_BEHAVIORAL_CORRECTNESS_AUDIT.md:22`).
- **`use_docking: bool` legacy non-docked path.** Should this be removed? Decisional — Andrew's call. **Factual context (2026-05-12):** default is `true` (`main.rs:599` with comment "Re-enabled after fixing layout gap"); legacy branch is reachable via toggle at `main.rs:8872`; used at `main.rs:9503/9508` in `EditorApp::update`. Branches call `show_legacy_left_panel` / `show_legacy_central_panel` when off. Both code paths compile and are exercised when toggled — "essentially abandoned" overstates the case.
- **`editor-full` feature flag (and sub-features `editor-graphs` / `editor-materials` / `editor-terrain` / `editor-nav` / `editor-sim`).** Should these be deleted or wired? Decisional — Andrew's call. **Factual context closed 2026-05-12:** these sub-feature flags appear ONLY in `Cargo.toml` (lines 19-24) — no `.rs` file references `#[cfg(feature = "editor-*")]` for any of them. They are dormant. The factual sub-question "whether any code checks these flags" is fully resolved (no).
- **Aspirational docs proliferation.** 20 `.md` files inside `tools/aw_editor/` (verified 2026-05-12) + 19 files in `docs/current/EDITOR_*` / `AW_EDITOR_*` (verified 2026-05-12). Should some be archived into `docs/journey/` per CLAUDE.md documentation organization rules? Decisional — Andrew's call.
- ~~**The `astract` UI library dependency** (`Cargo.toml:67`, path `../../crates/astract`). What is the scope of this library vs the rest of the editor's egui usage?~~ **CLOSED 2026-05-12 by deep-investigation pass.** `astract` is genuinely used (not just re-exported). Resolution lives in §4 upstream table; usage sites cataloged: `panels/animation.rs:11` (animation::*), `panels/charts_panel.rs:15` (charts::*), `panels/advanced_widgets_panel.rs:1` (advanced::*), `panels/entity_panel.rs:19` (prelude::*), `panels/graph_panel.rs:1` (graph::*), `panels/performance_panel.rs:5` (widgets::*), `tests/wave2_animation_easing_tests.rs:10` (animation::*). Introduced 2025-11-04 commit `7a5fcab74`.
- ~~**`telemetry.rs`** (re-exported at `lib.rs:109`). What does it collect, and where does it go?~~ **CLOSED 2026-05-12 by deep-investigation pass.** Resolution lives in §5 telemetry row. Collects 5 event types via `EditorTelemetryEvent` enum (`SelectionChanged`, `GridSettingsChanged`, `GizmoStarted`, `GizmoCommitted`, `GizmoCancelled`); routes through `tracing` (`use tracing::info;` at `telemetry.rs:4`); singleton via `OnceLock`. No remote endpoint.

### Hot-path / performance questions

- ~~**CPU-GPU texture readback every frame** (audit §1.5, 10-20ms overhead). Status post-Fix27.~~ **CLOSED 2026-05-12 by deep-investigation pass.** FIXED via 1-frame deferred async readback (`viewport/renderer.rs:1140-1232`); 0.5-2ms saved per frame. Resolution lives in §9 hot-paths.
- **`Box<dyn Interest>` per-tick allocation for `astraweave-net` viewers.** Not directly an editor question, but the editor is the canonical net consumer in development workflows; cross-reference `docs/architecture/net.md` §11.

### Settable-but-never-read surfaces (T.2d third pass, 2026-07-25)

Three found in one diagnostic session; all are Integration-Completeness §3 violations (a UI-exposed field that no downstream code reads). Full trace in `docs/audits/T2D_CAMERA_LIGHT.md` §10.1.1 and §10.6.

- ~~**The viewport shading-mode dropdown is inert.**~~ **CLOSED 2026-07-26 by ED-3.** The dropdown drives `set_debug_shading` end-to-end (scene-env `debug_mode` uniform + wireframe pipeline swap); five modes, all live; regression: `renderer.rs::ed3_debug_shading_reaches_every_shader_surface` + the GPU proof in `tests/ed3_proof.rs` (each mode >=99.8% different pixels from Lit — pre-fix all five frames were byte-identical). Wireframe is feature-gated in the UI (`toolbar.wireframe_supported`) so it can never silently no-op.
- ~~**A second, wider shading dropdown advertises two modes that do not exist.**~~ **CLOSED 2026-07-26 by ED-3.** Normals (world-space) and UVs are built; the docked dropdown maps 1:1 onto `ShadingMode`; the silent collapse in `main.rs` is deleted.
- ~~**The Terrain panel's "Base Amplitude" slider is inert.**~~ **CLOSED 2026-07-26 by ED-3.** Replaced by "Amplitude x (spline scale)" — a multiplier on the spline-derived amplitude (`NoiseConfig::base_amplitude_scale`, default 1.0 = bit-identical output; archetype splines remain the authority per E3 Phase A.2). Live-proven: three scales render pairwise ~99.5% different (`tests/ed3_proof.rs`); unit contract in `astraweave-terrain::ed3_base_amplitude_scale_is_live_and_identity_at_default`. **T.3's amplitude-finality gate is now actionable.**

### Documentation / scope questions

- **What is the relationship between the editor's `World` (`astraweave-core::World`, grid-based) and the engine's `Vec3`-based world?** The viewport renders via `astraweave-render` which is 3D-native; the editor's logical model is grid-based. The `engine_adapter.rs` is the seam. Should the editor migrate to a 3D-native model? Decisional — Andrew's call.
- **Is `aw_game_runtime` a durable ship target or a transitional developer test harness?** Decisional — Andrew's call. **Factual context closed 2026-05-12:** Per `src/bin/aw_game_runtime.rs:1-15` doc comment, scope is "standalone game runtime binary that runs games created with the AstraWeave Editor. It loads the game project configuration, initializes the engine systems, and runs the game loop." 336 lines. Clap-based CLI: `--project`, `--dev`, `--width`, `--height`, `--fullscreen`, `--skip-splash`. Imports `aw_editor_lib::game_project::GameProject` (re-uses editor's project schema). Creates winit window + wgpu device + surface (NOT headless). First-commit `32eb5a26c` (2026-01-14). Resolution lives in §4 downstream row, §5 src/bin row, §6 trap note, §7 Decision Log.

---

## 12. Maintenance Notes

**Update this doc when:**
- A new `PanelType` variant is added (§3 vocabulary, §5 file map per-panel summary, §11 panel-related questions).
- The viewport pipeline changes (§2 Stage 5, §5 viewport row, §6 dual-pipeline historical row if Fix27 is undone).
- The `ActiveTool` dispatcher gains more registered tools (§2 Stage 7, §6 mediator vs dispatcher row).
- A new `astraweave-*` crate dep is added (§4 upstream table, §1 "Where it primarily lives").
- A Cargo feature is added/removed (§1, §6 feature-flag trap row).
- A new `subsystems/` extraction happens (§5 subsystems row).
- A major audit publishes (e.g. a follow-up to `EDITOR_BEHAVIORAL_CORRECTNESS_AUDIT.md`).
- A `Phase N.M` campaign closes (linkable from §7 Decision Log).

**Verification process:**
- `find tools/aw_editor/src -name "*.rs" | wc -l` should return ≥ 216 (file count invariant grows).
- `find tools/aw_editor -name "*.rs" | xargs wc -l | tail -1` should return ≥ 224,584 (LoC invariant grows; should not shrink without a campaign documenting the deletion).
- `ls tools/aw_editor/src/panels | wc -l` should return ≥ 49 (panel count).
- `grep -c "^    [A-Z][a-zA-Z_]*," tools/aw_editor/src/panel_type.rs` should return ≥ 47 (41 PanelType + 6 PanelCategory variants).
- `cargo tree -p aw_editor --depth 1` should match the dependency list at §4.
- The structural reference docs (`docs/current/EDITOR_BEHAVIORAL_CORRECTNESS_AUDIT.md`, `docs/current/FIX27_UNIFIED_PIPELINE_CAMPAIGN.md`, `docs/current/EDITOR_MULTI_TOOL_ARCHITECTURE_CAMPAIGN.md`) should be re-checked for new chapters or status changes.
- Stamp the new commit hash and date in the metadata table after verification.

---

## Appendix A: Quick reference for agents

**If you're working on this system, remember:**

1. **The editor is massive.** Don't try to hold it all in your head. Use this trace as the structural map and dive into the relevant `panels/<X>.rs` or subdirectory only for the specific concern at hand.

2. **`EditorApp` is the god struct.** Adding state means adding a field to a 123-field struct (verified 2026-05-12). Per audit §1.1, extraction is a multi-month effort; in the meantime, follow the existing pattern.

3. **`PanelType` adding/removing has 4-5 sites to touch** (enum, helpers, dock_panels, tab_viewer match, sometimes `EditorApp` field). Use existing panels as templates.

4. **Read the audit and campaign docs BEFORE touching their respective concerns.** `EDITOR_BEHAVIORAL_CORRECTNESS_AUDIT.md` enumerates known issues. `FIX27_UNIFIED_PIPELINE_CAMPAIGN.md` documents the rendering architecture's history. `EDITOR_MULTI_TOOL_ARCHITECTURE_CAMPAIGN.md` is mid-flight as of 2026-05-08.

5. **The §7.7 trap is a thing.** Per the Multi-Tool Architecture campaign, "wrapped-component resource identity trap" has surfaced 4 times in this codebase. Watch for two locations holding the same logical state.

6. **Five "scene" representations coexist.** `LevelDoc`, `SceneData`, `World`, `EditorSceneState`, `EntityManager` are all "the scene" in different ways. Audit §1.2's cache-inconsistency-risk finding traces here.

7. **Parallel implementations of engine systems exist inside the editor.** `panels/input_bindings_panel.rs` (2,511 lines) reimplements input-domain types without importing `astraweave-input`. Verified via workspace grep.

**Files you'll most likely touch:**
- `tools/aw_editor/src/main.rs` — for `EditorApp` fields, frame loop, menu/dialog handlers.
- `tools/aw_editor/src/tab_viewer/mod.rs` — for per-panel dispatch logic.
- `tools/aw_editor/src/panels/<your-panel>.rs` — for panel-specific work.
- `tools/aw_editor/src/panel_type.rs` — when adding new dockable panels.
- `tools/aw_editor/src/viewport/` — for 3D rendering changes.
- `tools/aw_editor/src/command.rs` — when adding undo-aware operations.

**Files you should NOT touch without strong reason:**
- The 19 audit/roadmap `.md` files inside `tools/aw_editor/` — these are historical records, not editable docs (unless updating a per-phase status).
- `tools/aw_editor/src/mutation_tests.rs` — mutation-resistance assertions; changes here can mask bugs.
- The legacy mediator code at `main.rs:3833-3877` — being actively removed by the Multi-Tool Architecture campaign.

**Common mistakes when changing this system:**
- **Adding a new `PanelType` variant without updating all 4-5 coupling sites.** The compiler will catch most, but `panel_type.rs::all()` and the `tab_viewer/mod.rs` match arms need manual updates.
- **Mutating `EditorApp` fields in a panel `.ui()` without going through `EditorTabViewer`'s borrow-broker.** Will produce mutable-borrow conflicts.
- **Adding state to `EditorApp` instead of to the appropriate `EditorTabViewer` or panel struct.** The god struct grows; the audit's recommendation is the opposite direction.
- **Forgetting that `entity_id_to_world` can return `None` on overflow.** Callers must handle the Option.
- **Assuming `editor-full` is enabled in default builds.** It isn't; default features at Cargo.toml:17 list `editor-core` but not `editor-full`.
- **Forgetting that movement scripts don't run in edit mode.** `tick_movement_scripts` early-returns when not in play mode.
- **Modifying both the `astraweave-render::Renderer` AND `viewport/engine_adapter.rs` in the same change.** Per Fix27 the engine renderer is the canonical scene renderer; the adapter only wraps it. Bidirectional changes can re-introduce the dual-pipeline trap.

---

## Appendix B: Historical context

The editor's git history is dense. Key inflection points (verifiable via `git log --diff-filter=A -- tools/aw_editor/src/main.rs` and successor commits):

- **2025-09-09 commit `874609211`** — Initial creation ("Add tooling enhancements: Level Editor, Asset Pipeline, and Debug Toolkit (#45)"). The crate was born alongside `astraweave-asset` and the debug toolkit.
- **Phase 1-4 development** — Through late 2025 / early 2026, the editor grew from a level-editor TOML tool into a comprehensive scene/asset/entity authoring environment. Each phase produced one or more `.md` summary files.
- **2025-12-22 commit `3e51f652`** — `ARCHITECTURAL_AUDIT_REPORT.md` published (60 files, ~15K LoC at the time). Audit identified the god-struct anti-pattern.
- **2026-04-03** — `subsystems/` extraction work (per `main.rs:1-7` cleanup comment about reducing dead-code warnings from 738 to 0).
- **2026-04-04** — `EDITOR_BEHAVIORAL_CORRECTNESS_AUDIT.md` published (148 Rust files, 7 WGSL shaders, ~169,832 LoC at audit time — file count has grown to 216 and LoC to ~225K by `a2474c5b7`).
- **2026-04-05** — `FIX27_UNIFIED_PIPELINE_CAMPAIGN.md` published.
- **2026-04-17** — mimalloc experiment; `fast-alloc` moves to default features (per Cargo.toml:13-16).
- **2026-05-04 commits `75b68e7c7` / `8fad61bd3` / `8c92890b9` / `4556c267b` / `0a7df3cdf` / etc.** — Multi-Tool Architecture campaign launches; Sub-phase 1 diagnostic, Sub-phase 2 ActiveTool trait + Dispatcher core land in rapid succession.
- **2026-05-05 → 2026-05-08** — Sub-phase 3 Mediator Brush diagnostic rounds 1-8; "§7.7 wrapped-component resource identity trap" pattern elevates from candidate to canonical to fundamental anti-pattern to structural axiom across 4 distinct architectural layers.
- **Current state (commit `a2474c5b7`, 2026-05)** — 224,584 LoC; mid-campaign; Andrew-gate decisions pending for Real-Fix.D (Round 8 closure paint material capacity boundary fix).

The editor's evolution is unusually well-documented inside the repo. Future agents should read in this order when ramping up:
1. This trace doc (architectural shape).
2. `tools/aw_editor/ARCHITECTURAL_AUDIT_REPORT.md` (high-level architectural critique, 2025-12-22).
3. `docs/current/EDITOR_BEHAVIORAL_CORRECTNESS_AUDIT.md` (correctness-specific findings, 2026-04-04).
4. `docs/current/EDITOR_MULTI_TOOL_ARCHITECTURE_CAMPAIGN.md` Status header (current campaign state — long and dense but the source of truth for active work).
5. `docs/current/FIX27_UNIFIED_PIPELINE_CAMPAIGN.md` (rendering history).
6. The specific `tools/aw_editor/PHASE_*.md` corresponding to whichever phase contains the work the agent is touching.
