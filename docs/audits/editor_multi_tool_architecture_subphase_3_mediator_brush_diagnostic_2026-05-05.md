# Editor Multi-Tool Architecture — Sub-phase 3 Mediator Brush Diagnostic Audit

**Status**: Single confirmed root cause identified. H5 (Mediator drain logic regression) CONFIRMED at commit `f84eb09049` (2026-03-17, "imported kaykit complete asset package"). Bug: `viewport.set_terrain_brush_active(brush_active)` and `viewport.set_terrain_brush_params(brush_radius, brush_is_paint)` at `tools/aw_editor/src/main.rs:3869-3870` are placed inside `if let (Some(world), Some(viewport)) = (world_opt, self.viewport.as_mut())` gate. When user generates terrain via TerrainPanel without first loading a scene, `scene_state` is `None`, `world_opt` is `None`, and the brush state never reaches the viewport widget. All three observable symptoms (no cursor change, no ring overlay, no terrain modification on click+drag) explained by single defect. Recommended fix: small (~10 lines) — relocate the two calls outside the `Some(world)` gate. H1-H4 + H6-H8 REFUTED or non-causal secondary observations.

**Author**: Editor Multi-Tool Architecture Sub-phase 3 Mediator Brush Diagnostic session, 2026-05-05.

**Scope**: Failure-diagnosis of TerrainPanel brush mediator path regression. Triggered by Sub-phase 3 Andrew-gate REGRESS 2026-05-04. Defect predates Sub-phase 3 per Andrew Q1 verification (verified at commit `79e483e6c` Sub-phase 2 Core.E pre-Sub-phase 3 state). DIAGNOSTIC ONLY per Andrew Q2 — no production code changes; fix prompt drafted separately.

**Predecessors**:
- `docs/current/EDITOR_MULTI_TOOL_ARCHITECTURE_CAMPAIGN.md` — campaign context.
- `docs/audits/editor_multi_tool_architecture_diagnostic_2026-05-04.md` — Sub-phase 1 Diagnostic audit (mediator path extent + integration targets + methodology lessons).
- `docs/audits/g_pointer_events_diagnostic_2026-05-03.md` — single-reference for mediator path code locations + AstraWeave classification (approach (B) with main.rs mediator).
- `docs/audits/f5_paint_panel_registration_diagnostic_2026-05-03.md` — failure-diagnosis hypothesis-classification structure precedent + methodology revision pattern.
- `docs/current/REGIONAL_ARCHETYPE_VARIATION_CAMPAIGN.md` §10 — F.5-paint cascade commit list (relevant for hypothesis 1 refutation evidence).

---

## §1 — Symptom Inventory

### §1.1 Andrew's observations 2026-05-04

Per Sub-phase 3 Andrew-gate REGRESS verdict and Sub-phase 3 Mediator Brush Diagnostic prompt §0.3:

1. **No cursor change on brush activation.** Click "🔴 Brush Active" toggle in TerrainPanel; mouse cursor over viewport remains default (no brush-cursor change).
2. **No ring overlay on terrain hover.** Hover over generated terrain with brush active; no orange ring overlay drawn at cursor position.
3. **No terrain modification on click+drag.** Click+drag in viewport with brush active over terrain; no height delta applied; no texture paint applied.
4. **No panic; runtime healthy.** Editor compiles + runs; no crashes. Silent failure mode.
5. **FPS spike on paint mode switch.** Switching to BrushMode::Paint causes one-time FPS dip; no visible brush feedback follows.
6. **All 8 brush modes affected.** Sculpt, paint, lower, smooth, flatten, erode, noise, zoneblend — none exhibit visible brush behavior.
7. **Pre-Sub-phase 3 also broken.** Per Andrew Q1, checkout pre-Sub-phase 3 (commit `79e483e6c`) reproduces same failure mode. Defect predates Sub-phase 3.
8. **Sculpt + flatten worked at one point in past.** Per Andrew Q4. Temporal anchor — brush worked sometime previously; uncertain when last functional.

### §1.2 Last-known-working anchor

Per audit §3 investigation: working state corresponds to a brush-active session where `EditorApp.scene_state` is `Some(_)` (i.e., a scene/world is loaded). Bug is path-conditional, not commit-conditional after `f84eb09049` (2026-03-17). Specifically:

- **Pre-`f84eb09049`** (before 2026-03-17): the entire terrain brush mediator path didn't exist in main.rs (`set_terrain_brush_active` and `take_terrain_brush_hits` had no call sites; `terrain_brush_active` field on ViewportWidget didn't exist). Brush was non-functional but not regressed — the feature was unfinished.
- **`f84eb09049` and later** (2026-03-17 onward): brush mediator wired up but with the gating bug. Brush worked **only** when scene was loaded; failed silently when terrain was generated standalone via TerrainPanel without a scene.

