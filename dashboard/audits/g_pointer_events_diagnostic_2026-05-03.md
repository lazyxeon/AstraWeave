# G-pointer-events Diagnostic Audit

**Date**: 2026-05-03
**Trigger**: F-fix Andrew-gate brush UX REGRESS — code inspection grounded in G-research's pattern catalog (`docs/audits/g_pointer_events_research_2026-05-03.md`).
**Predecessor work**: G-research audit `6992f4b39` (research catalog) + `e748a6304` (campaign doc) + `506dec13c` (hash-fixup).
**Scope**: code inspection + hypothesis investigation + decision-request surfacing per Andrew's Q1 Option B. **NO production code changes; NO single-path remediation recommendation.** Audit's §7 presents (B-extend) vs (A) vs (C) vs hybrid choice grounded in observed code state; Andrew makes the architectural call between this session and G-pointer-events-fix.

---

## §1 — Investigation methodology

### §1.1 Hypothesis enumeration

Per G-diagnostic prompt §2.1, five hypotheses mapped to the research's (A)/(B)/(C) framework:

- **H1 — approach (B)**: viewport widget checks active-tool state internally.
- **H2 — approach (A)**: higher-layer widget pre-empts via egui Modal layer / layer-priority hit testing.
- **H3 — approach (C)**: editor-level dispatcher with per-tool registration (AAA canonical).
- **H4 — ad-hoc**: none of (A)/(B)/(C) cleanly; bespoke mechanism without canonical classification.
- **H5 — mechanism mismatch**: TerrainPanel works through a different mechanism than would apply to RegionalArchetypePanel.

### §1.2 Investigation phases executed