Andrew's "scult and flatten worked at one point" memory most likely corresponds to a session where a scene was loaded (e.g., via "Open Scene…" or "Create Scene…"), causing the gate to pass.

### §1.3 Specific symptom-code mapping

For each symptom:

**Symptom 1 (no cursor change) + Symptom 2 (no ring overlay):**
- Code path: `tools/aw_editor/src/viewport/widget.rs:1428` — `if self.terrain_brush_active && !self.gizmo_state.is_active()`. Cursor + ring rendering gated by `self.terrain_brush_active == true`.
- Why not firing: `self.terrain_brush_active` is `false` because `set_terrain_brush_active(true)` is never called. See §4.5.

**Symptom 3 (no terrain modification on click+drag):**
- Code path: `tools/aw_editor/src/viewport/widget.rs:1365-1419` — `if self.terrain_brush_active && !self.gizmo_state.is_active() && (response.dragged_by(Primary) || response.clicked_by(Primary))`. Hit collection gated identically.
- Why not firing: same gate. `terrain_brush_hits` Vec stays empty. `take_terrain_brush_hits` at `tools/aw_editor/src/main.rs:3886` returns empty Vec; `apply_terrain_brush_at` never called; terrain unmodified.

**Symptom 4 (no panic):**
- Consistent with silent-gate failure mode. Code paths exist; gate condition is simply false; control flow falls through.

**Symptom 5 (FPS spike on paint mode switch):**
- Code path: `tools/aw_editor/src/panels/terrain_panel.rs:1257` — `self.ensure_thumbnails_loaded(ui.ctx())` called each frame in paint mode UI rendering (gated by `if self.brush_mode == BrushMode::Paint`).
- Method body at `terrain_panel.rs:1936-1967`: `if self.thumbnails_loaded { return; }` — first-call loads 22 material PNG thumbnails at 64×64 + uploads textures via `ctx.load_texture`. Subsequent calls no-op.
- Classification: secondary observation, NOT regression cause. First-time texture upload causes one-time FPS dip; doesn't affect brush mediator path. See §4.7.

**Symptom 6 (all 8 modes affected):**
- All 8 modes flow through identical mediator path (UI toggle → brush_enabled flip → is_brush_active accessor → main.rs:3858 `brush_active` value → main.rs:3869 set on viewport ← bug here ← all 8 fail identically).

**Symptom 8 (worked at one point):**
- Consistent with path-conditional gate. Sessions where scene_state was Some passed the gate; sessions without a loaded scene fail silently.

---

## §2 — Methodology

### §2.1 Hypothesis-classification structure

Per Sub-phase 1 Diagnostic audit §13.2 methodology lesson: **failure-diagnosis audits enumerate hypotheses; collect evidence per hypothesis; identify which hypothesis is confirmed**. Precedent: F.5-paint.E-diagnostic + G-pointer-events-diagnostic.

### §2.2 Inspection scope

Per single-reference-points discipline (Sub-phase 1 Diagnostic):