- **Phase 1** (Editor input dispatch entry points): traced `eframe::App::update()` at [tools/aw_editor/src/main.rs:9062-9063](../../tools/aw_editor/src/main.rs#L9062) → `EditorApp::update_inner()` → viewport rendering at main.rs:3825-3858 → `ViewportWidget::ui()` at [tools/aw_editor/src/viewport/widget.rs:439](../../tools/aw_editor/src/viewport/widget.rs#L439).

- **Phase 2** (TerrainPanel inspection per Q4 Interpretation B): identified `is_brush_active()` at [tools/aw_editor/src/panels/terrain_panel.rs:797](../../tools/aw_editor/src/panels/terrain_panel.rs#L797); identified main.rs sync at [tools/aw_editor/src/main.rs:3834-3877](../../tools/aw_editor/src/main.rs#L3834). Q4 Interpretation B discipline held: classified the approach without investigating multi-texture limitation history.

- **Phase 3** (RegionalArchetypePanel inspection): identified F.5-paint.A scaffold at [tools/aw_editor/src/panels/regional_archetype_panel.rs:75 (`paint_active: bool`)](../../tools/aw_editor/src/panels/regional_archetype_panel.rs#L75) + [line 138 (`queue_paint_op`)](../../tools/aw_editor/src/panels/regional_archetype_panel.rs#L138) + [line 233 (`apply_pending_paint_ops_to_owned`)](../../tools/aw_editor/src/panels/regional_archetype_panel.rs#L233); confirmed via grep that **zero references in main.rs** exist for `RegionalArchetypePanel` or `regional_archetype` (versus 7+ for terrain brush plumbing).

- **Phase 4** (Hypothesis classification): H1 confirmed cleanly; H2/H3/H4/H5 ruled out (evidence in §6 below).

- **Phase 5** (Decision-request authoring): four options presented in §7 — including Option B-extend that surfaces during investigation as the literal smallest scope, beyond the prompt's three-option (A)/(C)/hybrid framing. Per Q1 Option B's "surface decision honestly," this option is included rather than constrained to the prompt's framing.

- **Phase 6** (Audit synthesis): this document.

### §1.3 Supplemental SOTA research (per Q6)

**None executed.** G-research's catalog was sufficient for hypothesis classification + decision-request authoring. Code observations mapped cleanly to research §5.1's (A)/(B)/(C) framework without supplemental searches needed.

### §1.4 Out-of-scope discipline held

- **Save/load**: NOT investigated (H-saveload chain).
- **Climate Preview overlay**: NOT investigated (F.5-overlay-and-gate).
- **TerrainPanel multi-texture limitation history**: NOT investigated (Q4 Interpretation A; classification observation only).
- **Pre-existing `terrain_panel::tests::test_terrain_panel_creation` failure**: noted from F-fix.A-supplement; NOT investigated.
- **F.4 byte-identity regression**: not re-tested in this session (sanity check ran in F-fix; unchanged code paths).

---

## §2 — Andrew-gate brush UX symptom precise catalog

Per F-fix.B §10, observation captured during Andrew-gate verification at HEAD `722b70ae5`:

- **Click+drag in viewport with `RegionalArchetypePanel` open + Paint mode active**: events route to camera pan, NOT to panel brush queue.
- **Click+drag in viewport with `TerrainPanel` open + sculpt brush active**: events route to terrain brush hit collection (functional). This is the working baseline that G-diagnostic mirrors against.
- **Save/load buttons in `RegionalArchetypePanel`**: deferred to H-saveload-diagnostic; not investigated here.

**G-diagnostic did NOT reproduce the symptom in a live editor session**. Per G-research prompt's "no editor launch needed" framing, working from F-fix.B-captured observation as ground truth. The symptom's root cause is unambiguous from code inspection (§3-§6 below); reproduction would not add diagnostic value.

**Modifier-key + button-specific behavior** (inferred from code at [viewport/widget.rs:1167-1255](../../tools/aw_editor/src/viewport/widget.rs#L1167)):

- **Middle-button drag**: always orbit camera (with Shift+Middle = pan); not gated on tool state. So middle-drag works for camera regardless of which panel is active.
- **Primary-button drag**: orbits camera if `!terrain_brush_active && !gizmo_active`; routes to terrain brush if `terrain_brush_active && !gizmo_active`. **No `regional_archetype_paint_active` branch exists** — primary-button drag with RegionalArchetype panel open + Paint mode → falls through to camera orbit (the observed REGRESS).
- **Secondary-button drag** ([line 1336](../../tools/aw_editor/src/viewport/widget.rs#L1336)): camera-related action (likely orbit or pan; gated on `can_control_camera`).

---

## §3 — Editor input dispatch architecture findings

### §3.1 Overall architecture

The editor uses **a single `ViewportWidget`** at [tools/aw_editor/src/viewport/widget.rs](../../tools/aw_editor/src/viewport/widget.rs) for all 3D viewport rendering + input handling. The widget allocates space with `egui::Sense::click_and_drag()` at [line 439](../../tools/aw_editor/src/viewport/widget.rs#L439):

```rust
let (rect, response) = ui.allocate_exact_size(viewport_size, egui::Sense::click_and_drag());
```

This is the canonical egui 3D-viewport pattern from research §2.1. The widget then dispatches all pointer-event handling internally via `ViewportWidget::handle_input()` at [line 864](../../tools/aw_editor/src/viewport/widget.rs#L864), which is called at [line 467](../../tools/aw_editor/src/viewport/widget.rs#L467) of the `ui()` method.

### §3.2 No editor-level dispatcher

Grep across `tools/aw_editor/src/` for the canonical research §3 patterns (`ActiveTool`, `ToolMode`, `dispatch_pointer_event`, `register_tool`, `_forward_3d_gui_input`, `FEdMode`, `forward_input`, etc.) returns **zero matches**. There is:

- **No `ActiveTool` trait** or equivalent.
- **No tool registry** mapping tool name → tool instance.
- **No central dispatcher** that fans out viewport events to registered tools.
- **No Modal layer usage** (`Memory::set_modal_layer()` not called anywhere in the editor).
- **No higher-layer overlay** patterns (e.g., transparent `Window` or `Area` allocated above the viewport rect).

### §3.3 main.rs as per-frame mediator

The editor's pointer-event arbitration logic lives in **`main.rs`'s update loop**, specifically [main.rs:3833-3877](../../tools/aw_editor/src/main.rs#L3833):

```rust
// main.rs:3834 — pre-render: read TerrainPanel state, push to viewport
let brush_active = self.dock_tab_viewer.is_terrain_brush_active();
let brush_radius = self.dock_tab_viewer.terrain_brush_radius();
let brush_is_paint = self.dock_tab_viewer.terrain_brush_is_paint();
// ...
viewport.set_terrain_brush_active(brush_active);
viewport.set_terrain_brush_params(brush_radius, brush_is_paint);

// main.rs:3862 — post-render: drain viewport-collected hits, push back to TerrainPanel
let hits = viewport.take_terrain_brush_hits();
for hit in hits {
    self.dock_tab_viewer.apply_terrain_brush_at(hit[0], hit[1]);
}
if viewport.take_terrain_brush_stroke_ended() {
    // ... undo/redo plumbing ...
}
```

This main.rs mediator is the **only** plumbing between TerrainPanel state and viewport pointer events. There is no equivalent for `RegionalArchetypePanel`.

### §3.4 ViewportWidget knows about TerrainPanel specifically

[ViewportWidget struct fields at lines 162-177](../../tools/aw_editor/src/viewport/widget.rs#L162) include hardcoded TerrainPanel knowledge:

```rust
/// Whether terrain brush mode is active (set externally)
terrain_brush_active: bool,

/// Brush radius for cursor visualization
terrain_brush_radius: f32,
/// Whether the active brush is a paint brush (blue) vs sculpt (green)
terrain_brush_is_paint: bool,

/// Queued terrain brush hits (world X, Z) from mouse clicks in viewport
terrain_brush_hits: Vec<[f32; 2]>,

/// Whether a terrain brush stroke just ended (mouse released)
terrain_brush_stroke_ended: bool,

/// Throttle: last time a brush hit was emitted (limits to ~15 Hz during drag)
last_brush_time: std::time::Instant,
```

Six fields on ViewportWidget reference TerrainPanel's specific tool. They're not generic over tool type; the field names encode the tool name (`terrain_brush_*`).

---

## §4 — TerrainPanel inspection findings (per Q4 Interpretation B)

Per Q4 Interpretation B (lightly expanded — enough to classify; not multi-texture limitation history):

### §4.1 TerrainPanel's brush activation API

[TerrainPanel::is_brush_active() at line 797](../../tools/aw_editor/src/panels/terrain_panel.rs#L797):

```rust
/// Returns true if a sculpting brush mode is active and terrain exists
pub fn is_brush_active(&self) -> bool {
    self.brush_enabled && self.terrain_state.has_terrain()
}
```

Internal state: `brush_enabled: bool` + `terrain_state.has_terrain()` predicate. Brush state is per-panel; main.rs reads it via this method.

### §4.2 BrushMode enum at [terrain_panel.rs:509](../../tools/aw_editor/src/panels/terrain_panel.rs#L509)

TerrainPanel has multiple brush modes (Sculpt, Lower, Smooth, Flatten, Paint, Erode, Noise, ZoneBlend) selected via a `BrushMode` enum. **All brush modes share the same activation flag** (`brush_enabled`); the mode determines what the brush does on each hit, not whether the brush is active.

Important: TerrainPanel's `BrushMode::Paint` is a **terrain texture paint** (paints terrain layer textures), NOT the same concept as `RegionalArchetypePanel`'s archetype paint mode. Naming collision; different domain.

### §4.3 Brush hit application

[TerrainPanel::apply_brush_at() — referenced from main.rs:3864](../../tools/aw_editor/src/main.rs#L3864) and the trait implementation in dock_tab_viewer's `apply_terrain_brush_at(hit[0], hit[1])` — receives world-XZ coordinates and applies the brush operation to the terrain heightmap. main.rs is the bridge.

### §4.4 Classification: approach (B) confirmed

Per research §5.1 (B): "Viewport widget checks active-tool state internally — Unity-style — viewport's `Sense::drag()` handler checks 'is paint mode active?' and routes to brush-queue or camera accordingly."

TerrainPanel's pattern matches (B):
- ViewportWidget has `terrain_brush_active: bool` field (state internal to viewport widget).
- Viewport's `Sense::click_and_drag()` handler at [line 1180-1255](../../tools/aw_editor/src/viewport/widget.rs#L1180) branches on this field.
- Camera orbit is gated `!self.terrain_brush_active`.
- Terrain brush dispatch fires on `self.terrain_brush_active && (dragged || clicked)`.

The only nuance versus pure-(B) is that **the activation state is synced from TerrainPanel via main.rs each frame**, rather than ViewportWidget reading TerrainPanel directly. This is "(B) with main.rs mediator" — not (C), because there's no trait/abstraction; the mediator is hardcoded TerrainPanel-specific.

**Q4 Interpretation A discipline held**: classification observation captured; multi-texture limitation history not investigated. Whether the (B) pattern is *why* multi-texture-paint never expanded is an informational hypothesis surfaced in §9 below; not investigated in this audit.

---

## §5 — RegionalArchetypePanel inspection findings

### §5.1 F.5-paint.A scaffold has all building blocks

[regional_archetype_panel.rs:75 — `paint_active: bool` field](../../tools/aw_editor/src/panels/regional_archetype_panel.rs#L75):

```rust
/// Whether the panel is active (capturing pointer events for paint
/// operations). Set during click+drag; cleared on release.
pub paint_active: bool,
```

The doc comment **explicitly anticipates** the pointer-event capture flag pattern. It's declared `pub`, default `false`, never set or read by anything except the panel's own tests.

[regional_archetype_panel.rs:138 — `queue_paint_op(world_x, world_z)`](../../tools/aw_editor/src/panels/regional_archetype_panel.rs#L138):

```rust
pub fn queue_paint_op(&mut self, world_x: f32, world_z: f32) {
    let archetype_id = match self.paint_mode {
        PaintMode::Paint => self.selected_archetype.to_mask_id(),
        PaintMode::Erase => 0,
    };
    self.pending_paint_ops.push(PaintOp {
        world_x, world_z,
        brush_size_pixels: self.brush_size_pixels,
        archetype_id,
    });
}
```

This receives world-XZ hits identically to TerrainPanel's `apply_brush_at(world_x, world_z)`. **The signature already matches what main.rs would call.**

[regional_archetype_panel.rs:233 — `apply_pending_paint_ops_to_owned()`](../../tools/aw_editor/src/panels/regional_archetype_panel.rs#L233):

```rust
pub fn apply_pending_paint_ops_to_owned(&mut self) {
    if self.pending_paint_ops.is_empty() {
        return;
    }
    self.ensure_mask();
    if let Some(mut mask) = self.mask.take() {
        self.apply_pending_paint_ops(&mut mask);
        self.mask = Some(mask);
    }
}
```

End-of-stroke flush already implemented.

### §5.2 The gap

Workspace-wide grep for `RegionalArchetypePanel` or `regional_archetype` in `tools/aw_editor/src/main.rs` returns **zero matches**. There is:

- **No `is_regional_archetype_paint_active()`** equivalent on dock_tab_viewer.
- **No `viewport.set_regional_archetype_paint_active()`** call.
- **No `viewport.take_regional_archetype_paint_hits()`** drain.
- **No fields on ViewportWidget** mirroring `terrain_brush_*`.
- **No branches in `ViewportWidget::handle_input()`** for archetype paint state.

The panel struct is sound (30 unit tests pass per F-fix.A-supplement scoreboard), but the (B)-pattern plumbing through main.rs + ViewportWidget is missing entirely. F.5-paint.A's panel was never wired into the viewport's pointer-event flow.

### §5.3 Implication

The F.5-paint.A scaffold's `paint_active`, `queue_paint_op`, and `apply_pending_paint_ops_to_owned` are **API-shaped exactly to mirror TerrainPanel's pattern**. F.5-paint.A's author either anticipated the (B)-extend path (most likely) or designed for a future (C) refactor that would have a similar API surface. Either way, the existing API is compatible with both Option B-extend (literal mirror) and Option C (registration-based) without changes.

---

## §6 — Hypothesis classification

**H1 — approach (B) confirmed cleanly.** Evidence:

1. ViewportWidget has typed `terrain_brush_active: bool` field at [widget.rs:163](../../tools/aw_editor/src/viewport/widget.rs#L163) (state internal to viewport widget per (B)'s definition).
2. ViewportWidget's `handle_input()` branches on this field at [widget.rs:1180, 1200, 1258, 1263](../../tools/aw_editor/src/viewport/widget.rs#L1180) (routing per (B)'s behavior).
3. Camera orbit gated on `!self.terrain_brush_active` at [widget.rs:1183](../../tools/aw_editor/src/viewport/widget.rs#L1183) ("camera consumes only when tool inactive" per (B)).
4. main.rs mediator pattern at [main.rs:3834-3877](../../tools/aw_editor/src/main.rs#L3834) (hardcoded TerrainPanel sync; no abstraction).

**H2 — approach (A) ruled out.** Evidence:
- No `Memory::set_modal_layer` calls anywhere in `tools/aw_editor/src/`.
- No transparent overlay or `egui::Area::interactable` patterns found.
- Layer manipulation absent — viewport allocates with default LayerId; panels render in their dock tabs at default LayerId.
- No `egui::Order::Foreground` or similar layer-elevation patterns.

**H3 — approach (C) ruled out.** Evidence:
- No `ActiveTool` / `ToolMode` / `Tool` trait found.
- No tool registry struct.
- No central dispatcher that fans out events to registered tools.
- The `Panel` trait at [tools/aw_editor/src/panels/mod.rs:9-18](../../tools/aw_editor/src/panels/mod.rs#L9) is purely about UI rendering (`name() + show(&mut Ui)`); has no pointer-event hooks like Godot's `_forward_3d_gui_input` or Unreal's `FEdMode::InputKey`.

**H4 — ad-hoc ruled out.** The pattern IS canonically (B); the main.rs mediator is a (B) refinement, not an ad-hoc divergence. The classification is clean.

**H5 — mechanism mismatch ruled out.** TerrainPanel works through the same mechanism that would apply to RegionalArchetypePanel: typed flag on ViewportWidget + handle_input branching + main.rs sync + post-render hit drain. RegionalArchetypePanel's `queue_paint_op(world_x, world_z)` signature already matches the call site shape. The only difference: the plumbing isn't built.

**Conclusion**: AstraWeave editor uses **approach (B) with main.rs as per-frame mediator**. The architecture is hardcoded for TerrainPanel; adding any new viewport-pointer-receiving panel requires either extending the (B) pattern (mirror TerrainPanel's plumbing) or refactoring to (A) or (C).

---

## §7 — Architectural decision request

**Per Q1 Option B: surfaced to Andrew, not recommended by agent.**

Investigation surfaced **four** options rather than the prompt's three-option (A)/(C)/hybrid framing. The fourth — **Option B-extend** — is the literal smallest scope given the existing approach (B). It was added per Q1 Option B's "surface decision honestly" discipline; its omission would mislead Andrew about actual scope alternatives.

### Option B-extend (literal smallest scope) — Mirror TerrainPanel's pattern exactly

**What it requires** (concrete steps based on [widget.rs:162-177](../../tools/aw_editor/src/viewport/widget.rs#L162) + [main.rs:3833-3877](../../tools/aw_editor/src/main.rs#L3833) precedent):

1. **ViewportWidget fields** (5 new fields, mirror lines 162-177 of widget.rs):
   - `regional_archetype_paint_active: bool` (mirror `terrain_brush_active`)
   - `regional_archetype_brush_radius: f32` (mirror `terrain_brush_radius` — convert from panel's `brush_size_pixels` × world-extent-per-pixel)
   - `regional_archetype_paint_hits: Vec<[f32; 2]>` (mirror `terrain_brush_hits`)
   - `regional_archetype_stroke_ended: bool` (mirror `terrain_brush_stroke_ended`)
   - `last_archetype_paint_time: Instant` (mirror `last_brush_time`)
2. **ViewportWidget setters** (3 new methods):
   - `set_regional_archetype_paint_active(active: bool)` (mirror line 352)
   - `set_regional_archetype_paint_params(radius: f32)` (mirror line 357)
   - `take_regional_archetype_paint_hits() -> Vec<[f32; 2]>` (mirror existing `take_terrain_brush_hits`)
   - `take_regional_archetype_paint_stroke_ended() -> bool`
3. **ViewportWidget::handle_input() branches** (mirror lines 1180-1255):
   - Camera orbit gate: extend `&& !self.regional_archetype_paint_active` to the camera-orbit condition at line 1183.
   - Archetype paint dispatch: add an `if self.regional_archetype_paint_active && !self.gizmo_state.is_active() && (dragged_by(Primary) || clicked_by(Primary))` block mirroring lines 1200-1255.
   - Stroke end detection: mirror line 1258.
4. **TerrainPanel-style accessor on `RegionalArchetypePanel`**:
   - Add `pub fn is_paint_active(&self) -> bool { self.paint_active && self.mask.is_some() }` (mirror TerrainPanel::is_brush_active pattern).
   - The existing `paint_active: bool` field (line 75) is the activation flag; needs to be set when user enters paint mode (via a Paint/Idle toggle or implicit on PaintMode::Paint selection — design decision in G-fix).
5. **Bridge accessor on tab_viewer mod.rs**:
   - Add `is_regional_archetype_paint_active(&self) -> bool` calling through to the panel (mirror existing `is_terrain_brush_active`).
   - Add `apply_regional_archetype_paint_at(&mut self, world_x: f32, world_z: f32)` calling through to `self.regional_archetype_panel.queue_paint_op(world_x, world_z)`.
   - Add `regional_archetype_paint_radius() -> f32` accessor.
6. **main.rs sync + drain** (mirror lines 3834-3877):
   - Pre-render block: read panel state; push to viewport via setters.
   - Post-render block: drain hits via `viewport.take_regional_archetype_paint_hits()`; route to `dock_tab_viewer.apply_regional_archetype_paint_at(...)`. Detect stroke end; flush via `apply_pending_paint_ops_to_owned()`.

**Estimated scope**: 1 commit, ~3-4 hours wall-clock. ~150-200 lines of edits across 3 files (`widget.rs`, `main.rs`, `tab_viewer/mod.rs`) + 1 new accessor on `regional_archetype_panel.rs`.

**Trade-offs**:
- ✅ Smallest possible scope; mirrors existing battle-tested pattern; minimal risk to TerrainPanel's working brush.
- ✅ G-fix Andrew-gate scope is narrow (verify archetype paint works just like terrain paint).
- ❌ **Inherits the multi-tool scaling failure mode**: each future paint tool (splat, scatter, vegetation override, weather zones) requires another full set of viewport widget fields + accessor methods + main.rs sync code + handle_input branches. ViewportWidget grows unbounded.
- ❌ Doesn't address the underlying architectural concern G-research surfaced: the (B) pattern is the **most likely structural reason** TerrainPanel's multi-texture work never expanded (Q4 Interpretation A informational note from research §5.3). Choosing this option means future paint tools face the same friction.
- ❌ The naming convention `terrain_brush_*` / `regional_archetype_paint_*` / `splat_paint_*` / `scatter_paint_*` etc. accumulates in ViewportWidget with no abstraction.

**Code anchor**: ViewportWidget at `tools/aw_editor/src/viewport/widget.rs` (struct + handle_input); main.rs at line 3833-3877 (mediator pattern).

**Risk factors**:
- TerrainPanel's `terrain_brush_active` and the new `regional_archetype_paint_active` need mutex-style arbitration (only one tool active at a time — what happens if user has both panels open with paint modes active simultaneously?). Code might need a "first one wins" or "global active tool ID" disambiguation. This concern surfaces in Option C as a built-in benefit; in Option B-extend it's an open question for G-fix to resolve.

### Option A (medium scope) — Higher-layer widget pre-emption via egui Modal layer

**What it requires** (concrete steps based on research §2.3 + §5.1 (A)):

1. **RegionalArchetypePanel renders an additional transparent overlay** on a higher LayerId when paint mode is active:
   - When `panel.paint_mode_active`, allocate `egui::Area::new("regional_archetype_paint_overlay").order(egui::Order::Foreground)` covering the viewport rect.
   - Inside the Area, allocate `Sense::click_and_drag()` rect overlapping the viewport.
   - On `response.dragged_by(Primary)`, project pointer to world-XZ via the panel's existing `screen_to_world_xz_y0()` helper (already present at panel line 257).
   - Call `self.queue_paint_op(world_x, world_z)`.
   - Optionally set `Memory::set_modal_layer(self.area.layer_id)` to block lower-layer interactions.
2. **ViewportWidget remains unchanged** for archetype paint case (no new fields, no new branches). TerrainPanel keeps using its existing (B) pattern.
3. **Unified end-of-stroke detection** via `response.drag_stopped_by(Primary)` inside the overlay; flush via `apply_pending_paint_ops_to_owned()`.

**Estimated scope**: 1-2 commits, ~5-6 hours wall-clock. ~80-120 lines of edits in `regional_archetype_panel.rs` only (no main.rs / widget.rs changes for this panel).

**Trade-offs**:
- ✅ Doesn't touch ViewportWidget or main.rs (insulated change).
- ✅ Doesn't propagate (B) pattern — RegionalArchetypePanel uses (A); TerrainPanel keeps using (B); they coexist.
- ✅ Future paint tools could use Option A pattern, slowly migrating the codebase.
- ❌ Introduces a NEW pattern in the codebase (egui Modal / `egui::Area::Foreground`); no precedent to mirror. Higher implementation risk than mirroring battle-tested (B).
- ❌ Two patterns coexist (B for TerrainPanel, A for RegionalArchetypePanel); inconsistency may confuse future contributors.
- ❌ Modal layer interaction with other panels (Inspector, Hierarchy, etc.) is non-trivial — incorrectly setting modal layer could block panel interactions globally. Risk of cross-panel UX regression.
- ❌ Coordinate math: overlay's world-XZ projection must match camera state; if camera is moved while paint mode is active, ray-plane intersection drift may produce incorrect paint positions. TerrainPanel's (B) approach uses depth-buffer reads from the viewport renderer (widget.rs:1219-1234) which is more accurate than ray-plane projection; Option A loses this without further work.
- ❌ Does NOT address multi-tool scaling concern at the architectural level — it just routes around the (B) pattern for one panel.

**Code anchor**: `tools/aw_editor/src/panels/regional_archetype_panel.rs::show()` method (would extend to allocate the overlay Area). egui Modal pattern reference: research §2.3 + [egui PR #5358](https://github.com/emilk/egui/pull/5358).

**Risk factors**:
- Overlay-vs-panel interaction edge cases (does the overlay block clicking on the dock tab system?).
- Coordinate accuracy without depth-buffer access.
- Modal layer global side effects.

### Option C (broad scope) — Editor-level dispatcher with per-tool registration

**What it requires** (concrete steps based on research §3 AAA editors + §4.1 Fyrox precedent):

1. **Define an `ActiveTool` trait** (or similar) at `tools/aw_editor/src/active_tool.rs` (new file):
   - `fn handle_pointer_event(&mut self, event: PointerEvent, world_xz: Option<[f32; 2]>) -> EventDisposition;`
   - `fn is_active(&self) -> bool;`
   - `fn name(&self) -> &str;`
   - `EventDisposition::Consumed` / `EventDisposition::PassThrough` enum (mirror Godot's `AfterGUIInput`).
2. **Add a tool registry on `EditorTabViewer` or separate `ActiveToolRegistry`**:
   - Stores `Vec<Box<dyn ActiveTool>>` or per-tool struct fields with trait dispatch.
   - `fn dispatch_pointer_event(&mut self, event: PointerEvent) -> EventDisposition;` — iterates registered tools; first to return `Consumed` wins.
3. **Migrate TerrainPanel to implement `ActiveTool`**:
   - Move terrain-brush state + dispatch logic from ViewportWidget INTO TerrainPanel.
   - Remove `terrain_brush_active` field + handle_input branches from ViewportWidget.
   - Remove main.rs mediator code for TerrainPanel (now handled by ActiveTool dispatch).
4. **Implement `ActiveTool` for RegionalArchetypePanel**: similar scope; routes events to `queue_paint_op`.
5. **Wire ActiveTool dispatch into ViewportWidget**:
   - In `handle_input()`, before camera-orbit logic, call `active_tool_registry.dispatch_pointer_event(event)`.
   - If returns `Consumed` → skip camera handling. If `PassThrough` → continue to camera.
6. **Add Pattern A regression tests** for the dispatch pattern: active tool consumes; inactive passes through; multi-tool exclusivity; modifier-key arbitration.

**Estimated scope**: 3-5 sessions (each its own commit chain):
- Session 1 (architecture): define `ActiveTool` trait + registry + EventDisposition enum + Pattern A tests. ~1 day.
- Session 2 (TerrainPanel migration): refactor TerrainPanel to ActiveTool implementation; remove ViewportWidget's typed fields; remove main.rs mediator; verify TerrainPanel's existing brush still works (Andrew-gate). ~1 day. Risk to TerrainPanel functionality.
- Session 3 (RegionalArchetypePanel registration): implement ActiveTool for RegionalArchetypePanel. ~half day.
- Session 4 (integration tests + Pattern A regression): ~half day.
- Session 5 (closeout + Andrew-gate): ~half day.

Total: ~3-4 days of focused work; possibly longer if TerrainPanel migration surfaces unexpected coupling.

**Trade-offs**:
- ✅ Forward-compatible: future paint tools (splat, scatter, vegetation, weather) implement `ActiveTool` and register; ViewportWidget code is unchanged.
- ✅ Matches AAA canonical pattern (research §3).
- ✅ Clean separation of concerns: tools own their logic; registry handles arbitration; viewport handles rendering.
- ✅ Retroactively benefits TerrainPanel — multi-texture-paint expansion would no longer require ViewportWidget edits.
- ❌ **Substantially larger upfront cost**: 3-5 sessions vs Option B-extend's 1 commit.
- ❌ Refactor risk to TerrainPanel's working brush during Session 2. F.5-paint's blocking issue gets resolved later in the chain (after Sessions 1-3), so brush UX REGRESS persists longer.
- ❌ New abstraction may surface unforeseen issues (e.g., depth-buffer access from inside ActiveTool implementations; ownership lifetimes for pointer events).
- ❌ Campaign velocity cost: F.5-paint COMPLETE date pushes back several days; F.5-overlay-and-gate further delayed; F.6-F.8 chain shifted.

**Code anchor**: New file `tools/aw_editor/src/active_tool.rs`; refactor of `widget.rs` (lines 162-1255) and `main.rs` (lines 3833-3877); migration of `terrain_panel.rs::is_brush_active` + related; addition to `regional_archetype_panel.rs`.

**Risk factors**:
- TerrainPanel migration may surface tight coupling between viewport renderer (depth buffer reads at widget.rs:1219-1234) and brush state. Refactoring this cleanly may require additional rendering-layer changes.
- Pattern A test design for dispatcher requires synthetic egui input harness (more complex than F-fix.A's tests which only verified registry membership + struct instantiation).
- Three downstream sessions (G-fix-2 TerrainPanel migration + G-fix-3 RegionalArchetypePanel registration + G-fix-4 integration tests) each carry their own Andrew-gate risk.

### Hybrid (B-extend-now, C-later) — Mirror existing pattern now, refactor as future architectural campaign

**What it requires**:
- **G-fix executes Option B-extend** (1 commit, 3-4 hours).
- **A future architectural campaign (e.g., "Editor Tool Architecture Refactor")** executes Option C when Andrew chooses to schedule it. Likely after F.5-paint COMPLETE, possibly after F.5-overlay-and-gate, possibly during F.6-F.7 if the multi-tool scaling concern becomes acute.

**Estimated scope**: G-fix bounded by Option B-extend (1 commit, 3-4 hours); future Option C campaign sized separately when scheduled (3-5 sessions when it happens).

**Trade-offs**:
- ✅ Unblocks F.5-paint quickly: brush UX works at end of G-fix; H-saveload follows immediately; F.5-paint COMPLETE in days, not weeks.
- ✅ Preserves option to do (C) later when its strategic value is higher.
- ✅ Lowest immediate risk: only 1 commit; mirrors existing pattern.
- ❌ Risks (C) being deferred indefinitely under priority pressure: if F.6-F.8 take precedence, the (B) pattern accumulates more tool-specific fields with each new paint tool, making (C) refactor progressively harder.
- ❌ Two future paint tools (splat, scatter) added before (C) lands would entrench (B) further — each adds another set of typed fields + branches that (C) must later refactor away.
- ❌ Naming-collision risk persists between TerrainPanel's `BrushMode::Paint` (terrain texture paint) and RegionalArchetypePanel's `PaintMode::Paint` (archetype paint). (B)-extend doesn't disambiguate; future tool additions could compound.

**Code anchor**: same as Option B-extend; future (C) campaign anchor deferred.

**Risk factors**:
- All Option B-extend risks (multi-tool scaling, mutex arbitration question) for now.
- Future (C) campaign risks (TerrainPanel migration, unforeseen coupling) when scheduled.
- Strategic risk: deferral pressure compounds technical debt.

### Decision factors for Andrew

These are the strategic factors only Andrew can weigh:

1. **How soon will splat / scatter / vegetation override / weather zone painting actually be built?**
   - If "soon" (within F.5-overlay-and-gate or F.6 timeframe): Option C's multi-tool benefit pays back quickly.
   - If "deferred indefinitely" or "speculative": Option B-extend's narrow scope is more efficient.

2. **Is editor-architecture refactor a priority to take on now?**
   - If yes (e.g., other architectural concerns are also accumulating): Option C bundles concerns into one campaign.
   - If no (priorities are elsewhere): Option B-extend or Hybrid keeps focus on F.5-paint.

3. **Are other AstraWeave priorities competing for attention?**
   - F.6 (scattered-convolution at regional layer) is next on the campaign roadmap.
   - F.7 (principal Andrew-gate) is the campaign's principal verification.
   - F.5-overlay-and-gate is the next forward-progress session after F.5-paint COMPLETE.
   - All three benefit from F.5-paint closing faster (Option B-extend or Hybrid).

4. **Is the campaign-velocity cost of Option C's 3-5 sessions acceptable?**
   - If Option C lands after H-saveload but before F.5-overlay-and-gate: ~1 week delay to F.5-paint COMPLETE.
   - If Option C lands as a separate campaign post-F.5: F.5 completes faster but multi-tool scaling concern remains during F.5-overlay-and-gate's overlay work.

5. **Is the multi-tool scaling concern from research §5 actionable now or speculative?**
   - The concern is **structural** (Option B's coupling to ViewportWidget); the **failure mode** ("multi-texture paint never expanded") is observed in TerrainPanel but causal attribution to (B) is informational not investigative per Q4 Interpretation A.
   - If Andrew weights structural concerns highly: Option C.
   - If Andrew weights observed-failure-mode evidence as required for refactor justification: more data needed before Option C; Hybrid or Option B-extend now.

6. **What's the appetite for refactor risk to TerrainPanel during Option C's Session 2?**
   - TerrainPanel is currently working (its brush is the F-fix.B Andrew-gate baseline).
   - Option C migrates TerrainPanel to ActiveTool; refactor risk is real.
   - Option B-extend leaves TerrainPanel completely untouched.
   - If Andrew is risk-averse for working systems: Option B-extend or Hybrid.
   - If Andrew accepts refactor risk for forward-compatibility benefit: Option C.

---

## §8 — Pattern A regression test sketches (per chosen approach)

Per research §7, regression tests for the pointer-event class. Sketches differ by approach:

### §8.1 — If Option B-extend chosen

Pattern A tests mirror F-fix.A's existing pattern (panel registry membership + EditorTabViewer instantiation). Add:

```rust
// In tab_viewer/mod.rs tests:
#[test]
fn editor_tab_viewer_provides_regional_archetype_paint_active_accessor() {
    let tv = EditorTabViewer::new();
    assert_eq!(tv.is_regional_archetype_paint_active(), false);
}

#[test]
fn editor_tab_viewer_routes_paint_hits_to_panel() {
    let mut tv = EditorTabViewer::new();
    tv.regional_archetype_panel.paint_active = true;
    tv.apply_regional_archetype_paint_at(100.0, 200.0);
    assert_eq!(tv.regional_archetype_panel.pending_paint_ops.len(), 1);
}

// In viewport/widget.rs tests (if test harness allows):
#[test]
fn viewport_widget_has_regional_archetype_paint_active_field() {
    let w = ViewportWidget::default_for_tests();  // hypothetical
    assert_eq!(w.regional_archetype_paint_active, false);
    w.set_regional_archetype_paint_active(true);
    assert_eq!(w.regional_archetype_paint_active, true);
}
```

These verify the (B)-extend plumbing exists. Functional verification of "drag actually routes to panel queue" requires synthetic egui input harness; achievable but non-trivial. Andrew-gate covers functional verification.

### §8.2 — If Option A chosen

Pattern A tests verify the higher-layer overlay's existence + interaction:

```rust
#[test]
fn regional_archetype_panel_allocates_overlay_when_paint_mode_active() {
    let mut panel = RegionalArchetypePanel::default();
    panel.paint_mode_active = true;  // hypothetical activation
    
    let mut ctx = egui::Context::default();
    ctx.run(Default::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            panel.show(ui);
        });
    });
    
    // Verify a foreground Area was registered
    let layers = ctx.memory(|m| m.layer_ids());
    assert!(layers.iter().any(|l| l.id == panel.area_id()));  // hypothetical
}
```

These are structurally harder than (B)-extend's because they require running egui's render pipeline in tests.

### §8.3 — If Option C chosen

Pattern A tests verify the dispatcher pattern:

```rust
#[test]
fn active_tool_dispatcher_routes_to_active_tool() {
    let mut registry = ActiveToolRegistry::new();
    let archetype_panel = RegionalArchetypePanel::default();
    archetype_panel.paint_mode_active = true;
    registry.register(Box::new(archetype_panel));
    
    let event = PointerEvent::Drag { world_xz: Some([100.0, 200.0]), button: Primary };
    let disposition = registry.dispatch_pointer_event(event);
    
    assert_eq!(disposition, EventDisposition::Consumed);
    assert_eq!(registry.tool_named("regional_archetype").pending_paint_ops().len(), 1);
}

#[test]
fn inactive_tool_passes_event_through() {
    let mut registry = ActiveToolRegistry::new();
    let archetype_panel = RegionalArchetypePanel::default();
    // paint_mode_active = false (default)
    registry.register(Box::new(archetype_panel));
    
    let event = PointerEvent::Drag { world_xz: Some([100.0, 200.0]), button: Primary };
    let disposition = registry.dispatch_pointer_event(event);
    
    assert_eq!(disposition, EventDisposition::PassThrough);
}

#[test]
fn only_one_active_tool_consumes() {
    let mut registry = ActiveToolRegistry::new();
    let mut terrain = TerrainPanel::default();
    let mut archetype = RegionalArchetypePanel::default();
    terrain.brush_enabled = false;
    archetype.paint_mode_active = true;
    registry.register(Box::new(terrain));
    registry.register(Box::new(archetype));
    
    let event = PointerEvent::Drag { world_xz: Some([100.0, 200.0]), button: Primary };
    let disposition = registry.dispatch_pointer_event(event);
    
    assert_eq!(disposition, EventDisposition::Consumed);
    // Verify only archetype received the event:
    assert!(registry.tool_named("terrain").pending_brush_ops().is_empty());
    assert_eq!(registry.tool_named("regional_archetype").pending_paint_ops().len(), 1);
}
```

These are strongest forward-compatible regression tests; close the multi-tool exclusivity class permanently.

### §8.4 — If Hybrid chosen

§8.1 tests for G-fix; §8.3 tests for the future Option C campaign. (§8.2 not needed — Hybrid skips Option A.)

---

## §9 — Out-of-scope observations and forward references

### §9.1 TerrainPanel multi-texture limitation correlates with approach (B)

Per Q4 Interpretation A informational framing: the research §5.3 raised the hypothesis that TerrainPanel's known multi-texture-paint limitation **may** correlate with approach (B)'s multi-tool scaling failure mode. **G-diagnostic confirms approach (B) is in place**, so the structural hypothesis is now anchored by code observation.

This is informational only — G-diagnostic does NOT investigate whether multi-texture-paint failed because of (B) specifically vs other reasons (feature-incomplete, priority deferral, etc.). The observation is preserved for future architectural decisions:

- If Andrew chooses Option C, the migration retroactively addresses this structural concern.
- If Andrew chooses Option B-extend or Hybrid, the structural concern persists and may resurface as future paint tools are added.

### §9.2 Save/load deferred to H-saveload chain

Untestable without working brush. H-saveload-diagnostic begins after G-fix Andrew-gate verifies brush UX.

### §9.3 Climate Preview overlay deferred to F.5-overlay-and-gate

Per F.5 split (research §0 inheritance): Climate Preview overlay is not in any G-pointer-events session's scope.

### §9.4 Pre-existing terrain_panel test failure noted

Per F-fix.A-supplement: `panels::terrain_panel::tests::test_terrain_panel_creation` fails on pre-supplement state too (chunk_radius asserted 5 but actual 10). NOT investigated in G-diagnostic; tracked for separate standalone follow-up.

### §9.5 Mutex arbitration question (G-fix to resolve)

If the user activates **both** TerrainPanel's brush AND RegionalArchetypePanel's paint mode simultaneously (e.g., both panels open in different docks; both have brush-enabled toggled on), the current code would have ViewportWidget's `handle_input()` route events to BOTH branches in sequence (camera orbit gated `!terrain_brush_active && !regional_archetype_paint_active`; terrain brush dispatch fires; archetype paint dispatch also fires). This is potentially confusing UX.

Options for G-fix to resolve:
- Mutex: at any time at most one paint mode is active; activating one auto-disables the other.
- Stack: most-recently-activated tool wins; older tools deactivated when new ones activate.
- Concurrent: both tools receive events; user explicitly responsible for not having both active.
- Per-dock: tool active state tied to which dock has focus.

This question affects all four options:
- Option B-extend: mutex/stack arbitration handled in main.rs mediator.
- Option A: modal layer naturally enforces single-tool-active (modal layer claims everything).
- Option C: dispatcher's iteration order or "first-Consumed-wins" rule handles arbitration.
- Hybrid: mutex/stack now (G-fix); dispatcher-based later (C).

### §9.6 Existing RegionalArchetypePanel `paint_active` field is unused

[regional_archetype_panel.rs:75 — `paint_active: bool`](../../tools/aw_editor/src/panels/regional_archetype_panel.rs#L75) field exists with anticipatory documentation but is never read or written by any code path (only by the panel's own `Default::default()` at line 104 and a unit test assertion at line 684). G-fix should activate this field per chosen option's semantics.

### §9.7 Forward chain (per prompt §0)

1. **Andrew architectural decision** (between G-diagnostic and G-fix): chooses Option B-extend, A, C, or Hybrid based on §7 trade-offs + §9.5 arbitration question handling.
2. **G-pointer-events-fix prompt drafting**: shapes G-fix scope based on chosen approach.
3. **G-pointer-events-fix execution**: applies the chosen approach + Pattern A regression tests + Andrew-gate verification.
4. **H-saveload-diagnostic + H-saveload-fix**: save/load remediation chain; unblocked by working brush.
5. **Final Andrew-gate full PASS** → **F.5-paint COMPLETE** → **F.5-overlay-and-gate** → F.6 → F.7 (principal Andrew-gate) → F.8 closeout.

---

## §10 — Supplemental SOTA research bibliography

**No supplemental searches executed.** G-research's catalog (see `docs/audits/g_pointer_events_research_2026-05-03.md` §9 bibliography) was sufficient for hypothesis classification + decision-request authoring. Code observations mapped cleanly to research's (A)/(B)/(C) framework without needing additional research.

If G-fix's prompt drafting surfaces a need for additional patterns (e.g., specific egui::Area::Foreground semantics for Option A; specific ActiveTool trait shape for Option C), G-fix's prompt §1 should authorize targeted supplemental searches at agent discretion per Andrew's Q6 framing.

---

*End of G-pointer-events diagnostic audit.*