- `tools/aw_editor/src/main.rs:3854-3902` — actual mediator region (not 3833-3877 as the prompt's stale quote indicated; line drift since G-diagnostic).
- `tools/aw_editor/src/viewport/widget.rs:163-177` — ViewportWidget per-tool fields.
- `tools/aw_editor/src/viewport/widget.rs:1029-1467` — handle_input including terrain brush branches at 1365 + 1423 + 1428.
- `tools/aw_editor/src/panels/terrain_panel.rs:814-901` — brush state machine (is_brush_active + apply_brush_at + end_brush_stroke + brush_mode_name + brush_radius + is_paint_mode).
- `tools/aw_editor/src/panels/terrain_panel.rs:1184-1349` — show_brush_section UI (brush_enabled toggle + mode picker + paint mode thumbnail grid).
- `tools/aw_editor/src/tab_viewer/mod.rs:1352-1453` — per-tool accessors.

### §2.3 Git-history-grounded investigation

`git log --all -- <mediator-file>` for each of the four mediator files; classification of each touching commit by file-section-modified relevance to brush flow; targeted `git blame -L` on the gating block.

### §2.4 Per-hypothesis verification mode

Each hypothesis assessed by code citation + git log entry + behavior derivation. Working state is path-conditional per §1.2; runtime checkout-and-verify of historical states deferred (Andrew can confirm working-with-scene path post-fix).

---

## §3 — Last-Known-Working Anchor Identification

### §3.1 Best-guess: F.4 closeout

Per prompt §3.1, F.4 closeout (commit `a64f12320`, 2026-05-02 timeframe per Phase 1.X-pause.A) was the prime candidate.

**Verdict**: working state of F.4 closeout is **path-conditional**, not commit-conditional. The mediator-path bug exists at F.4 closeout commit (introduced 7 weeks earlier at `f84eb09049` 2026-03-17). Whether F.4 closeout exhibits the bug depends on whether a scene is loaded during the brush test:

- F.4 closeout + scene loaded: brush works.
- F.4 closeout + no scene loaded: brush fails identically to current HEAD.

Andrew's Q1 verification (pre-Sub-phase 3 also broken at `79e483e6c`) is consistent with the without-scene path being tested.

### §3.2 Walk-forward investigation outcome

Walk-forward from `f84eb09049` (the candidate regression-introducing commit) through subsequent commits modifying mediator-relevant files:

| Commit | Files modified | Mediator-path relevance |
|--------|---------------|-------------------------|
| `f84eb09049` | main.rs + viewport/widget.rs (initial wire-up) | **CAUSAL** — introduced gating bug |
| `67454011f` | main.rs + viewport/widget.rs | Modified viewport but not gating; non-causal |
| `4c6119643` | main.rs + viewport/widget.rs + viewport/renderer.rs | Modified depth pick + brush; non-causal (gating unchanged) |
| `c76782ecf` (mimalloc) | main.rs | Allocator change; non-causal |
| `dbde39642` (UI consistency refactor) | viewport/widget.rs + viewport panels | UI refactor; non-causal |
| `0de315693` (subsystems extract) | main.rs + viewport/widget.rs | Subsystems extraction; non-causal (gate preserved) |
| `8cfd05fb2` (terrain splat-array) | main.rs + viewport/renderer.rs + tab_viewer/mod.rs | Terrain material pipeline; non-causal (gate preserved) |
| `b2df0be20` (F.5-paint.F-fix.A) | tab_viewer/mod.rs + panel_type.rs | RegionalArchetype panel registration; non-causal |
| `0dea0bebc` (Sub-phase 3.A) | main.rs + panels/terrain_panel.rs | Additive ActiveTool integration; non-causal (preserved mediator) |
| `41ec3b192` (Sub-phase 3.B) | main.rs + viewport/widget.rs + tab_viewer/mod.rs + panels/terrain_panel.rs | Cached-then-dispatch wiring; non-causal (preserved mediator) |

Result: single regression-introducing commit identified at `f84eb09049`. All subsequent commits preserved the bug without modification.

---

## §4 — Hypothesis Enumeration + Evidence Collection

### §4.1 Hypothesis 1 — F.5-paint cascade introduced regression

**Status: REFUTED.**

Evidence — F.5-paint cascade commits and their effect on mediator-relevant files:

| Commit | F.5-paint sub-phase | Files modified | Mediator-path effect |
|--------|---------------------|----------------|----------------------|
| `26a3864b8` | F.5-paint.A | `panels/mod.rs` + `panels/regional_archetype_panel.rs` (new) | **None** — no main.rs / viewport/widget.rs / terrain_panel.rs / tab_viewer/mod.rs changes |
| `226572bae` | F.5-paint.B | `panels/regional_archetype_panel.rs` + `Cargo.lock` | **None** |
| `2b230d94e` | F.5-paint.C | `panels/regional_archetype_panel.rs` | **None** |
| `e9d2a7922` | F.5-paint.D | `REGIONAL_ARCHETYPE_VARIATION_CAMPAIGN.md` | **None** (doc-only) |
| `b6dd9de58` | F.5-paint.D fixup | (hash-fixup) | **None** |
| `5f772bea3` | F.5-paint.E-diagnostic.A | (audit doc) | **None** |
| `e561d4dce` | F.5-paint.E-diagnostic.B | (campaign doc) | **None** |
| `b2df0be20` | F.5-paint.F-fix.A | `panel_type.rs` + `tab_viewer/mod.rs` | **Panel registration only** — added RegionalArchetype rows; NO terrain mediator path modifications |
| `722b70ae5` | F.5-paint.F-fix.A-supplement | (exhaustive-match dispatch coverage) | **None to terrain mediator** |
| `dee94ea05` | F.5-paint.F-fix.B | (audit amendment + closeout) | **None** |

F.5-paint cascade introduced no modifications to the four mediator-relevant code paths (main.rs:3854-3902 + viewport/widget.rs:163-1467 brush sections + terrain_panel.rs brush state machine + tab_viewer/mod.rs brush accessors). The prime-suspect framing in the prompt §1.4 was based on incorrect causal attribution; the actual regression-introducing commit predates F.5-paint by ~7 weeks.

### §4.2 Hypothesis 2 — Pre-F.5-paint regression in earlier campaign

**Status: CONFIRMED at unexpected granularity.**

The regression was introduced at commit `f84eb09049` (2026-03-17) — predates F.5-paint cascade by ~7 weeks. The commit message ("imported kaykit complete asset package") obscures that this commit also introduced the entire terrain brush mediator wire-up — and did so with the gating bug from inception. This is not a regression in the conventional sense (working code → broken code); it's a defective initial implementation that has never been functional in the no-scene-loaded path.

Evidence — `git log -S "set_terrain_brush_active" -- tools/aw_editor/src/main.rs`:
```
f84eb0904 imported kaykit complete asset package
```

Single matching commit. Prior to `f84eb09049`, neither `set_terrain_brush_active` nor `terrain_brush_hits` nor `is_terrain_brush_active` appear in main.rs. The brush feature didn't exist.

Also REFUTED for the more specific "pre-F.5-paint working state" interpretation: there is no working state of the no-scene-loaded path in any commit on main; the bug is co-temporal with the feature's existence.

### §4.3 Hypothesis 3 — TerrainPanel brush state machine regression

**Status: REFUTED.**

TerrainPanel brush state machine code at `panels/terrain_panel.rs:814-901` is internally correct:

- Line 815: `pub fn is_brush_active(&self) -> bool { self.brush_enabled && self.terrain_state.has_terrain() }`. Conjunction of `brush_enabled` UI flag + terrain presence. Both correctly populated.
- Line 1197: `self.brush_enabled = !self.brush_enabled;` toggles correctly when user clicks "🔴 Brush Active" / "⚪ Brush Inactive" button at line 1188-1209.
- Line 836: `pub fn apply_brush_at(&mut self, world_x: f32, world_z: f32)` correctly routes by `brush_mode` to `apply_brush_paint_material` (Paint mode) or `apply_brush` (other 7 modes).
- Line 881: `pub fn end_brush_stroke(...)` correctly delegates to `terrain_state.end_stroke()`.

Behavior derivation: when user clicks "🔴 Brush Active" toggle, `brush_enabled` becomes `true`; if terrain has been generated, `is_brush_active()` returns `true`; main.rs:3858 reads this correctly. The downstream defect is in main.rs's handling of the returned `brush_active` value, not in TerrainPanel.

### §4.4 Hypothesis 4 — ViewportWidget input-flow regression

**Status: REFUTED.**

ViewportWidget input-flow code at `viewport/widget.rs:1029-1467` is internally correct:

- Line 595: `let (rect, response) = ui.allocate_exact_size(viewport_size, egui::Sense::click_and_drag());` — correct sense for click+drag detection.
- Line 1365-1419: terrain brush hit collection correctly gated by `self.terrain_brush_active`; correctly uses `response.dragged_by(Primary)` + `response.clicked_by(Primary)`; correct depth-buffer pick + Y=0 plane fallback.
- Line 1423-1425: stroke-end detection correctly gated by `self.terrain_brush_active`; correctly uses `response.drag_stopped_by(Primary)`.
- Line 1428-1467: cursor + ring overlay rendering correctly gated; same depth-buffer + fallback pattern.
- Line 398-403: `set_terrain_brush_active` and `set_terrain_brush_params` setters correctly mutate fields.
- Line 409-416: `take_terrain_brush_hits` and `take_terrain_brush_stroke_ended` correctly use `mem::take`.

The widget is structurally correct. The defect is upstream — main.rs never tells the widget about the brush state.

### §4.5 Hypothesis 5 — Mediator drain logic regression

**Status: CONFIRMED. SINGLE ROOT CAUSE.**

Defect location: `tools/aw_editor/src/main.rs:3867-3879`.

```rust
3854:                // Get mutable world from scene state
3855:                let world_opt = self.scene_state.as_mut().map(|s| s.world_mut());
3856:
3857:                // Check terrain brush state before borrowing dock_tab_viewer mutably
3858:                let brush_active = self.dock_tab_viewer.is_terrain_brush_active();
3859:                let brush_radius = self.dock_tab_viewer.terrain_brush_radius();
3860:                let brush_is_paint = self.dock_tab_viewer.terrain_brush_is_paint();
...
3867:                if let (Some(world), Some(viewport)) = (world_opt, self.viewport.as_mut()) {
3868:                    // Sync terrain brush state: tell viewport when brush is active
3869:                    viewport.set_terrain_brush_active(brush_active);
3870:                    viewport.set_terrain_brush_params(brush_radius, brush_is_paint);
3871:
3872:                    context = context
3873:                        .with_viewport(viewport)
3874:                        .with_world(world)
3875:                        .with_entity_manager(&mut self.entity_manager)
3876:                        .with_undo_stack(&mut self.undo_stack)
3877:                        .with_prefab_manager(&mut self.prefab_manager)
3878:                        .with_viewport_layout(viewport_layout);
3879:                }
```

**Mechanism**: lines 3869-3870 (the only call sites for `viewport.set_terrain_brush_active` and `viewport.set_terrain_brush_params` in the entire codebase per `Grep` evidence below) are **inside** an `if let (Some(world), Some(viewport)) = (world_opt, self.viewport.as_mut())` block. The `Some(world)` arm requires `world_opt` to be `Some`, which requires `self.scene_state` to be `Some`.

`scene_state` is the canonical edit-mode world owner. It is populated when:
- User opens a scene via "File > Open Scene…".
- User creates a new scene via "File > Create Scene…" or World Wizard.
- An autosave or recent-files load fires.

`scene_state` is **NOT** populated by:
- TerrainPanel's terrain generation (which writes to `terrain_panel.terrain_state`, an entirely separate state owner).
- Toggling the brush.
- Default editor startup.

**Path that fails**: User opens editor → defaults to no scene → navigates to Terrain panel → generates terrain (terrain_state populated; scene_state still None) → toggles "🔴 Brush Active" (brush_enabled flips true; is_brush_active returns true since both brush_enabled AND terrain_state.has_terrain() are true; main.rs:3858 captures `brush_active = true`). At line 3867, `world_opt` is `None` (because scene_state is None), so the `if let` arm DOES NOT execute. Lines 3869-3870 are skipped. `viewport.terrain_brush_active` remains its default `false`. All three brush-active gates in viewport/widget.rs (lines 1365, 1423, 1428) evaluate to false. Brush is silently inert.

**Path that works**: User opens editor → loads/creates a scene (scene_state becomes Some) → generates terrain → toggles brush. At line 3867, both `world_opt` and `self.viewport` are `Some`, the `if let` arm executes, lines 3869-3870 fire, viewport's `terrain_brush_active` becomes true, all three gates fire, brush works. This explains Andrew's Q4 "scult and flatten worked at one point" memory.

**Grep evidence — single call site for the setters**:
```
tools/aw_editor/src/main.rs:3869:                    viewport.set_terrain_brush_active(brush_active);
tools/aw_editor/src/main.rs:3870:                    viewport.set_terrain_brush_params(brush_radius, brush_is_paint);
tools/aw_editor/src/viewport/widget.rs:398:    pub fn set_terrain_brush_active(&mut self, active: bool) {
tools/aw_editor/src/viewport/widget.rs:403:    pub fn set_terrain_brush_params(&mut self, radius: f32, is_paint: bool) {
```

No alternative sync path exists. Lines 3869-3870 are the unique gateway from UI brush state to viewport-side brush activation.

**Note**: the **hit drain** at main.rs:3884-3902 is correctly placed in a separate `if let Some(viewport) = self.viewport.as_mut()` block that does NOT require a world. So the drain side of the mediator is correct; the **state push** side is the defect.

**Introducing commit**: `f84eb09049` (2026-03-17, "imported kaykit complete asset package"). Per `git blame -L 3855,3879 tools/aw_editor/src/main.rs`:

```
dd20d9a78b (2026-01-16) 3855)  let world_opt = self.scene_state.as_mut().map(|s| s.world_mut());
f84eb09049 (2026-03-17) 3858)  let brush_active = self.dock_tab_viewer.is_terrain_brush_active();
4c6119643  (2026-03-24) 3859)  let brush_radius = self.dock_tab_viewer.terrain_brush_radius();
4c6119643  (2026-03-24) 3860)  let brush_is_paint = self.dock_tab_viewer.terrain_brush_is_paint();
dd20d9a78b (2026-01-16) 3867)  if let (Some(world), Some(viewport)) = (world_opt, self.viewport.as_mut()) {
f84eb09049 (2026-03-17) 3869)      viewport.set_terrain_brush_active(brush_active);
4c6119643  (2026-03-24) 3870)      viewport.set_terrain_brush_params(brush_radius, brush_is_paint);
```

The `if let (Some(world), Some(viewport))` gate at line 3867 was introduced earlier by `dd20d9a78b` (2026-01-16) for the original purpose of populating `EditorDrawContext` (which legitimately requires both world and viewport for gizmo + entity rendering). Commit `f84eb09049` (2026-03-17) added the brush sync wiring at line 3869 inside that pre-existing gate without recognizing that brush state is independent of world presence. Commit `4c6119643` (2026-03-24) extended with `set_terrain_brush_params` at line 3870, also inside the gate.

**Recommended fix scope (small, ~10 lines)**: relocate lines 3869-3870 outside the `Some(world)` gate, into a separate `if let Some(viewport) = self.viewport.as_mut()` block that doesn't require a world. Single-commit fix; no architectural changes; no API changes; preserves the gate's legitimate purpose for context population.

Sketch (NOT for application — fix prompt drafted separately per Andrew Q2):
```rust
// Sync terrain brush state to viewport (independent of world presence)
if let Some(viewport) = self.viewport.as_mut() {
    viewport.set_terrain_brush_active(brush_active);
    viewport.set_terrain_brush_params(brush_radius, brush_is_paint);
}

// EditorDrawContext requires both world and viewport
if let (Some(world), Some(viewport)) = (world_opt, self.viewport.as_mut()) {
    context = context
        .with_viewport(viewport)
        .with_world(world)
        .with_entity_manager(&mut self.entity_manager)
        .with_undo_stack(&mut self.undo_stack)
        .with_prefab_manager(&mut self.prefab_manager)
        .with_viewport_layout(viewport_layout);
}
```

Two `&mut self.viewport.as_mut()` borrows in the same closure are non-overlapping (sequential), so borrow checker accepts this pattern.

### §4.6 Hypothesis 6 — tab_viewer accessor regression

**Status: REFUTED.**

tab_viewer accessors at `tab_viewer/mod.rs:1352-1453`:

```rust
1352: pub fn is_terrain_brush_active(&self) -> bool {
1353:     self.terrain_panel.is_brush_active()
1354: }
...
1420: pub fn apply_terrain_brush_at(&mut self, world_x: f32, world_z: f32) {
1421:     self.terrain_panel.apply_brush_at(world_x, world_z);
1422: }
...
1438: pub fn terrain_brush_radius(&self) -> f32 {
1439:     self.terrain_panel.brush_radius()
1440: }
...
1443: pub fn terrain_brush_is_paint(&self) -> bool {
1444:     self.terrain_panel.is_paint_mode()
1445: }
```

Each accessor correctly delegates to `self.terrain_panel.<method>()`. The `terrain_panel: TerrainPanel` field at `tab_viewer/mod.rs:665` is a single owned instance; the same instance is rendered in the dock at `tab_viewer/mod.rs:7647` (`PanelType::Terrain => { ... self.terrain_panel.show(ui); ... }`). No instance-mismatch issue. When user toggles brush on the rendered panel, the `brush_enabled` field flips on this single instance; the accessor reads correctly.

### §4.7 Hypothesis 7 — FPS-spike-anchored hypothesis (paint mode)

**Status: PARTIAL CONFIRMATION as secondary observation. NOT regression cause.**

Source of FPS spike: `tools/aw_editor/src/panels/terrain_panel.rs:1257`:

```rust
1256: if self.brush_mode == BrushMode::Paint {
1257:     self.ensure_thumbnails_loaded(ui.ctx());
1258:     ...
```

`ensure_thumbnails_loaded` at lines 1936-1967 loads 22 material PNG thumbnails via `image::open(&path)`, resizes each to 64×64 via `image::imageops::FilterType::Triangle`, converts to `egui::ColorImage`, uploads to GPU via `ctx.load_texture`. First call is expensive (file IO + image decode + GPU upload × 22); subsequent calls are gated by `if self.thumbnails_loaded { return; }` and become no-ops.

Behavior derivation: user switches to BrushMode::Paint → next frame `show_brush_section` enters paint-mode branch at line 1256 → `ensure_thumbnails_loaded` runs first-time path → FPS spike for ~one frame → `thumbnails_loaded = true` → subsequent frames are cheap.

**This is NOT the brush regression cause.** It is an independent UX quirk: the paint-mode UI defers thumbnail loading until first display rather than loading at panel construction. Splitting the FPS cost across program lifetime (one-time vs at startup) is a defensible design choice; the spike is bounded to ~one frame.

The FPS spike clue **was a red herring** for the brush regression but a valid secondary observation for forward documentation. Anti-drift discipline §1.5 temptation 4 ("optimize the spike-causing code") explicitly resisted — fixing the FPS spike is out of scope for this diagnostic.

### §4.8 Hypothesis 8 — Build-system / dependency regression

**Status: REFUTED.**

No `Cargo.toml`, `Cargo.lock`, or `cfg`-flag changes in commits modifying mediator-relevant files affect the brush mediator path. `c76782ecf` introduced mimalloc as an optional global allocator but doesn't modify mediator code. No conditional compilation gates exist on `set_terrain_brush_active`, `take_terrain_brush_hits`, or any of the brush state machine methods.

Verified via `Grep` for `#[cfg` and `cfg!` in mediator files: no relevant occurrences.

### §4.9 Additional hypotheses

None surfaced during investigation. The §4.5 confirmation explained all observable symptoms (1, 2, 3, 4, 6, 8) parsimoniously; the FPS spike (symptom 5) was an independent secondary observation.

---

## §5 — Evidence Convergence + Root Cause Identification

### §5.1 Hypothesis confirmation pattern

| Hypothesis | Status | Evidence |
|------------|--------|----------|
| H1 — F.5-paint cascade | REFUTED | F.5-paint commits did not modify mediator-relevant files; `b2df0be20` (F-fix.A) only touched panel registration plumbing |
| H2 — Pre-F.5-paint regression | CONFIRMED at unexpected granularity (initial-implementation defect at `f84eb09049` 2026-03-17) |
| H3 — TerrainPanel state machine | REFUTED | Internal logic correct; brush_enabled toggles correctly; is_brush_active returns correct value |
| H4 — ViewportWidget input flow | REFUTED | handle_input branches correct; depth-buffer pick correct; default field values correct |
| H5 — Mediator drain logic | **CONFIRMED — SINGLE ROOT CAUSE** | Lines 3869-3870 inside `Some(world)` gate; scene-independence violation |
| H6 — tab_viewer accessor | REFUTED | Single-instance ownership; correct delegation |
| H7 — FPS spike (paint mode) | Secondary observation | One-time thumbnail upload; not regression cause |
| H8 — Build-system | REFUTED | No relevant Cargo or cfg changes |

Convergence: **single confirmed hypothesis with clear evidence — root cause identified**.

### §5.2 Confirmed root cause

**Defect**: `viewport.set_terrain_brush_active(brush_active)` and `viewport.set_terrain_brush_params(brush_radius, brush_is_paint)` are placed inside an `if let (Some(world), Some(viewport)) = (world_opt, self.viewport.as_mut())` gate. The brush sync should be independent of world presence (terrain generation populates `terrain_panel.terrain_state`, not `scene_state.world`).

**Code location**: `tools/aw_editor/src/main.rs:3867-3879` (gate), specifically lines 3869-3870 (the misplaced calls).

**Introducing commit**: `f84eb09049` (2026-03-17, "imported kaykit complete asset package"). Commit `4c6119643` (2026-03-24) extended with `set_terrain_brush_params` inside the same gate.

**Mechanism**: `scene_state` is None when no scene is loaded; `world_opt = scene_state.as_mut().map(|s| s.world_mut())` is None; the `Some(world)` arm of the `if let` doesn't execute; lines 3869-3870 are skipped; `viewport.terrain_brush_active` remains `false` regardless of UI toggle state; all three brush-active gates in `viewport/widget.rs` (lines 1365, 1423, 1428) are silently false; brush is inert.

**Why all 3 symptoms explained by one defect**:
- Symptom 1 (no cursor) ← line 1428 gate
- Symptom 2 (no ring overlay) ← same line 1428 gate (cursor + ring share render block)
- Symptom 3 (no terrain modification) ← line 1365 gate (hit collection)

All three gates check `self.terrain_brush_active`, which is `false` due to the misplaced setter call.

**Recommended fix scope**: SMALL. ~10 lines. Single commit. No architectural changes; no API changes; no test changes (existing tests don't cover the gating path because they're internal to TerrainPanel + ViewportWidget; integration test for the mediator path would catch this).

### §5.3 Recommended fix prompt scope

Per Sub-phase 3 Mediator Brush Diagnostic prompt §0.5 outcome 1 (single root cause + small fix):

**Files to modify**: `tools/aw_editor/src/main.rs` only (lines 3867-3879).

**Changes**:
1. Extract `viewport.set_terrain_brush_active(brush_active)` + `viewport.set_terrain_brush_params(brush_radius, brush_is_paint)` from the `if let (Some(world), Some(viewport))` block.
2. Place them in a preceding `if let Some(viewport) = self.viewport.as_mut()` block that doesn't require a world.
3. Verify the second `if let` block still works (sequential `&mut` borrows are non-overlapping; borrow checker accepts).

**Verification post-fix**:
- Re-run Sub-phase 3 Andrew-gate. Andrew tests all 8 brush modes (sculpt/paint/lower/smooth/flatten/erode/noise/zoneblend) without loading a scene first. Each mode tested for: panic-free operation; brush feedback visible (cursor + ring overlay); stroke completion produces undo entry.
- Cross-test with scene loaded path to confirm no regression in the previously-working scenario.

**Suggested fix prompt structure**: small (single-commit) prompt with explicit code-relocation diff sketch + verification criteria. Bounded enough for a single small session; ≤30-line code change.

---

## §6 — Forward Observations (out-of-scope but documented)

Per anti-drift discipline §1.5: concerns surfaced during investigation that aren't the diagnostic's target but warrant documentation.

### §6.1 Other latent defects in main.rs:3867 gate region

The `if let (Some(world), Some(viewport))` pattern in main.rs is used at the `EditorDrawContext` population site. If other systems (e.g., post-process settings, weather, lighting, audio) similarly require viewport state to be sync'd independent of world presence, they may have identical bugs. NOT investigated in this diagnostic; tagged for future observability sweep (e.g., next silent-failure-hunter pass on `aw_editor`).

### §6.2 Architectural concern about mediator pattern resilience

The defect demonstrates a structural fragility of the main.rs mediator pattern: each new sync site must be manually placed in correct gating context, with no compile-time enforcement of independence-from-world correctness. Mediator Removal session per Q6 will eliminate this pattern entirely (replaced by ActiveTool dispatch with dispatcher integration at a single point). The Editor Multi-Tool Architecture campaign was already targeting this via Sub-phase 5 + Mediator Removal session; this diagnostic adds empirical weight to the architectural motivation.

### §6.3 Misleading commit message at `f84eb09049`

`f84eb09049` is titled "imported kaykit complete asset package" but actually mixes (a) asset import, (b) terrain brush mediator wire-up (~80 lines of brush-specific code in main.rs + widget.rs), (c) scatter placement integration, (d) drag-asset handler refactor. Methodology lesson: defects are harder to attribute when commits combine unrelated concerns under a misleading title. Commit-strategist discipline (per CLAUDE.md commit-strategist agent) recommends small focused commits; this commit violated that pattern. Documentation only — no fix.

### §6.4 FPS spike on paint mode switch (independent quirk)

`ensure_thumbnails_loaded` first-call cost is bounded but visible. Future UX-polish session could pre-load thumbnails at TerrainPanel construction (or at first show()) instead of deferring to first paint-mode display. Not in any current campaign scope; NOT a brush regression.

---

## §7 — Methodology Lessons

### §7.1 Sub-phase Andrew-gate as defect-discovery mechanism

The Editor Multi-Tool Architecture campaign chain's Andrew-gate discipline produced this diagnostic. Without Sub-phase 3's required Andrew-gate verification (Q9 narrowed to all 8 brush modes per Andrew Q6 (a)), the latent mediator brush regression at `f84eb09049` may have remained undiscovered through Sub-phase 5 + Mediator Removal session — at which point root cause attribution would be much harder (is it dispatcher? is it removal mechanics? is it ActiveTool design? is it pre-existing mediator brokenness?).

The two-stage Andrew-gate verification per Q2 risk-bounding (Sub-phase 3 with both paths active; Mediator Removal with ActiveTool path alone) is exactly the discipline pattern that made this discovery clean. Document for future foundational architectural campaigns: **Andrew-gate's value compounds beyond verifying the current sub-phase's deliverables — it surfaces latent defects in surrounding code by forcing comprehensive verification**.

### §7.2 Sub-phase Andrew-gate REGRESS-not-from-sub-phase pattern

Sub-phase 3's Andrew-gate REGRESS verdict was not caused by Sub-phase 3 changes (per Andrew Q1 verification at pre-Sub-phase 3 commit `79e483e6c`). Pattern: **sub-phase Andrew-gate may surface defects that predate the sub-phase**. Discipline:

1. Verify causality before assuming sub-phase regression.
2. Investigate whether defect is in sub-phase scope or upstream.
3. Route fix accordingly:
   - In-scope sub-phase regression: scope expansion within current sub-phase.
   - Upstream defect surfaced by sub-phase verification: separate diagnostic + fix session, sub-phase closeout deferred.

This diagnostic exemplifies the "upstream defect surfaced by Andrew-gate" pattern. Future Andrew-gate REGRESS reports apply same disambiguation.

### §7.3 Symptom-to-code-path tracing as primary investigation tool

When the prompt's prime-suspect framing (F.5-paint cascade per §1.4) was REFUTED quickly (4 commits' file lists checked, none touched mediator), investigation pivoted to symptom-code-path tracing per prompt §3.3 fallback. The tracing converged in ~3 file inspections:

1. Symptom-trace from "no cursor change" → cursor render block at widget.rs:1428 → gated by `self.terrain_brush_active` → setter at widget.rs:398 → call site at main.rs:3869.
2. Inspection of main.rs:3867 gate → discovery of `Some(world)` requirement.
3. Behavior derivation: terrain generation populates terrain_state, not scene_state.world; scene_state is None in default flow; gate fails; setter not called.

**Methodology lesson**: when prime-suspect framing is wrong, symptom-to-code-path tracing is reliable backup. Single-symptom traces (cursor → render → gate → setter → call site) are bounded and convergent; multiple-hypothesis fan-outs are not necessary if the trace identifies the gate condition.

### §7.4 "Catch-all gate" anti-pattern

The defect is a specific instance of a general anti-pattern: **placing independent state syncs inside a catch-all conditional that aggregates dependencies from unrelated concerns**. The `if let (Some(world), Some(viewport))` gate aggregated:
- Context population (legitimately requires both)
- Brush state sync (only requires viewport)

Aggregation appears to simplify the code (single `if let` block) but couples independent concerns under a conjunction that's stricter than necessary. Future code review discipline: when adding a state sync site to an existing conditional, verify whether the new sync truly requires all existing arm conditions.

---

*End of Sub-phase 3 Mediator Brush Diagnostic audit.*
